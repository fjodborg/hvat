//! Dropdown widget for selecting from a list of options.
//!
//! The dropdown renders its popup below the button when open.
//! It auto-closes when:
//! - An option is clicked
//! - The mouse leaves the dropdown area (button + popup)
//!
//! The popup can optionally render above other widgets using the overlay feature.

use crate::{Color, Event, Layout, Limits, MouseButton, Point, Rectangle, Renderer, Widget};

/// A dropdown widget for selecting from a list of options.
pub struct Dropdown<Message> {
    /// The options to display
    options: Vec<String>,
    /// Currently selected index
    selected: usize,
    /// Whether the dropdown is open (controlled externally)
    is_open: bool,
    /// Whether the button is hovered
    is_hovered: bool,
    /// Hovered option index in popup
    hovered_option: Option<usize>,
    /// Callback when selection changes
    on_select: Option<Box<dyn Fn(usize) -> Message>>,
    /// Callback when dropdown should close (mouse left or selection made)
    on_close: Option<Box<dyn Fn() -> Message>>,
    /// Callback when dropdown should open (button clicked)
    on_open: Option<Box<dyn Fn() -> Message>>,
    /// Width of the dropdown
    width: f32,
    /// Whether to render popup as overlay (above other content)
    render_as_overlay: bool,
    /// Colors
    button_color: Color,
    button_hover_color: Color,
    text_color: Color,
    popup_bg_color: Color,
    popup_hover_color: Color,
}

impl<Message> Dropdown<Message> {
    /// Create a new dropdown with options.
    pub fn new(options: Vec<String>, selected: usize) -> Self {
        Self {
            options,
            selected,
            is_open: false,
            is_hovered: false,
            hovered_option: None,
            on_select: None,
            on_close: None,
            on_open: None,
            width: 100.0,
            render_as_overlay: false,
            button_color: Color::rgb(0.2, 0.3, 0.4),
            button_hover_color: Color::rgb(0.3, 0.4, 0.5),
            text_color: Color::WHITE,
            popup_bg_color: Color::rgb(0.15, 0.2, 0.25),
            popup_hover_color: Color::rgb(0.25, 0.35, 0.45),
        }
    }

    /// Set the callback for when an option is selected.
    pub fn on_select<F>(mut self, f: F) -> Self
    where
        F: Fn(usize) -> Message + 'static,
    {
        self.on_select = Some(Box::new(f));
        self
    }

    /// Set the callback for when the dropdown should close.
    pub fn on_close<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'static,
    {
        self.on_close = Some(Box::new(f));
        self
    }

    /// Set the callback for when the dropdown should open.
    pub fn on_open<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'static,
    {
        self.on_open = Some(Box::new(f));
        self
    }

    /// Set the width of the dropdown.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Set the button color.
    pub fn button_color(mut self, color: Color) -> Self {
        self.button_color = color;
        self
    }

    /// Set the button hover color.
    pub fn button_hover_color(mut self, color: Color) -> Self {
        self.button_hover_color = color;
        self
    }

    /// Set the text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set the popup background color.
    pub fn popup_bg_color(mut self, color: Color) -> Self {
        self.popup_bg_color = color;
        self
    }

    /// Set the popup hover color.
    pub fn popup_hover_color(mut self, color: Color) -> Self {
        self.popup_hover_color = color;
        self
    }

    /// Set the open state (for external control).
    pub fn open(mut self, is_open: bool) -> Self {
        self.is_open = is_open;
        self
    }

    /// Enable overlay rendering for the popup.
    /// When enabled, the popup will render above other widgets (except scrollbars).
    pub fn overlay(mut self, render_as_overlay: bool) -> Self {
        self.render_as_overlay = render_as_overlay;
        self
    }

    /// Get the button height.
    fn button_height(&self) -> f32 {
        24.0
    }

    /// Get the option height.
    fn option_height(&self) -> f32 {
        22.0
    }

    /// Get the current selected text.
    fn selected_text(&self) -> &str {
        self.options
            .get(self.selected)
            .map(|s| s.as_str())
            .unwrap_or("---")
    }

    /// Calculate the popup bounds.
    fn popup_bounds(&self, button_bounds: &Rectangle) -> Rectangle {
        let popup_height = self.options.len() as f32 * self.option_height();
        Rectangle::new(
            button_bounds.x,
            button_bounds.y + button_bounds.height,
            button_bounds.width,
            popup_height,
        )
    }

    /// Calculate the total hover area (button + popup when open).
    fn hover_area(&self, button_bounds: &Rectangle) -> Rectangle {
        if self.is_open && !self.options.is_empty() {
            let popup = self.popup_bounds(button_bounds);
            Rectangle::new(
                button_bounds.x,
                button_bounds.y,
                button_bounds.width,
                button_bounds.height + popup.height,
            )
        } else {
            *button_bounds
        }
    }
}

