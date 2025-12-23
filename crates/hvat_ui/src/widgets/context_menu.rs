//! Context menu widget for right-click actions
//!
//! A popup menu that appears at a specific position (typically where the user right-clicked).
//! Displays a list of menu items that can be selected.

use crate::callback::Callback;
use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Size};
use crate::renderer::{Color, Renderer};
use crate::widget::{EventResult, Widget};
use crate::widgets::overlay::OverlayCloseHelper;

/// A menu item in the context menu
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// Unique identifier for this item
    pub id: String,
    /// Display label
    pub label: String,
    /// Optional color swatch to display
    pub color: Option<[u8; 3]>,
    /// Whether this item is a separator
    pub is_separator: bool,
    /// Whether this item is disabled (grayed out)
    pub disabled: bool,
}

impl MenuItem {
    /// Create a new menu item
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            color: None,
            is_separator: false,
            disabled: false,
        }
    }

    /// Create a menu item with a color swatch
    pub fn with_color(mut self, color: [u8; 3]) -> Self {
        self.color = Some(color);
        self
    }

    /// Mark this item as disabled
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Create a separator item
    pub fn separator() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            color: None,
            is_separator: true,
            disabled: false,
        }
    }
}

/// Configuration for the context menu
#[derive(Debug, Clone)]
pub struct ContextMenuConfig {
    /// Background color
    pub bg_color: Color,
    /// Hover color for items
    pub hover_color: Color,
    /// Text color
    pub text_color: Color,
    /// Disabled text color
    pub disabled_color: Color,
    /// Border color
    pub border_color: Color,
    /// Font size
    pub font_size: f32,
    /// Item height
    pub item_height: f32,
    /// Horizontal padding
    pub padding_x: f32,
    /// Separator height
    pub separator_height: f32,
    /// Color swatch size
    pub swatch_size: f32,
    /// Minimum width
    pub min_width: f32,
    /// Shadow offset
    pub shadow_offset: f32,
    /// Shadow color
    pub shadow_color: Color,
}

impl Default for ContextMenuConfig {
    fn default() -> Self {
        Self {
            bg_color: Color::rgba(0.15, 0.15, 0.18, 0.98),
            hover_color: Color::rgba(0.25, 0.25, 0.3, 1.0),
            text_color: Color::TEXT_PRIMARY,
            disabled_color: Color::TEXT_SECONDARY,
            border_color: Color::BORDER,
            font_size: 14.0,
            item_height: 28.0,
            padding_x: 12.0,
            separator_height: 9.0,
            swatch_size: 14.0,
            min_width: 150.0,
            shadow_offset: 3.0,
            shadow_color: Color::rgba(0.0, 0.0, 0.0, 0.3),
        }
    }
}

/// Context menu widget
///
/// Displays a popup menu at a specified position with a list of selectable items.
/// Closes when an item is selected, when clicking outside, or when pressing Escape.
pub struct ContextMenu<M> {
    /// Menu items to display
    items: Vec<MenuItem>,
    /// Whether the menu is open
    is_open: bool,
    /// Position to display the menu (screen coordinates)
    position: (f32, f32),
    /// Viewport size for boundary detection
    viewport_size: (f32, f32),
    /// Configuration
    config: ContextMenuConfig,
    /// Currently hovered item index
    hover_index: Option<usize>,
    /// Callback when an item is selected (receives item ID)
    on_select: Callback<String, M>,
    /// Callback when menu is closed without selection
    on_close: Callback<(), M>,
}

impl<M> Default for ContextMenu<M> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            is_open: false,
            position: (0.0, 0.0),
            viewport_size: (800.0, 600.0),
            config: ContextMenuConfig::default(),
            hover_index: None,
            on_select: Callback::none(),
            on_close: Callback::none(),
        }
    }
}

