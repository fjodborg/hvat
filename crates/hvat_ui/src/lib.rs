//! hvat_ui - A GPU-accelerated immediate-mode UI framework
//!
//! This crate provides a simple, immediate-mode UI framework built on top of wgpu.
//! It follows the Elm Architecture pattern with Applications, Messages, and Views.

// Re-export hvat_gpu for convenience
pub use hvat_gpu;

// Core modules
mod application;
mod element;
mod event;
pub mod icon;
mod image;
mod layout;
mod layout_cache;
mod length;
mod overlay;
mod renderer;
mod text_metrics;
mod widget;

// Widget implementations
pub mod widgets;

// Public API
pub use application::{run, Application, Settings};
pub use element::{Element, WidgetId};
pub use event::{Event, EventResult, Key, Modifiers, MouseButton};
pub use image::{ImageHandle, HyperspectralImageHandle};
pub use layout::{
    Layout, Limits, Point, Rectangle, Size, SizingMode,
    // Type-safe sizing types
    ConcreteSize, ConcreteSizeXY, Bounded, Unbounded,
};
pub use length::Length;
pub use renderer::{Color, Renderer};
pub use hvat_gpu::{ImageAdjustments, BandSelectionUniform};
pub use widget::Widget;
pub use text_metrics::{TextMetrics, measure_text, line_height};
pub use layout_cache::{LayoutCache, LayoutKey, LayoutPath, LayoutContext, CacheStats};
pub use overlay::{Overlay, OverlayItem, OverlayShape};
