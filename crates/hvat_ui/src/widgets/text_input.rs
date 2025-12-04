//! Text input widget for single-line text entry.

use crate::{Color, Event, Key, Layout, Limits, Modifiers, MouseButton, Point, Rectangle, Renderer, Widget};

/// A single-line text input widget.
pub struct TextInput<Message> {
    value: String,
    placeholder: String,
    width: Option<f32>,
    height: Option<f32>,
    is_focused: bool,
    on_change: Option<Box<dyn Fn(String) -> Message>>,
    on_submit: Option<Box<dyn Fn(String) -> Message>>,
    on_focus: Option<Box<dyn Fn(bool) -> Message>>,
}

impl<Message> TextInput<Message> {
    /// Create a new text input with the given value.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            placeholder: String::new(),
            width: None,
            height: None,
            is_focused: false,
            on_change: None,
            on_submit: None,
            on_focus: None,
        }
    }

    /// Set the placeholder text shown when empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the width of the input.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the height of the input.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set whether the input is focused.
    pub fn focused(mut self, focused: bool) -> Self {
        self.is_focused = focused;
        self
    }

    /// Set the callback when text changes.
    pub fn on_change<F>(mut self, f: F) -> Self
    where
        F: Fn(String) -> Message + 'static,
    {
        self.on_change = Some(Box::new(f));
        self
    }

    /// Set the callback when Enter is pressed.
    pub fn on_submit<F>(mut self, f: F) -> Self
    where
        F: Fn(String) -> Message + 'static,
    {
        self.on_submit = Some(Box::new(f));
        self
    }

    /// Set the callback when focus changes.
    pub fn on_focus<F>(mut self, f: F) -> Self
    where
        F: Fn(bool) -> Message + 'static,
    {
        self.on_focus = Some(Box::new(f));
        self
    }
}

impl<Message: Clone> Widget<Message> for TextInput<Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let default_width = 150.0;
        let default_height = 30.0;

        let width = self
            .width
            .unwrap_or(default_width)
            .max(limits.min_width)
            .min(limits.max_width);
        let height = self
            .height
            .unwrap_or(default_height)
            .max(limits.min_height)
            .min(limits.max_height);

        let bounds = Rectangle::new(0.0, 0.0, width, height);
        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        // Background color - darker when focused
        let bg_color = if self.is_focused {
            Color::rgb(0.15, 0.18, 0.22)
        } else {
            Color::rgb(0.12, 0.14, 0.18)
        };

        // Border color - accent when focused
        let border_color = if self.is_focused {
            Color::rgb(0.3, 0.5, 0.8)
        } else {
            Color::rgb(0.3, 0.35, 0.4)
        };

        // Draw background
        renderer.fill_rect(bounds, bg_color);

        // Draw border
        renderer.stroke_rect(bounds, border_color, 1.0);

        // Draw text or placeholder
        let display_text = if self.value.is_empty() {
            &self.placeholder
        } else {
            &self.value
        };

        let text_color = if self.value.is_empty() {
            Color::rgb(0.5, 0.5, 0.5) // Placeholder color
        } else {
            Color::WHITE
        };

        let text_x = bounds.x + 8.0;
        let text_y = bounds.y + bounds.height / 2.0 - 7.0;
        renderer.draw_text(display_text, Point::new(text_x, text_y), text_color, 14.0);

        // Draw cursor when focused
        if self.is_focused {
            let cursor_x = text_x + (self.value.len() as f32 * 8.0);
            let cursor_y = bounds.y + 5.0;
            let cursor_height = bounds.height - 10.0;
            renderer.fill_rect(
                Rectangle::new(cursor_x, cursor_y, 2.0, cursor_height),
                Color::WHITE,
            );
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();

        match event {
            Event::MousePressed {
                button: MouseButton::Left,
                position,
            } => {
                let was_focused = self.is_focused;
                let clicked_inside = bounds.contains(*position);
                self.is_focused = clicked_inside;

                // Only send message when gaining focus (clicking inside)
                // When losing focus (clicking outside), don't consume the event
                // so other widgets (like buttons) can process it
                if clicked_inside && !was_focused {
                    if let Some(ref on_focus) = self.on_focus {
                        return Some(on_focus(true));
                    }
                    // Fallback: trigger change message to cause redraw
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(self.value.clone()));
                    }
                }
                // When clicking outside, just update internal state without consuming event
                // The external state will be updated via keyboard_disabled on next frame
                None
            }
            Event::KeyPressed { key, modifiers } if self.is_focused => {
                // Ignore key presses with Ctrl/Alt/Meta modifiers (let them pass through for hotkeys)
                if modifiers.ctrl || modifiers.alt || modifiers.meta {
                    return None;
                }

                match key {
                    Key::Char(c) => {
                        // Add character to value
                        self.value.push(*c);
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(self.value.clone()));
                        }
                    }
                    Key::Backspace => {
                        // Remove last character
                        self.value.pop();
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(self.value.clone()));
                        }
                    }
                    Key::Enter => {
                        // Submit
                        if let Some(ref on_submit) = self.on_submit {
                            return Some(on_submit(self.value.clone()));
                        }
                    }
                    Key::Escape => {
                        // Unfocus
                        self.is_focused = false;
                        if let Some(ref on_focus) = self.on_focus {
                            return Some(on_focus(false));
                        }
                        // Fallback: trigger change message
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(self.value.clone()));
                        }
                    }
                    _ => {}
                }
                None
            }
            _ => None,
        }
    }
}

/// Create a new text input widget.
pub fn text_input<Message>(value: impl Into<String>) -> TextInput<Message> {
    TextInput::new(value)
}
