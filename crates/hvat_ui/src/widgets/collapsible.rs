//! Collapsible container widget with title bar and expand/collapse functionality.

use crate::{Color, Element, Event, Layout, Limits, MouseButton, Point, Rectangle, Renderer, Widget};

/// A collapsible container that shows/hides its content.
pub struct Collapsible<'a, Message> {
    /// Title shown in the header
    title: String,
    /// Child content (shown when expanded)
    child: Element<'a, Message>,
    /// Optional action element shown on the right side of the header
    header_action: Option<Element<'a, Message>>,
    /// Whether the container is collapsed
    is_collapsed: bool,
    /// Whether the header is hovered
    is_hovered: bool,
    /// Callback when collapse state changes
    on_toggle: Option<Box<dyn Fn(bool) -> Message>>,
    /// Header background color
    header_color: Color,
    /// Header hover color
    header_hover_color: Color,
    /// Text color
    text_color: Color,
    /// Content background color (optional)
    content_bg: Option<Color>,
    /// Header height
    header_height: f32,
    /// Padding for content
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
            header_color: Color::rgb(0.2, 0.25, 0.3),
            header_hover_color: Color::rgb(0.25, 0.3, 0.35),
            text_color: Color::WHITE,
            content_bg: None,
            header_height: 24.0,
            padding: 4.0,
        }
    }

    /// Set an action element to display on the right side of the header.
    /// This is typically used for buttons like reset/settings that should
    /// always be visible regardless of collapsed state.
    pub fn header_action(mut self, action: Element<'a, Message>) -> Self {
        self.header_action = Some(action);
        self
    }

    /// Set the collapsed state.
    pub fn collapsed(mut self, is_collapsed: bool) -> Self {
        self.is_collapsed = is_collapsed;
        self
    }

    /// Set the callback for when the container is toggled.
    pub fn on_toggle<F>(mut self, f: F) -> Self
    where
        F: Fn(bool) -> Message + 'static,
    {
        self.on_toggle = Some(Box::new(f));
        self
    }

    /// Set the header background color.
    pub fn header_color(mut self, color: Color) -> Self {
        self.header_color = color;
        self
    }

    /// Set the header hover color.
    pub fn header_hover_color(mut self, color: Color) -> Self {
        self.header_hover_color = color;
        self
    }

    /// Set the text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set the content background color.
    pub fn content_bg(mut self, color: Color) -> Self {
        self.content_bg = Some(color);
        self
    }

    /// Set the header height.
    pub fn header_height(mut self, height: f32) -> Self {
        self.header_height = height;
        self
    }

    /// Set the content padding.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Get the arrow indicator for the current state.
    fn arrow(&self) -> &str {
        if self.is_collapsed {
            "▶"
        } else {
            "▼"
        }
    }
}

