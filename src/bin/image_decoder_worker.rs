//! Image decoder Web Worker
//!
//! Compiled as a separate WASM binary that runs in a Web Worker thread.
//! This offloads CPU-intensive image decoding AND RGBA packing from the main
//! thread to enable smooth UI while preloading images in the background.
//!
//! ## Message Protocol
//!
//! ### Request (main → worker):
//! ```json
//! { "id": u32, "name": "path/to/image.png", "bytes": Uint8Array }
//! ```
//!
//! ### Response (worker → main):
//! Success:
//! ```json
//! {
//!   "id": u32,
//!   "name": "path/to/image.png",
//!   "width": u32,
//!   "height": u32,
//!   "num_bands": u32,
//!   "num_layers": u32,
//!   "layers": [Uint8Array, Uint8Array, ...]  // Pre-packed RGBA layers ready for GPU upload
//! }
//! ```
//! Error:
//! ```json
//! { "id": u32, "name": "path/to/image.png", "error": "error message" }
//! ```
//!
//! ### Ready Signal (worker → main):
//! ```json
//! { "type": "ready" }
//! ```

// This binary is WASM-only - on native we just provide a dummy main
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("image-decoder-worker is a WASM-only binary");
    std::process::exit(1);
}

// WASM needs an empty main - wasm_bindgen(start) handles the actual entry point
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Object, Reflect, Uint8Array};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

/// Number of spectral bands packed into each RGBA texture layer.
/// Duplicated from constants.rs since this is a separate binary.
#[cfg(target_arch = "wasm32")]
const BANDS_PER_LAYER: usize = 4;

/// Minimum texture array layers for GPU textures (WebGL2 workaround).
#[cfg(target_arch = "wasm32")]
const MIN_TEXTURE_LAYERS: u32 = 2;

/// Entry point - called when WASM is initialized in the worker
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();

    let global: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();

    // Set up message handler for decode requests
    let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
        handle_message(event);
    }) as Box<dyn Fn(MessageEvent)>);

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget(); // Keep closure alive for the lifetime of the worker

    // Signal to main thread that worker is ready
    let ready_msg = Object::new();
    Reflect::set(&ready_msg, &"type".into(), &"ready".into()).unwrap();
    if let Err(e) = global.post_message(&ready_msg) {
        web_sys::console::error_1(&format!("Failed to send ready signal: {:?}", e).into());
    }

    web_sys::console::log_1(&"[Worker] Image decoder worker initialized".into());
}

/// Handle incoming decode request from main thread
#[cfg(target_arch = "wasm32")]
fn handle_message(event: MessageEvent) {
    let data = event.data();

    // Extract request fields
    let id = match Reflect::get(&data, &"id".into()) {
        Ok(v) => v.as_f64().unwrap_or(0.0) as u32,
        Err(_) => {
            web_sys::console::error_1(&"[Worker] Missing 'id' field in request".into());
            return;
        }
    };

    let name = match Reflect::get(&data, &"name".into()) {
        Ok(v) => v.as_string().unwrap_or_default(),
        Err(_) => {
            send_error(id, "", "Missing 'name' field in request");
            return;
        }
    };

    let bytes_js = match Reflect::get(&data, &"bytes".into()) {
        Ok(v) => v,
        Err(_) => {
            send_error(id, &name, "Missing 'bytes' field in request");
            return;
        }
    };

    let bytes_array: Uint8Array = bytes_js.unchecked_into();
    let bytes = bytes_array.to_vec();

    web_sys::console::log_1(
        &format!("[Worker] Decoding image: {} ({} bytes)", name, bytes.len()).into(),
    );

    // Perform the decode and pack into RGBA layers
    match decode_and_pack(&bytes) {
        Ok(result) => {
            send_success(id, &name, result);
        }
        Err(e) => {
            send_error(id, &name, &e);
        }
    }
}

/// Result of decoding: pre-packed RGBA layers ready for GPU upload
#[cfg(target_arch = "wasm32")]
struct DecodeResult {
    width: u32,
    height: u32,
    num_bands: usize,
    num_layers: u32,
    /// Each layer is a Vec<u8> of RGBA data (width * height * 4 bytes)
    layers: Vec<Vec<u8>>,
}

