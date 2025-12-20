//! Widget implementations

mod button;
mod collapsible;
mod color_picker;
mod color_swatch;
mod column;
pub mod config;
mod container_helpers;
mod dropdown;
mod flex_layout;
mod image_viewer;
mod number_input;
mod row;
pub mod scrollbar;
mod scrollable;
mod slider;
mod text;
pub mod text_core;
mod text_input;

pub use button::Button;
pub use collapsible::{Collapsible, CollapsibleConfig};
pub use color_picker::ColorPicker;
pub use color_swatch::ColorSwatch;
pub use column::Column;
pub use config::BaseInputConfig;
pub use dropdown::{Dropdown, DropdownConfig};
pub use image_viewer::{AnnotationOverlay, ImagePointerEvent, ImageViewer, OverlayShape, PointerEventKind};
pub use number_input::{NumberInput, NumberInputConfig};
pub use row::Row;
pub use scrollable::{Scrollable, ScrollDirection, ScrollbarVisibility, ScrollbarConfig};
pub use slider::{Slider, SliderConfig};
pub use text::Text;
pub use text_input::{TextInput, TextInputConfig};

use crate::element::Element;
use crate::renderer::TextureId;
use crate::Context;

/// Create a column of widgets using a builder function
pub fn col<M: 'static>(builder: impl FnOnce(&mut Context<M>)) -> Element<M> {
    let mut ctx = Context::new();
    builder(&mut ctx);
    Element::new(Column::new(ctx.take()))
}

/// Alias for col
pub fn column<M: 'static>(builder: impl FnOnce(&mut Context<M>)) -> Element<M> {
    col(builder)
}

/// Create a row of widgets using a builder function
pub fn row<M: 'static>(builder: impl FnOnce(&mut Context<M>)) -> Element<M> {
    let mut ctx = Context::new();
    builder(&mut ctx);
    Element::new(Row::new(ctx.take()))
}

/// Create a text widget
pub fn text<M: 'static>(content: impl Into<String>) -> Element<M> {
    Element::new(Text::new(content))
}

/// Create a button widget
pub fn button<M: 'static>(label: impl Into<String>) -> Button<M> {
    Button::new(label)
}

/// Create an image viewer widget with a texture
pub fn image_viewer<M: 'static>(texture_id: TextureId, width: u32, height: u32) -> ImageViewer<M> {
    ImageViewer::new(texture_id, width, height)
}

/// Create a scrollable container with content built by a builder function
///
/// The scrollable takes immutable state reference (clones it internally) and
/// emits state changes via on_scroll callback.
pub fn scrollable<M: 'static>(builder: impl FnOnce(&mut Context<M>)) -> Scrollable<M> {
    let mut ctx = Context::new();
    builder(&mut ctx);
    let content = Element::new(Column::new(ctx.take()));
    Scrollable::new(content)
}

/// Create a dropdown widget
///
/// The dropdown takes options and emits selection via on_select callback.
pub fn dropdown<M: 'static>() -> Dropdown<M> {
    Dropdown::new()
}

/// Create a collapsible section widget
///
/// The collapsible takes a header text and content built via closure.
pub fn collapsible<M: 'static>(header: impl Into<String>) -> Collapsible<M> {
    Collapsible::new(header)
}

/// Create a slider widget with a range
///
/// The slider emits state changes via on_change callback.
/// Use `.show_input(true)` to add an editable value field.
pub fn slider<M: 'static>(min: f32, max: f32) -> Slider<M> {
    Slider::new(min, max)
}

/// Create a text input widget
///
/// The text input emits changes via on_change callback.
pub fn text_input<M: 'static>() -> TextInput<M> {
    TextInput::new()
}

/// Create a number input widget
///
/// The number input emits value changes via on_change callback.
/// Supports increment/decrement buttons and keyboard/scroll input.
pub fn number_input<M: 'static>() -> NumberInput<M> {
    NumberInput::new()
}
