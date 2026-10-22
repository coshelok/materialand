//! Render SVG icons into premultiplied pixels without a session bus or image portal.
use super::Surface;
use std::{
    ffi::{c_char, c_int, c_void, CString},
    os::unix::ffi::OsStrExt,
    path::Path,
};

#[repr(C)]
struct Rectangle {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
#[link(name = "rsvg-2")]
unsafe extern "C" {
    fn rsvg_handle_new_from_file(path: *const c_char, error: *mut *mut c_void) -> *mut c_void;
    fn rsvg_handle_get_intrinsic_size_in_pixels(
        handle: *mut c_void,
        w: *mut f64,
        h: *mut f64,
    ) -> c_int;
    fn rsvg_handle_render_document(
        handle: *mut c_void,
        cr: *mut c_void,
        viewport: *const Rectangle,
        error: *mut *mut c_void,
    ) -> c_int;
}
#[link(name = "cairo")]
unsafe extern "C" {
    fn cairo_image_surface_create(format: c_int, width: c_int, height: c_int) -> *mut c_void;
    fn cairo_surface_status(surface: *mut c_void) -> c_int;
    fn cairo_surface_flush(surface: *mut c_void);
    fn cairo_image_surface_get_data(surface: *mut c_void) -> *const u8;
    fn cairo_image_surface_get_stride(surface: *mut c_void) -> c_int;
    fn cairo_surface_destroy(surface: *mut c_void);
    fn cairo_create(surface: *mut c_void) -> *mut c_void;
    fn cairo_destroy(cr: *mut c_void);
    fn cairo_status(cr: *mut c_void) -> c_int;
}
#[link(name = "gobject-2.0")]
unsafe extern "C" {
    fn g_object_unref(p: *mut c_void);
}

struct Handle(*mut c_void);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { g_object_unref(self.0) }
    }
}
struct Image(*mut c_void);
impl Drop for Image {
    fn drop(&mut self) {
        unsafe { cairo_surface_destroy(self.0) }
    }
}
struct Context(*mut c_void);
impl Drop for Context {
    fn drop(&mut self) {
        unsafe { cairo_destroy(self.0) }
    }
}

pub(super) fn load(path: &Path, size: u32) -> Option<Surface> {
    let name = CString::new(path.as_os_str().as_bytes()).ok()?;
    // Native objects are owned by the guards; pixels are copied before their destruction.
    unsafe {
        let raw = rsvg_handle_new_from_file(name.as_ptr(), std::ptr::null_mut());
        if raw.is_null() {
            return None;
        }
        let handle = Handle(raw);
        let (mut width, mut height) = (0.0, 0.0);
        let intrinsic =
            rsvg_handle_get_intrinsic_size_in_pixels(handle.0, &mut width, &mut height) != 0;
        let size = size.clamp(1, 256);
        let (w, h) = if intrinsic
            && width.is_finite()
            && height.is_finite()
            && width > 0.0
            && height > 0.0
        {
            let ratio = f64::from(size) / width.max(height);
            (
                (width * ratio).round().max(1.0) as u32,
                (height * ratio).round().max(1.0) as u32,
            )
        } else {
            (size, size)
        };
        // CAIRO_FORMAT_ARGB32 is native-endian, premultiplied 0xAARRGGBB.
        let image = Image(cairo_image_surface_create(0, w as c_int, h as c_int));
        if cairo_surface_status(image.0) != 0 {
            return None;
        }
        let context = Context(cairo_create(image.0));
        if cairo_status(context.0) != 0 {
            return None;
        }
        let viewport = Rectangle {
            x: 0.0,
            y: 0.0,
            width: w.into(),
            height: h.into(),
        };
        if rsvg_handle_render_document(handle.0, context.0, &viewport, std::ptr::null_mut()) == 0 {
            return None;
        }
        cairo_surface_flush(image.0);
        let stride = cairo_image_surface_get_stride(image.0);
        let data = cairo_image_surface_get_data(image.0);
        if data.is_null() || stride < w as i32 * 4 {
            return None;
        }
        let bytes = std::slice::from_raw_parts(data, stride as usize * h as usize);
        let mut result = Surface::new(w, h);
        for y in 0..h as usize {
            for x in 0..w as usize {
                let src = y * stride as usize + x * 4;
                let argb = u32::from_ne_bytes(bytes[src..src + 4].try_into().ok()?);
                let dst = (y * w as usize + x) * 4;
                result.pixmap.data_mut()[dst..dst + 4].copy_from_slice(&[
                    (argb >> 16) as u8,
                    (argb >> 8) as u8,
                    argb as u8,
                    (argb >> 24) as u8,
                ]);
            }
        }
        Some(result)
    }
}
