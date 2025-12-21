//! WASM entry point for HVAT

use wasm_bindgen::prelude::*;

use crate::HvatApp;
use hvat_ui::{run_with_settings, ClearColor, Settings};

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(async {
        run_app().await;
    });
}

async fn run_app() {
    let settings = Settings::new()
        .title("HVAT - Hyperspectral Vision Annotation Tool")
        .size(1400, 900)
        .background(ClearColor::rgb(0.12, 0.12, 0.15))
        .target_fps(60);

    log::info!("HVAT WASM starting...");

    if let Err(e) = run_with_settings(HvatApp::new(), settings) {
        log::error!("Application error: {}", e);
    }
}
