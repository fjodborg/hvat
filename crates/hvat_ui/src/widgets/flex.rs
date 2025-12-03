//! Flexible layout widget that can arrange children horizontally or vertically.
//!
//! This module provides a unified `FlexLayout` widget that handles both row and column layouts,
//! eliminating code duplication while supporting fill-behavior for flexible sizing.

use crate::{Element, Event, Layout, Limits, Rectangle, Renderer, Widget};

/// Direction for flex layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    /// Arrange children horizontally (row).
    Horizontal,
    /// Arrange children vertically (column).
    #[default]
    Vertical,
}

/// A flexible layout that arranges children either horizontally or vertically.
///
/// This widget supports:
/// - Configurable direction (row or column)
/// - Spacing between children
/// - Fill-behavior where children with 0 size in the main axis share remaining space
pub struct FlexLayout<'a, Message> {
    children: Vec<Element<'a, Message>>,
    spacing: f32,
    direction: FlexDirection,
}

impl<'a, Message> FlexLayout<'a, Message> {
    /// Create a new flex layout with the given direction.
    pub fn new(direction: FlexDirection) -> Self {
        Self {
            children: Vec::new(),
            spacing: 0.0,
            direction,
        }
    }

    /// Create a horizontal flex layout (row).
    pub fn row() -> Self {
        Self::new(FlexDirection::Horizontal)
    }

    /// Create a vertical flex layout (column).
    pub fn column() -> Self {
        Self::new(FlexDirection::Vertical)
    }

    /// Create a flex layout with children.
    pub fn with_children(direction: FlexDirection, children: Vec<Element<'a, Message>>) -> Self {
        Self {
            children,
            spacing: 0.0,
            direction,
        }
    }

    /// Add a child element.
    pub fn push(mut self, child: Element<'a, Message>) -> Self {
        self.children.push(child);
        self
    }

    /// Set the spacing between children.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set the layout direction.
    pub fn direction(mut self, direction: FlexDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Check if this is a horizontal layout.
    pub fn is_horizontal(&self) -> bool {
        self.direction == FlexDirection::Horizontal
    }

    /// Get the number of children.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Check if the layout has no children.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    // === Internal helpers for main/cross axis abstraction ===

    /// Get the main axis size from bounds (width for horizontal, height for vertical).
    fn main_size(&self, bounds: &Rectangle) -> f32 {
        if self.is_horizontal() {
            bounds.width
        } else {
            bounds.height
        }
    }

    /// Get the cross axis size from bounds (height for horizontal, width for vertical).
    fn cross_size(&self, bounds: &Rectangle) -> f32 {
        if self.is_horizontal() {
            bounds.height
        } else {
            bounds.width
        }
    }

    /// Create bounds for a child given position in main axis.
    fn child_bounds(&self, parent: &Rectangle, main_pos: f32, main_size: f32) -> Rectangle {
        if self.is_horizontal() {
            Rectangle::new(main_pos, parent.y, main_size, parent.height)
        } else {
            Rectangle::new(parent.x, main_pos, parent.width, main_size)
        }
    }

    /// Get the main axis start position from bounds.
    fn main_start(&self, bounds: &Rectangle) -> f32 {
        if self.is_horizontal() {
            bounds.x
        } else {
            bounds.y
        }
    }

    /// Create limits for child layout.
    fn child_limits(&self, bounds: &Rectangle) -> Limits {
        if self.is_horizontal() {
            Limits::with_range(0.0, f32::INFINITY, 0.0, bounds.height)
        } else {
            Limits::with_range(0.0, bounds.width, 0.0, f32::INFINITY)
        }
    }

    /// Get the main axis size from a child layout.
    fn child_main_size(&self, layout: &Layout) -> f32 {
        let size = layout.size();
        if self.is_horizontal() {
            size.width
        } else {
            size.height
        }
    }

    /// Get the cross axis size from a child layout (for max tracking).
    fn child_cross_size(&self, layout: &Layout) -> f32 {
        let size = layout.size();
        if self.is_horizontal() {
            size.height
        } else {
            size.width
        }
    }

    /// Calculate child sizes with fill handling.
    /// Returns (sizes, fill_size_per_child) where sizes is a vec of (main_size, is_fill).
    fn calculate_child_sizes(&self, bounds: &Rectangle) -> (Vec<(f32, bool)>, f32) {
        let child_limits = self.child_limits(bounds);
        let mut child_sizes: Vec<(f32, bool)> = Vec::new();
        let mut total_fixed = 0.0;
        let mut fill_count = 0;
        let mut max_cross: f32 = 0.0;

        for (i, child) in self.children.iter().enumerate() {
            let child_layout = child.widget().layout(&child_limits);
            let child_main = self.child_main_size(&child_layout);
            max_cross = max_cross.max(self.child_cross_size(&child_layout));

            if child_main == 0.0 {
                // This child wants to fill remaining space
                child_sizes.push((0.0, true));
                fill_count += 1;
            } else {
                child_sizes.push((child_main, false));
                total_fixed += child_main;
            }

            if i > 0 {
                total_fixed += self.spacing;
            }
        }

        // Calculate fill size per child
        let parent_main = self.main_size(bounds);
        let fill_size = if fill_count > 0 && parent_main.is_finite() {
            let remaining = (parent_main - total_fixed).max(0.0);
            remaining / fill_count as f32
        } else {
            0.0
        };

        (child_sizes, fill_size)
    }
}

impl<'a, Message> Widget<Message> for FlexLayout<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let child_limits = Limits::fill();
        let mut total_main: f32 = 0.0;
        let mut max_cross: f32 = 0.0;

