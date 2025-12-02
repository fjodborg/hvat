use crate::{Color, Event, Layout, Limits, MouseButton, Point, Rectangle, Renderer, Widget};

/// A button widget that can be clicked.
pub struct Button<Message> {
    label: String,
    on_press: Option<Message>,
    width: Option<f32>,
    height: Option<f32>,
    is_hovered: bool,
}

impl<Message: Clone> Button<Message> {
    /// Create a new button with a label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_press: None,
            width: None,
            height: None,
            is_hovered: false,
        }
    }

    /// Set the message to emit when the button is pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Set the button width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the button height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
}

impl<Message: Clone> Widget<Message> for Button<Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Default button dimensions
        let default_width = 120.0;
        let default_height = 40.0;

        let width = self.width.unwrap_or(default_width).max(limits.min_width).min(limits.max_width);
        let height = self.height.unwrap_or(default_height).max(limits.min_height).min(limits.max_height);

        let bounds = Rectangle::new(0.0, 0.0, width, height);
        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        // Choose button color based on state
        let button_color = if self.is_hovered {
            Color::rgb(0.3, 0.4, 0.6) // Lighter blue when hovered
        } else {
            Color::rgb(0.2, 0.3, 0.5) // Normal blue
        };

        // Draw button background
        renderer.fill_rect(bounds, button_color);

        // Draw button border
        renderer.stroke_rect(bounds, Color::WHITE, 1.0);

        // Draw button text (centered)
        let text_position = Point::new(
            bounds.x + bounds.width / 2.0 - (self.label.len() as f32 * 5.0),
            bounds.y + bounds.height / 2.0 - 8.0,
        );
        renderer.draw_text(&self.label, text_position, Color::WHITE, 16.0);
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();

        match event {
            Event::MouseMoved { position } => {
                self.is_hovered = bounds.contains(*position);
                None
            }
            Event::MousePressed { button: MouseButton::Left, position } => {
                if bounds.contains(*position) && self.on_press.is_some() {
                    self.on_press.clone()
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Helper function to create a button.
pub fn button<Message: Clone>(label: impl Into<String>) -> Button<Message> {
    Button::new(label)
}
