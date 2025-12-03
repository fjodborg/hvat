//! Renderer for the hvat_ui framework.
//!
//! The renderer provides a simple, immediate-mode drawing API for widgets.
//! It collects draw commands during the draw phase, then executes them in
//! the correct order with proper clipping and transforms.

use std::collections::HashMap;
use std::sync::Arc;
use winit::window::Window;

use hvat_gpu::{ColorPipeline, GpuContext, ImageAdjustments, TexturePipeline, Texture, TransformUniform};
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
    /// Current clip rectangle in screen space (None = full viewport)
    clip: Option<Rectangle>,
    /// Stack of saved states for push/pop
    stack: Vec<(f32, Option<Rectangle>)>,
}

impl RenderState {
    fn new() -> Self {
        Self {
            scroll_y: 0.0,
            clip: None,
            stack: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.scroll_y = 0.0;
        self.clip = None;
        self.stack.clear();
    }

    /// Push current state and apply scroll offset
    fn push_scroll(&mut self, offset: f32) {
        self.stack.push((self.scroll_y, self.clip));
        self.scroll_y += offset;
    }

    /// Push current state and apply clip (intersecting with current clip)
    fn push_clip(&mut self, clip_in_layout_space: Rectangle) {
        self.stack.push((self.scroll_y, self.clip));

        // Transform clip from layout space to screen space using current scroll_y.
        // This is correct because:
        // - Scrollable calls push_clip BEFORE push_scroll, so its viewport clip uses
        //   parent's scroll_y (or 0), keeping the viewport fixed on screen.
        // - Child widgets call push_clip AFTER scrollable's push_scroll, so their
        //   clips get transformed to match where the content actually renders.
        let screen_clip = Rectangle::new(
            clip_in_layout_space.x,
            clip_in_layout_space.y - self.scroll_y,  // Transform by current scroll
            clip_in_layout_space.width,
            clip_in_layout_space.height,
        );

        log::debug!(
            "push_clip: layout={:?} -> screen={:?}, scroll_y={}",
            clip_in_layout_space, screen_clip, self.scroll_y
        );

        // Intersect with current clip
        self.clip = Some(match self.clip {
            Some(current) => current.intersect(&screen_clip),
            None => screen_clip,
        });
    }

    /// Pop to previous state
    fn pop(&mut self) {
        if let Some((scroll_y, clip)) = self.stack.pop() {
            self.scroll_y = scroll_y;
            self.clip = clip;
        }
    }

    /// Transform a point from layout space to screen space
    fn to_screen_point(&self, x: f32, y: f32) -> (f32, f32) {
        (x, y - self.scroll_y)
    }

    /// Transform a rectangle from layout space to screen space
    /// Clamps negative dimensions to 0
    fn to_screen_rect(&self, rect: Rectangle) -> Rectangle {
        Rectangle::new(
            rect.x,
            rect.y - self.scroll_y,
            rect.width.max(0.0),
            rect.height.max(0.0),
        )
    }
}

/// The renderer abstracts away GPU rendering details from widgets.
pub struct Renderer {
    gpu_ctx: GpuContext,
    texture_pipeline: TexturePipeline,
    color_pipeline: ColorPipeline,
    // Glyphon text rendering
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_cache: Cache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    width: u32,
    height: u32,
    /// Draw commands collected during draw phase
    commands: Vec<DrawCommand>,
    /// Render state for coordinate transforms
    state: RenderState,
    /// Cache for GPU textures
    texture_cache: HashMap<usize, (Texture, wgpu::BindGroup)>,
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

