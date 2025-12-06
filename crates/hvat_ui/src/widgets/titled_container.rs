//! A container widget with a small title bar in the corner.

use crate::{Color, Element, Event, Layout, Limits, MeasureContext, Rectangle, Renderer, Widget};

/// Position of the title bar (left or right).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitlePosition {
    /// Title on left side (default)
    #[default]
    Left,
    /// Title on right side
    Right,
}

/// Style of the title bar display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitleStyle {
    /// Title bar overlays the content in the corner (original style)
    #[default]
    Inside,
    /// Title bar is above the content, full width, content starts below it
    Above,
    /// No title bar displayed
    None,
}

/// A container with a small title label displayed in a corner.
pub struct TitledContainer<'a, Message> {
    /// Title text
    title: String,
    /// Child content
    child: Element<'a, Message>,
    /// Optional header content (rendered at top, below title bar)
    header: Option<Element<'a, Message>>,
    /// Optional footer content (rendered at bottom, above status bar area)
    footer: Option<Element<'a, Message>>,
    /// Title position (left or right)
    title_position: TitlePosition,
    /// Title style (inside, above, or none)
    title_style: TitleStyle,
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
    /// Cached header height for use in draw/event
    header_height: std::cell::Cell<f32>,
    /// Cached footer height for use in draw/event
    footer_height: std::cell::Cell<f32>,
}

impl<'a, Message> TitledContainer<'a, Message> {
    /// Create a new titled container.
    pub fn new(title: impl Into<String>, child: Element<'a, Message>) -> Self {
        Self {
            title: title.into(),
            child,
            header: None,
            footer: None,
            title_position: TitlePosition::Left,
            title_style: TitleStyle::Inside,
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
            header_height: std::cell::Cell::new(0.0),
            footer_height: std::cell::Cell::new(0.0),
        }
    }

    /// Set a header element to be rendered at the top of the container (below title bar).
    /// The header is measured first, then remaining height goes to the main child.
    pub fn header(mut self, header: Element<'a, Message>) -> Self {
        self.header = Some(header);
        self
    }

