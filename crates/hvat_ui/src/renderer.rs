//! Renderer for the hvat_ui framework.
//!
//! The renderer provides a simple, immediate-mode drawing API for widgets.
//! It collects draw commands during the draw phase, then executes them in
//! the correct order with proper clipping and transforms.

use std::collections::HashMap;
use std::sync::Arc;
use winit::window::Window;

use hvat_gpu::{
    ColorPipeline, ColorVertex, GpuContext, ImageAdjustments, TexturePipeline, Texture, TransformUniform,
    HyperspectralPipeline, HyperspectralGpuData, BandSelectionUniform, Pipeline, Vertex,
};
use wgpu::util::DeviceExt;
use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphonColor, FontSystem, Metrics,
    Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};

use crate::{Element, Limits, Rectangle, Point};

/// RGBA color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const RED: Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create a new color with the same RGB but different alpha.
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

/// A draw command to be executed during rendering.
/// All coordinates are in screen space.
#[derive(Debug, Clone)]
enum DrawCommand {
    FillRect {
        rect: Rectangle,
        color: Color,
    },
    StrokeRect {
        rect: Rectangle,
        color: Color,
        width: f32,
    },
    DrawLine {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        width: f32,
    },
    FillCircle {
        cx: f32,
        cy: f32,
        radius: f32,
        color: Color,
    },
    StrokeCircle {
        cx: f32,
        cy: f32,
        radius: f32,
        color: Color,
        width: f32,
    },
    DrawText {
        text: String,
        position: Point,
        color: Color,
        size: f32,
    },
    DrawImage {
        handle: crate::ImageHandle,
        rect: Rectangle,
        pan: (f32, f32),
        zoom: f32,
        adjustments: ImageAdjustments,
    },
    /// Draw a hyperspectral image with GPU-based band compositing
    DrawHyperspectralImage {
        handle: crate::HyperspectralImageHandle,
        rect: Rectangle,
        pan: (f32, f32),
        zoom: f32,
        band_selection: BandSelectionUniform,
        adjustments: ImageAdjustments,
    },
    /// Push a clip rectangle (screen space)
    PushClip(Rectangle),
    /// Pop the clip rectangle
    PopClip,
}

/// Rendering state that tracks transforms and clips.
/// This is used during the draw phase to transform coordinates.
#[derive(Debug)]
struct RenderState {
    /// Current scroll Y offset (accumulated from nested scrollables)
    scroll_y: f32,
    /// Current scroll X offset (accumulated from nested scrollables)
    scroll_x: f32,
    /// Current clip rectangle in screen space (None = full viewport)
    clip: Option<Rectangle>,
    /// Stack of saved states for push/pop
    stack: Vec<(f32, f32, Option<Rectangle>)>,
}

