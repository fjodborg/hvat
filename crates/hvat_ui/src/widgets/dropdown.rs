//! Dropdown/select widget

use crate::constants::{
    DROPDOWN_ARROW_WIDTH, DROPDOWN_TEXT_PADDING_X, SCROLLBAR_PADDING,
    SCROLLBAR_WIDTH_COMPACT,
};
use crate::widgets::scrollbar::draw_simple_vertical_scrollbar;
use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer};
use crate::state::DropdownState;
use crate::widget::Widget;

const SEARCH_BOX_BG_COLOR: Color = Color::rgba(0.12, 0.12, 0.15, 1.0);

/// Placeholder text for searchable dropdowns
const SEARCH_PLACEHOLDER: &str = "Type to filter...";

/// Dropdown configuration
#[derive(Debug, Clone)]
pub struct DropdownConfig {
    /// Button background color
    pub button_bg: Color,
    /// Button hover color
    pub button_hover: Color,
    /// Button text color
    pub text_color: Color,
    /// Popup background color
    pub popup_bg: Color,
    /// Option hover color
    pub option_hover: Color,
    /// Selected option color
    pub selected_bg: Color,
    /// Border color
    pub border_color: Color,
    /// Font size
    pub font_size: f32,
    /// Option height
    pub option_height: f32,
    /// Max visible options before scrolling
    pub max_visible_options: usize,
}

impl Default for DropdownConfig {
    fn default() -> Self {
        Self {
            button_bg: Color::BUTTON_BG,
            button_hover: Color::BUTTON_HOVER,
            text_color: Color::TEXT_PRIMARY,
            popup_bg: Color::rgba(0.15, 0.15, 0.18, 0.98),
            option_hover: Color::rgba(0.25, 0.25, 0.3, 1.0),
            selected_bg: Color::ACCENT,
            border_color: Color::BORDER,
            font_size: 14.0,
            option_height: 28.0,
            max_visible_options: 8,
        }
    }
}

/// A dropdown/select widget
///
/// This widget owns a clone of the dropdown state and emits changes via callbacks.
/// This allows it to work with immutable borrows in view() methods.
pub struct Dropdown<M> {
    /// Internal state (cloned from external)
    state: DropdownState,
    /// List of options
    options: Vec<String>,
    /// Currently selected index
    selected: Option<usize>,
    /// Width constraint
    width: Length,
    /// Placeholder text when nothing selected
    placeholder: String,
    /// Whether search is enabled
    searchable: bool,
    /// Configuration
    config: DropdownConfig,
    /// Callback when an option is selected
    on_select: Option<Box<dyn Fn(usize) -> M>>,
    /// Callback when dropdown state changes (open/close/highlight)
    on_change: Option<Box<dyn Fn(DropdownState) -> M>>,
    /// Internal: cached button bounds
    button_bounds: Bounds,
    /// Internal: is hovering over button
    hover_button: bool,
    /// Internal: hover index in options
    hover_option: Option<usize>,
    /// Cached filtered options (original_index, text_ref_index)
    /// Invalidated when search_text changes
    filtered_cache: Option<(String, Vec<usize>)>,
    /// Cached viewport height for determining popup direction
    viewport_height: f32,
}

