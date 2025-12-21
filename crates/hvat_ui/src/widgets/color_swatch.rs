//! Color swatch widget for displaying and selecting colors

use crate::constants::{line_height, DEFAULT_FONT_SIZE};
use crate::event::{Event, MouseButton};
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer};
use crate::widget::Widget;

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
        // Draw the color fill
        renderer.fill_rect(bounds, self.to_render_color());

        // Draw border (slightly darker when hovered)
        let border_color = if self.hovered {
            Color::rgb(0.8, 0.8, 0.85)
        } else {
            Color::BORDER
        };
        renderer.stroke_rect(bounds, border_color, 1.0);
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        match event {
            Event::MouseMove { position, .. } => {
                self.hovered = bounds.contains(position.0, position.1);
                None
            }
            Event::MouseRelease {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if bounds.contains(position.0, position.1) {
                    self.on_click.clone()
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
