//! Hyperspectral image data structure and loading

/// CPU-side hyperspectral data, used for initial upload to GPU.
pub struct HyperspectralData {
    pub(crate) bands: Vec<Vec<f32>>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    #[allow(dead_code)] // Reserved for future use (band labels in UI)
    pub(crate) labels: Vec<String>,
}

impl HyperspectralData {
    /// Load from an image file (PNG, JPEG, etc).
    /// Converts RGB channels to 3 bands.
    pub fn from_image_file(path: &std::path::Path) -> Result<Self, String> {
        let img = image::open(path)
            .map_err(|e| format!("Failed to open image: {}", e))?
            .to_rgba8();

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

    /// Load from raw bytes (for WASM).
    /// Converts RGB channels to 3 bands.
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let img = image::load_from_memory(data)
            .map_err(|e| format!("Failed to decode image: {}", e))?
            .to_rgba8();

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

    /// Accessor methods for accessing fields from other modules
    pub fn bands(&self) -> &[Vec<f32>] {
        &self.bands
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn num_bands(&self) -> usize {
        self.bands.len()
    }
}
