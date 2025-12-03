//! Render pipeline abstractions.
//!
//! This module provides common traits and implementations for GPU render pipelines.

pub mod builder;
pub mod color;
pub mod texture;

pub use builder::{BindGroupLayoutBuilder, PipelineBuilder};
pub use color::ColorPipeline;
pub use texture::TexturePipeline;

/// Common trait for render pipelines.
pub trait Pipeline {
    /// Get a reference to the underlying wgpu render pipeline.
    fn render_pipeline(&self) -> &wgpu::RenderPipeline;
}
