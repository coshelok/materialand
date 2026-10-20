//! Notification state is shared briefly; socket I/O stays on the service thread.
use std::time::{Duration, Instant};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub id: u32,
    pub app: String,
    pub summary: String,
    pub body: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub available: bool,
    pub items: Vec<Notice>,
    pub error: Option<String>,
    pub revision: u64,
}
#[derive(Default)]
pub(super) struct Store {
    pub view: Snapshot,
    next: u32,
    deadlines: Vec<(u32, Instant)>,
    signals: Vec<(u32, u32)>,
}
impl Store {
    pub fn notify(
        &mut self,
        replace: u32,
        app: String,
        summary: String,
        body: String,
        timeout: i32,
    ) -> u32 {
        let existing = self.view.items.iter().any(|n| n.id == replace);
        let id = if existing {
            replace
        } else {
            loop {
                self.next = self.next.wrapping_add(1).max(1);
                if !self.view.items.iter().any(|n| n.id == self.next) {
                    break self.next;
                }
            }
        };
        let text = |s: String, max| {
            s.chars()
                .filter(|c| !c.is_control() || *c == '\n')
                .take(max)
                .collect()
        };
        let notice = Notice {
            id,
            app: text(app, 256),
            summary: text(summary, 512),
            body: text(body, 4096),
        };
        if let Some(old) = self.view.items.iter_mut().find(|n| n.id == id) {
            *old = notice;
        } else {
            if self.view.items.len() >= 64 {
                let oldest = self.view.items[0].id;
                self.close(oldest, 4);
            }
            self.view.items.push(notice);
        }
        self.deadlines.retain(|(key, _)| *key != id);
        if timeout > 0 {
            self.deadlines
                .push((id, Instant::now() + Duration::from_millis(timeout as u64)));
        }
        self.view.revision = self.view.revision.wrapping_add(1);
        id
    }
    pub fn close(&mut self, id: u32, reason: u32) -> bool {
        let Some(index) = self.view.items.iter().position(|n| n.id == id) else {
            return false;
        };
        self.view.items.remove(index);
        self.deadlines.retain(|(key, _)| *key != id);
        self.signals.push((id, reason));
        self.view.revision = self.view.revision.wrapping_add(1);
        true
    }
    pub fn signals(&mut self, now: Instant) -> Vec<(u32, u32)> {
        let expired: Vec<_> = self
            .deadlines
            .iter()
            .filter(|(_, deadline)| *deadline <= now)
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            self.close(id, 1);
        }
        std::mem::take(&mut self.signals)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replace_expiry_and_dismiss_keep_unique_ids_and_correct_close_reasons() {
        let mut store = Store::default();
        let id = store.notify(0, "App".into(), "First".into(), "Body".into(), 0);
        assert_ne!(id, 0);
        assert_eq!(
            store.notify(id, "App".into(), "Replaced".into(), "".into(), 5),
            id
        );
        assert_eq!(store.view.items.len(), 1);
        assert_eq!(store.view.items[0].summary, "Replaced");
        assert_eq!(
            store.signals(Instant::now() + Duration::from_secs(1)),
            [(id, 1)]
        );
        let next = store.notify(0, "App".into(), "Persistent".into(), "".into(), 0);
        assert_ne!(id, next);
        assert!(store.close(next, 2));
        assert!(!store.close(next, 2));
        assert_eq!(store.signals(Instant::now()), [(next, 2)]);
    }
    #[test]
    fn notifications_stay_bounded_and_default_to_persistence() {
        let mut store = Store::default();
        for _ in 0..70 {
            store.notify(0, "App".into(), "New".into(), "".into(), -1);
        }
        assert_eq!(store.view.items.len(), 64);
        assert_eq!(
            store
                .signals(Instant::now() + Duration::from_secs(86400))
                .len(),
            6
        );
        assert_eq!(store.view.items.len(), 64);
    }
}
