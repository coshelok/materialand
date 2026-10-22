//! Decode PNG and SVG directly; retain GdkPixbuf for legacy raster formats.
use super::Surface;
use std::{
    ffi::{c_char, c_int, c_void, CString},
    os::unix::ffi::OsStrExt,
    path::Path,
};
#[link(name = "gdk_pixbuf-2.0")]
unsafe extern "C" {
    fn gdk_pixbuf_new_from_file_at_scale(
        path: *const c_char,
        width: c_int,
        height: c_int,
        preserve: c_int,
        error: *mut *mut c_void,
    ) -> *mut c_void;
    fn gdk_pixbuf_get_width(p: *const c_void) -> c_int;
    fn gdk_pixbuf_get_height(p: *const c_void) -> c_int;
    fn gdk_pixbuf_get_rowstride(p: *const c_void) -> c_int;
    fn gdk_pixbuf_get_n_channels(p: *const c_void) -> c_int;
    fn gdk_pixbuf_get_bits_per_sample(p: *const c_void) -> c_int;
    fn gdk_pixbuf_get_pixels_with_length(p: *const c_void, length: *mut u32) -> *const u8;
}
#[link(name = "gobject-2.0")]
unsafe extern "C" {
    fn g_object_unref(p: *mut c_void);
}
#[link(name = "glib-2.0")]
unsafe extern "C" {
    fn g_error_free(p: *mut c_void);
}
struct Pixbuf(*mut c_void);
impl Drop for Pixbuf {
    fn drop(&mut self) {
        unsafe { g_object_unref(self.0) }
    }
}
pub(super) fn load(path: &Path, size: u32) -> Option<Surface> {
    let size = size.clamp(1, 256);
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
    {
        return super::svg::load(path, size);
    }
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("png"))
    {
        let source = Surface {
            pixmap: tiny_skia::Pixmap::load_png(path).ok()?,
        };
        let ratio = size as f32 / source.width().max(source.pixmap.height()) as f32;
        let mut scaled = Surface::new(
            (source.width() as f32 * ratio).round().max(1.0) as u32,
            (source.pixmap.height() as f32 * ratio).round().max(1.0) as u32,
        );
        scaled.blit(
            &source,
            tiny_skia::Transform::from_scale(ratio, ratio),
            1.0,
            None,
        );
        return Some(scaled);
    }
    let name = CString::new(path.as_os_str().as_bytes()).ok()?;
    let size = size.clamp(1, 256) as i32;
    // All pointers remain owned by Pixbuf until copied into a Rust-owned pixmap.
    unsafe {
        let mut error = std::ptr::null_mut();
        let raw = gdk_pixbuf_new_from_file_at_scale(name.as_ptr(), size, size, 1, &mut error);
        if !error.is_null() {
            g_error_free(error);
        }
        if raw.is_null() {
            return None;
        }
        let p = Pixbuf(raw);
        let w = gdk_pixbuf_get_width(p.0);
        let h = gdk_pixbuf_get_height(p.0);
        let stride = gdk_pixbuf_get_rowstride(p.0);
        let channels = gdk_pixbuf_get_n_channels(p.0);
        if w <= 0
            || h <= 0
            || w > size
            || h > size
            || !(3..=4).contains(&channels)
            || gdk_pixbuf_get_bits_per_sample(p.0) != 8
            || stride < w * channels
        {
            return None;
        }
        let mut length = 0;
        let pixels = gdk_pixbuf_get_pixels_with_length(p.0, &mut length);
        if pixels.is_null()
            || (length as usize) < (h as usize - 1) * stride as usize + (w * channels) as usize
        {
            return None;
        }
        let bytes = std::slice::from_raw_parts(pixels, length as usize);
        let mut s = Surface::new(w as u32, h as u32);
        for y in 0..h as usize {
            for x in 0..w as usize {
                let src = y * stride as usize + x * channels as usize;
                let dst = (y * w as usize + x) * 4;
                let a = if channels == 4 { bytes[src + 3] } else { 255 };
                for c in 0..3 {
                    s.pixmap.data_mut()[dst + c] =
                        ((u16::from(bytes[src + c]) * u16::from(a) + 127) / 255) as u8;
                }
                s.pixmap.data_mut()[dst + 3] = a;
            }
        }
        Some(s)
    }
}
