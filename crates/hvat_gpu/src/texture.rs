use wgpu;

use crate::config::TextureConfig;
use crate::context::GpuContext;
use crate::error::{GpuError, Result};

/// GPU texture wrapper
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
}

impl Texture {
    /// Create a texture from RGBA8 bytes with default configuration.
    pub fn from_rgba8(ctx: &GpuContext, data: &[u8], width: u32, height: u32) -> Result<Self> {
        Self::from_rgba8_with_config(ctx, data, width, height, TextureConfig::default())
    }

    /// Create a texture from RGBA8 bytes with custom configuration.
    pub fn from_rgba8_with_config(
        ctx: &GpuContext,
        data: &[u8],
        width: u32,
        height: u32,
        config: TextureConfig,
    ) -> Result<Self> {
        // Validate data size
        let expected_size = (width * height * 4) as usize;
        if data.len() != expected_size {
            return Err(GpuError::Texture(format!(
                "Invalid data size: expected {} bytes for {}x{} RGBA8, got {}",
                expected_size,
                width,
                height,
                data.len()
            )));
        }

        // Create texture
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Image Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload data to texture
        ctx.queue.write_texture(
            texture.as_image_copy(),
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        // Create texture view
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler with config
        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Image Sampler"),
            address_mode_u: config.address_mode_u,
            address_mode_v: config.address_mode_v,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: config.mag_filter,
            min_filter: config.min_filter,
            mipmap_filter: config.mipmap_filter,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
            width,
            height,
        })
    }

    /// Get aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}
