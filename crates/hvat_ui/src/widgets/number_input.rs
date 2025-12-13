//! Number input widget for editable numeric values

use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Length, Padding, Size};
use crate::renderer::{Color, Renderer};
use crate::state::NumberInputState;
use crate::widget::Widget;

/// Default padding
const DEFAULT_PADDING: Padding = Padding {
    top: 6.0,
    right: 8.0,
    bottom: 6.0,
    left: 8.0,
};

/// Default font size
const DEFAULT_FONT_SIZE: f32 = 14.0;
/// Cursor width
const CURSOR_WIDTH: f32 = 1.0;
/// Character width approximation
const CHAR_WIDTH_FACTOR: f32 = 0.6;
/// Button width for increment/decrement buttons
const BUTTON_WIDTH: f32 = 20.0;

/// Configuration for number input appearance
#[derive(Debug, Clone)]
pub struct NumberInputConfig {
    /// Background color
    pub background_color: Color,
    /// Background color when focused
    pub focused_background_color: Color,
    /// Border color
    pub border_color: Color,
    /// Border color when focused
    pub focused_border_color: Color,
    /// Text color
    pub text_color: Color,
    /// Cursor color
    pub cursor_color: Color,
    /// Selection background color
    pub selection_color: Color,
    /// Button background color
    pub button_color: Color,
    /// Button hover color
    pub button_hover_color: Color,
    /// Button text color
    pub button_text_color: Color,
}

impl Default for NumberInputConfig {
    fn default() -> Self {
        Self {
            background_color: Color::rgb(0.15, 0.15, 0.17),
            focused_background_color: Color::rgb(0.18, 0.18, 0.2),
            border_color: Color::BORDER,
            focused_border_color: Color::ACCENT,
            text_color: Color::TEXT_PRIMARY,
            cursor_color: Color::ACCENT,
            selection_color: Color::rgba(0.4, 0.6, 1.0, 0.3),
            button_color: Color::rgb(0.2, 0.2, 0.24),
            button_hover_color: Color::rgb(0.28, 0.28, 0.32),
            button_text_color: Color::TEXT_PRIMARY,
        }
    }
}

/// Button hover state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonHover {
    None,
    Increment,
    Decrement,
}

/// A number input widget for editable numeric values
pub struct NumberInput<M> {
    /// Minimum value (optional)
    min: Option<f32>,
    /// Maximum value (optional)
    max: Option<f32>,
    /// Step size for increment/decrement
    step: f32,
    /// Widget state (cloned from external)
    state: NumberInputState,
    /// Width
    width: Length,
    /// Height
    height: Length,
    /// Padding
    padding: Padding,
    /// Font size
    font_size: f32,
    /// Whether to show increment/decrement buttons
    show_buttons: bool,
    /// Configuration
    config: NumberInputConfig,
    /// Button hover state
    button_hover: ButtonHover,
    /// Callback for value changes
    on_change: Option<Box<dyn Fn(f32, NumberInputState) -> M>>,
}

