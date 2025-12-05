/// Main HVAT Application entry point for native builds
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use hvat::ui_constants::window;
    use hvat::HvatApp;
    use hvat_ui::{run, Settings};

    // Create and run the application with logging configured in Settings
    let settings = Settings {
        window_title: Some("HVAT - Hyperspectral Annotation Tool".to_string()),
        window_size: window::DEFAULT_SIZE,
        min_window_size: Some(window::MIN_SIZE),
        resizable: true,
        log_level: log::LevelFilter::Debug,
    };

    if let Err(e) = run::<HvatApp>(settings) {
        eprintln!("Application error: {}", e);
    }
}

// WASM doesn't use main(), it uses wasm_bindgen's start function
#[cfg(target_arch = "wasm32")]
fn main() {}
