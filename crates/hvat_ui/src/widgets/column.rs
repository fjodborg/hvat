//! Column layout widget that arranges children vertically.

use crate::{Element, Event, Layout, Limits, Renderer, Widget};
use super::flex::{FlexDirection, FlexLayout};

/// A column layout that arranges children vertically.
///
/// This is a convenience wrapper around `FlexLayout` with vertical direction.
/// It supports fill-behavior where children with 0 height share remaining space.
pub struct Column<'a, Message> {
    inner: FlexLayout<'a, Message>,
}

impl<'a, Message> Column<'a, Message> {
    /// Create a new column.
    pub fn new() -> Self {
        Self {
            inner: FlexLayout::new(FlexDirection::Vertical),
        }
    }

    /// Create a column with children.
    pub fn with_children(children: Vec<Element<'a, Message>>) -> Self {
        Self {
            inner: FlexLayout::with_children(FlexDirection::Vertical, children),
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

    /// Check if the column has no children.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<'a, Message> Default for Column<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message> Widget<Message> for Column<'a, Message> {
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

/// Helper function to create a column.
pub fn column<'a, Message>() -> Column<'a, Message> {
    Column::new()
}
