use std::collections::HashMap;
use std::path::{Path, PathBuf};

use fontdue::{Font, FontSettings};

use crate::style::TextStyle;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontKind {
    Regular,
    Medium,
    Bold,
    Icon,
}

struct Glyph {
    width: usize,
    height: usize,
    xmin: i32,
    ymin: i32,
    advance: f32,
    coverage: Vec<u8>,
}

pub struct Fonts {
    regular: Font,
    medium: Font,
    bold: Font,
    icons: Font,
    icon_codepoints: HashMap<String, char>,
    cache: HashMap<(FontKind, char, u16), Glyph>,
}

mod blending;
mod discovery;
mod glyphs;
mod loading;
mod painting;
use blending::*;
use discovery::*;
#[cfg(test)]
mod tests;
