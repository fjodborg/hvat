//! Row layout widget that arranges children horizontally.

use std::marker::PhantomData;
use crate::{Bounded, ConcreteSize, ConcreteSizeXY, Element, Event, Layout, Limits, Renderer, Unbounded, Widget};
use super::flex::{FlexDirection, FlexLayout};

/// A row layout that arranges children horizontally.
///
/// This is a convenience wrapper around `FlexLayout` with horizontal direction.
///
/// # Context Type Parameter
///
/// The `Context` type parameter controls whether Fill children are allowed:
/// - `Bounded` (default): Fill children allowed, use in normal layouts
/// - `Unbounded`: Fill children NOT allowed, use inside scrollables
pub struct Row<'a, Message, Context = Bounded> {
    inner: FlexLayout<'a, Message, Context>,
    _context: PhantomData<Context>,
}

// ============================================================================
// Common methods for all contexts
// ============================================================================

impl<'a, Message, Context> Row<'a, Message, Context> {
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

    /// Enable wrap mode - children wrap to the next line when they exceed available width.
    pub fn wrap(mut self) -> Self {
        self.inner = self.inner.wrap();
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

// ============================================================================
// Bounded-only methods
// ============================================================================

impl<'a, Message> Row<'a, Message, Bounded> {
    /// Create a new row in bounded context.
    pub fn new() -> Self {
        Self {
            inner: FlexLayout::new(FlexDirection::Horizontal),
            _context: PhantomData,
        }
    }

    /// Create a row with children in bounded context.
    pub fn with_children(children: Vec<Element<'a, Message>>) -> Self {
        Self {
            inner: FlexLayout::with_children(FlexDirection::Horizontal, children),
            _context: PhantomData,
        }
    }

    /// Convert to unbounded context for use inside scrollables.
    pub fn into_unbounded(self) -> Row<'a, Message, Unbounded> {
        Row {
            inner: self.inner.into_unbounded(),
            _context: PhantomData,
        }
    }
}

impl<'a, Message> Default for Row<'a, Message, Bounded> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Unbounded-only methods
// ============================================================================

impl<'a, Message> Row<'a, Message, Unbounded> {
    /// Create a new row in unbounded context.
    pub fn new_unbounded() -> Self {
        Self {
            inner: FlexLayout::row_unbounded(),
            _context: PhantomData,
        }
    }
}

// ============================================================================
// Widget implementation
// ============================================================================

impl<'a, Message, Context> Widget<Message> for Row<'a, Message, Context> {
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

/// Helper function to create a row in bounded context.
pub fn row<'a, Message>() -> Row<'a, Message, Bounded> {
    Row::new()
}

/// Helper function to create a row in unbounded context (for use inside scrollables).
pub fn row_unbounded<'a, Message>() -> Row<'a, Message, Unbounded> {
    Row::new_unbounded()
}
