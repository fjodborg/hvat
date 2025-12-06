use crate::{builder_option, Color, ConcreteSize, ConcreteSizeXY, Element, Event, Layout, Limits, MouseButton, Point, Rectangle, Renderer, Widget};
use crate::theme::colors;
use super::tooltip::{Tooltip, TooltipPosition};

/// A button widget that can be clicked.
pub struct Button<Message> {
    label: String,
    on_press: Option<Message>,
    width: Option<f32>,
    height: Option<f32>,
    bg_color: Option<Color>,
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
            bg_color: None,
            is_hovered: false,
        }
    }

    /// Set the message to emit when the button is pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    // Builder methods using macros
    builder_option!(width, f32);
    builder_option!(height, f32);
    builder_option!(bg_color, Color);

    /// Wrap this button in a tooltip.
    pub fn tooltip(self, text: impl Into<String>) -> Tooltip<'static, Message>
    where
        Message: 'static,
    {
        Tooltip::new(Element::new(self), text)
    }

    /// Wrap this button in a tooltip with custom position.
    pub fn tooltip_with_position(
        self,
        text: impl Into<String>,
        position: TooltipPosition,
    ) -> Tooltip<'static, Message>
    where
        Message: 'static,
    {
        Tooltip::new(Element::new(self), text).position(position)
    }
}

impl<Message: Clone> Widget<Message> for Button<Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let default_width = 120.0;
        let default_height = 40.0;

        let width = self.width.unwrap_or(default_width).max(limits.min_width).min(limits.max_width);
        let height = self.height.unwrap_or(default_height).max(limits.min_height).min(limits.max_height);

        Layout::new(Rectangle::new(0.0, 0.0, width, height))
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        // Choose button color based on state and custom bg_color
        let button_color = if let Some(custom_color) = self.bg_color {
            if self.is_hovered {
                // Lighten the custom color slightly when hovered
                Color::new(
                    (custom_color.r + 0.1).min(1.0),
                    (custom_color.g + 0.1).min(1.0),
                    (custom_color.b + 0.1).min(1.0),
                    custom_color.a,
                )
            } else {
                custom_color
            }
        } else if self.is_hovered {
            colors::BUTTON_HOVER
        } else {
            colors::BUTTON_NORMAL
        };

        renderer.fill_rect(bounds, button_color);
        renderer.stroke_rect(bounds, Color::WHITE, 1.0);

        // Draw button text (centered)
        // Use char count for proper Unicode support (e.g., "✓" is 1 char, not 3 bytes)
        let char_count = self.label.chars().count() as f32;
        let font_size = 16.0;
        let char_width = font_size * 0.6; // Approximate monospace char width
        let text_width = char_count * char_width;
        let text_position = Point::new(
            bounds.x + (bounds.width - text_width) / 2.0,
            bounds.y + (bounds.height - font_size) / 2.0,
        );
        renderer.draw_text(&self.label, text_position, Color::WHITE, font_size);
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

    fn natural_size(&self, _max_width: ConcreteSize) -> ConcreteSizeXY {
        ConcreteSizeXY::from_f32(
            self.width.unwrap_or(120.0),
            self.height.unwrap_or(40.0),
        )
    }

    fn minimum_size(&self) -> ConcreteSizeXY {
        ConcreteSizeXY::from_f32(
            self.width.unwrap_or(40.0),
            self.height.unwrap_or(24.0),
        )
    }
}

/// Helper function to create a button.
pub fn button<Message: Clone>(label: impl Into<String>) -> Button<Message> {
    Button::new(label)
}
