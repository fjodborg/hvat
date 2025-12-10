//! Widget implementations

mod button;
mod column;
mod container_helpers;
mod dropdown;
mod image_viewer;
mod row;
mod scrollable;
mod text;

pub use button::Button;
pub use column::Column;
pub use dropdown::{Dropdown, DropdownConfig};
pub use image_viewer::ImageViewer;
pub use row::Row;
pub use scrollable::{Scrollable, ScrollDirection, ScrollbarVisibility, ScrollbarConfig};
pub use text::Text;

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
