//! Renderer for drawing UI elements using hvat_gpu

use crate::constants::{
    LINE_HEIGHT_FACTOR, RENDERER_CLIP_STACK_CAPACITY, RENDERER_COLOR_INDEX_CAPACITY,
    RENDERER_COLOR_VERTEX_CAPACITY, RENDERER_OVERLAY_INDEX_CAPACITY,
    RENDERER_OVERLAY_TEXT_REQUEST_CAPACITY, RENDERER_OVERLAY_VERTEX_CAPACITY,
    RENDERER_TEXT_CACHE_CAPACITY, RENDERER_TEXT_REQUEST_CAPACITY, RENDERER_TEXTURE_REQUEST_CAPACITY,
};
use crate::layout::Bounds;
use crate::overlay::OverlayRegistry;
use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphonColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Wrap,
};
use hvat_gpu::{ColorPipeline, ColorVertex, GpuContext, ImageAdjustments, Pipeline, Texture, TexturePipeline, TransformUniform};
use std::collections::HashMap;
use wgpu::util::DeviceExt;

/// RGBA color
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
    pub const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Color = Color::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);
    pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

    // UI colors
    pub const DARK_BG: Color = Color::rgb(0.12, 0.12, 0.14);
    pub const BUTTON_BG: Color = Color::rgb(0.2, 0.2, 0.24);
    pub const BUTTON_HOVER: Color = Color::rgb(0.28, 0.28, 0.32);
    pub const BUTTON_ACTIVE: Color = Color::rgb(0.35, 0.35, 0.4);
    pub const TEXT_PRIMARY: Color = Color::rgb(0.9, 0.9, 0.92);
    pub const TEXT_SECONDARY: Color = Color::rgb(0.6, 0.6, 0.65);
    pub const ACCENT: Color = Color::rgb(0.4, 0.6, 1.0);
    pub const BORDER: Color = Color::rgb(0.3, 0.3, 0.35);

    // Scrollbar colors
    pub const SCROLLBAR_TRACK: Color = Color::rgba(0.1, 0.1, 0.12, 0.5);
    pub const SCROLLBAR_THUMB: Color = Color::rgba(0.5, 0.5, 0.55, 0.8);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Convert to glyphon color format
    pub fn to_glyphon(&self) -> GlyphonColor {
        GlyphonColor::rgba(
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

/// Text alignment for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Text draw request stored for batch rendering
struct TextRequest {
    text: String,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    clip: Option<Bounds>,
    is_overlay: bool,
    /// Optional width constraint for word wrapping
    wrap_width: Option<f32>,
    /// Text alignment within the wrap width
    align: TextAlign,
}

/// Key for text buffer cache (text content + font size + wrap width + alignment)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextCacheKey {
    text: String,
    /// Font size in tenths of a point (to avoid float hashing issues)
    size_tenths: u32,
    /// Wrap width in pixels (None = no wrapping)
    wrap_width: Option<u32>,
    /// Text alignment
    align: TextAlign,
}

impl std::hash::Hash for TextAlign {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (*self as u8).hash(state);
    }
}

impl TextCacheKey {
    fn new(text: &str, size: f32, wrap_width: Option<f32>, align: TextAlign) -> Self {
        Self {
            text: text.to_string(),
            size_tenths: (size * 10.0) as u32,
            wrap_width: wrap_width.map(|w| w as u32),
            align,
        }
    }
}

/// Texture draw request stored for batch rendering
struct TextureRequest {
    texture_id: TextureId,
    bounds: Bounds,
    transform: TransformUniform,
    adjustments: ImageAdjustments,
    clip: Option<Bounds>,
}

/// Unique identifier for a registered texture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(usize);

