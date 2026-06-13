//! SAM2 Decoder for WASM using onnxruntime-web via JavaScript.
//!
//! This module provides the decoder that runs ONNX inference via
//! onnxruntime-web JavaScript library called through wasm-bindgen.
//!
//! The decoder is lightweight (~16.5MB) and runs on the main thread
//! for real-time preview during interactive segmentation.

use super::{ImageEmbeddings, SAM2Mask, extract_contour};
use wasm_bindgen::prelude::*;

/// Simplification epsilon for contour extraction (in pixels).
const CONTOUR_EPSILON: f32 = 1.0;

// JavaScript bindings for SAM2 inference
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = sam2, js_name = runDecoder)]
    async fn js_run_decoder(
        embeddings: JsValue,
        positive_points: JsValue,
        negative_points: JsValue,
        bounding_box: JsValue,
    ) -> JsValue;

    #[wasm_bindgen(js_namespace = sam2, js_name = isReady)]
    fn js_is_ready() -> bool;
}

/// SAM2 decoder for mask generation using onnxruntime-web.
///
/// This is a thin wrapper that delegates to JavaScript for actual inference.
/// The decoder model is loaded and managed by the JavaScript side.
pub struct SAM2Decoder {
    // No internal state - models are managed by JavaScript
    _private: (),
}

impl SAM2Decoder {
    /// Creates a new decoder wrapper.
    ///
    /// Note: This doesn't load the model - that's done via JavaScript.
    /// Use `load_models_js()` to trigger model loading.
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Creates decoder from bytes (for API compatibility).
    ///
    /// Note: This is a no-op for the JS-based implementation.
    /// Models are loaded via JavaScript's `sam2.loadModels()`.
    pub fn load_from_bytes(_model_bytes: &[u8]) -> Result<Self, SAM2DecoderError> {
        // Models are loaded by JavaScript, not Rust
        log::info!("SAM2 decoder: load_from_bytes called (JS-based, no-op)");
        Ok(Self::new())
    }

    /// Checks if the decoder is ready (models loaded in JS).
    pub fn is_ready() -> bool {
        js_is_ready()
    }

    /// Runs inference to generate a mask from prompts (async version).
    ///
    /// # Arguments
    ///
    /// * `embeddings` - Image embeddings from the encoder
    /// * `positive_points` - Foreground points in image coordinates
    /// * `negative_points` - Background points in image coordinates
    /// * `bounding_box` - Optional bounding box (x, y, width, height)
    /// * `_previous_mask` - Optional previous mask for refinement (not yet implemented)
    ///
    /// # Returns
    ///
    /// A `SAM2Mask` containing the predicted segmentation mask.
    pub async fn decode_async(
        &self,
        embeddings: &ImageEmbeddings,
        positive_points: &[(f32, f32)],
        negative_points: &[(f32, f32)],
        bounding_box: Option<(f32, f32, f32, f32)>,
        _previous_mask: Option<&SAM2Mask>,
    ) -> Result<SAM2Mask, SAM2DecoderError> {
        log::info!(
            "SAM2 decode: {} positive, {} negative points (onnxruntime-web)",
            positive_points.len(),
            negative_points.len()
        );

        // Convert embeddings to JS object
        let embeddings_js = Self::embeddings_to_js(embeddings)?;

        // Convert points to JS arrays
        let positive_js = Self::points_to_js(positive_points);
        let negative_js = Self::points_to_js(negative_points);

        // Convert bounding box to JS
        let bbox_js = match bounding_box {
            Some((x, y, w, h)) => {
                let arr = js_sys::Array::new();
                arr.push(&JsValue::from_f64(x as f64));
                arr.push(&JsValue::from_f64(y as f64));
                arr.push(&JsValue::from_f64(w as f64));
                arr.push(&JsValue::from_f64(h as f64));
                arr.into()
            }
            None => JsValue::NULL,
        };

        // Call JavaScript decoder
        let result = js_run_decoder(embeddings_js, positive_js, negative_js, bbox_js).await;

        // Parse result from JavaScript
        Self::parse_decoder_result(result)
    }

    /// Synchronous decode (for API compatibility).
    ///
    /// Note: This returns an error because async is required for JS interop.
    /// Use `decode_async` instead.
    pub fn decode(
        &self,
        _embeddings: &ImageEmbeddings,
        _positive_points: &[(f32, f32)],
        _negative_points: &[(f32, f32)],
        _bounding_box: Option<(f32, f32, f32, f32)>,
        _previous_mask: Option<&SAM2Mask>,
    ) -> Result<SAM2Mask, SAM2DecoderError> {
        Err(SAM2DecoderError::Inference(
            "Use decode_async for WASM builds".to_string(),
        ))
    }

