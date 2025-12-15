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
        log::debug!("Row layout: available={:?}", available);

        let inner_available = Size::new(
            available.width - self.padding.horizontal(),
            available.height - self.padding.vertical(),
        );

        // First pass: layout children with full available size and detect fill children
        // A child is "fill" if it returns a width equal (or very close) to what we gave it
        let mut total_fixed_width = 0.0;
        let mut total_fill_weight = 0.0;
        let mut max_height: f32 = 0.0;
        let mut child_widths: Vec<f32> = Vec::with_capacity(self.children.len());
        let mut fill_indices: Vec<usize> = Vec::new();

        for (i, child) in self.children.iter_mut().enumerate() {
            // Layout with full available width
            let child_size = child.layout(Size::new(inner_available.width, inner_available.height));

            // A child is considered "fill" if it returns the full available width (or close to it)
            // This is a heuristic: if width returned >= available width - 1, it's a fill child
            let is_fill = child_size.width >= inner_available.width - 1.0;

            if is_fill {
                // Mark as fill child, use weight 1.0
                fill_indices.push(i);
                total_fill_weight += 1.0;
                child_widths.push(0.0); // Will be calculated later
                max_height = max_height.max(child_size.height);
                log::debug!("  Row child {} is FILL: size={:?}", i, child_size);
            } else {
                // Fixed width child
                total_fixed_width += child_size.width;
                child_widths.push(child_size.width);
                max_height = max_height.max(child_size.height);
                log::debug!("  Row child {} is FIXED: width={}", i, child_size.width);
            }
        }

        // Add spacing to fixed width calculation
        if !self.children.is_empty() {
            total_fixed_width += self.spacing * (self.children.len() - 1) as f32;
        }

        // Calculate fill space and distribute to fill children
        let remaining_width = (inner_available.width - total_fixed_width).max(0.0);
        let fill_width_per_unit = if total_fill_weight > 0.0 {
            remaining_width / total_fill_weight
        } else {
            0.0
        };

        // Update fill children widths and re-layout them with their allocated space
        for &idx in &fill_indices {
            let fill_width = fill_width_per_unit;
            child_widths[idx] = fill_width;

            // Re-layout fill child with its allocated width
            let child_size = self.children[idx].layout(Size::new(fill_width, inner_available.height));
            max_height = max_height.max(child_size.height);
            log::debug!("  Row child {} FILL allocated: width={}, got={:?}", idx, fill_width, child_size);
        }

        // Second pass: calculate actual positions
        self.child_bounds.clear();
        let mut x = self.padding.left;

        for (i, child) in self.children.iter().enumerate() {
            let child_width = child_widths[i];
            let child_height = child.cached_size().height;

            let y_offset = self.align_y.align(max_height, child_height);

            let child_bounds = Bounds::new(
                x,
                self.padding.top + y_offset,
                child_width,
                child_height,
            );
            log::debug!("  Row child {} final bounds: {:?}", i, child_bounds);
            self.child_bounds.push(child_bounds);

            x += child_width + self.spacing;
        }

        // Calculate total size
        let content_width = inner_available.width;
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
