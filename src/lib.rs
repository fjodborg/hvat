//! HVAT - Hyperspectral Annotation Tool
//!
//! A GPU-accelerated desktop and web application for hyperspectral image annotation.

mod app;
mod constants;
mod data;
mod message;
mod model;
mod state;
mod test_image;
mod ui;

pub use app::HvatApp;

// WASM entry point
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
