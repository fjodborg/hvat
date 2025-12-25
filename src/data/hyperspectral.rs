//! Hyperspectral image data structure and loading.

/// CPU-side hyperspectral data, used for initial upload to GPU.
pub struct HyperspectralData {
    /// Band data as flattened f32 arrays (one per band)
    pub bands: Vec<Vec<f32>>,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Band labels (e.g., wavelength names)
    #[allow(dead_code)] // Reserved for future use (band labels in UI)
    pub labels: Vec<String>,
}

impl HyperspectralData {
    /// Create a new hyperspectral image with the given dimensions and bands.
    pub fn new(bands: Vec<Vec<f32>>, width: u32, height: u32, labels: Vec<String>) -> Self {
        Self {
            bands,
            width,
            height,
            labels,
        }
    }

    /// Create from pre-decoded band data (e.g., from a Web Worker).
    ///
    /// This is used when band data has already been decoded elsewhere
    /// and just needs to be wrapped in a `HyperspectralData` struct.
    #[allow(dead_code)]
    pub fn from_raw_bands(bands: Vec<Vec<f32>>, width: u32, height: u32) -> Self {
        let labels = (0..bands.len())
            .map(|i| match i {
                0 => "Red".to_string(),
                1 => "Green".to_string(),
                2 => "Blue".to_string(),
                _ => format!("Band {}", i + 1),
            })
            .collect();

        Self {
            bands,
            width,
            height,
            labels,
        }
    }

    /// Load from an image file (PNG, JPEG, etc).
    /// Converts RGB channels to 3 bands.
    ///
    /// Note: Prefer `from_bytes()` with `ProjectState::get_image_data()` for unified
    /// cross-platform loading.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(dead_code)] // Kept for direct native file loading use cases
    pub fn from_image_file(path: &std::path::Path) -> Result<Self, String> {
        let img = image::open(path)
            .map_err(|e| format!("Failed to open image: {}", e))?
            .to_rgba8();

        Self::from_rgba_image(img)
    }

    /// Load from raw image bytes (works on both WASM and native).
    ///
    /// This is the preferred method for cross-platform image loading.
    /// Use with `ProjectState::get_image_data()` for unified access.
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let img = image::load_from_memory(data)
            .map_err(|e| format!("Failed to decode image: {}", e))?
            .to_rgba8();

        Self::from_rgba_image(img)
    }

    /// Convert an RGBA image to hyperspectral data.
    /// Extracts RGB channels as 3 bands.
    fn from_rgba_image(img: image::RgbaImage) -> Result<Self, String> {
        let width = img.width();
        let height = img.height();
        let pixel_count = (width * height) as usize;

        let mut r_band = Vec::with_capacity(pixel_count);
        let mut g_band = Vec::with_capacity(pixel_count);
        let mut b_band = Vec::with_capacity(pixel_count);

        for pixel in img.pixels() {
            r_band.push(pixel[0] as f32 / 255.0);
            g_band.push(pixel[1] as f32 / 255.0);
            b_band.push(pixel[2] as f32 / 255.0);
        }

        Ok(Self {
            bands: vec![r_band, g_band, b_band],
            width,
            height,
            labels: vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()],
        })
    }
}
