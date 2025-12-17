//! Row layout widget - horizontal arrangement of children
//!
//! This is a thin wrapper around FlexLayout for API compatibility.

use crate::element::Element;
use crate::event::Event;
use crate::layout::{Alignment, Bounds, Length, Padding, Size};
use crate::renderer::Renderer;
use crate::widget::Widget;

use super::flex_layout::{FlexDirection, FlexLayout};

/// A horizontal row layout widget
pub struct Row<M> {
    inner: FlexLayout<M>,
}

impl<M> Row<M> {
    /// Create a new row with the given children
    pub fn new(children: Vec<Element<M>>) -> Self {
        Self {
            inner: FlexLayout::new(FlexDirection::Horizontal, children),
        }
    }

    /// Set spacing between children
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.inner = self.inner.spacing(spacing);
        self
    }

    /// Set padding around the row
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

    /// Set vertical alignment of children
    pub fn align_y(mut self, align: Alignment) -> Self {
        self.inner = self.inner.align_y(align);
        self
    }
}

impl<M: 'static> Widget<M> for Row<M> {
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

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        self.inner.on_event(event, bounds)
    }
}
