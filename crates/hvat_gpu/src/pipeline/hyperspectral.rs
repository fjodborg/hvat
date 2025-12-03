//! Hyperspectral band compositing pipeline.
//!
//! This pipeline renders hyperspectral images by compositing selected bands
//! into an RGB output. Band data is stored in GPU textures (4 bands per RGBA texture)
//! and the compositing is done entirely on the GPU via fragment shader.

use wgpu::util::DeviceExt;

use super::{BindGroupLayoutBuilder, Pipeline, PipelineBuilder};
use crate::bindings::hyperspectral as bindings;
use crate::config::TextureConfig;
use crate::context::GpuContext;
use crate::uniform::{BandSelectionUniform, ImageAdjustments, TransformUniform};
use crate::vertex::Vertex;

/// Maximum number of bands supported (2 textures × 4 channels = 8 bands).
pub const MAX_BANDS: usize = 8;

/// GPU texture holding packed band data (4 bands per RGBA texture).
pub struct BandTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl BandTexture {
    /// Create a band texture from f32 band data.
    ///
    /// Takes up to 4 bands and packs them into RGBA channels.
    /// Missing bands are filled with zeros.
    pub fn from_bands(
        ctx: &GpuContext,
        bands: &[&[f32]],
        width: u32,
        height: u32,
        label: &str,
    ) -> Self {
        let pixel_count = (width * height) as usize;

        // Pack bands into RGBA format (f32 values normalized to 0-255 u8)
        let mut rgba_data = vec![0u8; pixel_count * 4];

        for (channel_idx, band) in bands.iter().enumerate().take(4) {
            // Skip bands with incorrect size
            if band.len() != pixel_count {
                continue;
            }
            for (pixel_idx, &value) in band.iter().enumerate() {
                let byte_value = (value.clamp(0.0, 1.0) * 255.0) as u8;
                rgba_data[pixel_idx * 4 + channel_idx] = byte_value;
            }
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, // Linear, not sRGB for band data
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        ctx.queue.write_texture(
            texture.as_image_copy(),
            &rgba_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            width,
            height,
        }
    }

    /// Create an empty (black) band texture.
    pub fn empty(ctx: &GpuContext, width: u32, height: u32, label: &str) -> Self {
        Self::from_bands(ctx, &[], width, height, label)
    }
}

/// Hyperspectral image data stored on GPU.
pub struct HyperspectralGpuData {
    /// Band texture 0 (bands 0-3)
    pub band_texture_0: BandTexture,
    /// Band texture 1 (bands 4-7)
    pub band_texture_1: BandTexture,
    /// Sampler for band textures
    pub sampler: wgpu::Sampler,
    /// Bind group for band textures
    pub bind_group: wgpu::BindGroup,
    /// Image width
    pub width: u32,
    /// Image height
    pub height: u32,
    /// Number of bands
    pub num_bands: usize,
}

impl HyperspectralGpuData {
    /// Upload hyperspectral band data to GPU.
    ///
    /// # Arguments
    /// * `ctx` - GPU context
    /// * `bands` - Vector of band data, each band is a Vec<f32> with width*height values (0.0-1.0)
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `bind_group_layout` - Layout from HyperspectralPipeline
    pub fn from_bands(
        ctx: &GpuContext,
        bands: &[Vec<f32>],
        width: u32,
        height: u32,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let num_bands = bands.len().min(MAX_BANDS);

        // Split bands into two groups of 4
        let bands_0: Vec<&[f32]> = bands.iter().take(4).map(|b| b.as_slice()).collect();
        let bands_1: Vec<&[f32]> = bands.iter().skip(4).take(4).map(|b| b.as_slice()).collect();

        let band_texture_0 = if !bands_0.is_empty() {
            BandTexture::from_bands(ctx, &bands_0, width, height, "Band Texture 0 (bands 0-3)")
        } else {
            BandTexture::empty(ctx, width, height, "Band Texture 0 (empty)")
        };

        let band_texture_1 = if !bands_1.is_empty() {
            BandTexture::from_bands(ctx, &bands_1, width, height, "Band Texture 1 (bands 4-7)")
        } else {
            BandTexture::empty(ctx, width, height, "Band Texture 1 (empty)")
        };

        // Create sampler
        let config = TextureConfig::default();
        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Band Sampler"),
            address_mode_u: config.address_mode_u,
            address_mode_v: config.address_mode_v,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: config.mag_filter,
            min_filter: config.min_filter,
            mipmap_filter: config.mipmap_filter,
            ..Default::default()
        });

        // Create bind group
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Hyperspectral Band Bind Group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: bindings::BAND_TEXTURE_0_BINDING,
                    resource: wgpu::BindingResource::TextureView(&band_texture_0.view),
                },
                wgpu::BindGroupEntry {
                    binding: bindings::BAND_TEXTURE_1_BINDING,
                    resource: wgpu::BindingResource::TextureView(&band_texture_1.view),
                },
                wgpu::BindGroupEntry {
                    binding: bindings::BAND_SAMPLER_BINDING,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            band_texture_0,
            band_texture_1,
            sampler,
            bind_group,
            width,
            height,
            num_bands,
        }
    }
}

