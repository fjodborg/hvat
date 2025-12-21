//! GPU texture cache for preloaded hyperspectral images.
//!
//! Caches `HyperspectralGpuData` (band textures) for images within the preload
//! range to enable instant image switching without GPU re-upload.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use hvat_gpu::{GpuContext, HyperspectralGpuData};

use crate::data::HyperspectralData;

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
    /// Paths currently being loaded (to avoid duplicate loads)
    loading: HashSet<PathBuf>,
}

impl GpuTextureCache {
    /// Create a new empty cache with the given preload count.
    pub fn new(preload_count: usize) -> Self {
        Self {
            entries: HashMap::new(),
            preload_count,
            loading: HashSet::new(),
        }
    }

    /// Get the current preload count setting.
    pub fn preload_count(&self) -> usize {
        self.preload_count
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

    /// Check if an image is currently being loaded.
    pub fn is_loading(&self, path: &PathBuf) -> bool {
        self.loading.contains(path)
    }

    /// Mark an image as currently loading.
    pub fn mark_loading(&mut self, path: &PathBuf) {
        self.loading.insert(path.clone());
    }

    /// Remove loading mark for an image.
    pub fn unmark_loading(&mut self, path: &PathBuf) {
        self.loading.remove(path);
    }

    /// Get cached GPU data for an image (immutable reference).
    #[allow(dead_code)]
    pub fn get(&self, path: &PathBuf) -> Option<&CachedGpuTexture> {
        self.entries.get(path)
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

    /// Clear the entire cache (e.g., when folder changes).
    pub fn clear(&mut self) {
        let count = self.entries.len();
        self.entries.clear();
        self.loading.clear();
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
    /// Returns paths that are within preload range and not yet cached or loading.
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
            // Skip if it's the current image (wraparound case), already cached, or loading
            if forward_path != current_path
                && !self.contains(forward_path)
                && !self.is_loading(forward_path)
            {
                to_preload.push(forward_path.clone());
            }

            // Backward (previous images) with wrap-around
            let backward_idx = if offset > current_index {
                len - (offset - current_index)
            } else {
                current_index - offset
            };
            let backward_path = &images[backward_idx];
            // Skip if it's the current image (wraparound case), already cached, or loading
            if backward_path != current_path
                && !self.contains(backward_path)
                && !self.is_loading(backward_path)
            {
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
            let backward_idx = if offset > current_index {
                len - (offset - current_index)
            } else {
                current_index - offset
            };
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
}
