//! Chunked GPU upload queue for WASM.
//!
//! Queues texture layer uploads and processes them in small chunks during the main
//! tick loop. This spreads GPU work across frames to avoid lag spikes during preloading.
//!
//! Architecture:
//! 1. Worker decodes image AND packs into RGBA layers (off main thread)
//! 2. Main thread receives pre-packed layers via postMessage
//! 3. Main thread creates GPU texture (allocation only, fast)
//! 4. Each tick: upload CHUNK of rows to GPU (configurable blocking duration)
//! 5. When all layers done: move texture to cache
//!
//! The chunk size is controlled by `GPU_UPLOAD_ROWS_PER_TICK` in constants.rs.

use std::collections::VecDeque;
use std::path::PathBuf;

use hvat_gpu::GpuContext;

use crate::constants::{BANDS_PER_LAYER, GPU_UPLOAD_ROWS_PER_TICK, MIN_TEXTURE_LAYERS};

/// Calculate the number of texture layers needed for a given number of bands.
///
/// Packs bands into RGBA texture layers (4 bands per layer).
/// Ensures at least `MIN_TEXTURE_LAYERS` for WebGL2 compatibility.
pub fn calculate_num_layers(num_bands: usize) -> u32 {
    ((num_bands + BANDS_PER_LAYER - 1) / BANDS_PER_LAYER).max(MIN_TEXTURE_LAYERS as usize) as u32
}

/// Create a GPU texture for band data with the given dimensions and layers.
///
/// This is a helper to avoid duplicating the TextureDescriptor boilerplate.
fn create_band_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    num_layers: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Band Texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: num_layers,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

/// A single texture layer with row-based chunking support.
pub struct PendingLayer {
    /// RGBA pixel data (width * height * 4 bytes)
    pub rgba_data: Vec<u8>,
    /// Layer index in the texture array
    pub layer_index: u32,
    /// Next row to upload (for chunked uploads within a layer)
    pub next_row: u32,
}

/// An image being uploaded in chunks.
pub struct ChunkedUpload {
    /// Path for cache key
    pub path: PathBuf,
    /// Image dimensions
    pub width: u32,
    pub height: u32,
    /// Number of bands
    pub num_bands: usize,
    /// Total number of texture layers
    pub num_layers: u32,
    /// Layers waiting to be uploaded
    pub pending_layers: VecDeque<PendingLayer>,
    /// GPU texture (created upfront, layers uploaded incrementally)
    pub texture: wgpu::Texture,
}

/// Result of a completed chunked upload, ready for cache insertion.
pub struct CompletedChunkedUpload {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub num_bands: usize,
    pub num_layers: u32,
    pub texture: wgpu::Texture,
}

/// Queue for chunked GPU uploads.
///
/// Call `queue_upload` to add decoded images, then call `process_one_layer`
/// each tick to upload layers incrementally.
pub struct ChunkedUploadQueue {
    /// Queue of images being uploaded
    uploads: VecDeque<ChunkedUpload>,
    /// Completed uploads ready for cache
    completed: Vec<CompletedChunkedUpload>,
}

impl ChunkedUploadQueue {
    /// Create a new empty queue.
    pub fn new() -> Self {
        Self {
            uploads: VecDeque::new(),
            completed: Vec::new(),
        }
    }

    /// Queue pre-packed layers for chunked upload.
    ///
    /// The RGBA packing has already been done by the worker thread.
    /// This just creates the GPU texture and queues the layers for upload.
    /// Actual layer uploads happen via `process_one_layer`.
    pub fn queue_prepacked(
        &mut self,
        path: PathBuf,
        width: u32,
        height: u32,
        num_bands: usize,
        num_layers: u32,
        layers: Vec<super::preload_worker::PackedLayer>,
        device: &wgpu::Device,
    ) {
        log::info!(
            "Queueing pre-packed upload for {:?}: {}x{}, {} bands, {} layers",
            path,
            width,
            height,
            num_bands,
            num_layers
        );

        // Convert pre-packed layers to pending layers
        let pending_layers: VecDeque<PendingLayer> = layers
            .into_iter()
            .map(|packed| PendingLayer {
                rgba_data: packed.rgba_data,
                layer_index: packed.layer_index,
                next_row: 0,
            })
            .collect();

        // Create the texture upfront (fast, just allocation)
        let texture = create_band_texture(device, width, height, num_layers);

        self.uploads.push_back(ChunkedUpload {
            path,
            width,
            height,
            num_bands,
            num_layers,
            pending_layers,
            texture,
        });
    }

    /// Queue decoded image data for chunked upload (legacy - does RGBA packing on main thread).
    ///
    /// Pre-packs band data into RGBA layers immediately (CPU work).
    /// Creates the GPU texture (allocation only).
    /// Actual layer uploads happen via `process_one_layer`.
    ///
    /// NOTE: Prefer `queue_prepacked` when the worker has already done the RGBA packing.
    #[allow(dead_code)]
    pub fn queue_upload(
        &mut self,
        path: PathBuf,
        bands: Vec<Vec<f32>>,
        width: u32,
        height: u32,
        device: &wgpu::Device,
    ) {
        let num_bands = bands.len();
        let num_layers = calculate_num_layers(num_bands);
        let pixel_count = (width * height) as usize;

        log::info!(
            "Queueing chunked upload for {:?}: {}x{}, {} bands → {} layers",
            path,
            width,
            height,
            num_bands,
            num_layers
        );

        // Pre-pack all layers into RGBA format (CPU work, done now)
        let mut pending_layers = VecDeque::with_capacity(num_layers as usize);
        for layer_idx in 0..num_layers {
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

            pending_layers.push_back(PendingLayer {
                rgba_data,
                layer_index: layer_idx,
                next_row: 0,
            });
        }

        // Create the texture upfront (fast, just allocation)
        let texture = create_band_texture(device, width, height, num_layers);

        self.uploads.push_back(ChunkedUpload {
            path,
            width,
            height,
            num_bands,
            num_layers,
            pending_layers,
            texture,
        });
    }