/// The UI renderer that collects and executes draw commands
pub struct Renderer {
    /// Color pipeline for shapes
    color_pipeline: ColorPipeline,
    /// Texture pipeline for images
    texture_pipeline: TexturePipeline,
    /// Current window size
    window_size: (u32, u32),
    /// Batched color vertices
    color_vertices: Vec<ColorVertex>,
    /// Batched color indices
    color_indices: Vec<u16>,
    /// Overlay color vertices (rendered after textures)
    overlay_vertices: Vec<ColorVertex>,
    /// Overlay color indices
    overlay_indices: Vec<u16>,
    /// Whether currently drawing to overlay layer
    drawing_overlay: bool,
    /// Font system for text rendering
    font_system: FontSystem,
    /// Swash cache for glyph rasterization
    swash_cache: SwashCache,
    /// Glyphon cache
    #[allow(dead_code)]
    glyphon_cache: Cache,
    /// Text atlas for GPU glyph cache (normal text)
    text_atlas: TextAtlas,
    /// Glyphon text renderer (normal text)
    text_renderer: TextRenderer,
    /// Text atlas for overlay text
    overlay_text_atlas: TextAtlas,
    /// Glyphon text renderer for overlay text
    overlay_text_renderer: TextRenderer,
    /// Glyphon viewport
    viewport: Viewport,
    /// Queued text requests (non-overlay)
    text_requests: Vec<TextRequest>,
    /// Queued overlay text requests
    overlay_text_requests: Vec<TextRequest>,
    /// Queued texture requests
    texture_requests: Vec<TextureRequest>,
    /// Registered texture bind groups
    texture_bind_groups: HashMap<TextureId, wgpu::BindGroup>,
    /// Next texture ID
    next_texture_id: usize,
    /// Current clip stack
    clip_stack: Vec<Bounds>,
    /// Text buffer cache for reusing shaped text between frames
    text_buffer_cache: HashMap<TextCacheKey, Buffer>,
    /// Keys used this frame (for cache cleanup)
    text_cache_used_keys: Vec<TextCacheKey>,
    /// Pre-allocated vertex buffer for color shapes (reused each frame)
    color_vertex_buffer: Option<wgpu::Buffer>,
    /// Pre-allocated index buffer for color shapes (reused each frame)
    color_index_buffer: Option<wgpu::Buffer>,
    /// Capacity of pre-allocated vertex buffer
    vertex_buffer_capacity: usize,
    /// Capacity of pre-allocated index buffer
    index_buffer_capacity: usize,
    /// Overlay registry for tracking active overlays (cleared each frame)
    overlay_registry: OverlayRegistry,
}

/// Embedded font for WASM compatibility (no system font access)
const EMBEDDED_FONT: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");

impl Renderer {
    /// Create a new renderer
    pub fn new(gpu_ctx: &GpuContext) -> Self {
        let color_pipeline = ColorPipeline::new(&gpu_ctx.device, gpu_ctx.surface_config.format);
        let texture_pipeline = TexturePipeline::new(gpu_ctx);

        // Initialize glyphon text rendering
        // Use embedded font for WASM compatibility
        let mut font_system = FontSystem::new();
        font_system.db_mut().load_font_data(EMBEDDED_FONT.to_vec());
        log::info!("Loaded embedded font for text rendering");
        let swash_cache = SwashCache::new();
        let glyphon_cache = Cache::new(&gpu_ctx.device);
        let mut text_atlas = TextAtlas::new(
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &glyphon_cache,
            gpu_ctx.surface_config.format,
        );
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            &gpu_ctx.device,
            wgpu::MultisampleState::default(),
            None,
        );

        // Create separate atlas and renderer for overlay text
        let mut overlay_text_atlas = TextAtlas::new(
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &glyphon_cache,
            gpu_ctx.surface_config.format,
        );
        let overlay_text_renderer = TextRenderer::new(
            &mut overlay_text_atlas,
            &gpu_ctx.device,
            wgpu::MultisampleState::default(),
            None,
        );

        let viewport = Viewport::new(&gpu_ctx.device, &glyphon_cache);

