//! GPU rendering state for hyperspectral images.
//!
//! This module separates GPU resources into:
//! - `SharedGpuPipeline`: The stateless rendering pipeline (created once, shared)
//! - `GpuRenderState`: Per-image GPU data (band textures + render target)

use hvat_gpu::{
    BandSelectionUniform, GpuContext, GpuError, HyperspectralGpuData, HyperspectralPipeline,
    ImageAdjustments, Texture,
};

use super::CachedGpuTexture;
use crate::data::HyperspectralData;

/// Shared GPU pipeline for hyperspectral rendering.
///
/// This is created once during application setup and reused for all image rendering.
/// The pipeline contains the shader, render pipeline, and uniform buffers which are
/// stateless and can be shared across multiple images.
pub struct SharedGpuPipeline {
    /// The hyperspectral rendering pipeline
    pipeline: HyperspectralPipeline,
}

impl SharedGpuPipeline {
    /// Create a new shared GPU pipeline.
    pub fn new(gpu_ctx: &GpuContext) -> Self {
        let pipeline = HyperspectralPipeline::new(gpu_ctx);
        log::info!("Created shared HyperspectralPipeline");
        Self { pipeline }
    }

    /// Get the bind group layout for band textures.
    ///
    /// Required when creating `HyperspectralGpuData` for cached textures.
    pub fn band_texture_layout(&self) -> &wgpu::BindGroupLayout {
        self.pipeline.band_texture_layout()
    }

    /// Update band selection uniform.
    pub fn update_band_selection(
        &self,
        gpu_ctx: &GpuContext,
        band_selection: BandSelectionUniform,
    ) {
        self.pipeline.update_band_selection(gpu_ctx, band_selection);
    }

    /// Update image adjustments uniform.
    pub fn update_adjustments(&self, gpu_ctx: &GpuContext, adjustments: ImageAdjustments) {
        self.pipeline.update_adjustments(gpu_ctx, adjustments);
    }

    /// Render using the given band data and render target.
    pub fn render(
        &self,
        gpu_ctx: &GpuContext,
        render_target: &Texture,
        band_data: &HyperspectralGpuData,
    ) {
        let mut encoder = gpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Hyperspectral Render Encoder"),
            });

        self.pipeline
            .render(&mut encoder, &render_target.view, &band_data.bind_group);

        gpu_ctx.queue.submit(std::iter::once(encoder.finish()));
    }
}

/// Per-image GPU render state.
///
/// Contains GPU resources specific to one image: band textures and render target.
/// The band textures can come from fresh upload or from the GPU cache.
pub struct GpuRenderState {
    /// Band data uploaded to GPU texture array
    pub band_data: HyperspectralGpuData,
    /// Render target texture for compositing
    pub render_target: Texture,
    /// Image dimensions
    pub width: u32,
    pub height: u32,
    /// Number of bands
    pub num_bands: usize,
}

impl GpuRenderState {
    /// Create GPU render state from hyperspectral data (fresh upload).
    pub fn new(
        gpu_ctx: &GpuContext,
        pipeline: &SharedGpuPipeline,
        hyper: &HyperspectralData,
        band_selection: (usize, usize, usize),
        adjustments: ImageAdjustments,
    ) -> Result<Self, GpuError> {
        let band_data = HyperspectralGpuData::from_bands(
            gpu_ctx,
            &hyper.bands,
            hyper.width,
            hyper.height,
            pipeline.band_texture_layout(),
        );
        log::info!(
            "Uploaded {} bands ({}x{}) to GPU texture array",
            hyper.bands.len(),
            hyper.width,
            hyper.height
        );

        let render_target = Texture::render_target(gpu_ctx, hyper.width, hyper.height)?;
        log::info!(
            "Created render target texture ({}x{})",
            hyper.width,
            hyper.height
        );

        // Set initial uniforms
        pipeline.update_band_selection(
            gpu_ctx,
            BandSelectionUniform {
                red_band: band_selection.0 as u32,
                green_band: band_selection.1 as u32,
                blue_band: band_selection.2 as u32,
                num_bands: hyper.bands.len() as u32,
            },
        );
        pipeline.update_adjustments(gpu_ctx, adjustments);

        Ok(Self {
            band_data,
            render_target,
            width: hyper.width,
            height: hyper.height,
            num_bands: hyper.bands.len(),
        })
    }

    /// Create GPU render state from cached band data.
    ///
    /// Takes ownership of the cached GPU data and creates a new render target.
    /// This is much faster than `new()` since band textures are already on GPU.
    pub fn from_cached(
        gpu_ctx: &GpuContext,
        pipeline: &SharedGpuPipeline,
        cached: CachedGpuTexture,
        band_selection: (usize, usize, usize),
        adjustments: ImageAdjustments,
    ) -> Result<Self, GpuError> {
        let render_target = Texture::render_target(gpu_ctx, cached.width, cached.height)?;
        log::info!(
            "Created render target from cached GPU data ({}x{}, {} bands)",
            cached.width,
            cached.height,
            cached.num_bands
        );

        // Set initial uniforms
        pipeline.update_band_selection(
            gpu_ctx,
            BandSelectionUniform {
                red_band: band_selection.0 as u32,
                green_band: band_selection.1 as u32,
                blue_band: band_selection.2 as u32,
                num_bands: cached.num_bands as u32,
            },
        );
        pipeline.update_adjustments(gpu_ctx, adjustments);

        Ok(Self {
            band_data: cached.gpu_data,
            render_target,
            width: cached.width,
            height: cached.height,
            num_bands: cached.num_bands,
        })
    }

    /// Render to the render target texture using the shared pipeline.
    pub fn render(
        &self,
        gpu_ctx: &GpuContext,
        pipeline: &SharedGpuPipeline,
        band_selection: BandSelectionUniform,
        adjustments: ImageAdjustments,
    ) {
        // Update uniforms
        pipeline.update_band_selection(gpu_ctx, band_selection);
        pipeline.update_adjustments(gpu_ctx, adjustments);

        // Render using shared pipeline
        pipeline.render(gpu_ctx, &self.render_target, &self.band_data);
    }

    /// Convert this render state back into a cached texture.
    ///
    /// Consumes self and returns the band data as a `CachedGpuTexture`.
    /// The render target is dropped, but band textures are preserved.
    pub fn into_cached(self) -> CachedGpuTexture {
        CachedGpuTexture {
            gpu_data: self.band_data,
            width: self.width,
            height: self.height,
            num_bands: self.num_bands,
        }
    }
}
