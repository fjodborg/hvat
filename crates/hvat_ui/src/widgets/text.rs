//! Text widget

use crate::event::Event;
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer};
use crate::widget::Widget;

/// Default font size in pixels
const DEFAULT_FONT_SIZE: f32 = 14.0;

/// Approximate character width ratio for monospace-ish text
const CHAR_WIDTH_RATIO: f32 = 0.6;

/// A text display widget
pub struct Text {
    content: String,
    size: f32,
    color: Color,
    width: Length,
}

impl Text {
    /// Create a new text widget
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: DEFAULT_FONT_SIZE,
            color: Color::TEXT_PRIMARY,
            width: Length::Shrink,
        }
    }

    /// Set the font size
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Set the text color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Calculate approximate text dimensions
    fn measure(&self) -> Size {
        // Simple approximation - real implementation would use text metrics
        let char_width = self.size * CHAR_WIDTH_RATIO;
        let width = self.content.len() as f32 * char_width;
        let height = self.size * 1.2; // Line height
        Size::new(width, height)
    }
}

impl<M> Widget<M> for Text {
    fn layout(&mut self, available: Size) -> Size {
        let content_size = self.measure();
        Size::new(
            self.width.resolve(available.width, content_size.width),
            content_size.height,
        )
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        log::debug!("Text draw: '{}' at {:?}", self.content, bounds);

        // Draw a subtle background so text bounds are visible
        renderer.fill_rect(bounds, Color::rgba(0.2, 0.2, 0.25, 0.8));

        // Draw text (placeholder - actual text rendering TBD)
        // For now, just draw a horizontal line to indicate where text would be
        let text_y = bounds.y + bounds.height / 2.0;
        renderer.line(
            bounds.x + 2.0,
            text_y,
            bounds.x + bounds.width - 2.0,
            text_y,
            self.color,
            1.0
        );

        // Call renderer.text for the log message (it logs what text would be drawn)
        renderer.text(&self.content, bounds.x, bounds.y, self.size, self.color);
    }

    fn on_event(&mut self, _event: &Event, _bounds: Bounds) -> Option<M> {
        None // Text doesn't handle events
    }
}
