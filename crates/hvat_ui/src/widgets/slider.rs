//! Slider widget for selecting numeric values

use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer};
use crate::state::SliderState;
use crate::widget::Widget;

/// Default slider height
const DEFAULT_HEIGHT: f32 = 24.0;
/// Track height
const TRACK_HEIGHT: f32 = 4.0;
/// Thumb radius
const THUMB_RADIUS: f32 = 8.0;
/// Default font size
const FONT_SIZE: f32 = 12.0;
/// Input field width
const INPUT_WIDTH: f32 = 60.0;
/// Input field padding
const INPUT_PADDING: f32 = 4.0;
/// Spacing between slider and input
const INPUT_SPACING: f32 = 8.0;
/// Cursor width for input
const CURSOR_WIDTH: f32 = 1.0;
/// Character width approximation
const CHAR_WIDTH_FACTOR: f32 = 0.6;

/// Configuration for slider appearance
#[derive(Debug, Clone)]
pub struct SliderConfig {
    /// Track background color
    pub track_color: Color,
    /// Filled portion of track color
    pub track_fill_color: Color,
    /// Thumb color
    pub thumb_color: Color,
    /// Thumb hover color
    pub thumb_hover_color: Color,
    /// Thumb active color (when dragging)
    pub thumb_active_color: Color,
    /// Border color
    pub border_color: Color,
    /// Label color
    pub label_color: Color,
    /// Input background color
    pub input_background_color: Color,
    /// Input focused background color
    pub input_focused_background_color: Color,
    /// Input text color
    pub input_text_color: Color,
    /// Input cursor color
    pub input_cursor_color: Color,
    /// Input selection color
    pub input_selection_color: Color,
}

impl Default for SliderConfig {
    fn default() -> Self {
        Self {
            track_color: Color::rgb(0.2, 0.2, 0.24),
            track_fill_color: Color::ACCENT,
            thumb_color: Color::rgb(0.9, 0.9, 0.92),
            thumb_hover_color: Color::rgb(1.0, 1.0, 1.0),
            thumb_active_color: Color::ACCENT,
            border_color: Color::BORDER,
            label_color: Color::TEXT_SECONDARY,
            input_background_color: Color::rgb(0.15, 0.15, 0.17),
            input_focused_background_color: Color::rgb(0.18, 0.18, 0.2),
            input_text_color: Color::TEXT_PRIMARY,
            input_cursor_color: Color::ACCENT,
            input_selection_color: Color::rgba(0.4, 0.6, 1.0, 0.3),
        }
    }
}

/// A slider widget for selecting numeric values
pub struct Slider<M> {
    /// Minimum value
    min: f32,
    /// Maximum value
    max: f32,
    /// Step size (None for continuous)
    step: Option<f32>,
    /// Widget state (cloned from external)
    state: SliderState,
    /// Width
    width: Length,
    /// Height
    height: Length,
    /// Whether the thumb is hovered
    hovered: bool,
    /// Whether to show value label (above thumb)
    show_value: bool,
    /// Whether to show editable input field
    show_input: bool,
    /// Value format function (for custom display)
    format_value: Option<Box<dyn Fn(f32) -> String>>,
    /// Configuration
    config: SliderConfig,
    /// Callback for value changes
    on_change: Option<Box<dyn Fn(SliderState) -> M>>,
}

impl<M> Slider<M> {
    /// Create a new slider with min and max values
    pub fn new(min: f32, max: f32) -> Self {
        Self {
            min,
            max,
            step: None,
            state: SliderState::default(),
            width: Length::Fill(1.0),
            height: Length::Fixed(DEFAULT_HEIGHT),
            hovered: false,
            show_value: false,
            show_input: false,
            format_value: None,
            config: SliderConfig::default(),
            on_change: None,
        }
    }

    /// Set the state
    pub fn state(mut self, state: &SliderState) -> Self {
        self.state = state.clone();
        self
    }

