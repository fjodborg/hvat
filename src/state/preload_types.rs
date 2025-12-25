//! Shared types for background preloading.
//!
//! These types are used by both WASM (Web Worker) and native (background thread)
//! preloading implementations to provide a unified interface.

use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use crate::constants::{BANDS_PER_LAYER, MIN_TEXTURE_LAYERS};

/// Calculate the number of texture layers needed for a given number of bands.
///
/// Packs bands into RGBA texture layers (4 bands per layer).
/// Ensures at least `MIN_TEXTURE_LAYERS` for WebGL2 compatibility.
#[cfg(not(target_arch = "wasm32"))]
pub fn calculate_num_layers(num_bands: usize) -> u32 {
    ((num_bands + BANDS_PER_LAYER - 1) / BANDS_PER_LAYER).max(MIN_TEXTURE_LAYERS as usize) as u32
}

/// Pack band data into RGBA texture layers.
///
/// Takes spectral band data (f32 normalized to 0.0-1.0) and packs it into
/// RGBA u8 layers suitable for GPU texture upload.
///
/// # Arguments
/// * `bands` - Slice of band data, each band is a Vec<f32> of pixel values
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `num_layers` - Number of texture layers to create
///
/// # Returns
/// Vector of `PackedLayer` ready for GPU upload.
#[cfg(not(target_arch = "wasm32"))]
pub fn pack_bands_to_layers(
    bands: &[Vec<f32>],
    width: u32,
    height: u32,
    num_layers: u32,
) -> Vec<PackedLayer> {
    let pixel_count = (width * height) as usize;
    let num_bands = bands.len();

    (0..num_layers)
        .map(|layer_idx| {
            let base_band = (layer_idx as usize) * BANDS_PER_LAYER;
            let mut rgba_data = vec![0u8; pixel_count * 4];

            // Pack up to BANDS_PER_LAYER bands into RGBA channels
            for channel_idx in 0..BANDS_PER_LAYER {
                let band_idx = base_band + channel_idx;
                if band_idx >= num_bands {
                    break;
                }

                let band = &bands[band_idx];
                if band.len() != pixel_count {
                    log::warn!(
                        "Band {} has wrong size: {} vs expected {}",
                        band_idx,
                        band.len(),
                        pixel_count
                    );
                    continue;
                }

                for (pixel_idx, &value) in band.iter().enumerate() {
                    let byte_value = (value.clamp(0.0, 1.0) * 255.0) as u8;
                    rgba_data[pixel_idx * 4 + channel_idx] = byte_value;
                }
            }

            PackedLayer {
                rgba_data,
                layer_index: layer_idx,
            }
        })
        .collect()
}

/// Pre-packed RGBA layer data ready for GPU upload.
///
/// Contains pixel data for one texture layer, with bands packed into RGBA channels.
#[derive(Debug)]
pub struct PackedLayer {
    /// RGBA pixel data (width * height * 4 bytes)
    pub rgba_data: Vec<u8>,
    /// Layer index in the texture array
    pub layer_index: u32,
}

/// Decoded and pre-packed image data ready for GPU upload.
///
/// This is the result of background decoding, containing all data needed
/// for chunked GPU upload.
#[derive(Debug)]
pub struct DecodedImage {
    /// Path/name of the decoded image (for cache key)
    pub path: PathBuf,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Number of spectral bands
    pub num_bands: usize,
    /// Number of texture layers (ceil(num_bands / 4), minimum 2)
    pub num_layers: u32,
    /// Pre-packed RGBA layers ready for GPU upload
    pub layers: Vec<PackedLayer>,
}

/// Error result from a decode attempt.
#[derive(Debug)]
pub struct DecodeError {
    /// Path/name of the image that failed
    pub path: PathBuf,
    /// Error message describing the failure
    pub error: String,
}

/// Result from background decoding - either decoded data or an error.
pub enum DecodeResult {
    /// Successfully decoded image
    Decoded(DecodedImage),
    /// Decode failed with error
    Error(DecodeError),
}
