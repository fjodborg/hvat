//! Text widget

use crate::constants::{char_width, line_height, DEFAULT_FONT_SIZE};
use crate::event::Event;
use crate::layout::{Alignment, Bounds, Length, Size};
use crate::renderer::{Color, Renderer, TextAlign};
use crate::widget::{EventResult, Widget};

/// A text display widget
pub struct Text {
    content: String,
    size: f32,
    color: Color,
    width: Length,
    /// Horizontal text alignment
    text_align: Alignment,
    /// Whether text should wrap to next line when it exceeds available width
    wrap: bool,
    /// Cached layout width for wrapped text rendering
    layout_width: Option<f32>,
}

impl Text {
    /// Create a new text widget
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: DEFAULT_FONT_SIZE,
            color: Color::TEXT_PRIMARY,
            // Shrink to content by default (works well in rows)
            width: Length::Shrink,
            text_align: Alignment::Left,
            wrap: false,
            layout_width: None,
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

    /// Set horizontal text alignment
    pub fn text_align(mut self, align: Alignment) -> Self {
        self.text_align = align;
        self
    }

    /// Center the text horizontally
    ///
    /// This sets both the text alignment to center AND width to Fill,
    /// so that centering actually has visible effect.
    pub fn centered(mut self) -> Self {
        self.width = Length::Fill(1.0);
        self.text_align = Alignment::Center;
        self
    }

    /// Enable word wrapping
    ///
    /// When enabled, text will wrap to the next line if any word would be clipped.
    /// For this to work properly, set an explicit width or use `Length::Fill`.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Calculate approximate text dimensions (single line)
    fn measure(&self) -> Size {
        // Simple approximation using centralized constants
        let width = self.content.len() as f32 * char_width(self.size);
        let height = line_height(self.size);
        Size::new(width, height)
    }

    /// Estimate wrapped text height given a width constraint
    fn estimate_wrapped_height(&self, available_width: f32) -> f32 {
        let single_line = self.measure();
        if single_line.width <= available_width {
            // Text fits on one line
            return single_line.height;
        }

        // Estimate number of lines needed
        // Use character-based estimation (approximate)
        let chars_per_line = (available_width / char_width(self.size)).max(1.0);
        let total_chars = self.content.len() as f32;
        let estimated_lines = (total_chars / chars_per_line).ceil().max(1.0);

        estimated_lines * line_height(self.size)
    }
}

impl<M> Widget<M> for Text {
    fn layout(&mut self, available: Size) -> Size {
        let content_size = self.measure();
        let resolved_width = self.width.resolve(available.width, content_size.width);

        // Store layout width for wrapped text rendering
        if self.wrap {
            self.layout_width = Some(resolved_width);
        }

        let height = if self.wrap && resolved_width < content_size.width {
            // Estimate wrapped height
            self.estimate_wrapped_height(resolved_width)
        } else {
            content_size.height
        };

        Size::new(resolved_width, height)
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        // Convert alignment to TextAlign for renderer
        let text_align = match self.text_align {
            Alignment::Left => TextAlign::Left,
            Alignment::Center => TextAlign::Center,
            Alignment::Right => TextAlign::Right,
        };

        // Handle wrapped text - use glyphon's native alignment
        if self.wrap {
            if let Some(wrap_width) = self.layout_width {
                log::trace!(
                    "Text draw wrapped: '{}' at {:?}, wrap_width={}, align={:?}",
                    self.content,
                    bounds,
                    wrap_width,
                    text_align
                );
                renderer.text_wrapped(
                    &self.content,
                    bounds.x,
                    bounds.y,
                    self.size,
                    self.color,
                    wrap_width,
                    text_align,
                );
                return;
            }
        }

        // For non-wrapped text, manually calculate position based on alignment
        // Use the fast approximate measurement for all alignments
        // (previously centered text used expensive measure_text_width)
        let content_size = self.measure();
        let text_x = bounds.x + self.text_align.align(bounds.width, content_size.width);

        log::trace!(
            "Text draw: '{}' at {:?}, text_x={}",
            self.content,
            bounds,
            text_x
        );

        // Draw text using glyphon
        renderer.text(&self.content, text_x, bounds.y, self.size, self.color);
    }

    fn on_event(&mut self, _event: &Event, _bounds: Bounds) -> EventResult<M> {
        EventResult::None // Text doesn't handle events
    }
}
