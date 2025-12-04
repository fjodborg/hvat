//! Modal dialog widget for centered popup dialogs.
//!
//! The modal wraps underlying content and shows a dialog on top when visible.
//! It uses the overlay system to render the dialog above all other content.

use crate::{Color, Element, Event, Layout, Limits, Point, Rectangle, Renderer, Widget};

/// A modal dialog that wraps content and shows a dialog on top when visible.
pub struct Modal<'a, Message> {
    /// The underlying content (always rendered)
    underlying: Element<'a, Message>,
    /// The dialog content to show when visible
    dialog: Element<'a, Message>,
    /// Whether the modal dialog is visible
    is_visible: bool,
    /// Width of the modal dialog area
    width: f32,
    /// Height of the modal dialog area (None = auto)
    height: Option<f32>,
    /// Callback when backdrop is clicked (to close)
    on_backdrop_click: Option<Box<dyn Fn() -> Message + 'a>>,
    /// Backdrop color (semi-transparent)
    backdrop_color: Color,
    /// Modal background color
    background_color: Color,
    /// Modal border color
    border_color: Color,
}

impl<'a, Message> Modal<'a, Message> {
    /// Create a new modal wrapping underlying content with a dialog.
    pub fn new(underlying: Element<'a, Message>, dialog: Element<'a, Message>) -> Self {
        Self {
            underlying,
            dialog,
            is_visible: false,
            width: 300.0,
            height: None,
            on_backdrop_click: None,
            backdrop_color: Color::new(0.0, 0.0, 0.0, 0.5),
            background_color: Color::rgb(0.15, 0.18, 0.22),
            border_color: Color::rgb(0.3, 0.35, 0.4),
        }
    }

    /// Set whether the modal dialog is visible.
    pub fn visible(mut self, visible: bool) -> Self {
        self.is_visible = visible;
        self
    }

    /// Set the width of the modal dialog area.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Set the height of the modal dialog area.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set the callback when the backdrop is clicked.
    pub fn on_backdrop_click<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'a,
    {
        self.on_backdrop_click = Some(Box::new(f));
        self
    }

    /// Set the backdrop color.
    pub fn backdrop_color(mut self, color: Color) -> Self {
        self.backdrop_color = color;
        self
    }

    /// Set the modal background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Set the modal border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Calculate the modal dialog rect given viewport size.
    fn calc_dialog_rect(&self, viewport_width: f32, viewport_height: f32) -> Rectangle {
        // Calculate dialog content size
        let content_limits = Limits::new(
            self.width - 40.0,
            self.height.unwrap_or(viewport_height * 0.8) - 40.0,
        );
        let content_layout = self.dialog.widget().layout(&content_limits);
        let content_size = content_layout.size();

        // Use specified width and auto height from content
        let dialog_width = self.width;
        let dialog_height = self.height.unwrap_or(content_size.height + 40.0); // Add padding

        // Center the dialog
        let dialog_x = (viewport_width - dialog_width) / 2.0;
        let dialog_y = (viewport_height - dialog_height) / 2.0;

        Rectangle::new(dialog_x, dialog_y, dialog_width, dialog_height)
    }
}

