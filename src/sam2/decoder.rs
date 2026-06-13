//! SAM2 Decoder for mask generation from prompts.
//!
//! This module provides the decoder that runs ONNX inference
//! to generate segmentation masks from user prompts (points, boxes).
//!
//! The decoder is lightweight (~20MB) and runs on the main thread
//! for real-time preview during interactive segmentation.

use super::{ImageEmbeddings, SAM2Mask, extract_contour};
use ndarray::{Array1, Array2, Array3, Array4};
use ort::session::{Session, builder::GraphOptimizationLevel};
use ort::value::TensorRef;
use std::path::Path;

/// Simplification epsilon for contour extraction (in pixels).
/// Higher values = fewer vertices, faster but less accurate.
/// Using 1.0 for better contour fidelity with SAM2's high-quality masks.
const CONTOUR_EPSILON: f32 = 1.0;

/// SAM2 decoder for mask generation.
///
/// Loads the ONNX decoder model and provides inference methods
/// to generate masks from image embeddings and user prompts.
pub struct SAM2Decoder {
    session: Session,
}

impl SAM2Decoder {
    /// Loads the SAM2 decoder from an ONNX model file.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to the decoder ONNX file
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be loaded.
    pub fn load(model_path: impl AsRef<Path>) -> Result<Self, SAM2DecoderError> {
        log::info!("Loading SAM2 decoder from {:?}", model_path.as_ref());

        let session = Session::builder()
            .map_err(|e| SAM2DecoderError::ModelLoad(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| SAM2DecoderError::ModelLoad(e.to_string()))?
            .with_intra_threads(4)
            .map_err(|e| SAM2DecoderError::ModelLoad(e.to_string()))?
            .commit_from_file(model_path.as_ref())
            .map_err(|e| SAM2DecoderError::ModelLoad(e.to_string()))?;

        log::info!("SAM2 decoder loaded successfully");
        Ok(Self { session })
    }

    /// Runs inference to generate a mask from prompts.
    ///
    /// # Arguments
    ///
    /// * `embeddings` - Image embeddings from the encoder
    /// * `positive_points` - Foreground points in image coordinates
    /// * `negative_points` - Background points in image coordinates
    /// * `bounding_box` - Optional bounding box (x, y, width, height)
    /// * `previous_mask` - Optional previous mask for refinement
    ///
    /// # Returns
    ///
    /// A `SAM2Mask` containing the predicted segmentation mask.
    pub fn decode(
        &mut self,
        embeddings: &ImageEmbeddings,
        positive_points: &[(f32, f32)],
        negative_points: &[(f32, f32)],
        bounding_box: Option<(f32, f32, f32, f32)>,
        previous_mask: Option<&SAM2Mask>,
    ) -> Result<SAM2Mask, SAM2DecoderError> {
        let (img_w, img_h) = embeddings.original_size;

        // Prepare point inputs
        let (point_coords, point_labels) =
            self.prepare_points(positive_points, negative_points, bounding_box, img_w, img_h);

        // Prepare mask input (optional previous mask)
        let (mask_input, has_mask_input) = self.prepare_mask_input(previous_mask);

        // Get embeddings as arrays
        let image_embed = self.embeddings_to_array(embeddings)?;

        // Get high-resolution features from embeddings (critical for mask quality)
        let high_res_feats_0 = self.high_res_feats_to_array(
            &embeddings.high_res_feats_0,
            embeddings.high_res_feats_0_shape,
        )?;
        let high_res_feats_1 = self.high_res_feats_to_array(
            &embeddings.high_res_feats_1,
            embeddings.high_res_feats_1_shape,
        )?;

        log::info!(
            "SAM2 decode: {} positive, {} negative points, embeddings shape {:?}",
            positive_points.len(),
            negative_points.len(),
            embeddings.shape
        );
        log::info!(
            "SAM2 high_res_feats: 0={:?}, 1={:?}",
            embeddings.high_res_feats_0_shape,
            embeddings.high_res_feats_1_shape
        );

        // Convert arrays to TensorRef for ort
        let image_embed_tensor = TensorRef::from_array_view(image_embed.view())
            .map_err(|e| SAM2DecoderError::Inference(format!("image_embed tensor: {}", e)))?;
        let high_res_0_tensor = TensorRef::from_array_view(high_res_feats_0.view())
            .map_err(|e| SAM2DecoderError::Inference(format!("high_res_0 tensor: {}", e)))?;
        let high_res_1_tensor = TensorRef::from_array_view(high_res_feats_1.view())
            .map_err(|e| SAM2DecoderError::Inference(format!("high_res_1 tensor: {}", e)))?;
        let point_coords_tensor = TensorRef::from_array_view(point_coords.view())
            .map_err(|e| SAM2DecoderError::Inference(format!("point_coords tensor: {}", e)))?;
        let point_labels_tensor = TensorRef::from_array_view(point_labels.view())
            .map_err(|e| SAM2DecoderError::Inference(format!("point_labels tensor: {}", e)))?;
        let mask_input_tensor = TensorRef::from_array_view(mask_input.view())
            .map_err(|e| SAM2DecoderError::Inference(format!("mask_input tensor: {}", e)))?;
        let has_mask_tensor = TensorRef::from_array_view(has_mask_input.view())
            .map_err(|e| SAM2DecoderError::Inference(format!("has_mask tensor: {}", e)))?;

        // Run inference
        let outputs = self
            .session
            .run(ort::inputs![
                "image_embed" => image_embed_tensor,
                "high_res_feats_0" => high_res_0_tensor,
                "high_res_feats_1" => high_res_1_tensor,
                "point_coords" => point_coords_tensor,
                "point_labels" => point_labels_tensor,
                "mask_input" => mask_input_tensor,
                "has_mask_input" => has_mask_tensor,
            ])
            .map_err(|e| SAM2DecoderError::Inference(e.to_string()))?;

        // Extract mask output
        let masks_output = &outputs["masks"];
        let iou_output = &outputs["iou_predictions"];

        // Log the actual shapes
        if let Ok(masks_arr) = masks_output.try_extract_array::<f32>() {
            log::info!("SAM2 decoder masks shape: {:?}", masks_arr.shape());
        }
        if let Ok(iou_arr) = iou_output.try_extract_array::<f32>() {
            log::info!(
                "SAM2 decoder iou_predictions shape: {:?}, values: {:?}",
                iou_arr.shape(),
                iou_arr.iter().copied().collect::<Vec<_>>()
            );
        }

        // Get best mask (highest IoU) - extract data while outputs is still valid
        let (mask_data, score) = Self::extract_best_mask(masks_output, iou_output, img_w, img_h)?;

        // Create mask and extract contour
        let mut mask = SAM2Mask::new(mask_data, img_w, img_h, score);
        mask.contour = extract_contour(&mask, CONTOUR_EPSILON);

        log::debug!(
            "SAM2 mask: {}x{}, score={:.2}, {} contour vertices",
            img_w,
            img_h,
            score,
            mask.contour.len()
        );

        Ok(mask)
    }

    /// Prepares point coordinates and labels for the decoder.
    ///
    /// SAM2 uses ResizeLongestSide(1024) which scales both dimensions uniformly
    /// such that the longest side becomes 1024. This is different from stretching
    /// to fit 1024x1024. We must always include a padding point with label -1.
    fn prepare_points(
        &self,
        positive_points: &[(f32, f32)],
        negative_points: &[(f32, f32)],
        bounding_box: Option<(f32, f32, f32, f32)>,
        img_w: u32,
        img_h: u32,
    ) -> (Array3<f32>, Array2<f32>) {
        // Calculate total points: positive + negative + box corners (if any) + 1 padding point
        let box_points = if bounding_box.is_some() { 2 } else { 0 };
        let actual_points = positive_points.len() + negative_points.len() + box_points;

        // Always include a padding point (required by SAM2 ONNX format)
        let total_points = actual_points.max(1) + 1;

        // Create arrays [1, num_points, 2] and [1, num_points]
        let mut coords = Array3::<f32>::zeros((1, total_points, 2));
        let mut labels = Array2::<f32>::zeros((1, total_points));

        // SAM2 uses ResizeLongestSide(1024) - scale uniformly based on longest side
        // This maintains aspect ratio rather than stretching to 1024x1024
        let longest_side = img_w.max(img_h) as f32;
        let scale = 1024.0 / longest_side;

        log::debug!(
            "SAM2 prepare_points: img={}x{}, longest_side={}, scale={:.4}",
            img_w,
            img_h,
            longest_side,
            scale
        );

        let mut idx = 0;

        // Add positive points (label = 1)
        for (x, y) in positive_points {
            coords[[0, idx, 0]] = x * scale;
            coords[[0, idx, 1]] = y * scale;
            labels[[0, idx]] = 1.0;
            log::debug!(
                "  Positive point {}: ({:.1}, {:.1}) -> ({:.1}, {:.1})",
                idx,
                x,
                y,
                x * scale,
                y * scale
            );
            idx += 1;
        }

        // Add negative points (label = 0)
        for (x, y) in negative_points {
            coords[[0, idx, 0]] = x * scale;
            coords[[0, idx, 1]] = y * scale;
            labels[[0, idx]] = 0.0;
            log::debug!(
                "  Negative point {}: ({:.1}, {:.1}) -> ({:.1}, {:.1})",
                idx,
                x,
                y,
                x * scale,
                y * scale
            );
            idx += 1;
        }

        // Add bounding box corners (labels 2 and 3 for SAM2)
        if let Some((bx, by, bw, bh)) = bounding_box {
            // Top-left corner (label = 2)
            coords[[0, idx, 0]] = bx * scale;
            coords[[0, idx, 1]] = by * scale;
            labels[[0, idx]] = 2.0;
            log::debug!(
                "  Box top-left {}: ({:.1}, {:.1}) -> ({:.1}, {:.1})",
                idx,
                bx,
                by,
                bx * scale,
                by * scale
            );
            idx += 1;

            // Bottom-right corner (label = 3)
            let br_x = bx + bw;
            let br_y = by + bh;
            coords[[0, idx, 0]] = br_x * scale;
            coords[[0, idx, 1]] = br_y * scale;
            labels[[0, idx]] = 3.0;
            log::debug!(
                "  Box bottom-right {}: ({:.1}, {:.1}) -> ({:.1}, {:.1})",
                idx,
                br_x,
                br_y,
                br_x * scale,
                br_y * scale
            );
            idx += 1;
        }

        // If no actual prompts, add a dummy ignored point before padding
        if actual_points == 0 {
            coords[[0, 0, 0]] = 512.0;
            coords[[0, 0, 1]] = 512.0;
            labels[[0, 0]] = -1.0; // Ignored point
            idx = 1;
        }

        // Always add padding point at the end (label = -1)
        coords[[0, idx, 0]] = 0.0;
        coords[[0, idx, 1]] = 0.0;
        labels[[0, idx]] = -1.0;

        log::debug!(
            "  Padding point {}: (0.0, 0.0), label=-1. Total points: {}",
            idx,
            total_points
        );

        (coords, labels)
    }

    /// Prepares mask input for iterative refinement.
    /// Returns (mask_input [1,1,256,256], has_mask_input [1])
    fn prepare_mask_input(&self, previous_mask: Option<&SAM2Mask>) -> (Array4<f32>, Array1<f32>) {
        if let Some(mask) = previous_mask {
            // Resize mask to 256x256 for decoder input
            let mut mask_input = Array4::<f32>::zeros((1, 1, 256, 256));

            let scale_x = mask.width as f32 / 256.0;
            let scale_y = mask.height as f32 / 256.0;

            for y in 0..256 {
                for x in 0..256 {
                    let src_x = (x as f32 * scale_x) as u32;
                    let src_y = (y as f32 * scale_y) as u32;
                    let val = mask.get(src_x, src_y);
                    mask_input[[0, 0, y, x]] = if val > 127 { 1.0 } else { 0.0 };
                }
            }

            // has_mask_input is rank 1 with shape [1]
            let has_mask = Array1::from_elem(1, 1.0f32);
            (mask_input, has_mask)
        } else {
            let mask_input = Array4::<f32>::zeros((1, 1, 256, 256));
            // has_mask_input is rank 1 with shape [1]
            let has_mask = Array1::from_elem(1, 0.0f32);
            (mask_input, has_mask)
        }
    }

    /// Converts ImageEmbeddings to ndarray Array4.
    fn embeddings_to_array(
        &self,
        embeddings: &ImageEmbeddings,
    ) -> Result<Array4<f32>, SAM2DecoderError> {
        let [b, c, h, w] = embeddings.shape;
        Array4::from_shape_vec((b, c, h, w), embeddings.data.clone())
            .map_err(|e| SAM2DecoderError::Inference(format!("Invalid embeddings shape: {}", e)))
    }

    /// Converts high-resolution features to ndarray Array4.
    fn high_res_feats_to_array(
        &self,
        data: &[f32],
        shape: [usize; 4],
    ) -> Result<Array4<f32>, SAM2DecoderError> {
        let [b, c, h, w] = shape;
        Array4::from_shape_vec((b, c, h, w), data.to_vec()).map_err(|e| {
            SAM2DecoderError::Inference(format!("Invalid high_res_feats shape: {}", e))
        })
    }

    /// Extracts the best mask from decoder output.
    ///
    /// The decoder outputs a mask in the 1024x1024 coordinate space (downsampled to 256x256).
    /// Since we use ResizeLongestSide(1024), only a portion of this mask contains valid data.
    /// We need to extract just the valid region and scale it back to original dimensions.
    fn extract_best_mask(
        masks_output: &ort::value::Value,
        iou_output: &ort::value::Value,
        img_w: u32,
        img_h: u32,
    ) -> Result<(Vec<u8>, f32), SAM2DecoderError> {
        // Extract IoU predictions to find best mask
        let iou_array = iou_output
            .try_extract_array::<f32>()
            .map_err(|e| SAM2DecoderError::Inference(format!("Failed to extract IoU: {}", e)))?;

        let iou_data: Vec<f32> = iou_array.iter().copied().collect();

        // Find index of best mask
        let best_idx = iou_data
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        let best_score = iou_data.get(best_idx).copied().unwrap_or(0.0);

        // Extract masks tensor
        let masks_array = masks_output
            .try_extract_array::<f32>()
            .map_err(|e| SAM2DecoderError::Inference(format!("Failed to extract masks: {}", e)))?;

        // Get mask shape [batch, num_masks, H, W]
        let mask_shape = masks_array.shape();
        if mask_shape.len() < 4 {
            return Err(SAM2DecoderError::Inference(format!(
                "Unexpected mask shape: {:?}",
                mask_shape
            )));
        }

        let mask_h = mask_shape[2];
        let mask_w = mask_shape[3];

        // Calculate the valid region in the mask.
        // ResizeLongestSide(1024) scales the image so longest side = 1024.
        // The mask is at 256x256 (1/4 of 1024x1024).
        // The valid region in the mask corresponds to the scaled image dimensions.
        let longest_side = img_w.max(img_h) as f32;
        let scale = 1024.0 / longest_side;

        // Dimensions of the image in the 1024x1024 space
        let scaled_w = (img_w as f32 * scale).round();
        let scaled_h = (img_h as f32 * scale).round();

        // Dimensions of the valid region in the mask (256x256 is 1/4 of 1024x1024)
        let mask_scale = mask_w as f32 / 1024.0;
        let valid_mask_w = (scaled_w * mask_scale).round() as usize;
        let valid_mask_h = (scaled_h * mask_scale).round() as usize;

        log::debug!(
            "SAM2 extract_best_mask: img={}x{}, scale={:.4}, scaled={}x{}, mask={}x{}, valid_mask={}x{}",
            img_w,
            img_h,
            scale,
            scaled_w,
            scaled_h,
            mask_w,
            mask_h,
            valid_mask_w,
            valid_mask_h
        );

        // Resize mask from valid region to original image dimensions using bilinear interpolation
        let mut output = vec![0u8; (img_w * img_h) as usize];

        // Scale factors for mapping output coords to mask coords (within valid region)
        let scale_x = (valid_mask_w.saturating_sub(1)) as f32 / (img_w - 1).max(1) as f32;
        let scale_y = (valid_mask_h.saturating_sub(1)) as f32 / (img_h - 1).max(1) as f32;

        // Clamp bounds for valid mask region (don't go outside valid_mask_w/h)
        let max_mask_x = valid_mask_w.saturating_sub(1).min(mask_w - 1);
        let max_mask_y = valid_mask_h.saturating_sub(1).min(mask_h - 1);

        // Access using dynamic indexing since we have IxDyn shape
        for y in 0..img_h {
            for x in 0..img_w {
                // Map to mask coordinates within valid region
                let src_x = x as f32 * scale_x;
                let src_y = y as f32 * scale_y;

                // Bilinear interpolation (clamped to valid region)
                let x0 = (src_x.floor() as usize).min(max_mask_x);
                let y0 = (src_y.floor() as usize).min(max_mask_y);
                let x1 = (x0 + 1).min(max_mask_x);
                let y1 = (y0 + 1).min(max_mask_y);

                let fx = src_x - x0 as f32;
                let fy = src_y - y0 as f32;

                // Sample the 4 corners from the valid region of the mask
                let v00 = masks_array[ndarray::IxDyn(&[0, best_idx, y0, x0])];
                let v01 = masks_array[ndarray::IxDyn(&[0, best_idx, y0, x1])];
                let v10 = masks_array[ndarray::IxDyn(&[0, best_idx, y1, x0])];
                let v11 = masks_array[ndarray::IxDyn(&[0, best_idx, y1, x1])];

                // Bilinear blend
                let val = v00 * (1.0 - fx) * (1.0 - fy)
                    + v01 * fx * (1.0 - fy)
                    + v10 * (1.0 - fx) * fy
                    + v11 * fx * fy;

                // Threshold the interpolated logit value
                output[(y * img_w + x) as usize] = if val > 0.0 { 255 } else { 0 };
            }
        }

        Ok((output, best_score))
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

#[cfg(test)]
mod tests {
    // Tests require loading actual ONNX model which is expensive
    // Integration tests should be in a separate file with proper setup
}
