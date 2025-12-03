//! Row layout widget that arranges children horizontally.

use crate::{Element, Event, Layout, Limits, Renderer, Widget};
use super::flex::{FlexDirection, FlexLayout};

/// A row layout that arranges children horizontally.
///
/// This is a convenience wrapper around `FlexLayout` with horizontal direction.
pub struct Row<'a, Message> {
    inner: FlexLayout<'a, Message>,
}

impl<'a, Message> Row<'a, Message> {
    /// Create a new row.
    pub fn new() -> Self {
        Self {
            inner: FlexLayout::new(FlexDirection::Horizontal),
        }
    }

    /// Create a row with children.
    pub fn with_children(children: Vec<Element<'a, Message>>) -> Self {
        Self {
            inner: FlexLayout::with_children(FlexDirection::Horizontal, children),
        }
    }

    /// Add a child element.
    pub fn push(mut self, child: Element<'a, Message>) -> Self {
        self.inner = self.inner.push(child);
        self
    }

    /// Set the spacing between children.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.inner = self.inner.spacing(spacing);
        self
    }

    /// Get the number of children.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the row has no children.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<'a, Message> Default for Row<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message> Widget<Message> for Row<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        self.inner.layout(limits)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        self.inner.draw(renderer, layout)
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        self.inner.on_event(event, layout)
    }
}

/// Helper function to create a row.
pub fn row<'a, Message>() -> Row<'a, Message> {
    Row::new()
}
