//! DMA-BUF requests are validated by the renderer that will actually draw them.
use super::*;
use smithay::backend::renderer::{gles::GlesRenderer, ImportDma};
impl State {
    pub fn validate_imports(&mut self, renderer: &mut GlesRenderer) {
        for (buffer, notifier) in self.pending_imports.drain(..) {
            match renderer.import_dmabuf(&buffer, None) {
                Ok(_) => {
                    let _ = notifier.successful::<Self>();
                }
                Err(error) => {
                    tracing::warn!(?error, "client DMA-BUF cannot be imported");
                    notifier.failed();
                }
            }
        }
    }
    pub fn advertise_formats(&mut self, formats: Vec<Format>) {
        if self.advertised_formats.len() == formats.len()
            && formats.iter().all(|f| self.advertised_formats.contains(f))
        {
            return;
        }
        self.dmabuf_state
            .disable_global::<Self>(&self.display_handle, &self._dmabuf_global);
        let next = self
            .dmabuf_state
            .create_global::<Self>(&self.display_handle, formats.clone());
        self.retired_dmabuf_globals
            .push(std::mem::replace(&mut self._dmabuf_global, next));
        self.advertised_formats = formats;
    }
}
