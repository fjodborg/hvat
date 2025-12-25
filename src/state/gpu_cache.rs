//! GPU texture cache for preloaded hyperspectral images.
//!
//! Caches `HyperspectralGpuData` (band textures) for images within the preload
//! range to enable instant image switching without GPU re-upload.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use hvat_gpu::{GpuContext, HyperspectralGpuData};

use crate::data::HyperspectralData;

/// Calculate backward index with wraparound, avoiding underflow.
///
/// For a circular list of `len` items, starting at `current`, go back `offset` positions.
/// The formula `(current + len - offset % len) % len` handles:
/// - Normal case: current=5, offset=2, len=10 → (5+10-2)%10 = 13%10 = 3
/// - Wraparound: current=1, offset=3, len=10 → (1+10-3)%10 = 8
/// - Large offset: current=0, offset=15, len=5 → (0+5-0)%5 = 0 (offset%len=0)
#[inline]
fn wrap_backward(current: usize, offset: usize, len: usize) -> usize {
    (current + len - (offset % len)) % len
}

/// Cached GPU texture data for a hyperspectral image.
pub struct CachedGpuTexture {
    /// GPU data (band textures uploaded to GPU)
    pub gpu_data: HyperspectralGpuData,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Number of spectral bands
    pub num_bands: usize,
}

/// Cache for preloaded GPU textures.
///
/// Stores `HyperspectralGpuData` for images that have been preloaded to GPU.
/// The cache is keyed by image path (PathBuf).
pub struct GpuTextureCache {
    /// Cached textures indexed by image path
    entries: HashMap<PathBuf, CachedGpuTexture>,
    /// Maximum number of images to preload in each direction
    preload_count: usize,
}

impl GpuTextureCache {
    /// Create a new empty cache with the given preload count.
    pub fn new(preload_count: usize) -> Self {
        Self {
            entries: HashMap::new(),
            preload_count,
        }
    }

    /// Set the preload count.
    pub fn set_preload_count(&mut self, count: usize) {
        self.preload_count = count;
        log::debug!("GPU cache preload count set to: {}", count);
    }

    /// Check if an image is already cached.
    pub fn contains(&self, path: &PathBuf) -> bool {
        self.entries.contains_key(path)
    }

    /// Take cached GPU data, removing it from cache.
    /// Used when transferring ownership to GpuRenderState.
    pub fn take(&mut self, path: &PathBuf) -> Option<CachedGpuTexture> {
        self.entries.remove(path)
    }

    /// Insert cached GPU data into the cache.
    pub fn insert(&mut self, path: PathBuf, cached: CachedGpuTexture) {
        self.entries.insert(path, cached);
    }

    /// Get the current cache size (number of cached images).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Upload hyperspectral data to GPU and cache it.
    ///
    /// Returns Ok(()) on success, logs warning on failure.
    pub fn upload_and_cache(
        &mut self,
        gpu_ctx: &GpuContext,
        path: PathBuf,
        hyper: &HyperspectralData,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        log::info!(
            "Caching GPU texture for: {:?} ({}x{}, {} bands)",
            path,
            hyper.width,
            hyper.height,
            hyper.bands.len()
        );

        let gpu_data = HyperspectralGpuData::from_bands(
            gpu_ctx,
            &hyper.bands,
            hyper.width,
            hyper.height,
            bind_group_layout,
        );

        let cached = CachedGpuTexture {
            gpu_data,
            width: hyper.width,
            height: hyper.height,
            num_bands: hyper.bands.len(),
        };

        self.insert(path.clone(), cached);
        log::info!(
            "Cached GPU texture for {:?} (cache size: {})",
            path,
            self.entries.len()
        );
    }

    /// Insert a pre-uploaded texture into the cache (for chunked upload workflow).
    ///
    /// Creates the `HyperspectralGpuData` from an already-uploaded texture and caches it.
    #[allow(dead_code)] // Used only in WASM builds
    pub fn insert_from_texture(
        &mut self,
        gpu_ctx: &GpuContext,
        path: PathBuf,
        texture: wgpu::Texture,
        width: u32,
        height: u32,
        num_bands: usize,
        num_layers: u32,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        log::info!(
            "Caching pre-uploaded GPU texture for: {:?} ({}x{}, {} bands, {} layers)",
            path,
            width,
            height,
            num_bands,
            num_layers
        );

        let gpu_data = HyperspectralGpuData::from_texture(
            gpu_ctx,
            texture,
            width,
            height,
            num_bands,
            num_layers,
            bind_group_layout,
        );

        let cached = CachedGpuTexture {
            gpu_data,
            width,
            height,
            num_bands,
        };

        self.insert(path.clone(), cached);
        log::info!(
            "Cached pre-uploaded GPU texture for {:?} (cache size: {})",
            path,
            self.entries.len()
        );
    }

    /// Clear the entire cache (e.g., when folder changes).
    pub fn clear(&mut self) {
        let count = self.entries.len();
        self.entries.clear();
        if count > 0 {
            log::info!("Cleared GPU texture cache ({} entries)", count);
        }
    }

