//! Trait-based hyperspectral data loading system.
//!
//! This module provides an extensible system for loading hyperspectral data
//! from various file formats. New formats can be added by implementing the
//! `HyperspectralLoader` trait.
//!
//! ## Supported Formats
//!
//! - **Standard Images**: PNG, JPEG, BMP, TIFF, WebP (3-band RGB)
//! - **NumPy Arrays**: `.npy` files with 2D (grayscale) or 3D (bands × H × W or H × W × bands) arrays
//!
//! ## Usage
//!
//! ```rust,ignore
//! use hvat::data::{LoaderRegistry, HyperspectralData};
//!
//! let registry = LoaderRegistry::new();
//! let data = registry.load_from_bytes(bytes, Some("image.npy"))?;
//! ```

use crate::data::HyperspectralData;

/// Error type for loader operations.
#[derive(Debug, Clone)]
pub struct LoaderError {
    /// Human-readable error message.
    pub message: String,
    /// The loader that produced this error (if known).
    pub loader_id: Option<&'static str>,
}

impl LoaderError {
    /// Create a new loader error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            loader_id: None,
        }
    }

    /// Create an error with loader context.
    pub fn with_loader(mut self, loader_id: &'static str) -> Self {
        self.loader_id = Some(loader_id);
        self
    }
}

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(loader) = self.loader_id {
            write!(f, "[{}] {}", loader, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for LoaderError {}

impl From<String> for LoaderError {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for LoaderError {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Trait for hyperspectral data format loaders.
///
/// Each format (standard images, NumPy, ENVI, HDF5, etc.) implements this trait
/// to provide loading from raw bytes to `HyperspectralData`.
pub trait HyperspectralLoader: Send + Sync {
    /// Unique identifier for this loader (e.g., "image", "npy", "envi").
    fn id(&self) -> &'static str;

    /// Human-readable name for UI display.
    fn display_name(&self) -> &'static str;

    /// File extensions this loader handles (lowercase, without dots).
    ///
    /// Example: `&["npy"]` for NumPy, `&["png", "jpg", "jpeg"]` for images.
    fn extensions(&self) -> &'static [&'static str];

    /// Check if this loader can handle the given data.
    ///
    /// This is used for format auto-detection when the file extension is unknown
    /// or ambiguous. Implementations should check magic bytes or headers.
    ///
    /// Returns `true` if this loader can likely handle the data.
    fn can_load(&self, data: &[u8]) -> bool;

    /// Load hyperspectral data from raw bytes.
    ///
    /// # Arguments
    /// * `data` - Raw file bytes
    ///
    /// # Returns
    /// * `Ok(HyperspectralData)` - Successfully loaded data
    /// * `Err(LoaderError)` - Loading failed with error details
    fn load(&self, data: &[u8]) -> Result<HyperspectralData, LoaderError>;

    /// Priority for format detection (higher = checked first).
    ///
    /// Used when multiple loaders claim to handle the same extension.
    /// Default is 0. Specialized formats should use higher values.
    fn priority(&self) -> i32 {
        0
    }
}

/// Registry of available hyperspectral data loaders.
///
/// Provides format detection and unified loading interface.
pub struct LoaderRegistry {
    loaders: Vec<Box<dyn HyperspectralLoader>>,
}

impl LoaderRegistry {
    /// Create a new registry with all built-in loaders.
    pub fn new() -> Self {
        let mut registry = Self {
            loaders: Vec::new(),
        };

        // Register built-in loaders (order matters for priority ties)
        registry.register(Box::new(super::loaders::ImageLoader));
        registry.register(Box::new(super::loaders::NpyLoader));

        // Sort by priority (highest first)
        registry
            .loaders
            .sort_by(|a, b| b.priority().cmp(&a.priority()));

        registry
    }

    /// Register a new loader.
    pub fn register(&mut self, loader: Box<dyn HyperspectralLoader>) {
        self.loaders.push(loader);
        // Re-sort after adding
        self.loaders.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Get all supported file extensions (for file filtering).
    pub fn supported_extensions(&self) -> Vec<&'static str> {
        let mut extensions: Vec<&'static str> = self
            .loaders
            .iter()
            .flat_map(|l| l.extensions().iter().copied())
            .collect();
        extensions.sort();
        extensions.dedup();
        extensions
    }

    /// Find loaders that handle a given extension.
    fn loaders_for_extension(&self, ext: &str) -> Vec<&dyn HyperspectralLoader> {
        let ext_lower = ext.to_lowercase();
        self.loaders
            .iter()
            .filter(|l| l.extensions().iter().any(|e| *e == ext_lower))
            .map(|l| l.as_ref())
            .collect()
    }

    /// Find a loader by trying magic byte detection.
    fn detect_loader(&self, data: &[u8]) -> Option<&dyn HyperspectralLoader> {
        self.loaders
            .iter()
            .find(|l| l.can_load(data))
            .map(|l| l.as_ref())
    }

    /// Load data, auto-detecting the format.
    ///
    /// Tries loaders in this order:
    /// 1. By file extension (if filename provided)
    /// 2. By magic byte detection
    /// 3. All loaders as fallback
    pub fn load(
        &self,
        data: &[u8],
        filename: Option<&str>,
    ) -> Result<HyperspectralData, LoaderError> {
        // Extract extension from filename
        let extension = filename.and_then(|f| f.rsplit('.').next().map(|e| e.to_lowercase()));

        // Try loaders matching the extension first
        if let Some(ref ext) = extension {
            let matching_loaders = self.loaders_for_extension(ext);
            for loader in &matching_loaders {
                match loader.load(data) {
                    Ok(result) => {
                        log::debug!("Loaded with {} loader (by extension)", loader.id());
                        return Ok(result);
                    }
                    Err(e) => {
                        log::trace!("Loader {} failed: {}", loader.id(), e);
                    }
                }
            }
        }

        // Try magic byte detection
        if let Some(loader) = self.detect_loader(data) {
            match loader.load(data) {
                Ok(result) => {
                    log::debug!("Loaded with {} loader (by detection)", loader.id());
                    return Ok(result);
                }
                Err(e) => {
                    log::trace!("Detected loader {} failed: {}", loader.id(), e);
                }
            }
        }

        // Last resort: try all loaders
        for loader in &self.loaders {
            if let Ok(result) = loader.load(data) {
                log::debug!("Loaded with {} loader (fallback)", loader.id());
                return Ok(result);
            }
        }

        Err(LoaderError::new(format!(
            "No loader could handle the data{}",
            filename
                .map(|f| format!(" (file: {})", f))
                .unwrap_or_default()
        )))
    }

    /// Check if a filename has a supported extension.
    pub fn is_supported_file(&self, filename: &str) -> bool {
        let lower = filename.to_lowercase();
        self.supported_extensions()
            .iter()
            .any(|ext| lower.ends_with(&format!(".{}", ext)))
    }

    /// Get all registered loaders.
    pub fn loaders(&self) -> &[Box<dyn HyperspectralLoader>] {
        &self.loaders
    }
}

impl Default for LoaderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_loaders() {
        let registry = LoaderRegistry::new();
        assert!(!registry.loaders().is_empty());
    }

    #[test]
    fn test_supported_extensions() {
        let registry = LoaderRegistry::new();
        let extensions = registry.supported_extensions();

        // Should include image formats
        assert!(extensions.contains(&"png"));
        assert!(extensions.contains(&"jpg"));

        // Should include npy
        assert!(extensions.contains(&"npy"));
    }

    #[test]
    fn test_is_supported_file() {
        let registry = LoaderRegistry::new();

        assert!(registry.is_supported_file("image.png"));
        assert!(registry.is_supported_file("data.npy"));
        assert!(registry.is_supported_file("IMAGE.PNG")); // case insensitive
        assert!(!registry.is_supported_file("document.pdf"));
    }
}
