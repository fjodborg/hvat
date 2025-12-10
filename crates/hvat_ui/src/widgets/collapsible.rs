//! Collapsible/expandable section widget

use crate::element::Element;
use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer};
use crate::state::CollapsibleState;
use crate::widget::Widget;
use crate::Context;

// Layout constants
const HEADER_HEIGHT: f32 = 32.0;
const HEADER_PADDING_X: f32 = 8.0;
const ICON_SIZE: f32 = 12.0;
const ICON_MARGIN_RIGHT: f32 = 8.0;

/// Configuration for collapsible widget appearance
#[derive(Debug, Clone)]
pub struct CollapsibleConfig {
    /// Header background color
    pub header_bg: Color,
    /// Header hover color
    pub header_hover: Color,
    /// Header text color
    pub header_text_color: Color,
    /// Content background color
    pub content_bg: Color,
    /// Border color
    pub border_color: Color,
    /// Header font size
    pub header_font_size: f32,
    /// Header height
    pub header_height: f32,
}

impl Default for CollapsibleConfig {
    fn default() -> Self {
        Self {
            header_bg: Color::rgba(0.15, 0.15, 0.18, 1.0),
            header_hover: Color::rgba(0.2, 0.2, 0.24, 1.0),
            header_text_color: Color::TEXT_PRIMARY,
            content_bg: Color::rgba(0.12, 0.12, 0.14, 1.0),
            border_color: Color::BORDER,
            header_font_size: 14.0,
            header_height: HEADER_HEIGHT,
        }
    }
}

/// A collapsible/expandable section widget
///
/// Features:
/// - Click header to toggle expanded/collapsed state
/// - Chevron icon that rotates based on state
/// - Custom header content support
pub struct Collapsible<M> {
    /// Internal state (cloned from external)
    state: CollapsibleState,
    /// Header title text
    header_text: String,
    /// Content element (built via closure)
    content: Option<Element<M>>,
    /// Width constraint
    width: Length,
    /// Configuration
    config: CollapsibleConfig,
    /// Callback when toggled
    on_toggle: Option<Box<dyn Fn(CollapsibleState) -> M>>,
    /// Internal: cached header bounds
    header_bounds: Bounds,
    /// Internal: cached content size
    content_size: Size,
    /// Internal: is hovering over header
    hover_header: bool,
}

impl<M: 'static> Collapsible<M> {
    /// Create a new collapsible section
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            state: CollapsibleState::default(),
            header_text: header.into(),
            content: None,
            width: Length::Fill(1.0),
            config: CollapsibleConfig::default(),
            on_toggle: None,
            header_bounds: Bounds::ZERO,
            content_size: Size::ZERO,
            hover_header: false,
        }
    }

    /// Set the collapsible state (clones from external state)
    pub fn state(mut self, state: &CollapsibleState) -> Self {
        self.state = state.clone();
        self
    }

    /// Set the content using a builder function
    pub fn content<F>(mut self, builder: F) -> Self
    where
        F: FnOnce(&mut Context<M>),
    {
        use crate::widgets::Column;
        let mut ctx = Context::new();
        builder(&mut ctx);
        self.content = Some(Element::new(Column::new(ctx.take())));
        self
    }

    /// Set the width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Set configuration
    pub fn config(mut self, config: CollapsibleConfig) -> Self {
        self.config = config;
        self
    }

    /// Set header background color
    pub fn header_color(mut self, color: Color) -> Self {
        self.config.header_bg = color;
        self
    }

    /// Set callback for toggle events
    pub fn on_toggle<F>(mut self, callback: F) -> Self
    where
        F: Fn(CollapsibleState) -> M + 'static,
    {
        self.on_toggle = Some(Box::new(callback));
        self
    }

    /// Emit a state change if handler is set
    fn emit_change(&self) -> Option<M> {
        self.on_toggle.as_ref().map(|f| f(self.state.clone()))
    }

    /// Calculate visible content height (full height if expanded, 0 if collapsed)
    fn visible_content_height(&self) -> f32 {
        if self.state.is_expanded {
            self.content_size.height
        } else {
            0.0
        }
    }
}

impl<M: 'static> Default for Collapsible<M> {
    fn default() -> Self {
        Self::new("Section")
    }
}

