//! A container widget with a small title bar in the corner.

use crate::{Color, Element, Event, Layout, Limits, Rectangle, Renderer, Widget};

/// Position of the title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitlePosition {
    /// Title in top-left corner (default)
    #[default]
    TopLeft,
    /// Title in top-right corner
    TopRight,
}

/// A container with a small title label displayed in a corner.
pub struct TitledContainer<'a, Message> {
    /// Title text
    title: String,
    /// Child content
    child: Element<'a, Message>,
    /// Title position
    title_position: TitlePosition,
    /// Title bar background color
    title_bg_color: Color,
    /// Title text color
    title_text_color: Color,
    /// Title font size
    title_font_size: f32,
    /// Title bar padding (horizontal)
    title_padding_h: f32,
    /// Title bar padding (vertical)
    title_padding_v: f32,
    /// Content padding
    content_padding: f32,
    /// Border color
    border_color: Option<Color>,
    /// Border width
    border_width: f32,
    /// Background color for the content area
    background: Option<Color>,
    /// Whether to fill available space
    fill: bool,
}

impl<'a, Message> TitledContainer<'a, Message> {
    /// Create a new titled container.
    pub fn new(title: impl Into<String>, child: Element<'a, Message>) -> Self {
        Self {
            title: title.into(),
            child,
            title_position: TitlePosition::TopLeft,
            title_bg_color: Color::new(0.15, 0.18, 0.22, 0.95),
            title_text_color: Color::new(0.8, 0.85, 0.9, 1.0),
            title_font_size: 11.0,
            title_padding_h: 8.0,
            title_padding_v: 3.0,
            content_padding: 0.0,
            border_color: None,
            border_width: 1.0,
            background: None,
            fill: false,
        }
    }

    /// Make the container fill all available space.
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Set the title position.
    pub fn title_position(mut self, position: TitlePosition) -> Self {
        self.title_position = position;
        self
    }

    /// Set the title background color.
    pub fn title_bg_color(mut self, color: Color) -> Self {
        self.title_bg_color = color;
        self
    }

    /// Set the title text color.
    pub fn title_text_color(mut self, color: Color) -> Self {
        self.title_text_color = color;
        self
    }

    /// Set the title font size.
    pub fn title_font_size(mut self, size: f32) -> Self {
        self.title_font_size = size;
        self
    }

    /// Set the content padding.
    pub fn padding(mut self, padding: f32) -> Self {
        self.content_padding = padding;
        self
    }

    /// Set the border color.
    pub fn border(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Set the border width.
    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width;
        self
    }

    /// Set the background color.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Calculate the title bar height.
    fn title_bar_height(&self) -> f32 {
        self.title_font_size + self.title_padding_v * 2.0
    }

    /// Calculate the estimated title bar width based on text.
    fn title_bar_width(&self) -> f32 {
        // Rough estimate: ~7 pixels per character at font size 11
        let char_width = self.title_font_size * 0.6;
        self.title.len() as f32 * char_width + self.title_padding_h * 2.0
    }
}

impl<'a, Message> Widget<Message> for TitledContainer<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Calculate child limits (accounting for content padding)
        let child_max_width = if limits.max_width.is_finite() {
            (limits.max_width - self.content_padding * 2.0).max(0.0)
        } else {
            f32::INFINITY
        };
        let child_max_height = if limits.max_height.is_finite() {
            (limits.max_height - self.content_padding * 2.0).max(0.0)
        } else {
            f32::INFINITY
        };

        let child_limits = Limits::with_range(0.0, child_max_width, 0.0, child_max_height);
        let child_layout = self.child.widget().layout(&child_limits);
        let child_size = child_layout.size();

        // Container size depends on fill mode
        let (width, height) = if self.fill {
            // Fill mode: use all available space (up to limits)
            // Return 0 when limits are infinite to signal "fill remaining space" to parent
            let w = if limits.max_width.is_finite() {
                limits.max_width
            } else {
                0.0 // Signal to parent that we want to fill
            };
            let h = if limits.max_height.is_finite() {
                limits.max_height
            } else {
                0.0 // Signal to parent that we want to fill
            };
            (w, h)
        } else {
            // Content mode: size to child + padding, capped by limits
            let w = (child_size.width + self.content_padding * 2.0).min(limits.max_width);
            let h = (child_size.height + self.content_padding * 2.0).min(limits.max_height);
            (w, h)
        };

        Layout::new(Rectangle::new(0.0, 0.0, width, height))
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        // Draw background if specified
        if let Some(bg) = self.background {
            renderer.fill_rect(bounds, bg);
        }

        // Draw border if specified
        if let Some(border) = self.border_color {
            renderer.stroke_rect(bounds, border, self.border_width);
        }

        // Draw child with offset for padding
        let child_width = (bounds.width - self.content_padding * 2.0).max(0.0);
        let child_height = (bounds.height - self.content_padding * 2.0).max(0.0);
        let child_bounds = Rectangle::new(
            bounds.x + self.content_padding,
            bounds.y + self.content_padding,
            child_width,
            child_height,
        );
        let child_layout = Layout::new(child_bounds);
        self.child.widget().draw(renderer, &child_layout);

        // Draw title bar on top of content
        let title_height = self.title_bar_height();
        let title_width = self.title_bar_width();

        let title_x = match self.title_position {
            TitlePosition::TopLeft => bounds.x,
            TitlePosition::TopRight => bounds.x + bounds.width - title_width,
        };
        let title_y = bounds.y;

        // Title bar background with slight rounded appearance (via solid rect)
        let title_rect = Rectangle::new(title_x, title_y, title_width, title_height);
        renderer.fill_rect(title_rect, self.title_bg_color);

        // Title text
        let text_x = title_x + self.title_padding_h;
        let text_y = title_y + self.title_padding_v;
        renderer.draw_text(
            &self.title,
            crate::Point::new(text_x, text_y),
            self.title_text_color,
            self.title_font_size,
        );
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let child_width = (bounds.width - self.content_padding * 2.0).max(0.0);
        let child_height = (bounds.height - self.content_padding * 2.0).max(0.0);
        let child_bounds = Rectangle::new(
            bounds.x + self.content_padding,
            bounds.y + self.content_padding,
            child_width,
            child_height,
        );
        let child_layout = Layout::new(child_bounds);
        self.child.widget_mut().on_event(event, &child_layout)
    }
}

/// Helper function to create a titled container.
pub fn titled_container<'a, Message>(
    title: impl Into<String>,
    child: Element<'a, Message>,
) -> TitledContainer<'a, Message> {
    TitledContainer::new(title, child)
}
