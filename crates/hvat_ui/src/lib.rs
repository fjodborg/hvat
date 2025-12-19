//! hvat_ui - A simple, ergonomic UI framework built on wgpu
//!
//! This crate provides a callback-based widget system with a clean builder API.

mod application;
mod callback;
pub mod constants;
mod context;
mod element;
mod event;
mod layout;
mod overlay;
mod renderer;
mod state;
pub mod theme;
pub mod units;
mod widget;
mod widgets;

/// Reusable demo components
pub mod demos;

pub use application::{Application, Resources, Settings};
pub use callback::{Callback, Callback0, SideEffect};
pub use context::Context;
pub use element::Element;
pub use event::{Event, KeyCode, KeyModifiers, MouseButton};
pub use layout::{Bounds, Length, Padding, Size};
pub use renderer::{Color, Renderer, TextureId};
pub use state::*;
pub use theme::Theme;
pub use units::{FontSize, Spacing, ZoomLevel};
pub use widget::Widget;

// Re-export widgets
pub use widgets::{
    button, col, collapsible, column, dropdown, image_viewer, number_input, row, scrollable,
    slider, text, text_input, AnnotationOverlay, BaseInputConfig, Collapsible, CollapsibleConfig,
    Column, Dropdown, DropdownConfig, ImagePointerEvent, NumberInput, NumberInputConfig,
    OverlayShape, PointerEventKind, Row, Scrollable, ScrollDirection, ScrollbarConfig,
    ScrollbarVisibility, Slider, SliderConfig, Text, TextInput, TextInputConfig,
};

// Re-export hvat_gpu types that users need
pub use hvat_gpu::{ClearColor, GpuContext, ImageAdjustments, Texture, TransformUniform};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::application::{Application, Resources, Settings};
    pub use crate::callback::{Callback, Callback0, SideEffect};
    pub use crate::context::Context;
    pub use crate::element::Element;
    pub use crate::event::{Event, KeyCode, KeyModifiers, MouseButton};
    pub use crate::layout::{Bounds, Length, Padding, Size};
    pub use crate::renderer::TextureId;
    pub use crate::state::*;
    pub use crate::theme::Theme;
    pub use crate::units::{FontSize, Spacing, ZoomLevel};
    pub use crate::widgets::{
        button, col, collapsible, column, dropdown, image_viewer, number_input, row, scrollable,
        slider, text, text_input, ScrollDirection, ScrollbarVisibility,
    };
    pub use crate::{ClearColor, ImageAdjustments, Texture};
}

/// Run an application with default settings
pub fn run<A: Application + 'static>(app: A) -> Result<(), Box<dyn std::error::Error>> {
    application::run(app, Settings::default())
}

/// Run an application with custom settings
pub fn run_with_settings<A: Application + 'static>(
    app: A,
    settings: Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    application::run(app, settings)
}
