//! Flexible layout widget that can be either horizontal (Row) or vertical (Column)
//!
//! This module provides a unified implementation for both Row and Column layouts,
//! reducing code duplication while maintaining the same API.

use crate::constants::{DEFAULT_SPACING, FILL_DETECTION_TOLERANCE};
use crate::element::Element;
use crate::event::Event;
use crate::layout::{Alignment, Bounds, Length, Padding, Size};
use crate::renderer::Renderer;
use crate::widget::Widget;

use super::container_helpers;

/// Direction for flex layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    /// Horizontal layout (Row)
    Horizontal,
    /// Vertical layout (Column)
    Vertical,
}

/// A flexible layout widget that arranges children either horizontally or vertically
pub struct FlexLayout<M> {
    children: Vec<Element<M>>,
    spacing: f32,
    padding: Padding,
    width: Length,
    height: Length,
    /// Cross-axis alignment (vertical for Row, horizontal for Column)
    cross_align: Alignment,
    /// Layout direction
    direction: FlexDirection,
    /// Cached child bounds from layout
    child_bounds: Vec<Bounds>,
}

impl<M> FlexLayout<M> {
    /// Create a new flex layout with the given direction and children
    pub fn new(direction: FlexDirection, children: Vec<Element<M>>) -> Self {
        Self {
            children,
            spacing: DEFAULT_SPACING,
            padding: Padding::ZERO,
            width: Length::Shrink,
            height: Length::Shrink,
            cross_align: Alignment::Start,
            direction,
            child_bounds: Vec::new(),
        }
    }

    /// Set spacing between children
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set padding around the layout
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

    /// Set cross-axis alignment (vertical for Row, horizontal for Column)
    pub fn cross_align(mut self, align: Alignment) -> Self {
        self.cross_align = align;
        self
    }

    /// Set vertical alignment (for Row - alias for cross_align)
    pub fn align_y(self, align: Alignment) -> Self {
        self.cross_align(align)
    }

    /// Set horizontal alignment (for Column - alias for cross_align)
    pub fn align_x(self, align: Alignment) -> Self {
        self.cross_align(align)
    }

    /// Calculate the inner available space after padding is subtracted
    #[inline]
    fn inner_available(&self, available: Size) -> Size {
        Size::new(
            (available.width - self.padding.horizontal()).max(0.0),
            (available.height - self.padding.vertical()).max(0.0),
        )
    }
}

impl<M: 'static> Widget<M> for FlexLayout<M> {
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
        match self.direction {
            FlexDirection::Horizontal => self.layout_horizontal(available),
            FlexDirection::Vertical => self.layout_vertical(available),
        }
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        log::debug!(
            "{:?} draw: bounds={:?}, {} children",
            self.direction,
            bounds,
            self.children.len()
        );
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

