//! Column layout widget - vertical arrangement of children
//!
//! This is a thin wrapper around FlexLayout for API compatibility.

use crate::element::Element;
use crate::event::Event;
use crate::layout::{Alignment, Bounds, Length, Padding, Size};
use crate::renderer::Renderer;
use crate::widget::{EventResult, Widget};

use super::flex_layout::{FlexDirection, FlexLayout};

/// A vertical column layout widget
pub struct Column<M> {
    inner: FlexLayout<M>,
}

impl<M> Column<M> {
    /// Create a new column with the given children
    pub fn new(children: Vec<Element<M>>) -> Self {
        Self {
            inner: FlexLayout::new(FlexDirection::Vertical, children),
        }
    }

    /// Set spacing between children
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.inner = self.inner.spacing(spacing);
        self
    }

    /// Set padding around the column
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.inner = self.inner.padding(padding);
        self
    }

    /// Set the width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.inner = self.inner.width(width);
        self
    }

    /// Set the height
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.inner = self.inner.height(height);
        self
    }

    /// Set horizontal alignment of children
    pub fn align_x(mut self, align: Alignment) -> Self {
        self.inner = self.inner.align_x(align);
        self
    }
}

impl<M: 'static> Widget<M> for Column<M> {
    fn has_active_overlay(&self) -> bool {
        self.inner.has_active_overlay()
    }

    fn has_active_drag(&self) -> bool {
        self.inner.has_active_drag()
    }

    fn capture_bounds(&self, layout_bounds: Bounds) -> Option<Bounds> {
        self.inner.capture_bounds(layout_bounds)
    }

    fn layout(&mut self, available: Size) -> Size {
        self.inner.layout(available)
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        self.inner.draw(renderer, bounds)
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> EventResult<M> {
        self.inner.on_event(event, bounds)
    }
}
