//! org.freedesktop.Notifications on Materialand's private session bus.
use super::model::Store;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use zbus::{fdo, zvariant::OwnedValue};
pub(super) const PATH: &str = "/org/freedesktop/Notifications";
pub(super) const NAME: &str = "org.freedesktop.Notifications";
pub(super) struct Server(pub Arc<Mutex<Store>>);
#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    fn get_capabilities(&self) -> Vec<&str> {
        vec!["body", "persistence"]
    }
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        // This version advertises text and persistence; optional image/action
        // data is accepted but never presented as a nonfunctional control.
        let _ = (app_icon, actions, hints);
        self.0
            .lock()
            .unwrap()
            .notify(replaces_id, app_name, summary, body, expire_timeout)
    }
    fn close_notification(&self, id: u32) -> fdo::Result<()> {
        if self.0.lock().unwrap().close(id, 3) {
            Ok(())
        } else {
            Err(fdo::Error::InvalidArgs("Unknown notification".into()))
        }
    }
    #[zbus(signal)]
    async fn notification_closed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;
    #[zbus(out_args("name", "vendor", "version", "spec_version"))]
    fn get_server_information(&self) -> (&str, &str, &str, &str) {
        (
            "Materialand",
            "Materialand",
            env!("CARGO_PKG_VERSION"),
            "1.2",
        )
    }
}
