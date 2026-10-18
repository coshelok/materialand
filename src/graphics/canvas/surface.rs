//! Surface.
use super::*;

impl Surface {
    pub fn new(width: u32, height: u32) -> Self {
        let pixmap = Pixmap::new(width.max(1), height.max(1)).expect("allocate surface");
        Self { pixmap }
    }

    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn duplicate(&self) -> Self {
        Self {
            pixmap: self.pixmap.clone(),
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.pixmap.fill(color);
    }

    pub fn to_memory_buffer(&self) -> MemoryRenderBuffer {
        MemoryRenderBuffer::from_slice(
            self.pixmap.data(),
            Fourcc::Abgr8888,
            (self.pixmap.width() as i32, self.pixmap.height() as i32),
            1,
            Transform::Normal,
            None,
        )
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        self.pixmap.data_mut()
    }
}
