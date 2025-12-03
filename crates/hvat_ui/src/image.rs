use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global counter for generating unique image IDs.
static NEXT_IMAGE_ID: AtomicU64 = AtomicU64::new(1);

/// A handle to an image texture.
///
/// ImageHandle provides a way to reference image data that will be uploaded
/// to the GPU. Multiple widgets can share the same handle, and the framework
/// will cache the GPU texture to avoid redundant uploads.
///
/// Each ImageHandle has a unique ID that is used for GPU texture caching.
/// This ensures that when new image data is created (even if it happens to
/// be allocated at the same memory address as previous data), it will get
/// a fresh GPU texture upload rather than returning stale cached data.
#[derive(Clone, Debug)]
pub struct ImageHandle {
    /// Unique identifier for this image (used for GPU texture caching)
    id: u64,
    /// The raw RGBA8 image data
    data: Arc<Vec<u8>>,
    /// Width in pixels
    width: u32,
    /// Height in pixels
    height: u32,
}

impl ImageHandle {
    /// Generate a unique ID for a new image handle.
    fn new_id() -> u64 {
        NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed)
    }

    /// Create a new image handle from RGBA8 data.
    ///
    /// # Arguments
    /// * `data` - RGBA8 pixel data (4 bytes per pixel)
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    ///
    /// # Panics
    /// Panics if data.len() != width * height * 4
    pub fn from_rgba8(data: Vec<u8>, width: u32, height: u32) -> Self {
        assert_eq!(
            data.len(),
            (width * height * 4) as usize,
            "Image data size mismatch"
        );

        Self {
            id: Self::new_id(),
            data: Arc::new(data),
            width,
            height,
        }
    }

    /// Create a new image handle from Arc-wrapped RGBA8 data.
    ///
    /// This is more efficient when the data is already in an Arc.
    pub fn from_rgba8_arc(data: Arc<Vec<u8>>, width: u32, height: u32) -> Self {
        assert_eq!(
            data.len(),
            (width * height * 4) as usize,
            "Image data size mismatch"
        );

        Self {
            id: Self::new_id(),
            data,
            width,
            height,
        }
    }

    /// Get the unique ID of this image handle.
    ///
    /// This ID is used for GPU texture caching and is guaranteed to be unique
    /// for each image handle created during the application's lifetime.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the image data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get the image width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the image height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the aspect ratio (width / height).
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}