impl<Message: Clone> Widget<Message> for Dropdown<Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let width = self.width.min(limits.max_width);
        let height = self.button_height();
        Layout::new(Rectangle::new(0.0, 0.0, width, height))
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        // Draw button background
        let bg_color = if self.is_hovered || self.is_open {
            self.button_hover_color
        } else {
            self.button_color
        };
        renderer.fill_rect(bounds, bg_color);
        renderer.stroke_rect(bounds, Color::rgb(0.4, 0.5, 0.6), 1.0);

        // Draw selected text
        let text = self.selected_text();
        let text_x = bounds.x + 6.0;
        let text_y = bounds.y + (bounds.height - 12.0) / 2.0;
        renderer.draw_text(text, Point::new(text_x, text_y), self.text_color, 12.0);

        // Draw dropdown arrow
        let arrow = if self.is_open { "▲" } else { "▼" };
        let arrow_x = bounds.x + bounds.width - 16.0;
        renderer.draw_text(arrow, Point::new(arrow_x, text_y), self.text_color, 10.0);

        // Draw popup if open
        if self.is_open && !self.options.is_empty() {
            let popup_rect = self.popup_bounds(&bounds);

            if self.render_as_overlay {
                // Use deferred overlay rendering
                self.draw_popup_overlay(renderer, &bounds, &popup_rect);
            } else {
                // Draw popup normally (may be clipped by parent scrollable)
                self.draw_popup(renderer, &bounds, &popup_rect);
            }
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let hover_area = self.hover_area(&bounds);

        match event {
            Event::MouseMoved { position } => {
                let was_hovered = self.is_hovered;
                self.is_hovered = bounds.contains(*position);

                // Check if hovering over popup options
                if self.is_open {
                    let popup_rect = self.popup_bounds(&bounds);
                    self.hovered_option = None;

                    for i in 0..self.options.len() {
                        let option_y = popup_rect.y + (i as f32 * self.option_height());
                        let option_rect = Rectangle::new(
                            bounds.x,
                            option_y,
                            bounds.width,
                            self.option_height(),
                        );

                        if option_rect.contains(*position) {
                            self.hovered_option = Some(i);
                            break;
                        }
                    }

                    // Check if mouse left the entire dropdown area
                    if !hover_area.contains(*position) && (was_hovered || self.hovered_option.is_some()) {
                        // Mouse left - close the dropdown
                        return self.on_close.as_ref().map(|f| f());
                    }
                }
                None
            }
            Event::MousePressed {
                button: MouseButton::Left,
                position,
            } => {
                // Check if clicking on button
                if bounds.contains(*position) {
                    if self.is_open {
                        // Already open - close it
                        return self.on_close.as_ref().map(|f| f());
                    } else {
                        // Open the dropdown
                        return self.on_open.as_ref().map(|f| f());
                    }
                }

                // Check if clicking on popup option
                if self.is_open {
                    let popup_rect = self.popup_bounds(&bounds);

                    if popup_rect.contains(*position) {
                        // Find which option was clicked
                        for i in 0..self.options.len() {
                            let option_y = popup_rect.y + (i as f32 * self.option_height());
                            let option_rect = Rectangle::new(
                                bounds.x,
                                option_y,
                                bounds.width,
                                self.option_height(),
                            );

                            if option_rect.contains(*position) {
                                // Option clicked - select it and close
                                // First emit on_select, then on_close
                                if let Some(ref on_select) = self.on_select {
                                    return Some(on_select(i));
                                }
                                // If no on_select, just close
                                return self.on_close.as_ref().map(|f| f());
                            }
                        }
                    } else {
                        // Clicked outside dropdown area - close it
                        return self.on_close.as_ref().map(|f| f());
                    }
                }
                None
            }
            _ => None,
        }
    }
}

impl<Message> Dropdown<Message> {
    /// Draw the popup without overlay (normal rendering).
    fn draw_popup(&self, renderer: &mut Renderer, button_bounds: &Rectangle, popup_rect: &Rectangle) {
        // Draw popup background
        renderer.fill_rect(*popup_rect, self.popup_bg_color);
        renderer.stroke_rect(*popup_rect, Color::rgb(0.4, 0.5, 0.6), 1.0);

        // Draw options
        for (i, option) in self.options.iter().enumerate() {
            let option_y = popup_rect.y + (i as f32 * self.option_height());
            let option_rect = Rectangle::new(
                button_bounds.x,
                option_y,
                button_bounds.width,
                self.option_height(),
            );

            // Highlight hovered option
            if self.hovered_option == Some(i) {
                renderer.fill_rect(option_rect, self.popup_hover_color);
            }

            // Highlight selected option
            if i == self.selected {
                renderer.stroke_rect(option_rect, Color::rgb(0.5, 0.6, 0.7), 1.0);
            }

            // Draw option text
            let opt_text_y = option_y + (self.option_height() - 12.0) / 2.0;
            renderer.draw_text(
                option,
                Point::new(button_bounds.x + 6.0, opt_text_y),
                self.text_color,
                12.0,
            );
        }
    }

    /// Draw the popup as an overlay (deferred rendering above other content).
    fn draw_popup_overlay(
        &self,
        renderer: &mut Renderer,
        button_bounds: &Rectangle,
        popup_rect: &Rectangle,
    ) {
        // Begin recording to overlay layer
        renderer.begin_overlay();

        // Draw popup (commands will go to overlay buffer)
        self.draw_popup(renderer, button_bounds, popup_rect);

        // End overlay recording
        renderer.end_overlay();
    }
}

/// Helper function to create a dropdown.
pub fn dropdown<Message>(options: Vec<String>, selected: usize) -> Dropdown<Message> {
    Dropdown::new(options, selected)
}