impl<M: 'static> Widget<M> for Collapsible<M> {
    fn layout(&mut self, available: Size) -> Size {
        let width = self.width.resolve(available.width, available.width);

        // Header is always visible
        self.header_bounds = Bounds::new(0.0, 0.0, width, self.config.header_height);

        // Layout content if present
        if let Some(content) = &mut self.content {
            let content_available = Size::new(width, available.height - self.config.header_height);
            self.content_size = content.layout(content_available);
        } else {
            self.content_size = Size::ZERO;
        }

        // Total height = header + visible content
        let visible_height = self.visible_content_height();
        let total_height = self.config.header_height + visible_height;

        Size::new(width, total_height)
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        log::debug!(
            "Collapsible draw: bounds={:?}, is_expanded={}",
            bounds,
            self.state.is_expanded,
        );

        // Draw header
        let header_bounds = Bounds::new(
            bounds.x,
            bounds.y,
            self.header_bounds.width,
            self.config.header_height,
        );

        let header_bg = if self.hover_header {
            self.config.header_hover
        } else {
            self.config.header_bg
        };

        renderer.fill_rect(header_bounds, header_bg);
        renderer.stroke_rect(header_bounds, self.config.border_color, 1.0);

        // Draw chevron icon
        let icon = if self.state.is_expanded { "▼" } else { "▶" };
        let icon_x = header_bounds.x + HEADER_PADDING_X;
        let icon_y = header_bounds.y + (self.config.header_height - ICON_SIZE) / 2.0;
        renderer.text(
            icon,
            icon_x,
            icon_y,
            ICON_SIZE,
            self.config.header_text_color,
        );

        // Draw header text
        let text_x = icon_x + ICON_SIZE + ICON_MARGIN_RIGHT;
        let text_y = header_bounds.y + (self.config.header_height - self.config.header_font_size) / 2.0;
        renderer.text(
            &self.header_text,
            text_x,
            text_y,
            self.config.header_font_size,
            self.config.header_text_color,
        );

        // Draw content if expanded
        if self.state.is_expanded {
            if let Some(content) = &self.content {
                let content_bounds = Bounds::new(
                    bounds.x,
                    header_bounds.bottom(),
                    bounds.width,
                    self.content_size.height,
                );

                // Draw content background
                renderer.fill_rect(content_bounds, self.config.content_bg);

                // Draw content
                content.draw(renderer, content_bounds);

                // Draw bottom border
                renderer.stroke_rect(content_bounds, self.config.border_color, 1.0);
            }
        }
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        let header_bounds = Bounds::new(
            bounds.x,
            bounds.y,
            self.header_bounds.width,
            self.config.header_height,
        );

        match event {
            Event::MousePress {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if header_bounds.contains(position.0, position.1) {
                    log::debug!("Collapsible header clicked - toggling");
                    self.state.toggle();
                    return self.emit_change();
                }

                // Forward to content if expanded
                if self.state.is_expanded {
                    if let Some(content) = &mut self.content {
                        let content_bounds = Bounds::new(
                            bounds.x,
                            header_bounds.bottom(),
                            self.content_size.width,
                            self.content_size.height,
                        );
                        if content_bounds.contains(position.0, position.1) {
                            return content.on_event(event, content_bounds);
                        }
                    }
                }
            }

            Event::MouseMove { position, .. } => {
                self.hover_header = header_bounds.contains(position.0, position.1);

                // Forward to content if expanded
                if self.state.is_expanded {
                    if let Some(content) = &mut self.content {
                        let content_bounds = Bounds::new(
                            bounds.x,
                            header_bounds.bottom(),
                            self.content_size.width,
                            self.content_size.height,
                        );
                        return content.on_event(event, content_bounds);
                    }
                }
            }

            Event::MouseRelease { .. } => {
                // Forward to content if expanded
                if self.state.is_expanded {
                    if let Some(content) = &mut self.content {
                        let content_bounds = Bounds::new(
                            bounds.x,
                            header_bounds.bottom(),
                            self.content_size.width,
                            self.content_size.height,
                        );
                        return content.on_event(event, content_bounds);
                    }
                }
            }

            Event::KeyPress { key, .. } => {
                // Toggle on Enter/Space when hovering over header
                if self.hover_header {
                    match key {
                        KeyCode::Enter | KeyCode::Space => {
                            self.state.toggle();
                            return self.emit_change();
                        }
                        _ => {}
                    }
                }

                // Forward to content if expanded
                if self.state.is_expanded {
                    if let Some(content) = &mut self.content {
                        let content_bounds = Bounds::new(
                            bounds.x,
                            header_bounds.bottom(),
                            self.content_size.width,
                            self.content_size.height,
                        );
                        return content.on_event(event, content_bounds);
                    }
                }
            }

            _ => {
                // Forward other events to content if expanded
                if self.state.is_expanded {
                    if let Some(content) = &mut self.content {
                        let content_bounds = Bounds::new(
                            bounds.x,
                            header_bounds.bottom(),
                            self.content_size.width,
                            self.content_size.height,
                        );
                        return content.on_event(event, content_bounds);
                    }
                }
            }
        }

        None
    }
}
