//! Application state management modules.

mod gpu;
mod gpu_cache;
mod image_data;
mod project;
mod snapshot;
mod zip_import;

pub use gpu::{GpuRenderState, SharedGpuPipeline};
pub use gpu_cache::{CachedGpuTexture, GpuTextureCache};
#[allow(unused_imports)]
pub use image_data::ImageData;
pub use image_data::ImageDataStore;
#[cfg(target_arch = "wasm32")]
pub use project::is_image_filename;
pub use project::{LoadedImage, ProjectState};
pub use snapshot::{AnnotationState, AppSnapshot};
#[cfg(target_arch = "wasm32")]
pub use zip_import::{extract_images_from_zip_bytes, is_zip_file};
#[cfg(not(target_arch = "wasm32"))]
pub use zip_import::{extract_images_from_zip_file, is_zip_path};
