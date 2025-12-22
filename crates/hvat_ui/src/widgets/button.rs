//! Button widget

use crate::constants::{char_width, line_height, BUTTON_PADDING, DEFAULT_FONT_SIZE};
use crate::event::{Event, MouseButton};
use crate::layout::{Alignment, Bounds, Length, Padding, Size};
use crate::renderer::{Color, Renderer};
use crate::widget::{EventResult, Widget};

/// Button visual style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonStyle {
    /// Standard button with background and border
    #[default]
    Normal,
    /// Text-only button (transparent background, no border, underline on hover)
    Text,
}

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
    margin: Padding,
    state: ButtonState,
    /// Horizontal text alignment
    text_align: Alignment,
    /// Font size for button label
    font_size: f32,
    /// Visual style of the button
    style: ButtonStyle,
}

impl<M> Button<M> {
    /// Create a new button with the given label
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: BUTTON_PADDING,
            margin: Padding::ZERO,
            state: ButtonState::Normal,
            text_align: Alignment::Center,
            font_size: DEFAULT_FONT_SIZE,
            style: ButtonStyle::default(),
        }
    }

    /// Set the button style
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
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

    /// Set the margin (space around the button)
    pub fn margin(mut self, margin: impl Into<Padding>) -> Self {
        self.margin = margin.into();
        self
    }

    /// Set horizontal text alignment
    pub fn text_align(mut self, align: Alignment) -> Self {
        self.text_align = align;
        self
    }

    /// Set the font size for the button label
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Calculate content size
    fn content_size(&self) -> Size {
        // Approximate text size using centralized constants
        let text_width = self.label.len() as f32 * char_width(self.font_size);
        let text_height = line_height(self.font_size);
        Size::new(text_width, text_height)
    }

    /// Get background color based on state and style
    fn background_color(&self) -> Option<Color> {
        match self.style {
            ButtonStyle::Normal => Some(match self.state {
                ButtonState::Normal => Color::BUTTON_BG,
                ButtonState::Hovered => Color::BUTTON_HOVER,
                ButtonState::Pressed => Color::BUTTON_ACTIVE,
            }),
            ButtonStyle::Text => {
                // Text buttons have subtle hover/press feedback
                match self.state {
                    ButtonState::Normal => None,
                    ButtonState::Hovered => Some(Color::rgba(1.0, 1.0, 1.0, 0.05)),
                    ButtonState::Pressed => Some(Color::rgba(1.0, 1.0, 1.0, 0.1)),
                }
            }
        }
    }

    /// Get text color based on state and style
    fn text_color(&self) -> Color {
        match self.style {
            ButtonStyle::Normal => Color::TEXT_PRIMARY,
            ButtonStyle::Text => match self.state {
                ButtonState::Normal => Color::TEXT_SECONDARY,
                ButtonState::Hovered | ButtonState::Pressed => Color::TEXT_PRIMARY,
            },
        }
    }
}

impl<M: Clone + 'static> Widget<M> for Button<M> {
    fn layout(&mut self, available: Size) -> Size {
        let content = self.content_size();
        let min_width = content.width + self.padding.horizontal();
        let min_height = content.height + self.padding.vertical();

        // Account for margin in the resolved size
        let inner_width = self
            .width
            .resolve(available.width - self.margin.horizontal(), min_width);
        let inner_height = self
            .height
            .resolve(available.height - self.margin.vertical(), min_height);

        Size::new(
            inner_width + self.margin.horizontal(),
            inner_height + self.margin.vertical(),
        )
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        // Apply margin to get the actual button bounds
        let button_bounds = bounds.shrink(self.margin);

        // Draw background (if any)
        if let Some(bg_color) = self.background_color() {
            renderer.fill_rect(button_bounds, bg_color);
        }

        // Draw border (only for Normal style)
        if self.style == ButtonStyle::Normal {
            renderer.stroke_rect(button_bounds, Color::BORDER, 1.0);
        }

        // Calculate text position based on alignment
        let content = self.content_size();
        let inner_width = button_bounds.width - self.padding.horizontal();

        // Clamp text width to available inner space
        let text_width = content.width.min(inner_width);

        let text_x =
            button_bounds.x + self.padding.left + self.text_align.align(inner_width, text_width);

        // Center vertically using font size directly (not line_height)
        // Text rendering positions from top, and the visual center of most fonts
        // is slightly above the mathematical center due to descenders
        // Using font_size gives better visual centering than line_height
        let text_y = button_bounds.y + (button_bounds.height - self.font_size) / 2.0;

        renderer.text(
            &self.label,
            text_x,
            text_y,
            self.font_size,
            self.text_color(),
        );
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> EventResult<M> {
        // Apply margin to get the clickable button area
        let button_bounds = bounds.shrink(self.margin);

        match event {
            Event::MouseMove { position, .. } => {
                let inside = button_bounds.contains(position.0, position.1);
                let old_state = self.state;

                if inside && self.state != ButtonState::Pressed {
                    self.state = ButtonState::Hovered;
                } else if !inside && self.state == ButtonState::Hovered {
                    self.state = ButtonState::Normal;
                }

                // Return Redraw if state changed
                if self.state != old_state {
                    EventResult::Redraw
                } else {
                    EventResult::None
                }
            }

            Event::MousePress {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if button_bounds.contains(position.0, position.1) {
                    self.state = ButtonState::Pressed;
                    EventResult::Redraw
                } else {
                    EventResult::None
                }
            }

            Event::MouseRelease {
                button: MouseButton::Left,
                position,
                ..
            } => {
                let was_pressed = self.state == ButtonState::Pressed;
                let inside = button_bounds.contains(position.0, position.1);

                self.state = if inside {
                    ButtonState::Hovered
                } else {
                    ButtonState::Normal
                };

                // Fire click on release when button was pressed and mouse is still inside
                // This is standard button behavior that allows users to cancel by moving off
                if was_pressed && inside {
                    match self.on_click.clone() {
                        Some(msg) => EventResult::Message(msg),
                        None => EventResult::Redraw,
                    }
                } else if was_pressed {
                    // State changed from Pressed to something else
                    EventResult::Redraw
                } else {
                    EventResult::None
                }
            }

            _ => EventResult::None,
        }
    }
}
