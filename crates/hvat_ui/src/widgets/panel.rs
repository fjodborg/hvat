//! Panel widget - a container that can draw borders on specific sides
//!
//! Useful for sidebar and section styling.

use crate::element::Element;
use crate::event::Event;
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer};
use crate::widget::{EventResult, Widget};

/// Specifies which sides should have a border
#[derive(Debug, Clone, Copy, Default)]
pub struct BorderSides {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

impl BorderSides {
    /// No borders
    pub const NONE: Self = Self {
        top: false,
        right: false,
        bottom: false,
        left: false,
    };

    /// All borders
    pub const ALL: Self = Self {
        top: true,
        right: true,
        bottom: true,
        left: true,
    };

    /// Create with specified sides
    pub fn new(top: bool, right: bool, bottom: bool, left: bool) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Border on left side only
    pub fn left_only() -> Self {
        Self {
            left: true,
            ..Self::NONE
        }
    }

    /// Border on right side only
    pub fn right_only() -> Self {
        Self {
            right: true,
            ..Self::NONE
        }
    }

    /// Border on top side only
    pub fn top_only() -> Self {
        Self {
            top: true,
            ..Self::NONE
        }
    }

    /// Border on bottom side only
    pub fn bottom_only() -> Self {
        Self {
            bottom: true,
            ..Self::NONE
        }
    }

    /// Border on left and top
    pub fn left_top() -> Self {
        Self {
            left: true,
            top: true,
            ..Self::NONE
        }
    }

    /// Border on right and top
    pub fn right_top() -> Self {
        Self {
            right: true,
            top: true,
            ..Self::NONE
        }
    }
}

/// A panel container that can draw borders on specific sides
pub struct Panel<M> {
    /// Child content
    content: Element<M>,
    /// Which sides have borders
    border_sides: BorderSides,
    /// Border color
    border_color: Color,
    /// Border width
    border_width: f32,
    /// Background color (optional)
    background: Option<Color>,
    /// Width constraint
    width: Length,
    /// Height constraint
    height: Length,
}

impl<M: 'static> Panel<M> {
    /// Create a new panel with content
    pub fn new(content: Element<M>) -> Self {
        Self {
            content,
            border_sides: BorderSides::NONE,
            border_color: Color::BORDER,
            border_width: 1.0,
            background: None,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    /// Set which sides have borders
    pub fn borders(mut self, sides: BorderSides) -> Self {
        self.border_sides = sides;
        self
    }

    /// Set the border color
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Set the border width
    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width;
        self
    }

    /// Set the background color
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
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
}

impl<M: 'static> Widget<M> for Panel<M> {
    fn has_active_overlay(&self) -> bool {
        self.content.has_active_overlay()
    }

    fn has_active_drag(&self) -> bool {
        self.content.has_active_drag()
    }

    fn capture_bounds(&self, layout_bounds: Bounds) -> Option<Bounds> {
        self.content.capture_bounds(layout_bounds)
    }

    fn layout(&mut self, available: Size) -> Size {
        let content_size = self.content.layout(available);
        Size::new(
            self.width.resolve(available.width, content_size.width),
            self.height.resolve(available.height, content_size.height),
        )
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        // Draw background if set
        if let Some(bg) = self.background {
            renderer.fill_rect(bounds, bg);
        }

        // Draw content
        self.content.draw(renderer, bounds);

        // Draw borders on specified sides using stroke_rect_sides
        // which draws borders inside the bounds (consistent with stroke_rect)
        renderer.stroke_rect_sides(
            bounds,
            self.border_color,
            self.border_width,
            self.border_sides,
        );
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> EventResult<M> {
        self.content.on_event(event, bounds)
    }
}
