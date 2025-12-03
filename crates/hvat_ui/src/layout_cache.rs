//! Layout caching system for improved performance.
//!
//! This module provides a cache for computed layouts that persists across frames,
//! avoiding recalculation when the widget tree structure and constraints haven't changed.

use std::collections::HashMap;

use crate::{Layout, Limits, Rectangle};

/// A unique identifier for a layout node in the widget tree.
/// Composed of tree path (depth and sibling index) and constraint hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutKey {
    /// Hash of the tree path (parent indices chain)
    path_hash: u64,
    /// Hash of the layout constraints
    constraints_hash: u64,
}

impl LayoutKey {
    /// Create a new layout key from path and constraints.
    pub fn new(path: &[usize], limits: &Limits) -> Self {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut path_hasher = DefaultHasher::new();
        path.hash(&mut path_hasher);
        let path_hash = path_hasher.finish();

        let mut constraints_hasher = DefaultHasher::new();
        // Hash the constraint values (converted to bits for deterministic hashing)
        limits.min_width.to_bits().hash(&mut constraints_hasher);
        limits.max_width.to_bits().hash(&mut constraints_hasher);
        limits.min_height.to_bits().hash(&mut constraints_hasher);
        limits.max_height.to_bits().hash(&mut constraints_hasher);
        let constraints_hash = constraints_hasher.finish();

        Self {
            path_hash,
            constraints_hash,
        }
    }
}

/// Cached layout entry with validity tracking.
#[derive(Debug, Clone)]
struct CacheEntry {
    layout: Layout,
    /// Frame number when this entry was last used
    last_used_frame: u64,
}

/// A cache for computed widget layouts.
///
/// The cache stores layouts keyed by their tree path and constraints.
/// Entries are automatically cleaned up when not used for several frames.
#[derive(Debug)]
pub struct LayoutCache {
    entries: HashMap<LayoutKey, CacheEntry>,
    current_frame: u64,
    /// Number of frames to keep unused entries before cleanup
    max_unused_frames: u64,
    /// Statistics for debugging
    hits: u64,
    misses: u64,
}

impl Default for LayoutCache {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutCache {
    /// Create a new empty layout cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            current_frame: 0,
            max_unused_frames: 3,
            hits: 0,
            misses: 0,
        }
    }

    /// Start a new frame. Call this at the beginning of each render cycle.
    pub fn begin_frame(&mut self) {
        self.current_frame += 1;
    }

    /// End the current frame and cleanup stale entries.
    pub fn end_frame(&mut self) {
        // Remove entries that haven't been used recently
        let current = self.current_frame;
        let max_unused = self.max_unused_frames;
        self.entries.retain(|_, entry| {
            current - entry.last_used_frame <= max_unused
        });
    }

    /// Get a cached layout if available.
    pub fn get(&mut self, key: &LayoutKey) -> Option<Layout> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_used_frame = self.current_frame;
            self.hits += 1;
            Some(entry.layout.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Store a layout in the cache.
    pub fn insert(&mut self, key: LayoutKey, layout: Layout) {
        self.entries.insert(key, CacheEntry {
            layout,
            last_used_frame: self.current_frame,
        });
    }

    /// Get or compute a layout.
    ///
    /// If the layout is cached, return it. Otherwise compute it with the
    /// provided closure and store the result.
    pub fn get_or_insert_with<F>(&mut self, key: LayoutKey, compute: F) -> Layout
    where
        F: FnOnce() -> Layout,
    {
        if let Some(layout) = self.get(&key) {
            layout
        } else {
            let layout = compute();
            self.insert(key, layout.clone());
            layout
        }
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Invalidate the cache. Call when the widget tree structure changes.
    pub fn invalidate(&mut self) {
        self.entries.clear();
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }
}

/// Cache performance statistics.
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    /// Number of cached entries
    pub entries: usize,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f64,
}

/// Helper for tracking the current path in the widget tree during layout.
#[derive(Debug, Clone, Default)]
pub struct LayoutPath {
    indices: Vec<usize>,
}

