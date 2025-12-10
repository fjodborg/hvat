//! Button widget

use crate::event::{Event, MouseButton};
use crate::layout::{Bounds, Length, Padding, Size};
use crate::renderer::{Color, Renderer};
use crate::widget::Widget;

/// Default button padding
const DEFAULT_PADDING: Padding = Padding {
    top: 8.0,
    right: 16.0,
    bottom: 8.0,
    left: 16.0,
};

/// Default font size
const DEFAULT_FONT_SIZE: f32 = 14.0;

/// Button state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ButtonState {
    #[default]
    Normal,
    Hovered,
    Pressed,
}

/// A clickable button widget
pub struct Button<M> {
    label: String,
    on_click: Option<M>,
    width: Length,
    height: Length,
    padding: Padding,
    state: ButtonState,
}

impl<M> Button<M> {
    /// Create a new button with the given label
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: DEFAULT_PADDING,
            state: ButtonState::Normal,
        }
    }

    /// Set the click handler
    pub fn on_click(mut self, message: M) -> Self {
        self.on_click = Some(message);
        self
    }

    /// Set the width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Set the height
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Set the padding
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Calculate content size
    fn content_size(&self) -> Size {
        // Approximate text size
        let char_width = DEFAULT_FONT_SIZE * 0.6;
        let text_width = self.label.len() as f32 * char_width;
        let text_height = DEFAULT_FONT_SIZE * 1.2;
        Size::new(text_width, text_height)
    }

    /// Get background color based on state
    fn background_color(&self) -> Color {
        match self.state {
            ButtonState::Normal => Color::BUTTON_BG,
            ButtonState::Hovered => Color::BUTTON_HOVER,
            ButtonState::Pressed => Color::BUTTON_ACTIVE,
        }
    }
}

impl<M: Clone + 'static> Widget<M> for Button<M> {
    fn layout(&mut self, available: Size) -> Size {
        let content = self.content_size();
        let min_width = content.width + self.padding.horizontal();
        let min_height = content.height + self.padding.vertical();

        Size::new(
            self.width.resolve(available.width, min_width),
            self.height.resolve(available.height, min_height),
        )
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        // Draw background
        renderer.fill_rect(bounds, self.background_color());

        // Draw border
        renderer.stroke_rect(bounds, Color::BORDER, 1.0);

        // Draw centered label
        let content = self.content_size();
        let text_x = bounds.x + (bounds.width - content.width) / 2.0;
        // Center vertically, accounting for text baseline
        let text_y = bounds.y + (bounds.height - content.height) / 2.0;

        renderer.text(&self.label, text_x, text_y, DEFAULT_FONT_SIZE, Color::TEXT_PRIMARY);
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        match event {
            Event::MouseMove { position, .. } => {
                let inside = bounds.contains(position.0, position.1);
                if inside && self.state != ButtonState::Pressed {
                    self.state = ButtonState::Hovered;
                } else if !inside && self.state == ButtonState::Hovered {
                    self.state = ButtonState::Normal;
                }
                None
            }

            Event::MousePress {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if bounds.contains(position.0, position.1) {
                    self.state = ButtonState::Pressed;
                }
                None
            }

            Event::MouseRelease {
                button: MouseButton::Left,
                position,
                ..
            } => {
                let was_pressed = self.state == ButtonState::Pressed;
                let inside = bounds.contains(position.0, position.1);

                self.state = if inside {
                    ButtonState::Hovered
                } else {
                    ButtonState::Normal
                };

                if was_pressed && inside {
                    self.on_click.clone()
                } else {
                    None
                }
            }

            _ => None,
        }
    }
}
