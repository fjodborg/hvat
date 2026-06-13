//! SAM2 Encoder Web Worker - WASM only
//!
//! This worker runs the compute-intensive SAM2 encoder in a separate thread.
//! It receives image data from the main thread, processes it through the encoder,
//! and returns the image embeddings for fast mask decoding.
//!
//! # Build
//!
//! This binary is built separately from the main app:
//! ```bash
//! cargo build --bin sam2-encoder-worker --target wasm32-unknown-unknown --features sam2
//! wasm-bindgen target/wasm32-unknown-unknown/release/sam2_encoder_worker.wasm \
//!   --out-dir dist/sam2 --target web
//! ```
//!
//! # Protocol
//!
//! Request (main → worker):
//! ```json
//! { "id": u32, "image_data": Uint8Array, "width": u32, "height": u32 }
//! ```
//!
//! Success response (worker → main):
//! ```json
//! { "id": u32, "embeddings": Float32Array, "shape": [u32; 4] }
//! ```
//!
//! Error response:
//! ```json
//! { "id": u32, "error": string }
//! ```
//!
//! Ready signal:
//! ```json
//! { "type": "ready" }
//! ```

// Native build just prints an error and exits
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("sam2-encoder-worker is a WASM-only binary for running in a Web Worker");
    eprintln!("On native platforms, use the native SAM2 encoder thread instead");
    std::process::exit(1);
}

// WASM worker implementation
#[cfg(target_arch = "wasm32")]
fn main() {
    // Entry point is handled by wasm_bindgen(start)
}

#[cfg(target_arch = "wasm32")]
mod worker {
    use wasm_bindgen::prelude::*;

    /// WASM entry point - called when the worker module loads.
    #[wasm_bindgen(start)]
    pub fn start() {
        // Set up panic hook for better error messages
        console_error_panic_hook::set_once();

        // TODO: Initialize ONNX runtime and load encoder model
        // TODO: Set up message handler for encode requests

        // For now, just log that we started
        web_sys::console::log_1(&"SAM2 encoder worker started (stub)".into());

        // Signal ready to main thread
        // let ready_msg = js_sys::Object::new();
        // js_sys::Reflect::set(&ready_msg, &"type".into(), &"ready".into()).unwrap();
        // let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
        // global.post_message(&ready_msg).unwrap();
    }
}