impl<M: 'static> ContextMenu<M> {
    /// Create a new context menu
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether the menu is open
    pub fn open(mut self, is_open: bool) -> Self {
        self.is_open = is_open;
        self
    }

    /// Set the menu position (screen coordinates)
    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.position = (x, y);
        self
    }

    /// Set the viewport size for boundary detection
    pub fn viewport_size(mut self, width: f32, height: f32) -> Self {
        self.viewport_size = (width, height);
        self
    }

    /// Set the menu items
    pub fn items(mut self, items: Vec<MenuItem>) -> Self {
        self.items = items;
        self
    }

    /// Set the configuration
    pub fn config(mut self, config: ContextMenuConfig) -> Self {
        self.config = config;
        self
    }

    /// Set callback for item selection
    pub fn on_select<F>(mut self, callback: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.on_select = Callback::new(callback);
        self
    }

    /// Set callback for menu close
    pub fn on_close<F>(mut self, callback: F) -> Self
    where
        F: Fn(()) -> M + 'static,
    {
        self.on_close = Callback::new(callback);
        self
    }

    /// Calculate the total height of the menu
    fn calculate_height(&self) -> f32 {
        self.items.iter().fold(0.0, |acc, item| {
            acc + if item.is_separator {
                self.config.separator_height
            } else {
                self.config.item_height
            }
        })
    }

    /// Calculate the width based on longest item (estimates based on character count)
    fn calculate_width(&self) -> f32 {
        // Estimate text width: roughly 8 pixels per character at 14pt font
        let char_width = self.config.font_size * 0.6;
        let max_text_width = self
            .items
            .iter()
            .filter(|item| !item.is_separator)
            .map(|item| {
                let mut width = item.label.len() as f32 * char_width;
                if item.color.is_some() {
                    width += self.config.swatch_size + 8.0; // swatch + spacing
                }
                width
            })
            .fold(0.0f32, |a: f32, b: f32| a.max(b));

        (max_text_width + self.config.padding_x * 2.0).max(self.config.min_width)
    }

    /// Calculate menu bounds, adjusting for viewport edges
    fn calculate_bounds(&self) -> Bounds {
        let height = self.calculate_height();
        let width = self.calculate_width();

        let mut x = self.position.0;
        let mut y = self.position.1;

        // Adjust if menu would go off right edge
        if x + width > self.viewport_size.0 {
            x = (self.viewport_size.0 - width - 5.0).max(5.0);
        }

        // Adjust if menu would go off bottom edge
        if y + height > self.viewport_size.1 {
            y = (self.viewport_size.1 - height - 5.0).max(5.0);
        }

        Bounds::new(x, y, width, height)
    }

    /// Get item at a Y position
    fn item_at_position(&self, menu_bounds: Bounds, y: f32) -> Option<usize> {
        if y < menu_bounds.y || y > menu_bounds.bottom() {
            return None;
        }

        let mut current_y = menu_bounds.y;
        for (index, item) in self.items.iter().enumerate() {
            let item_height = if item.is_separator {
                self.config.separator_height
            } else {
                self.config.item_height
            };

            if y >= current_y && y < current_y + item_height {
                // Don't return separator or disabled items
                if item.is_separator || item.disabled {
                    return None;
                }
                return Some(index);
            }

            current_y += item_height;
        }

        None
    }
}