    /// Remove entries not in the given set of paths.
    /// Used to evict images that are no longer within preload range.
    pub fn retain_only(&mut self, paths: &HashSet<PathBuf>) {
        let before = self.entries.len();
        self.entries.retain(|path, _| paths.contains(path));
        let evicted = before - self.entries.len();
        if evicted > 0 {
            log::debug!(
                "Evicted {} entries from GPU cache (remaining: {})",
                evicted,
                self.entries.len()
            );
        }
    }

    /// Get paths that should be preloaded based on current index.
    ///
    /// Returns paths that are within preload range and not yet cached.
    /// The current image is excluded since it's already being displayed.
    pub fn paths_to_preload(&self, images: &[PathBuf], current_index: usize) -> Vec<PathBuf> {
        if self.preload_count == 0 || images.is_empty() {
            return Vec::new();
        }

        let mut to_preload = Vec::new();
        let len = images.len();
        let current_path = &images[current_index];

        // Preload N images before and after current
        for offset in 1..=self.preload_count {
            // Forward (next images)
            let forward_idx = (current_index + offset) % len;
            let forward_path = &images[forward_idx];
            // Skip if it's the current image (wraparound case) or already cached
            if forward_path != current_path && !self.contains(forward_path) {
                to_preload.push(forward_path.clone());
            }

            // Backward (previous images)
            let backward_idx = wrap_backward(current_index, offset, len);
            let backward_path = &images[backward_idx];
            // Skip if it's the current image (wraparound case) or already cached
            if backward_path != current_path && !self.contains(backward_path) {
                to_preload.push(backward_path.clone());
            }
        }

        to_preload
    }

    /// Get the set of paths that should be kept in cache.
    ///
    /// Includes current image and N images before/after.
    pub fn paths_to_keep(&self, images: &[PathBuf], current_index: usize) -> HashSet<PathBuf> {
        let mut keep = HashSet::new();

        if images.is_empty() {
            return keep;
        }

        let len = images.len();

        // Keep current image
        keep.insert(images[current_index].clone());

        // Keep N images before and after
        for offset in 1..=self.preload_count {
            // Forward
            let forward_idx = (current_index + offset) % len;
            keep.insert(images[forward_idx].clone());

            // Backward
            let backward_idx = wrap_backward(current_index, offset, len);
            keep.insert(images[backward_idx].clone());
        }

        keep
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_to_preload() {
        let cache = GpuTextureCache::new(2);
        let images: Vec<PathBuf> = (0..5)
            .map(|i| PathBuf::from(format!("img{}.png", i)))
            .collect();

        // At index 0, should preload indices 1, 2 (forward) and 4, 3 (backward)
        let to_preload = cache.paths_to_preload(&images, 0);
        assert!(to_preload.contains(&PathBuf::from("img1.png")));
        assert!(to_preload.contains(&PathBuf::from("img2.png")));
        assert!(to_preload.contains(&PathBuf::from("img4.png")));
        assert!(to_preload.contains(&PathBuf::from("img3.png")));
    }

    #[test]
    fn test_paths_to_keep() {
        let cache = GpuTextureCache::new(1);
        let images: Vec<PathBuf> = (0..5)
            .map(|i| PathBuf::from(format!("img{}.png", i)))
            .collect();

        // At index 2, should keep indices 1, 2, 3
        let to_keep = cache.paths_to_keep(&images, 2);
        assert!(to_keep.contains(&PathBuf::from("img1.png")));
        assert!(to_keep.contains(&PathBuf::from("img2.png")));
        assert!(to_keep.contains(&PathBuf::from("img3.png")));
        assert!(!to_keep.contains(&PathBuf::from("img0.png")));
        assert!(!to_keep.contains(&PathBuf::from("img4.png")));
    }

    #[test]
    fn test_preload_count_zero() {
        let cache = GpuTextureCache::new(0);
        let images: Vec<PathBuf> = (0..5)
            .map(|i| PathBuf::from(format!("img{}.png", i)))
            .collect();

        let to_preload = cache.paths_to_preload(&images, 2);
        assert!(to_preload.is_empty());
    }

    #[test]
    fn test_single_image() {
        // Test with single image - should not panic
        let cache = GpuTextureCache::new(2);
        let images: Vec<PathBuf> = vec![PathBuf::from("single.png")];

        // paths_to_preload should return empty (current image excluded)
        let to_preload = cache.paths_to_preload(&images, 0);
        assert!(to_preload.is_empty());

        // paths_to_keep should return just the current image
        let to_keep = cache.paths_to_keep(&images, 0);
        assert_eq!(to_keep.len(), 1);
        assert!(to_keep.contains(&PathBuf::from("single.png")));
    }

    #[test]
    fn test_preload_count_larger_than_images() {
        // Test when preload count is larger than number of images
        let cache = GpuTextureCache::new(10);
        let images: Vec<PathBuf> = (0..3)
            .map(|i| PathBuf::from(format!("img{}.png", i)))
            .collect();

        // Should not panic even with high preload count
        let to_preload = cache.paths_to_preload(&images, 0);
        // Should include img1 and img2 (not img0 which is current)
        assert!(to_preload.contains(&PathBuf::from("img1.png")));
        assert!(to_preload.contains(&PathBuf::from("img2.png")));

        let to_keep = cache.paths_to_keep(&images, 0);
        // Should keep all 3 images
        assert_eq!(to_keep.len(), 3);
    }
}
