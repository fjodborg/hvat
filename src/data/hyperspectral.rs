//! Hyperspectral image data structure and loading.

use crate::data::LoaderRegistry;

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
        let data = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let filename = path.file_name().and_then(|n| n.to_str());
        Self::from_bytes_with_hint(&data, filename)
    }

    /// Load from raw bytes, auto-detecting the format.
    ///
    /// This is the preferred method for cross-platform loading.
    /// Uses the `LoaderRegistry` to detect and load various formats including:
    /// - Standard images (PNG, JPEG, BMP, TIFF, WebP)
    /// - NumPy arrays (.npy)
    ///
    /// Use with `ProjectState::get_image_data()` for unified access.
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        Self::from_bytes_with_hint(data, None)
    }

    /// Load from raw bytes with an optional filename hint for format detection.
    ///
    /// The filename hint helps the loader registry choose the right loader
    /// based on file extension before falling back to magic byte detection.
    pub fn from_bytes_with_hint(data: &[u8], filename: Option<&str>) -> Result<Self, String> {
        let registry = LoaderRegistry::new();
        registry.load(data, filename).map_err(|e| e.to_string())
    }

    /// Get the number of bands.
    pub fn num_bands(&self) -> usize {
        self.bands.len()
    }
}
