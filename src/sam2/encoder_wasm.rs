//! SAM2 Encoder for WASM using onnxruntime-web via JavaScript.
//!
//! This module provides the encoder that runs ONNX inference via
//! onnxruntime-web JavaScript library called through wasm-bindgen.
//!
//! The encoder is compute-intensive (~109MB model) and runs on the
//! main thread via onnxruntime-web's WASM backend.

use super::ImageEmbeddings;
use wasm_bindgen::prelude::*;

/// SAM2 encoder input size (model expects 1024x1024 RGB).
pub const ENCODER_INPUT_SIZE: u32 = 1024;

// JavaScript bindings for SAM2 inference
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = sam2, js_name = runEncoder)]
    async fn js_run_encoder(image_data: &[u8], width: u32, height: u32) -> JsValue;

    #[wasm_bindgen(js_namespace = sam2, js_name = isReady)]
    fn js_is_ready() -> bool;
}

/// SAM2 encoder for computing image embeddings using onnxruntime-web.
///
/// This is a thin wrapper that delegates to JavaScript for actual inference.
/// The encoder model is loaded and managed by the JavaScript side.
pub struct SAM2Encoder {
    // No internal state - models are managed by JavaScript
    _private: (),
}

impl SAM2Encoder {
    /// Creates a new encoder wrapper.
    ///
    /// Note: This doesn't load the model - that's done via JavaScript.
    /// Use `load_models_js()` to trigger model loading.
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Creates encoder from bytes (for API compatibility).
    ///
    /// Note: This is a no-op for the JS-based implementation.
    /// Models are loaded via JavaScript's `sam2.loadModels()`.
    pub fn load_from_bytes(_model_bytes: &[u8]) -> Result<Self, SAM2EncoderError> {
        // Models are loaded by JavaScript, not Rust
        log::info!("SAM2 encoder: load_from_bytes called (JS-based, no-op)");
        Ok(Self::new())
    }

    /// Checks if the encoder is ready (models loaded in JS).
    pub fn is_ready() -> bool {
        js_is_ready()
    }

    /// Encodes an image to compute embeddings.
    ///
    /// # Arguments
    ///
    /// * `image_data` - Raw image data as RGB bytes (width * height * 3)
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    ///
    /// # Returns
    ///
    /// `ImageEmbeddings` containing the encoder outputs needed by the decoder.
    pub async fn encode_async(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<ImageEmbeddings, SAM2EncoderError> {
        log::info!("SAM2 encoding image {}x{} (onnxruntime-web)", width, height);

        // Call JavaScript encoder
        let result = js_run_encoder(image_data, width, height).await;

        // Parse result from JavaScript
        Self::parse_encoder_result(result, width, height)
    }

    /// Synchronous encode (for API compatibility).
    ///
    /// Note: This panics because async is required for JS interop.
    /// Use `encode_async` instead.
    pub fn encode(
        &self,
        _image_data: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<ImageEmbeddings, SAM2EncoderError> {
        Err(SAM2EncoderError::Inference(
            "Use encode_async for WASM builds".to_string(),
        ))
    }

    /// Parse the result from JavaScript encoder.
    fn parse_encoder_result(
        result: JsValue,
        width: u32,
        height: u32,
    ) -> Result<ImageEmbeddings, SAM2EncoderError> {
        use js_sys::{Array, Object, Reflect};

        // Check if result is an object with success field
        let success = Reflect::get(&result, &JsValue::from_str("success"))
            .map_err(|_| SAM2EncoderError::Inference("Failed to get success field".to_string()))?
            .as_bool()
            .unwrap_or(false);

        if !success {
            let error = Reflect::get(&result, &JsValue::from_str("error"))
                .ok()
                .and_then(|e| e.as_string())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(SAM2EncoderError::Inference(error));
        }

        // Get embeddings object
        let embeddings = Reflect::get(&result, &JsValue::from_str("embeddings"))
            .map_err(|_| SAM2EncoderError::Inference("Failed to get embeddings".to_string()))?;

        // Extract image_embed
        let image_embed_obj = Reflect::get(&embeddings, &JsValue::from_str("imageEmbed"))
            .map_err(|_| SAM2EncoderError::Inference("Failed to get imageEmbed".to_string()))?;
        let (image_embed, image_embed_shape) = Self::extract_tensor(&image_embed_obj)?;

        // Extract high_res_feats_0
        let high_res_0_obj = Reflect::get(&embeddings, &JsValue::from_str("highResFeats0"))
            .map_err(|_| SAM2EncoderError::Inference("Failed to get highResFeats0".to_string()))?;
        let (high_res_feats_0, high_res_0_shape) = Self::extract_tensor(&high_res_0_obj)?;

        // Extract high_res_feats_1
        let high_res_1_obj = Reflect::get(&embeddings, &JsValue::from_str("highResFeats1"))
            .map_err(|_| SAM2EncoderError::Inference("Failed to get highResFeats1".to_string()))?;
        let (high_res_feats_1, high_res_1_shape) = Self::extract_tensor(&high_res_1_obj)?;

        log::info!(
            "SAM2 encoder output shapes: image_embed {:?}, high_res_0 {:?}, high_res_1 {:?}",
            image_embed_shape,
            high_res_0_shape,
            high_res_1_shape
        );

        Ok(ImageEmbeddings::new(
            image_embed,
            image_embed_shape,
            high_res_feats_0,
            high_res_0_shape,
            high_res_feats_1,
            high_res_1_shape,
            (width, height),
        ))
    }

    /// Extract tensor data and shape from JS object.
    fn extract_tensor(obj: &JsValue) -> Result<(Vec<f32>, [usize; 4]), SAM2EncoderError> {
        use js_sys::{Array, Reflect};

        // Get data array
        let data_js = Reflect::get(obj, &JsValue::from_str("data"))
            .map_err(|_| SAM2EncoderError::Inference("Failed to get tensor data".to_string()))?;
        let data_array = Array::from(&data_js);
        let data: Vec<f32> = data_array
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        // Get shape array
        let shape_js = Reflect::get(obj, &JsValue::from_str("shape"))
            .map_err(|_| SAM2EncoderError::Inference("Failed to get tensor shape".to_string()))?;
        let shape_array = Array::from(&shape_js);
        if shape_array.length() != 4 {
            return Err(SAM2EncoderError::Inference(format!(
                "Expected 4D shape, got {} dimensions",
                shape_array.length()
            )));
        }
        let shape = [
            shape_array.get(0).as_f64().unwrap_or(0.0) as usize,
            shape_array.get(1).as_f64().unwrap_or(0.0) as usize,
            shape_array.get(2).as_f64().unwrap_or(0.0) as usize,
            shape_array.get(3).as_f64().unwrap_or(0.0) as usize,
        ];

        Ok((data, shape))
    }
}

impl Default for SAM2Encoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during SAM2 encoding.
#[derive(Debug, Clone)]
pub enum SAM2EncoderError {
    /// Failed to load the ONNX model.
    ModelLoad(String),
    /// Inference failed.
    Inference(String),
}

impl std::fmt::Display for SAM2EncoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SAM2EncoderError::ModelLoad(msg) => write!(f, "Failed to load SAM2 encoder: {}", msg),
            SAM2EncoderError::Inference(msg) => write!(f, "SAM2 encoding failed: {}", msg),
        }
    }
}

impl std::error::Error for SAM2EncoderError {}