        Ok(Self {
            gpu_ctx,
            texture_pipeline,
            color_pipeline,
            font_system,
            swash_cache,
            text_cache,
            text_atlas,
            text_renderer,
            viewport,
            width,
            height,
            commands: Vec::new(),
            state: RenderState::new(),
            texture_cache: HashMap::new(),
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

    /// Push a scroll offset. All subsequent drawing will be offset by this amount.
    pub fn push_scroll_offset(&mut self, offset: f32) {
        self.state.push_scroll(offset);
    }

    /// Pop the most recent scroll offset.
    pub fn pop_scroll_offset(&mut self) {
        self.state.pop();
    }

    /// Get the total accumulated scroll offset.
    pub fn total_scroll_offset(&self) -> f32 {
        self.state.scroll_y
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Emit a draw command, wrapping it with clip commands if a clip is active.
    fn emit_with_clip(&mut self, cmd: DrawCommand) {
        if let Some(clip) = self.state.clip {
            // Only draw if the command is visible within the clip
            let should_draw = match &cmd {
                DrawCommand::FillRect { rect, .. } |
                DrawCommand::StrokeRect { rect, .. } |
                DrawCommand::DrawImage { rect, .. } => {
                    rects_overlap(rect, &clip)
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

        // Render images first (they're background content)
        self.render_images(&mut encoder, &view);

        // Render UI elements (rects and text)
        self.render_ui(&mut encoder, &view);

        // Submit and present
        self.gpu_ctx.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    fn prepare_textures(&mut self) {
        for cmd in &self.commands {
            if let DrawCommand::DrawImage { handle, .. } = cmd {
                let key = handle.data().as_ptr() as usize;
                if !self.texture_cache.contains_key(&key) {
                    match Texture::from_rgba8(&self.gpu_ctx, handle.data(), handle.width(), handle.height()) {
                        Ok(texture) => {
                            let bind_group = self.texture_pipeline.create_texture_bind_group(&self.gpu_ctx, &texture);
                            self.texture_cache.insert(key, (texture, bind_group));
                        }
                        Err(e) => log::error!("Failed to create texture: {:?}", e),
                    }
                }
            }
        }
    }

    fn render_images(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
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
                    let cache_key = handle.data().as_ptr() as usize;
                    if let Some((_texture, bind_group)) = self.texture_cache.get(&cache_key) {
                        // Calculate transform
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
                _ => {}
            }
        }
    }

    fn calculate_image_transform(&self, handle: &crate::ImageHandle, rect: &Rectangle, pan: (f32, f32), zoom: f32) -> TransformUniform {
        // Convert rect position to NDC (Normalized Device Coordinates)
        let ndc_x = (rect.x / self.width as f32) * 2.0 - 1.0;
        let ndc_y = 1.0 - (rect.y / self.height as f32) * 2.0;
        let ndc_w = (rect.width / self.width as f32) * 2.0;
        let ndc_h = (rect.height / self.height as f32) * 2.0;

        // Calculate aspect ratios
        let img_aspect = handle.width() as f32 / handle.height() as f32;
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

    fn render_ui(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        // Collect text items with their clip bounds
        struct TextItem {
            text: String,
            x: f32,
            y: f32,
            size: f32,
            color: Color,
            clip: Option<Rectangle>,
        }
        let mut text_items: Vec<TextItem> = Vec::new();
        let mut current_clip: Option<Rectangle> = None;

        for cmd in &self.commands {
            match cmd {
                DrawCommand::PushClip(rect) => current_clip = Some(*rect),
                DrawCommand::PopClip => current_clip = None,
                DrawCommand::DrawText { text, position, color, size } => {
                    text_items.push(TextItem {
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

        // Render rectangles first
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Rects Pass"),
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

            pass.set_pipeline(&self.color_pipeline.render_pipeline);
            pass.set_scissor_rect(0, 0, self.width, self.height);

            // Render rectangles with scissor clipping
            for cmd in &self.commands {
                match cmd {
                    DrawCommand::PushClip(rect) => {
                        if let Some((x, y, w, h)) = self.clip_to_scissor(rect) {
                            pass.set_scissor_rect(x, y, w, h);
                        }
                    }
                    DrawCommand::PopClip => {
                        pass.set_scissor_rect(0, 0, self.width, self.height);
                    }
                    DrawCommand::FillRect { rect, color } => {
                        let (vb, ib, count) = ColorPipeline::create_rect_vertices(
                            &self.gpu_ctx.device,
                            rect.x, rect.y, rect.width, rect.height,
                            [color.r, color.g, color.b, color.a],
                            self.width as f32, self.height as f32,
                        );
                        pass.set_vertex_buffer(0, vb.slice(..));
                        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
                        pass.draw_indexed(0..count, 0, 0..1);
                    }
                    DrawCommand::StrokeRect { rect, color, width } => {
                        let (vb, ib, count) = ColorPipeline::create_stroke_rect_vertices(
                            &self.gpu_ctx.device,
                            rect.x, rect.y, rect.width, rect.height,
                            [color.r, color.g, color.b, color.a],
                            *width,
                            self.width as f32, self.height as f32,
                        );
                        pass.set_vertex_buffer(0, vb.slice(..));
                        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
                        pass.draw_indexed(0..count, 0, 0..1);
                    }
                    _ => {}
                }
            }
        }

        // Render text using glyphon with TextBounds for clipping
        if !text_items.is_empty() {
            // Update viewport
            self.viewport.update(
                &self.gpu_ctx.queue,
                Resolution {
                    width: self.width,
                    height: self.height,
                },
            );

            // Create text areas for each text item
            let mut text_areas: Vec<TextArea> = Vec::new();
            let mut buffers: Vec<Buffer> = Vec::new();

            for item in &text_items {
                // Create a buffer for this text
                let mut buffer = Buffer::new(
                    &mut self.font_system,
                    Metrics::new(item.size, item.size * 1.2),
                );
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
            for (i, item) in text_items.iter().enumerate() {
                let bounds = if let Some(clip) = item.clip {
                    // Use clip bounds for text clipping - this is the key feature of glyphon!
                    TextBounds {
                        left: clip.x as i32,
                        top: clip.y as i32,
                        right: (clip.x + clip.width) as i32,
                        bottom: (clip.y + clip.height) as i32,
                    }
                } else {
                    // No clip - use full viewport
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

            // Prepare text for rendering
            if let Err(e) = self.text_renderer.prepare(
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
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("UI Text Pass"),
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

                if let Err(e) = self.text_renderer.render(&self.text_atlas, &self.viewport, &mut pass) {
                    log::error!("Failed to render text: {:?}", e);
                }
            }

            // Trim atlas to free unused space
            self.text_atlas.trim();
        }
    }
}

/// Check if two rectangles overlap.
fn rects_overlap(a: &Rectangle, b: &Rectangle) -> bool {
    a.x < b.x + b.width &&
    a.x + a.width > b.x &&
    a.y < b.y + b.height &&
    a.y + a.height > b.y
}
