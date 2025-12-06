//! Icon button widget with hover effects.

use crate::{
    builder_field, Color, ConcreteSize, ConcreteSizeXY, Element, Event, ImageHandle, Layout, Limits, MouseButton, Rectangle, Renderer,
    Widget,
};
use crate::theme::colors;
use super::tooltip::{Tooltip, TooltipPosition};

/// An icon button widget that displays an image and responds to clicks.
pub struct IconButton<Message> {
    icon: ImageHandle,
    on_press: Option<Message>,
    size: f32,
    padding: f32,
    is_hovered: bool,
    is_active: bool,
    active_bg: Color,
    hover_bg: Color,
    normal_bg: Color,
}

impl<Message: Clone> IconButton<Message> {
    /// Create a new icon button with an icon image.
    pub fn new(icon: ImageHandle) -> Self {
        Self {
            icon,
            on_press: None,
            size: 32.0,
            padding: 4.0,
            is_hovered: false,
            is_active: false,
            active_bg: colors::ICON_BUTTON_ACTIVE,
            hover_bg: colors::ICON_BUTTON_HOVER,
            normal_bg: colors::ICON_BUTTON_NORMAL,
        }
    }

    /// Set the message to emit when clicked.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    // Builder methods using macros
    builder_field!(size, f32);
    builder_field!(padding, f32);
    builder_field!(active_bg, Color);
    builder_field!(hover_bg, Color);
    builder_field!(normal_bg, Color);

    /// Set whether the button is in active state.
    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Wrap this icon button in a tooltip.
    pub fn tooltip(self, text: impl Into<String>) -> Tooltip<'static, Message>
    where
        Message: 'static,
    {
        Tooltip::new(Element::new(self), text)
    }

    /// Wrap this icon button in a tooltip with custom position.
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

impl<Message: Clone> Widget<Message> for IconButton<Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let size = self.size.max(limits.min_width).min(limits.max_width);
        Layout::new(Rectangle::new(0.0, 0.0, size, size))
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        let bg_color = if self.is_active {
            self.active_bg
        } else if self.is_hovered {
            self.hover_bg
        } else {
            self.normal_bg
        };

        renderer.fill_rect(bounds, bg_color);

        if self.is_active {
            renderer.stroke_rect(bounds, Color::WHITE, 1.0);
        }

        // Draw icon centered with padding
        let icon_size = self.size - self.padding * 2.0;
        let icon_rect = Rectangle::new(
            bounds.x + self.padding,
            bounds.y + self.padding,
            icon_size,
            icon_size,
        );
        renderer.draw_image(&self.icon, icon_rect);
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
        ConcreteSizeXY::from_f32(self.size, self.size)
    }

    fn minimum_size(&self) -> ConcreteSizeXY {
        ConcreteSizeXY::from_f32(self.size, self.size)
    }

    fn is_shrinkable(&self) -> bool {
        false
    }
}

/// Helper function to create an icon button.
pub fn icon_button<Message: Clone>(icon: ImageHandle) -> IconButton<Message> {
    IconButton::new(icon)
}