impl RenderState {
    fn new() -> Self {
        Self {
            scroll_y: 0.0,
            scroll_x: 0.0,
            clip: None,
            stack: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.scroll_y = 0.0;
        self.scroll_x = 0.0;
        self.clip = None;
        self.stack.clear();
    }

    /// Push current state and apply vertical scroll offset
    fn push_scroll_y(&mut self, offset: f32) {
        self.stack.push((self.scroll_y, self.scroll_x, self.clip));
        self.scroll_y += offset;
    }

    /// Push current state and apply horizontal scroll offset
    fn push_scroll_x(&mut self, offset: f32) {
        self.stack.push((self.scroll_y, self.scroll_x, self.clip));
        self.scroll_x += offset;
    }

    /// Push current state and apply clip (intersecting with current clip)
    fn push_clip(&mut self, clip_in_layout_space: Rectangle) {
        self.stack.push((self.scroll_y, self.scroll_x, self.clip));

        // Transform clip from layout space to screen space using current scroll offsets.
        // This is correct because:
        // - Scrollable calls push_clip BEFORE push_scroll, so its viewport clip uses
        //   parent's scroll (or 0), keeping the viewport fixed on screen.
        // - Child widgets call push_clip AFTER scrollable's push_scroll, so their
        //   clips get transformed to match where the content actually renders.
        let screen_clip = Rectangle::new(
            clip_in_layout_space.x - self.scroll_x,  // Transform by current horizontal scroll
            clip_in_layout_space.y - self.scroll_y,  // Transform by current vertical scroll
            clip_in_layout_space.width,
            clip_in_layout_space.height,
        );

        log::debug!(
            "push_clip: layout={:?} -> screen={:?}, scroll=({}, {})",
            clip_in_layout_space, screen_clip, self.scroll_x, self.scroll_y
        );

        // Intersect with current clip
        self.clip = Some(match self.clip {
            Some(current) => current.intersect(&screen_clip),
            None => screen_clip,
        });
    }

    /// Pop to previous state
    fn pop(&mut self) {
        if let Some((scroll_y, scroll_x, clip)) = self.stack.pop() {
            self.scroll_y = scroll_y;
            self.scroll_x = scroll_x;
            self.clip = clip;
        }
    }

    /// Transform a point from layout space to screen space
    fn to_screen_point(&self, x: f32, y: f32) -> (f32, f32) {
        (x - self.scroll_x, y - self.scroll_y)
    }

    /// Transform a rectangle from layout space to screen space
    /// Clamps negative dimensions to 0
    fn to_screen_rect(&self, rect: Rectangle) -> Rectangle {
        Rectangle::new(
            rect.x - self.scroll_x,
            rect.y - self.scroll_y,
            rect.width.max(0.0),
            rect.height.max(0.0),
        )
    }
}

/// Cached hyperspectral GPU data (bind group is stored inside gpu_data)
struct CachedHyperspectralData {
    gpu_data: HyperspectralGpuData,
}

/// The renderer abstracts away GPU rendering details from widgets.
pub struct Renderer {
    gpu_ctx: GpuContext,
    texture_pipeline: TexturePipeline,
    color_pipeline: ColorPipeline,
    hyperspectral_pipeline: Option<HyperspectralPipeline>,
    // Glyphon text rendering
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_cache: Cache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    /// Separate text renderer for overlay text (tooltips, modals)
    /// This allows main text and overlay text to be rendered in separate passes
    overlay_text_renderer: TextRenderer,
    viewport: Viewport,
    width: u32,
    height: u32,
    /// Draw commands collected during draw phase
    commands: Vec<DrawCommand>,
    /// Overlay draw commands (rendered after main UI, above other content)
    overlay_commands: Vec<DrawCommand>,
    /// Whether we're currently recording to overlay
    recording_overlay: bool,
    /// Render state for coordinate transforms
    state: RenderState,
    /// Cache for GPU textures (keyed by ImageHandle unique ID)
    texture_cache: HashMap<u64, (Texture, wgpu::BindGroup)>,
    /// Cache for hyperspectral GPU data (keyed by HyperspectralImageHandle unique ID)
    hyperspectral_cache: HashMap<u64, CachedHyperspectralData>,
}

impl Renderer {
    /// Create a new renderer for the given window.
    pub async fn new(window: Arc<Window>) -> Result<Self, String> {
        let gpu_ctx = GpuContext::new(window)
            .await
            .map_err(|e| format!("Failed to create GPU context: {:?}", e))?;

        let texture_pipeline = TexturePipeline::new(&gpu_ctx);
        let format = gpu_ctx.surface_config.format;
        let color_pipeline = ColorPipeline::new(&gpu_ctx.device, format);

        let width = gpu_ctx.width();
        let height = gpu_ctx.height();

        // Initialize glyphon text rendering
        // For WASM, we need to create a font system with embedded fonts
        // since system fonts aren't available
        #[cfg(target_arch = "wasm32")]
        let font_system = {
            // For WASM, we must use new_with_locale_and_db to set the correct font family names.
            // new_with_fonts() hardcodes "Noto Sans Mono" etc which don't match our embedded font.
            // See: https://github.com/pop-os/cosmic-text/issues/126
            let mut db = cosmic_text::fontdb::Database::new();
            let font_data: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");
            db.load_font_data(font_data.to_vec());
            // Set our font as the default for all generic family lookups
            db.set_monospace_family("DejaVu Sans Mono");
            db.set_sans_serif_family("DejaVu Sans Mono");
            db.set_serif_family("DejaVu Sans Mono");
            FontSystem::new_with_locale_and_db("en-US".to_string(), db)
        };

        #[cfg(not(target_arch = "wasm32"))]
        let font_system = {
            let mut fs = FontSystem::new();
            // Also load our embedded font as fallback on native
            let font_data = include_bytes!("../assets/DejaVuSansMono.ttf");
            fs.db_mut().load_font_data(font_data.to_vec());
            fs
        };
        let swash_cache = SwashCache::new();
        let text_cache = Cache::new(&gpu_ctx.device);
        let viewport = Viewport::new(&gpu_ctx.device, &text_cache);
        let mut text_atlas = TextAtlas::new(&gpu_ctx.device, &gpu_ctx.queue, &text_cache, format);
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            &gpu_ctx.device,
            wgpu::MultisampleState::default(),
            None,
        );
        let overlay_text_renderer = TextRenderer::new(
            &mut text_atlas,
            &gpu_ctx.device,
            wgpu::MultisampleState::default(),
            None,
        );

        // Create hyperspectral pipeline
        let hyperspectral_pipeline = Some(HyperspectralPipeline::new(&gpu_ctx));

        Ok(Self {
            gpu_ctx,
            texture_pipeline,
            color_pipeline,
            hyperspectral_pipeline,
            font_system,
            swash_cache,
            text_cache,
            text_atlas,
            text_renderer,
            overlay_text_renderer,
            viewport,
            width,
            height,
            commands: Vec::new(),
            overlay_commands: Vec::new(),
            recording_overlay: false,
            state: RenderState::new(),
            texture_cache: HashMap::new(),
            hyperspectral_cache: HashMap::new(),
        })
    }