impl<'a, Message: Clone + 'a> Widget<Message> for Modal<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Layout is determined by the underlying content
        self.underlying.widget().layout(limits)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        // Always draw the underlying content
        self.underlying.widget().draw(renderer, layout);

        // If visible, draw the modal dialog on top via overlay
        if self.is_visible {
            let viewport_width = renderer.viewport_width();
            let viewport_height = renderer.viewport_height();

            // Start overlay rendering
            renderer.begin_overlay();

            // Draw semi-transparent backdrop covering the entire viewport
            let backdrop_rect = Rectangle::new(0.0, 0.0, viewport_width, viewport_height);
            renderer.fill_rect(backdrop_rect, self.backdrop_color);

            // Calculate dialog rect
            let dialog_rect = self.calc_dialog_rect(viewport_width, viewport_height);

            // Draw dialog background with border
            let border_rect = Rectangle::new(
                dialog_rect.x - 2.0,
                dialog_rect.y - 2.0,
                dialog_rect.width + 4.0,
                dialog_rect.height + 4.0,
            );
            renderer.fill_rect(border_rect, self.border_color);
            renderer.fill_rect(dialog_rect, self.background_color);

            // Draw dialog content with padding
            let content_x = dialog_rect.x + 20.0;
            let content_y = dialog_rect.y + 20.0;
            let content_width = dialog_rect.width - 40.0;
            let content_height = dialog_rect.height - 40.0;

            let content_rect = Rectangle::new(content_x, content_y, content_width, content_height);
            let content_layout = Layout::new(content_rect);

            self.dialog.widget().draw(renderer, &content_layout);

            // End overlay rendering
            renderer.end_overlay();
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        // If modal is visible, handle events for the dialog first
        if self.is_visible {
            // Use the layout bounds to estimate viewport size
            // The underlying content fills the viewport, so its bounds give us the viewport size
            let bounds = layout.bounds();
            // Account for any offset - use bounds position as origin reference
            let viewport_width = bounds.x + bounds.width;
            let viewport_height = bounds.y + bounds.height;

            let dialog_rect = self.calc_dialog_rect(viewport_width, viewport_height);

            match event {
                Event::MousePressed { position, button } if *button == crate::MouseButton::Left => {
                    let pos = Point::new(position.x, position.y);

                    if dialog_rect.contains(pos) {
                        // Forward event to dialog content
                        let content_x = dialog_rect.x + 20.0;
                        let content_y = dialog_rect.y + 20.0;
                        let content_width = dialog_rect.width - 40.0;
                        let content_height = dialog_rect.height - 40.0;
                        let content_rect =
                            Rectangle::new(content_x, content_y, content_width, content_height);
                        let content_layout = Layout::new(content_rect);

                        if let Some(msg) =
                            self.dialog.widget_mut().on_event(event, &content_layout)
                        {
                            return Some(msg);
                        }
                        // Consume the event even if dialog doesn't handle it
                        return None;
                    } else {
                        // Click on backdrop - close modal
                        if let Some(ref on_click) = self.on_backdrop_click {
                            return Some(on_click());
                        }
                        return None;
                    }
                }
                Event::MouseReleased { position, .. } | Event::MouseMoved { position } => {
                    let pos = Point::new(position.x, position.y);

                    if dialog_rect.contains(pos) {
                        let content_x = dialog_rect.x + 20.0;
                        let content_y = dialog_rect.y + 20.0;
                        let content_width = dialog_rect.width - 40.0;
                        let content_height = dialog_rect.height - 40.0;
                        let content_rect =
                            Rectangle::new(content_x, content_y, content_width, content_height);
                        let content_layout = Layout::new(content_rect);

                        if let Some(msg) =
                            self.dialog.widget_mut().on_event(event, &content_layout)
                        {
                            return Some(msg);
                        }
                    }
                    // When modal is open, don't forward mouse events to underlying content
                    return None;
                }
                Event::KeyPressed { .. } | Event::KeyReleased { .. } => {
                    // Forward keyboard events to dialog
                    let content_x = dialog_rect.x + 20.0;
                    let content_y = dialog_rect.y + 20.0;
                    let content_width = dialog_rect.width - 40.0;
                    let content_height = dialog_rect.height - 40.0;
                    let content_rect =
                        Rectangle::new(content_x, content_y, content_width, content_height);
                    let content_layout = Layout::new(content_rect);

                    if let Some(msg) = self.dialog.widget_mut().on_event(event, &content_layout) {
                        return Some(msg);
                    }
                    return None;
                }
                _ => {
                    // Block other events when modal is open
                    return None;
                }
            }
        }

        // Modal not visible - forward events to underlying content
        self.underlying.widget_mut().on_event(event, layout)
    }
}

/// Create a new modal widget wrapping underlying content with a dialog.
pub fn modal<'a, Message>(
    underlying: Element<'a, Message>,
    dialog: Element<'a, Message>,
) -> Modal<'a, Message> {
    Modal::new(underlying, dialog)
}
