//! Button widget

use crate::constants::{char_width, line_height, BUTTON_PADDING, DEFAULT_FONT_SIZE};
use crate::event::{Event, MouseButton};
use crate::layout::{Alignment, Bounds, Length, Padding, Size};
use crate::renderer::{Color, Renderer};
use crate::state::TooltipContent;
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
    /// Custom background color (overrides style-based colors)
    custom_bg: Option<Color>,
    /// Custom text color (overrides style-based colors)
    custom_text: Option<Color>,
    /// Optional tooltip content
    tooltip_content: Option<(String, TooltipContent)>,
    /// Callback to generate tooltip request message
    on_tooltip_request:
        Option<Box<dyn Fn(String, TooltipContent, Bounds, (f32, f32)) -> M + 'static>>,
    /// Callback to generate tooltip clear message
    on_tooltip_clear: Option<Box<dyn Fn(String) -> M + 'static>>,
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
            custom_bg: None,
            custom_text: None,
            tooltip_content: None,
            on_tooltip_request: None,
            on_tooltip_clear: None,
        }
    }

    /// Add a tooltip to this button
    ///
    /// The tooltip will be shown when hovering over the button.
    /// You must provide factory functions to create the hover and leave messages.
    ///
    /// # Example
    /// ```ignore
    /// button("Polygon")
    ///     .tooltip(
    ///         "tool_polygon",
    ///         TooltipContent::rich("Polygon Tool", "Hotkey: R\nDraw polygon annotations"),
    ///         |id, content, bounds, pos| Message::TooltipRequest(id, content, bounds, pos),
    ///         |id| Message::TooltipClear(id),
    ///     )
    /// ```
    pub fn tooltip(
        mut self,
        id: impl Into<String>,
        content: TooltipContent,
        on_hover: impl Fn(String, TooltipContent, Bounds, (f32, f32)) -> M + 'static,
        on_leave: impl Fn(String) -> M + 'static,
    ) -> Self {
        self.tooltip_content = Some((id.into(), content));
        self.on_tooltip_request = Some(Box::new(on_hover));
        self.on_tooltip_clear = Some(Box::new(on_leave));
        self
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

    /// Set a custom background color
    ///
    /// When set, this overrides the style-based background colors.
    /// The color will be slightly lightened on hover and darkened on press.
    /// Also automatically sets a contrasting text color for readability.
    pub fn background_color(mut self, color: Color) -> Self {
        // Automatically set contrasting text color for readability
        self.custom_text = Some(color.contrasting_text());
        self.custom_bg = Some(color);
        self
    }

    /// Set a custom text color
    ///
    /// When set, this overrides both the style-based colors and
    /// any auto-calculated contrasting color from background_color().
    pub fn text_color(mut self, color: Color) -> Self {
        self.custom_text = Some(color);
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
    fn get_background_color(&self) -> Option<Color> {
        // If custom background is set, use it with state variations
        if let Some(base) = self.custom_bg {
            return Some(match self.state {
                ButtonState::Normal => base,
                ButtonState::Hovered => base.lighten(0.15),
                ButtonState::Pressed => base.darken(0.15),
            });
        }

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
    fn get_text_color(&self) -> Color {
        // If custom text color is set, use it
        if let Some(color) = self.custom_text {
            return color;
        }

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
        if let Some(bg_color) = self.get_background_color() {
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
            self.get_text_color(),
        );
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> EventResult<M> {
        // Apply margin to get the clickable button area
        let button_bounds = bounds.shrink(self.margin);

        match event {
            Event::MouseMove { position, .. } => {
                let inside = button_bounds.contains(position.0, position.1);
                let old_state = self.state;
                let was_hovered = old_state == ButtonState::Hovered;

                if inside && self.state != ButtonState::Pressed {
                    self.state = ButtonState::Hovered;
                } else if !inside && self.state == ButtonState::Hovered {
                    self.state = ButtonState::Normal;
                }

                let is_hovered = self.state == ButtonState::Hovered;

                // Handle tooltip messages on hover state change
                // Use RedrawWithMessage to preserve button hover state (no view rebuild)
                if let (Some((ref id, ref content)), Some(ref on_request), Some(ref on_clear)) = (
                    &self.tooltip_content,
                    &self.on_tooltip_request,
                    &self.on_tooltip_clear,
                ) {
                    if !was_hovered && is_hovered {
                        // Just entered hover - emit tooltip request
                        let msg = on_request(id.clone(), content.clone(), button_bounds, *position);
                        return EventResult::RedrawWithMessage(msg);
                    } else if was_hovered && !is_hovered {
                        // Just left hover - emit tooltip clear
                        let msg = on_clear(id.clone());
                        return EventResult::RedrawWithMessage(msg);
                    } else if is_hovered {
                        // Still hovering - emit tooltip request with updated position
                        let msg = on_request(id.clone(), content.clone(), button_bounds, *position);
                        return EventResult::RedrawWithMessage(msg);
                    }
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
                    // Fire click on press (not release) to handle the case where
                    // a text input blur causes a view rebuild before MouseRelease.
                    // This is common in immediate-mode UIs where widget state
                    // doesn't persist between frames.
                    match self.on_click.clone() {
                        Some(msg) => EventResult::Message(msg),
                        None => EventResult::Redraw,
                    }
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

                // Just update visual state on release, click already fired on press
                if was_pressed {
                    EventResult::Redraw
                } else {
                    EventResult::None
                }
            }

            Event::CursorLeft => {
                // Cursor left the window - clear hover state and tooltip
                if self.state == ButtonState::Hovered {
                    self.state = ButtonState::Normal;
                    // Clear tooltip if we have one
                    if let (Some((ref id, _)), _, Some(ref on_clear)) = (
                        &self.tooltip_content,
                        &self.on_tooltip_request,
                        &self.on_tooltip_clear,
                    ) {
                        let msg = on_clear(id.clone());
                        return EventResult::Message(msg);
                    }
                    return EventResult::Redraw;
                }
                EventResult::None
            }

            _ => EventResult::None,
        }
    }
}