/// Hyperspectral rendering pipeline.
///
/// Composites hyperspectral bands into RGB output on the GPU.
/// Band selection changes only require updating a uniform buffer,
/// not regenerating textures.
pub struct HyperspectralPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub adjustments_buffer: wgpu::Buffer,
    pub band_selection_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub band_texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl HyperspectralPipeline {
    pub fn new(ctx: &GpuContext) -> Self {
        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Hyperspectral Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/hyperspectral.wgsl").into()),
        });

        // Create uniform buffers
        let transform_uniform = TransformUniform::new();
        let uniform_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Hyperspectral Transform Uniform Buffer"),
            contents: bytemuck::cast_slice(&[transform_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let adjustments = ImageAdjustments::new();
        let adjustments_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Hyperspectral Adjustments Buffer"),
            contents: bytemuck::cast_slice(&[adjustments]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let band_selection = BandSelectionUniform::default();
        let band_selection_buffer =
            ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Band Selection Buffer"),
                contents: bytemuck::cast_slice(&[band_selection]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layouts
        let uniform_bind_group_layout = BindGroupLayoutBuilder::new(&ctx.device)
            .with_label("Hyperspectral Uniform Bind Group Layout")
            .add_uniform_buffer(bindings::UNIFORM_TRANSFORM_BINDING, wgpu::ShaderStages::VERTEX)
            .add_uniform_buffer(
                bindings::UNIFORM_ADJUSTMENTS_BINDING,
                wgpu::ShaderStages::FRAGMENT,
            )
            .add_uniform_buffer(
                bindings::UNIFORM_BAND_SELECTION_BINDING,
                wgpu::ShaderStages::FRAGMENT,
            )
            .build();

        let band_texture_bind_group_layout = BindGroupLayoutBuilder::new(&ctx.device)
            .with_label("Band Texture Bind Group Layout")
            .add_texture_2d(bindings::BAND_TEXTURE_0_BINDING, wgpu::ShaderStages::FRAGMENT)
            .add_texture_2d(bindings::BAND_TEXTURE_1_BINDING, wgpu::ShaderStages::FRAGMENT)
            .add_sampler(bindings::BAND_SAMPLER_BINDING, wgpu::ShaderStages::FRAGMENT)
            .build();

        // Create uniform bind group
        let uniform_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Hyperspectral Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: bindings::UNIFORM_TRANSFORM_BINDING,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: bindings::UNIFORM_ADJUSTMENTS_BINDING,
                    resource: adjustments_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: bindings::UNIFORM_BAND_SELECTION_BINDING,
                    resource: band_selection_buffer.as_entire_binding(),
                },
            ],
        });

        // Create render pipeline
        let render_pipeline = PipelineBuilder::new(&ctx.device, ctx.surface_config.format)
            .with_label("Hyperspectral Render Pipeline")
            .with_shader(&shader, "vs_main", "fs_main")
            .with_vertex_buffer(Vertex::desc())
            .with_bind_group_layouts(&[&uniform_bind_group_layout, &band_texture_bind_group_layout])
            .with_blend_state(wgpu::BlendState::REPLACE)
            .with_cull_mode(Some(wgpu::Face::Back))
            .build();

        // Create fullscreen quad vertices
        let vertices = [
            Vertex {
                position: [-1.0, -1.0],
                tex_coords: [0.0, 1.0],
            }, // Bottom-left
            Vertex {
                position: [1.0, -1.0],
                tex_coords: [1.0, 1.0],
            }, // Bottom-right
            Vertex {
                position: [1.0, 1.0],
                tex_coords: [1.0, 0.0],
            }, // Top-right
            Vertex {
                position: [-1.0, 1.0],
                tex_coords: [0.0, 0.0],
            }, // Top-left
        ];

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let vertex_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Hyperspectral Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Hyperspectral Index Buffer"),
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
            band_selection_buffer,
            uniform_bind_group,
            band_texture_bind_group_layout,
        }
    }

    /// Update transform uniform.
    pub fn update_transform(&self, ctx: &GpuContext, transform: TransformUniform) {
        ctx.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[transform]));
    }

    /// Update image adjustments.
    pub fn update_adjustments(&self, ctx: &GpuContext, adjustments: ImageAdjustments) {
        ctx.queue.write_buffer(
            &self.adjustments_buffer,
            0,
            bytemuck::cast_slice(&[adjustments]),
        );
    }

    /// Update band selection (the fast path - just updates a uniform).
    pub fn update_band_selection(&self, ctx: &GpuContext, selection: BandSelectionUniform) {
        ctx.queue.write_buffer(
            &self.band_selection_buffer,
            0,
            bytemuck::cast_slice(&[selection]),
        );
    }

    /// Render hyperspectral image with current band selection.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        band_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Hyperspectral Render Pass"),
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
        render_pass.set_bind_group(bindings::UNIFORM_GROUP, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(bindings::BAND_TEXTURE_GROUP, band_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }

    /// Get the bind group layout for band textures (needed when creating HyperspectralGpuData).
    pub fn band_texture_layout(&self) -> &wgpu::BindGroupLayout {
        &self.band_texture_bind_group_layout
    }
}

impl Pipeline for HyperspectralPipeline {
    fn render_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.render_pipeline
    }
}
