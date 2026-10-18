//! Palette seed decoding.
use super::*;
pub fn parse_seed(raw: &str) -> Option<u32> {
    let trimmed = raw.trim().trim_start_matches('#');
    let digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    if digits.is_empty() || digits.len() > 8 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    // A six-digit value carries no alpha; make it fully opaque.
    Some(if digits.len() <= 6 {
        0xff00_0000 | value
    } else {
        value
    })
}

fn argb_to_u32(argb: Argb) -> u32 {
    ((argb.alpha as u32) << 24)
        | ((argb.red as u32) << 16)
        | ((argb.green as u32) << 8)
        | argb.blue as u32
}

/// Quick check that the file starts with a known image magic number.
///
/// `material_colors`' reader *panics* on undecodable data, so filtering out
/// obvious non-images here keeps the common mistake quiet.
fn looks_like_image(bytes: &[u8]) -> bool {
    let starts = |magic: &[u8]| bytes.starts_with(magic);
    starts(&[0x89, b'P', b'N', b'G'])
        || starts(&[0xff, 0xd8, 0xff])
        || starts(b"GIF8")
        || starts(b"BM")
        || starts(b"II*\0")
        || starts(b"MM\0*")
        || starts(b"qoif")
        || (bytes.len() >= 12 && starts(b"RIFF") && &bytes[8..12] == b"WEBP")
}

/// Extract the Material You source color from a wallpaper image, if possible.
pub fn seed_from_wallpaper(path: &Path) -> Option<u32> {
    let bytes = std::fs::read(path).ok()?;
    if !looks_like_image(&bytes) {
        tracing::warn!(path = %path.display(), "wallpaper is not a recognised image");
        return None;
    }
    // The reader panics on corrupt data; silence the hook for this call and
    // recover the failure as a plain `None`.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut image = material_colors::image::ImageReader::open(path).ok()?;
        image.resize(96, 96, material_colors::image::FilterType::Triangle);
        Some(argb_to_u32(
            material_colors::image::ImageReader::extract_color(&image),
        ))
    }));
    std::panic::set_hook(previous_hook);
    match result {
        Ok(seed) => seed,
        Err(_) => {
            tracing::warn!(path = %path.display(), "wallpaper could not be decoded; ignoring it");
            None
        }
    }
}
