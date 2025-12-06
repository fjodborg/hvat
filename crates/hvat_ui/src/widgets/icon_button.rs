//! Icon button widget with hover effects.

use crate::{
    Color, ConcreteSize, ConcreteSizeXY, Element, Event, ImageHandle, Layout, Limits, MouseButton, Rectangle, Renderer,
    Widget,
};
use super::tooltip::{Tooltip, TooltipPosition};

/// An icon button widget that displays an image and responds to clicks.
pub struct IconButton<Message> {
    /// The icon image to display
    icon: ImageHandle,
    /// Message to emit when clicked
    on_press: Option<Message>,
    /// Button size (width and height)
    size: f32,
    /// Padding around the icon
    padding: f32,
    /// Whether the button is currently hovered
    is_hovered: bool,
    /// Whether the button is in "active" state (e.g., selected tool)
    is_active: bool,
    /// Background color when active
    active_bg: Color,
    /// Background color when hovered (but not active)
    hover_bg: Color,
    /// Normal background color
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
            active_bg: Color::rgb(0.3, 0.5, 0.7),
            hover_bg: Color::rgb(0.25, 0.25, 0.3),
            normal_bg: Color::rgb(0.15, 0.15, 0.2),
        }
    }

    /// Set the message to emit when clicked.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Set the button size (both width and height).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Set the padding around the icon.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set whether the button is in active state.
    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Set the background color when active.
    pub fn active_bg(mut self, color: Color) -> Self {
        self.active_bg = color;
        self
    }

    /// Set the background color when hovered.
    pub fn hover_bg(mut self, color: Color) -> Self {
        self.hover_bg = color;
        self
    }

    /// Set the normal background color.
    pub fn normal_bg(mut self, color: Color) -> Self {
        self.normal_bg = color;
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
        let bounds = Rectangle::new(0.0, 0.0, size, size);
        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        // Determine background color based on state
        let bg_color = if self.is_active {
            self.active_bg
        } else if self.is_hovered {
            self.hover_bg
        } else {
            self.normal_bg
        };

        // Draw background
        renderer.fill_rect(bounds, bg_color);

        // Draw border when active
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
            Event::MousePressed {
                button: MouseButton::Left,
                position,
            } => {
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
        // Icon button has fixed minimum size
        ConcreteSizeXY::from_f32(self.size, self.size)
    }

    fn is_shrinkable(&self) -> bool {
        false // Icon buttons don't shrink
    }
}

/// Helper function to create an icon button.
pub fn icon_button<Message: Clone>(icon: ImageHandle) -> IconButton<Message> {
    IconButton::new(icon)
}
