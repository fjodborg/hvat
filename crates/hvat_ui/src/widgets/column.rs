//! Column layout widget

use crate::constants::DEFAULT_SPACING;
use crate::element::Element;
use crate::event::Event;
use crate::layout::{Alignment, Bounds, Length, Padding, Size};
use crate::renderer::Renderer;
use crate::widget::Widget;

use super::container_helpers;

/// A vertical column layout widget
pub struct Column<M> {
    children: Vec<Element<M>>,
    spacing: f32,
    padding: Padding,
    width: Length,
    height: Length,
    align_x: Alignment,
    /// Cached child bounds from layout
    child_bounds: Vec<Bounds>,
}

impl<M> Column<M> {
    /// Create a new column with the given children
    pub fn new(children: Vec<Element<M>>) -> Self {
        Self {
            children,
            spacing: DEFAULT_SPACING,
            padding: Padding::ZERO,
            width: Length::Shrink,
            height: Length::Shrink,
            align_x: Alignment::Start,
            child_bounds: Vec::new(),
        }
    }

    /// Set spacing between children
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set padding around the column
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

    /// Set horizontal alignment of children
    pub fn align_x(mut self, align: Alignment) -> Self {
        self.align_x = align;
        self
    }
}

impl<M: 'static> Widget<M> for Column<M> {
    fn has_active_overlay(&self) -> bool {
        self.children.iter().any(|c| c.has_active_overlay())
    }

    fn has_active_drag(&self) -> bool {
        self.children.iter().any(|c| c.has_active_drag())
    }

    fn capture_bounds(&self, layout_bounds: Bounds) -> Option<Bounds> {
        // Find any child with active overlay and get its capture bounds
        for (child, child_bounds) in self.children.iter().zip(self.child_bounds.iter()) {
            if child.has_active_overlay() {
                let absolute_bounds = Bounds::new(
                    layout_bounds.x + child_bounds.x,
                    layout_bounds.y + child_bounds.y,
                    child_bounds.width,
                    child_bounds.height,
                );
                if let Some(child_capture) = child.capture_bounds(absolute_bounds) {
                    return Some(layout_bounds.union(&child_capture));
                }
            }
        }
        None
    }

    fn layout(&mut self, available: Size) -> Size {
        log::debug!("Column layout: available={:?}", available);

        let inner_available = Size::new(
            available.width - self.padding.horizontal(),
            available.height - self.padding.vertical(),
        );

        // First pass: layout all children
        let mut total_fixed_height = 0.0;
        let mut max_width: f32 = 0.0;

        for (i, child) in self.children.iter_mut().enumerate() {
            let child_size = child.layout(Size::new(inner_available.width, inner_available.height));
            log::debug!("  Column child {} layout: {:?}", i, child_size);
            max_width = max_width.max(child_size.width);
            total_fixed_height += child_size.height;
        }

        // Add spacing
        if !self.children.is_empty() {
            total_fixed_height += self.spacing * (self.children.len() - 1) as f32;
        }

        // Calculate fill space
        let remaining_height = (inner_available.height - total_fixed_height).max(0.0);

        // Second pass: calculate actual positions
        self.child_bounds.clear();
        let mut y = self.padding.top;

        for (i, child) in self.children.iter().enumerate() {
            let child_size = child.cached_size();
            let child_height = child_size.height;

            let x_offset = self.align_x.align(max_width, child_size.width);

            let child_bounds = Bounds::new(
                self.padding.left + x_offset,
                y,
                child_size.width,
                child_height,
            );
            log::debug!("  Column child {} bounds: {:?}", i, child_bounds);
            self.child_bounds.push(child_bounds);

            y += child_height + self.spacing;
        }

        // Calculate total size
        let content_height = if self.children.is_empty() {
            0.0
        } else {
            y - self.spacing + self.padding.bottom - self.padding.top
        };
        let content_width = max_width + self.padding.horizontal();

        Size::new(
            self.width.resolve(available.width, content_width),
            self.height.resolve(available.height, content_height + self.padding.vertical()),
        )
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        log::debug!("Column draw: bounds={:?}, {} children", bounds, self.children.len());
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