impl<'a, Message> Widget<Message> for Collapsible<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Use a reasonable default width if max_width is infinite
        let width = if limits.max_width.is_finite() {
            limits.max_width
        } else {
            400.0 // Default width for unbounded layouts
        };

        if self.is_collapsed {
            // Only header
            Layout::new(Rectangle::new(0.0, 0.0, width, self.header_height))
        } else {
            // Header + content
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

        // Guard against invalid bounds
        if !bounds.x.is_finite() || !bounds.y.is_finite() {
            return;
        }

        // Draw header
        let header_rect = Rectangle::new(bounds.x, bounds.y, bounds.width, self.header_height);

        let header_bg = if self.is_hovered {
            self.header_hover_color
        } else {
            self.header_color
        };
        renderer.fill_rect(header_rect, header_bg);
        renderer.stroke_rect(header_rect, Color::rgb(0.3, 0.35, 0.4), 1.0);

        // Draw arrow
        let arrow_x = bounds.x + 6.0;
        let arrow_y = bounds.y + (self.header_height - 12.0) / 2.0;
        renderer.draw_text(self.arrow(), Point::new(arrow_x, arrow_y), self.text_color, 10.0);

        // Draw title
        let title_x = bounds.x + 22.0;
        let title_y = bounds.y + (self.header_height - 12.0) / 2.0;
        renderer.draw_text(&self.title, Point::new(title_x, title_y), self.text_color, 12.0);

        // Draw header action on the right side if present
        if let Some(ref action) = self.header_action {
            // Layout the action element to get its size
            let action_limits = Limits::with_range(0.0, 100.0, 0.0, self.header_height);
            let action_layout = action.widget().layout(&action_limits);
            let action_size = action_layout.size();

            // Position it on the right side of the header, centered vertically
            let action_x = bounds.x + bounds.width - action_size.width - 4.0;
            let action_y = bounds.y + (self.header_height - action_size.height) / 2.0;

            let action_bounds = Rectangle::new(action_x, action_y, action_size.width, action_size.height);
            let action_layout = Layout::new(action_bounds);
            action.widget().draw(renderer, &action_layout);
        }

        // Draw content if expanded
        if !self.is_collapsed {
            let content_y = bounds.y + self.header_height;

            // Draw content background if specified
            if let Some(bg_color) = self.content_bg {
                let content_height = (bounds.height - self.header_height).max(0.0);
                let content_rect = Rectangle::new(bounds.x, content_y, bounds.width, content_height);
                renderer.fill_rect(content_rect, bg_color);
            }

            // Calculate child layout and draw - use actual child layout size
            let child_x = bounds.x + self.padding;
            let child_y = content_y + self.padding;

            // Get the child's actual layout to determine its size
            let child_max_width = (bounds.width - self.padding * 2.0).max(0.0);
            let child_limits = Limits::with_range(0.0, child_max_width, 0.0, f32::INFINITY);
            let actual_child_layout = self.child.widget().layout(&child_limits);
            let child_size = actual_child_layout.size();

            let child_bounds = Rectangle::new(child_x, child_y, child_size.width, child_size.height);
            let child_layout = Layout::new(child_bounds);

            self.child.widget().draw(renderer, &child_layout);
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let header_rect = Rectangle::new(bounds.x, bounds.y, bounds.width, self.header_height);

        // Helper to get header action layout
        let get_action_layout = |bounds: &Rectangle, header_height: f32, action: &Element<'_, Message>| {
            let action_limits = Limits::with_range(0.0, 100.0, 0.0, header_height);
            let action_layout = action.widget().layout(&action_limits);
            let action_size = action_layout.size();
            let action_x = bounds.x + bounds.width - action_size.width - 4.0;
            let action_y = bounds.y + (header_height - action_size.height) / 2.0;
            Layout::new(Rectangle::new(action_x, action_y, action_size.width, action_size.height))
        };

        // Helper to get child layout
        let get_child_layout = |bounds: &Rectangle, padding: f32, child: &Element<'_, Message>| {
            let content_y = bounds.y + 24.0; // header_height
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

                // Forward to header action if present
                if let Some(ref mut action) = self.header_action {
                    let action_layout = get_action_layout(&bounds, self.header_height, action);
                    if let Some(msg) = action.widget_mut().on_event(event, &action_layout) {
                        return Some(msg);
                    }
                }

                // Forward to child if expanded
                if !self.is_collapsed {
                    let child_layout = get_child_layout(&bounds, self.padding, &self.child);
                    return self.child.widget_mut().on_event(event, &child_layout);
                }
                None
            }
            Event::MousePressed {
                button: MouseButton::Left,
                position,
            } => {
                // First check if clicking on header action (takes priority over toggle)
                if let Some(ref mut action) = self.header_action {
                    let action_layout = get_action_layout(&bounds, self.header_height, action);
                    if action_layout.bounds().contains(*position) {
                        return action.widget_mut().on_event(event, &action_layout);
                    }
                }

                // Check if clicking on header (but not on action)
                if header_rect.contains(*position) {
                    self.is_collapsed = !self.is_collapsed;
                    return self.on_toggle.as_ref().map(|f| f(self.is_collapsed));
                }

                // Forward to child if expanded and clicked in content area
                if !self.is_collapsed {
                    let child_layout = get_child_layout(&bounds, self.padding, &self.child);
                    return self.child.widget_mut().on_event(event, &child_layout);
                }
                None
            }
            _ => {
                // Forward to header action if present
                if let Some(ref mut action) = self.header_action {
                    let action_layout = get_action_layout(&bounds, self.header_height, action);
                    if let Some(msg) = action.widget_mut().on_event(event, &action_layout) {
                        return Some(msg);
                    }
                }

                // Forward other events to child if expanded
                if !self.is_collapsed {
                    let child_layout = get_child_layout(&bounds, self.padding, &self.child);
                    return self.child.widget_mut().on_event(event, &child_layout);
                }
                None
            }
        }
    }
}

/// Helper function to create a collapsible container.
pub fn collapsible<'a, Message>(
    title: impl Into<String>,
    child: Element<'a, Message>,
) -> Collapsible<'a, Message> {
    Collapsible::new(title, child)
}
