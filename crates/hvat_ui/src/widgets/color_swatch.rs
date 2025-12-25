//! Color swatch widget for displaying and selecting colors

use crate::constants::{line_height, DEFAULT_FONT_SIZE};
use crate::event::{Event, MouseButton};
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer};
use crate::widget::{EventResult, Widget};

/// A color swatch widget that displays a color and optionally allows clicking to open a picker
pub struct ColorSwatch<M> {
    /// The color to display (RGB as [u8; 3])
    color: [u8; 3],
    /// Width of the swatch
    width: Length,
    /// Height of the swatch
    height: Length,
    /// Whether the swatch is being hovered
    hovered: bool,
    /// Whether to show filled color (true) or just border outline (false)
    filled: bool,
    /// Callback when clicked
    on_click: Option<M>,
}

impl<M> ColorSwatch<M> {
    /// Create a new color swatch with the given RGB color
    pub fn new(color: [u8; 3]) -> Self {
        Self {
            color,
            width: Length::Fixed(20.0),
            height: Length::Fixed(20.0),
            hovered: false,
            filled: true,
            on_click: None,
        }
    }

    /// Set the swatch color
    pub fn color(mut self, color: [u8; 3]) -> Self {
        self.color = color;
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

    /// Set the click handler
    pub fn on_click(mut self, message: M) -> Self {
        self.on_click = Some(message);
        self
    }

    /// Set whether the swatch shows filled color or just an outline
    ///
    /// When `filled` is true (default), the swatch displays the solid color.
    /// When `filled` is false, only the border is drawn in the swatch color.
    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }

    /// Convert RGB bytes to Color
    fn to_render_color(&self) -> Color {
        Color::rgb(
            self.color[0] as f32 / 255.0,
            self.color[1] as f32 / 255.0,
            self.color[2] as f32 / 255.0,
        )
    }
}

impl<M: Clone + 'static> Widget<M> for ColorSwatch<M> {
    fn layout(&mut self, available: Size) -> Size {
        let default_size = line_height(DEFAULT_FONT_SIZE);
        Size::new(
            self.width.resolve(available.width, default_size),
            self.height.resolve(available.height, default_size),
        )
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        let swatch_color = self.to_render_color();
        log::trace!(
            "ColorSwatch draw: filled={}, color={:?}, bounds={:?}",
            self.filled,
            self.color,
            bounds
        );

        if self.filled {
            // Draw the color fill
            renderer.fill_rect(bounds, swatch_color);

            // Draw border (slightly lighter when hovered)
            let border_color = if self.hovered {
                Color::rgb(0.8, 0.8, 0.85)
            } else {
                Color::BORDER
            };
            renderer.stroke_rect(bounds, border_color, 1.0);
        } else {
            // Outline mode: draw a visible border in the swatch color
            // Use a thicker stroke (3px) so it's clearly visible
            let border_color = if self.hovered {
                swatch_color.lighten(0.2)
            } else {
                swatch_color
            };
            renderer.stroke_rect(bounds, border_color, 3.0);
        }
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> EventResult<M> {
        match event {
            Event::MouseMove { position, .. } => {
                self.hovered = bounds.contains(position.0, position.1);
                EventResult::None
            }
            Event::MouseRelease {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if bounds.contains(position.0, position.1) {
                    self.on_click.clone().into()
                } else {
                    EventResult::None
                }
            }
            _ => EventResult::None,
        }
    }
}