    /// Process one chunk of rows from the current layer.
    ///
    /// Returns `true` if work was done (rows were uploaded).
    /// Call this once per tick to spread GPU work across frames.
    ///
    /// The number of rows uploaded per call is controlled by `GPU_UPLOAD_ROWS_PER_TICK`.
    /// Set to 0 in constants.rs to upload entire layers at once (old behavior).
    pub fn process_one_layer(&mut self, gpu_ctx: &GpuContext) -> bool {
        let Some(upload) = self.uploads.front_mut() else {
            return false;
        };

        let Some(layer) = upload.pending_layers.front_mut() else {
            // No more layers, shouldn't happen but handle gracefully
            self.uploads.pop_front();
            return false;
        };

        let height = upload.height;
        let width = upload.width;
        let start_row = layer.next_row;

        // Determine how many rows to upload this tick
        let rows_to_upload = if GPU_UPLOAD_ROWS_PER_TICK == 0 {
            // Upload entire layer at once (old behavior)
            height - start_row
        } else {
            GPU_UPLOAD_ROWS_PER_TICK.min(height - start_row)
        };

        if rows_to_upload == 0 {
            // Layer complete, remove it
            upload.pending_layers.pop_front();

            // Check if upload is complete
            if upload.pending_layers.is_empty() {
                log::info!(
                    "Chunked upload complete for {:?} ({} layers)",
                    upload.path,
                    upload.num_layers
                );

                let finished = self.uploads.pop_front().unwrap();
                self.completed.push(CompletedChunkedUpload {
                    path: finished.path,
                    width: finished.width,
                    height: finished.height,
                    num_bands: finished.num_bands,
                    num_layers: finished.num_layers,
                    texture: finished.texture,
                });
            }
            return true;
        }

        // Calculate byte offsets for the row range
        let bytes_per_row = 4 * width;
        let start_byte = (start_row * bytes_per_row) as usize;
        let end_byte = ((start_row + rows_to_upload) * bytes_per_row) as usize;

        log::trace!(
            "Uploading rows {}-{}/{} of layer {}/{} for {:?}",
            start_row,
            start_row + rows_to_upload,
            height,
            layer.layer_index + 1,
            upload.num_layers,
            upload.path
        );

        // Upload this chunk of rows to GPU
        gpu_ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &upload.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: start_row,
                    z: layer.layer_index,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &layer.rgba_data[start_byte..end_byte],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(rows_to_upload),
            },
            wgpu::Extent3d {
                width,
                height: rows_to_upload,
                depth_or_array_layers: 1,
            },
        );

        // Advance to next chunk
        layer.next_row = start_row + rows_to_upload;

        // Check if this layer is now complete
        if layer.next_row >= height {
            upload.pending_layers.pop_front();

            // Check if the entire upload is complete
            if upload.pending_layers.is_empty() {
                log::info!(
                    "Chunked upload complete for {:?} ({} layers)",
                    upload.path,
                    upload.num_layers
                );

                let finished = self.uploads.pop_front().unwrap();
                self.completed.push(CompletedChunkedUpload {
                    path: finished.path,
                    width: finished.width,
                    height: finished.height,
                    num_bands: finished.num_bands,
                    num_layers: finished.num_layers,
                    texture: finished.texture,
                });
            }
        }

        true
    }

    /// Check if there are pending uploads.
    pub fn has_pending(&self) -> bool {
        !self.uploads.is_empty()
    }

    /// Check if there are completed uploads ready for cache.
    pub fn has_completed(&self) -> bool {
        !self.completed.is_empty()
    }

    /// Take all completed uploads.
    pub fn take_completed(&mut self) -> Vec<CompletedChunkedUpload> {
        std::mem::take(&mut self.completed)
    }

    /// Get count of pending uploads.
    #[allow(dead_code)]
    pub fn pending_count(&self) -> usize {
        self.uploads.len()
    }

    /// Get total pending layers across all uploads.
    #[allow(dead_code)]
    pub fn pending_layers_count(&self) -> usize {
        self.uploads.iter().map(|u| u.pending_layers.len()).sum()
    }

    /// Check if a path is already queued or being uploaded.
    pub fn is_queued(&self, path: &PathBuf) -> bool {
        self.uploads.iter().any(|u| &u.path == path)
    }

    /// Clear all pending uploads (e.g., when folder changes).
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        let count = self.uploads.len();
        self.uploads.clear();
        self.completed.clear();
        if count > 0 {
            log::info!("Cleared {} pending chunked uploads", count);
        }
    }
}

impl Default for ChunkedUploadQueue {
    fn default() -> Self {
        Self::new()
    }
}
