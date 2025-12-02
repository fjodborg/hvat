// Shared application logic
mod app;

// HVAT application (shared between native and WASM)
mod hvat_app;
pub use hvat_app::HvatApp;

// WASM entry point for hvat
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;

// Native entry point for hvat
#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub use native::run;

// Placeholder public API for the library
pub fn init() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        println!("HVAT - Hyperspectral Annotation Tool");
        println!("Use `cargo run --bin hvat-native` to run the native version");
    }
}

// Test image generator
pub fn generate_test_image(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            // Create a colorful test pattern
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            let b = (((x + y) as f32 / (width + height) as f32) * 255.0) as u8;

            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255); // Alpha
        }
    }

    data
}
