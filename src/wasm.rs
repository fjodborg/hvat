use wasm_bindgen::prelude::*;

// Import the main application from this crate
use crate::ui_constants::window;
use crate::HvatApp;
use hvat_ui::{run, Settings};

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(async {
        run_app().await;
    });
}

async fn run_app() {
    // Run the hvat_ui application with default settings
    let settings = Settings {
        window_title: Some("HVAT - Hyperspectral Annotation Tool".to_string()),
        window_size: window::DEFAULT_SIZE,
        min_window_size: Some(window::MIN_SIZE),
        resizable: true,
        log_level: log::LevelFilter::Debug,
    };

    log::info!("HVAT WASM starting...");
    log::info!("Initializing HVAT UI...");
    log::info!("Note: Winit uses exceptions for control flow - any exception errors below are expected and can be ignored.");

    if let Err(e) = run::<HvatApp>(settings) {
        log::error!("Application error: {}", e);
    }
}
