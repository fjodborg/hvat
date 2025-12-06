//! Collapsible container widget with title bar and expand/collapse functionality.

use crate::{builder_field, builder_option, callback_setter, Color, ConcreteSize, ConcreteSizeXY, Element, Event, Layout, Limits, MouseButton, Point, Rectangle, Renderer, Widget};
use crate::theme::{colors, ui};

/// A collapsible container that shows/hides its content.
pub struct Collapsible<'a, Message> {
    title: String,
    child: Element<'a, Message>,
    header_action: Option<Element<'a, Message>>,
    is_collapsed: bool,
    is_hovered: bool,
    on_toggle: Option<Box<dyn Fn(bool) -> Message>>,
    header_color: Color,
    header_hover_color: Color,
    text_color: Color,
    content_bg: Option<Color>,
    header_height: f32,
    padding: f32,
}

impl<'a, Message> Collapsible<'a, Message> {
    /// Create a new collapsible container.
    pub fn new(title: impl Into<String>, child: Element<'a, Message>) -> Self {
        Self {
            title: title.into(),
            child,
            header_action: None,
            is_collapsed: false,
            is_hovered: false,
            on_toggle: None,
            header_color: colors::COLLAPSIBLE_HEADER,
            header_hover_color: colors::COLLAPSIBLE_HEADER_HOVER,
            text_color: Color::WHITE,
            content_bg: None,
            header_height: 24.0,
            padding: 4.0,
        }
    }

    /// Set an action element to display on the right side of the header.
    pub fn header_action(mut self, action: Element<'a, Message>) -> Self {
        self.header_action = Some(action);
        self
    }

    /// Set the collapsed state.
    pub fn collapsed(mut self, is_collapsed: bool) -> Self {
        self.is_collapsed = is_collapsed;
        self
    }

    // Callback setter using macro
    callback_setter!(on_toggle, bool);

    // Builder methods using macros
    builder_field!(header_color, Color);
    builder_field!(header_hover_color, Color);
    builder_field!(text_color, Color);
    builder_option!(content_bg, Color);
    builder_field!(header_height, f32);
    builder_field!(padding, f32);

    /// Get the arrow indicator for the current state.
    fn arrow(&self) -> &str {
        if self.is_collapsed { ui::ARROW_COLLAPSED } else { ui::ARROW_EXPANDED }
    }
}

