//! Renderer for drawing UI elements using hvat_gpu

use crate::layout::Bounds;
use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphonColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use hvat_gpu::{ColorPipeline, ColorVertex, GpuContext, Pipeline, Texture, TexturePipeline, TransformUniform};
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

/// Text draw request stored for batch rendering
struct TextRequest {
    text: String,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    clip: Option<Bounds>,
}

/// Texture draw request stored for batch rendering
struct TextureRequest {
    texture_id: TextureId,
    bounds: Bounds,
    transform: TransformUniform,
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
    /// Text atlas for GPU glyph cache
    text_atlas: TextAtlas,
    /// Glyphon text renderer
    text_renderer: TextRenderer,
    /// Glyphon viewport
    viewport: Viewport,
    /// Queued text requests
    text_requests: Vec<TextRequest>,
    /// Queued texture requests
    texture_requests: Vec<TextureRequest>,
    /// Registered texture bind groups
    texture_bind_groups: HashMap<TextureId, wgpu::BindGroup>,
    /// Next texture ID
    next_texture_id: usize,
    /// Current clip stack
    clip_stack: Vec<Bounds>,
}

impl Renderer {
    /// Create a new renderer
    pub fn new(gpu_ctx: &GpuContext) -> Self {
        let color_pipeline = ColorPipeline::new(&gpu_ctx.device, gpu_ctx.surface_config.format);
        let texture_pipeline = TexturePipeline::new(gpu_ctx);

        // Initialize glyphon text rendering
        let font_system = FontSystem::new();
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
        let viewport = Viewport::new(&gpu_ctx.device, &glyphon_cache);

        Self {
            color_pipeline,
            texture_pipeline,
            window_size: (gpu_ctx.width(), gpu_ctx.height()),
            color_vertices: Vec::new(),
            color_indices: Vec::new(),
            overlay_vertices: Vec::new(),
            overlay_indices: Vec::new(),
            drawing_overlay: false,
            font_system,
            swash_cache,
            glyphon_cache,
            text_atlas,
            text_renderer,
            viewport,
            text_requests: Vec::new(),
            texture_requests: Vec::new(),
            texture_bind_groups: HashMap::new(),
            next_texture_id: 0,
            clip_stack: Vec::new(),
        }
    }

    /// Update window size
    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_size = (width, height);
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
        self.texture_requests.clear();
    }

    /// Start drawing to the overlay layer (rendered after textures)
    pub fn begin_overlay(&mut self) {
        self.drawing_overlay = true;
    }

    /// Stop drawing to the overlay layer
    pub fn end_overlay(&mut self) {
        self.drawing_overlay = false;
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

    /// Stroke a rectangle outline
    pub fn stroke_rect(&mut self, bounds: Bounds, color: Color, thickness: f32) {
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

    /// Draw a line
    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, thickness: f32) {
        let (w, h) = self.window_size;
        let (vertices, indices) = self.get_current_buffers();
        ColorPipeline::append_line(
            vertices,
            indices,
            x1,
            y1,
            x2,
            y2,
            color.to_array(),
            thickness,
            w as f32,
            h as f32,
        );
    }

    /// Draw a filled circle
    pub fn fill_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
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

    /// Queue text for rendering
    pub fn text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        let clip = self.clip_stack.last().cloned();
        self.text_requests.push(TextRequest {
            text: text.to_string(),
            x,
            y,
            size,
            color,
            clip,
        });
    }

    /// Push a clip rectangle
    pub fn push_clip(&mut self, bounds: Bounds) {
        // If there's already a clip, intersect with it
        let clip = if let Some(current) = self.clip_stack.last() {
            current.intersect(&bounds)
        } else {
            Some(bounds)
        };

        if let Some(c) = clip {
            self.clip_stack.push(c);
        } else {
            // Empty intersection - push a zero-size clip
            self.clip_stack.push(Bounds::new(bounds.x, bounds.y, 0.0, 0.0));
        }
    }

    /// Pop the current clip rectangle
    pub fn pop_clip(&mut self) {
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
        let clip = self.clip_stack.last().cloned();
        self.texture_requests.push(TextureRequest {
            texture_id,
            bounds,
            transform,
            clip,
        });
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
            let vertex_buffer =
                gpu_ctx
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("UI Color Vertices"),
                        contents: bytemuck::cast_slice(&self.color_vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
            let index_buffer =
                gpu_ctx
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("UI Color Indices"),
                        contents: bytemuck::cast_slice(&self.color_indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });

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

        // Render textures
        if !self.texture_requests.is_empty() {
            self.render_textures(gpu_ctx, encoder, view);
        }

        // Render overlay (on top of textures)
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

        // Render text
        if !self.text_requests.is_empty() {
            self.render_text(gpu_ctx, encoder, view);
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

            // Update transform uniform
            self.texture_pipeline.update_transform(gpu_ctx, request.transform);

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

    /// Render all queued text
    fn render_text(
        &mut self,
        gpu_ctx: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let (width, height) = self.window_size;

        // Update viewport
        self.viewport.update(&gpu_ctx.queue, Resolution { width, height });

        // Create text buffers for each request
        let mut buffers: Vec<Buffer> = Vec::new();

        for request in &self.text_requests {
            let mut buffer =
                Buffer::new(&mut self.font_system, Metrics::new(request.size, request.size * 1.2));

            buffer.set_size(&mut self.font_system, Some(width as f32), Some(height as f32));

            buffer.set_text(
                &mut self.font_system,
                &request.text,
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
            );

            buffer.shape_until_scroll(&mut self.font_system, false);

            buffers.push(buffer);
        }

        // Create text areas
        let text_areas: Vec<TextArea> = self
            .text_requests
            .iter()
            .enumerate()
            .map(|(i, request)| {
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

                TextArea {
                    buffer: &buffers[i],
                    left: request.x,
                    top: request.y,
                    scale: 1.0,
                    bounds,
                    default_color: request.color.to_glyphon(),
                    custom_glyphs: &[],
                }
            })
            .collect();

        // Prepare text renderer
        if let Err(e) = self.text_renderer.prepare(
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &mut self.font_system,
            &mut self.text_atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        ) {
            log::error!("Failed to prepare text: {:?}", e);
            return;
        }

        // Render text
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Text Render Pass"),
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

            if let Err(e) = self.text_renderer.render(&self.text_atlas, &self.viewport, &mut render_pass) {
                log::error!("Failed to render text: {:?}", e);
            }
        }
    }
}
