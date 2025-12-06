//! Dropdown widget for selecting from a list of options.
//!
//! The dropdown renders its popup below the button when open. If there isn't
//! enough space below (near bottom of viewport), it automatically opens upward.
//! It auto-closes when:
//! - An option is clicked
//! - The mouse leaves the dropdown area (button + popup)

use crate::{builder_field, callback_setter, Color, Event, Layout, Limits, MouseButton, Point, Rectangle, Renderer, Widget};
use crate::theme::{colors, ui};

/// A dropdown widget for selecting from a list of options.
pub struct Dropdown<Message> {
    options: Vec<String>,
    selected: usize,
    is_open: bool,
    is_hovered: bool,
    hovered_option: Option<usize>,
    on_select: Option<Box<dyn Fn(usize) -> Message>>,
    on_close: Option<Box<dyn Fn() -> Message>>,
    on_open: Option<Box<dyn Fn() -> Message>>,
    width: f32,
    render_as_overlay: bool,
    button_color: Color,
    button_hover_color: Color,
    text_color: Color,
    popup_bg_color: Color,
    popup_hover_color: Color,
    /// Cached drop-up state (calculated during draw based on viewport)
    drop_up: bool,
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
            button_color: colors::DROPDOWN_BG,
            button_hover_color: colors::DROPDOWN_HOVER,
            text_color: Color::WHITE,
            popup_bg_color: colors::DROPDOWN_MENU,
            popup_hover_color: colors::DROPDOWN_OPTION_HOVER,
            drop_up: false,
        }
    }

    // Callback setters using macros
    callback_setter!(on_select, usize);
    callback_setter!(on_close);
    callback_setter!(on_open);

    // Builder methods using macros
    builder_field!(width, f32);
    builder_field!(button_color, Color);
    builder_field!(button_hover_color, Color);
    builder_field!(text_color, Color);
    builder_field!(popup_bg_color, Color);
    builder_field!(popup_hover_color, Color);

    /// Set the open state (for external control).
    pub fn open(mut self, is_open: bool) -> Self {
        self.is_open = is_open;
        self
    }

    /// Enable overlay rendering for the popup.
    pub fn overlay(mut self, render_as_overlay: bool) -> Self {
        self.render_as_overlay = render_as_overlay;
        self
    }

    fn button_height(&self) -> f32 { 24.0 }
    fn option_height(&self) -> f32 { 22.0 }

    fn popup_height(&self) -> f32 {
        self.options.len() as f32 * self.option_height()
    }

    fn selected_text(&self) -> &str {
        self.options.get(self.selected).map(|s| s.as_str()).unwrap_or("---")
    }

    /// Check if popup should open upward based on button position and viewport height.
    fn should_drop_up(&self, button_bounds: &Rectangle, viewport_height: f32) -> bool {
        let popup_height = self.popup_height();
        let space_below = viewport_height - (button_bounds.y + button_bounds.height);
        // Drop up if not enough space below
        popup_height > space_below
    }

    fn popup_bounds(&self, button_bounds: &Rectangle, drop_up: bool) -> Rectangle {
        let popup_height = self.popup_height();
        let y = if drop_up {
            // Drop up: position popup above the button
            button_bounds.y - popup_height
        } else {
            // Drop down: position popup below the button
            button_bounds.y + button_bounds.height
        };
        Rectangle::new(button_bounds.x, y, button_bounds.width, popup_height)
    }

    fn hover_area(&self, button_bounds: &Rectangle, drop_up: bool) -> Rectangle {
        if self.is_open && !self.options.is_empty() {
            let popup = self.popup_bounds(button_bounds, drop_up);
            if drop_up {
                // Hover area includes popup above and button
                Rectangle::new(
                    button_bounds.x,
                    popup.y,
                    button_bounds.width,
                    popup.height + button_bounds.height,
                )
            } else {
                // Hover area includes button and popup below
                Rectangle::new(
                    button_bounds.x,
                    button_bounds.y,
                    button_bounds.width,
                    button_bounds.height + popup.height,
                )
            }
        } else {
            *button_bounds
        }
    }
}

