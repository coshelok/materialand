//! One ordered writer, with a bounded slot for the latest requested preferences.
use super::{Appearance, AppearanceStore};
use std::{
    io,
    sync::{Arc, Condvar, Mutex},
    thread,
};

#[derive(Default)]
pub struct AppearanceWriter {
    store: AppearanceStore,
    worker: Option<Worker>,
    revision: u64,
}
#[derive(Default)]
struct Pending {
    next: Option<(u64, Appearance)>,
    completed: Option<(u64, io::Result<()>)>,
    processed: u64,
    closed: bool,
}
struct Worker {
    shared: Arc<(Mutex<Pending>, Condvar)>,
    thread: Option<thread::JoinHandle<()>>,
}
impl AppearanceWriter {
    pub fn new(store: AppearanceStore) -> Self {
        Self {
            store,
            ..Self::default()
        }
    }
    pub fn request(&mut self, appearance: Appearance) -> io::Result<()> {
        if self.store.path.is_none() {
            return Ok(());
        }
        if self.worker.is_none() {
            let store = self.store.clone();
            self.worker = Some(Worker::start(move |value| store.save(value))?);
        }
        self.revision += 1;
        self.worker
            .as_ref()
            .unwrap()
            .request(self.revision, appearance);
        Ok(())
    }
    /// Old write results must not clear an error belonging to a newer request.
    pub fn poll(&mut self) -> Option<io::Result<()>> {
        let worker = self.worker.as_ref()?;
        let mut pending = worker.shared.0.try_lock().ok()?;
        let (revision, result) = pending.completed.take()?;
        (revision == self.revision).then_some(result)
    }
    #[cfg(test)]
    pub fn flush(&self) {
        if let Some(worker) = &self.worker {
            worker.flush(self.revision);
        }
    }
}
impl Worker {
    fn start(
        mut save: impl FnMut(Appearance) -> io::Result<()> + Send + 'static,
    ) -> io::Result<Self> {
        let shared = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let writer = shared.clone();
        let thread = thread::Builder::new()
            .name("appearance-save".into())
            .spawn(move || loop {
                let (revision, appearance) = {
                    let (lock, wake) = &*writer;
                    let mut pending = lock.lock().unwrap();
                    loop {
                        if let Some(next) = pending.next.take() {
                            break next;
                        }
                        if pending.closed {
                            return;
                        }
                        pending = wake.wait(pending).unwrap();
                    }
                };
                // Parsing, validation, fsync and rename happen without holding the
                // queue lock. UI requests can always replace the next pending value.
                let result = save(appearance);
                if let Err(error) = &result {
                    tracing::warn!(%error, "could not save appearance");
                }
                let mut pending = writer.0.lock().unwrap();
                pending.processed = revision;
                pending.completed = Some((revision, result));
                writer.1.notify_all();
            })?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }
    fn request(&self, revision: u64, appearance: Appearance) {
        self.shared.0.lock().unwrap().next = Some((revision, appearance));
        self.shared.1.notify_one();
    }
    #[cfg(test)]
    fn flush(&self, revision: u64) {
        let pending = self.shared.0.lock().unwrap();
        let (pending, timeout) = self
            .shared
            .1
            .wait_timeout_while(pending, std::time::Duration::from_secs(5), |p| {
                p.processed < revision
            })
            .unwrap();
        assert!(
            !timeout.timed_out() && pending.processed >= revision,
            "appearance worker did not finish"
        );
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.shared.0.lock().unwrap().closed = true;
        self.shared.1.notify_one();
        // Normal session shutdown drains the last requested value before exit.
        // This join never runs on a theme toggle or an ordinary frame.
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                tracing::warn!("appearance writer panicked during shutdown");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, time::Duration};
    fn value(dark: bool) -> Appearance {
        Appearance {
            dark,
            variant: crate::theme::ColorVariant::TonalSpot,
            reduce_motion: false,
        }
    }

    #[test]
    fn blocked_disk_does_not_block_requests_and_shutdown_saves_latest_value() {
        let (started, entered) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let writes = Arc::new(Mutex::new(Vec::new()));
        let record = writes.clone();
        let worker = Worker::start(move |appearance| {
            let first = record.lock().unwrap().is_empty();
            if first {
                started.send(()).unwrap();
                gate.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            record.lock().unwrap().push(appearance);
            Ok(())
        })
        .unwrap();
        worker.request(1, value(false));
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        // These complete while the writer is still blocked. Pending memory is
        // one slot regardless of how often a user toggles the controls.
        for revision in 2..=1000 {
            worker.request(revision, value(true));
        }
        assert_eq!(
            worker.shared.0.lock().unwrap().next,
            Some((1000, value(true)))
        );
        release.send(()).unwrap();
        drop(worker);
        assert_eq!(*writes.lock().unwrap(), vec![value(false), value(true)]);
    }

    #[test]
    fn stale_failure_is_ignored_and_current_success_is_reported_once() {
        let (started, entered) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let worker = Worker::start(move |appearance| {
            started.send(()).unwrap();
            gate.recv_timeout(Duration::from_secs(5)).unwrap();
            if appearance.dark {
                Ok(())
            } else {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            }
        })
        .unwrap();
        let mut writer = AppearanceWriter {
            store: AppearanceStore::at(Some("unused".into())),
            worker: Some(worker),
            revision: 0,
        };
        writer.request(value(false)).unwrap();
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        writer.request(value(true)).unwrap();
        release.send(()).unwrap();
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            writer.poll().is_none(),
            "failure of the superseded value is stale"
        );
        release.send(()).unwrap();
        writer.flush();
        assert!(writer.poll().unwrap().is_ok());
        assert!(writer.poll().is_none());
    }
}
