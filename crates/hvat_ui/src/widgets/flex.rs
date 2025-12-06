//! Flexible layout widget that can arrange children horizontally or vertically.
//!
//! This module provides a unified `FlexLayout` widget that handles both row and column layouts.
//! It uses explicit SizingMode from Layout to determine which children want to fill available space.
//!
//! # Compile-Time Context Safety
//!
//! `FlexLayout` is generic over a `Context` type parameter that controls whether Fill children
//! are allowed:
//!
//! - `FlexLayout<'a, Message, Bounded>` - Normal context, Fill children allowed
//! - `FlexLayout<'a, Message, Unbounded>` - Inside scrollable, Fill children NOT allowed
//!
//! This prevents the common bug of putting Fill widgets inside scrollables, which would
//! cause infinite expansion.

use std::marker::PhantomData;
use crate::{Bounded, ConcreteSize, ConcreteSizeXY, Element, Event, Layout, Limits, Rectangle, Renderer, SizingMode, Unbounded, Widget};

/// Direction for flex layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    /// Arrange children horizontally (row).
    Horizontal,
    /// Arrange children vertically (column).
    #[default]
    Vertical,
}

/// Information about a child's layout for sizing calculations.
struct ChildLayoutInfo {
    /// The layout returned by the child
    #[allow(dead_code)]
    layout: Layout,
    /// Whether this child wants to fill in the main axis
    fills_main: bool,
    /// Whether this child wants to fill in the cross axis
    fills_cross: bool,
    /// The fill weight (0 if not filling)
    fill_weight: f32,
    /// The measured main axis size
    main_size: f32,
    /// The measured cross axis size
    cross_size: f32,
}

/// A flexible layout that arranges children either horizontally or vertically.
///
/// This widget supports:
/// - Configurable direction (row or column)
/// - Spacing between children
/// - Fill-behavior where children with SizingMode::Fill share remaining space
/// - Wrap mode for horizontal layouts (wraps to next line when content exceeds width)
///
/// # Context Type Parameter
///
/// The `Context` type parameter controls whether Fill children are allowed:
/// - `Bounded` (default): Fill children allowed, use in normal layouts
/// - `Unbounded`: Fill children NOT allowed, use inside scrollables
///
/// This is enforced at compile time - you cannot add Fill children to an unbounded flex.
pub struct FlexLayout<'a, Message, Context = Bounded> {
    children: Vec<Element<'a, Message>>,
    spacing: f32,
    direction: FlexDirection,
    /// Whether to wrap children to next line when they exceed available width (horizontal only)
    wrap: bool,
    /// Phantom data for context type
    _context: PhantomData<Context>,
}

// ============================================================================
// Common methods for all contexts
// ============================================================================

