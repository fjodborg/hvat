use std::sync::Arc;

/// A handle to an image texture.
///
/// ImageHandle provides a way to reference image data that will be uploaded
/// to the GPU. Multiple widgets can share the same handle, and the framework
/// will cache the GPU texture to avoid redundant uploads.
#[derive(Clone, Debug)]
pub struct ImageHandle {
    /// The raw RGBA8 image data
    data: Arc<Vec<u8>>,
    /// Width in pixels
    width: u32,
    /// Height in pixels
    height: u32,
}

impl ImageHandle {
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
            data,
            width,
            height,
        }
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
