//! Text widget

use crate::constants::{char_width, line_height, DEFAULT_FONT_SIZE};
use crate::event::Event;
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer};
use crate::widget::Widget;

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
        // Simple approximation using centralized constants
        let width = self.content.len() as f32 * char_width(self.size);
        let height = line_height(self.size);
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
        log::trace!("Text draw: '{}' at {:?}", self.content, bounds);

        // Draw text using glyphon
        renderer.text(&self.content, bounds.x, bounds.y, self.size, self.color);
    }

    fn on_event(&mut self, _event: &Event, _bounds: Bounds) -> Option<M> {
        None // Text doesn't handle events
    }
}
