//! HVAT - Hyperspectral Vision Annotation Tool
//!
//! A GPU-accelerated desktop and web application for hyperspectral image annotation.

mod app;
mod constants;
mod data;
pub mod format;
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

/// Create default application settings for both native and WASM builds.
pub fn default_settings() -> Settings {
    Settings::new()
        .title(APP_TITLE)
        .size(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)
        .background(ClearColor::rgb(0.12, 0.12, 0.15))
        .target_fps(TARGET_FPS)
}

// WASM entry point
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
