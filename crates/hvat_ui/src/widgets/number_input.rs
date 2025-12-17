//! Number input widget for editable numeric values

use crate::constants::{
    line_height, DEFAULT_FONT_SIZE, NUMBER_INPUT_BUTTON_WIDTH, NUMBER_INPUT_DEFAULT_WIDTH,
    TEXT_INPUT_PADDING,
};
use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Length, Padding, Size};
use crate::renderer::{Color, Renderer};
use crate::state::NumberInputState;
use crate::widget::Widget;
use crate::widgets::text_core;

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
    /// Side-effect callback for undo point (called when input gains focus)
    /// This is called BEFORE on_change, allowing the app to save a snapshot.
    on_undo_point: Option<Box<dyn Fn()>>,
}

impl<M> NumberInput<M> {
    /// Create a new number input
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            step: 1.0,
            state: NumberInputState::default(),
            width: Length::Fixed(NUMBER_INPUT_DEFAULT_WIDTH),
            height: Length::Shrink,
            padding: TEXT_INPUT_PADDING,
            font_size: DEFAULT_FONT_SIZE,
            show_buttons: true,
            config: NumberInputConfig::default(),
            button_hover: ButtonHover::None,
            on_change: None,
            on_undo_point: None,
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

    /// Set the undo point handler (called when input gains focus)
    ///
    /// This is a side-effect callback invoked at the start of an edit operation
    /// (when focus is gained), BEFORE `on_change` is called. Use this to save an
    /// undo snapshot of the current state before editing begins.
    ///
    /// Unlike `on_change`, this callback does not return a message - it's called
    /// for its side effects only (typically to push state onto an undo stack).
    pub fn on_undo_point<F>(mut self, callback: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.on_undo_point = Some(Box::new(callback));
        self
    }

    /// Get the content bounds (inside padding, excluding buttons)
    fn content_bounds(&self, bounds: Bounds) -> Bounds {
        let button_space = if self.show_buttons {
            NUMBER_INPUT_BUTTON_WIDTH * 2.0
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
            bounds.x + bounds.width - NUMBER_INPUT_BUTTON_WIDTH,
            bounds.y,
            NUMBER_INPUT_BUTTON_WIDTH,
            bounds.height,
        ))
    }

    /// Get the decrement button bounds
    fn decrement_bounds(&self, bounds: Bounds) -> Option<Bounds> {
        if !self.show_buttons {
            return None;
        }
        Some(Bounds::new(
            bounds.x + bounds.width - NUMBER_INPUT_BUTTON_WIDTH * 2.0,
            bounds.y,
            NUMBER_INPUT_BUTTON_WIDTH,
            bounds.height,
        ))
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
        // Use text_core validation
        if !text_core::is_valid_number_char(c, self.state.cursor, &self.state.text) {
            return false;
        }

        // Push undo BEFORE making changes
        self.state.push_undo();

        // If there's a selection, delete it first, then insert
        self.state.cursor = text_core::insert_text(
            &mut self.state.text,
            self.state.cursor,
            self.state.selection,
            &c.to_string(),
        );
        self.state.selection = None;
        true
    }

    /// Handle backspace (with undo support)
    fn handle_backspace(&mut self) -> bool {
        // Check if backspace would do anything
        let would_modify = self.state.selection.is_some()
            || (self.state.cursor > 0 && !self.state.text.is_empty());

        if would_modify {
            // Push undo BEFORE making changes
            self.state.push_undo();

            if let Some(new_cursor) = text_core::handle_backspace(
                &mut self.state.text,
                self.state.cursor,
                self.state.selection,
            ) {
                self.state.cursor = new_cursor;
                self.state.selection = None;
                return true;
            }
        }
        false
    }

    /// Handle delete (with undo support)
    fn handle_delete(&mut self) -> bool {
        // Check if delete would do anything
        let would_modify =
            self.state.selection.is_some() || self.state.cursor < self.state.text.len();

        if would_modify {
            // Push undo BEFORE making changes
            self.state.push_undo();

            if let Some(new_cursor) = text_core::handle_delete(
                &mut self.state.text,
                self.state.cursor,
                self.state.selection,
            ) {
                self.state.cursor = new_cursor;
                self.state.selection = None;
                return true;
            }
        }
        false
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

    /// Emit a state change if handler is set and value is valid
    fn emit_change(&self) -> Option<M> {
        self.state.value().and_then(|value| {
            self.on_change
                .as_ref()
                .map(|f| f(value, self.state.clone()))
        })
    }
}

