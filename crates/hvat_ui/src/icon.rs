//! Icon loading and rasterization utilities.
//!
//! This module provides SVG icon loading and caching functionality.
//! Icons are rasterized at runtime using resvg/tiny-skia.
//!
//! # Global Icon Cache
//!
//! For convenience, a global icon cache is provided via `get_icon()`.
//! Icons are rasterized on first access and cached for subsequent use.

use crate::ImageHandle;
use std::collections::HashMap;
use std::sync::Mutex;

/// Global icon cache using lazy initialization.
static ICON_CACHE: std::sync::OnceLock<Mutex<IconCache>> = std::sync::OnceLock::new();

/// Get the global icon cache.
fn global_cache() -> &'static Mutex<IconCache> {
    ICON_CACHE.get_or_init(|| Mutex::new(IconCache::new()))
}

/// Get an icon from the global cache, rasterizing it if needed.
///
/// This is the primary API for getting icons. Icons are cached globally
/// and will only be rasterized once.
///
/// # Arguments
/// * `name` - Icon name (for caching, e.g., "cursor", "bounding-box")
/// * `svg_data` - Raw SVG bytes (use `icons::` constants)
/// * `size` - Target size in pixels (icons are square)
/// * `color` - RGBA color bytes to use (replaces "currentColor" in SVG)
///
/// # Example
/// ```ignore
/// use hvat_ui::icon::{get_icon, icons};
///
/// let icon = get_icon("cursor", icons::CURSOR, 24, [255, 255, 255, 255]);
/// ```
pub fn get_icon(name: &str, svg_data: &[u8], size: u32, color: [u8; 4]) -> Option<ImageHandle> {
    let mut cache = global_cache().lock().ok()?;
    cache.get_or_rasterize(name, svg_data, size, color)
}

/// A cache for rasterized icon images.
///
/// Icons are loaded once and cached as ImageHandles for efficient rendering.
#[derive(Default)]
pub struct IconCache {
    /// Cached icons keyed by (icon_name, size)
    cache: HashMap<(String, u32), ImageHandle>,
}

impl IconCache {
    /// Create a new empty icon cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get an icon from cache, or rasterize and cache it.
    ///
    /// # Arguments
    /// * `name` - Icon name (for caching)
    /// * `svg_data` - Raw SVG bytes
    /// * `size` - Target size in pixels (icons are square)
    /// * `color` - RGBA color to use (replaces "currentColor" in SVG)
    pub fn get_or_rasterize(
        &mut self,
        name: &str,
        svg_data: &[u8],
        size: u32,
        color: [u8; 4],
    ) -> Option<ImageHandle> {
        let key = (name.to_string(), size);

        if let Some(handle) = self.cache.get(&key) {
            return Some(handle.clone());
        }

        // Rasterize the SVG
        if let Some(handle) = rasterize_svg(svg_data, size, color) {
            self.cache.insert(key, handle.clone());
            Some(handle)
        } else {
            None
        }
    }

    /// Preload an icon into the cache.
    pub fn preload(&mut self, name: &str, svg_data: &[u8], size: u32, color: [u8; 4]) {
        let _ = self.get_or_rasterize(name, svg_data, size, color);
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

/// Rasterize an SVG to an RGBA ImageHandle.
///
/// # Arguments
/// * `svg_data` - Raw SVG bytes
/// * `size` - Target size in pixels (icons are square)
/// * `color` - RGBA color to apply
pub fn rasterize_svg(svg_data: &[u8], size: u32, color: [u8; 4]) -> Option<ImageHandle> {
    // Convert SVG bytes to string and replace currentColor with the actual color
    let svg_str = std::str::from_utf8(svg_data).ok()?;
    let hex_color = format!(
        "#{:02x}{:02x}{:02x}",
        color[0], color[1], color[2]
    );
    let svg_with_color = svg_str.replace("currentColor", &hex_color);

    log::debug!("Rasterizing SVG: {} bytes, target size: {}", svg_data.len(), size);

    // Parse SVG
    let tree = match resvg::usvg::Tree::from_str(
        &svg_with_color,
        &resvg::usvg::Options::default(),
    ) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to parse SVG: {:?}", e);
            return None;
        }
    };

    // Calculate scaling to fit in target size
    let svg_size = tree.size();
    let scale = (size as f32) / svg_size.width().max(svg_size.height());

    let width = (svg_size.width() * scale).ceil() as u32;
    let height = (svg_size.height() * scale).ceil() as u32;

    log::debug!("SVG size: {:?}, scale: {}, output: {}x{}", svg_size, scale, width, height);

    // Create pixmap for rendering
    let mut pixmap = match tiny_skia::Pixmap::new(width, height) {
        Some(p) => p,
        None => {
            log::error!("Failed to create pixmap {}x{}", width, height);
            return None;
        }
    };

    // Render with scaling transform
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Check if we got any non-transparent pixels
    let has_content = pixmap.data().chunks(4).any(|c| c[3] > 0);
    let opaque_count = pixmap.data().chunks(4).filter(|c| c[3] > 0).count();
    let total_pixels = (width * height) as usize;
    log::debug!("Pixmap has content: {}, opaque pixels: {}/{}", has_content, opaque_count, total_pixels);

    // Log first few pixels to verify content
    let first_pixels: Vec<_> = pixmap.data().chunks(4).take(5).collect();
    log::debug!("First 5 pixels (RGBA premult): {:?}", first_pixels);

    // Keep premultiplied alpha format - tiny-skia outputs this natively
    // and we use PREMULTIPLIED_ALPHA_BLENDING in the texture pipeline
    let rgba_data = pixmap.data().to_vec();

    Some(ImageHandle::from_rgba8(rgba_data, width, height))
}

/// Built-in icons embedded from Bootstrap Icons.
pub mod icons {
    // Annotation tools
    pub const CURSOR: &[u8] = include_bytes!("../assets/icons/cursor.svg");
    pub const BOUNDING_BOX: &[u8] = include_bytes!("../assets/icons/bounding-box.svg");
    pub const HEXAGON: &[u8] = include_bytes!("../assets/icons/hexagon.svg");
    pub const GEO_ALT: &[u8] = include_bytes!("../assets/icons/geo-alt.svg");

    // Zoom controls
    pub const ZOOM_IN: &[u8] = include_bytes!("../assets/icons/zoom-in.svg");
    pub const ZOOM_OUT: &[u8] = include_bytes!("../assets/icons/zoom-out.svg");
    pub const ASPECT_RATIO: &[u8] = include_bytes!("../assets/icons/aspect-ratio.svg");
    pub const RULERS: &[u8] = include_bytes!("../assets/icons/rulers.svg");

    // Navigation
    pub const ARROW_LEFT: &[u8] = include_bytes!("../assets/icons/arrow-left.svg");
    pub const ARROW_RIGHT: &[u8] = include_bytes!("../assets/icons/arrow-right.svg");

    // Actions
    pub const TRASH: &[u8] = include_bytes!("../assets/icons/trash.svg");
    pub const FOLDER_OPEN: &[u8] = include_bytes!("../assets/icons/folder2-open.svg");
    pub const DOWNLOAD: &[u8] = include_bytes!("../assets/icons/download.svg");
    pub const ESCAPE: &[u8] = include_bytes!("../assets/icons/escape.svg");
    pub const X_CIRCLE: &[u8] = include_bytes!("../assets/icons/x-circle.svg");
}