        Self {
            color_pipeline,
            texture_pipeline,
            window_size: (gpu_ctx.width(), gpu_ctx.height()),
            color_vertices: Vec::with_capacity(RENDERER_COLOR_VERTEX_CAPACITY),
            color_indices: Vec::with_capacity(RENDERER_COLOR_INDEX_CAPACITY),
            overlay_vertices: Vec::with_capacity(RENDERER_OVERLAY_VERTEX_CAPACITY),
            overlay_indices: Vec::with_capacity(RENDERER_OVERLAY_INDEX_CAPACITY),
            drawing_overlay: false,
            font_system,
            swash_cache,
            glyphon_cache,
            text_atlas,
            text_renderer,
            overlay_text_atlas,
            overlay_text_renderer,
            viewport,
            text_requests: Vec::with_capacity(RENDERER_TEXT_REQUEST_CAPACITY),
            overlay_text_requests: Vec::with_capacity(RENDERER_OVERLAY_TEXT_REQUEST_CAPACITY),
            texture_requests: Vec::with_capacity(RENDERER_TEXTURE_REQUEST_CAPACITY),
            texture_bind_groups: HashMap::new(),
            next_texture_id: 0,
            clip_stack: Vec::with_capacity(RENDERER_CLIP_STACK_CAPACITY),
            text_buffer_cache: HashMap::with_capacity(RENDERER_TEXT_CACHE_CAPACITY),
            text_cache_used_keys: Vec::with_capacity(RENDERER_TEXT_CACHE_CAPACITY),
            color_vertex_buffer: None,
            color_index_buffer: None,
            vertex_buffer_capacity: 0,
            index_buffer_capacity: 0,
            overlay_registry: OverlayRegistry::new(),
        }
    }

    /// Update window size
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.window_size != (width, height) {
            self.window_size = (width, height);
            // Clear text buffer cache since buffers were created with old size
            self.text_buffer_cache.clear();
            self.text_cache_used_keys.clear();
            log::debug!("Renderer resized to {}x{}, text cache cleared", width, height);
        }
    }

    /// Get current window size
    pub fn window_size(&self) -> (u32, u32) {
        self.window_size
    }

    /// Clear all batched commands
    pub fn clear(&mut self) {
        self.color_vertices.clear();
        self.color_indices.clear();
        self.overlay_vertices.clear();
        self.overlay_indices.clear();
        self.drawing_overlay = false;
        self.text_requests.clear();
        self.overlay_text_requests.clear();
        self.texture_requests.clear();
        // NOTE: We intentionally do NOT clear overlay_registry here.
        // The registry is cleared at the start of the next frame's draw phase
        // so that event handlers can query it before the new frame is rendered.
        // This allows the overlay hint to work correctly.

        // Clean up unused text buffers from cache (remove entries not used this frame)
        if !self.text_cache_used_keys.is_empty() {
            self.text_buffer_cache.retain(|k, _| self.text_cache_used_keys.contains(k));
            self.text_cache_used_keys.clear();
        }
    }

    /// Clear overlay registry (called at start of draw phase)
    ///
    /// This should be called before widgets start drawing so they can
    /// re-register their overlays for the current frame.
    pub fn clear_overlay_registry(&mut self) {
        self.overlay_registry.clear();
    }

    /// Start drawing to the overlay layer (rendered after textures)
    pub fn begin_overlay(&mut self) {
        self.drawing_overlay = true;
    }

    /// Stop drawing to the overlay layer
    pub fn end_overlay(&mut self) {
        self.drawing_overlay = false;
    }

    /// Register an overlay's capture bounds
    ///
    /// Call this during `draw()` when rendering an overlay (dropdown popup, tooltip, etc.)
    /// The overlay registry is cleared each frame, so overlays must re-register every frame.
    pub fn register_overlay(&mut self, bounds: Bounds) {
        self.overlay_registry.register(bounds);
    }

    /// Register an overlay with explicit z-order
    pub fn register_overlay_with_z_order(&mut self, bounds: Bounds, z_order: u32) {
        self.overlay_registry.register_with_z_order(bounds, z_order);
    }

    /// Check if a position is within any registered overlay
    pub fn has_overlay_at(&self, x: f32, y: f32) -> bool {
        self.overlay_registry.has_overlay_at(x, y)
    }

    /// Get a reference to the overlay registry
    pub fn overlay_registry(&self) -> &OverlayRegistry {
        &self.overlay_registry
    }

    /// Get the current vertices and indices based on overlay mode
    fn get_current_buffers(&mut self) -> (&mut Vec<ColorVertex>, &mut Vec<u16>) {
        if self.drawing_overlay {
            (&mut self.overlay_vertices, &mut self.overlay_indices)
        } else {
            (&mut self.color_vertices, &mut self.color_indices)
        }
    }

    /// Fill a rectangle
    pub fn fill_rect(&mut self, bounds: Bounds, color: Color) {
        // Apply CPU-side clipping if there's an active clip region
        // Skip clipping when drawing overlays - they should render above everything
        let clipped_bounds = if !self.drawing_overlay {
            if let Some(clip) = self.clip_stack.last() {
                match bounds.intersect(clip) {
                    Some(b) if b.width > 0.0 && b.height > 0.0 => b,
                    _ => return, // Completely clipped out
                }
            } else {
                bounds
            }
        } else {
            bounds
        };

        let (w, h) = self.window_size;
        let (vertices, indices) = self.get_current_buffers();
        ColorPipeline::append_rect(
            vertices,
            indices,
            clipped_bounds.x,
            clipped_bounds.y,
            clipped_bounds.width,
            clipped_bounds.height,
            color.to_array(),
            w as f32,
            h as f32,
        );
    }

    /// Stroke a rectangle outline
    pub fn stroke_rect(&mut self, bounds: Bounds, color: Color, thickness: f32) {
        // Skip clipping when drawing overlays - they should render above everything
        if !self.drawing_overlay {
            // Apply CPU-side clipping - draw each edge as a separate clipped rectangle
            if let Some(clip) = self.clip_stack.last().cloned() {
                // Skip if completely outside clip region
                if bounds.intersect(&clip).is_none() {
                    return;
                }

                let half_thick = thickness / 2.0;

                // Top edge: horizontal line at top of bounds
                let top_edge = Bounds::new(
                    bounds.x - half_thick,
                    bounds.y - half_thick,
                    bounds.width + thickness,
                    thickness,
                );
                if let Some(clipped) = top_edge.intersect(&clip) {
                    if clipped.width > 0.0 && clipped.height > 0.0 {
                        self.fill_rect_no_clip(clipped, color);
                    }
                }

                // Bottom edge: horizontal line at bottom of bounds
                let bottom_edge = Bounds::new(
                    bounds.x - half_thick,
                    bounds.y + bounds.height - half_thick,
                    bounds.width + thickness,
                    thickness,
                );
                if let Some(clipped) = bottom_edge.intersect(&clip) {
                    if clipped.width > 0.0 && clipped.height > 0.0 {
                        self.fill_rect_no_clip(clipped, color);
                    }
                }

                // Left edge: vertical line at left of bounds (excluding corners to avoid overlap)
                let left_edge = Bounds::new(
                    bounds.x - half_thick,
                    bounds.y + half_thick,
                    thickness,
                    bounds.height - thickness,
                );
                if let Some(clipped) = left_edge.intersect(&clip) {
                    if clipped.width > 0.0 && clipped.height > 0.0 {
                        self.fill_rect_no_clip(clipped, color);
                    }
                }

                // Right edge: vertical line at right of bounds (excluding corners to avoid overlap)
                let right_edge = Bounds::new(
                    bounds.x + bounds.width - half_thick,
                    bounds.y + half_thick,
                    thickness,
                    bounds.height - thickness,
                );
                if let Some(clipped) = right_edge.intersect(&clip) {
                    if clipped.width > 0.0 && clipped.height > 0.0 {
                        self.fill_rect_no_clip(clipped, color);
                    }
                }
                return;
            }
        }

        // No clip or drawing overlay - use the GPU stroke_rect directly
        let (w, h) = self.window_size;
        let (vertices, indices) = self.get_current_buffers();
        ColorPipeline::append_stroke_rect(
            vertices,
            indices,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            color.to_array(),
            thickness,
            w as f32,
            h as f32,
        );
    }

    /// Internal fill_rect that doesn't apply clipping (for use by stroke_rect which pre-clips)
    fn fill_rect_no_clip(&mut self, bounds: Bounds, color: Color) {
        let (w, h) = self.window_size;
        let (vertices, indices) = self.get_current_buffers();
        ColorPipeline::append_rect(
            vertices,
            indices,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            color.to_array(),
            w as f32,
            h as f32,
        );
    }

    /// Draw a line (clipped to current clip region using Liang-Barsky algorithm)
    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, thickness: f32) {
        // Skip clipping when drawing overlays
        let (cx1, cy1, cx2, cy2) = if !self.drawing_overlay {
            if let Some(clip) = self.clip_stack.last() {
                match Self::clip_line(x1, y1, x2, y2, clip) {
                    Some((cx1, cy1, cx2, cy2)) => (cx1, cy1, cx2, cy2),
                    None => return, // Line completely outside clip region
                }
            } else {
                (x1, y1, x2, y2)
            }
        } else {
            (x1, y1, x2, y2)
        };

        let (w, h) = self.window_size;
        let (vertices, indices) = self.get_current_buffers();
        ColorPipeline::append_line(
            vertices,
            indices,
            cx1,
            cy1,
            cx2,
            cy2,
            color.to_array(),
            thickness,
            w as f32,
            h as f32,
        );
    }

    /// Clip line to rectangle using Liang-Barsky algorithm.
    fn clip_line(x1: f32, y1: f32, x2: f32, y2: f32, clip: &Bounds) -> Option<(f32, f32, f32, f32)> {
        let (dx, dy) = (x2 - x1, y2 - y1);
        let (mut t0, mut t1) = (0.0_f32, 1.0_f32);

        for (p, q) in [
            (-dx, x1 - clip.x),                  // left
            (dx, clip.x + clip.width - x1),      // right
            (-dy, y1 - clip.y),                  // top
            (dy, clip.y + clip.height - y1),     // bottom
        ] {
            if p == 0.0 {
                if q < 0.0 { return None; }
            } else {
                let t = q / p;
                if p < 0.0 { t0 = t0.max(t); }
                else { t1 = t1.min(t); }
                if t0 > t1 { return None; }
            }
        }

        Some((x1 + t0 * dx, y1 + t0 * dy, x1 + t1 * dx, y1 + t1 * dy))
    }

    /// Draw a filled circle
    pub fn fill_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        // Skip clipping when drawing overlays
        if !self.drawing_overlay {
            // Skip if completely outside clip region
            if let Some(clip) = self.clip_stack.last() {
                let circle_bounds =
                    Bounds::new(cx - radius, cy - radius, radius * 2.0, radius * 2.0);
                if circle_bounds.intersect(clip).is_none() {
                    return; // Completely clipped out
                }
            }
        }

        let (w, h) = self.window_size;
        let (vertices, indices) = self.get_current_buffers();
        ColorPipeline::append_circle(
            vertices,
            indices,
            cx,
            cy,
            radius,
            color.to_array(),
            w as f32,
            h as f32,
        );
    }

    /// Measure text width using the actual font system
    ///
    /// Returns the actual rendered width of the text string at the given font size.
    /// This is more accurate than approximate char_width calculations.
    pub fn measure_text_width(&mut self, text: &str, size: f32) -> f32 {
        use glyphon::{Attrs, Family, Metrics, Shaping};

        // Create a temporary buffer to measure
        let mut buffer = glyphon::Buffer::new(
            &mut self.font_system,
            Metrics::new(size, size * LINE_HEIGHT_FACTOR),
        );

        // Set a large width so text doesn't wrap
        buffer.set_size(&mut self.font_system, Some(10000.0), Some(size * 2.0));

        buffer.set_text(
            &mut self.font_system,
            text,
            &Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
        );

        buffer.shape_until_scroll(&mut self.font_system, false);

        // Get the actual width from the layout
        let mut max_width: f32 = 0.0;
        for run in buffer.layout_runs() {
            let line_width = run.glyphs.iter().map(|g| g.w).sum::<f32>();
            // Also account for the starting x offset of the last glyph
            if let Some(last_glyph) = run.glyphs.last() {
                max_width = max_width.max(last_glyph.x + last_glyph.w);
            } else {
                max_width = max_width.max(line_width);
            }
        }

        max_width
    }

    /// Measure text dimensions with word wrapping at a given width
    ///
    /// Returns (width, height) where height accounts for multiple lines when wrapping.
    pub fn measure_text_wrapped(&mut self, text: &str, size: f32, max_width: f32) -> (f32, f32) {
        use glyphon::{Attrs, Family, Metrics, Shaping, Wrap};

        // Create a temporary buffer to measure
        let mut buffer = glyphon::Buffer::new(
            &mut self.font_system,
            Metrics::new(size, size * LINE_HEIGHT_FACTOR),
        );

        // Set the width constraint for wrapping
        buffer.set_size(&mut self.font_system, Some(max_width), None);
        buffer.set_wrap(&mut self.font_system, Wrap::Word);

        buffer.set_text(
            &mut self.font_system,
            text,
            &Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
        );

        buffer.shape_until_scroll(&mut self.font_system, false);

        // Get the dimensions from the layout
        let mut actual_width: f32 = 0.0;
        let mut line_count: usize = 0;
        for run in buffer.layout_runs() {
            line_count += 1;
            if let Some(last_glyph) = run.glyphs.last() {
                actual_width = actual_width.max(last_glyph.x + last_glyph.w);
            }
        }

        // If no lines, use single line height
        let line_count = line_count.max(1);
        let height = line_count as f32 * size * LINE_HEIGHT_FACTOR;

        (actual_width, height)
    }

    /// Queue text for rendering
    pub fn text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        self.text_impl(text, x, y, size, color, None, TextAlign::Left);
    }

    /// Queue text for rendering with word wrapping at a specified width
    pub fn text_wrapped(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color, wrap_width: f32, align: TextAlign) {
        self.text_impl(text, x, y, size, color, Some(wrap_width), align);
    }

    /// Internal implementation for queuing text
    fn text_impl(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color, wrap_width: Option<f32>, align: TextAlign) {
        // Skip clipping when drawing overlays
        let clip = if self.drawing_overlay {
            None
        } else {
            self.clip_stack.last().cloned()
        };
        let request = TextRequest {
            text: text.to_string(),
            x,
            y,
            size,
            color,
            clip,
            is_overlay: self.drawing_overlay,
            wrap_width,
            align,
        };
        // Add to appropriate list based on overlay state
        if self.drawing_overlay {
            self.overlay_text_requests.push(request);
        } else {
            self.text_requests.push(request);
        }
    }

    /// Push a clip rectangle
    pub fn push_clip(&mut self, bounds: Bounds) {
        // If there's already a clip, intersect with it
        let clip = if let Some(current) = self.clip_stack.last() {
            let intersected = current.intersect(&bounds);
            log::debug!(
                "Renderer push_clip: requested={:?}, current={:?}, intersected={:?}",
                bounds,
                current,
                intersected
            );
            intersected
        } else {
            log::debug!("Renderer push_clip: requested={:?} (no existing clip)", bounds);
            Some(bounds)
        };

        if let Some(c) = clip {
            log::debug!(
                "Renderer: clip stack depth {} -> {}, active clip={:?}",
                self.clip_stack.len(),
                self.clip_stack.len() + 1,
                c
            );
            self.clip_stack.push(c);
        } else {
            // Empty intersection - push a zero-size clip
            log::debug!(
                "Renderer: clip stack depth {} -> {} (EMPTY clip - no intersection!)",
                self.clip_stack.len(),
                self.clip_stack.len() + 1
            );
            self.clip_stack.push(Bounds::new(bounds.x, bounds.y, 0.0, 0.0));
        }
    }

    /// Pop the current clip rectangle
    pub fn pop_clip(&mut self) {
        log::debug!(
            "Renderer pop_clip: depth {} -> {}",
            self.clip_stack.len(),
            self.clip_stack.len().saturating_sub(1)
        );
        self.clip_stack.pop();
    }

    /// Register a texture and return its ID
    pub fn register_texture(&mut self, gpu_ctx: &GpuContext, texture: &Texture) -> TextureId {
        let id = TextureId(self.next_texture_id);
        self.next_texture_id += 1;

        let bind_group = self.texture_pipeline.create_texture_bind_group(gpu_ctx, texture);
        self.texture_bind_groups.insert(id, bind_group);

        log::debug!("Registered texture {:?}", id);
        id
    }

    /// Unregister a texture
    pub fn unregister_texture(&mut self, id: TextureId) {
        self.texture_bind_groups.remove(&id);
        log::debug!("Unregistered texture {:?}", id);
    }

    /// Queue a texture for rendering within bounds with a transform
    pub fn texture(
        &mut self,
        texture_id: TextureId,
        bounds: Bounds,
        transform: TransformUniform,
    ) {
        self.texture_with_adjustments(texture_id, bounds, transform, ImageAdjustments::default());
    }

    /// Queue a texture for rendering with image adjustments (brightness, contrast, etc.)
    pub fn texture_with_adjustments(
        &mut self,
        texture_id: TextureId,
        bounds: Bounds,
        transform: TransformUniform,
        adjustments: ImageAdjustments,
    ) {
        let clip = self.clip_stack.last().cloned();
        self.texture_requests.push(TextureRequest {
            texture_id,
            bounds,
            transform,
            adjustments,
            clip,
        });
    }

    /// Ensure vertex buffer has enough capacity, creating or resizing as needed
    fn ensure_vertex_buffer(&mut self, gpu_ctx: &GpuContext, required_size: usize) {
        if self.vertex_buffer_capacity < required_size || self.color_vertex_buffer.is_none() {
            // Round up to next power of 2 for efficient resizing
            let new_capacity = required_size.next_power_of_two().max(1024);
            self.color_vertex_buffer = Some(gpu_ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("UI Color Vertices (reusable)"),
                size: (new_capacity * std::mem::size_of::<ColorVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.vertex_buffer_capacity = new_capacity;
            log::debug!("Resized vertex buffer to {} vertices", new_capacity);
        }
    }

    /// Ensure index buffer has enough capacity, creating or resizing as needed
    fn ensure_index_buffer(&mut self, gpu_ctx: &GpuContext, required_size: usize) {
        if self.index_buffer_capacity < required_size || self.color_index_buffer.is_none() {
            // Round up to next power of 2 for efficient resizing
            let new_capacity = required_size.next_power_of_two().max(2048);
            self.color_index_buffer = Some(gpu_ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("UI Color Indices (reusable)"),
                size: (new_capacity * std::mem::size_of::<u16>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.index_buffer_capacity = new_capacity;
            log::debug!("Resized index buffer to {} indices", new_capacity);
        }
    }

    /// Execute all draw commands
    pub fn render(
        &mut self,
        gpu_ctx: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        // Render batched color primitives
        if !self.color_vertices.is_empty() {
            // Ensure buffers have enough capacity
            self.ensure_vertex_buffer(gpu_ctx, self.color_vertices.len());
            self.ensure_index_buffer(gpu_ctx, self.color_indices.len());

            // Update buffer contents via queue.write_buffer (more efficient than recreating)
            if let Some(vertex_buffer) = &self.color_vertex_buffer {
                gpu_ctx.queue.write_buffer(
                    vertex_buffer,
                    0,
                    bytemuck::cast_slice(&self.color_vertices),
                );
            }
            if let Some(index_buffer) = &self.color_index_buffer {
                gpu_ctx.queue.write_buffer(
                    index_buffer,
                    0,
                    bytemuck::cast_slice(&self.color_indices),
                );
            }

            let vertex_buffer = self.color_vertex_buffer.as_ref().unwrap();
            let index_buffer = self.color_index_buffer.as_ref().unwrap();

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(self.color_pipeline.render_pipeline());
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.color_indices.len() as u32, 0, 0..1);
        }

        // Render normal text (before textures and overlays)
        if !self.text_requests.is_empty() {
            self.render_text(gpu_ctx, encoder, view, false);
        }

        // Render textures
        if !self.texture_requests.is_empty() {
            self.render_textures(gpu_ctx, encoder, view);
        }

        // Render overlay shapes (on top of textures)
        if !self.overlay_vertices.is_empty() {
            let vertex_buffer =
                gpu_ctx
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("UI Overlay Vertices"),
                        contents: bytemuck::cast_slice(&self.overlay_vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
            let index_buffer =
                gpu_ctx
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("UI Overlay Indices"),
                        contents: bytemuck::cast_slice(&self.overlay_indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Overlay Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(self.color_pipeline.render_pipeline());
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.overlay_indices.len() as u32, 0, 0..1);
        }

        // Render overlay text (on top of overlay shapes)
        if !self.overlay_text_requests.is_empty() {
            self.render_text(gpu_ctx, encoder, view, true);
        }

        // Clear for next frame
        self.clear();
    }

    /// Render all queued textures
    fn render_textures(
        &mut self,
        gpu_ctx: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let (window_width, window_height) = self.window_size;

        for request in &self.texture_requests {
            let Some(bind_group) = self.texture_bind_groups.get(&request.texture_id) else {
                log::warn!("Texture {:?} not found", request.texture_id);
                continue;
            };

            // Calculate clip bounds
            let clip = request.clip.unwrap_or(Bounds::new(
                0.0,
                0.0,
                window_width as f32,
                window_height as f32,
            ));

            // Intersect with widget bounds
            let effective_clip = clip.intersect(&request.bounds).unwrap_or(Bounds::ZERO);

            if effective_clip.width <= 0.0 || effective_clip.height <= 0.0 {
                continue;
            }

            // Update transform uniform and image adjustments
            self.texture_pipeline.update_transform(gpu_ctx, request.transform);
            self.texture_pipeline.update_adjustments(gpu_ctx, request.adjustments);

            // Render with scissor rect
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Texture Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                // Set scissor rect to clip to widget bounds
                render_pass.set_scissor_rect(
                    effective_clip.x as u32,
                    effective_clip.y as u32,
                    effective_clip.width as u32,
                    effective_clip.height as u32,
                );

                // Set viewport to widget bounds so the fullscreen quad maps to widget area
                render_pass.set_viewport(
                    request.bounds.x,
                    request.bounds.y,
                    request.bounds.width,
                    request.bounds.height,
                    0.0,
                    1.0,
                );

                render_pass.set_pipeline(self.texture_pipeline.render_pipeline());
                render_pass.set_bind_group(0, &self.texture_pipeline.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.texture_pipeline.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    self.texture_pipeline.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                render_pass.draw_indexed(0..self.texture_pipeline.num_indices, 0, 0..1);
            }
        }
    }

    /// Render text requests (either normal or overlay)
    /// Uses separate atlas/renderer for each to allow two-pass rendering
    fn render_text(
        &mut self,
        gpu_ctx: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        is_overlay: bool,
    ) {
        let (width, height) = self.window_size;

        // Select the appropriate request list
        let requests = if is_overlay {
            &self.overlay_text_requests
        } else {
            &self.text_requests
        };

        if requests.is_empty() {
            return;
        }

        // Update viewport
        self.viewport.update(&gpu_ctx.queue, Resolution { width, height });

        // Ensure all text buffers are in cache (or create new ones)
        for request in requests {
            let key = TextCacheKey::new(&request.text, request.size, request.wrap_width, request.align);

            // Track that this key is used this frame
            if !self.text_cache_used_keys.contains(&key) {
                self.text_cache_used_keys.push(key.clone());
            }

            // Create buffer if not in cache
            if !self.text_buffer_cache.contains_key(&key) {
                let mut buffer = Buffer::new(
                    &mut self.font_system,
                    Metrics::new(request.size, request.size * LINE_HEIGHT_FACTOR),
                );

                // Set size based on whether wrapping is enabled
                let buffer_width = request.wrap_width.unwrap_or(width as f32);
                buffer.set_size(&mut self.font_system, Some(buffer_width), Some(height as f32));

                // Enable word wrapping if wrap_width is specified
                if request.wrap_width.is_some() {
                    buffer.set_wrap(&mut self.font_system, Wrap::Word);
                }

                buffer.set_text(
                    &mut self.font_system,
                    &request.text,
                    &Attrs::new().family(Family::SansSerif),
                    Shaping::Advanced,
                );

                // Set text alignment for each line
                let align = match request.align {
                    TextAlign::Left => glyphon::cosmic_text::Align::Left,
                    TextAlign::Center => glyphon::cosmic_text::Align::Center,
                    TextAlign::Right => glyphon::cosmic_text::Align::Right,
                };
                for line in buffer.lines.iter_mut() {
                    line.set_align(Some(align));
                }

                buffer.shape_until_scroll(&mut self.font_system, false);

                self.text_buffer_cache.insert(key, buffer);
            }
        }

        // Create text areas using cached buffers
        let text_areas: Vec<TextArea> = requests
            .iter()
            .filter_map(|request| {
                let key = TextCacheKey::new(&request.text, request.size, request.wrap_width, request.align);
                let buffer = self.text_buffer_cache.get(&key)?;

                let bounds = if let Some(clip) = &request.clip {
                    TextBounds {
                        left: clip.x as i32,
                        top: clip.y as i32,
                        right: (clip.x + clip.width) as i32,
                        bottom: (clip.y + clip.height) as i32,
                    }
                } else {
                    TextBounds {
                        left: 0,
                        top: 0,
                        right: width as i32,
                        bottom: height as i32,
                    }
                };

                Some(TextArea {
                    buffer,
                    left: request.x,
                    top: request.y,
                    scale: 1.0,
                    bounds,
                    default_color: request.color.to_glyphon(),
                    custom_glyphs: &[],
                })
            })
            .collect();

        // Select appropriate renderer and atlas
        let (text_renderer, text_atlas) = if is_overlay {
            (&mut self.overlay_text_renderer, &mut self.overlay_text_atlas)
        } else {
            (&mut self.text_renderer, &mut self.text_atlas)
        };

        // Prepare text renderer
        if let Err(e) = text_renderer.prepare(
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &mut self.font_system,
            text_atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        ) {
            log::error!("Failed to prepare text: {:?}", e);
            return;
        }

        // Render text
        {
            let label = if is_overlay { "Overlay Text Render Pass" } else { "Text Render Pass" };
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Err(e) = text_renderer.render(text_atlas, &self.viewport, &mut render_pass) {
                log::error!("Failed to render text: {:?}", e);
            }
        }
    }
}
