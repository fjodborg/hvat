//! Application state management modules.

mod gpu;
mod gpu_cache;
mod idle_upload;
mod image_data;
#[cfg(not(target_arch = "wasm32"))]
mod native_decoder;
mod preload_types;
#[cfg(target_arch = "wasm32")]
mod preload_worker;
mod project;
mod snapshot;
mod zip_import;

pub use gpu::{GpuRenderState, SharedGpuPipeline};
pub use gpu_cache::{CachedGpuTexture, GpuTextureCache};
pub use idle_upload::ChunkedUploadQueue;
#[allow(unused_imports)]
pub use image_data::ImageData;
pub use image_data::ImageDataStore;
#[cfg(not(target_arch = "wasm32"))]
pub use native_decoder::NativeDecoderThread;
pub use preload_types::DecodeResult;
#[cfg(target_arch = "wasm32")]
pub use preload_worker::ImageDecoderWorker;
#[cfg(target_arch = "wasm32")]
pub use project::is_image_filename;
pub use project::{LoadedImage, ProjectState};
pub use snapshot::{AnnotationState, AppSnapshot};
#[cfg(target_arch = "wasm32")]
pub use zip_import::{extract_images_from_zip_bytes, is_zip_file};
#[cfg(not(target_arch = "wasm32"))]
pub use zip_import::{extract_images_from_zip_file, is_zip_path};

// =============================================================================
// Platform-Specific Preloading State
// =============================================================================

/// WASM preloading state for async image decoding and chunked GPU upload.
///
/// Groups all WASM-specific fields needed for the three-stage async preloading pipeline:
/// 1. Web Worker decodes image + packs into RGBA layers
/// 2. Chunked upload queue spreads GPU uploads across frames
/// 3. Cache insertion when complete
#[cfg(target_arch = "wasm32")]
pub struct WasmPreloadState {
    /// Image decoder worker for background preloading
    pub decoder_worker: Option<ImageDecoderWorker>,
    /// Chunked GPU upload queue for spreading texture uploads across frames
    pub chunked_upload_queue: ChunkedUploadQueue,
}

#[cfg(target_arch = "wasm32")]
impl WasmPreloadState {
    /// Create a new WASM preload state, spawning the decoder worker.
    pub fn new() -> Self {
        Self {
            decoder_worker: ImageDecoderWorker::spawn().ok(),
            chunked_upload_queue: ChunkedUploadQueue::new(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for WasmPreloadState {
    fn default() -> Self {
        Self::new()
    }
}

/// Native preloading state for async image decoding and chunked GPU upload.
///
/// Groups all native-specific fields needed for the three-stage async preloading pipeline:
/// 1. Background thread decodes image + packs into RGBA layers
/// 2. Chunked upload queue spreads GPU uploads across frames
/// 3. Cache insertion when complete
#[cfg(not(target_arch = "wasm32"))]
pub struct NativePreloadState {
    /// Background decoder thread for preloading
    pub decoder: NativeDecoderThread,
    /// Chunked GPU upload queue for spreading texture uploads across frames
    pub chunked_upload_queue: ChunkedUploadQueue,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativePreloadState {
    /// Create a new native preload state, spawning the decoder thread.
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            decoder: NativeDecoderThread::spawn()?,
            chunked_upload_queue: ChunkedUploadQueue::new(),
        })
    }
}
