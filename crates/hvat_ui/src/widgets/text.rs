use crate::{Color, Event, Layout, Limits, Point, Rectangle, Renderer, Widget, TextMetrics};

/// A text widget that displays a string.
pub struct Text {
    content: String,
    size: f32,
    color: Color,
}

impl Text {
    /// Create a new text widget.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: 16.0,
            color: Color::WHITE,
        }
    }

    /// Set the text size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Set the text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<Message> Widget<Message> for Text {
    fn layout(&self, limits: &Limits) -> Layout {
        // Use TextMetrics for proper measurement
        let metrics = TextMetrics::new(self.size);
        let (text_width, text_height) = metrics.measure(&self.content);

        let width = text_width.min(limits.max_width);
        let height = text_height.max(metrics.line_height()); // At least one line

        let size = limits.resolve(width, height);
        let bounds = Rectangle::new(0.0, 0.0, size.width, size.height);

        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();
        let position = Point::new(bounds.x, bounds.y);

        renderer.draw_text(&self.content, position, self.color, self.size);
    }

    fn on_event(&mut self, _event: &Event, _layout: &Layout) -> Option<Message> {
        None // Text doesn't handle events
    }
}

/// Helper function to create text.
pub fn text(content: impl Into<String>) -> Text {
    Text::new(content)
}
