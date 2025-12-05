//! Tooltip widget that shows hover text above any wrapped content.
//!
//! The tooltip visibility is controlled by the `show` parameter, which should
//! be computed from external state that tracks hover timing.
//!
//! The widget can optionally emit messages when hover state changes via
//! `on_hover_change` callback.

use crate::{Color, Element, Event, Layout, Limits, Point, Rectangle, Renderer, Widget};

/// Tooltip position relative to the hovered element.
#[derive(Debug, Clone, Copy, Default)]
pub enum TooltipPosition {
    /// Tooltip appears above the element.
    #[default]
    Top,
    /// Tooltip appears below the element.
    Bottom,
    /// Tooltip appears to the left of the element.
    Left,
    /// Tooltip appears to the right of the element.
    Right,
}

/// A wrapper widget that shows a tooltip when `show` is true.
///
/// The tooltip visibility is controlled externally - typically by checking
/// hover state and elapsed time in the application's state management.
pub struct Tooltip<'a, Message> {
    /// The wrapped content.
    content: Element<'a, Message>,
    /// The tooltip text to display.
    text: String,
    /// Whether to show the tooltip.
    show: bool,
    /// Position of the tooltip.
    position: TooltipPosition,
    /// Tooltip background color.
    bg_color: Color,
    /// Tooltip text color.
    text_color: Color,
    /// Tooltip padding.
    padding: f32,
    /// Callback when hover state changes (receives true when hovered, false when not).
    on_hover_change: Option<Box<dyn Fn(bool) -> Message>>,
    /// Whether this tooltip is currently the active one (for detecting hover-out).
    is_active: bool,
}

impl<'a, Message> Tooltip<'a, Message> {
    /// Create a new tooltip wrapper around content.
    pub fn new(content: Element<'a, Message>, text: impl Into<String>) -> Self {
        Self {
            content,
            text: text.into(),
            show: false,
            position: TooltipPosition::default(),
            bg_color: Color::rgb(0.1, 0.1, 0.1),
            text_color: Color::WHITE,
            padding: 6.0,
            on_hover_change: None,
            is_active: false,
        }
    }

    /// Set whether the tooltip should be shown.
    pub fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    /// Set whether this tooltip is the currently active/hovered one.
    /// When active and mouse moves away, will emit hover_change(false).
    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Set the tooltip position.
    pub fn position(mut self, position: TooltipPosition) -> Self {
        self.position = position;
        self
    }

    /// Set the tooltip background color.
    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    /// Set the tooltip text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set the tooltip padding.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set callback for when hover state changes.
    /// The callback receives `true` when the mouse enters, `false` when it leaves.
    pub fn on_hover_change<F>(mut self, f: F) -> Self
    where
        F: Fn(bool) -> Message + 'static,
    {
        self.on_hover_change = Some(Box::new(f));
        self
    }

    /// Calculate tooltip bounds based on content bounds and position.
    fn tooltip_bounds(&self, content_bounds: &Rectangle) -> Rectangle {
        // Approximate text dimensions (rough estimate)
        let char_width = 7.0;
        let text_width = self.text.len() as f32 * char_width;
        let text_height = 14.0;
        let tooltip_width = text_width + self.padding * 2.0;
        let tooltip_height = text_height + self.padding * 2.0;

        let (x, y) = match self.position {
            TooltipPosition::Top => (
                content_bounds.x + (content_bounds.width - tooltip_width) / 2.0,
                content_bounds.y - tooltip_height - 4.0,
            ),
            TooltipPosition::Bottom => (
                content_bounds.x + (content_bounds.width - tooltip_width) / 2.0,
                content_bounds.y + content_bounds.height + 4.0,
            ),
            TooltipPosition::Left => (
                content_bounds.x - tooltip_width - 4.0,
                content_bounds.y + (content_bounds.height - tooltip_height) / 2.0,
            ),
            TooltipPosition::Right => (
                content_bounds.x + content_bounds.width + 4.0,
                content_bounds.y + (content_bounds.height - tooltip_height) / 2.0,
            ),
        };

        Rectangle::new(x.max(0.0), y.max(0.0), tooltip_width, tooltip_height)
    }

    /// Draw the tooltip overlay.
    fn draw_tooltip(&self, renderer: &mut Renderer, content_bounds: &Rectangle) {
        let tooltip_rect = self.tooltip_bounds(content_bounds);

        renderer.begin_overlay();

        // Draw background
        renderer.fill_rect(tooltip_rect, self.bg_color);

        // Draw border
        renderer.stroke_rect(tooltip_rect, Color::rgb(0.3, 0.3, 0.3), 1.0);

        // Draw text
        let text_pos = Point::new(
            tooltip_rect.x + self.padding,
            tooltip_rect.y + self.padding,
        );
        renderer.draw_text(&self.text, text_pos, self.text_color, 12.0);

        renderer.end_overlay();
    }
}

impl<'a, Message> Widget<Message> for Tooltip<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Layout is determined by the content
        self.content.widget().layout(limits)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        // Draw the content
        self.content.widget().draw(renderer, layout);

        // Draw tooltip if show flag is set
        if self.show {
            self.draw_tooltip(renderer, &layout.bounds());
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();

        // Check hover state and emit message when hovered
        // The flex layout ensures all siblings receive MouseMoved events
        if let Event::MouseMoved { position } = event {
            let is_hovered = bounds.contains(*position);

            // Forward to content first
            let content_result = self.content.widget_mut().on_event(event, layout);

            if let Some(ref on_hover_change) = self.on_hover_change {
                if is_hovered {
                    // Mouse is over this tooltip - emit hover(true)
                    return Some(on_hover_change(true));
                } else if self.is_active {
                    // This was the active tooltip but mouse moved away - emit hover(false)
                    return Some(on_hover_change(false));
                }
            }

            return content_result;
        }

        // For non-MouseMoved events, just forward to content
        self.content.widget_mut().on_event(event, layout)
    }
}

/// Create a tooltip wrapper around content.
pub fn tooltip<'a, Message>(
    content: Element<'a, Message>,
    text: impl Into<String>,
) -> Tooltip<'a, Message> {
    Tooltip::new(content, text)
}