impl<M: 'static> Dropdown<M> {
    /// Create a new dropdown
    pub fn new() -> Self {
        Self {
            state: DropdownState::new(),
            options: Vec::new(),
            selected: None,
            width: Length::Fixed(200.0),
            placeholder: "Select...".to_string(),
            searchable: false,
            config: DropdownConfig::default(),
            on_select: None,
            on_change: None,
            button_bounds: Bounds::ZERO,
            hover_button: false,
            hover_option: None,
            filtered_cache: None,
            viewport_height: 0.0,
        }
    }

    /// Set the dropdown state (clones from external state)
    pub fn state(mut self, state: &DropdownState) -> Self {
        self.state = state.clone();
        self
    }

    /// Set the options list
    pub fn options<I, S>(mut self, options: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.options = options.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set the selected index
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Set the width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Set placeholder text
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Enable search/filter
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// Set callback for option selection
    pub fn on_select<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize) -> M + 'static,
    {
        self.on_select = Some(Box::new(callback));
        self
    }

    /// Set callback for state changes (open/close/highlight)
    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(DropdownState) -> M + 'static,
    {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Set configuration
    pub fn config(mut self, config: DropdownConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the viewport height (used to determine if popup should open upward)
    /// If not set, defaults to opening downward
    pub fn viewport_height(mut self, height: f32) -> Self {
        self.viewport_height = height;
        self
    }

    /// Get filtered option indices based on search text (uses cache when possible)
    fn get_filtered_indices(&mut self) -> &[usize] {
        // Check if cache is valid
        let cache_valid = self.filtered_cache.as_ref()
            .map(|(cached_search, _)| cached_search == &self.state.search_text)
            .unwrap_or(false);

        if !cache_valid {
            // Rebuild cache
            let indices: Vec<usize> = if !self.searchable || self.state.search_text.is_empty() {
                (0..self.options.len()).collect()
            } else {
                let search = self.state.search_text.to_lowercase();
                self.options
                    .iter()
                    .enumerate()
                    .filter(|(_, opt)| opt.to_lowercase().contains(&search))
                    .map(|(idx, _)| idx)
                    .collect()
            };
            self.filtered_cache = Some((self.state.search_text.clone(), indices));
        }

        &self.filtered_cache.as_ref().unwrap().1
    }

    /// Get filtered options based on search text (read-only version for drawing)
    fn filtered_options(&self) -> Vec<(usize, &String)> {
        // Use cached indices if available and valid
        if let Some((cached_search, indices)) = &self.filtered_cache {
            if cached_search == &self.state.search_text {
                return indices.iter()
                    .filter_map(|&idx| self.options.get(idx).map(|s| (idx, s)))
                    .collect();
            }
        }

        // Fallback to computing on the fly
        if !self.searchable || self.state.search_text.is_empty() {
            self.options.iter().enumerate().collect()
        } else {
            let search = self.state.search_text.to_lowercase();
            self.options
                .iter()
                .enumerate()
                .filter(|(_, opt)| opt.to_lowercase().contains(&search))
                .collect()
        }
    }

    /// Calculate the height of the popup
    fn popup_height(&self) -> f32 {
        let filtered = self.filtered_options();
        let visible_count = filtered.len().min(self.config.max_visible_options);
        let options_height = visible_count as f32 * self.config.option_height;
        // Add height for search box if searchable
        let search_box_height = if self.searchable {
            self.config.option_height
        } else {
            0.0
        };
        options_height + search_box_height
    }

    /// Calculate popup bounds
    fn popup_bounds(&self, button_bounds: Bounds) -> Bounds {
        let popup_height = self.popup_height();

        let popup_y = if self.state.opens_upward {
            // Open above the button
            button_bounds.y - popup_height
        } else {
            // Open below the button (default)
            button_bounds.bottom()
        };

        Bounds::new(button_bounds.x, popup_y, button_bounds.width, popup_height)
    }

    /// Determine if the popup should open upward based on available space
    /// Uses screen_click_y (the actual screen position of the click) to determine
    /// available space, since button_bounds may be in content-space when inside
    /// a scrollable container.
    fn should_open_upward(&self, screen_click_y: f32, button_height: f32) -> bool {
        let popup_height = self.popup_height();

        // Calculate available space based on screen position
        let button_screen_bottom = screen_click_y + button_height;
        let space_below = self.viewport_height - button_screen_bottom;
        let space_above = screen_click_y;

        log::debug!(
            "should_open_upward: screen_click_y={:.0}, button_height={:.0}, popup_height={:.0}, space_below={:.0}, space_above={:.0}, viewport_height={:.0}",
            screen_click_y, button_height, popup_height, space_below, space_above, self.viewport_height
        );

        // Open upward if:
        // 1. Not enough space below but enough space above, OR
        // 2. Button is in the bottom third of the viewport and there's room above
        let bottom_third = self.viewport_height * 2.0 / 3.0;
        let prefer_upward = screen_click_y > bottom_third && space_above >= popup_height;

        (space_below < popup_height && space_above >= popup_height) || prefer_upward
    }

    /// Get the Y offset for options (accounts for search box position)
    /// When opening upward, search box is at bottom so options start at top (offset = 0)
    /// When opening downward, search box is at top so options start below it
    fn options_y_offset(&self) -> f32 {
        if self.searchable && !self.state.opens_upward {
            self.config.option_height
        } else {
            0.0
        }
    }

    /// Calculate text position within bounds (left-padded, vertically centered)
    fn text_position(&self, bounds: Bounds) -> (f32, f32) {
        let x = bounds.x + DROPDOWN_TEXT_PADDING_X;
        let y = bounds.y + (bounds.height - self.config.font_size) / 2.0;
        (x, y)
    }

    /// Get display text for current selection
    fn display_text(&self) -> &str {
        self.selected
            .and_then(|i| self.options.get(i))
            .map(|s| s.as_str())
            .unwrap_or(&self.placeholder)
    }

    /// Emit a state change if handler is set
    fn emit_change(&self) -> Option<M> {
        self.on_change.as_ref().map(|f| f(self.state.clone()))
    }

    /// Get the original option index from a filtered index
    fn get_original_index(&self, filtered_index: usize) -> Option<usize> {
        self.filtered_options()
            .get(filtered_index)
            .map(|(idx, _)| *idx)
    }

    /// Calculate filtered index from a Y position in the popup
    /// Returns None if the position is in the search box area
    fn filtered_index_at_position(&self, popup_bounds: Bounds, y: f32) -> Option<usize> {
        // Check if in search box area (different position based on open direction)
        if self.searchable {
            if self.state.opens_upward {
                // Search box at bottom
                let search_top = popup_bounds.bottom() - self.config.option_height;
                if y >= search_top {
                    return None;
                }
            } else {
                // Search box at top
                let search_bottom = popup_bounds.y + self.config.option_height;
                if y < search_bottom {
                    return None;
                }
            }
        }

        let options_y_offset = self.options_y_offset();
        let relative_y = y - popup_bounds.y - options_y_offset;

        // Check if relative_y is negative (shouldn't happen after search box check, but be safe)
        if relative_y < 0.0 {
            return None;
        }

        let visible_index = (relative_y / self.config.option_height) as usize;
        let filtered_index = visible_index + self.state.scroll_offset;

        // Validate index is within bounds
        if filtered_index < self.filtered_options().len() {
            Some(filtered_index)
        } else {
            None
        }
    }
}

impl<M: 'static> Default for Dropdown<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for Dropdown<M> {
    fn layout(&mut self, available: Size) -> Size {
        let width = self.width.resolve(available.width, 200.0);
        let height = self.config.option_height;

        self.button_bounds = Bounds::new(0.0, 0.0, width, height);

        // Pre-compute filtered indices during layout (caches the result)
        if self.state.is_open {
            let _ = self.get_filtered_indices();
        }

        Size::new(width, height)
    }

    fn has_active_overlay(&self) -> bool {
        self.state.is_open
    }

    fn capture_bounds(&self, layout_bounds: Bounds) -> Option<Bounds> {
        if self.state.is_open {
            // When open, capture events in both button and popup area
            let button_bounds = Bounds::new(
                layout_bounds.x,
                layout_bounds.y,
                self.button_bounds.width,
                self.button_bounds.height,
            );
            let popup_bounds = self.popup_bounds(button_bounds);

            // Return combined bounds of button and popup
            // Handle both upward and downward popup directions
            let combined_y = button_bounds.y.min(popup_bounds.y);
            let combined_bottom = button_bounds.bottom().max(popup_bounds.bottom());

            Some(Bounds::new(
                button_bounds.x,
                combined_y,
                button_bounds.width.max(popup_bounds.width),
                combined_bottom - combined_y,
            ))
        } else {
            None
        }
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        log::debug!("Dropdown draw: bounds={:?}, is_open={}", bounds, self.state.is_open);

        // Draw button
        let button_bounds = Bounds::new(
            bounds.x,
            bounds.y,
            self.button_bounds.width,
            self.button_bounds.height,
        );

        let button_bg = if self.hover_button || self.state.is_open {
            self.config.button_hover
        } else {
            self.config.button_bg
        };

        renderer.fill_rect(button_bounds, button_bg);
        renderer.stroke_rect(button_bounds, self.config.border_color, 1.0);

        // Draw selected text
        let (text_x, text_y) = self.text_position(button_bounds);
        renderer.text(
            self.display_text(),
            text_x,
            text_y,
            self.config.font_size,
            self.config.text_color,
        );

        // Draw arrow indicator
        let arrow = if self.state.is_open { "▲" } else { "▼" };
        let arrow_x = button_bounds.right() - DROPDOWN_ARROW_WIDTH;
        renderer.text(
            arrow,
            arrow_x,
            text_y,
            self.config.font_size,
            self.config.text_color,
        );

        // Draw popup if open
        if self.state.is_open {
            self.draw_popup(renderer, button_bounds);
        }
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        let button_bounds = Bounds::new(
            bounds.x,
            bounds.y,
            self.button_bounds.width,
            self.button_bounds.height,
        );

        let popup_bounds = self.popup_bounds(button_bounds);

        match event {
            Event::MousePress {
                button: MouseButton::Left,
                position,
                screen_position,
                ..
            } => {
                log::debug!(
                    "Dropdown click: pos=({:.0}, {:.0}), screen_pos={:?}, button_bounds={:?}, popup_bounds={:?}, is_open={}",
                    position.0, position.1, screen_position, button_bounds, popup_bounds, self.state.is_open
                );

                // Click on button - toggle dropdown
                if button_bounds.contains(position.0, position.1) {
                    log::debug!("Click on button - toggling");
                    if self.state.is_open {
                        self.state.close();
                    } else {
                        // Determine popup direction before opening
                        // Use screen_position if available, otherwise fall back to position
                        let screen_y = screen_position.map(|(_, y)| y).unwrap_or(position.1);
                        self.state.opens_upward = self.should_open_upward(screen_y, button_bounds.height);
                        self.state.open();
                    }
                    return self.emit_change();
                }

                // Click on option in popup
                if self.state.is_open && popup_bounds.contains(position.0, position.1) {
                    if let Some(filtered_index) = self.filtered_index_at_position(popup_bounds, position.1) {
                        if let Some(idx) = self.get_original_index(filtered_index) {
                            log::debug!("Click on popup option {}", idx);
                            self.state.close();
                            if let Some(on_select) = &self.on_select {
                                return Some(on_select(idx));
                            }
                            return self.emit_change();
                        }
                    } else {
                        log::debug!("Click on search box - ignoring");
                        return None;
                    }
                }

                // Click outside - close
                if self.state.is_open {
                    self.state.close();
                    return self.emit_change();
                }
            }

            Event::MouseMove { position, .. } => {
                // Update button hover state
                self.hover_button = button_bounds.contains(position.0, position.1);

                // Update option hover state
                if self.state.is_open && popup_bounds.contains(position.0, position.1) {
                    if let Some(filtered_index) = self.filtered_index_at_position(popup_bounds, position.1) {
                        self.hover_option = Some(filtered_index);
                        self.state.highlighted = Some(filtered_index);
                    } else {
                        // Hovering over search box
                        self.hover_option = None;
                    }
                } else {
                    self.hover_option = None;
                }
            }

            Event::MouseScroll { delta, position, .. } => {
                if self.state.is_open {
                    // Handle scroll within popup
                    if popup_bounds.contains(position.0, position.1) {
                        let filtered_len = self.filtered_options().len();
                        let visible_count = self.config.max_visible_options;

                        // Scroll by 1 item per scroll unit (negative delta = scroll down)
                        let scroll_delta = if delta.1 < 0.0 { 1 } else { -1 };
                        self.state.scroll_by(scroll_delta, filtered_len, visible_count);

                        log::debug!(
                            "Dropdown scroll: delta={:?}, new_offset={}, max_items={}, visible={}",
                            delta, self.state.scroll_offset, filtered_len, visible_count
                        );

                        return self.emit_change();
                    }

                    // Scroll outside dropdown - close it
                    if !button_bounds.contains(position.0, position.1) {
                        log::debug!("Scroll outside dropdown - closing");
                        self.state.close();
                        return self.emit_change();
                    }
                }
            }

            Event::KeyPress { key, .. } => {
                if self.state.is_open {
                    match key {
                        KeyCode::Escape => {
                            self.state.close();
                            return self.emit_change();
                        }
                        KeyCode::Enter => {
                            if let Some(highlighted) = self.state.highlighted {
                                if let Some(idx) = self.get_original_index(highlighted) {
                                    self.state.close();
                                    if let Some(on_select) = &self.on_select {
                                        return Some(on_select(idx));
                                    }
                                    return self.emit_change();
                                }
                            }
                        }
                        KeyCode::Up => {
                            let filtered_len = self.filtered_options().len();
                            if filtered_len > 0 {
                                let current = self.state.highlighted.unwrap_or(0);
                                self.state.highlighted = Some(current.saturating_sub(1));
                                // Ensure highlighted item is visible
                                self.state.ensure_highlighted_visible(self.config.max_visible_options);
                                return self.emit_change();
                            }
                        }
                        KeyCode::Down => {
                            let filtered_len = self.filtered_options().len();
                            if filtered_len > 0 {
                                let current = self.state.highlighted.unwrap_or(0);
                                let max = filtered_len.saturating_sub(1);
                                self.state.highlighted = Some((current + 1).min(max));
                                // Ensure highlighted item is visible
                                self.state.ensure_highlighted_visible(self.config.max_visible_options);
                                return self.emit_change();
                            }
                        }
                        KeyCode::Backspace if self.searchable => {
                            // Remove last character from search text
                            if !self.state.search_text.is_empty() {
                                self.state.search_text.pop();
                                // Reset scroll and highlighted to first match
                                self.state.scroll_offset = 0;
                                self.state.highlighted = if self.filtered_options().is_empty() {
                                    None
                                } else {
                                    Some(0)
                                };
                                log::debug!("Dropdown search backspace: '{}'", self.state.search_text);
                                return self.emit_change();
                            }
                        }
                        _ => {}
                    }
                }
            }

            Event::TextInput { text } if self.searchable && self.state.is_open => {
                self.state.search_text.push_str(text);
                // Reset scroll and highlighted to first match
                self.state.scroll_offset = 0;
                self.state.highlighted = if self.filtered_options().is_empty() {
                    None
                } else {
                    Some(0)
                };
                log::debug!("Dropdown search text: '{}'", self.state.search_text);
                return self.emit_change();
            }

            Event::GlobalMousePress { position, .. } => {
                // Close dropdown if click is outside both button and popup
                if self.state.is_open {
                    let in_button = button_bounds.contains(position.0, position.1);
                    let in_popup = popup_bounds.contains(position.0, position.1);
                    if !in_button && !in_popup {
                        log::debug!("GlobalMousePress outside dropdown - closing");
                        self.state.close();
                        return self.emit_change();
                    }
                }
            }

            Event::FocusLost => {
                // Close dropdown when focus is lost (e.g., clicking elsewhere, tab away)
                if self.state.is_open {
                    log::debug!("FocusLost - closing dropdown");
                    self.state.close();
                    return self.emit_change();
                }
            }

            _ => {}
        }

        None
    }
}

