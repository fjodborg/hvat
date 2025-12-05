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
/// - Wrap mode for horizontal layouts (wraps to next line when content exceeds width)
pub struct FlexLayout<'a, Message> {
    children: Vec<Element<'a, Message>>,
    spacing: f32,
    direction: FlexDirection,
    /// Whether to wrap children to next line when they exceed available width (horizontal only)
    wrap: bool,
}

impl<'a, Message> FlexLayout<'a, Message> {
    /// Create a new flex layout with the given direction.
    pub fn new(direction: FlexDirection) -> Self {
        Self {
            children: Vec::new(),
            spacing: 0.0,
            direction,
            wrap: false,
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
            wrap: false,
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

    /// Enable wrap mode (horizontal layouts only).
    /// When enabled, children wrap to the next line when they exceed available width.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
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

    /// Calculate wrapped layout for horizontal flex with wrap enabled.
    /// Returns Vec of (child_bounds, row_index) for each child, plus total height.
    fn calculate_wrapped_layout(&self, bounds: &Rectangle) -> (Vec<Rectangle>, f32) {
        let child_limits = Limits::fill();
        let mut positions: Vec<Rectangle> = Vec::new();
        let mut x = bounds.x;
        let mut y = bounds.y;
        let mut row_height: f32 = 0.0;
        let mut total_height: f32 = 0.0;

        for (i, child) in self.children.iter().enumerate() {
            let child_layout = child.widget().layout(&child_limits);
            let size = child_layout.size();

            // Check if we need to wrap to next line
            if i > 0 && x + size.width > bounds.x + bounds.width {
                // Move to next line
                y += row_height + self.spacing;
                x = bounds.x;
                row_height = 0.0;
            }

            // Add spacing between items on same row
            if i > 0 && x > bounds.x {
                x += self.spacing;
            }

            positions.push(Rectangle::new(x, y, size.width, size.height));
            x += size.width;
            row_height = row_height.max(size.height);
            total_height = (y - bounds.y) + row_height;
        }

        (positions, total_height)
    }
}

impl<'a, Message> Widget<Message> for FlexLayout<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // For wrapped horizontal layout, we need to consider available width
        if self.wrap && self.is_horizontal() && limits.max_width.is_finite() {
            let child_limits = Limits::fill();
            let mut x: f32 = 0.0;
            let mut y: f32 = 0.0;
            let mut row_height: f32 = 0.0;
            let mut max_width: f32 = 0.0;

            for (i, child) in self.children.iter().enumerate() {
                let child_layout = child.widget().layout(&child_limits);
                let size = child_layout.size();

                // Check if we need to wrap
                if i > 0 && x + size.width > limits.max_width {
                    y += row_height + self.spacing;
                    max_width = max_width.max(x - self.spacing);
                    x = 0.0;
                    row_height = 0.0;
                }

                if i > 0 && x > 0.0 {
                    x += self.spacing;
                }

                x += size.width;
                row_height = row_height.max(size.height);
            }

            max_width = max_width.max(x);
            let total_height = y + row_height;

            return Layout::new(Rectangle::new(0.0, 0.0, max_width, total_height));
        }

        // Non-wrapped layout (original behavior)
        // For vertical layouts, propagate width constraint to children so wrapped rows can work
        let child_limits = if !self.is_horizontal() && limits.max_width.is_finite() {
            Limits::with_range(0.0, limits.max_width, 0.0, f32::INFINITY)
        } else {
            Limits::fill()
        };
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

        // Handle wrapped horizontal layout
        if self.wrap && self.is_horizontal() {
            let (positions, _) = self.calculate_wrapped_layout(&bounds);
            for (i, child) in self.children.iter().enumerate() {
                let positioned_layout = Layout::new(positions[i]);
                child.widget().draw(renderer, &positioned_layout);
            }
            return;
        }

        // Non-wrapped layout (original behavior)
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

        // For MouseMoved events, we need to process ALL children so hover state updates everywhere.
        // We return the FIRST message encountered, which gives priority to earlier children
        // (e.g., scrollbar drag messages take priority over tooltip hover messages).
        let is_mouse_moved = matches!(event, Event::MouseMoved { .. });

        // Handle wrapped horizontal layout
        if self.wrap && self.is_horizontal() {
            let (positions, _) = self.calculate_wrapped_layout(&bounds);
            let mut first_message: Option<Message> = None;

            for (i, child) in self.children.iter_mut().enumerate() {
                let positioned_layout = Layout::new(positions[i]);
                if let Some(message) = child.widget_mut().on_event(event, &positioned_layout) {
                    if is_mouse_moved {
                        // For MouseMoved, keep processing all children but remember the FIRST message
                        if first_message.is_none() {
                            first_message = Some(message);
                        }
                    } else {
                        // For other events, return immediately (original behavior)
                        return Some(message);
                    }
                }
            }
            return first_message;
        }

        // Non-wrapped layout
        let (child_sizes, fill_size) = self.calculate_child_sizes(&bounds);

        // Cache values to avoid borrowing self during iteration
        let is_horizontal = self.is_horizontal();
        let spacing = self.spacing;
        let main_start = if is_horizontal { bounds.x } else { bounds.y };
        let mut main_pos = main_start;
        let mut first_message: Option<Message> = None;

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
                if is_mouse_moved {
                    // For MouseMoved, keep processing all children but remember the FIRST message
                    if first_message.is_none() {
                        first_message = Some(message);
                    }
                } else {
                    // For other events, return immediately (original behavior)
                    return Some(message);
                }
            }

            main_pos += actual_size;
        }

        first_message
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