impl<M> Default for NumberInput<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Clone + 'static> Widget<M> for NumberInput<M> {
    fn layout(&mut self, available: Size) -> Size {
        let content_height = line_height(self.font_size);
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
            if let Some(selection) = self.state.selection {
                text_core::draw_selection(
                    renderer,
                    content,
                    selection,
                    self.font_size,
                    self.config.selection_color,
                );
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
            text_core::draw_cursor(
                renderer,
                content,
                self.state.cursor,
                self.font_size,
                self.config.cursor_color,
            );
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
                            return self.emit_change();
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
                            return self.emit_change();
                        }
                        return None;
                    }
                }

                // Check content area click
                if content.contains(x, y) || bounds.contains(x, y) {
                    let was_focused = self.state.is_focused;
                    self.state.is_focused = true;
                    let new_cursor = text_core::x_to_char_index(
                        x,
                        content.x,
                        self.font_size,
                        self.state.text.len(),
                    );

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

                    // Call on_undo_point when input gains focus (for undo tracking)
                    if !was_focused {
                        if let Some(ref on_undo_point) = self.on_undo_point {
                            log::debug!("NumberInput: calling on_undo_point (focus gained)");
                            on_undo_point();
                        }
                    }

                    return self.emit_change();
                } else if self.state.is_focused {
                    // Clicked outside - blur and validate
                    self.state.is_focused = false;
                    self.state.selection = None;
                    if let Some(value) = self.validate_value() {
                        log::debug!("NumberInput: blurred, validated value = {}", value);
                        return self.emit_change();
                    }
                }

                None
            }

            Event::TextInput { text } if self.state.is_focused => {
                let mut changed = false;
                for c in text.chars() {
                    // insert_char handles undo internally
                    if self.insert_char(c) {
                        changed = true;
                    }
                }
                if changed {
                    log::debug!("NumberInput: text input, value = '{}'", self.state.text);
                    return self.emit_change();
                }
                None
            }

            Event::KeyPress { key, modifiers, .. } if self.state.is_focused => {
                match key {
                    // Undo: Ctrl+Z
                    KeyCode::Z if modifiers.ctrl && !modifiers.shift => {
                        if self.state.undo() {
                            log::debug!("NumberInput: undo, value = '{}'", self.state.text);
                            return self.emit_change();
                        }
                    }
                    // Redo: Ctrl+Y or Ctrl+Shift+Z
                    KeyCode::Y if modifiers.ctrl => {
                        if self.state.redo() {
                            log::debug!("NumberInput: redo, value = '{}'", self.state.text);
                            return self.emit_change();
                        }
                    }
                    KeyCode::Z if modifiers.ctrl && modifiers.shift => {
                        if self.state.redo() {
                            log::debug!("NumberInput: redo (Ctrl+Shift+Z), value = '{}'", self.state.text);
                            return self.emit_change();
                        }
                    }
                    KeyCode::Backspace => {
                        // handle_backspace manages undo internally
                        if self.handle_backspace() {
                            log::debug!("NumberInput: backspace, value = '{}'", self.state.text);
                            return self.emit_change();
                        }
                    }
                    KeyCode::Delete => {
                        // handle_delete manages undo internally
                        if self.handle_delete() {
                            log::debug!("NumberInput: delete, value = '{}'", self.state.text);
                            return self.emit_change();
                        }
                    }
                    KeyCode::Left => {
                        let result = text_core::handle_left(
                            self.state.cursor,
                            self.state.selection,
                            modifiers.shift,
                        );
                        self.state.cursor = result.cursor;
                        self.state.selection = result.selection;
                    }
                    KeyCode::Right => {
                        let result = text_core::handle_right(
                            self.state.cursor,
                            self.state.selection,
                            self.state.text.len(),
                            modifiers.shift,
                        );
                        self.state.cursor = result.cursor;
                        self.state.selection = result.selection;
                    }
                    KeyCode::Home => {
                        let result = text_core::handle_home(
                            self.state.cursor,
                            self.state.selection,
                            modifiers.shift,
                        );
                        self.state.cursor = result.cursor;
                        self.state.selection = result.selection;
                    }
                    KeyCode::End => {
                        let result = text_core::handle_end(
                            self.state.cursor,
                            self.state.selection,
                            self.state.text.len(),
                            modifiers.shift,
                        );
                        self.state.cursor = result.cursor;
                        self.state.selection = result.selection;
                    }
                    KeyCode::A if modifiers.ctrl => {
                        let result = text_core::handle_select_all(self.state.text.len());
                        self.state.cursor = result.cursor;
                        self.state.selection = result.selection;
                    }
                    KeyCode::Up => {
                        if self.increment() {
                            log::debug!("NumberInput: up arrow increment, value = {}", self.state.text);
                            return self.emit_change();
                        }
                    }
                    KeyCode::Down => {
                        if self.decrement() {
                            log::debug!("NumberInput: down arrow decrement, value = {}", self.state.text);
                            return self.emit_change();
                        }
                    }
                    KeyCode::Enter | KeyCode::Escape => {
                        self.state.is_focused = false;
                        self.state.selection = None;
                        if let Some(value) = self.validate_value() {
                            log::debug!("NumberInput: enter/escape, validated value = {}", value);
                            return self.emit_change();
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
                        (0..steps.abs()).for_each(|_| { self.increment(); });
                    } else {
                        (0..steps.abs()).for_each(|_| { self.decrement(); });
                    }
                    return self.emit_change();
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
                        self.validate_value();
                        return self.emit_change();
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
                    self.validate_value();
                    return self.emit_change();
                }
                None
            }

            _ => None,
        }
    }
}
