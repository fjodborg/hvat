//! Texture rendering pipeline.

use wgpu::util::DeviceExt;

use super::{BindGroupLayoutBuilder, Pipeline, PipelineBuilder};
use crate::context::GpuContext;
use crate::texture::Texture;
use crate::uniform::{ImageAdjustments, TransformUniform};
use crate::vertex::Vertex;

/// Texture rendering pipeline
pub struct TexturePipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub adjustments_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl TexturePipeline {
    pub fn new(ctx: &GpuContext) -> Self {
        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Texture Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/texture.wgsl").into()),
        });

        // Create uniform buffers
        let transform_uniform = TransformUniform::new();
        let uniform_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transform Uniform Buffer"),
            contents: bytemuck::cast_slice(&[transform_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let adjustments = ImageAdjustments::new();
        let adjustments_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Image Adjustments Buffer"),
            contents: bytemuck::cast_slice(&[adjustments]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layouts using builder
        let uniform_bind_group_layout = BindGroupLayoutBuilder::new(&ctx.device)
            .with_label("Uniform Bind Group Layout")
            .add_uniform_buffer(0, wgpu::ShaderStages::VERTEX)
            .add_uniform_buffer(1, wgpu::ShaderStages::FRAGMENT)
            .build();

        let texture_bind_group_layout = BindGroupLayoutBuilder::new(&ctx.device)
            .with_label("Texture Bind Group Layout")
            .add_texture_2d(0, wgpu::ShaderStages::FRAGMENT)
            .add_sampler(1, wgpu::ShaderStages::FRAGMENT)
            .build();

        // Create uniform bind group
        let uniform_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: adjustments_buffer.as_entire_binding(),
                },
            ],
        });

        // Create render pipeline using builder
        let render_pipeline = PipelineBuilder::new(&ctx.device, ctx.surface_config.format)
            .with_label("Texture Render Pipeline")
            .with_shader(&shader, "vs_main", "fs_main")
            .with_vertex_buffer(Vertex::desc())
            .with_bind_group_layouts(&[&uniform_bind_group_layout, &texture_bind_group_layout])
            .with_blend_state(wgpu::BlendState::REPLACE)
            .with_cull_mode(Some(wgpu::Face::Back))
            .build();

        // Create fullscreen quad vertices
        let vertices = [
            Vertex { position: [-1.0, -1.0], tex_coords: [0.0, 1.0] }, // Bottom-left
            Vertex { position: [1.0, -1.0], tex_coords: [1.0, 1.0] },  // Bottom-right
            Vertex { position: [1.0, 1.0], tex_coords: [1.0, 0.0] },   // Top-right
            Vertex { position: [-1.0, 1.0], tex_coords: [0.0, 0.0] },  // Top-left
        ];

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let vertex_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            uniform_buffer,
            adjustments_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
        }
    }

    /// Create bind group for a texture
    pub fn create_texture_bind_group(&self, ctx: &GpuContext, texture: &Texture) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        })
    }

    /// Update transform uniform
    pub fn update_transform(&self, ctx: &GpuContext, transform: TransformUniform) {
        ctx.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[transform]));
    }

    /// Update image adjustments
    pub fn update_adjustments(&self, ctx: &GpuContext, adjustments: ImageAdjustments) {
        ctx.queue
            .write_buffer(&self.adjustments_buffer, 0, bytemuck::cast_slice(&[adjustments]));
    }

    /// Render textured quad
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        texture_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Texture Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.1,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}

impl Pipeline for TexturePipeline {
    fn render_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.render_pipeline
    }
}