        for (i, child) in self.children.iter().enumerate() {
            let child_layout = child.widget().layout(&child_limits);
            let size = child_layout.size();

            if self.is_horizontal() {
                total_main += size.width;
                max_cross = max_cross.max(size.height);
            } else {
                total_main += size.height;
                max_cross = max_cross.max(size.width);
            }

            if i > 0 {
                total_main += self.spacing;
            }
        }

        let bounds = if self.is_horizontal() {
            Rectangle::new(0.0, 0.0, total_main, max_cross)
        } else {
            Rectangle::new(0.0, 0.0, max_cross, total_main)
        };

        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        if !self.is_horizontal() {
            log::debug!(
                "📦 Flex(vertical) draw: bounds={{x:{:.1}, y:{:.1}, w:{:.1}, h:{:.1}}}",
                bounds.x, bounds.y, bounds.width, bounds.height
            );
        }

        let (child_sizes, fill_size) = self.calculate_child_sizes(&bounds);

        // Position children and collect for drawing
        let mut child_layouts: Vec<(Rectangle, &Element<'a, Message>)> = Vec::new();
        let mut main_pos = self.main_start(&bounds);

        for (i, child) in self.children.iter().enumerate() {
            if i > 0 {
                main_pos += self.spacing;
            }

            let (size, is_fill) = child_sizes[i];
            let actual_size = if is_fill { fill_size } else { size };

            let child_bounds = self.child_bounds(&bounds, main_pos, actual_size);
            child_layouts.push((child_bounds, child));

            main_pos += actual_size;
        }

        // Draw in reverse order so first child draws on top (for vertical layout overlapping)
        if self.is_horizontal() {
            // For horizontal, draw in order
            for (child_bounds, child) in child_layouts {
                let positioned_layout = Layout::new(child_bounds);
                child.widget().draw(renderer, &positioned_layout);
            }
        } else {
            // For vertical, draw in reverse so header appears on top
            for (child_bounds, child) in child_layouts.into_iter().rev() {
                let positioned_layout = Layout::new(child_bounds);
                child.widget().draw(renderer, &positioned_layout);
            }
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let (child_sizes, fill_size) = self.calculate_child_sizes(&bounds);

        // Cache values to avoid borrowing self during iteration
        let is_horizontal = self.is_horizontal();
        let spacing = self.spacing;
        let main_start = if is_horizontal { bounds.x } else { bounds.y };
        let mut main_pos = main_start;

        for (i, child) in self.children.iter_mut().enumerate() {
            if i > 0 {
                main_pos += spacing;
            }

            let (size, is_fill) = child_sizes[i];
            let actual_size = if is_fill { fill_size } else { size };

            // Compute bounds inline to avoid borrowing self
            let child_bounds = if is_horizontal {
                Rectangle::new(main_pos, bounds.y, actual_size, bounds.height)
            } else {
                Rectangle::new(bounds.x, main_pos, bounds.width, actual_size)
            };
            let positioned_layout = Layout::new(child_bounds);

            if let Some(message) = child.widget_mut().on_event(event, &positioned_layout) {
                return Some(message);
            }

            main_pos += actual_size;
        }

        None
    }
}

/// Helper function to create a horizontal flex layout (row).
pub fn flex_row<'a, Message>() -> FlexLayout<'a, Message> {
    FlexLayout::row()
}

/// Helper function to create a vertical flex layout (column).
pub fn flex_column<'a, Message>() -> FlexLayout<'a, Message> {
    FlexLayout::column()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flex_direction() {
        let row = FlexLayout::<()>::row();
        assert!(row.is_horizontal());
        assert!(row.is_empty());

        let col = FlexLayout::<()>::column();
        assert!(!col.is_horizontal());
        assert!(col.is_empty());
    }

    #[test]
    fn test_flex_spacing() {
        let flex = FlexLayout::<()>::row().spacing(10.0);
        assert_eq!(flex.spacing, 10.0);
    }
}