impl<'a, Message, Context> FlexLayout<'a, Message, Context> {
    /// Create a new flex layout with the given direction.
    fn new_with_context(direction: FlexDirection) -> Self {
        Self {
            children: Vec::new(),
            spacing: 0.0,
            direction,
            wrap: false,
            _context: PhantomData,
        }
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

    /// Add a child element.
    ///
    /// Note: In `Unbounded` context, this is the only way to add children.
    /// Fill widgets added here will have their Fill behavior ignored (they'll use minimum_size).
    pub fn push(mut self, child: Element<'a, Message>) -> Self {
        self.children.push(child);
        self
    }

    // === Internal helpers ===

    /// Get the main axis size from bounds (width for horizontal, height for vertical).
    fn main_size(&self, bounds: &Rectangle) -> f32 {
        if self.is_horizontal() {
            bounds.width
        } else {
            bounds.height
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

    /// Create limits for measuring children.
    /// Propagates the cross-axis constraint but allows infinite main axis for measurement.
    fn create_child_limits(&self, limits: &Limits) -> Limits {
        if self.is_horizontal() {
            let max_height = if limits.max_height.is_finite() {
                limits.max_height
            } else {
                f32::INFINITY
            };
            Limits::with_range(0.0, f32::INFINITY, 0.0, max_height)
        } else {
            let max_width = if limits.max_width.is_finite() {
                limits.max_width
            } else {
                f32::INFINITY
            };
            Limits::with_range(0.0, max_width, 0.0, f32::INFINITY)
        }
    }

    /// Measure all children and collect layout info.
    fn measure_children(&self, limits: &Limits) -> Vec<ChildLayoutInfo> {
        let child_limits = self.create_child_limits(limits);

        self.children
            .iter()
            .map(|child| {
                let layout = child.widget().layout(&child_limits);
                let size = layout.size();

                let (fills_main, fills_cross, fill_weight, main_size, cross_size) = if self.is_horizontal() {
                    (
                        layout.fills_width(),
                        layout.fills_height(),
                        layout.width_fill_weight(),
                        size.width,
                        size.height,
                    )
                } else {
                    (
                        layout.fills_height(),
                        layout.fills_width(),
                        layout.height_fill_weight(),
                        size.height,
                        size.width,
                    )
                };

                ChildLayoutInfo {
                    layout,
                    fills_main,
                    fills_cross,
                    fill_weight,
                    main_size,
                    cross_size,
                }
            })
            .collect()
    }

    /// Calculate the fill size and total weight from child infos.
    fn calculate_fill_distribution(&self, child_infos: &[ChildLayoutInfo], available_main: f32) -> (f32, f32) {
        let mut total_fixed: f32 = 0.0;
        let mut total_weight: f32 = 0.0;

        for (i, info) in child_infos.iter().enumerate() {
            if info.fills_main {
                total_weight += info.fill_weight;
            } else {
                total_fixed += info.main_size;
            }

            if i > 0 {
                total_fixed += self.spacing;
            }
        }

        let fill_size = if total_weight > 0.0 && available_main.is_finite() {
            (available_main - total_fixed).max(0.0)
        } else {
            0.0
        };

        (fill_size, total_weight)
    }

    /// Position a child given child info and current position.
    fn position_child(&self, parent: &Rectangle, _info: &ChildLayoutInfo, main_pos: f32, actual_main_size: f32, max_cross: f32) -> Rectangle {
        if self.is_horizontal() {
            let height = if max_cross > 0.0 && parent.height > max_cross * 2.0 {
                max_cross
            } else {
                parent.height
            };
            Rectangle::new(main_pos, parent.y, actual_main_size, height)
        } else {
            let width = if max_cross > 0.0 && parent.width > max_cross * 2.0 {
                max_cross
            } else {
                parent.width
            };
            Rectangle::new(parent.x, main_pos, width, actual_main_size)
        }
    }

    /// Calculate wrapped layout for horizontal flex with wrap enabled.
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

            if i > 0 && x + size.width > bounds.x + bounds.width {
                y += row_height + self.spacing;
                x = bounds.x;
                row_height = 0.0;
            }

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

    fn natural_size_row(&self, max_width: ConcreteSize) -> ConcreteSizeXY {
        let mut total_width = ConcreteSize::ZERO;
        let mut max_height = ConcreteSize::ZERO;

        let child_limits = Limits::with_range(0.0, f32::INFINITY, 0.0, f32::INFINITY);

        for (i, child) in self.children.iter().enumerate() {
            let child_layout = child.widget().layout(&child_limits);

            let child_size = if child_layout.fills_width() {
                child.widget().minimum_size()
            } else {
                child.widget().natural_size(max_width)
            };

            total_width = total_width + child_size.width;
            max_height = max_height.max(child_size.height);

            if i > 0 {
                total_width = total_width + ConcreteSize::new_unchecked(self.spacing);
            }
        }

        ConcreteSizeXY::new(total_width, max_height)
    }

    fn natural_size_column(&self, max_width: ConcreteSize) -> ConcreteSizeXY {
        let mut total_height = ConcreteSize::ZERO;
        let mut max_child_width = ConcreteSize::ZERO;

        let child_limits = Limits::with_range(0.0, max_width.get(), 0.0, f32::INFINITY);

        for (i, child) in self.children.iter().enumerate() {
            let child_layout = child.widget().layout(&child_limits);

            let child_size = if child_layout.fills_height() {
                child.widget().minimum_size()
            } else {
                child.widget().natural_size(max_width)
            };

            total_height = total_height + child_size.height;
            max_child_width = max_child_width.max(child_size.width);

            if i > 0 {
                total_height = total_height + ConcreteSize::new_unchecked(self.spacing);
            }
        }

        ConcreteSizeXY::new(max_child_width, total_height)
    }
}

// ============================================================================
// Bounded-only methods (normal context where Fill is allowed)
// ============================================================================

impl<'a, Message> FlexLayout<'a, Message, Bounded> {
    /// Create a new flex layout with the given direction (bounded context).
    pub fn new(direction: FlexDirection) -> Self {
        Self::new_with_context(direction)
    }

    /// Create a horizontal flex layout (row) in bounded context.
    pub fn row() -> Self {
        Self::new(FlexDirection::Horizontal)
    }

    /// Create a vertical flex layout (column) in bounded context.
    pub fn column() -> Self {
        Self::new(FlexDirection::Vertical)
    }

    /// Create a flex layout with children (bounded context).
    pub fn with_children(direction: FlexDirection, children: Vec<Element<'a, Message>>) -> Self {
        Self {
            children,
            spacing: 0.0,
            direction,
            wrap: false,
            _context: PhantomData,
        }
    }

    /// Convert this flex layout to unbounded context.
    ///
    /// Use this when placing a flex layout inside a scrollable.
    /// Any Fill children will have their Fill behavior ignored (they'll use minimum_size).
    pub fn into_unbounded(self) -> FlexLayout<'a, Message, Unbounded> {
        FlexLayout {
            children: self.children,
            spacing: self.spacing,
            direction: self.direction,
            wrap: self.wrap,
            _context: PhantomData,
        }
    }
}

// ============================================================================
// Unbounded-only methods (inside scrollable, Fill not allowed)
// ============================================================================

impl<'a, Message> FlexLayout<'a, Message, Unbounded> {
    /// Create a new flex layout with the given direction (unbounded context).
    pub fn new_unbounded(direction: FlexDirection) -> Self {
        Self::new_with_context(direction)
    }

    /// Create a horizontal flex layout (row) in unbounded context.
    pub fn row_unbounded() -> Self {
        Self::new_unbounded(FlexDirection::Horizontal)
    }

    /// Create a vertical flex layout (column) in unbounded context.
    pub fn column_unbounded() -> Self {
        Self::new_unbounded(FlexDirection::Vertical)
    }
}

// ============================================================================
// Widget implementation (same for all contexts)
// ============================================================================

impl<'a, Message, Context> Widget<Message> for FlexLayout<'a, Message, Context> {
    fn layout(&self, limits: &Limits) -> Layout {
        // For wrapped horizontal layout, handle separately
        if self.wrap && self.is_horizontal() && limits.max_width.is_finite() {
            let child_limits = Limits::fill();
            let mut x: f32 = 0.0;
            let mut y: f32 = 0.0;
            let mut row_height: f32 = 0.0;
            let mut max_width: f32 = 0.0;

            for (i, child) in self.children.iter().enumerate() {
                let child_layout = child.widget().layout(&child_limits);
                let size = child_layout.size();

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

        // Measure all children
        let child_infos = self.measure_children(limits);

        // Calculate totals
        let mut total_main: f32 = 0.0;
        let mut max_cross: f32 = 0.0;
        let mut has_fill_main = false;
        let mut has_fill_cross = false;

        for (i, info) in child_infos.iter().enumerate() {
            if info.fills_main {
                has_fill_main = true;
            } else {
                total_main += info.main_size;
            }
            if info.fills_cross {
                has_fill_cross = true;
            }
            max_cross = max_cross.max(info.cross_size);

            if i > 0 {
                total_main += self.spacing;
            }
        }

        // Determine our size
        let bounds = if self.is_horizontal() {
            let width = if has_fill_main && limits.max_width.is_finite() {
                limits.max_width
            } else {
                total_main
            };
            let height = if has_fill_cross && limits.max_height.is_finite() {
                limits.max_height
            } else {
                max_cross
            };
            Rectangle::new(0.0, 0.0, width, height)
        } else {
            let height = if has_fill_main && limits.max_height.is_finite() {
                limits.max_height
            } else {
                total_main
            };
            let width = if has_fill_cross && limits.max_width.is_finite() {
                limits.max_width
            } else {
                max_cross
            };
            Rectangle::new(0.0, 0.0, width, height)
        };

        // Report our own fill intent based on children's fill intent
        let mut layout = Layout::new(bounds);
        if self.is_horizontal() {
            if has_fill_main {
                layout = layout.with_width_mode(SizingMode::Fill(1.0));
            }
            if has_fill_cross {
                layout = layout.with_height_mode(SizingMode::Fill(1.0));
            }
        } else {
            if has_fill_main {
                layout = layout.with_height_mode(SizingMode::Fill(1.0));
            }
            if has_fill_cross {
                layout = layout.with_width_mode(SizingMode::Fill(1.0));
            }
        }

        layout
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

        // Create limits for measurement based on our bounds
        let measurement_limits = if self.is_horizontal() {
            Limits::with_range(0.0, f32::INFINITY, 0.0, bounds.height)
        } else {
            Limits::with_range(0.0, bounds.width, 0.0, f32::INFINITY)
        };

        // Measure children
        let child_infos = self.measure_children(&measurement_limits);

        // Find max cross size for reasonable bounds capping
        let max_cross: f32 = child_infos.iter().map(|i| i.cross_size).fold(0.0, f32::max);

        // Calculate fill distribution
        let available_main = self.main_size(&bounds);
        let (fill_size, total_weight) = self.calculate_fill_distribution(&child_infos, available_main);

        // Position and draw children
        let mut child_layouts: Vec<(Rectangle, &Element<'a, Message>)> = Vec::new();
        let mut main_pos = self.main_start(&bounds);

        for (i, (child, info)) in self.children.iter().zip(child_infos.iter()).enumerate() {
            if i > 0 {
                main_pos += self.spacing;
            }

            let actual_main_size = if info.fills_main {
                if total_weight > 0.0 {
                    fill_size * (info.fill_weight / total_weight)
                } else {
                    0.0
                }
            } else {
                info.main_size
            };

            let child_bounds = self.position_child(&bounds, info, main_pos, actual_main_size, max_cross);
            child_layouts.push((child_bounds, child));

            main_pos += actual_main_size;
        }

        // Draw children
        if self.is_horizontal() {
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

        // For MouseMoved events, process ALL children so hover state updates everywhere
        let is_mouse_moved = matches!(event, Event::MouseMoved { .. });

        // Handle wrapped horizontal layout
        if self.wrap && self.is_horizontal() {
            let (positions, _) = self.calculate_wrapped_layout(&bounds);
            let mut first_message: Option<Message> = None;

            for (i, child) in self.children.iter_mut().enumerate() {
                let positioned_layout = Layout::new(positions[i]);
                if let Some(message) = child.widget_mut().on_event(event, &positioned_layout) {
                    if is_mouse_moved {
                        if first_message.is_none() {
                            first_message = Some(message);
                        }
                    } else {
                        return Some(message);
                    }
                }
            }
            return first_message;
        }

        // Create limits for measurement based on our bounds
        let measurement_limits = if self.is_horizontal() {
            Limits::with_range(0.0, f32::INFINITY, 0.0, bounds.height)
        } else {
            Limits::with_range(0.0, bounds.width, 0.0, f32::INFINITY)
        };

        // Measure children
        let child_infos = self.measure_children(&measurement_limits);

        // Find max cross size
        let max_cross: f32 = child_infos.iter().map(|i| i.cross_size).fold(0.0, f32::max);

        // Calculate fill distribution
        let available_main = self.main_size(&bounds);
        let (fill_size, total_weight) = self.calculate_fill_distribution(&child_infos, available_main);

        let is_horizontal = self.is_horizontal();
        let spacing = self.spacing;
        let mut main_pos = self.main_start(&bounds);
        let mut first_message: Option<Message> = None;

        for (i, (child, info)) in self.children.iter_mut().zip(child_infos.iter()).enumerate() {
            if i > 0 {
                main_pos += spacing;
            }

            let actual_main_size = if info.fills_main {
                if total_weight > 0.0 {
                    fill_size * (info.fill_weight / total_weight)
                } else {
                    0.0
                }
            } else {
                info.main_size
            };

            // Position child
            let child_bounds = if is_horizontal {
                let height = if max_cross > 0.0 && bounds.height > max_cross * 2.0 {
                    max_cross
                } else {
                    bounds.height
                };
                Rectangle::new(main_pos, bounds.y, actual_main_size, height)
            } else {
                let width = if max_cross > 0.0 && bounds.width > max_cross * 2.0 {
                    max_cross
                } else {
                    bounds.width
                };
                Rectangle::new(bounds.x, main_pos, width, actual_main_size)
            };
            let positioned_layout = Layout::new(child_bounds);

            if let Some(message) = child.widget_mut().on_event(event, &positioned_layout) {
                if is_mouse_moved {
                    if first_message.is_none() {
                        first_message = Some(message);
                    }
                } else {
                    return Some(message);
                }
            }

            main_pos += actual_main_size;
        }

        first_message
    }

    fn natural_size(&self, max_width: ConcreteSize) -> ConcreteSizeXY {
        if self.children.is_empty() {
            return ConcreteSizeXY::ZERO;
        }

        if self.is_horizontal() {
            self.natural_size_row(max_width)
        } else {
            self.natural_size_column(max_width)
        }
    }

    fn minimum_size(&self) -> ConcreteSizeXY {
        if self.children.is_empty() {
            return ConcreteSizeXY::ZERO;
        }

        let mut total_main = ConcreteSize::ZERO;
        let mut max_cross = ConcreteSize::ZERO;

        for (i, child) in self.children.iter().enumerate() {
            let child_min = child.widget().minimum_size();

            if self.is_horizontal() {
                total_main = total_main + child_min.width;
                max_cross = max_cross.max(child_min.height);
            } else {
                total_main = total_main + child_min.height;
                max_cross = max_cross.max(child_min.width);
            }

            if i > 0 {
                total_main = total_main + ConcreteSize::new_unchecked(self.spacing);
            }
        }

        if self.is_horizontal() {
            ConcreteSizeXY::new(total_main, max_cross)
        } else {
            ConcreteSizeXY::new(max_cross, total_main)
        }
    }
}

/// Helper function to create a horizontal flex layout (row) in bounded context.
pub fn flex_row<'a, Message>() -> FlexLayout<'a, Message, Bounded> {
    FlexLayout::row()
}

/// Helper function to create a vertical flex layout (column) in bounded context.
pub fn flex_column<'a, Message>() -> FlexLayout<'a, Message, Bounded> {
    FlexLayout::column()
}

/// Helper function to create a horizontal flex layout (row) in unbounded context.
pub fn flex_row_unbounded<'a, Message>() -> FlexLayout<'a, Message, Unbounded> {
    FlexLayout::row_unbounded()
}

/// Helper function to create a vertical flex layout (column) in unbounded context.
pub fn flex_column_unbounded<'a, Message>() -> FlexLayout<'a, Message, Unbounded> {
    FlexLayout::column_unbounded()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flex_direction() {
        let row = FlexLayout::<(), Bounded>::row();
        assert!(row.is_horizontal());
        assert!(row.is_empty());

        let col = FlexLayout::<(), Bounded>::column();
        assert!(!col.is_horizontal());
        assert!(col.is_empty());
    }

    #[test]
    fn test_flex_spacing() {
        let flex = FlexLayout::<(), Bounded>::row().spacing(10.0);
        assert_eq!(flex.spacing, 10.0);
    }

    #[test]
    fn test_unbounded_flex() {
        let row = FlexLayout::<(), Unbounded>::row_unbounded();
        assert!(row.is_horizontal());
        assert!(row.is_empty());

        let col = FlexLayout::<(), Unbounded>::column_unbounded();
        assert!(!col.is_horizontal());
        assert!(col.is_empty());
    }

    #[test]
    fn test_into_unbounded() {
        let bounded = FlexLayout::<(), Bounded>::row().spacing(5.0);
        let unbounded = bounded.into_unbounded();
        assert!(unbounded.is_horizontal());
        assert_eq!(unbounded.spacing, 5.0);
    }
}
