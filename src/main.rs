//! HVAT - Hyperspectral Vision Annotation Tool
//! Native entry point

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use hvat::{HvatApp, default_settings};
    use hvat_ui::run_with_settings;

    // Note: env_logger is initialized by hvat_ui internally
    log::info!("HVAT starting...");

    if let Err(e) = run_with_settings(HvatApp::new(), default_settings()) {
        log::error!("Application error: {}", e);
    }
}

// WASM uses wasm_bindgen start function, not main()
#[cfg(target_arch = "wasm32")]
fn main() {}
