//! Text input widget for editable text fields

use crate::callback::{Callback, SideEffect};
use crate::constants::{line_height, DEFAULT_FONT_SIZE, TEXT_INPUT_PADDING};
use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Length, Padding, Size};
use crate::renderer::{Color, Renderer};
use crate::state::TextInputState;
use crate::widget::Widget;
use crate::widgets::config::BaseInputConfig;
use crate::widgets::text_core;

/// Configuration for text input appearance
#[derive(Debug, Clone)]
pub struct TextInputConfig {
    /// Base input configuration (colors for background, border, cursor, selection)
    pub base: BaseInputConfig,
    /// Placeholder text color
    pub placeholder_color: Color,
}

impl Default for TextInputConfig {
    fn default() -> Self {
        Self {
            base: BaseInputConfig::default(),
            placeholder_color: Color::TEXT_SECONDARY,
        }
    }
}

/// A text input widget for editable text
pub struct TextInput<M> {
    /// Current text value
    value: String,
    /// Placeholder text
    placeholder: String,
    /// Widget state (cloned from external)
    state: TextInputState,
    /// Width
    width: Length,
    /// Height
    height: Length,
    /// Padding
    padding: Padding,
    /// Font size
    font_size: f32,
    /// Configuration
    config: TextInputConfig,
    /// Callback for value changes
    on_change: Callback<(String, TextInputState), M>,
    /// Callback for submit (Enter pressed)
    on_submit: Callback<String, M>,
    /// Side-effect callback for undo point (called when input gains focus)
    /// This is called BEFORE on_change, allowing the app to save a snapshot.
    on_undo_point: SideEffect,
}

impl<M> Default for TextInput<M> {
    fn default() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            state: TextInputState::default(),
            width: Length::Fill(1.0),
            height: Length::Shrink,
            padding: TEXT_INPUT_PADDING,
            font_size: DEFAULT_FONT_SIZE,
            config: TextInputConfig::default(),
            on_change: Callback::none(),
            on_submit: Callback::none(),
            on_undo_point: SideEffect::none(),
        }
    }
}

