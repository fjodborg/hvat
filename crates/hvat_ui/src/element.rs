//! Type-erased widget wrapper

use crate::event::Event;
use crate::layout::{Bounds, Size};
use crate::renderer::Renderer;
use crate::widget::{EventResult, Widget};

/// A type-erased widget that can hold any widget type
pub struct Element<M> {
    widget: Box<dyn Widget<M>>,
    /// Cached layout size from last layout pass
    cached_size: Size,
}

impl<M> Element<M> {
    /// Create a new element from a widget
    pub fn new<W: Widget<M> + 'static>(widget: W) -> Self {
        Self {
            widget: Box::new(widget),
            cached_size: Size::ZERO,
        }
    }

    /// Calculate layout and cache the result
    pub fn layout(&mut self, available: Size) -> Size {
        self.cached_size = self.widget.layout(available);
        self.cached_size
    }

    /// Get the cached size from last layout
    pub fn cached_size(&self) -> Size {
        self.cached_size
    }

    /// Draw the widget
    pub fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        self.widget.draw(renderer, bounds);
    }

    /// Handle an event
    pub fn on_event(&mut self, event: &Event, bounds: Bounds) -> EventResult<M> {
        self.widget.on_event(event, bounds)
    }

    /// Check if this widget has an active overlay
    pub fn has_active_overlay(&self) -> bool {
        self.widget.has_active_overlay()
    }

    /// Check if this widget is currently being dragged
    pub fn has_active_drag(&self) -> bool {
        self.widget.has_active_drag()
    }

    /// Get the capture bounds for this widget, which may be larger than layout bounds
    /// when overlays are active (e.g., dropdown popups)
    pub fn capture_bounds(&self, layout_bounds: Bounds) -> Option<Bounds> {
        self.widget.capture_bounds(layout_bounds)
    }
}

impl<M> std::fmt::Debug for Element<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("cached_size", &self.cached_size)
            .finish_non_exhaustive()
    }
}
