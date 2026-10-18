//! Import completion is deferred until the backend renderer has tested it.
use super::super::*;
impl DmabufHandler for State {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }
    fn dmabuf_imported(&mut self, _: &DmabufGlobal, buffer: Dmabuf, notifier: ImportNotifier) {
        self.pending_imports.push((buffer, notifier));
    }
}