impl<M: 'static> FlexLayout<M> {
    /// Layout children horizontally (Row behavior)
    fn layout_horizontal(&mut self, available: Size) -> Size {
        log::debug!("Row layout: available={:?}", available);

        let inner_available = self.inner_available(available);

        // First pass: layout children with full available size and detect fill children
        let mut total_fixed_width = 0.0;
        let mut total_fill_weight = 0.0;
        let mut max_height: f32 = 0.0;
        let mut child_widths: Vec<f32> = Vec::with_capacity(self.children.len());
        let mut fill_indices: Vec<usize> = Vec::new();

        for (i, child) in self.children.iter_mut().enumerate() {
            let child_size = child.layout(Size::new(inner_available.width, inner_available.height));

            // A child is considered "fill" if it returns the full available width
            let is_fill = child_size.width >= inner_available.width - FILL_DETECTION_TOLERANCE;

            if is_fill {
                fill_indices.push(i);
                total_fill_weight += 1.0;
                child_widths.push(0.0);
                max_height = max_height.max(child_size.height);
                log::debug!("  Row child {} is FILL: size={:?}", i, child_size);
            } else {
                total_fixed_width += child_size.width;
                child_widths.push(child_size.width);
                max_height = max_height.max(child_size.height);
                log::debug!("  Row child {} is FIXED: width={}", i, child_size.width);
            }
        }

        // Add spacing
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

        // Update fill children widths and re-layout all children with final dimensions
        // We need to re-layout all children because the available height may have changed
        // (e.g., when a parent Column distributes remaining space to Fill children)
        for (idx, child) in self.children.iter_mut().enumerate() {
            let child_width = if fill_indices.contains(&idx) {
                child_widths[idx] = fill_width_per_unit;
                fill_width_per_unit
            } else {
                child_widths[idx]
            };

            let child_size = child.layout(Size::new(child_width, inner_available.height));
            max_height = max_height.max(child_size.height);

            if fill_indices.contains(&idx) {
                log::debug!(
                    "  Row child {} FILL allocated: width={}, got={:?}",
                    idx,
                    child_width,
                    child_size
                );
            }
        }

        // Third pass: calculate actual positions
        self.child_bounds.clear();
        let mut x = self.padding.left;

        for (i, child) in self.children.iter().enumerate() {
            let child_width = child_widths[i];
            let child_height = child.cached_size().height;
            let y_offset = self.cross_align.align(max_height, child_height);

            let bounds = Bounds::new(x, self.padding.top + y_offset, child_width, child_height);
            log::debug!("  Row child {} final bounds: {:?}", i, bounds);
            self.child_bounds.push(bounds);

            x += child_width + self.spacing;
        }

        // Calculate content dimensions (without padding - padding added in resolve)
        // x currently points past the last child, so subtract last spacing and starting padding
        let children_width = if self.children.is_empty() {
            0.0
        } else {
            x - self.spacing - self.padding.left
        };

        // Resolve final size, adding padding to content dimensions
        Size::new(
            self.width
                .resolve(available.width, children_width + self.padding.horizontal()),
            self.height
                .resolve(available.height, max_height + self.padding.vertical()),
        )
    }

    /// Layout children vertically (Column behavior)
    fn layout_vertical(&mut self, available: Size) -> Size {
        log::debug!("Column layout: available={:?}", available);

        let inner_available = self.inner_available(available);

        // First pass: layout children to determine fixed vs fill heights
        let mut total_fixed_height = 0.0;
        let mut total_fill_weight = 0.0;
        let mut max_width: f32 = 0.0;
        let mut child_heights: Vec<f32> = Vec::with_capacity(self.children.len());
        let mut fill_indices: Vec<usize> = Vec::new();

        for (i, child) in self.children.iter_mut().enumerate() {
            let child_size = child.layout(Size::new(inner_available.width, inner_available.height));

            // A child is considered "fill" if it returns the full available height
            let is_fill = child_size.height >= inner_available.height - FILL_DETECTION_TOLERANCE;

            if is_fill {
                fill_indices.push(i);
                total_fill_weight += 1.0;
                child_heights.push(0.0);
                max_width = max_width.max(child_size.width);
                log::debug!("  Column child {} is FILL: size={:?}", i, child_size);
            } else {
                total_fixed_height += child_size.height;
                child_heights.push(child_size.height);
                max_width = max_width.max(child_size.width);
                log::debug!("  Column child {} is FIXED: height={}", i, child_size.height);
            }
        }

        // Add spacing
        if !self.children.is_empty() {
            total_fixed_height += self.spacing * (self.children.len() - 1) as f32;
        }

        // Calculate fill space and distribute to fill children
        let remaining_height = (inner_available.height - total_fixed_height).max(0.0);
        let fill_height_per_unit = if total_fill_weight > 0.0 {
            remaining_height / total_fill_weight
        } else {
            0.0
        };

        // Update fill children heights with their proper allocation
        for &idx in &fill_indices {
            let fill_height = fill_height_per_unit;
            child_heights[idx] = fill_height;

            let child_size =
                self.children[idx].layout(Size::new(inner_available.width, fill_height));
            max_width = max_width.max(child_size.width);
            log::debug!(
                "  Column child {} FILL allocated: height={}, got={:?}",
                idx,
                fill_height,
                child_size
            );
        }

        // Second pass: calculate actual positions
        self.child_bounds.clear();
        let mut y = self.padding.top;

        for (i, child) in self.children.iter().enumerate() {
            let child_height = child_heights[i];
            let child_width = child.cached_size().width;
            let x_offset = self.cross_align.align(max_width, child_width);

            let bounds = Bounds::new(
                self.padding.left + x_offset,
                y,
                child_width,
                child_height,
            );
            log::debug!("  Column child {} bounds: {:?}", i, bounds);
            self.child_bounds.push(bounds);

            y += child_height + self.spacing;
        }

        // Calculate content dimensions (without padding - padding added in resolve)
        // y currently points past the last child, so subtract last spacing and starting padding
        let children_height = if self.children.is_empty() {
            0.0
        } else {
            y - self.spacing - self.padding.top
        };

        // Resolve final size, adding padding to content dimensions
        Size::new(
            self.width
                .resolve(available.width, max_width + self.padding.horizontal()),
            self.height
                .resolve(available.height, children_height + self.padding.vertical()),
        )
    }
}
