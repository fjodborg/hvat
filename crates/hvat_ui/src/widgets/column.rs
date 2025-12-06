//! Column layout widget that arranges children vertically.

use std::marker::PhantomData;
use crate::{Bounded, ConcreteSize, ConcreteSizeXY, Element, Event, Layout, Limits, Renderer, Unbounded, Widget};
use super::flex::{FlexDirection, FlexLayout};

/// A column layout that arranges children vertically.
///
/// This is a convenience wrapper around `FlexLayout` with vertical direction.
/// It supports fill-behavior where children with 0 height share remaining space.
///
/// # Context Type Parameter
///
/// The `Context` type parameter controls whether Fill children are allowed:
/// - `Bounded` (default): Fill children allowed, use in normal layouts
/// - `Unbounded`: Fill children NOT allowed, use inside scrollables
pub struct Column<'a, Message, Context = Bounded> {
    inner: FlexLayout<'a, Message, Context>,
    _context: PhantomData<Context>,
}

// ============================================================================
// Common methods for all contexts
// ============================================================================

impl<'a, Message, Context> Column<'a, Message, Context> {
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

// ============================================================================
// Bounded-only methods
// ============================================================================

impl<'a, Message> Column<'a, Message, Bounded> {
    /// Create a new column in bounded context.
    pub fn new() -> Self {
        Self {
            inner: FlexLayout::new(FlexDirection::Vertical),
            _context: PhantomData,
        }
    }

    /// Create a column with children in bounded context.
    pub fn with_children(children: Vec<Element<'a, Message>>) -> Self {
        Self {
            inner: FlexLayout::with_children(FlexDirection::Vertical, children),
            _context: PhantomData,
        }
    }

    /// Convert to unbounded context for use inside scrollables.
    pub fn into_unbounded(self) -> Column<'a, Message, Unbounded> {
        Column {
            inner: self.inner.into_unbounded(),
            _context: PhantomData,
        }
    }
}

impl<'a, Message> Default for Column<'a, Message, Bounded> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Unbounded-only methods
// ============================================================================

impl<'a, Message> Column<'a, Message, Unbounded> {
    /// Create a new column in unbounded context.
    pub fn new_unbounded() -> Self {
        Self {
            inner: FlexLayout::column_unbounded(),
            _context: PhantomData,
        }
    }
}

// ============================================================================
// Widget implementation
// ============================================================================

impl<'a, Message, Context> Widget<Message> for Column<'a, Message, Context> {
    fn layout(&self, limits: &Limits) -> Layout {
        self.inner.layout(limits)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        self.inner.draw(renderer, layout)
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        self.inner.on_event(event, layout)
    }

    fn natural_size(&self, max_width: ConcreteSize) -> ConcreteSizeXY {
        self.inner.natural_size(max_width)
    }

    fn minimum_size(&self) -> ConcreteSizeXY {
        self.inner.minimum_size()
    }
}

/// Helper function to create a column in bounded context.
pub fn column<'a, Message>() -> Column<'a, Message, Bounded> {
    Column::new()
}

/// Helper function to create a column in unbounded context (for use inside scrollables).
pub fn column_unbounded<'a, Message>() -> Column<'a, Message, Unbounded> {
    Column::new_unbounded()
}