impl<M> NumberInput<M> {
    /// Create a new number input
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            step: 1.0,
            state: NumberInputState::default(),
            width: Length::Fixed(120.0),
            height: Length::Shrink,
            padding: DEFAULT_PADDING,
            font_size: DEFAULT_FONT_SIZE,
            show_buttons: true,
            config: NumberInputConfig::default(),
            button_hover: ButtonHover::None,
            on_change: None,
        }
    }

    /// Set the minimum value
    pub fn min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }

    /// Set the maximum value
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    /// Set the range (min and max)
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Set the step size
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Set the state
    pub fn state(mut self, state: &NumberInputState) -> Self {
        self.state = state.clone();
        self
    }

    /// Set the width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Set the height
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Set the padding
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Set the font size
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Show or hide increment/decrement buttons
    pub fn show_buttons(mut self, show: bool) -> Self {
        self.show_buttons = show;
        self
    }

    /// Set the configuration
    pub fn config(mut self, config: NumberInputConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the change handler
    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(f32, NumberInputState) -> M + 'static,
    {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Get the content bounds (inside padding, excluding buttons)
    fn content_bounds(&self, bounds: Bounds) -> Bounds {
        let button_space = if self.show_buttons {
            BUTTON_WIDTH * 2.0
        } else {
            0.0
        };
        Bounds::new(
            bounds.x + self.padding.left,
            bounds.y + self.padding.top,
            bounds.width - self.padding.horizontal() - button_space,
            bounds.height - self.padding.vertical(),
        )
    }

    /// Get the increment button bounds
    fn increment_bounds(&self, bounds: Bounds) -> Option<Bounds> {
        if !self.show_buttons {
            return None;
        }
        Some(Bounds::new(
            bounds.x + bounds.width - BUTTON_WIDTH,
            bounds.y,
            BUTTON_WIDTH,
            bounds.height,
        ))
    }

    /// Get the decrement button bounds
    fn decrement_bounds(&self, bounds: Bounds) -> Option<Bounds> {
        if !self.show_buttons {
            return None;
        }
        Some(Bounds::new(
            bounds.x + bounds.width - BUTTON_WIDTH * 2.0,
            bounds.y,
            BUTTON_WIDTH,
            bounds.height,
        ))
    }

    /// Calculate character width
    fn char_width(&self) -> f32 {
        self.font_size * CHAR_WIDTH_FACTOR
    }

    /// Convert x position to character index
    fn x_to_char_index(&self, x: f32, content_x: f32) -> usize {
        let relative_x = x - content_x;
        let char_width = self.char_width();
        let index = (relative_x / char_width).round() as i32;
        index.clamp(0, self.state.text.len() as i32) as usize
    }

    /// Get cursor x position
    fn cursor_x(&self, content_x: f32) -> f32 {
        content_x + self.state.cursor as f32 * self.char_width()
    }

    /// Clamp value to min/max range
    fn clamp_value(&self, value: f32) -> f32 {
        let mut v = value;
        if let Some(min) = self.min {
            v = v.max(min);
        }
        if let Some(max) = self.max {
            v = v.min(max);
        }
        v
    }

    /// Handle character insertion (only allow valid number characters)
    fn insert_char(&mut self, c: char) -> bool {
        // Only allow digits, minus, and period
        if !c.is_ascii_digit() && c != '-' && c != '.' {
            return false;
        }

        // Minus only at start
        if c == '-' && self.state.cursor != 0 {
            return false;
        }

        // Only one period
        if c == '.' && self.state.text.contains('.') {
            return false;
        }

        // If there's a selection, delete it first
        if let Some((start, end)) = self.state.selection {
            let (start, end) = (start.min(end), start.max(end));
            self.state.text.drain(start..end);
            self.state.cursor = start;
            self.state.selection = None;
        }

        self.state.text.insert(self.state.cursor, c);
        self.state.cursor += 1;
        true
    }

    /// Handle backspace
    fn handle_backspace(&mut self) -> bool {
        if let Some((start, end)) = self.state.selection {
            let (start, end) = (start.min(end), start.max(end));
            self.state.text.drain(start..end);
            self.state.cursor = start;
            self.state.selection = None;
            true
        } else if self.state.cursor > 0 {
            self.state.text.remove(self.state.cursor - 1);
            self.state.cursor -= 1;
            true
        } else {
            false
        }
    }

    /// Handle delete
    fn handle_delete(&mut self) -> bool {
        if let Some((start, end)) = self.state.selection {
            let (start, end) = (start.min(end), start.max(end));
            self.state.text.drain(start..end);
            self.state.cursor = start;
            self.state.selection = None;
            true
        } else if self.state.cursor < self.state.text.len() {
            self.state.text.remove(self.state.cursor);
            true
        } else {
            false
        }
    }

    /// Increment the value
    fn increment(&mut self) -> bool {
        if let Some(value) = self.state.value() {
            let new_value = self.clamp_value(value + self.step);
            self.state.set_value(new_value);
            true
        } else {
            false
        }
    }

    /// Decrement the value
    fn decrement(&mut self) -> bool {
        if let Some(value) = self.state.value() {
            let new_value = self.clamp_value(value - self.step);
            self.state.set_value(new_value);
            true
        } else {
            false
        }
    }

    /// Validate and clamp the current text value
    fn validate_value(&mut self) -> Option<f32> {
        if let Some(value) = self.state.value() {
            let clamped = self.clamp_value(value);
            if (clamped - value).abs() > f32::EPSILON {
                self.state.set_value(clamped);
            }
            Some(clamped)
        } else {
            None
        }
    }
}

impl<M> Default for NumberInput<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Clone + 'static> Widget<M> for NumberInput<M> {
    fn layout(&mut self, available: Size) -> Size {
        let content_height = self.font_size * 1.2;
        let min_height = content_height + self.padding.vertical();
        let min_width = 80.0;

        Size::new(
            self.width.resolve(available.width, min_width),
            self.height.resolve(available.height, min_height),
        )
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        // Draw background
        let bg_color = if self.state.is_focused {
            self.config.focused_background_color
        } else {
            self.config.background_color
        };
        renderer.fill_rect(bounds, bg_color);

        // Draw border
        let border_color = if self.state.is_focused {
            self.config.focused_border_color
        } else {
            self.config.border_color
        };
        renderer.stroke_rect(bounds, border_color, 1.0);

        let content = self.content_bounds(bounds);

        // Draw selection if present
        if self.state.is_focused {
            if let Some((start, end)) = self.state.selection {
                let (start, end) = (start.min(end), start.max(end));
                let char_width = self.char_width();
                let sel_x = content.x + start as f32 * char_width;
                let sel_width = (end - start) as f32 * char_width;
                let sel_bounds = Bounds::new(sel_x, content.y, sel_width, content.height);
                renderer.fill_rect(sel_bounds, self.config.selection_color);
            }
        }

        // Draw text
        let text_y = content.y + (content.height - self.font_size) / 2.0;
        renderer.text(
            &self.state.text,
            content.x,
            text_y,
            self.font_size,
            self.config.text_color,
        );

        // Draw cursor if focused
        if self.state.is_focused {
            let cursor_x = self.cursor_x(content.x);
            let cursor_bounds = Bounds::new(
                cursor_x,
                content.y + 2.0,
                CURSOR_WIDTH,
                content.height - 4.0,
            );
            renderer.fill_rect(cursor_bounds, self.config.cursor_color);
        }

        // Draw buttons
        if self.show_buttons {
            // Decrement button
            if let Some(dec_bounds) = self.decrement_bounds(bounds) {
                let btn_color = if self.button_hover == ButtonHover::Decrement {
                    self.config.button_hover_color
                } else {
                    self.config.button_color
                };
                renderer.fill_rect(dec_bounds, btn_color);
                renderer.stroke_rect(dec_bounds, self.config.border_color, 1.0);
                // Draw minus
                let minus_y = dec_bounds.y + dec_bounds.height / 2.0;
                let minus_x = dec_bounds.x + 4.0;
                renderer.line(
                    minus_x,
                    minus_y,
                    minus_x + dec_bounds.width - 8.0,
                    minus_y,
                    self.config.button_text_color,
                    2.0,
                );
            }

            // Increment button
            if let Some(inc_bounds) = self.increment_bounds(bounds) {
                let btn_color = if self.button_hover == ButtonHover::Increment {
                    self.config.button_hover_color
                } else {
                    self.config.button_color
                };
                renderer.fill_rect(inc_bounds, btn_color);
                renderer.stroke_rect(inc_bounds, self.config.border_color, 1.0);
                // Draw plus
                let center_x = inc_bounds.x + inc_bounds.width / 2.0;
                let center_y = inc_bounds.y + inc_bounds.height / 2.0;
                let half_size = 5.0;
                renderer.line(
                    center_x - half_size,
                    center_y,
                    center_x + half_size,
                    center_y,
                    self.config.button_text_color,
                    2.0,
                );
                renderer.line(
                    center_x,
                    center_y - half_size,
                    center_x,
                    center_y + half_size,
                    self.config.button_text_color,
                    2.0,
                );
            }
        }
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        let content = self.content_bounds(bounds);

        match event {
            Event::MouseMove { position, .. } => {
                let (x, y) = *position;

                // Update button hover state
                self.button_hover = ButtonHover::None;
                if let Some(inc_bounds) = self.increment_bounds(bounds) {
                    if inc_bounds.contains(x, y) {
                        self.button_hover = ButtonHover::Increment;
                    }
                }
                if let Some(dec_bounds) = self.decrement_bounds(bounds) {
                    if dec_bounds.contains(x, y) {
                        self.button_hover = ButtonHover::Decrement;
                    }
                }

                None
            }

            Event::MousePress {
                button: MouseButton::Left,
                position,
                modifiers,
                ..
            } => {
                let (x, y) = *position;

                // Check button clicks
                if let Some(inc_bounds) = self.increment_bounds(bounds) {
                    if inc_bounds.contains(x, y) {
                        // Blur input when clicking button
                        self.state.is_focused = false;
                        self.state.selection = None;
                        if self.increment() {
                            log::debug!("NumberInput: increment, value = {}", self.state.text);
                            if let Some(ref on_change) = self.on_change {
                                if let Some(value) = self.state.value() {
                                    return Some(on_change(value, self.state.clone()));
                                }
                            }
                        }
                        return None;
                    }
                }

                if let Some(dec_bounds) = self.decrement_bounds(bounds) {
                    if dec_bounds.contains(x, y) {
                        // Blur input when clicking button
                        self.state.is_focused = false;
                        self.state.selection = None;
                        if self.decrement() {
                            log::debug!("NumberInput: decrement, value = {}", self.state.text);
                            if let Some(ref on_change) = self.on_change {
                                if let Some(value) = self.state.value() {
                                    return Some(on_change(value, self.state.clone()));
                                }
                            }
                        }
                        return None;
                    }
                }

                // Check content area click
                if content.contains(x, y) || bounds.contains(x, y) {
                    let was_focused = self.state.is_focused;
                    self.state.is_focused = true;
                    let new_cursor = self.x_to_char_index(x, content.x);

                    if modifiers.shift && was_focused {
                        // Extend selection
                        if let Some((start, _)) = self.state.selection {
                            self.state.selection = Some((start, new_cursor));
                        } else {
                            self.state.selection = Some((self.state.cursor, new_cursor));
                        }
                    } else {
                        self.state.cursor = new_cursor;
                        // Select all on focus
                        if !was_focused && !self.state.text.is_empty() {
                            self.state.selection = Some((0, self.state.text.len()));
                            self.state.cursor = self.state.text.len();
                        } else {
                            self.state.selection = None;
                        }
                    }

                    log::debug!("NumberInput: clicked, cursor = {}", self.state.cursor);
                    return None;
                } else if self.state.is_focused {
                    // Clicked outside - blur and validate
                    self.state.is_focused = false;
                    self.state.selection = None;
                    if let Some(value) = self.validate_value() {
                        log::debug!("NumberInput: blurred, validated value = {}", value);
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(value, self.state.clone()));
                        }
                    }
                }

                None
            }

            Event::TextInput { text } if self.state.is_focused => {
                let mut changed = false;
                for c in text.chars() {
                    if self.insert_char(c) {
                        changed = true;
                    }
                }
                if changed {
                    log::debug!("NumberInput: text input, value = '{}'", self.state.text);
                    if let Some(ref on_change) = self.on_change {
                        if let Some(value) = self.state.value() {
                            return Some(on_change(value, self.state.clone()));
                        }
                    }
                }
                None
            }

            Event::KeyPress { key, modifiers, .. } if self.state.is_focused => {
                match key {
                    KeyCode::Backspace => {
                        if self.handle_backspace() {
                            log::debug!("NumberInput: backspace, value = '{}'", self.state.text);
                            if let Some(ref on_change) = self.on_change {
                                if let Some(value) = self.state.value() {
                                    return Some(on_change(value, self.state.clone()));
                                }
                            }
                        }
                    }
                    KeyCode::Delete => {
                        if self.handle_delete() {
                            log::debug!("NumberInput: delete, value = '{}'", self.state.text);
                            if let Some(ref on_change) = self.on_change {
                                if let Some(value) = self.state.value() {
                                    return Some(on_change(value, self.state.clone()));
                                }
                            }
                        }
                    }
                    KeyCode::Left => {
                        if modifiers.shift {
                            if self.state.cursor > 0 {
                                let anchor = self
                                    .state
                                    .selection
                                    .map(|(s, _)| s)
                                    .unwrap_or(self.state.cursor);
                                self.state.cursor -= 1;
                                self.state.selection = Some((anchor, self.state.cursor));
                            }
                        } else {
                            if self.state.selection.is_some() {
                                let (start, end) = self.state.selection.unwrap();
                                self.state.cursor = start.min(end);
                                self.state.selection = None;
                            } else if self.state.cursor > 0 {
                                self.state.cursor -= 1;
                            }
                        }
                    }
                    KeyCode::Right => {
                        if modifiers.shift {
                            if self.state.cursor < self.state.text.len() {
                                let anchor = self
                                    .state
                                    .selection
                                    .map(|(s, _)| s)
                                    .unwrap_or(self.state.cursor);
                                self.state.cursor += 1;
                                self.state.selection = Some((anchor, self.state.cursor));
                            }
                        } else {
                            if self.state.selection.is_some() {
                                let (start, end) = self.state.selection.unwrap();
                                self.state.cursor = start.max(end);
                                self.state.selection = None;
                            } else if self.state.cursor < self.state.text.len() {
                                self.state.cursor += 1;
                            }
                        }
                    }
                    KeyCode::Home => {
                        if modifiers.shift {
                            let anchor = self
                                .state
                                .selection
                                .map(|(s, _)| s)
                                .unwrap_or(self.state.cursor);
                            self.state.cursor = 0;
                            self.state.selection = Some((anchor, 0));
                        } else {
                            self.state.cursor = 0;
                            self.state.selection = None;
                        }
                    }
                    KeyCode::End => {
                        if modifiers.shift {
                            let anchor = self
                                .state
                                .selection
                                .map(|(s, _)| s)
                                .unwrap_or(self.state.cursor);
                            self.state.cursor = self.state.text.len();
                            self.state.selection = Some((anchor, self.state.cursor));
                        } else {
                            self.state.cursor = self.state.text.len();
                            self.state.selection = None;
                        }
                    }
                    KeyCode::A if modifiers.ctrl => {
                        self.state.selection = Some((0, self.state.text.len()));
                        self.state.cursor = self.state.text.len();
                    }
                    KeyCode::Up => {
                        if self.increment() {
                            log::debug!("NumberInput: up arrow increment, value = {}", self.state.text);
                            if let Some(ref on_change) = self.on_change {
                                if let Some(value) = self.state.value() {
                                    return Some(on_change(value, self.state.clone()));
                                }
                            }
                        }
                    }
                    KeyCode::Down => {
                        if self.decrement() {
                            log::debug!("NumberInput: down arrow decrement, value = {}", self.state.text);
                            if let Some(ref on_change) = self.on_change {
                                if let Some(value) = self.state.value() {
                                    return Some(on_change(value, self.state.clone()));
                                }
                            }
                        }
                    }
                    KeyCode::Enter | KeyCode::Escape => {
                        self.state.is_focused = false;
                        self.state.selection = None;
                        if let Some(value) = self.validate_value() {
                            log::debug!("NumberInput: enter/escape, validated value = {}", value);
                            if let Some(ref on_change) = self.on_change {
                                return Some(on_change(value, self.state.clone()));
                            }
                        }
                    }
                    _ => {}
                }

                None
            }

            Event::MouseScroll {
                delta, position, ..
            } => {
                if bounds.contains(position.0, position.1) {
                    let steps = delta.1.signum() as i32;
                    if steps > 0 {
                        for _ in 0..steps.abs() {
                            self.increment();
                        }
                    } else {
                        for _ in 0..steps.abs() {
                            self.decrement();
                        }
                    }
                    if let Some(ref on_change) = self.on_change {
                        if let Some(value) = self.state.value() {
                            return Some(on_change(value, self.state.clone()));
                        }
                    }
                }
                None
            }

            Event::GlobalMousePress { position, .. } => {
                // Blur when clicking outside (but not inside - that's handled by MousePress)
                if self.state.is_focused {
                    let (x, y) = *position;
                    if !bounds.contains(x, y) {
                        self.state.is_focused = false;
                        self.state.selection = None;
                        log::debug!("NumberInput: GlobalMousePress outside, blurred");
                        if let Some(value) = self.validate_value() {
                            if let Some(ref on_change) = self.on_change {
                                return Some(on_change(value, self.state.clone()));
                            }
                        }
                    }
                }
                None
            }

            Event::FocusLost => {
                // Blur when window loses focus
                if self.state.is_focused {
                    self.state.is_focused = false;
                    self.state.selection = None;
                    log::debug!("NumberInput: FocusLost, blurred");
                    if let Some(value) = self.validate_value() {
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(value, self.state.clone()));
                        }
                    }
                }
                None
            }

            _ => None,
        }
    }
}