/// Decode image bytes and pack directly into RGBA layers
///
/// This does all the CPU-intensive work in the worker thread:
/// 1. Decode image bytes to pixels
/// 2. Extract RGB channels
/// 3. Pack into RGBA texture layers (4 bands per layer)
///
/// The main thread receives ready-to-upload data with no further processing needed.
#[cfg(target_arch = "wasm32")]
fn decode_and_pack(data: &[u8]) -> Result<DecodeResult, String> {
    let img = image::load_from_memory(data)
        .map_err(|e| format!("Failed to decode image: {}", e))?
        .to_rgba8();

    let width = img.width();
    let height = img.height();
    let pixel_count = (width * height) as usize;

    // For standard RGB images, we have 3 bands
    let num_bands = 3usize;
    // Calculate layers: ceil(num_bands / BANDS_PER_LAYER), min MIN_TEXTURE_LAYERS
    let num_layers = ((num_bands + BANDS_PER_LAYER - 1) / BANDS_PER_LAYER)
        .max(MIN_TEXTURE_LAYERS as usize) as u32;

    // Pack bands into RGBA layers
    // Layer 0: R, G, B, 0
    // Layer 1: 0, 0, 0, 0 (padding for WebGL2 workaround)
    let mut layers = Vec::with_capacity(num_layers as usize);

    for layer_idx in 0..num_layers {
        let mut rgba_data = vec![0u8; pixel_count * 4];

        // For layer 0, pack R, G, B into channels 0, 1, 2
        if layer_idx == 0 {
            for (pixel_idx, pixel) in img.pixels().enumerate() {
                rgba_data[pixel_idx * 4] = pixel[0]; // R
                rgba_data[pixel_idx * 4 + 1] = pixel[1]; // G
                rgba_data[pixel_idx * 4 + 2] = pixel[2]; // B
                // Channel 3 (A) stays 0 - we only have 3 bands
            }
        }
        // Layer 1+ are padding (all zeros) for the WebGL2 workaround
        // The zeros are already set by vec![0u8; ...]

        layers.push(rgba_data);
    }

    web_sys::console::log_1(
        &format!(
            "[Worker] Packed {}x{} image into {} layers ({} bands)",
            width, height, num_layers, num_bands
        )
        .into(),
    );

    Ok(DecodeResult {
        width,
        height,
        num_bands,
        num_layers,
        layers,
    })
}

/// Send successful decode result back to main thread
#[cfg(target_arch = "wasm32")]
fn send_success(id: u32, name: &str, result: DecodeResult) {
    let global: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();

    let response = Object::new();
    Reflect::set(&response, &"id".into(), &id.into()).unwrap();
    Reflect::set(&response, &"name".into(), &name.into()).unwrap();
    Reflect::set(&response, &"width".into(), &result.width.into()).unwrap();
    Reflect::set(&response, &"height".into(), &result.height.into()).unwrap();
    Reflect::set(
        &response,
        &"num_bands".into(),
        &(result.num_bands as u32).into(),
    )
    .unwrap();
    Reflect::set(&response, &"num_layers".into(), &result.num_layers.into()).unwrap();

    // Convert pre-packed RGBA layers to JS Uint8Arrays for efficient transfer
    let layers_array = Array::new();
    for layer in result.layers {
        let u8_array = Uint8Array::from(layer.as_slice());
        layers_array.push(&u8_array);
    }
    Reflect::set(&response, &"layers".into(), &layers_array).unwrap();

    if let Err(e) = global.post_message(&response) {
        web_sys::console::error_1(&format!("[Worker] Failed to send response: {:?}", e).into());
    }

    web_sys::console::log_1(
        &format!(
            "[Worker] Sent {} ({}x{}, {} bands, {} layers)",
            name, result.width, result.height, result.num_bands, result.num_layers
        )
        .into(),
    );
}

/// Send error result back to main thread
#[cfg(target_arch = "wasm32")]
fn send_error(id: u32, name: &str, error: &str) {
    let global: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();

    let response = Object::new();
    Reflect::set(&response, &"id".into(), &id.into()).unwrap();
    Reflect::set(&response, &"name".into(), &name.into()).unwrap();
    Reflect::set(&response, &"error".into(), &error.into()).unwrap();

    if let Err(e) = global.post_message(&response) {
        web_sys::console::error_1(&format!("[Worker] Failed to send error: {:?}", e).into());
    }

    web_sys::console::error_1(&format!("[Worker] Decode error for {}: {}", name, error).into());
}
