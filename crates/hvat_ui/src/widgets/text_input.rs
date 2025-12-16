//! Text input widget for editable text fields

use crate::constants::{line_height, DEFAULT_FONT_SIZE, TEXT_INPUT_PADDING};
use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Length, Padding, Size};
use crate::renderer::{Color, Renderer};
use crate::state::TextInputState;
use crate::widget::Widget;
use crate::widgets::text_core;

/// Configuration for text input appearance
#[derive(Debug, Clone)]
pub struct TextInputConfig {
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
    /// Placeholder text color
    pub placeholder_color: Color,
    /// Cursor color
    pub cursor_color: Color,
    /// Selection background color
    pub selection_color: Color,
}

impl Default for TextInputConfig {
    fn default() -> Self {
        Self {
            background_color: Color::rgb(0.15, 0.15, 0.17),
            focused_background_color: Color::rgb(0.18, 0.18, 0.2),
            border_color: Color::BORDER,
            focused_border_color: Color::ACCENT,
            text_color: Color::TEXT_PRIMARY,
            placeholder_color: Color::TEXT_SECONDARY,
            cursor_color: Color::ACCENT,
            selection_color: Color::rgba(0.4, 0.6, 1.0, 0.3),
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
    on_change: Option<Box<dyn Fn(String, TextInputState) -> M>>,
    /// Callback for submit (Enter pressed)
    on_submit: Option<Box<dyn Fn(String) -> M>>,
    /// Side-effect callback for undo point (called when input gains focus)
    /// This is called BEFORE on_change, allowing the app to save a snapshot.
    on_undo_point: Option<Box<dyn Fn()>>,
}

impl<M> TextInput<M> {
    /// Create a new text input
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            state: TextInputState::default(),
            width: Length::Fill(1.0),
            height: Length::Shrink,
            padding: TEXT_INPUT_PADDING,
            font_size: DEFAULT_FONT_SIZE,
            config: TextInputConfig::default(),
            on_change: None,
            on_submit: None,
            on_undo_point: None,
        }
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

    /// Set the state
    pub fn state(mut self, state: &TextInputState) -> Self {
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
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Set the submit handler (called when Enter is pressed)
    pub fn on_submit<F>(mut self, callback: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.on_submit = Some(Box::new(callback));
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

    /// Get the content bounds (inside padding)
    fn content_bounds(&self, bounds: Bounds) -> Bounds {
        Bounds::new(
            bounds.x + self.padding.left,
            bounds.y + self.padding.top,
            bounds.width - self.padding.horizontal(),
            bounds.height - self.padding.vertical(),
        )
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
}

impl<M> Default for TextInput<M> {
    fn default() -> Self {
        Self::new()
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
                self.config.text_color,
            );
        }

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
                        text_core::x_to_char_index(x, content.x, self.font_size, self.value.len());

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
                        if let Some(ref on_undo_point) = self.on_undo_point {
                            log::debug!("TextInput: calling on_undo_point (focus gained)");
                            on_undo_point();
                        }
                    }

                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(self.value.clone(), self.state.clone()));
                    }
                } else if self.state.is_focused {
                    // Clicked outside - blur
                    self.state.is_focused = false;
                    self.state.selection = None;
                    log::debug!("TextInput: blurred");
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(self.value.clone(), self.state.clone()));
                    }
                }

                None
            }

            Event::TextInput { text } if self.state.is_focused => {
                self.insert_text(text);
                if let Some(ref on_change) = self.on_change {
                    return Some(on_change(self.value.clone(), self.state.clone()));
                }
                None
            }

            Event::KeyPress { key, modifiers, .. } if self.state.is_focused => {
                match key {
                    KeyCode::Backspace => {
                        if self.handle_backspace() {
                            log::debug!("TextInput: backspace, value = '{}'", self.value);
                            if let Some(ref on_change) = self.on_change {
                                return Some(on_change(self.value.clone(), self.state.clone()));
                            }
                        }
                    }
                    KeyCode::Delete => {
                        if self.handle_delete() {
                            log::debug!("TextInput: delete, value = '{}'", self.value);
                            if let Some(ref on_change) = self.on_change {
                                return Some(on_change(self.value.clone(), self.state.clone()));
                            }
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
                            self.value.len(),
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
                            self.value.len(),
                            modifiers.shift,
                        );
                        self.state.cursor = result.cursor;
                        self.state.selection = result.selection;
                    }
                    KeyCode::A if modifiers.ctrl => {
                        let result = text_core::handle_select_all(self.value.len());
                        self.state.cursor = result.cursor;
                        self.state.selection = result.selection;
                    }
                    // Note: Ctrl+Z/Y (undo/redo) is handled at application level via UndoStack<T>
                    KeyCode::Enter => {
                        if let Some(ref on_submit) = self.on_submit {
                            log::debug!("TextInput: submit '{}'", self.value);
                            return Some(on_submit(self.value.clone()));
                        }
                    }
                    KeyCode::Escape => {
                        self.state.is_focused = false;
                        self.state.selection = None;
                        log::debug!("TextInput: escape, blurred");
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(self.value.clone(), self.state.clone()));
                        }
                    }
                    _ => {}
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
                        log::debug!("TextInput: GlobalMousePress outside, blurred");
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(self.value.clone(), self.state.clone()));
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
                    log::debug!("TextInput: FocusLost, blurred");
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(self.value.clone(), self.state.clone()));
                    }
                }
                None
            }

            _ => None,
        }
    }
}
