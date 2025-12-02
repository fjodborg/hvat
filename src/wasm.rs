use wasm_bindgen::prelude::*;

// Import the main application from this crate
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
    web_sys::console::log_1(&"HVAT WASM starting...".into());
    web_sys::console::log_1(&"Initializing HVAT UI...".into());
    web_sys::console::log_1(&"ℹ️  Note: Winit uses exceptions for control flow - any exception errors below are expected and can be ignored.".into());

    // Run the hvat_ui application with default settings
    let settings = Settings {
        window_title: Some("HVAT - Hyperspectral Annotation Tool".to_string()),
        window_size: (1200, 800),
        resizable: true,
    };

    if let Err(e) = run::<HvatApp>(settings) {
        web_sys::console::log_1(&format!("Application error: {}", e).into());
    }
}
