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

    /// Get the bounds for event capture, which may be larger than layout bounds
    /// when overlays are active (e.g., dropdown popups).
    /// Returns None to use the standard layout bounds.
    fn capture_bounds(&self, _layout_bounds: Bounds) -> Option<Bounds> {
        None
    }

    /// Whether this widget has an active overlay that should capture events globally.
    /// When true, the widget will receive events even outside its layout bounds.
    fn has_active_overlay(&self) -> bool {
        false
    }
}