    /// Convert ImageEmbeddings to JS object.
    fn embeddings_to_js(embeddings: &ImageEmbeddings) -> Result<JsValue, SAM2DecoderError> {
        use js_sys::{Array, Object, Reflect};

        let obj = Object::new();

        // Helper to create tensor object
        let create_tensor = |data: &[f32], shape: &[usize; 4]| -> JsValue {
            let tensor_obj = Object::new();

            // Convert data to JS array
            let data_arr = Array::new();
            for &v in data {
                data_arr.push(&JsValue::from_f64(v as f64));
            }
            Reflect::set(&tensor_obj, &JsValue::from_str("data"), &data_arr).ok();

            // Convert shape to JS array
            let shape_arr = Array::new();
            for &s in shape {
                shape_arr.push(&JsValue::from_f64(s as f64));
            }
            Reflect::set(&tensor_obj, &JsValue::from_str("shape"), &shape_arr).ok();

            tensor_obj.into()
        };

        // Set image_embed
        Reflect::set(
            &obj,
            &JsValue::from_str("imageEmbed"),
            &create_tensor(&embeddings.data, &embeddings.shape),
        )
        .map_err(|_| SAM2DecoderError::Inference("Failed to set imageEmbed".to_string()))?;

        // Set high_res_feats_0
        Reflect::set(
            &obj,
            &JsValue::from_str("highResFeats0"),
            &create_tensor(
                &embeddings.high_res_feats_0,
                &embeddings.high_res_feats_0_shape,
            ),
        )
        .map_err(|_| SAM2DecoderError::Inference("Failed to set highResFeats0".to_string()))?;

        // Set high_res_feats_1
        Reflect::set(
            &obj,
            &JsValue::from_str("highResFeats1"),
            &create_tensor(
                &embeddings.high_res_feats_1,
                &embeddings.high_res_feats_1_shape,
            ),
        )
        .map_err(|_| SAM2DecoderError::Inference("Failed to set highResFeats1".to_string()))?;

        // Set original_size
        let size_arr = Array::new();
        size_arr.push(&JsValue::from_f64(embeddings.original_size.0 as f64));
        size_arr.push(&JsValue::from_f64(embeddings.original_size.1 as f64));
        Reflect::set(&obj, &JsValue::from_str("originalSize"), &size_arr)
            .map_err(|_| SAM2DecoderError::Inference("Failed to set originalSize".to_string()))?;

        Ok(obj.into())
    }

    /// Convert points slice to JS array of [x, y] arrays.
    fn points_to_js(points: &[(f32, f32)]) -> JsValue {
        let arr = js_sys::Array::new();
        for (x, y) in points {
            let point = js_sys::Array::new();
            point.push(&JsValue::from_f64(*x as f64));
            point.push(&JsValue::from_f64(*y as f64));
            arr.push(&point);
        }
        arr.into()
    }

    /// Parse the result from JavaScript decoder.
    fn parse_decoder_result(result: JsValue) -> Result<SAM2Mask, SAM2DecoderError> {
        use js_sys::{Array, Reflect};

        // Check if result is an object with success field
        let success = Reflect::get(&result, &JsValue::from_str("success"))
            .map_err(|_| SAM2DecoderError::Inference("Failed to get success field".to_string()))?
            .as_bool()
            .unwrap_or(false);

        if !success {
            let error = Reflect::get(&result, &JsValue::from_str("error"))
                .ok()
                .and_then(|e| e.as_string())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(SAM2DecoderError::Inference(error));
        }

        // Get mask object
        let mask_obj = Reflect::get(&result, &JsValue::from_str("mask"))
            .map_err(|_| SAM2DecoderError::Inference("Failed to get mask".to_string()))?;

        // Extract mask data
        let data_js = Reflect::get(&mask_obj, &JsValue::from_str("data"))
            .map_err(|_| SAM2DecoderError::Inference("Failed to get mask data".to_string()))?;
        let data_array = Array::from(&data_js);
        let data: Vec<u8> = data_array
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as u8)
            .collect();

        // Extract dimensions
        let width = Reflect::get(&mask_obj, &JsValue::from_str("width"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as u32;
        let height = Reflect::get(&mask_obj, &JsValue::from_str("height"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as u32;
        let score = Reflect::get(&mask_obj, &JsValue::from_str("score"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        log::debug!(
            "SAM2 mask: {}x{}, score={:.2}, {} bytes",
            width,
            height,
            score,
            data.len()
        );

        // Create mask and extract contour
        let mut mask = SAM2Mask::new(data, width, height, score);
        mask.contour = extract_contour(&mask, CONTOUR_EPSILON);

        log::debug!("SAM2 mask contour: {} vertices", mask.contour.len());

        Ok(mask)
    }
}

impl Default for SAM2Decoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during SAM2 decoding.
#[derive(Debug, Clone)]
pub enum SAM2DecoderError {
    /// Failed to load the ONNX model.
    ModelLoad(String),
    /// Inference failed.
    Inference(String),
}

impl std::fmt::Display for SAM2DecoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SAM2DecoderError::ModelLoad(msg) => write!(f, "Failed to load SAM2 decoder: {}", msg),
            SAM2DecoderError::Inference(msg) => write!(f, "SAM2 inference failed: {}", msg),
        }
    }
}

impl std::error::Error for SAM2DecoderError {}
