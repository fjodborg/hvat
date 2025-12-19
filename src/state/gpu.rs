//! GPU rendering state for hyperspectral images.

use hvat_gpu::{
    BandSelectionUniform, GpuContext, GpuError, HyperspectralGpuData, HyperspectralPipeline,
    ImageAdjustments, Texture,
};

use crate::data::HyperspectralData;

/// GPU resources for hyperspectral rendering.
///
/// These are always created together and used together, so they're grouped
/// into a single struct to avoid multiple `Option` fields.
pub struct GpuRenderState {
    /// The hyperspectral rendering pipeline
    pub pipeline: HyperspectralPipeline,
    /// Band data uploaded to GPU texture array
    pub band_data: HyperspectralGpuData,
    /// Render target texture for compositing
    pub render_target: Texture,
}

impl GpuRenderState {
    /// Create GPU render state from hyperspectral data.
    pub fn new(
        gpu_ctx: &GpuContext,
        hyper: &HyperspectralData,
        band_selection: (usize, usize, usize),
        adjustments: ImageAdjustments,
    ) -> Result<Self, GpuError> {
        let pipeline = HyperspectralPipeline::new(gpu_ctx);
        log::info!("Created HyperspectralPipeline");

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
            pipeline,
            band_data,
            render_target,
        })
    }

    /// Render to the render target texture.
    pub fn render(
        &self,
        gpu_ctx: &GpuContext,
        band_selection: BandSelectionUniform,
        adjustments: ImageAdjustments,
    ) {
        // Update uniforms
        self.pipeline.update_band_selection(gpu_ctx, band_selection);
        self.pipeline.update_adjustments(gpu_ctx, adjustments);

        // Create command encoder and render
        let mut encoder = gpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Hyperspectral Render Encoder"),
            });

        self.pipeline
            .render(&mut encoder, &self.render_target.view, &self.band_data.bind_group);

        gpu_ctx.queue.submit(std::iter::once(encoder.finish()));
    }
}
