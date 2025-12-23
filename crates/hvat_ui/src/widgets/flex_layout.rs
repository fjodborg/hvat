//! Flexible layout widget that can be either horizontal (Row) or vertical (Column)
//!
//! This module provides a unified implementation for both Row and Column layouts,
//! reducing code duplication while maintaining the same API.

use crate::constants::{DEFAULT_SPACING, FILL_DETECTION_TOLERANCE};
use crate::element::Element;
use crate::event::Event;
use crate::layout::{Alignment, Bounds, Length, Padding, Size};
use crate::renderer::Renderer;
use crate::widget::{EventResult, Widget};

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
        // Row: Fill width by default, Center vertical alignment
        // Column: Shrink width, Center horizontal alignment
        let width = match direction {
            FlexDirection::Horizontal => Length::Fill(1.0),
            FlexDirection::Vertical => Length::Shrink,
        };

        // Row: Center vertical alignment (common for toolbars/buttons)
        // Column: Left horizontal alignment (common for text/forms)
        let cross_align = match direction {
            FlexDirection::Horizontal => Alignment::Center,
            FlexDirection::Vertical => Alignment::Left,
        };

        Self {
            children,
            spacing: DEFAULT_SPACING,
            padding: Padding::ZERO,
            width,
            height: Length::Shrink,
            cross_align,
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
        self.layout_flex(available)
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

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> EventResult<M> {
        container_helpers::dispatch_event_to_children(
            &mut self.children,
            &self.child_bounds,
            event,
            bounds,
        )
    }
}

impl<M: 'static> FlexLayout<M> {
    /// Unified layout implementation for both horizontal (Row) and vertical (Column) layouts.
    ///
    /// This method abstracts the axis-specific logic using closures to extract/set the
    /// main-axis and cross-axis dimensions, eliminating code duplication between Row and Column.
    fn layout_flex(&mut self, available: Size) -> Size {
        let is_horizontal = self.direction == FlexDirection::Horizontal;
        let direction_name = if is_horizontal { "Row" } else { "Column" };

        log::debug!("{} layout: available={:?}", direction_name, available);

        let inner_available = self.inner_available(available);

        // Axis accessor functions - main axis is the layout direction, cross axis is perpendicular
        let main_axis = |size: Size| -> f32 {
            if is_horizontal {
                size.width
            } else {
                size.height
            }
        };
        let cross_axis = |size: Size| -> f32 {
            if is_horizontal {
                size.height
            } else {
                size.width
            }
        };
        let make_size = |main: f32, cross: f32| -> Size {
            if is_horizontal {
                Size::new(main, cross)
            } else {
                Size::new(cross, main)
            }
        };

        let inner_main = main_axis(inner_available);
        let inner_cross = cross_axis(inner_available);

        // First pass: layout children with full available size and detect fill children
        let mut total_fixed = 0.0;
        let mut total_fill_weight = 0.0;
        let mut max_cross: f32 = 0.0;
        let mut child_mains: Vec<f32> = Vec::with_capacity(self.children.len());
        let mut fill_indices: Vec<usize> = Vec::with_capacity(self.children.len() / 2);

        for (i, child) in self.children.iter_mut().enumerate() {
            let child_size = child.layout(inner_available);
            let child_main = main_axis(child_size);
            let child_cross = cross_axis(child_size);

            // A child is considered "fill" if it returns the full available main-axis size
            let is_fill = child_main >= inner_main - FILL_DETECTION_TOLERANCE;

            if is_fill {
                fill_indices.push(i);
                total_fill_weight += 1.0;
                child_mains.push(0.0);
                max_cross = max_cross.max(child_cross);
                log::debug!(
                    "  {} child {} is FILL: size={:?}",
                    direction_name,
                    i,
                    child_size
                );
            } else {
                total_fixed += child_main;
                child_mains.push(child_main);
                max_cross = max_cross.max(child_cross);
                log::debug!(
                    "  {} child {} is FIXED: main={}",
                    direction_name,
                    i,
                    child_main
                );
            }
        }

        // Add spacing for visible children (both fixed and fill children need spacing)
        // Fixed children have non-zero main, fill children will get allocated space
        let fixed_count = child_mains.iter().filter(|&&m| m > 0.0).count();
        let fill_count = fill_indices.len();
        let visible_count = fixed_count + fill_count;
        if visible_count > 1 {
            total_fixed += self.spacing * (visible_count - 1) as f32;
        }

        // Calculate fill space and distribute to fill children
        let remaining = (inner_main - total_fixed).max(0.0);
        let fill_per_unit = if total_fill_weight > 0.0 {
            remaining / total_fill_weight
        } else {
            0.0
        };

        // Second pass: re-layout fill children with allocated main-axis size
        for &idx in &fill_indices {
            child_mains[idx] = fill_per_unit;
            let child_size = self.children[idx].layout(make_size(fill_per_unit, inner_cross));
            max_cross = max_cross.max(cross_axis(child_size));
            log::debug!(
                "  {} child {} FILL allocated: main={}, got={:?}",
                direction_name,
                idx,
                fill_per_unit,
                child_size
            );
        }

        // For horizontal layout, we may need to re-layout non-fill children too
        // since the cross-axis (height) may have changed
        if is_horizontal {
            for (idx, child) in self.children.iter_mut().enumerate() {
                if !fill_indices.contains(&idx) {
                    let child_size = child.layout(make_size(child_mains[idx], inner_cross));
                    max_cross = max_cross.max(cross_axis(child_size));
                }
            }
        }

        // Third pass: calculate actual positions
        self.child_bounds.clear();
        let (start_main, start_cross) = if is_horizontal {
            (self.padding.left, self.padding.top)
        } else {
            (self.padding.top, self.padding.left)
        };

        let mut pos = start_main;
        let mut had_visible_child = false;

        for (i, child) in self.children.iter().enumerate() {
            let child_main = child_mains[i];
            let cached = child.cached_size();
            let child_cross = cross_axis(cached);

            // Don't apply cross-alignment offset to zero-sized children (overlays)
            // They handle their own positioning
            let is_overlay = child_main == 0.0 && child_cross == 0.0;
            let cross_offset = if !is_overlay {
                self.cross_align.align(max_cross, child_cross)
            } else {
                0.0
            };

            // Add spacing before this child if there was a previous visible child
            if child_main > 0.0 && had_visible_child {
                pos += self.spacing;
            }

            let bounds = if is_horizontal {
                Bounds::new(pos, start_cross + cross_offset, child_main, child_cross)
            } else {
                Bounds::new(start_cross + cross_offset, pos, child_cross, child_main)
            };

            log::debug!(
                "  {} child {} final bounds: {:?}",
                direction_name,
                i,
                bounds
            );
            self.child_bounds.push(bounds);

            // Only advance position for non-zero main-axis children
            if child_main > 0.0 {
                pos += child_main;
                had_visible_child = true;
            }
        }

        // Calculate content dimensions
        let content_main = pos - start_main;

        // Resolve final size, adding padding to content dimensions
        let (content_width, content_height) = if is_horizontal {
            (
                content_main + self.padding.horizontal(),
                max_cross + self.padding.vertical(),
            )
        } else {
            (
                max_cross + self.padding.horizontal(),
                content_main + self.padding.vertical(),
            )
        };

        Size::new(
            self.width.resolve(available.width, content_width),
            self.height.resolve(available.height, content_height),
        )
    }
}