    /// Set a footer element to be rendered at the bottom of the container.
    /// The footer is measured first, then remaining height goes to the main child.
    pub fn footer(mut self, footer: Element<'a, Message>) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Make the container fill all available space.
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Set the title position (left or right).
    pub fn title_position(mut self, position: TitlePosition) -> Self {
        self.title_position = position;
        self
    }

    /// Set the title style (inside, above, or none).
    pub fn title_style(mut self, style: TitleStyle) -> Self {
        self.title_style = style;
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
        let content_max_width = if limits.max_width.is_finite() {
            (limits.max_width - self.content_padding * 2.0).max(0.0)
        } else {
            f32::INFINITY
        };

        // Calculate title bar height based on style
        let title_h = match self.title_style {
            TitleStyle::Above => self.title_bar_height(),
            TitleStyle::Inside | TitleStyle::None => 0.0,
        };

        // Measure header if present (propagate context)
        let header_h = if let Some(ref header) = self.header {
            let mut header_limits = Limits::with_range(0.0, content_max_width, 0.0, f32::INFINITY);
            header_limits.context = limits.context;
            let header_layout = header.widget().layout(&header_limits);
            header_layout.size().height
        } else {
            0.0
        };
        self.header_height.set(header_h);

        // Measure footer if present (propagate context)
        let footer_h = if let Some(ref footer) = self.footer {
            let mut footer_limits = Limits::with_range(0.0, content_max_width, 0.0, f32::INFINITY);
            footer_limits.context = limits.context;
            let footer_layout = footer.widget().layout(&footer_limits);
            footer_layout.size().height
        } else {
            0.0
        };
        self.footer_height.set(footer_h);

        // Calculate child limits (accounting for content padding, title bar (if Above), header, AND footer)
        let child_max_height = if limits.max_height.is_finite() {
            (limits.max_height - self.content_padding * 2.0 - title_h - header_h - footer_h).max(0.0)
        } else {
            f32::INFINITY
        };

        // Propagate measurement context to child
        let mut child_limits = Limits::with_range(0.0, content_max_width, 0.0, child_max_height);
        child_limits.context = limits.context;

        let child_layout = self.child.widget().layout(&child_limits);
        let child_size = child_layout.size();

        // In ContentMeasure mode, always report natural size (ignore fill)
        let is_content_measure = limits.context == MeasureContext::ContentMeasure;

        // Container size depends on fill mode (but not in ContentMeasure)
        let (width, height) = if self.fill && !is_content_measure {
            // Fill mode: use all available space (up to limits), fallback to content size
            let w = if limits.max_width.is_finite() {
                limits.max_width
            } else {
                child_size.width + self.content_padding * 2.0
            };
            let h = if limits.max_height.is_finite() {
                limits.max_height
            } else {
                child_size.height + title_h + header_h + footer_h + self.content_padding * 2.0
            };
            (w, h)
        } else {
            // Content mode: size to child + title (if Above) + header + footer + padding
            let w = (child_size.width + self.content_padding * 2.0).min(limits.max_width);
            let h = (child_size.height + title_h + header_h + footer_h + self.content_padding * 2.0)
                .min(limits.max_height);
            (w, h)
        };

        let bounds = Rectangle::new(0.0, 0.0, width, height);

        // Report fill intent (only when NOT in ContentMeasure mode)
        if self.fill && !is_content_measure {
            Layout::fill_both(bounds)
        } else {
            Layout::new(bounds)
        }
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();
        let header_h = self.header_height.get();
        let footer_h = self.footer_height.get();

        // Calculate title bar height based on style
        let title_h = match self.title_style {
            TitleStyle::Above => self.title_bar_height(),
            TitleStyle::Inside | TitleStyle::None => 0.0,
        };

        // Draw background if specified
        if let Some(bg) = self.background {
            renderer.fill_rect(bounds, bg);
        }

        // Draw border if specified
        if let Some(border) = self.border_color {
            renderer.stroke_rect(bounds, border, self.border_width);
        }

        let content_width = (bounds.width - self.content_padding * 2.0).max(0.0);

        // For TitleStyle::Above, draw full-width title bar first and offset content below
        if self.title_style == TitleStyle::Above {
            let title_height = self.title_bar_height();

            // Title bar background (full width)
            let title_rect = Rectangle::new(bounds.x, bounds.y, bounds.width, title_height);
            renderer.fill_rect(title_rect, self.title_bg_color);

            // Title text (positioned based on title_position)
            let text_x = match self.title_position {
                TitlePosition::Left => bounds.x + self.title_padding_h,
                TitlePosition::Right => {
                    bounds.x + bounds.width - self.title_bar_width() + self.title_padding_h
                }
            };
            let text_y = bounds.y + self.title_padding_v;
            renderer.draw_text(
                &self.title,
                crate::Point::new(text_x, text_y),
                self.title_text_color,
                self.title_font_size,
            );
        }

        // Draw header below title bar (if Above style) or at top
        if let Some(ref header) = self.header {
            let header_bounds = Rectangle::new(
                bounds.x + self.content_padding,
                bounds.y + self.content_padding + title_h,
                content_width,
                header_h,
            );
            let header_layout = Layout::new(header_bounds);
            header.widget().draw(renderer, &header_layout);
        }

        // Draw child with offset for padding, title bar (if Above), header, and reduced height for footer
        let child_height =
            (bounds.height - self.content_padding * 2.0 - title_h - header_h - footer_h).max(0.0);
        let child_bounds = Rectangle::new(
            bounds.x + self.content_padding,
            bounds.y + self.content_padding + title_h + header_h,
            content_width,
            child_height,
        );
        let child_layout = Layout::new(child_bounds);
        self.child.widget().draw(renderer, &child_layout);

        // Draw footer at bottom if present
        if let Some(ref footer) = self.footer {
            let footer_bounds = Rectangle::new(
                bounds.x + self.content_padding,
                bounds.y + bounds.height - footer_h - self.content_padding,
                content_width,
                footer_h,
            );
            let footer_layout = Layout::new(footer_bounds);
            footer.widget().draw(renderer, &footer_layout);
        }

        // For TitleStyle::Inside, draw title bar overlaying content (original behavior)
        if self.title_style == TitleStyle::Inside {
            let title_height = self.title_bar_height();
            let title_width = self.title_bar_width();

            let title_x = match self.title_position {
                TitlePosition::Left => bounds.x,
                TitlePosition::Right => bounds.x + bounds.width - title_width,
            };
            let title_y = bounds.y;

            // Title bar background
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
        // TitleStyle::None - don't draw any title
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let header_h = self.header_height.get();
        let footer_h = self.footer_height.get();
        let content_width = (bounds.width - self.content_padding * 2.0).max(0.0);

        // Calculate title bar height based on style
        let title_h = match self.title_style {
            TitleStyle::Above => self.title_bar_height(),
            TitleStyle::Inside | TitleStyle::None => 0.0,
        };

        let child_height =
            (bounds.height - self.content_padding * 2.0 - title_h - header_h - footer_h).max(0.0);

        // First, try header events
        if let Some(ref mut header) = self.header {
            let header_bounds = Rectangle::new(
                bounds.x + self.content_padding,
                bounds.y + self.content_padding + title_h,
                content_width,
                header_h,
            );
            let header_layout = Layout::new(header_bounds);
            if let Some(msg) = header.widget_mut().on_event(event, &header_layout) {
                return Some(msg);
            }
        }

        // Then try footer events
        if let Some(ref mut footer) = self.footer {
            let footer_bounds = Rectangle::new(
                bounds.x + self.content_padding,
                bounds.y + bounds.height - footer_h - self.content_padding,
                content_width,
                footer_h,
            );
            let footer_layout = Layout::new(footer_bounds);
            if let Some(msg) = footer.widget_mut().on_event(event, &footer_layout) {
                return Some(msg);
            }
        }

        // Finally try child events
        let child_bounds = Rectangle::new(
            bounds.x + self.content_padding,
            bounds.y + self.content_padding + title_h + header_h,
            content_width,
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