impl LayoutPath {
    /// Create a new root path.
    pub fn new() -> Self {
        Self { indices: Vec::new() }
    }

    /// Push a child index onto the path.
    pub fn push(&mut self, index: usize) {
        self.indices.push(index);
    }

    /// Pop the last child index from the path.
    pub fn pop(&mut self) {
        self.indices.pop();
    }

    /// Create a layout key for the current path and limits.
    pub fn key(&self, limits: &Limits) -> LayoutKey {
        LayoutKey::new(&self.indices, limits)
    }

    /// Get the current depth in the tree.
    pub fn depth(&self) -> usize {
        self.indices.len()
    }
}

/// Context for layout computation with caching support.
pub struct LayoutContext<'a> {
    cache: &'a mut LayoutCache,
    path: LayoutPath,
}

impl<'a> LayoutContext<'a> {
    /// Create a new layout context with a cache.
    pub fn new(cache: &'a mut LayoutCache) -> Self {
        Self {
            cache,
            path: LayoutPath::new(),
        }
    }

    /// Enter a child widget for layout.
    pub fn enter_child(&mut self, index: usize) {
        self.path.push(index);
    }

    /// Exit the current child widget.
    pub fn exit_child(&mut self) {
        self.path.pop();
    }

    /// Get or compute a layout for the current path.
    pub fn layout_with<F>(&mut self, limits: &Limits, compute: F) -> Layout
    where
        F: FnOnce() -> Layout,
    {
        let key = self.path.key(limits);
        self.cache.get_or_insert_with(key, compute)
    }

    /// Get the current cache.
    pub fn cache(&self) -> &LayoutCache {
        self.cache
    }

    /// Get the current cache mutably.
    pub fn cache_mut(&mut self) -> &mut LayoutCache {
        self.cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_key_creation() {
        let limits = Limits::new(100.0, 200.0);
        let path1 = vec![0, 1, 2];
        let path2 = vec![0, 1, 3];

        let key1 = LayoutKey::new(&path1, &limits);
        let key2 = LayoutKey::new(&path2, &limits);
        let key3 = LayoutKey::new(&path1, &limits);

        // Same path and limits should produce same key
        assert_eq!(key1, key3);
        // Different paths should produce different keys
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_cache_hit_miss() {
        let mut cache = LayoutCache::new();
        cache.begin_frame();

        let key = LayoutKey::new(&[0, 1], &Limits::new(100.0, 100.0));
        let layout = Layout::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));

        // First access should miss
        assert!(cache.get(&key).is_none());
        assert_eq!(cache.stats().misses, 1);

        // Insert layout
        cache.insert(key, layout.clone());

        // Second access should hit
        let cached = cache.get(&key);
        assert!(cached.is_some());
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_cache_cleanup() {
        let mut cache = LayoutCache::new();

        let key1 = LayoutKey::new(&[0], &Limits::new(100.0, 100.0));
        let key2 = LayoutKey::new(&[1], &Limits::new(100.0, 100.0));
        let layout = Layout::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));

        // Frame 1: insert both
        cache.begin_frame();
        cache.insert(key1, layout.clone());
        cache.insert(key2, layout.clone());
        cache.end_frame();
        assert_eq!(cache.stats().entries, 2);

        // Frames 2-4: only use key1
        for _ in 0..4 {
            cache.begin_frame();
            cache.get(&key1);
            cache.end_frame();
        }

        // key2 should be cleaned up (unused for 4 frames > max_unused_frames=3)
        assert_eq!(cache.stats().entries, 1);
        assert!(cache.get(&key1).is_some());
    }

    #[test]
    fn test_layout_path() {
        let mut path = LayoutPath::new();
        assert_eq!(path.depth(), 0);

        path.push(0);
        assert_eq!(path.depth(), 1);

        path.push(2);
        assert_eq!(path.depth(), 2);

        path.pop();
        assert_eq!(path.depth(), 1);
    }
}