impl<Message: Clone> Widget<Message> for Dropdown<Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let width = self.width.min(limits.max_width);
        Layout::new(Rectangle::new(0.0, 0.0, width, self.button_height()))
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();
        let viewport_height = renderer.viewport_height();

        // Calculate drop direction based on available space
        let drop_up = self.should_drop_up(&bounds, viewport_height);

        let bg_color = if self.is_hovered || self.is_open {
            self.button_hover_color
        } else {
            self.button_color
        };
        renderer.fill_rect(bounds, bg_color);
        renderer.stroke_rect(bounds, colors::BORDER_LIGHT, 1.0);

        // Draw selected text
        let text = self.selected_text();
        let text_x = bounds.x + 6.0;
        let text_y = bounds.y + (bounds.height - 12.0) / 2.0;
        renderer.draw_text(text, Point::new(text_x, text_y), self.text_color, 12.0);

        // Draw dropdown arrow - always points down when closed, up when open
        let arrow = if self.is_open { ui::ARROW_UP } else { ui::ARROW_DOWN };
        let arrow_x = bounds.x + bounds.width - 16.0;
        renderer.draw_text(arrow, Point::new(arrow_x, text_y), self.text_color, 10.0);

        // Draw popup if open
        if self.is_open && !self.options.is_empty() {
            let popup_rect = self.popup_bounds(&bounds, drop_up);

            if self.render_as_overlay {
                self.draw_popup_overlay(renderer, &bounds, &popup_rect);
            } else {
                self.draw_popup(renderer, &bounds, &popup_rect);
            }
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        // Use cached drop_up state for event handling
        let hover_area = self.hover_area(&bounds, self.drop_up);

        match event {
            Event::MouseMoved { position } => {
                let was_hovered = self.is_hovered;
                self.is_hovered = bounds.contains(*position);

                if self.is_open {
                    let popup_rect = self.popup_bounds(&bounds, self.drop_up);
                    self.hovered_option = None;

                    for i in 0..self.options.len() {
                        let option_y = popup_rect.y + (i as f32 * self.option_height());
                        let option_rect = Rectangle::new(bounds.x, option_y, bounds.width, self.option_height());

                        if option_rect.contains(*position) {
                            self.hovered_option = Some(i);
                            break;
                        }
                    }

                    // Close when mouse leaves the dropdown area
                    if !hover_area.contains(*position) && (was_hovered || self.hovered_option.is_some()) {
                        return self.on_close.as_ref().map(|f| f());
                    }
                }
                None
            }
            Event::MousePressed { button, position } => {
                // Left click on button toggles dropdown
                if *button == MouseButton::Left && bounds.contains(*position) {
                    if self.is_open {
                        return self.on_close.as_ref().map(|f| f());
                    } else {
                        return self.on_open.as_ref().map(|f| f());
                    }
                }

                if self.is_open {
                    let popup_rect = self.popup_bounds(&bounds, self.drop_up);

                    // Left click on popup option selects it
                    if *button == MouseButton::Left && popup_rect.contains(*position) {
                        for i in 0..self.options.len() {
                            let option_y = popup_rect.y + (i as f32 * self.option_height());
                            let option_rect = Rectangle::new(bounds.x, option_y, bounds.width, self.option_height());

                            if option_rect.contains(*position) {
                                if let Some(ref on_select) = self.on_select {
                                    return Some(on_select(i));
                                }
                                return self.on_close.as_ref().map(|f| f());
                            }
                        }
                    }

                    // Any click outside dropdown area closes it
                    if !hover_area.contains(*position) {
                        return self.on_close.as_ref().map(|f| f());
                    }
                }
                None
            }
            // Close dropdown on ANY scroll event when open (regardless of position)
            // This handles scrollbar interactions that may not report position inside our bounds
            Event::MouseWheel { .. } => {
                if self.is_open {
                    return self.on_close.as_ref().map(|f| f());
                }
                None
            }
            _ => None,
        }
    }
}

impl<Message> Dropdown<Message> {
    fn draw_popup(&self, renderer: &mut Renderer, button_bounds: &Rectangle, popup_rect: &Rectangle) {
        renderer.fill_rect(*popup_rect, self.popup_bg_color);
        renderer.stroke_rect(*popup_rect, colors::BORDER_LIGHT, 1.0);

        for (i, option) in self.options.iter().enumerate() {
            let option_y = popup_rect.y + (i as f32 * self.option_height());
            let option_rect = Rectangle::new(button_bounds.x, option_y, button_bounds.width, self.option_height());

            if self.hovered_option == Some(i) {
                renderer.fill_rect(option_rect, self.popup_hover_color);
            }

            if i == self.selected {
                renderer.stroke_rect(option_rect, colors::BORDER_LIGHT, 1.0);
            }

            let opt_text_y = option_y + (self.option_height() - 12.0) / 2.0;
            renderer.draw_text(option, Point::new(button_bounds.x + 6.0, opt_text_y), self.text_color, 12.0);
        }
    }

    fn draw_popup_overlay(&self, renderer: &mut Renderer, button_bounds: &Rectangle, popup_rect: &Rectangle) {
        renderer.begin_overlay();
        self.draw_popup(renderer, button_bounds, popup_rect);
        renderer.end_overlay();
    }
}

/// Helper function to create a dropdown.
pub fn dropdown<Message>(options: Vec<String>, selected: usize) -> Dropdown<Message> {
    Dropdown::new(options, selected)
}
