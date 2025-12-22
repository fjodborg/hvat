//! HVAT - Hyperspectral Vision Annotation Tool
//!
//! A GPU-accelerated desktop and web application for hyperspectral image annotation.

mod app;
pub mod config;
mod constants;
mod data;
pub mod format;
mod keybindings;
pub mod licenses;
mod message;
mod model;
mod state;
mod test_image;
mod ui;

pub use app::HvatApp;
use hvat_ui::{ClearColor, Settings};

/// Application title shown in window titlebar
const APP_TITLE: &str = "HVAT - Hyperspectral Vision Annotation Tool";

/// Default window width
const DEFAULT_WINDOW_WIDTH: u32 = 1400;

/// Default window height
const DEFAULT_WINDOW_HEIGHT: u32 = 900;

/// Target frames per second
const TARGET_FPS: u32 = 60;

/// Embedded window icon (128x128 PNG)
#[cfg(not(target_arch = "wasm32"))]
const WINDOW_ICON: &[u8] = include_bytes!("../assets/icon.png");

/// Create default application settings for both native and WASM builds.
pub fn default_settings() -> Settings {
    let settings = Settings::new()
        .title(APP_TITLE)
        .size(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)
        .background(ClearColor::rgb(0.12, 0.12, 0.15))
        .target_fps(TARGET_FPS);

    // Add embedded window icon for native builds
    #[cfg(not(target_arch = "wasm32"))]
    let settings = settings.icon_bytes(WINDOW_ICON);

    settings
}

// WASM entry point
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
