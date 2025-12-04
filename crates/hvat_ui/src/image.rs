use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global counter for generating unique image IDs.
static NEXT_IMAGE_ID: AtomicU64 = AtomicU64::new(1);

/// Global counter for generating unique hyperspectral image IDs.
static NEXT_HYPERSPECTRAL_ID: AtomicU64 = AtomicU64::new(1);

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

/// A handle to a hyperspectral image for GPU-based band compositing.
///
/// Unlike ImageHandle which holds pre-composited RGBA data, HyperspectralImageHandle
/// holds raw band data that will be composited on the GPU. This allows instant
/// band selection changes by just updating a uniform instead of regenerating
/// the entire composite image.
///
/// Each band is stored as f32 values (0.0-1.0) and will be packed into a
/// texture array on the GPU (4 bands per layer in RGBA channels).
/// This supports hundreds of bands, limited only by GPU texture array size.
#[derive(Clone, Debug)]
pub struct HyperspectralImageHandle {
    /// Unique identifier for this hyperspectral image (used for GPU data caching)
    id: u64,
    /// Band data: Vec of bands, each band is Vec<f32> with one value per pixel
    bands: Arc<Vec<Vec<f32>>>,
    /// Width in pixels
    width: u32,
    /// Height in pixels
    height: u32,
}

impl HyperspectralImageHandle {
    /// Generate a unique ID for a new hyperspectral handle.
    fn new_id() -> u64 {
        NEXT_HYPERSPECTRAL_ID.fetch_add(1, Ordering::Relaxed)
    }

    /// Create a new hyperspectral image handle from band data.
    ///
    /// # Arguments
    /// * `bands` - Vector of band data, each band is Vec<f32> with values 0.0-1.0
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    ///
    /// # Panics
    /// Panics if any band length != width * height
    pub fn from_bands(bands: Vec<Vec<f32>>, width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        for (i, band) in bands.iter().enumerate() {
            assert_eq!(
                band.len(),
                pixel_count,
                "Band {} size mismatch: expected {}, got {}",
                i,
                pixel_count,
                band.len()
            );
        }

        Self {
            id: Self::new_id(),
            bands: Arc::new(bands),
            width,
            height,
        }
    }

    /// Create a new hyperspectral image handle from Arc-wrapped band data.
    pub fn from_bands_arc(bands: Arc<Vec<Vec<f32>>>, width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        for (i, band) in bands.iter().enumerate() {
            assert_eq!(
                band.len(),
                pixel_count,
                "Band {} size mismatch: expected {}, got {}",
                i,
                pixel_count,
                band.len()
            );
        }

        Self {
            id: Self::new_id(),
            bands,
            width,
            height,
        }
    }

    /// Get the unique ID of this hyperspectral handle.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the band data.
    pub fn bands(&self) -> &[Vec<f32>] {
        &self.bands
    }

    /// Get the number of bands.
    pub fn num_bands(&self) -> usize {
        self.bands.len()
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
