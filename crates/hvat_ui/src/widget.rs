//! Widget trait and related types

use crate::event::Event;
use crate::layout::{Bounds, Size};
use crate::renderer::Renderer;

/// The core widget trait that all UI elements implement
pub trait Widget<M> {
    /// Calculate the size this widget wants given available space
    fn layout(&mut self, available: Size) -> Size;

    /// Draw the widget to the renderer
    fn draw(&self, renderer: &mut Renderer, bounds: Bounds);

    /// Handle an event, optionally producing a message
    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        let _ = (event, bounds);
        None
    }

    /// Get children for hit testing (default: no children)
    fn children(&self) -> &[crate::element::Element<M>] {
        &[]
    }

    /// Get mutable children
    fn children_mut(&mut self) -> &mut [crate::element::Element<M>] {
        &mut []
    }
}
