//! Renderer for drawing UI elements using hvat_gpu

use crate::layout::Bounds;
use hvat_gpu::{ColorPipeline, ColorVertex, GpuContext, Pipeline, TexturePipeline, TransformUniform};
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
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

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
}

impl Renderer {
    /// Create a new renderer
    pub fn new(gpu_ctx: &GpuContext) -> Self {
        let color_pipeline = ColorPipeline::new(&gpu_ctx.device, gpu_ctx.surface_config.format);
        let texture_pipeline = TexturePipeline::new(gpu_ctx);

        Self {
            color_pipeline,
            texture_pipeline,
            window_size: (gpu_ctx.width(), gpu_ctx.height()),
            color_vertices: Vec::new(),
            color_indices: Vec::new(),
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
    }

    /// Fill a rectangle
    pub fn fill_rect(&mut self, bounds: Bounds, color: Color) {
        let (w, h) = self.window_size;
        ColorPipeline::append_rect(
            &mut self.color_vertices,
            &mut self.color_indices,
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
        ColorPipeline::append_stroke_rect(
            &mut self.color_vertices,
            &mut self.color_indices,
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
        ColorPipeline::append_line(
            &mut self.color_vertices,
            &mut self.color_indices,
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
        ColorPipeline::append_circle(
            &mut self.color_vertices,
            &mut self.color_indices,
            cx,
            cy,
            radius,
            color.to_array(),
            w as f32,
            h as f32,
        );
    }

    /// Draw text (placeholder - stores for future text rendering)
    pub fn text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        // TODO: Implement text rendering with glyphon
        // For now, just log that we would draw text
        log::trace!("Would draw text '{}' at ({}, {}) size={}", text, x, y, size);
        let _ = color; // Silence unused warning
    }

    /// Push a clip rectangle (not yet implemented)
    pub fn push_clip(&mut self, _bounds: Bounds) {
        // TODO: Implement clipping via scissor rects
    }

    /// Pop the current clip rectangle
    pub fn pop_clip(&mut self) {
        // TODO: Implement clipping
    }

    /// Execute all draw commands
    pub fn render(&mut self, gpu_ctx: &GpuContext, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        // Render batched color primitives
        if !self.color_vertices.is_empty() {
            let vertex_buffer = gpu_ctx.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("UI Color Vertices"),
                    contents: bytemuck::cast_slice(&self.color_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            );
            let index_buffer = gpu_ctx.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("UI Color Indices"),
                    contents: bytemuck::cast_slice(&self.color_indices),
                    usage: wgpu::BufferUsages::INDEX,
                },
            );

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

        // Clear for next frame
        self.clear();
    }
}
