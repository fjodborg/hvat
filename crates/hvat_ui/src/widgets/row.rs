//! Row layout widget

use crate::element::Element;
use crate::event::Event;
use crate::layout::{Alignment, Bounds, Length, Padding, Size};
use crate::renderer::Renderer;
use crate::widget::Widget;

use super::container_helpers;

/// Default spacing between children
const DEFAULT_SPACING: f32 = 8.0;

/// A horizontal row layout widget
pub struct Row<M> {
    children: Vec<Element<M>>,
    spacing: f32,
    padding: Padding,
    width: Length,
    height: Length,
    align_y: Alignment,
    /// Cached child bounds from layout
    child_bounds: Vec<Bounds>,
}

impl<M> Row<M> {
    /// Create a new row with the given children
    pub fn new(children: Vec<Element<M>>) -> Self {
        Self {
            children,
            spacing: DEFAULT_SPACING,
            padding: Padding::ZERO,
            width: Length::Shrink,
            height: Length::Shrink,
            align_y: Alignment::Start,
            child_bounds: Vec::new(),
        }
    }

    /// Set spacing between children
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set padding around the row
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Set the width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Set the height
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Set vertical alignment of children
    pub fn align_y(mut self, align: Alignment) -> Self {
        self.align_y = align;
        self
    }
}

impl<M: 'static> Widget<M> for Row<M> {
    fn has_active_overlay(&self) -> bool {
        self.children.iter().any(|c| c.has_active_overlay())
    }

    fn has_active_drag(&self) -> bool {
        self.children.iter().any(|c| c.has_active_drag())
    }

    fn layout(&mut self, available: Size) -> Size {
        log::debug!("Row layout: available={:?}", available);

        let inner_available = Size::new(
            available.width - self.padding.horizontal(),
            available.height - self.padding.vertical(),
        );

        // First pass: layout non-fill children to get their sizes
        let mut total_fixed_width = 0.0;
        let mut max_height: f32 = 0.0;

        for (i, child) in self.children.iter_mut().enumerate() {
            // Layout with full height, shrink width for now
            let child_size = child.layout(Size::new(inner_available.width, inner_available.height));
            log::debug!("  Row child {} layout: {:?}", i, child_size);
            max_height = max_height.max(child_size.height);
            total_fixed_width += child_size.width;
        }

        // Add spacing
        if !self.children.is_empty() {
            total_fixed_width += self.spacing * (self.children.len() - 1) as f32;
        }

        // Calculate fill space
        let remaining_width = (inner_available.width - total_fixed_width).max(0.0);

        // Second pass: calculate actual positions
        self.child_bounds.clear();
        let mut x = self.padding.left;

        for (i, child) in self.children.iter().enumerate() {
            let child_size = child.cached_size();
            let child_width = child_size.width;

            let y_offset = self.align_y.align(max_height, child_size.height);

            let child_bounds = Bounds::new(
                x,
                self.padding.top + y_offset,
                child_width,
                child_size.height,
            );
            log::debug!("  Row child {} bounds: {:?}", i, child_bounds);
            self.child_bounds.push(child_bounds);

            x += child_width + self.spacing;
        }

        // Calculate total size
        let content_width = if self.children.is_empty() {
            0.0
        } else {
            x - self.spacing + self.padding.right - self.padding.left
        };
        let content_height = max_height + self.padding.vertical();

        Size::new(
            self.width.resolve(available.width, content_width + self.padding.horizontal()),
            self.height.resolve(available.height, content_height),
        )
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        log::debug!("Row draw: bounds={:?}, {} children", bounds, self.children.len());
        container_helpers::draw_children(&self.children, &self.child_bounds, renderer, bounds);
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        container_helpers::dispatch_event_to_children(
            &mut self.children,
            &self.child_bounds,
            event,
            bounds,
        )
    }
}
