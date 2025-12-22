//! Widget trait and related types

use crate::event::Event;
use crate::layout::{Bounds, Size};
use crate::renderer::Renderer;

/// Result of handling an event in a widget.
///
/// This enum allows widgets to signal whether they need a redraw
/// without necessarily producing an application message.
#[derive(Debug, Clone)]
pub enum EventResult<M> {
    /// Event had no effect - no redraw needed
    None,
    /// Widget changed visually but no message to emit
    /// (e.g., hover state changed, scrollbar thumb moved)
    Redraw,
    /// Widget produced a message (implies redraw)
    Message(M),
}

impl<M> EventResult<M> {
    /// Returns true if the widget needs to be redrawn
    #[inline]
    pub fn needs_redraw(&self) -> bool {
        !matches!(self, EventResult::None)
    }

    /// Extract the message if present
    #[inline]
    pub fn message(self) -> Option<M> {
        match self {
            EventResult::Message(m) => Some(m),
            _ => None,
        }
    }

    /// Convert to Option<M>, discarding redraw-only results
    #[inline]
    pub fn into_option(self) -> Option<M> {
        self.message()
    }
}

impl<M> Default for EventResult<M> {
    fn default() -> Self {
        EventResult::None
    }
}

/// Allows easy conversion from Option<M> for migration
impl<M> From<Option<M>> for EventResult<M> {
    fn from(opt: Option<M>) -> Self {
        match opt {
            Some(m) => EventResult::Message(m),
            None => EventResult::None,
        }
    }
}

/// The core widget trait that all UI elements implement
pub trait Widget<M> {
    /// Calculate the size this widget wants given available space
    fn layout(&mut self, available: Size) -> Size;

    /// Draw the widget to the renderer
    fn draw(&self, renderer: &mut Renderer, bounds: Bounds);

    /// Handle an event, returning the result
    ///
    /// Returns:
    /// - `EventResult::None` - event had no effect
    /// - `EventResult::Redraw` - widget changed visually (e.g., hover state)
    /// - `EventResult::Message(m)` - widget produced a message (implies redraw)
    fn on_event(&mut self, event: &Event, bounds: Bounds) -> EventResult<M> {
        let _ = (event, bounds);
        EventResult::None
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

    /// Whether this widget is currently being dragged.
    /// When true, the widget will receive MouseMove events even outside its layout bounds.
    fn has_active_drag(&self) -> bool {
        false
    }
}
