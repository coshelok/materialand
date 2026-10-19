//! Window state and display state have separate owners and lifetimes.
use super::*;
#[derive(Default)]
pub struct WindowManager {
    pub order: Vec<Window>,
    pub space: smithay::desktop::Space<Window>,
    pub props: HashMap<Window, WindowProps>,
    pub meta: HashMap<Window, WindowMeta>,
    pub focused: Option<Window>,
    pub cascade: i32,
    pub pending_resize: HashMap<Window, super::resize::ResizePlacement>,
}
pub struct DisplayModel {
    pub handle: Output,
    pub size: Size<i32, Logical>,
}
