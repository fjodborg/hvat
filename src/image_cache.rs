//! Image caching and loading abstraction layer.
//!
//! This module provides a unified interface for image loading and caching
//! that works across both native and WASM platforms.

use hvat_ui::ImageHandle;
use std::collections::{HashMap, HashSet};

/// Supported image file extensions.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff", "tif"];

/// Check if a filename has a supported image extension.
pub fn is_image_file(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    IMAGE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Result of loading an image.
#[derive(Debug)]
pub enum LoadResult {
    /// Image loaded successfully
    Success(ImageHandle),
    /// Image failed to load
    Error(String),
}

/// Image source - platform-specific image identifier.
#[cfg(not(target_arch = "wasm32"))]
pub type ImageSource = std::path::PathBuf;

#[cfg(target_arch = "wasm32")]
pub type ImageSource = (String, Vec<u8>); // (filename, raw_bytes)

/// Unified image cache that works across native and WASM.
pub struct ImageCache {
    /// Number of images to preload before and after current index
    preload_count: usize,
    /// Decoded image cache (index -> ImageHandle)
    decoded_cache: HashMap<usize, ImageHandle>,
    /// Image sources (paths for native, bytes for WASM)
    #[cfg(not(target_arch = "wasm32"))]
    sources: Vec<std::path::PathBuf>,
    #[cfg(target_arch = "wasm32")]
    sources: Vec<(String, Vec<u8>)>,
}

impl ImageCache {
    /// Create a new image cache.
    pub fn new(preload_count: usize) -> Self {
        Self {
            preload_count,
            decoded_cache: HashMap::new(),
            sources: Vec::new(),
        }
    }

    /// Clear all cached images and sources.
    pub fn clear(&mut self) {
        self.decoded_cache.clear();
        self.sources.clear();
    }

    /// Get the number of loaded image sources.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Check if no images are loaded.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Get the preload count.
    pub fn preload_count(&self) -> usize {
        self.preload_count
    }

    /// Set the preload count.
    pub fn set_preload_count(&mut self, count: usize) {
        self.preload_count = count;
    }

    /// Get the name/filename of an image at the given index.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_name(&self, index: usize) -> Option<String> {
        self.sources.get(index).and_then(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_name(&self, index: usize) -> Option<String> {
        self.sources.get(index).map(|(name, _)| name.clone())
    }

    /// Load images from sources (native: folder path, WASM: file bytes).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_from_folder(&mut self, folder: &std::path::Path) -> Result<usize, String> {
        let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(folder)
            .map_err(|e| e.to_string())?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false)
            })
            .collect();

        paths.sort();

        let count = paths.len();
        self.sources = paths;
        self.decoded_cache.clear();

        Ok(count)
    }

    /// Load images from raw file bytes (WASM).
    #[cfg(target_arch = "wasm32")]
    pub fn load_from_bytes(&mut self, files: Vec<(String, Vec<u8>)>) -> usize {
        // Filter to only image files
        let image_files: Vec<_> = files
            .into_iter()
            .filter(|(name, _)| is_image_file(name))
            .collect();

        let count = image_files.len();
        self.sources = image_files;
        self.decoded_cache.clear();

        count
    }

    /// Get a cached image or load it.
    /// Returns the image handle if available/loadable.
    pub fn get_or_load(&mut self, index: usize) -> Option<ImageHandle> {
        if index >= self.sources.len() {
            return None;
        }

        // Check cache first
        if let Some(handle) = self.decoded_cache.get(&index) {
            return Some(handle.clone());
        }

        // Load and cache
        match self.load_image_at_index(index) {
            LoadResult::Success(handle) => {
                self.decoded_cache.insert(index, handle.clone());
                Some(handle)
            }
            LoadResult::Error(e) => {
                log::error!("Failed to load image at index {}: {}", index, e);
                None
            }
        }
    }

    /// Load an image at the given index (internal implementation).
    #[cfg(not(target_arch = "wasm32"))]
    fn load_image_at_index(&self, index: usize) -> LoadResult {
        let path = &self.sources[index];
        log::info!("🖼️ Loading image: {:?}", path);

        match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                let data = rgba.into_raw();
                let handle = ImageHandle::from_rgba8(data, width, height);
                log::info!("🖼️ Loaded {}x{} image", width, height);
                LoadResult::Success(handle)
            }
            Err(e) => LoadResult::Error(e.to_string()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn load_image_at_index(&self, index: usize) -> LoadResult {
        let (name, bytes) = &self.sources[index];
        log::info!("🖼️ Decoding image: {} ({} bytes)", name, bytes.len());

        match image::load_from_memory(bytes) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                let data = rgba.into_raw();
                let handle = ImageHandle::from_rgba8(data, width, height);
                log::info!("🖼️ Decoded {}: {}x{}", name, width, height);
                LoadResult::Success(handle)
            }
            Err(e) => LoadResult::Error(e.to_string()),
        }
    }

    /// Preload images adjacent to the current index.
    /// Also cleans up cache entries outside the preload window.
    pub fn preload_adjacent(&mut self, current_index: usize) {
        let total = self.sources.len();
        if total == 0 {
            return;
        }

        // Calculate indices to keep
        let keep_indices: HashSet<usize> = (0..=self.preload_count)
            .flat_map(|i| {
                let next = (current_index + i) % total;
                let prev = if current_index >= i {
                    current_index - i
                } else {
                    total - (i - current_index)
                };
                vec![next, prev]
            })
            .collect();

        // Preload next images
        for i in 1..=self.preload_count {
            let next_idx = (current_index + i) % total;
            if !self.decoded_cache.contains_key(&next_idx) {
                log::debug!("🖼️ Preloading next image at index {}", next_idx);
                if let LoadResult::Success(handle) = self.load_image_at_index(next_idx) {
                    self.decoded_cache.insert(next_idx, handle);
                }
            }
        }

        // Preload previous images
        for i in 1..=self.preload_count {
            let prev_idx = if current_index >= i {
                current_index - i
            } else {
                total - (i - current_index)
            };
            if !self.decoded_cache.contains_key(&prev_idx) {
                log::debug!("🖼️ Preloading prev image at index {}", prev_idx);
                if let LoadResult::Success(handle) = self.load_image_at_index(prev_idx) {
                    self.decoded_cache.insert(prev_idx, handle);
                }
            }
        }

        // Clean up cache - keep only images within preload range
        self.decoded_cache.retain(|idx, _| keep_indices.contains(idx));
    }

    /// Navigate to next image index (wrapping).
    pub fn next_index(&self, current: usize) -> usize {
        if self.sources.is_empty() {
            0
        } else {
            (current + 1) % self.sources.len()
        }
    }

    /// Navigate to previous image index (wrapping).
    pub fn prev_index(&self, current: usize) -> usize {
        if self.sources.is_empty() {
            0
        } else if current == 0 {
            self.sources.len() - 1
        } else {
            current - 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_file() {
        // Supported formats
        assert!(is_image_file("test.png"));
        assert!(is_image_file("test.PNG"));
        assert!(is_image_file("test.jpg"));
        assert!(is_image_file("test.JPEG"));
        assert!(is_image_file("test.gif"));
        assert!(is_image_file("test.bmp"));
        assert!(is_image_file("test.webp"));
        assert!(is_image_file("test.tiff"));
        assert!(is_image_file("test.tif"));

        // Unsupported formats
        assert!(!is_image_file("test.txt"));
        assert!(!is_image_file("test.rs"));
        assert!(!is_image_file("test.pdf"));
        assert!(!is_image_file("test.svg"));
        assert!(!is_image_file("test"));

        // Edge cases
        assert!(!is_image_file(""));
        // Note: "png" ends with "png" so it matches - this is fine for our use case
        // as we're checking file extensions, not validating file names
        assert!(is_image_file(".png")); // Hidden file
        assert!(is_image_file("path/to/image.png"));
    }

    #[test]
    fn test_cache_creation() {
        let cache = ImageCache::new(2);
        assert_eq!(cache.preload_count(), 2);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_preload_count_modification() {
        let mut cache = ImageCache::new(1);
        assert_eq!(cache.preload_count(), 1);

        cache.set_preload_count(3);
        assert_eq!(cache.preload_count(), 3);
    }

    #[test]
    fn test_navigation_empty_cache() {
        let cache = ImageCache::new(1);

        // Empty cache should return 0 for both directions
        assert_eq!(cache.next_index(0), 0);
        assert_eq!(cache.prev_index(0), 0);
        assert_eq!(cache.next_index(5), 0);
        assert_eq!(cache.prev_index(5), 0);
    }

    #[test]
    fn test_clear() {
        let mut cache = ImageCache::new(1);
        // Clear should work on empty cache without panic
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_get_name_out_of_bounds() {
        let cache = ImageCache::new(1);
        assert_eq!(cache.get_name(0), None);
        assert_eq!(cache.get_name(100), None);
    }

    #[test]
    fn test_image_extensions_constant() {
        // Ensure all extensions are lowercase
        for ext in IMAGE_EXTENSIONS {
            assert_eq!(*ext, ext.to_lowercase());
        }

        // Ensure we have the common formats
        assert!(IMAGE_EXTENSIONS.contains(&"png"));
        assert!(IMAGE_EXTENSIONS.contains(&"jpg"));
        assert!(IMAGE_EXTENSIONS.contains(&"jpeg"));
    }
}
