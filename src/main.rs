/// Main HVAT Application entry point for native builds
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use hvat::HvatApp;
    use hvat_ui::{run, Settings};
    
    env_logger::init();

    // Create and run the application
    let settings = Settings {
        window_title: Some("HVAT - Hyperspectral Annotation Tool".to_string()),
        window_size: (1200, 800),
        resizable: true,
    };

    if let Err(e) = run::<HvatApp>(settings) {
        eprintln!("Application error: {}", e);
    }
}

// WASM doesn't use main(), it uses wasm_bindgen's start function
#[cfg(target_arch = "wasm32")]
fn main() {}
