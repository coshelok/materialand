//! Materialand executable entry point.
mod app;
mod backend;
mod config;
mod desktop;
mod graphics;
mod input;
mod motion;
mod shell;
mod style;
mod theme;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn main() -> Result<()> {
    app::run()
}

mod system;

mod notifications;
