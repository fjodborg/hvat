use crate::{Color, Element, Event, Layout, Limits, Rectangle, Renderer, Widget};

/// A container widget that wraps a single child with optional padding and background color.
pub struct Container<'a, Message> {
    child: Element<'a, Message>,
    padding: f32,
    background: Option<Color>,
    border_color: Option<Color>,
    border_width: f32,
}

impl<'a, Message> Container<'a, Message> {
    /// Create a new container with a child element.
    pub fn new(child: Element<'a, Message>) -> Self {
        Self {
            child,
            padding: 0.0,
            background: None,
            border_color: None,
            border_width: 1.0,
        }
    }

    /// Set the padding.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set the background color.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Set the border color (enables border).
    pub fn border(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Set the border width (default: 1.0).
    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width;
        self
    }
}

impl<'a, Message> Widget<Message> for Container<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Reduce limits by padding
        let child_limits = Limits::with_range(
            limits.min_width - self.padding * 2.0,
            limits.max_width - self.padding * 2.0,
            limits.min_height - self.padding * 2.0,
            limits.max_height - self.padding * 2.0,
        );

        let child_layout = self.child.widget().layout(&child_limits);
        let child_size = child_layout.size();

        // Container size is child size + padding
        let bounds = Rectangle::new(
            0.0,
            0.0,
            child_size.width + self.padding * 2.0,
            child_size.height + self.padding * 2.0,
        );

        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        // Draw background if specified
        if let Some(color) = self.background {
            renderer.fill_rect(bounds, color);
        }

        // Draw border if specified
        if let Some(color) = self.border_color {
            renderer.stroke_rect(bounds, color, self.border_width);
        }

        // Draw child with offset for padding
        let child_bounds = Rectangle::new(
            bounds.x + self.padding,
            bounds.y + self.padding,
            bounds.width - self.padding * 2.0,
            bounds.height - self.padding * 2.0,
        );
        let child_layout = Layout::new(child_bounds);
        self.child.widget().draw(renderer, &child_layout);
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let child_bounds = Rectangle::new(
            bounds.x + self.padding,
            bounds.y + self.padding,
            bounds.width - self.padding * 2.0,
            bounds.height - self.padding * 2.0,
        );
        let child_layout = Layout::new(child_bounds);
        self.child.widget_mut().on_event(event, &child_layout)
    }
}

/// Helper function to create a container.
pub fn container<'a, Message>(child: Element<'a, Message>) -> Container<'a, Message> {
    Container::new(child)
}
