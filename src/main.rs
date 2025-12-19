//! HVAT - Hyperspectral Annotation Tool
//! Native entry point

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use hvat::HvatApp;
    use hvat_ui::{run_with_settings, ClearColor, Settings};

    // Note: env_logger is initialized by hvat_ui internally
    log::info!("HVAT starting...");

    let settings = Settings::new()
        .title("HVAT - Hyperspectral Annotation Tool")
        .size(1400, 900)
        .background(ClearColor::rgb(0.12, 0.12, 0.15))
        .target_fps(60);

    if let Err(e) = run_with_settings(HvatApp::new(), settings) {
        log::error!("Application error: {}", e);
    }
}

// WASM uses wasm_bindgen start function, not main()
#[cfg(target_arch = "wasm32")]
fn main() {}