impl<M: 'static> Dropdown<M> {
    /// Draw the popup overlay
    fn draw_popup(&self, renderer: &mut Renderer, button_bounds: Bounds) {
        let popup_bounds = self.popup_bounds(button_bounds);
        let filtered = self.filtered_options();
        let total_items = filtered.len();
        let visible_items = self.config.max_visible_options.min(total_items);
        let scroll_offset = self.state.scroll_offset;
        let needs_scrollbar = total_items > visible_items;
        let options_y_offset = self.options_y_offset();

        // Register the popup as an overlay so events can be properly routed
        // This allows the overlay hint system to know events in this area are for the popup
        renderer.register_overlay(popup_bounds);

        // Start overlay rendering (on top of other content)
        renderer.begin_overlay();

        // Draw popup background
        renderer.fill_rect(popup_bounds, self.config.popup_bg);
        renderer.stroke_rect(popup_bounds, self.config.border_color, 1.0);

        // Draw search box if searchable
        if self.searchable {
            self.draw_search_box(renderer, popup_bounds);
        }

        // Calculate content width (narrower if scrollbar present)
        let content_width = if needs_scrollbar {
            popup_bounds.width - SCROLLBAR_WIDTH_COMPACT - SCROLLBAR_PADDING * 2.0
        } else {
            popup_bounds.width
        };

        // Draw visible options (with scroll offset)
        for visible_index in 0..visible_items {
            let filtered_index = scroll_offset + visible_index;
            if filtered_index >= total_items {
                break;
            }

            let (original_index, option_text) = &filtered[filtered_index];

            let option_bounds = Bounds::new(
                popup_bounds.x,
                popup_bounds.y + options_y_offset + visible_index as f32 * self.config.option_height,
                content_width,
                self.config.option_height,
            );

            // Determine background color - use filtered_index for selection/highlight
            let is_selected = self.selected == Some(*original_index);
            let is_highlighted = self.state.highlighted == Some(filtered_index);
            let is_hovered = self.hover_option == Some(filtered_index);

            let bg_color = if is_selected {
                self.config.selected_bg
            } else if is_highlighted || is_hovered {
                self.config.option_hover
            } else {
                self.config.popup_bg
            };

            renderer.fill_rect(option_bounds, bg_color);

            // Draw option text
            let (text_x, text_y) = self.text_position(option_bounds);
            renderer.text(
                option_text,
                text_x,
                text_y,
                self.config.font_size,
                self.config.text_color,
            );
        }

        // Draw scrollbar if needed
        if needs_scrollbar {
            self.draw_scrollbar(renderer, popup_bounds, total_items, visible_items);
        }

        // End overlay rendering
        renderer.end_overlay();
    }

    /// Draw the search box (at top when opening down, at bottom when opening up)
    fn draw_search_box(&self, renderer: &mut Renderer, popup_bounds: Bounds) {
        let search_y = if self.state.opens_upward {
            // Search box at bottom when opening upward
            popup_bounds.bottom() - self.config.option_height
        } else {
            // Search box at top when opening downward
            popup_bounds.y
        };

        let search_bounds = Bounds::new(
            popup_bounds.x,
            search_y,
            popup_bounds.width,
            self.config.option_height,
        );

        // Draw search box background
        renderer.fill_rect(search_bounds, SEARCH_BOX_BG_COLOR);

        // Draw search text or placeholder
        let (search_text, text_color) = if self.state.search_text.is_empty() {
            (SEARCH_PLACEHOLDER, Color::TEXT_SECONDARY)
        } else {
            (self.state.search_text.as_str(), self.config.text_color)
        };

        let (text_x, text_y) = self.text_position(search_bounds);
        renderer.text(search_text, text_x, text_y, self.config.font_size, text_color);

        // Draw separator line (above when at bottom, below when at top)
        let separator_y = if self.state.opens_upward {
            search_bounds.y
        } else {
            search_bounds.bottom() - 1.0
        };
        renderer.fill_rect(
            Bounds::new(popup_bounds.x, separator_y, popup_bounds.width, 1.0),
            self.config.border_color,
        );
    }

    /// Draw the scrollbar for the dropdown popup
    fn draw_scrollbar(
        &self,
        renderer: &mut Renderer,
        popup_bounds: Bounds,
        total_items: usize,
        visible_items: usize,
    ) {
        let options_y_offset = self.options_y_offset();
        let scrollbar_track_bounds = Bounds::new(
            popup_bounds.right() - SCROLLBAR_WIDTH_COMPACT - SCROLLBAR_PADDING,
            popup_bounds.y + options_y_offset + SCROLLBAR_PADDING,
            SCROLLBAR_WIDTH_COMPACT,
            popup_bounds.height - options_y_offset - SCROLLBAR_PADDING * 2.0,
        );

        // Convert item-based scrolling to pixel-based for the shared utility
        // Using total_items and visible_items as proxy for content/viewport size
        let max_scroll = total_items.saturating_sub(visible_items);
        if max_scroll == 0 {
            return;
        }

        draw_simple_vertical_scrollbar(
            renderer,
            scrollbar_track_bounds,
            total_items as f32,
            visible_items as f32,
            self.state.scroll_offset as f32,
            SCROLLBAR_WIDTH_COMPACT,
        );
    }
}