impl<M> TextInput<M> {
    /// Create a new text input
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current value
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    /// Set the placeholder text
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the state (copies the state)
    pub fn state(mut self, state: &TextInputState) -> Self {
        self.state = *state;
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

    /// Set the configuration
    pub fn config(mut self, config: TextInputConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the change handler
    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(String, TextInputState) -> M + 'static,
    {
        self.on_change = Callback::new(move |(value, state)| callback(value, state));
        self
    }

    /// Set the submit handler (called when Enter is pressed)
    pub fn on_submit<F>(mut self, callback: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.on_submit = Callback::new(callback);
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
        self.on_undo_point = SideEffect::new(callback);
        self
    }

    /// Get the content bounds (inside padding)
    fn content_bounds(&self, bounds: Bounds) -> Bounds {
        text_core::content_bounds(bounds, &self.padding)
    }

    /// Handle text insertion
    fn insert_text(&mut self, text: &str) {
        self.state.cursor =
            text_core::insert_text(&mut self.value, self.state.cursor, self.state.selection, text);
        self.state.selection = None;
    }

    /// Handle backspace
    fn handle_backspace(&mut self) -> bool {
        if let Some(new_cursor) = text_core::handle_backspace(
            &mut self.value,
            self.state.cursor,
            self.state.selection,
        ) {
            self.state.cursor = new_cursor;
            self.state.selection = None;
            true
        } else {
            false
        }
    }

    /// Handle delete
    fn handle_delete(&mut self) -> bool {
        if let Some(new_cursor) =
            text_core::handle_delete(&mut self.value, self.state.cursor, self.state.selection)
        {
            self.state.cursor = new_cursor;
            self.state.selection = None;
            true
        } else {
            false
        }
    }

    /// Emit a state change if handler is set
    fn emit_change(&self) -> Option<M> {
        self.on_change.call((self.value.clone(), self.state))
    }
}


impl<M: Clone + 'static> Widget<M> for TextInput<M> {
    fn layout(&mut self, available: Size) -> Size {
        let content_height = line_height(self.font_size);
        let min_height = content_height + self.padding.vertical();
        let min_width = 100.0;

        Size::new(
            self.width.resolve(available.width, min_width),
            self.height.resolve(available.height, min_height),
        )
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        let content = self.content_bounds(bounds);

        // Draw background and border
        text_core::draw_input_background(renderer, bounds, self.state.is_focused, &self.config.base);

        // Draw selection if present and focused
        if self.state.is_focused {
            if let Some(selection) = self.state.selection {
                text_core::draw_selection(
                    renderer,
                    content,
                    selection,
                    self.font_size,
                    self.config.base.selection_color,
                );
            }
        }

        // Draw text or placeholder
        let text_y = content.y + (content.height - self.font_size) / 2.0;
        if self.value.is_empty() {
            if !self.placeholder.is_empty() {
                renderer.text(
                    &self.placeholder,
                    content.x,
                    text_y,
                    self.font_size,
                    self.config.placeholder_color,
                );
            }
        } else {
            renderer.text(
                &self.value,
                content.x,
                text_y,
                self.font_size,
                self.config.base.text_color,
            );
        }

        // Draw cursor if focused
        if self.state.is_focused {
            text_core::draw_cursor(
                renderer,
                content,
                self.state.cursor,
                self.font_size,
                self.config.base.cursor_color,
            );
        }
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        let content = self.content_bounds(bounds);

        match event {
            Event::MousePress {
                button: MouseButton::Left,
                position,
                modifiers,
                ..
            } => {
                let (x, y) = *position;

                if bounds.contains(x, y) {
                    // Focus and position cursor
                    let was_focused = self.state.is_focused;
                    self.state.is_focused = true;
                    let new_cursor =
                        text_core::x_to_char_index(x, content.x, self.font_size, text_core::char_count(&self.value));

                    if modifiers.shift && was_focused {
                        // Extend selection
                        if let Some((start, _)) = self.state.selection {
                            self.state.selection = Some((start, new_cursor));
                        } else {
                            self.state.selection = Some((self.state.cursor, new_cursor));
                        }
                    } else {
                        self.state.cursor = new_cursor;
                        self.state.selection = None;
                    }

                    log::debug!("TextInput: clicked, cursor = {}", self.state.cursor);

                    // Call on_undo_point when input gains focus (for undo tracking)
                    if !was_focused {
                        log::debug!("TextInput: calling on_undo_point (focus gained)");
                        self.on_undo_point.emit();
                    }

                    return self.emit_change();
                } else if self.state.is_focused {
                    // Clicked outside - blur
                    self.state.is_focused = false;
                    self.state.selection = None;
                    log::debug!("TextInput: blurred");
                    return self.emit_change();
                }

                None
            }

            Event::TextInput { text } if self.state.is_focused => {
                self.insert_text(text);
                self.emit_change()
            }

            Event::KeyPress { key, modifiers, .. } if self.state.is_focused => {
                match key {
                    KeyCode::Backspace => {
                        if self.handle_backspace() {
                            log::debug!("TextInput: backspace, value = '{}'", self.value);
                            return self.emit_change();
                        }
                    }
                    KeyCode::Delete => {
                        if self.handle_delete() {
                            log::debug!("TextInput: delete, value = '{}'", self.value);
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
                            text_core::char_count(&self.value),
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
                            text_core::char_count(&self.value),
                            modifiers.shift,
                        );
                        self.state.cursor = result.cursor;
                        self.state.selection = result.selection;
                    }
                    KeyCode::A if modifiers.ctrl => {
                        let result = text_core::handle_select_all(text_core::char_count(&self.value));
                        self.state.cursor = result.cursor;
                        self.state.selection = result.selection;
                    }
                    // Note: Ctrl+Z/Y (undo/redo) is handled at application level via UndoStack<T>
                    KeyCode::Enter => {
                        log::debug!("TextInput: Enter pressed, value='{}', is_focused={}", self.value, self.state.is_focused);
                        if let Some(msg) = self.on_submit.call(self.value.clone()) {
                            log::debug!("TextInput: submit callback returned message");
                            return Some(msg);
                        } else {
                            log::debug!("TextInput: submit callback returned None (no handler?)");
                        }
                    }
                    KeyCode::Escape => {
                        self.state.is_focused = false;
                        self.state.selection = None;
                        log::debug!("TextInput: escape, blurred");
                        return self.emit_change();
                    }
                    _ => {}
                }

                None
            }

            Event::GlobalMousePress { position, .. } => {
                // Blur when clicking outside (but not inside - that's handled by MousePress)
                if text_core::handle_blur_on_outside_click(
                    &mut self.state.is_focused,
                    &mut self.state.selection,
                    *position,
                    bounds,
                ) {
                    log::debug!("TextInput: GlobalMousePress outside, blurred");
                    return self.emit_change();
                }
                None
            }

            Event::FocusLost => {
                // Blur when window loses focus
                if text_core::handle_focus_lost(&mut self.state.is_focused, &mut self.state.selection) {
                    log::debug!("TextInput: FocusLost, blurred");
                    return self.emit_change();
                }
                None
            }

            _ => None,
        }
    }
}