    /// Resize the renderer.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.gpu_ctx.resize(width, height);
    }

    /// Get the current surface size.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get the viewport width.
    pub fn viewport_width(&self) -> f32 {
        self.width as f32
    }

    /// Get the viewport height.
    pub fn viewport_height(&self) -> f32 {
        self.height as f32
    }

    /// Get a reference to the GPU context.
    pub fn gpu_context(&self) -> &GpuContext {
        &self.gpu_ctx
    }

    /// Get a reference to the texture pipeline.
    pub fn texture_pipeline(&self) -> &TexturePipeline {
        &self.texture_pipeline
    }

    // =========================================================================
    // Drawing API - called by widgets during draw phase
    // =========================================================================

    /// Draw a filled rectangle.
    pub fn fill_rect(&mut self, rect: Rectangle, color: Color) {
        let screen_rect = self.state.to_screen_rect(rect);

        // Emit clip commands if needed, then draw
        self.emit_with_clip(DrawCommand::FillRect {
            rect: screen_rect,
            color,
        });
    }

    /// Draw a rectangle outline.
    pub fn stroke_rect(&mut self, rect: Rectangle, color: Color, width: f32) {
        let screen_rect = self.state.to_screen_rect(rect);

        self.emit_with_clip(DrawCommand::StrokeRect {
            rect: screen_rect,
            color,
            width,
        });
    }

    /// Draw a line segment.
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32) {
        let (sx1, sy1) = self.state.to_screen_point(x1, y1);
        let (sx2, sy2) = self.state.to_screen_point(x2, y2);

        self.emit_with_clip(DrawCommand::DrawLine {
            x1: sx1,
            y1: sy1,
            x2: sx2,
            y2: sy2,
            color,
            width,
        });
    }

    /// Draw a filled circle.
    pub fn fill_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        let (scx, scy) = self.state.to_screen_point(cx, cy);

        self.emit_with_clip(DrawCommand::FillCircle {
            cx: scx,
            cy: scy,
            radius,
            color,
        });
    }

    /// Draw a circle outline.
    pub fn stroke_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color, width: f32) {
        let (scx, scy) = self.state.to_screen_point(cx, cy);

        self.emit_with_clip(DrawCommand::StrokeCircle {
            cx: scx,
            cy: scy,
            radius,
            color,
            width,
        });
    }

    /// Draw text at a position.
    pub fn draw_text(&mut self, text: &str, position: Point, color: Color, size: f32) {
        let (screen_x, screen_y) = self.state.to_screen_point(position.x, position.y);

        self.emit_with_clip(DrawCommand::DrawText {
            text: text.to_string(),
            position: Point::new(screen_x, screen_y),
            color,
            size,
        });
    }

    /// Draw an image.
    pub fn draw_image(&mut self, handle: &crate::ImageHandle, rect: Rectangle) {
        self.draw_image_with_adjustments(handle, rect, (0.0, 0.0), 1.0, ImageAdjustments::new());
    }

    /// Draw an image with pan and zoom.
    pub fn draw_image_transformed(
        &mut self,
        handle: &crate::ImageHandle,
        rect: Rectangle,
        pan: (f32, f32),
        zoom: f32,
    ) {
        self.draw_image_with_adjustments(handle, rect, pan, zoom, ImageAdjustments::new());
    }

    /// Draw an image with full control.
    pub fn draw_image_with_adjustments(
        &mut self,
        handle: &crate::ImageHandle,
        rect: Rectangle,
        pan: (f32, f32),
        zoom: f32,
        adjustments: ImageAdjustments,
    ) {
        let screen_rect = self.state.to_screen_rect(rect);

        self.emit_with_clip(DrawCommand::DrawImage {
            handle: handle.clone(),
            rect: screen_rect,
            pan,
            zoom,
            adjustments,
        });
    }

    /// Draw a texture (low-level).
    pub fn draw_texture(&mut self, _texture: &Texture, _rect: Rectangle) {
        // TODO: Implement if needed
    }

    /// Draw a hyperspectral image with GPU-based band compositing.
    ///
    /// This is the main entry point for rendering hyperspectral images.
    /// Band selection happens on the GPU, so changing bands only updates
    /// a uniform buffer - no CPU-side image regeneration needed.
    pub fn draw_hyperspectral_image(
        &mut self,
        handle: &crate::HyperspectralImageHandle,
        rect: Rectangle,
        band_selection: BandSelectionUniform,
    ) {
        self.draw_hyperspectral_image_with_adjustments(
            handle,
            rect,
            (0.0, 0.0),
            1.0,
            band_selection,
            ImageAdjustments::new(),
        );
    }

    /// Draw a hyperspectral image with pan/zoom and band selection.
    pub fn draw_hyperspectral_image_transformed(
        &mut self,
        handle: &crate::HyperspectralImageHandle,
        rect: Rectangle,
        pan: (f32, f32),
        zoom: f32,
        band_selection: BandSelectionUniform,
    ) {
        self.draw_hyperspectral_image_with_adjustments(
            handle,
            rect,
            pan,
            zoom,
            band_selection,
            ImageAdjustments::new(),
        );
    }

    /// Draw a hyperspectral image with full control.
    pub fn draw_hyperspectral_image_with_adjustments(
        &mut self,
        handle: &crate::HyperspectralImageHandle,
        rect: Rectangle,
        pan: (f32, f32),
        zoom: f32,
        band_selection: BandSelectionUniform,
        adjustments: ImageAdjustments,
    ) {
        let screen_rect = self.state.to_screen_rect(rect);

        self.emit_with_clip(DrawCommand::DrawHyperspectralImage {
            handle: handle.clone(),
            rect: screen_rect,
            pan,
            zoom,
            band_selection,
            adjustments,
        });
    }

    // =========================================================================
    // Clip and scroll API
    // =========================================================================

    /// Push a clip rectangle. All subsequent drawing will be clipped to this rect.
    /// The rect is in layout space and will be transformed to screen space.
    pub fn push_clip(&mut self, rect: Rectangle) {
        self.state.push_clip(rect);
    }

    /// Pop the most recent clip rectangle.
    pub fn pop_clip(&mut self) {
        self.state.pop();
    }

    /// Get the current clip rectangle (in screen space).
    pub fn current_clip(&self) -> Option<Rectangle> {
        self.state.clip
    }

    /// Push a vertical scroll offset. All subsequent drawing will be offset by this amount.
    pub fn push_scroll_offset_y(&mut self, offset: f32) {
        self.state.push_scroll_y(offset);
    }

    /// Push a horizontal scroll offset. All subsequent drawing will be offset by this amount.
    pub fn push_scroll_offset_x(&mut self, offset: f32) {
        self.state.push_scroll_x(offset);
    }

    /// Pop the most recent vertical scroll offset.
    pub fn pop_scroll_offset_y(&mut self) {
        self.state.pop();
    }

    /// Pop the most recent horizontal scroll offset.
    pub fn pop_scroll_offset_x(&mut self) {
        self.state.pop();
    }

    /// Get the total accumulated vertical scroll offset.
    pub fn total_scroll_offset_y(&self) -> f32 {
        self.state.scroll_y
    }

    /// Get the total accumulated horizontal scroll offset.
    pub fn total_scroll_offset_x(&self) -> f32 {
        self.state.scroll_x
    }

    // =========================================================================
    // Overlay API
    // =========================================================================

    /// Begin recording draw commands to the overlay layer.
    /// Overlay commands are rendered after all main UI commands,
    /// appearing above other content (except scrollbars).
    pub fn begin_overlay(&mut self) {
        self.recording_overlay = true;
    }

    /// End recording to the overlay layer.
    pub fn end_overlay(&mut self) {
        self.recording_overlay = false;
    }

    /// Check if currently recording to overlay.
    pub fn is_recording_overlay(&self) -> bool {
        self.recording_overlay
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Emit a draw command, wrapping it with clip commands if a clip is active.
    /// Note: Overlay commands bypass clipping to allow tooltips/dropdowns to extend
    /// outside their parent container bounds.
    fn emit_with_clip(&mut self, cmd: DrawCommand) {
        // Overlay commands bypass clipping entirely - they should render above
        // everything and not be constrained by parent container clips
        if self.recording_overlay {
            self.overlay_commands.push(cmd);
            return;
        }

        if let Some(clip) = self.state.clip {
            // Only draw if the command is visible within the clip
            let should_draw = match &cmd {
                DrawCommand::FillRect { rect, .. } |
                DrawCommand::StrokeRect { rect, .. } |
                DrawCommand::DrawImage { rect, .. } |
                DrawCommand::DrawHyperspectralImage { rect, .. } => {
                    rects_overlap(rect, &clip)
                }
                DrawCommand::DrawLine { x1, y1, x2, y2, width, .. } => {
                    // Approximate line bounds
                    let min_x = x1.min(*x2) - width;
                    let min_y = y1.min(*y2) - width;
                    let max_x = x1.max(*x2) + width;
                    let max_y = y1.max(*y2) + width;
                    let line_rect = Rectangle::new(min_x, min_y, max_x - min_x, max_y - min_y);
                    rects_overlap(&line_rect, &clip)
                }
                DrawCommand::FillCircle { cx, cy, radius, .. } |
                DrawCommand::StrokeCircle { cx, cy, radius, .. } => {
                    let circle_rect = Rectangle::new(cx - radius, cy - radius, radius * 2.0, radius * 2.0);
                    rects_overlap(&circle_rect, &clip)
                }
                DrawCommand::DrawText { position, size, .. } => {
                    // Approximate text bounds
                    let text_rect = Rectangle::new(
                        position.x,
                        position.y,
                        1000.0, // Generous width
                        *size * 1.5,
                    );
                    rects_overlap(&text_rect, &clip)
                }
                _ => true,
            };

            if should_draw {
                self.commands.push(DrawCommand::PushClip(clip));
                self.commands.push(cmd);
                self.commands.push(DrawCommand::PopClip);
            }
        } else {
            self.commands.push(cmd);
        }
    }

    /// Convert clip rectangle to scissor parameters.
    fn clip_to_scissor(&self, clip: &Rectangle) -> Option<(u32, u32, u32, u32)> {
        let left = clip.x.max(0.0);
        let top = clip.y.max(0.0);
        let right = (clip.x + clip.width).min(self.width as f32).max(0.0);
        let bottom = (clip.y + clip.height).min(self.height as f32).max(0.0);

        let w = (right - left) as u32;
        let h = (bottom - top) as u32;

        if w > 0 && h > 0 {
            Some((left as u32, top as u32, w, h))
        } else {
            None
        }
    }

    // =========================================================================
    // Render execution
    // =========================================================================

    /// Render an element tree.
    pub fn render<Message>(&mut self, element: Element<Message>) {
        // Clear state from last frame
        self.commands.clear();
        self.overlay_commands.clear();
        self.recording_overlay = false;
        self.state.clear();

        // Layout the element
        let limits = Limits::new(self.width as f32, self.height as f32);
        let layout = element.widget().layout(&limits);

        // Collect draw commands
        element.widget().draw(self, &layout);

        // Execute rendering
        self.execute_render();
    }

    fn execute_render(&mut self) {
        // Get frame
        let frame = match self.gpu_ctx.surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                log::error!("Failed to get frame: {:?}", e);
                return;
            }
        };

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Prepare textures
        self.prepare_textures();

        // Create command encoder
        let mut encoder = self.gpu_ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") }
        );

        // Clear pass
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        // Render large images first (they're background content like the main viewer)
        self.render_images(&mut encoder, &view, false);

        // Render main UI shapes (buttons, panels, etc.)
        self.render_ui_shapes(&mut encoder, &view);

        // Render small images (icons) AFTER main UI shapes so they appear on top of button backgrounds
        // but BEFORE overlay so tooltips appear on top of icons
        self.render_images(&mut encoder, &view, true);

        // Render main UI text BEFORE overlay shapes
        // This ensures text is visible but can be covered by overlay backgrounds
        self.render_all_text_main(&mut encoder, &view);

        // Render overlay shapes (tooltips, modals) on top of main UI including its text
        if !self.overlay_commands.is_empty() {
            self.render_overlay_shapes(&mut encoder, &view);
        }

        // Render overlay text AFTER overlay shapes so tooltip text appears on top
        self.render_all_text_overlay(&mut encoder, &view);

        // Submit and present
        self.gpu_ctx.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    fn prepare_textures(&mut self) {
        for cmd in &self.commands {
            match cmd {
                DrawCommand::DrawImage { handle, .. } => {
                    // Use the unique image ID as cache key instead of memory pointer.
                    // This ensures that new image data always gets a fresh GPU texture,
                    // even if the allocator reuses the same memory address.
                    let key = handle.id();
                    if !self.texture_cache.contains_key(&key) {
                        log::debug!("Creating texture for image id={}, size={}x{}, data_len={}",
                            key, handle.width(), handle.height(), handle.data().len());
                        match Texture::from_rgba8(&self.gpu_ctx, handle.data(), handle.width(), handle.height()) {
                            Ok(texture) => {
                                log::debug!("Texture created successfully for id={}", key);
                                let bind_group = self.texture_pipeline.create_texture_bind_group(&self.gpu_ctx, &texture);
                                self.texture_cache.insert(key, (texture, bind_group));
                            }
                            Err(e) => log::error!("Failed to create texture for id={}: {:?}", key, e),
                        }
                    }
                }
                DrawCommand::DrawHyperspectralImage { handle, .. } => {
                    // Prepare hyperspectral GPU data if not cached
                    let key = handle.id();
                    if !self.hyperspectral_cache.contains_key(&key) {
                        if let Some(ref pipeline) = self.hyperspectral_pipeline {
                            // Upload band data to GPU (the bind group is created internally)
                            let gpu_data = HyperspectralGpuData::from_bands(
                                &self.gpu_ctx,
                                handle.bands(),
                                handle.width(),
                                handle.height(),
                                pipeline.band_texture_layout(),
                            );
                            self.hyperspectral_cache.insert(key, CachedHyperspectralData {
                                gpu_data,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Render images.
    /// If `small_only` is true, only render small images (icons, <=32px).
    /// If `small_only` is false, only render large images (main content).
    fn render_images(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, small_only: bool) {
        // Track current scissor for proper clipping
        let mut clip_stack: Vec<Rectangle> = Vec::new();

        for cmd in &self.commands {
            match cmd {
                DrawCommand::PushClip(rect) => {
                    clip_stack.push(*rect);
                }
                DrawCommand::PopClip => {
                    clip_stack.pop();
                }
                DrawCommand::DrawImage { handle, rect, pan, zoom, adjustments } => {
                    // Filter based on image size
                    let is_small = handle.width() <= 32 && handle.height() <= 32;
                    if is_small != small_only {
                        continue;
                    }

                    let cache_key = handle.id();
                    if let Some((_texture, bind_group)) = self.texture_cache.get(&cache_key) {
                        // For small images (icons), use per-icon vertex buffers with baked positions
                        // to avoid uniform buffer synchronization issues.
                        // Note: Reusable buffer optimization was attempted but caused rendering issues
                        // due to queue.write_buffer timing with render pass submission.
                        if is_small {
                            // Create vertex buffer with positions baked in (NDC coordinates)
                            let (vertices, indices) = self.create_icon_vertices(rect);

                            let vertex_buffer = self.gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Icon Vertex Buffer"),
                                contents: bytemuck::cast_slice(&vertices),
                                usage: wgpu::BufferUsages::VERTEX,
                            });
                            let index_buffer = self.gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Icon Index Buffer"),
                                contents: bytemuck::cast_slice(&indices),
                                usage: wgpu::BufferUsages::INDEX,
                            });

                            // Use identity transform since positions are already in NDC
                            self.texture_pipeline.update_transform(&self.gpu_ctx, TransformUniform::new());
                            self.texture_pipeline.update_adjustments(&self.gpu_ctx, *adjustments);

                            // Render
                            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Icon Pass"),
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

                            // Set scissor
                            if let Some(clip) = clip_stack.last() {
                                if let Some((x, y, w, h)) = self.clip_to_scissor(clip) {
                                    pass.set_scissor_rect(x, y, w, h);
                                }
                            } else {
                                pass.set_scissor_rect(0, 0, self.width, self.height);
                            }

                            pass.set_pipeline(&self.texture_pipeline.render_pipeline);
                            pass.set_bind_group(0, &self.texture_pipeline.uniform_bind_group, &[]);
                            pass.set_bind_group(1, bind_group, &[]);
                            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                            pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
                        } else {
                            // For large images, use the standard transform-based approach
                            let transform = self.calculate_image_transform(handle, rect, *pan, *zoom);
                            self.texture_pipeline.update_transform(&self.gpu_ctx, transform);
                            self.texture_pipeline.update_adjustments(&self.gpu_ctx, *adjustments);

                            // Render
                            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Image Pass"),
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

                            // Set scissor
                            if let Some(clip) = clip_stack.last() {
                                if let Some((x, y, w, h)) = self.clip_to_scissor(clip) {
                                    pass.set_scissor_rect(x, y, w, h);
                                }
                            } else {
                                pass.set_scissor_rect(0, 0, self.width, self.height);
                            }

                            pass.set_pipeline(&self.texture_pipeline.render_pipeline);
                            pass.set_bind_group(0, &self.texture_pipeline.uniform_bind_group, &[]);
                            pass.set_bind_group(1, bind_group, &[]);
                            pass.set_vertex_buffer(0, self.texture_pipeline.vertex_buffer.slice(..));
                            pass.set_index_buffer(self.texture_pipeline.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                            pass.draw_indexed(0..self.texture_pipeline.num_indices, 0, 0..1);
                        }
                    }
                }
                DrawCommand::DrawHyperspectralImage { handle, rect, pan, zoom, band_selection, adjustments } => {
                    // Hyperspectral images are always "large" content - skip in small_only pass
                    if small_only {
                        continue;
                    }

                    let cache_key = handle.id();
                    if let (Some(cached), Some(ref pipeline)) = (
                        self.hyperspectral_cache.get(&cache_key),
                        &self.hyperspectral_pipeline,
                    ) {
                        // Calculate transform
                        let transform = self.calculate_hyperspectral_transform(handle, rect, *pan, *zoom);
                        pipeline.update_transform(&self.gpu_ctx, transform);
                        pipeline.update_adjustments(&self.gpu_ctx, *adjustments);
                        pipeline.update_band_selection(&self.gpu_ctx, *band_selection);

                        // Render
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Hyperspectral Image Pass"),
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

                        // Set scissor
                        if let Some(clip) = clip_stack.last() {
                            if let Some((x, y, w, h)) = self.clip_to_scissor(clip) {
                                pass.set_scissor_rect(x, y, w, h);
                            }
                        } else {
                            pass.set_scissor_rect(0, 0, self.width, self.height);
                        }

                        pass.set_pipeline(&pipeline.render_pipeline);
                        pass.set_bind_group(0, &pipeline.uniform_bind_group, &[]);
                        pass.set_bind_group(1, &cached.gpu_data.bind_group, &[]);
                        pass.set_vertex_buffer(0, pipeline.vertex_buffer.slice(..));
                        pass.set_index_buffer(pipeline.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        pass.draw_indexed(0..pipeline.num_indices, 0, 0..1);
                    }
                }
                _ => {}
            }
        }
    }

    /// Calculate transform matrix for rendering an image with pan and zoom.
    /// This is the shared implementation used by both regular and hyperspectral images.
    fn calculate_transform_for_dimensions(
        &self,
        img_width: u32,
        img_height: u32,
        rect: &Rectangle,
        pan: (f32, f32),
        zoom: f32,
    ) -> TransformUniform {
        // Convert rect position to NDC (Normalized Device Coordinates)
        let ndc_x = (rect.x / self.width as f32) * 2.0 - 1.0;
        let ndc_y = 1.0 - (rect.y / self.height as f32) * 2.0;
        let ndc_w = (rect.width / self.width as f32) * 2.0;
        let ndc_h = (rect.height / self.height as f32) * 2.0;

        // Calculate aspect ratios
        let img_aspect = img_width as f32 / img_height as f32;
        let rect_aspect = rect.width / rect.height;
        let window_aspect = self.width as f32 / self.height as f32;

        // Calculate scale to fit image in rect while preserving aspect ratio
        // We need to account for the window aspect ratio since NDC space is stretched
        let (scale_x, scale_y) = if img_aspect > rect_aspect {
            // Image is wider than rect - fit to width
            let sx = ndc_w / 2.0;
            // Adjust Y scale to preserve aspect ratio, accounting for window stretch
            let sy = sx / img_aspect * window_aspect;
            (sx, sy)
        } else {
            // Image is taller than rect - fit to height
            let sy = ndc_h / 2.0;
            // Adjust X scale to preserve aspect ratio, accounting for window stretch
            let sx = sy * img_aspect / window_aspect;
            (sx, sy)
        };

        let scale_x = scale_x * zoom;
        let scale_y = scale_y * zoom;

        let pan_ndc_x = (pan.0 / self.width as f32) * 2.0;
        let pan_ndc_y = -(pan.1 / self.height as f32) * 2.0;

        let center_x = ndc_x + ndc_w / 2.0 + pan_ndc_x;
        let center_y = ndc_y - ndc_h / 2.0 + pan_ndc_y;

        TransformUniform {
            matrix: [
                [scale_x, 0.0, 0.0, 0.0],
                [0.0, scale_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [center_x, center_y, 0.0, 1.0],
            ],
        }
    }

    fn calculate_image_transform(
        &self,
        handle: &crate::ImageHandle,
        rect: &Rectangle,
        pan: (f32, f32),
        zoom: f32,
    ) -> TransformUniform {
        self.calculate_transform_for_dimensions(handle.width(), handle.height(), rect, pan, zoom)
    }

    fn calculate_hyperspectral_transform(
        &self,
        handle: &crate::HyperspectralImageHandle,
        rect: &Rectangle,
        pan: (f32, f32),
        zoom: f32,
    ) -> TransformUniform {
        self.calculate_transform_for_dimensions(handle.width(), handle.height(), rect, pan, zoom)
    }

    /// Create vertex and index data for an icon at a specific screen rectangle.
    /// Returns vertices with positions in NDC coordinates (for use with identity transform).
    fn create_icon_vertices(&self, rect: &Rectangle) -> (Vec<Vertex>, Vec<u16>) {
        // Convert screen coordinates to NDC
        let left = (rect.x / self.width as f32) * 2.0 - 1.0;
        let right = ((rect.x + rect.width) / self.width as f32) * 2.0 - 1.0;
        let top = 1.0 - (rect.y / self.height as f32) * 2.0;
        let bottom = 1.0 - ((rect.y + rect.height) / self.height as f32) * 2.0;

        let vertices = vec![
            Vertex { position: [left, bottom], tex_coords: [0.0, 1.0] },   // Bottom-left
            Vertex { position: [right, bottom], tex_coords: [1.0, 1.0] }, // Bottom-right
            Vertex { position: [right, top], tex_coords: [1.0, 0.0] },    // Top-right
            Vertex { position: [left, top], tex_coords: [0.0, 0.0] },     // Top-left
        ];

        let indices = vec![0, 1, 2, 0, 2, 3];

        (vertices, indices)
    }

    fn render_ui_shapes(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let batches = self.build_shape_batches(&self.commands, true);
        self.render_shape_batches(encoder, view, &batches, "UI Shapes Pass");
    }

    fn render_overlay_shapes(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let batches = self.build_shape_batches(&self.overlay_commands, false);
        self.render_shape_batches(encoder, view, &batches, "Overlay Shapes Pass");
    }

    /// Build shape batches from draw commands.
    /// If `include_all_shapes` is true, includes lines and circles; otherwise only rects.
    fn build_shape_batches(&self, commands: &[DrawCommand], include_all_shapes: bool) -> Vec<ShapeBatch> {
        let window_width = self.width as f32;
        let window_height = self.height as f32;
        let mut batches: Vec<ShapeBatch> = Vec::new();
        let mut current_scissor: Option<(u32, u32, u32, u32)> = None;

        for cmd in commands {
            match cmd {
                DrawCommand::PushClip(rect) => {
                    current_scissor = self.clip_to_scissor(rect);
                }
                DrawCommand::PopClip => {
                    current_scissor = None;
                }
                DrawCommand::FillRect { rect, color } => {
                    let batch = get_or_create_batch(&mut batches, current_scissor);
                    ColorPipeline::append_rect(
                        &mut batch.vertices, &mut batch.indices,
                        rect.x, rect.y, rect.width, rect.height,
                        [color.r, color.g, color.b, color.a],
                        window_width, window_height,
                    );
                }
                DrawCommand::StrokeRect { rect, color, width } => {
                    let batch = get_or_create_batch(&mut batches, current_scissor);
                    ColorPipeline::append_stroke_rect(
                        &mut batch.vertices, &mut batch.indices,
                        rect.x, rect.y, rect.width, rect.height,
                        [color.r, color.g, color.b, color.a],
                        *width,
                        window_width, window_height,
                    );
                }
                DrawCommand::DrawLine { x1, y1, x2, y2, color, width } if include_all_shapes => {
                    let batch = get_or_create_batch(&mut batches, current_scissor);
                    ColorPipeline::append_line(
                        &mut batch.vertices, &mut batch.indices,
                        *x1, *y1, *x2, *y2,
                        [color.r, color.g, color.b, color.a],
                        *width,
                        window_width, window_height,
                    );
                }
                DrawCommand::FillCircle { cx, cy, radius, color } if include_all_shapes => {
                    let batch = get_or_create_batch(&mut batches, current_scissor);
                    ColorPipeline::append_circle(
                        &mut batch.vertices, &mut batch.indices,
                        *cx, *cy, *radius,
                        [color.r, color.g, color.b, color.a],
                        window_width, window_height,
                    );
                }
                DrawCommand::StrokeCircle { cx, cy, radius, color, width } if include_all_shapes => {
                    let batch = get_or_create_batch(&mut batches, current_scissor);
                    ColorPipeline::append_stroke_circle(
                        &mut batch.vertices, &mut batch.indices,
                        *cx, *cy, *radius,
                        [color.r, color.g, color.b, color.a],
                        *width,
                        window_width, window_height,
                    );
                }
                _ => {}
            }
        }
        batches
    }

    /// Render shape batches to the GPU.
    fn render_shape_batches(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        batches: &[ShapeBatch],
        label: &str,
    ) {
        if batches.is_empty() {
            return;
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

        pass.set_pipeline(self.color_pipeline.render_pipeline());

        for batch in batches {
            if batch.vertices.is_empty() {
                continue;
            }

            // Set scissor
            if let Some((x, y, w, h)) = batch.scissor {
                pass.set_scissor_rect(x, y, w, h);
            } else {
                pass.set_scissor_rect(0, 0, self.width, self.height);
            }

            // Create GPU buffers
            let vertex_buffer = self.gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Shape Vertex Buffer"),
                contents: bytemuck::cast_slice(&batch.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = self.gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Shape Index Buffer"),
                contents: bytemuck::cast_slice(&batch.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..batch.indices.len() as u32, 0, 0..1);
        }
    }

    /// Render main UI text (before overlay shapes).
    fn render_all_text_main(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut text_items: Vec<TextItem> = Vec::new();
        collect_text_items(&self.commands, &mut text_items, &[]);

        if text_items.is_empty() {
            return;
        }

        self.render_text_items(encoder, view, text_items, false);
    }

    /// Render overlay text (after overlay shapes, for tooltip text).
    fn render_all_text_overlay(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut text_items: Vec<TextItem> = Vec::new();
        collect_text_items(&self.overlay_commands, &mut text_items, &[]);

        if text_items.is_empty() {
            return;
        }

        self.render_text_items(encoder, view, text_items, true);
    }

    /// Internal helper to render a list of text items using the appropriate renderer.
    fn render_text_items(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        text_items: Vec<TextItem>,
        use_overlay_renderer: bool,
    ) {
        // Update viewport
        self.viewport.update(
            &self.gpu_ctx.queue,
            Resolution {
                width: self.width,
                height: self.height,
            },
        );

        // Create buffers for each text item
        let mut buffers: Vec<Buffer> = Vec::with_capacity(text_items.len());

        for item in &text_items {
            let metrics = Metrics::new(item.size, item.size * 1.2);
            let mut buffer = Buffer::new(&mut self.font_system, metrics);
            buffer.set_size(&mut self.font_system, Some(self.width as f32), Some(self.height as f32));
            buffer.set_text(
                &mut self.font_system,
                &item.text,
                &Attrs::new(),
                Shaping::Advanced,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
            buffers.push(buffer);
        }

        // Create text areas from buffers
        let mut text_areas: Vec<TextArea> = Vec::with_capacity(text_items.len());
        for (i, item) in text_items.iter().enumerate() {
            let bounds = if let Some(clip) = item.clip {
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
                    right: self.width as i32,
                    bottom: self.height as i32,
                }
            };

            text_areas.push(TextArea {
                buffer: &buffers[i],
                left: item.x,
                top: item.y,
                scale: 1.0,
                bounds,
                default_color: GlyphonColor::rgba(
                    (item.color.r * 255.0) as u8,
                    (item.color.g * 255.0) as u8,
                    (item.color.b * 255.0) as u8,
                    (item.color.a * 255.0) as u8,
                ),
                custom_glyphs: &[],
            });
        }

        // Select the appropriate text renderer
        let text_renderer = if use_overlay_renderer {
            &mut self.overlay_text_renderer
        } else {
            &mut self.text_renderer
        };

        // Prepare text for rendering
        if let Err(e) = text_renderer.prepare(
            &self.gpu_ctx.device,
            &self.gpu_ctx.queue,
            &mut self.font_system,
            &mut self.text_atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        ) {
            log::error!("Failed to prepare text: {:?}", e);
        }

        // Render text
        let label = if use_overlay_renderer { "Overlay Text Pass" } else { "Main Text Pass" };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

            if let Err(e) = text_renderer.render(&self.text_atlas, &self.viewport, &mut pass) {
                log::error!("Failed to render text: {:?}", e);
            }
        }

        // Trim atlas to free unused space
        self.text_atlas.trim();
    }
}

/// Check if two rectangles overlap.
fn rects_overlap(a: &Rectangle, b: &Rectangle) -> bool {
    a.x < b.x + b.width &&
    a.x + a.width > b.x &&
    a.y < b.y + b.height &&
    a.y + a.height > b.y
}

/// A batch of shapes sharing the same scissor state.
struct ShapeBatch {
    scissor: Option<(u32, u32, u32, u32)>,
    vertices: Vec<ColorVertex>,
    indices: Vec<u16>,
}

/// Get or create a batch for the given scissor state.
fn get_or_create_batch(batches: &mut Vec<ShapeBatch>, scissor: Option<(u32, u32, u32, u32)>) -> &mut ShapeBatch {
    // Find existing batch with same scissor, or create new one
    let idx = if let Some(pos) = batches.iter().position(|b| b.scissor == scissor) {
        pos
    } else {
        batches.push(ShapeBatch {
            scissor,
            vertices: Vec::new(),
            indices: Vec::new(),
        });
        batches.len() - 1
    };
    &mut batches[idx]
}

/// A text item collected for rendering.
struct TextItem {
    text: String,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    clip: Option<Rectangle>,
}

/// Collect text items from draw commands.
/// If `exclude_rects` is non-empty, text overlapping those rectangles is skipped.
fn collect_text_items(commands: &[DrawCommand], items: &mut Vec<TextItem>, exclude_rects: &[Rectangle]) {
    let mut current_clip: Option<Rectangle> = None;
    for cmd in commands {
        match cmd {
            DrawCommand::PushClip(rect) => current_clip = Some(*rect),
            DrawCommand::PopClip => current_clip = None,
            DrawCommand::DrawText { text, position, color, size } => {
                // Skip text that overlaps with exclusion rects
                if !exclude_rects.is_empty() {
                    let text_rect = Rectangle::new(
                        position.x,
                        position.y,
                        text.len() as f32 * size * 0.6,  // Approximate width
                        *size * 1.2,                      // Approximate height
                    );
                    if exclude_rects.iter().any(|r| rects_overlap(&text_rect, r)) {
                        continue;
                    }
                }
                items.push(TextItem {
                    text: text.clone(),
                    x: position.x,
                    y: position.y,
                    size: *size,
                    color: *color,
                    clip: current_clip,
                });
            }
            _ => {}
        }
    }
}