impl<M: 'static> Widget<M> for ContextMenu<M> {
    fn layout(&mut self, _available: Size) -> Size {
        // Context menu is an overlay, it doesn't take up space in the layout
        Size::ZERO
    }

    fn has_active_overlay(&self) -> bool {
        self.is_open
    }

    fn capture_bounds(&self, _layout_bounds: Bounds) -> Option<Bounds> {
        if self.is_open {
            Some(self.calculate_bounds())
        } else {
            None
        }
    }

    fn draw(&self, renderer: &mut Renderer, _bounds: Bounds) {
        if !self.is_open || self.items.is_empty() {
            return;
        }

        let menu_bounds = self.calculate_bounds();

        // Register overlay for event routing
        renderer.register_overlay(menu_bounds);

        // Start overlay rendering
        renderer.begin_overlay();

        // Draw shadow
        let shadow_bounds = Bounds::new(
            menu_bounds.x + self.config.shadow_offset,
            menu_bounds.y + self.config.shadow_offset,
            menu_bounds.width,
            menu_bounds.height,
        );
        renderer.fill_rect(shadow_bounds, self.config.shadow_color);

        // Draw background
        renderer.fill_rect(menu_bounds, self.config.bg_color);
        renderer.stroke_rect(menu_bounds, self.config.border_color, 1.0);

        // Draw items
        let mut current_y = menu_bounds.y;
        for (index, item) in self.items.iter().enumerate() {
            if item.is_separator {
                // Draw separator line
                let sep_y = current_y + self.config.separator_height / 2.0;
                renderer.fill_rect(
                    Bounds::new(
                        menu_bounds.x + self.config.padding_x,
                        sep_y,
                        menu_bounds.width - self.config.padding_x * 2.0,
                        1.0,
                    ),
                    self.config.border_color,
                );
                current_y += self.config.separator_height;
            } else {
                let item_bounds = Bounds::new(
                    menu_bounds.x,
                    current_y,
                    menu_bounds.width,
                    self.config.item_height,
                );

                // Draw hover background
                if self.hover_index == Some(index) && !item.disabled {
                    renderer.fill_rect(item_bounds, self.config.hover_color);
                }

                // Draw color swatch if present
                let mut text_x = menu_bounds.x + self.config.padding_x;
                if let Some(color) = item.color {
                    let swatch_y =
                        current_y + (self.config.item_height - self.config.swatch_size) / 2.0;
                    let swatch_bounds = Bounds::new(
                        text_x,
                        swatch_y,
                        self.config.swatch_size,
                        self.config.swatch_size,
                    );
                    renderer.fill_rect(
                        swatch_bounds,
                        Color::rgba(
                            color[0] as f32 / 255.0,
                            color[1] as f32 / 255.0,
                            color[2] as f32 / 255.0,
                            1.0,
                        ),
                    );
                    renderer.stroke_rect(swatch_bounds, self.config.border_color, 1.0);
                    text_x += self.config.swatch_size + 8.0;
                }

                // Draw label
                let text_y = current_y + (self.config.item_height - self.config.font_size) / 2.0;
                let text_color = if item.disabled {
                    self.config.disabled_color
                } else {
                    self.config.text_color
                };
                renderer.text(
                    &item.label,
                    text_x,
                    text_y,
                    self.config.font_size,
                    text_color,
                );

                current_y += self.config.item_height;
            }
        }

        renderer.end_overlay();
    }

    fn on_event(&mut self, event: &Event, _bounds: Bounds) -> EventResult<M> {
        if !self.is_open {
            return EventResult::None;
        }

        let menu_bounds = self.calculate_bounds();

        match event {
            Event::MouseMove { position, .. } => {
                // Update hover state
                self.hover_index = self.item_at_position(menu_bounds, position.1);
                if menu_bounds.contains(position.0, position.1) {
                    // Update hover based on X as well
                    if position.0 < menu_bounds.x || position.0 > menu_bounds.right() {
                        self.hover_index = None;
                    }
                }
            }

            Event::MousePress {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if menu_bounds.contains(position.0, position.1) {
                    if let Some(index) = self.item_at_position(menu_bounds, position.1) {
                        if let Some(item) = self.items.get(index) {
                            if !item.disabled && !item.is_separator {
                                log::debug!("Context menu item selected: {}", item.id);
                                self.is_open = false;
                                self.hover_index = None;
                                if let Some(msg) = self.on_select.call(item.id.clone()) {
                                    return EventResult::Message(msg);
                                }
                            }
                        }
                    }
                    return EventResult::None;
                }
            }

            Event::GlobalMousePress { position, .. } => {
                // Close if clicking outside
                if OverlayCloseHelper::should_close_on_global_press(*position, menu_bounds) {
                    log::debug!("Context menu closed via GlobalMousePress outside");
                    self.is_open = false;
                    self.hover_index = None;
                    return self.on_close.call(()).into();
                }
            }

            Event::KeyPress {
                key: KeyCode::Escape,
                ..
            } => {
                log::debug!("Context menu closed via Escape");
                self.is_open = false;
                self.hover_index = None;
                return self.on_close.call(()).into();
            }

            Event::FocusLost => {
                log::debug!("Context menu closed via FocusLost");
                self.is_open = false;
                self.hover_index = None;
                return self.on_close.call(()).into();
            }

            _ => {}
        }

        EventResult::None
    }
}
