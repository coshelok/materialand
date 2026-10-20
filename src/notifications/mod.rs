//! Session-local desktop notifications; the compositor never blocks on D-Bus.
mod model;
mod service;
#[cfg(test)]
pub use model::Notice;
pub use model::Snapshot;
use model::Store;
use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct Notifications {
    store: Arc<Mutex<Store>>,
    revision: Option<u64>,
    _lifetime: Option<mpsc::Sender<()>>,
}
impl Notifications {
    pub fn start() -> Self {
        let store = Arc::new(Mutex::new(Store::default()));
        let shared = store.clone();
        let (lifetime, stop) = mpsc::channel();
        thread::spawn(move || {
            let connection = zbus::blocking::connection::Builder::session()
                .and_then(|b| b.serve_at(service::PATH, service::Server(shared.clone())))
                .and_then(|b| b.name(service::NAME))
                .and_then(|b| b.build());
            let connection = match connection {
                Ok(c) => {
                    let mut s = shared.lock().unwrap();
                    s.view.available = true;
                    s.view.revision += 1;
                    c
                }
                Err(error) => {
                    tracing::warn!(%error, "notification service unavailable");
                    let mut s = shared.lock().unwrap();
                    s.view.error = Some("Notification service unavailable".into());
                    s.view.revision += 1;
                    return;
                }
            };
            while matches!(
                stop.recv_timeout(Duration::from_millis(100)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                let signals = shared.lock().unwrap().signals(Instant::now());
                for (id, reason) in signals {
                    if let Err(error) = connection.emit_signal(
                        None::<&str>,
                        service::PATH,
                        service::NAME,
                        "NotificationClosed",
                        &(id, reason),
                    ) {
                        tracing::warn!(%error, "could not signal notification dismissal");
                    }
                }
            }
        });
        Self {
            store,
            revision: None,
            _lifetime: Some(lifetime),
        }
    }
    pub fn poll(&mut self) -> Option<Snapshot> {
        let store = self.store.try_lock().ok()?;
        if self.revision == Some(store.view.revision) {
            return None;
        }
        self.revision = Some(store.view.revision);
        Some(store.view.clone())
    }
    pub fn dismiss(&mut self, id: Option<u32>) {
        let mut store = self.store.lock().unwrap();
        let ids = id
            .map(|id| vec![id])
            .unwrap_or_else(|| store.view.items.iter().map(|n| n.id).collect());
        for id in ids {
            store.close(id, 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a private session D-Bus and Unix socket sendmsg permissions"]
    fn real_session_bus_delivers_replaces_and_withdraws_notifications() {
        let _bus = crate::app::session_bus::SessionBus::start().expect("private D-Bus");
        let mut notifications = Notifications::start();
        let until = Instant::now() + Duration::from_secs(5);
        while !notifications.poll().is_some_and(|view| view.available) {
            assert!(Instant::now() < until, "notification service must register");
            thread::sleep(Duration::from_millis(10));
        }
        let connection = zbus::blocking::Connection::session().unwrap();
        type NotifyArgs = (
            String,
            u32,
            String,
            String,
            String,
            Vec<String>,
            std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
            i32,
        );
        let send = |method, args: &NotifyArgs| {
            connection
                .call_method(
                    Some(service::NAME),
                    service::PATH,
                    Some(service::NAME),
                    method,
                    args,
                )
                .unwrap()
                .body()
                .deserialize::<u32>()
                .unwrap()
        };
        let args = |replace, title: &str| {
            (
                "Test app".into(),
                replace,
                "".into(),
                title.into(),
                "Message".into(),
                vec![],
                std::collections::HashMap::new(),
                0,
            )
        };
        let id = send("Notify", &args(0, "First"));
        assert_eq!(send("Notify", &args(id, "Replaced")), id);
        let snapshot = notifications.poll().unwrap();
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].summary, "Replaced");
        connection
            .call_method(
                Some(service::NAME),
                service::PATH,
                Some(service::NAME),
                "CloseNotification",
                &(id,),
            )
            .unwrap();
        assert!(notifications.poll().unwrap().items.is_empty());
    }
}
