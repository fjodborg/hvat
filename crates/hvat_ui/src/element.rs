//! Type-erased widget wrapper

use crate::event::Event;
use crate::layout::{Bounds, Size};
use crate::renderer::Renderer;
use crate::widget::Widget;

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
    pub fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        self.widget.on_event(event, bounds)
    }
}

impl<M> std::fmt::Debug for Element<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("cached_size", &self.cached_size)
            .finish_non_exhaustive()
    }
}
