//! SAM2 Encoder for computing image embeddings.
//!
//! This module provides the encoder that runs ONNX inference
//! to compute image embeddings from RGB input images.
//!
//! The encoder is compute-intensive (~134MB model) and should
//! ideally run in a background thread or web worker.

use super::ImageEmbeddings;
use ndarray::Array4;
use ort::session::{Session, builder::GraphOptimizationLevel};
use ort::value::TensorRef;
use std::path::Path;

/// SAM2 encoder input size (model expects 1024x1024 RGB).
pub const ENCODER_INPUT_SIZE: u32 = 1024;

/// SAM2 encoder for computing image embeddings.
///
/// Loads the ONNX encoder model and provides inference methods
/// to compute embeddings from images.
pub struct SAM2Encoder {
    session: Session,
}

impl SAM2Encoder {
    /// Loads the SAM2 encoder from an ONNX model file.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to the encoder ONNX file
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be loaded.
    pub fn load(model_path: impl AsRef<Path>) -> Result<Self, SAM2EncoderError> {
        log::info!("Loading SAM2 encoder from {:?}", model_path.as_ref());

        let session = Session::builder()
            .map_err(|e| SAM2EncoderError::ModelLoad(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| SAM2EncoderError::ModelLoad(e.to_string()))?
            .with_intra_threads(4)
            .map_err(|e| SAM2EncoderError::ModelLoad(e.to_string()))?
            .commit_from_file(model_path.as_ref())
            .map_err(|e| SAM2EncoderError::ModelLoad(e.to_string()))?;

        log::info!("SAM2 encoder loaded successfully");
        Ok(Self { session })
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
    pub fn encode(
        &mut self,
        image_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<ImageEmbeddings, SAM2EncoderError> {
        log::info!("SAM2 encoding image {}x{}", width, height);

        // Preprocess: resize to 1024x1024 and normalize
        let input = self.preprocess(image_data, width, height)?;

        // Create tensor
        let input_tensor = TensorRef::from_array_view(input.view())
            .map_err(|e| SAM2EncoderError::Inference(format!("input tensor: {}", e)))?;

        // Run inference
        let start = std::time::Instant::now();
        let outputs = self
            .session
            .run(ort::inputs!["image" => input_tensor])
            .map_err(|e| SAM2EncoderError::Inference(e.to_string()))?;

        let elapsed = start.elapsed();
        log::info!("SAM2 encoder inference took {:?}", elapsed);

        // Extract outputs - all three are needed for high-quality masks
        let image_embed = Self::extract_embedding(&outputs, "image_embed")?;
        let high_res_feats_0 = Self::extract_embedding(&outputs, "high_res_feats_0")?;
        let high_res_feats_1 = Self::extract_embedding(&outputs, "high_res_feats_1")?;

        // Get shapes
        let shape = [
            image_embed.shape()[0],
            image_embed.shape()[1],
            image_embed.shape()[2],
            image_embed.shape()[3],
        ];
        let high_res_0_shape = [
            high_res_feats_0.shape()[0],
            high_res_feats_0.shape()[1],
            high_res_feats_0.shape()[2],
            high_res_feats_0.shape()[3],
        ];
        let high_res_1_shape = [
            high_res_feats_1.shape()[0],
            high_res_feats_1.shape()[1],
            high_res_feats_1.shape()[2],
            high_res_feats_1.shape()[3],
        ];

        log::info!(
            "SAM2 encoder output shapes: image_embed {:?}, high_res_0 {:?}, high_res_1 {:?}",
            shape,
            high_res_0_shape,
            high_res_1_shape
        );

        // into_raw_vec_and_offset returns (vec, offset) - offset is always 0 for owned arrays
        let (data, _) = image_embed.into_raw_vec_and_offset();
        let (high_res_0_data, _) = high_res_feats_0.into_raw_vec_and_offset();
        let (high_res_1_data, _) = high_res_feats_1.into_raw_vec_and_offset();

        Ok(ImageEmbeddings::new(
            data,
            shape,
            high_res_0_data,
            high_res_0_shape,
            high_res_1_data,
            high_res_1_shape,
            (width, height),
        ))
    }

    /// Preprocesses an image for the encoder.
    ///
    /// Uses ResizeLongestSide(1024) transformation:
    /// - Scales image so longest side is 1024 (maintains aspect ratio)
    /// - Pads the shorter dimension with normalized zeros
    /// - Converts to float [0, 1] and normalizes with ImageNet mean/std
    /// - Converts to NCHW format
    fn preprocess(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Array4<f32>, SAM2EncoderError> {
        let expected_size = (width * height * 3) as usize;
        if image_data.len() != expected_size {
            return Err(SAM2EncoderError::Inference(format!(
                "Image data size mismatch: expected {} bytes, got {}",
                expected_size,
                image_data.len()
            )));
        }

        // ImageNet normalization constants
        let mean = [0.485, 0.456, 0.406];
        let std = [0.229, 0.224, 0.225];

        // Initialize output with padding value (normalized zero)
        // For ImageNet normalization, (0 - mean) / std gives the padding value
        let mut output = Array4::<f32>::zeros((
            1,
            3,
            ENCODER_INPUT_SIZE as usize,
            ENCODER_INPUT_SIZE as usize,
        ));

        // Fill with normalized zero (padding value)
        for c in 0..3 {
            let pad_value = (0.0 - mean[c]) / std[c];
            for y in 0..ENCODER_INPUT_SIZE as usize {
                for x in 0..ENCODER_INPUT_SIZE as usize {
                    output[[0, c, y, x]] = pad_value;
                }
            }
        }

        // ResizeLongestSide: scale so longest side becomes 1024
        let longest_side = width.max(height) as f32;
        let scale = ENCODER_INPUT_SIZE as f32 / longest_side;

        // Calculate new dimensions after scaling
        let new_width = (width as f32 * scale).round() as u32;
        let new_height = (height as f32 * scale).round() as u32;

        log::debug!(
            "SAM2 preprocess: {}x{} -> {}x{} (scale={:.4}, longest_side={})",
            width,
            height,
            new_width,
            new_height,
            scale,
            longest_side
        );

        // Scale factor for sampling from source image
        // We map output pixels [0, new_width) to source [0, width)
        let inv_scale = 1.0 / scale;

        // Resize and copy to top-left corner (no centering, matches SAM2 behavior)
        for y in 0..new_height.min(ENCODER_INPUT_SIZE) {
            for x in 0..new_width.min(ENCODER_INPUT_SIZE) {
                // Map back to source coordinates
                let src_x = ((x as f32 * inv_scale) as u32).min(width - 1);
                let src_y = ((y as f32 * inv_scale) as u32).min(height - 1);
                let src_idx = ((src_y * width + src_x) * 3) as usize;

                for c in 0..3 {
                    let pixel = image_data[src_idx + c] as f32 / 255.0;
                    let normalized = (pixel - mean[c]) / std[c];
                    output[[0, c, y as usize, x as usize]] = normalized;
                }
            }
        }

        Ok(output)
    }

    /// Extracts an embedding array from outputs.
    fn extract_embedding(
        outputs: &ort::session::SessionOutputs<'_>,
        name: &str,
    ) -> Result<Array4<f32>, SAM2EncoderError> {
        let value = outputs
            .get(name)
            .ok_or_else(|| SAM2EncoderError::Inference(format!("Missing output: {}", name)))?;

        let array = value.try_extract_array::<f32>().map_err(|e| {
            SAM2EncoderError::Inference(format!("Failed to extract {}: {}", name, e))
        })?;

        // Convert dynamic array to Array4
        let shape = array.shape();
        if shape.len() != 4 {
            return Err(SAM2EncoderError::Inference(format!(
                "Expected 4D tensor for {}, got {}D",
                name,
                shape.len()
            )));
        }

        let data: Vec<f32> = array.iter().copied().collect();
        Array4::from_shape_vec((shape[0], shape[1], shape[2], shape[3]), data)
            .map_err(|e| SAM2EncoderError::Inference(format!("Shape conversion failed: {}", e)))
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