impl<'a, Message> Widget<Message> for Collapsible<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let width = if limits.max_width.is_finite() { limits.max_width } else { 400.0 };

        if self.is_collapsed {
            Layout::new(Rectangle::new(0.0, 0.0, width, self.header_height))
        } else {
            let child_max_width = (width - self.padding * 2.0).max(0.0);
            let child_max_height = if limits.max_height.is_finite() {
                (limits.max_height - self.header_height - self.padding * 2.0).max(0.0)
            } else {
                f32::INFINITY
            };

            let child_limits = Limits::with_range(0.0, child_max_width, 0.0, child_max_height);
            let child_layout = self.child.widget().layout(&child_limits);
            let child_size = child_layout.size();

            let total_height = self.header_height + child_size.height + self.padding * 2.0;
            Layout::new(Rectangle::new(0.0, 0.0, width, total_height))
        }
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        if !bounds.x.is_finite() || !bounds.y.is_finite() {
            return;
        }

        // Draw header
        let header_rect = Rectangle::new(bounds.x, bounds.y, bounds.width, self.header_height);
        let header_bg = if self.is_hovered { self.header_hover_color } else { self.header_color };
        renderer.fill_rect(header_rect, header_bg);
        renderer.stroke_rect(header_rect, colors::BORDER, 1.0);

        // Draw arrow
        let arrow_x = bounds.x + 6.0;
        let arrow_y = bounds.y + (self.header_height - 12.0) / 2.0;
        renderer.draw_text(self.arrow(), Point::new(arrow_x, arrow_y), self.text_color, 10.0);

        // Draw title
        let title_x = bounds.x + 22.0;
        let title_y = bounds.y + (self.header_height - 12.0) / 2.0;
        renderer.draw_text(&self.title, Point::new(title_x, title_y), self.text_color, 12.0);

        // Draw header action if present
        if let Some(ref action) = self.header_action {
            let action_limits = Limits::with_range(0.0, 100.0, 0.0, self.header_height);
            let action_layout = action.widget().layout(&action_limits);
            let action_size = action_layout.size();

            let action_x = bounds.x + bounds.width - action_size.width - 4.0;
            let action_y = bounds.y + (self.header_height - action_size.height) / 2.0;

            let action_bounds = Rectangle::new(action_x, action_y, action_size.width, action_size.height);
            action.widget().draw(renderer, &Layout::new(action_bounds));
        }

        // Draw content if expanded
        if !self.is_collapsed {
            let content_y = bounds.y + self.header_height;

            if let Some(bg_color) = self.content_bg {
                let content_height = (bounds.height - self.header_height).max(0.0);
                let content_rect = Rectangle::new(bounds.x, content_y, bounds.width, content_height);
                renderer.fill_rect(content_rect, bg_color);
            }

            let child_x = bounds.x + self.padding;
            let child_y = content_y + self.padding;
            let child_max_width = (bounds.width - self.padding * 2.0).max(0.0);
            let child_limits = Limits::with_range(0.0, child_max_width, 0.0, f32::INFINITY);
            let actual_child_layout = self.child.widget().layout(&child_limits);
            let child_size = actual_child_layout.size();

            let child_bounds = Rectangle::new(child_x, child_y, child_size.width, child_size.height);
            self.child.widget().draw(renderer, &Layout::new(child_bounds));
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let header_rect = Rectangle::new(bounds.x, bounds.y, bounds.width, self.header_height);

        let get_action_layout = |bounds: &Rectangle, header_height: f32, action: &Element<'_, Message>| {
            let action_limits = Limits::with_range(0.0, 100.0, 0.0, header_height);
            let action_layout = action.widget().layout(&action_limits);
            let action_size = action_layout.size();
            let action_x = bounds.x + bounds.width - action_size.width - 4.0;
            let action_y = bounds.y + (header_height - action_size.height) / 2.0;
            Layout::new(Rectangle::new(action_x, action_y, action_size.width, action_size.height))
        };

        let get_child_layout = |bounds: &Rectangle, padding: f32, child: &Element<'_, Message>| {
            let content_y = bounds.y + 24.0;
            let child_x = bounds.x + padding;
            let child_y = content_y + padding;
            let child_max_width = (bounds.width - padding * 2.0).max(0.0);
            let child_limits = Limits::with_range(0.0, child_max_width, 0.0, f32::INFINITY);
            let actual_child_layout = child.widget().layout(&child_limits);
            let child_size = actual_child_layout.size();
            Layout::new(Rectangle::new(child_x, child_y, child_size.width, child_size.height))
        };

        match event {
            Event::MouseMoved { position } => {
                self.is_hovered = header_rect.contains(*position);

                if let Some(ref mut action) = self.header_action {
                    let action_layout = get_action_layout(&bounds, self.header_height, action);
                    if let Some(msg) = action.widget_mut().on_event(event, &action_layout) {
                        return Some(msg);
                    }
                }

                if !self.is_collapsed {
                    let child_layout = get_child_layout(&bounds, self.padding, &self.child);
                    return self.child.widget_mut().on_event(event, &child_layout);
                }
                None
            }
            Event::MousePressed { button: MouseButton::Left, position } => {
                if let Some(ref mut action) = self.header_action {
                    let action_layout = get_action_layout(&bounds, self.header_height, action);
                    if action_layout.bounds().contains(*position) {
                        return action.widget_mut().on_event(event, &action_layout);
                    }
                }

                if header_rect.contains(*position) {
                    self.is_collapsed = !self.is_collapsed;
                    return self.on_toggle.as_ref().map(|f| f(self.is_collapsed));
                }

                if !self.is_collapsed {
                    let child_layout = get_child_layout(&bounds, self.padding, &self.child);
                    return self.child.widget_mut().on_event(event, &child_layout);
                }
                None
            }
            _ => {
                if let Some(ref mut action) = self.header_action {
                    let action_layout = get_action_layout(&bounds, self.header_height, action);
                    if let Some(msg) = action.widget_mut().on_event(event, &action_layout) {
                        return Some(msg);
                    }
                }

                if !self.is_collapsed {
                    let child_layout = get_child_layout(&bounds, self.padding, &self.child);
                    return self.child.widget_mut().on_event(event, &child_layout);
                }
                None
            }
        }
    }

    fn natural_size(&self, max_width: ConcreteSize) -> ConcreteSizeXY {
        let width = max_width.get().min(400.0);

        if self.is_collapsed {
            ConcreteSizeXY::from_f32(width, self.header_height)
        } else {
            let child_max = ConcreteSize::new_unchecked((width - self.padding * 2.0).max(0.0));
            let child_size = self.child.widget().natural_size(child_max);
            ConcreteSizeXY::from_f32(width, self.header_height + child_size.height.get() + self.padding * 2.0)
        }
    }

    fn minimum_size(&self) -> ConcreteSizeXY {
        ConcreteSizeXY::from_f32(100.0, self.header_height)
    }
}

/// Helper function to create a collapsible container.
pub fn collapsible<'a, Message>(title: impl Into<String>, child: Element<'a, Message>) -> Collapsible<'a, Message> {
    Collapsible::new(title, child)
}
