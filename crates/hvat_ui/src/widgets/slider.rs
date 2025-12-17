//! Slider widget for selecting numeric values

use crate::constants::{
    format_number, BIG_STEP_MULTIPLIER, DEFAULT_STEP_DIVIDER, FINE_STEP_MULTIPLIER,
    SLIDER_HEIGHT, SLIDER_INPUT_PADDING, SLIDER_INPUT_SPACING, SLIDER_INPUT_WIDTH,
    SLIDER_THUMB_RADIUS, SLIDER_TRACK_HEIGHT, SMALL_FONT_SIZE, THUMB_HIT_AREA_MULTIPLIER,
};
use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer};
use crate::state::SliderState;
use crate::widget::Widget;
use crate::widgets::config::BaseInputConfig;
use crate::widgets::text_core;

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
    /// Input field configuration (reuses BaseInputConfig)
    pub input: BaseInputConfig,
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
            input: BaseInputConfig::default(),
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
    /// Side-effect callback for undo point (called when edit starts: drag begins or input focuses)
    /// This is called BEFORE on_change, allowing the app to save a snapshot of current state.
    on_undo_point: Option<Box<dyn Fn()>>,
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
            height: Length::Fixed(SLIDER_HEIGHT),
            hovered: false,
            show_value: false,
            show_input: false,
            format_value: None,
            config: SliderConfig::default(),
            on_change: None,
            on_undo_point: None,
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

    /// Set the undo point handler (called when drag starts or input field gains focus)
    ///
    /// This is a side-effect callback invoked at the start of an edit operation,
    /// BEFORE `on_change` is called. Use this to save an undo snapshot of the current
    /// state before the edit begins.
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
        format_number(value)
    }

    /// Get slider bounds (excludes input area if shown)
    fn slider_bounds(&self, bounds: Bounds) -> Bounds {
        if self.show_input {
            Bounds::new(
                bounds.x,
                bounds.y,
                bounds.width - SLIDER_INPUT_WIDTH - SLIDER_INPUT_SPACING,
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
                bounds.x + bounds.width - SLIDER_INPUT_WIDTH,
                bounds.y + (bounds.height - SLIDER_HEIGHT) / 2.0,
                SLIDER_INPUT_WIDTH,
                SLIDER_HEIGHT,
            ))
        } else {
            None
        }
    }

    /// Get track bounds (accounting for thumb radius)
    fn track_bounds(&self, slider_bounds: Bounds) -> Bounds {
        let padding = SLIDER_THUMB_RADIUS;
        let track_y = slider_bounds.y + (slider_bounds.height - SLIDER_TRACK_HEIGHT) / 2.0;
        Bounds::new(
            slider_bounds.x + padding,
            track_y,
            slider_bounds.width - padding * 2.0,
            SLIDER_TRACK_HEIGHT,
        )
    }

    /// Get thumb bounds
    fn thumb_bounds(&self, slider_bounds: Bounds) -> Bounds {
        let track = self.track_bounds(slider_bounds);
        let progress = self.value_to_progress();
        let thumb_x = track.x + progress * track.width;
        let thumb_y = slider_bounds.y + slider_bounds.height / 2.0;
        Bounds::new(
            thumb_x - SLIDER_THUMB_RADIUS,
            thumb_y - SLIDER_THUMB_RADIUS,
            SLIDER_THUMB_RADIUS * 2.0,
            SLIDER_THUMB_RADIUS * 2.0,
        )
    }

    /// Handle input field character insertion
    fn input_insert_char(&mut self, c: char) -> bool {
        // Use text_core validation for number chars
        if !text_core::is_valid_number_char(c, self.state.input_cursor, &self.state.input_text) {
            return false;
        }

        // Push undo state before making changes
        self.state.push_text_undo();

        // Insert text using text_core (handles selection deletion)
        self.state.input_cursor = text_core::insert_text(
            &mut self.state.input_text,
            self.state.input_cursor,
            self.state.input_selection,
            &c.to_string(),
        );
        self.state.input_selection = None;
        true
    }

    /// Handle input field backspace (with undo support)
    fn input_handle_backspace(&mut self) -> bool {
        // Check if backspace would do anything
        let would_modify = self.state.input_selection.is_some()
            || (self.state.input_cursor > 0 && !self.state.input_text.is_empty());

        if would_modify {
            // Push undo BEFORE making changes
            self.state.push_text_undo();

            if let Some(new_cursor) = text_core::handle_backspace(
                &mut self.state.input_text,
                self.state.input_cursor,
                self.state.input_selection,
            ) {
                self.state.input_cursor = new_cursor;
                self.state.input_selection = None;
                return true;
            }
        }
        false
    }

    /// Handle input field delete (with undo support)
    fn input_handle_delete(&mut self) -> bool {
        // Check if delete would do anything
        let would_modify = self.state.input_selection.is_some()
            || self.state.input_cursor < self.state.input_text.len();

        if would_modify {
            // Push undo BEFORE making changes
            self.state.push_text_undo();

            if let Some(new_cursor) = text_core::handle_delete(
                &mut self.state.input_text,
                self.state.input_cursor,
                self.state.input_selection,
            ) {
                self.state.input_cursor = new_cursor;
                self.state.input_selection = None;
                return true;
            }
        }
        false
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

    /// Emit a state change if handler is set
    fn emit_change(&self) -> Option<M> {
        self.on_change.as_ref().map(|f| f(self.state.clone()))
    }
}