    /// Set the step size
    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step);
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

    /// Show the current value label (above thumb)
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Show an editable input field next to the slider
    pub fn show_input(mut self, show: bool) -> Self {
        self.show_input = show;
        self
    }

    /// Set custom value formatting
    pub fn format<F>(mut self, f: F) -> Self
    where
        F: Fn(f32) -> String + 'static,
    {
        self.format_value = Some(Box::new(f));
        self
    }

    /// Set the configuration
    pub fn config(mut self, config: SliderConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the change handler
    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(SliderState) -> M + 'static,
    {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Convert a position to a value
    fn position_to_value(&self, x: f32, track_bounds: &Bounds) -> f32 {
        let progress = ((x - track_bounds.x) / track_bounds.width).clamp(0.0, 1.0);
        let mut value = self.min + progress * (self.max - self.min);

        // Snap to step if configured
        if let Some(step) = self.step {
            value = ((value - self.min) / step).round() * step + self.min;
        }

        value.clamp(self.min, self.max)
    }

    /// Convert a value to a position (0.0 to 1.0)
    fn value_to_progress(&self) -> f32 {
        if (self.max - self.min).abs() < f32::EPSILON {
            return 0.0;
        }
        ((self.state.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    /// Format value for display (label)
    fn format_current_value(&self) -> String {
        if let Some(ref format) = self.format_value {
            format(self.state.value)
        } else if let Some(step) = self.step {
            // Determine decimal places from step
            let decimals = if step >= 1.0 {
                0
            } else {
                ((-step.log10()).ceil() as usize).min(3)
            };
            format!("{:.decimals$}", self.state.value, decimals = decimals)
        } else {
            format!("{:.1}", self.state.value)
        }
    }

    /// Format value for input field (always editable format)
    fn format_value_for_input(&self, value: f32) -> String {
        // If it's close to an integer, display as integer
        if (value - value.round()).abs() < 0.0001 {
            format!("{}", value.round() as i32)
        } else {
            // Otherwise display with up to 3 decimal places, trimming trailing zeros
            let formatted = format!("{:.3}", value);
            formatted.trim_end_matches('0').trim_end_matches('.').to_string()
        }
    }

    /// Get slider bounds (excludes input area if shown)
    fn slider_bounds(&self, bounds: Bounds) -> Bounds {
        if self.show_input {
            Bounds::new(
                bounds.x,
                bounds.y,
                bounds.width - INPUT_WIDTH - INPUT_SPACING,
                bounds.height,
            )
        } else {
            bounds
        }
    }

    /// Get input field bounds
    fn input_bounds(&self, bounds: Bounds) -> Option<Bounds> {
        if self.show_input {
            Some(Bounds::new(
                bounds.x + bounds.width - INPUT_WIDTH,
                bounds.y + (bounds.height - DEFAULT_HEIGHT) / 2.0,
                INPUT_WIDTH,
                DEFAULT_HEIGHT,
            ))
        } else {
            None
        }
    }

    /// Get track bounds (accounting for thumb radius)
    fn track_bounds(&self, slider_bounds: Bounds) -> Bounds {
        let padding = THUMB_RADIUS;
        let track_y = slider_bounds.y + (slider_bounds.height - TRACK_HEIGHT) / 2.0;
        Bounds::new(
            slider_bounds.x + padding,
            track_y,
            slider_bounds.width - padding * 2.0,
            TRACK_HEIGHT,
        )
    }

    /// Get thumb bounds
    fn thumb_bounds(&self, slider_bounds: Bounds) -> Bounds {
        let track = self.track_bounds(slider_bounds);
        let progress = self.value_to_progress();
        let thumb_x = track.x + progress * track.width;
        let thumb_y = slider_bounds.y + slider_bounds.height / 2.0;
        Bounds::new(
            thumb_x - THUMB_RADIUS,
            thumb_y - THUMB_RADIUS,
            THUMB_RADIUS * 2.0,
            THUMB_RADIUS * 2.0,
        )
    }

    /// Handle input field character insertion
    fn input_insert_char(&mut self, c: char) -> bool {
        // Only allow digits, minus, and period
        if !c.is_ascii_digit() && c != '-' && c != '.' {
            return false;
        }

        // Minus only at start
        if c == '-' && self.state.input_cursor != 0 {
            return false;
        }

        // Only one period
        if c == '.' && self.state.input_text.contains('.') {
            return false;
        }

        // Push undo state before making changes
        self.state.push_text_undo();

        // If there's a selection, delete it first
        if let Some((start, end)) = self.state.input_selection {
            let (start, end) = (start.min(end), start.max(end));
            self.state.input_text.drain(start..end);
            self.state.input_cursor = start;
            self.state.input_selection = None;
        }

        self.state.input_text.insert(self.state.input_cursor, c);
        self.state.input_cursor += 1;
        true
    }

    /// Handle input field backspace
    fn input_handle_backspace(&mut self) -> bool {
        if let Some((start, end)) = self.state.input_selection {
            // Push undo state before making changes
            self.state.push_text_undo();
            let (start, end) = (start.min(end), start.max(end));
            self.state.input_text.drain(start..end);
            self.state.input_cursor = start;
            self.state.input_selection = None;
            true
        } else if self.state.input_cursor > 0 {
            // Push undo state before making changes
            self.state.push_text_undo();
            self.state.input_text.remove(self.state.input_cursor - 1);
            self.state.input_cursor -= 1;
            true
        } else {
            false
        }
    }

    /// Handle input field delete
    fn input_handle_delete(&mut self) -> bool {
        if let Some((start, end)) = self.state.input_selection {
            // Push undo state before making changes
            self.state.push_text_undo();
            let (start, end) = (start.min(end), start.max(end));
            self.state.input_text.drain(start..end);
            self.state.input_cursor = start;
            self.state.input_selection = None;
            true
        } else if self.state.input_cursor < self.state.input_text.len() {
            // Push undo state before making changes
            self.state.push_text_undo();
            self.state.input_text.remove(self.state.input_cursor);
            true
        } else {
            false
        }
    }

    /// Parse input text and update value
    fn apply_input_value(&mut self) -> bool {
        if let Ok(value) = self.state.input_text.parse::<f32>() {
            let clamped = value.clamp(self.min, self.max);
            if (clamped - self.state.value).abs() > f32::EPSILON {
                self.state.value = clamped;
                self.state.input_text = self.format_value_for_input(clamped);
                self.state.input_cursor = self.state.input_text.len();
                return true;
            } else {
                // Value same but text may differ - update text to canonical form
                self.state.input_text = self.format_value_for_input(clamped);
                self.state.input_cursor = self.state.input_text.len();
            }
        } else {
            // Invalid input - revert to current value
            self.state.input_text = self.format_value_for_input(self.state.value);
            self.state.input_cursor = self.state.input_text.len();
        }
        false
    }

    /// Sync input text from slider value
    fn sync_input_from_value(&mut self) {
        if !self.state.input_focused {
            self.state.input_text = self.format_value_for_input(self.state.value);
            self.state.input_cursor = self.state.input_text.len();
        }
    }

    /// Calculate character width for input field
    fn input_char_width(&self) -> f32 {
        FONT_SIZE * CHAR_WIDTH_FACTOR
    }

    /// Convert x position to character index in input
    fn input_x_to_char_index(&self, x: f32, input_bounds: &Bounds) -> usize {
        let content_x = input_bounds.x + INPUT_PADDING;
        let relative_x = x - content_x;
        let char_width = self.input_char_width();
        let index = (relative_x / char_width).round() as i32;
        index.clamp(0, self.state.input_text.len() as i32) as usize
    }
}

impl<M: Clone + 'static> Widget<M> for Slider<M> {
    fn layout(&mut self, available: Size) -> Size {
        let width = self.width.resolve(available.width, 200.0);
        let height = self.height.resolve(available.height, DEFAULT_HEIGHT);
        Size::new(width, height)
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        let slider_bounds = self.slider_bounds(bounds);
        let track = self.track_bounds(slider_bounds);
        let progress = self.value_to_progress();

        // Draw track background
        renderer.fill_rect(track, self.config.track_color);

        // Draw filled portion
        let filled = Bounds::new(track.x, track.y, track.width * progress, track.height);
        renderer.fill_rect(filled, self.config.track_fill_color);

        // Draw track border
        renderer.stroke_rect(track, self.config.border_color, 1.0);

        // Draw thumb
        let thumb = self.thumb_bounds(slider_bounds);
        let thumb_color = if self.state.dragging {
            self.config.thumb_active_color
        } else if self.hovered {
            self.config.thumb_hover_color
        } else {
            self.config.thumb_color
        };

        let cx = thumb.x + thumb.width / 2.0;
        let cy = thumb.y + thumb.height / 2.0;
        renderer.fill_circle(cx, cy, THUMB_RADIUS, thumb_color);

        // Draw value label if enabled (above thumb)
        if self.show_value {
            let text = self.format_current_value();
            let text_x = thumb.x + THUMB_RADIUS - (text.len() as f32 * FONT_SIZE * 0.3);
            let text_y = slider_bounds.y + 2.0;
            renderer.text(&text, text_x, text_y, FONT_SIZE, self.config.label_color);
        }

        // Draw input field if enabled
        if let Some(input_bounds) = self.input_bounds(bounds) {
            // Background
            let bg_color = if self.state.input_focused {
                self.config.input_focused_background_color
            } else {
                self.config.input_background_color
            };
            renderer.fill_rect(input_bounds, bg_color);

            // Border
            let border_color = if self.state.input_focused {
                self.config.track_fill_color
            } else {
                self.config.border_color
            };
            renderer.stroke_rect(input_bounds, border_color, 1.0);

            // Content area
            let content_x = input_bounds.x + INPUT_PADDING;
            let content_y = input_bounds.y + (input_bounds.height - FONT_SIZE) / 2.0;
            let content_width = input_bounds.width - INPUT_PADDING * 2.0;

            // Selection
            if self.state.input_focused {
                if let Some((start, end)) = self.state.input_selection {
                    let (start, end) = (start.min(end), start.max(end));
                    let char_width = self.input_char_width();
                    let sel_x = content_x + start as f32 * char_width;
                    let sel_width = (end - start) as f32 * char_width;
                    let sel_bounds = Bounds::new(
                        sel_x,
                        input_bounds.y + 2.0,
                        sel_width.min(content_width),
                        input_bounds.height - 4.0,
                    );
                    renderer.fill_rect(sel_bounds, self.config.input_selection_color);
                }
            }

            // Text
            renderer.text(
                &self.state.input_text,
                content_x,
                content_y,
                FONT_SIZE,
                self.config.input_text_color,
            );

            // Cursor
            if self.state.input_focused {
                let cursor_x = content_x + self.state.input_cursor as f32 * self.input_char_width();
                let cursor_bounds = Bounds::new(
                    cursor_x,
                    input_bounds.y + 4.0,
                    CURSOR_WIDTH,
                    input_bounds.height - 8.0,
                );
                renderer.fill_rect(cursor_bounds, self.config.input_cursor_color);
            }
        }
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        let slider_bounds = self.slider_bounds(bounds);
        let track = self.track_bounds(slider_bounds);
        let thumb = self.thumb_bounds(slider_bounds);
        let input_bounds = self.input_bounds(bounds);

        match event {
            Event::MouseMove { position, .. } => {
                let (x, y) = *position;

                // Check if hovering thumb
                let thumb_center_x = thumb.x + THUMB_RADIUS;
                let thumb_center_y = thumb.y + THUMB_RADIUS;
                let dist_sq = (x - thumb_center_x).powi(2) + (y - thumb_center_y).powi(2);
                self.hovered = dist_sq <= (THUMB_RADIUS * 1.5).powi(2);

                // Handle drag
                if self.state.dragging {
                    let new_value = self.position_to_value(x, &track);
                    if (new_value - self.state.value).abs() > f32::EPSILON {
                        self.state.value = new_value;
                        self.sync_input_from_value();
                        log::debug!("Slider drag: value = {}", new_value);
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(self.state.clone()));
                        }
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

                // Check input field click first
                if let Some(ib) = &input_bounds {
                    if ib.contains(x, y) {
                        let was_focused = self.state.input_focused;
                        self.state.input_focused = true;
                        let new_cursor = self.input_x_to_char_index(x, ib);

                        if modifiers.shift && was_focused {
                            if let Some((start, _)) = self.state.input_selection {
                                self.state.input_selection = Some((start, new_cursor));
                            } else {
                                self.state.input_selection = Some((self.state.input_cursor, new_cursor));
                            }
                        } else {
                            self.state.input_cursor = new_cursor;
                            if !was_focused && !self.state.input_text.is_empty() {
                                // Select all on focus
                                self.state.input_selection = Some((0, self.state.input_text.len()));
                                self.state.input_cursor = self.state.input_text.len();
                            } else {
                                self.state.input_selection = None;
                            }
                        }

                        log::debug!("Slider input: clicked, cursor = {}", self.state.input_cursor);
                        // Emit message to trigger redraw (selection highlight)
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(self.state.clone()));
                        }
                        return None;
                    }
                }

                // Blur input if clicking elsewhere
                if self.state.input_focused {
                    self.state.input_focused = false;
                    self.state.input_selection = None;
                    self.apply_input_value();
                    log::debug!("Slider input: blurred by clicking elsewhere");
                    // Always emit to trigger redraw (clears visual focus)
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(self.state.clone()));
                    }
                }

                // Check if clicking on thumb
                let thumb_center_x = thumb.x + THUMB_RADIUS;
                let thumb_center_y = thumb.y + THUMB_RADIUS;
                let dist_sq = (x - thumb_center_x).powi(2) + (y - thumb_center_y).powi(2);

                if dist_sq <= (THUMB_RADIUS * 1.5).powi(2) {
                    self.state.dragging = true;
                    log::debug!("Slider: started dragging");
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(self.state.clone()));
                    }
                } else if slider_bounds.contains(x, y) {
                    let new_value = self.position_to_value(x, &track);
                    self.state.value = new_value;
                    self.state.dragging = true;
                    self.sync_input_from_value();
                    log::debug!("Slider: clicked track, value = {}", new_value);
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(self.state.clone()));
                    }
                }

                None
            }

            Event::MouseRelease {
                button: MouseButton::Left,
                ..
            } => {
                if self.state.dragging {
                    self.state.dragging = false;
                    log::debug!("Slider: stopped dragging");
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(self.state.clone()));
                    }
                }
                None
            }

            Event::TextInput { text } if self.state.input_focused => {
                let mut changed = false;
                for c in text.chars() {
                    if self.input_insert_char(c) {
                        changed = true;
                    }
                }
                if changed {
                    // Try to parse and update value live
                    if let Ok(value) = self.state.input_text.parse::<f32>() {
                        let clamped = value.clamp(self.min, self.max);
                        if (clamped - self.state.value).abs() > f32::EPSILON {
                            self.state.value = clamped;
                            log::debug!("Slider input: live update value = {}", clamped);
                            if let Some(ref on_change) = self.on_change {
                                return Some(on_change(self.state.clone()));
                            }
                        }
                    }
                }
                None
            }

            Event::KeyPress { key, modifiers, .. } => {
                // Handle input field keys
                if self.state.input_focused {
                    match key {
                        KeyCode::Backspace => {
                            if self.input_handle_backspace() {
                                if let Ok(value) = self.state.input_text.parse::<f32>() {
                                    let clamped = value.clamp(self.min, self.max);
                                    self.state.value = clamped;
                                    if let Some(ref on_change) = self.on_change {
                                        return Some(on_change(self.state.clone()));
                                    }
                                }
                            }
                        }
                        KeyCode::Delete => {
                            if self.input_handle_delete() {
                                if let Ok(value) = self.state.input_text.parse::<f32>() {
                                    let clamped = value.clamp(self.min, self.max);
                                    self.state.value = clamped;
                                    if let Some(ref on_change) = self.on_change {
                                        return Some(on_change(self.state.clone()));
                                    }
                                }
                            }
                        }
                        KeyCode::Left => {
                            if modifiers.shift {
                                if self.state.input_cursor > 0 {
                                    let anchor = self.state.input_selection.map(|(s, _)| s).unwrap_or(self.state.input_cursor);
                                    self.state.input_cursor -= 1;
                                    self.state.input_selection = Some((anchor, self.state.input_cursor));
                                }
                            } else if self.state.input_selection.is_some() {
                                let (start, end) = self.state.input_selection.unwrap();
                                self.state.input_cursor = start.min(end);
                                self.state.input_selection = None;
                            } else if self.state.input_cursor > 0 {
                                self.state.input_cursor -= 1;
                            }
                        }
                        KeyCode::Right => {
                            if modifiers.shift {
                                if self.state.input_cursor < self.state.input_text.len() {
                                    let anchor = self.state.input_selection.map(|(s, _)| s).unwrap_or(self.state.input_cursor);
                                    self.state.input_cursor += 1;
                                    self.state.input_selection = Some((anchor, self.state.input_cursor));
                                }
                            } else if self.state.input_selection.is_some() {
                                let (start, end) = self.state.input_selection.unwrap();
                                self.state.input_cursor = start.max(end);
                                self.state.input_selection = None;
                            } else if self.state.input_cursor < self.state.input_text.len() {
                                self.state.input_cursor += 1;
                            }
                        }
                        KeyCode::Home => {
                            if modifiers.shift {
                                let anchor = self.state.input_selection.map(|(s, _)| s).unwrap_or(self.state.input_cursor);
                                self.state.input_cursor = 0;
                                self.state.input_selection = Some((anchor, 0));
                            } else {
                                self.state.input_cursor = 0;
                                self.state.input_selection = None;
                            }
                        }
                        KeyCode::End => {
                            if modifiers.shift {
                                let anchor = self.state.input_selection.map(|(s, _)| s).unwrap_or(self.state.input_cursor);
                                self.state.input_cursor = self.state.input_text.len();
                                self.state.input_selection = Some((anchor, self.state.input_cursor));
                            } else {
                                self.state.input_cursor = self.state.input_text.len();
                                self.state.input_selection = None;
                            }
                        }
                        KeyCode::A if modifiers.ctrl => {
                            self.state.input_selection = Some((0, self.state.input_text.len()));
                            self.state.input_cursor = self.state.input_text.len();
                        }
                        KeyCode::Z if modifiers.ctrl && modifiers.shift => {
                            // Ctrl+Shift+Z = Redo
                            if self.state.text_redo() {
                                log::debug!("Slider input: text redo");
                                // Try to update slider value from new text
                                if let Ok(value) = self.state.input_text.parse::<f32>() {
                                    self.state.value = value.clamp(self.min, self.max);
                                }
                                if let Some(ref on_change) = self.on_change {
                                    return Some(on_change(self.state.clone()));
                                }
                            }
                            return None;
                        }
                        KeyCode::Z if modifiers.ctrl => {
                            // Ctrl+Z = Undo
                            if self.state.text_undo() {
                                log::debug!("Slider input: text undo");
                                // Try to update slider value from new text
                                if let Ok(value) = self.state.input_text.parse::<f32>() {
                                    self.state.value = value.clamp(self.min, self.max);
                                }
                                if let Some(ref on_change) = self.on_change {
                                    return Some(on_change(self.state.clone()));
                                }
                            }
                            return None;
                        }
                        KeyCode::Y if modifiers.ctrl => {
                            // Ctrl+Y = Redo (Windows style)
                            if self.state.text_redo() {
                                log::debug!("Slider input: text redo (Ctrl+Y)");
                                // Try to update slider value from new text
                                if let Ok(value) = self.state.input_text.parse::<f32>() {
                                    self.state.value = value.clamp(self.min, self.max);
                                }
                                if let Some(ref on_change) = self.on_change {
                                    return Some(on_change(self.state.clone()));
                                }
                            }
                            return None;
                        }
                        KeyCode::Enter | KeyCode::Escape => {
                            self.state.input_focused = false;
                            self.state.input_selection = None;
                            self.apply_input_value();
                            log::debug!("Slider input: confirmed value = {}", self.state.value);
                            // Always emit to trigger redraw (clears visual selection)
                            if let Some(ref on_change) = self.on_change {
                                return Some(on_change(self.state.clone()));
                            }
                        }
                        KeyCode::Up => {
                            let step = self.step.unwrap_or((self.max - self.min) / 100.0);
                            let new_value = (self.state.value + step).clamp(self.min, self.max);
                            if (new_value - self.state.value).abs() > f32::EPSILON {
                                self.state.value = new_value;
                                self.state.input_text = self.format_value_for_input(new_value);
                                self.state.input_cursor = self.state.input_text.len();
                                self.state.input_selection = Some((0, self.state.input_text.len()));
                                if let Some(ref on_change) = self.on_change {
                                    return Some(on_change(self.state.clone()));
                                }
                            }
                        }
                        KeyCode::Down => {
                            let step = self.step.unwrap_or((self.max - self.min) / 100.0);
                            let new_value = (self.state.value - step).clamp(self.min, self.max);
                            if (new_value - self.state.value).abs() > f32::EPSILON {
                                self.state.value = new_value;
                                self.state.input_text = self.format_value_for_input(new_value);
                                self.state.input_cursor = self.state.input_text.len();
                                self.state.input_selection = Some((0, self.state.input_text.len()));
                                if let Some(ref on_change) = self.on_change {
                                    return Some(on_change(self.state.clone()));
                                }
                            }
                        }
                        _ => {}
                    }
                    return None;
                }

                // Handle slider keys (when hovered)
                if !self.hovered {
                    return None;
                }

                let step = self.step.unwrap_or((self.max - self.min) / 100.0);
                let big_step = step * 10.0;

                let delta = match key {
                    KeyCode::Left | KeyCode::Down => Some(-step),
                    KeyCode::Right | KeyCode::Up => Some(step),
                    KeyCode::PageDown => Some(-big_step),
                    KeyCode::PageUp => Some(big_step),
                    KeyCode::Home => {
                        self.state.value = self.min;
                        self.sync_input_from_value();
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(self.state.clone()));
                        }
                        return None;
                    }
                    KeyCode::End => {
                        self.state.value = self.max;
                        self.sync_input_from_value();
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(self.state.clone()));
                        }
                        return None;
                    }
                    _ => None,
                };

                if let Some(d) = delta {
                    let multiplier = if modifiers.shift { 0.1 } else { 1.0 };
                    let new_value = (self.state.value + d * multiplier).clamp(self.min, self.max);
                    if (new_value - self.state.value).abs() > f32::EPSILON {
                        self.state.value = new_value;
                        self.sync_input_from_value();
                        log::debug!("Slider key: value = {}", new_value);
                        if let Some(ref on_change) = self.on_change {
                            return Some(on_change(self.state.clone()));
                        }
                    }
                }

                None
            }

            Event::MouseScroll {
                delta, position, ..
            } => {
                // Handle scroll on input field
                if let Some(ib) = &input_bounds {
                    if ib.contains(position.0, position.1) {
                        let step = self.step.unwrap_or((self.max - self.min) / 100.0);
                        let scroll_delta = delta.1.signum() * step;
                        let new_value = (self.state.value + scroll_delta).clamp(self.min, self.max);
                        if (new_value - self.state.value).abs() > f32::EPSILON {
                            self.state.value = new_value;
                            self.sync_input_from_value();
                            if let Some(ref on_change) = self.on_change {
                                return Some(on_change(self.state.clone()));
                            }
                        }
                        return None;
                    }
                }

                // Handle scroll on slider
                if !slider_bounds.contains(position.0, position.1) {
                    return None;
                }

                let step = self.step.unwrap_or((self.max - self.min) / 100.0);
                let scroll_delta = delta.1 * step;
                let new_value = (self.state.value + scroll_delta).clamp(self.min, self.max);

                if (new_value - self.state.value).abs() > f32::EPSILON {
                    self.state.value = new_value;
                    self.sync_input_from_value();
                    log::debug!("Slider scroll: value = {}", new_value);
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(self.state.clone()));
                    }
                }

                None
            }

            Event::FocusLost => {
                // Window lost focus - blur input if focused
                if self.state.input_focused {
                    self.state.input_focused = false;
                    self.state.input_selection = None;
                    self.apply_input_value();
                    log::debug!("Slider input: blurred due to window focus loss");
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(self.state.clone()));
                    }
                }
                None
            }

            Event::GlobalMousePress { position, .. } => {
                // Global click - blur input if focused and click is outside our input bounds
                if self.state.input_focused {
                    let (x, y) = *position;
                    // Check if click is outside input bounds
                    if let Some(ib) = &input_bounds {
                        if !ib.contains(x, y) {
                            self.state.input_focused = false;
                            self.state.input_selection = None;
                            self.apply_input_value();
                            log::debug!("Slider input: blurred by global click outside");
                            if let Some(ref on_change) = self.on_change {
                                return Some(on_change(self.state.clone()));
                            }
                        }
                    }
                }
                None
            }

            Event::CursorLeft => {
                // Cursor left window - release any drag state
                if self.state.dragging {
                    self.state.dragging = false;
                    log::debug!("Slider: stopped dragging (cursor left window)");
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(self.state.clone()));
                    }
                }
                None
            }

            _ => None,
        }
    }

    fn has_active_drag(&self) -> bool {
        self.state.dragging
    }
}
