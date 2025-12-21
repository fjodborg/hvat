//! WASM entry point for HVAT

use wasm_bindgen::prelude::*;

use crate::{default_settings, HvatApp};
use hvat_ui::run_with_settings;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(async {
        run_app().await;
    });
}

async fn run_app() {
    log::info!("HVAT WASM starting...");

    if let Err(e) = run_with_settings(HvatApp::new(), default_settings()) {
        log::error!("Application error: {}", e);
    }
}