impl<M: Clone + 'static> Widget<M> for Slider<M> {
    fn layout(&mut self, available: Size) -> Size {
        let width = self.width.resolve(available.width, 200.0);
        let height = self.height.resolve(available.height, SLIDER_HEIGHT);
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
        let thumb_color = if self.state.drag.is_dragging() {
            self.config.thumb_active_color
        } else if self.hovered {
            self.config.thumb_hover_color
        } else {
            self.config.thumb_color
        };

        let cx = thumb.x + thumb.width / 2.0;
        let cy = thumb.y + thumb.height / 2.0;
        renderer.fill_circle(cx, cy, SLIDER_THUMB_RADIUS, thumb_color);

        // Draw value label if enabled (above thumb)
        if self.show_value {
            let text = self.format_current_value();
            let text_x = thumb.x + SLIDER_THUMB_RADIUS - (text.len() as f32 * SMALL_FONT_SIZE * 0.3);
            let text_y = slider_bounds.y + 2.0;
            renderer.text(&text, text_x, text_y, SMALL_FONT_SIZE, self.config.label_color);
        }

        // Draw input field if enabled
        if let Some(input_bounds) = self.input_bounds(bounds) {
            // Background and border using input config helpers
            renderer.fill_rect(input_bounds, self.config.input.background(self.state.input_focused));
            // Use track_fill_color for focused border to match slider theme
            let border_color = if self.state.input_focused {
                self.config.track_fill_color
            } else {
                self.config.border_color
            };
            renderer.stroke_rect(input_bounds, border_color, 1.0);

            // Content area for text_core functions
            let content = Bounds::new(
                input_bounds.x + SLIDER_INPUT_PADDING,
                input_bounds.y + 2.0,
                input_bounds.width - SLIDER_INPUT_PADDING * 2.0,
                input_bounds.height - 4.0,
            );
            let content_y = input_bounds.y + (input_bounds.height - SMALL_FONT_SIZE) / 2.0;

            // Selection
            if self.state.input_focused {
                if let Some(selection) = self.state.input_selection {
                    text_core::draw_selection(
                        renderer,
                        content,
                        selection,
                        SMALL_FONT_SIZE,
                        self.config.input.selection_color,
                    );
                }
            }

            // Text
            renderer.text(
                &self.state.input_text,
                content.x,
                content_y,
                SMALL_FONT_SIZE,
                self.config.input.text_color,
            );

            // Cursor
            if self.state.input_focused {
                text_core::draw_cursor(
                    renderer,
                    content,
                    self.state.input_cursor,
                    SMALL_FONT_SIZE,
                    self.config.input.cursor_color,
                );
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

                // Check if hovering thumb (with expanded hit area)
                let thumb_center_x = thumb.x + SLIDER_THUMB_RADIUS;
                let thumb_center_y = thumb.y + SLIDER_THUMB_RADIUS;
                let dist_sq = (x - thumb_center_x).powi(2) + (y - thumb_center_y).powi(2);
                let hit_radius = SLIDER_THUMB_RADIUS * THUMB_HIT_AREA_MULTIPLIER;
                self.hovered = dist_sq <= hit_radius.powi(2);

                // Handle drag
                if self.state.drag.is_dragging() {
                    let new_value = self.position_to_value(x, &track);
                    if (new_value - self.state.value).abs() > f32::EPSILON {
                        self.state.value = new_value;
                        self.sync_input_from_value();
                        log::debug!("Slider drag: value = {}", new_value);
                        return self.emit_change();
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
                        let content_x = ib.x + SLIDER_INPUT_PADDING;
                        let new_cursor = text_core::x_to_char_index(
                            x,
                            content_x,
                            SMALL_FONT_SIZE,
                            self.state.input_text.len(),
                        );

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

                        // Call on_undo_point when input field gains focus (for undo tracking)
                        if !was_focused {
                            if let Some(ref on_undo_point) = self.on_undo_point {
                                log::debug!("Slider input: calling on_undo_point (focus gained)");
                                on_undo_point();
                            }
                        }

                        // Emit message to trigger redraw (selection highlight)
                        return self.emit_change();
                    }
                }

                // Blur input if clicking elsewhere
                if self.state.input_focused {
                    self.state.input_focused = false;
                    self.state.input_selection = None;
                    self.apply_input_value();
                    log::debug!("Slider input: blurred by clicking elsewhere");
                    // Always emit to trigger redraw (clears visual focus)
                    return self.emit_change();
                }

                // Check if clicking on thumb (with expanded hit area)
                let thumb_center_x = thumb.x + SLIDER_THUMB_RADIUS;
                let thumb_center_y = thumb.y + SLIDER_THUMB_RADIUS;
                let dist_sq = (x - thumb_center_x).powi(2) + (y - thumb_center_y).powi(2);
                let hit_radius = SLIDER_THUMB_RADIUS * THUMB_HIT_AREA_MULTIPLIER;

                if dist_sq <= hit_radius.powi(2) {
                    // Start dragging - call on_undo_point to save snapshot before changes
                    if let Some(ref on_undo_point) = self.on_undo_point {
                        log::debug!("Slider: calling on_undo_point (drag start)");
                        on_undo_point();
                    }
                    self.state.drag.start_drag();
                    log::debug!("Slider: started dragging");
                    return self.emit_change();
                } else if slider_bounds.contains(x, y) {
                    // Click on track - call on_undo_point to save snapshot before changes
                    if let Some(ref on_undo_point) = self.on_undo_point {
                        log::debug!("Slider: calling on_undo_point (track click)");
                        on_undo_point();
                    }
                    let new_value = self.position_to_value(x, &track);
                    self.state.value = new_value;
                    self.state.drag.start_drag();
                    self.sync_input_from_value();
                    log::debug!("Slider: clicked track, value = {}", new_value);
                    return self.emit_change();
                }

                None
            }

            Event::MouseRelease {
                button: MouseButton::Left,
                ..
            } => {
                if self.state.drag.is_dragging() {
                    self.state.drag.stop_drag();
                    log::debug!("Slider: stopped dragging");
                    return self.emit_change();
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
                            return self.emit_change();
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
                                    return self.emit_change();
                                }
                            }
                        }
                        KeyCode::Delete => {
                            if self.input_handle_delete() {
                                if let Ok(value) = self.state.input_text.parse::<f32>() {
                                    let clamped = value.clamp(self.min, self.max);
                                    self.state.value = clamped;
                                    return self.emit_change();
                                }
                            }
                        }
                        KeyCode::Left => {
                            let result = text_core::handle_left(
                                self.state.input_cursor,
                                self.state.input_selection,
                                modifiers.shift,
                            );
                            self.state.input_cursor = result.cursor;
                            self.state.input_selection = result.selection;
                        }
                        KeyCode::Right => {
                            let result = text_core::handle_right(
                                self.state.input_cursor,
                                self.state.input_selection,
                                self.state.input_text.len(),
                                modifiers.shift,
                            );
                            self.state.input_cursor = result.cursor;
                            self.state.input_selection = result.selection;
                        }
                        KeyCode::Home => {
                            let result = text_core::handle_home(
                                self.state.input_cursor,
                                self.state.input_selection,
                                modifiers.shift,
                            );
                            self.state.input_cursor = result.cursor;
                            self.state.input_selection = result.selection;
                        }
                        KeyCode::End => {
                            let result = text_core::handle_end(
                                self.state.input_cursor,
                                self.state.input_selection,
                                self.state.input_text.len(),
                                modifiers.shift,
                            );
                            self.state.input_cursor = result.cursor;
                            self.state.input_selection = result.selection;
                        }
                        KeyCode::A if modifiers.ctrl => {
                            let result = text_core::handle_select_all(self.state.input_text.len());
                            self.state.input_cursor = result.cursor;
                            self.state.input_selection = result.selection;
                        }
                        KeyCode::Z if modifiers.ctrl && modifiers.shift => {
                            // Ctrl+Shift+Z = Redo
                            if self.state.text_redo() {
                                log::debug!("Slider input: text redo");
                                // Try to update slider value from new text
                                if let Ok(value) = self.state.input_text.parse::<f32>() {
                                    self.state.value = value.clamp(self.min, self.max);
                                }
                                return self.emit_change();
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
                                return self.emit_change();
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
                                return self.emit_change();
                            }
                            return None;
                        }
                        KeyCode::Enter | KeyCode::Escape => {
                            self.state.input_focused = false;
                            self.state.input_selection = None;
                            self.apply_input_value();
                            log::debug!("Slider input: confirmed value = {}", self.state.value);
                            // Always emit to trigger redraw (clears visual selection)
                            return self.emit_change();
                        }
                        KeyCode::Up => {
                            let step = self.step.unwrap_or((self.max - self.min) / DEFAULT_STEP_DIVIDER);
                            let new_value = (self.state.value + step).clamp(self.min, self.max);
                            if (new_value - self.state.value).abs() > f32::EPSILON {
                                self.state.value = new_value;
                                self.state.input_text = self.format_value_for_input(new_value);
                                self.state.input_cursor = self.state.input_text.len();
                                self.state.input_selection = Some((0, self.state.input_text.len()));
                                return self.emit_change();
                            }
                        }
                        KeyCode::Down => {
                            let step = self.step.unwrap_or((self.max - self.min) / DEFAULT_STEP_DIVIDER);
                            let new_value = (self.state.value - step).clamp(self.min, self.max);
                            if (new_value - self.state.value).abs() > f32::EPSILON {
                                self.state.value = new_value;
                                self.state.input_text = self.format_value_for_input(new_value);
                                self.state.input_cursor = self.state.input_text.len();
                                self.state.input_selection = Some((0, self.state.input_text.len()));
                                return self.emit_change();
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

                let step = self.step.unwrap_or((self.max - self.min) / DEFAULT_STEP_DIVIDER);
                let big_step = step * BIG_STEP_MULTIPLIER;

                let delta = match key {
                    KeyCode::Left | KeyCode::Down => Some(-step),
                    KeyCode::Right | KeyCode::Up => Some(step),
                    KeyCode::PageDown => Some(-big_step),
                    KeyCode::PageUp => Some(big_step),
                    KeyCode::Home => {
                        self.state.value = self.min;
                        self.sync_input_from_value();
                        return self.emit_change();
                    }
                    KeyCode::End => {
                        self.state.value = self.max;
                        self.sync_input_from_value();
                        return self.emit_change();
                    }
                    _ => None,
                };

                if let Some(d) = delta {
                    let multiplier = if modifiers.shift { FINE_STEP_MULTIPLIER } else { 1.0 };
                    let new_value = (self.state.value + d * multiplier).clamp(self.min, self.max);
                    if (new_value - self.state.value).abs() > f32::EPSILON {
                        self.state.value = new_value;
                        self.sync_input_from_value();
                        log::debug!("Slider key: value = {}", new_value);
                        return self.emit_change();
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
                        let step = self.step.unwrap_or((self.max - self.min) / DEFAULT_STEP_DIVIDER);
                        let scroll_delta = delta.1.signum() * step;
                        let new_value = (self.state.value + scroll_delta).clamp(self.min, self.max);
                        if (new_value - self.state.value).abs() > f32::EPSILON {
                            self.state.value = new_value;
                            self.sync_input_from_value();
                            return self.emit_change();
                        }
                        return None;
                    }
                }

                // Handle scroll on slider
                if !slider_bounds.contains(position.0, position.1) {
                    return None;
                }

                let step = self.step.unwrap_or((self.max - self.min) / DEFAULT_STEP_DIVIDER);
                let scroll_delta = delta.1 * step;
                let new_value = (self.state.value + scroll_delta).clamp(self.min, self.max);

                if (new_value - self.state.value).abs() > f32::EPSILON {
                    self.state.value = new_value;
                    self.sync_input_from_value();
                    log::debug!("Slider scroll: value = {}", new_value);
                    return self.emit_change();
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
                    return self.emit_change();
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
                            return self.emit_change();
                        }
                    }
                }
                None
            }

            Event::CursorLeft => {
                // Cursor left window - release any drag state
                if self.state.drag.is_dragging() {
                    self.state.drag.stop_drag();
                    log::debug!("Slider: stopped dragging (cursor left window)");
                    return self.emit_change();
                }
                None
            }

            _ => None,
        }
    }

    fn has_active_drag(&self) -> bool {
        self.state.drag.is_dragging()
    }
}
