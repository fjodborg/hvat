//! Color picker widget for selecting colors from a palette
//!
//! This widget renders as an overlay popup with:
//! - A 4x4 grid of predefined colors for quick selection
//! - RGB sliders for custom color selection

use crate::callback::Callback;
use crate::constants::DEFAULT_FONT_SIZE;
use crate::event::{Event, MouseButton};
use crate::layout::{Alignment, Bounds, Size};
use crate::renderer::{Color, Renderer};
use crate::state::{ColorPickerDragging, ColorPickerState};
use crate::widget::{EventResult, Widget};
use crate::widgets::overlay::OverlayCloseHelper;

/// Predefined color palette for quick selection
const COLOR_PALETTE: [[u8; 3]; 16] = [
    [255, 100, 100], // Red-ish
    [255, 180, 100], // Orange
    [255, 255, 100], // Yellow
    [180, 255, 100], // Yellow-green
    [100, 255, 100], // Green
    [100, 255, 180], // Cyan-green
    [100, 255, 255], // Cyan
    [100, 180, 255], // Light blue
    [100, 100, 255], // Blue
    [180, 100, 255], // Purple
    [255, 100, 255], // Magenta
    [255, 100, 180], // Pink
    [200, 200, 200], // Light gray
    [150, 150, 150], // Medium gray
    [100, 100, 100], // Dark gray
    [50, 50, 50],    // Very dark gray
];

/// Number of columns in the color grid
const GRID_COLS: usize = 4;
/// Size of each color cell
const CELL_SIZE: f32 = 24.0;
/// Spacing between cells
const CELL_SPACING: f32 = 2.0;
/// Padding around the picker
const PICKER_PADDING: f32 = 8.0;
/// Height of each RGB slider row
const SLIDER_ROW_HEIGHT: f32 = 20.0;
/// Slider track width
const SLIDER_WIDTH: f32 = 100.0;
/// Slider thumb radius
const SLIDER_THUMB_RADIUS: f32 = 6.0;
/// Space between palette and sliders
const SECTION_SPACING: f32 = 8.0;
/// Color preview width
const PREVIEW_WIDTH: f32 = 20.0;
/// Gap between preview and label
const PREVIEW_GAP: f32 = 4.0;

// PickerAlignment is now replaced by Alignment from layout module
// Use Alignment::Right (default) or Alignment::Left for picker positioning
// Note: Alignment::Center is not used for picker positioning

/// A color picker popup widget with a palette of predefined colors and RGB sliders
pub struct ColorPicker<M> {
    /// Currently selected/editing color (RGB)
    current_color: [u8; 3],
    /// Whether the picker is visible/open
    is_open: bool,
    /// Position to render the overlay at (set by parent via layout bounds)
    overlay_position: (f32, f32),
    /// Horizontal alignment of the picker relative to anchor
    alignment: Alignment,
    /// Horizontal offset from anchor position (positive = move right)
    x_offset: f32,
    /// Vertical offset from anchor position (negative = move up)
    y_offset: f32,
    /// Hovered cell index in palette
    hovered_cell: Option<usize>,
    /// External state (cloned from app state)
    state: ColorPickerState,
    /// Callback when color changes (live updates from sliders)
    on_change: Callback<[u8; 3], M>,
    /// Callback when a color is selected from palette (confirms and closes)
    on_select: Callback<[u8; 3], M>,
    /// Callback when the picker is closed without selecting
    on_close: Option<M>,
    /// Callback when state changes (drag start/stop)
    on_state_change: Callback<ColorPickerState, M>,
}

impl<M> ColorPicker<M> {
    /// Create a new color picker
    pub fn new() -> Self {
        Self {
            current_color: [128, 128, 128],
            is_open: false,
            overlay_position: (0.0, 0.0),
            alignment: Alignment::Right,
            x_offset: 0.0,
            y_offset: 0.0,
            hovered_cell: None,
            state: ColorPickerState::default(),
            on_change: Callback::none(),
            on_select: Callback::none(),
            on_close: None,
            on_state_change: Callback::none(),
        }
    }

    /// Set the currently selected color
    pub fn selected(mut self, color: [u8; 3]) -> Self {
        self.current_color = color;
        self
    }

    /// Set whether the picker is open
    pub fn open(mut self, is_open: bool) -> Self {
        self.is_open = is_open;
        self
    }

    /// Set the external state (for drag tracking)
    pub fn state(mut self, state: &ColorPickerState) -> Self {
        self.state = *state;
        self
    }

    /// Set the overlay position (top-left corner)
    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.overlay_position = (x, y);
        self
    }

    /// Set the horizontal alignment of the picker
    ///
    /// - `Alignment::Right`: Opens to the right of the anchor (default)
    /// - `Alignment::Left`: Opens to the left of the anchor
    /// - `Alignment::Center`: Not used for picker positioning (treated as Right)
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Convenience: open picker to the left of the anchor point
    pub fn align_left(mut self) -> Self {
        self.alignment = Alignment::Left;
        self
    }

    /// Set the horizontal offset from the anchor position
    ///
    /// Use positive values to move the picker right (e.g., past a swatch)
    pub fn x_offset(mut self, offset: f32) -> Self {
        self.x_offset = offset;
        self
    }

    /// Set the vertical offset from the anchor position
    ///
    /// Use negative values to move the picker up (e.g., to align with a button above)
    pub fn y_offset(mut self, offset: f32) -> Self {
        self.y_offset = offset;
        self
    }

    /// Set the change callback (for live slider updates, doesn't close picker)
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn([u8; 3]) -> M + 'static,
    {
        self.on_change = Callback::new(handler);
        self
    }

    /// Set the selection callback (for palette clicks, closes picker)
    pub fn on_select<F>(mut self, handler: F) -> Self
    where
        F: Fn([u8; 3]) -> M + 'static,
    {
        self.on_select = Callback::new(handler);
        self
    }

    /// Set the close callback
    pub fn on_close(mut self, message: M) -> Self {
        self.on_close = Some(message);
        self
    }

    /// Set the state change callback (for drag start/stop)
    pub fn on_state_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(ColorPickerState) -> M + 'static,
    {
        self.on_state_change = Callback::new(handler);
        self
    }

    /// Calculate total picker size
    pub fn picker_size() -> Size {
        let cols = GRID_COLS as f32;
        let rows = (COLOR_PALETTE.len() as f32 / cols).ceil();
        let palette_width = cols * CELL_SIZE + (cols - 1.0) * CELL_SPACING;
        let palette_height = rows * CELL_SIZE + (rows - 1.0) * CELL_SPACING;

        // Sliders section: 3 rows (R, G, B)
        // Width includes: preview + gap + label + slider + value text
        let sliders_height = 3.0 * SLIDER_ROW_HEIGHT;
        let sliders_width = PREVIEW_WIDTH + PREVIEW_GAP + 15.0 + SLIDER_WIDTH + 30.0;

        Size::new(
            palette_width.max(sliders_width) + PICKER_PADDING * 2.0,
            palette_height + SECTION_SPACING + sliders_height + PICKER_PADDING * 2.0,
        )
    }

    /// Get the Y position where sliders start
    fn sliders_start_y(pos: (f32, f32)) -> f32 {
        let rows = (COLOR_PALETTE.len() as f32 / GRID_COLS as f32).ceil();
        let palette_height = rows * CELL_SIZE + (rows - 1.0) * CELL_SPACING;
        pos.1 + PICKER_PADDING + palette_height + SECTION_SPACING
    }

    /// Get bounds for a slider track
    fn slider_track_bounds(pos: (f32, f32), slider_index: usize) -> Bounds {
        let y = Self::sliders_start_y(pos) + slider_index as f32 * SLIDER_ROW_HEIGHT;
        // Account for preview rectangle + gap + label width
        let label_offset = PREVIEW_WIDTH + PREVIEW_GAP + 15.0;
        Bounds::new(
            pos.0 + PICKER_PADDING + label_offset,
            y + 2.0,
            SLIDER_WIDTH,
            SLIDER_ROW_HEIGHT - 4.0,
        )
    }

    /// Check if point is on a slider and which one
    fn hit_test_slider(pos: (f32, f32), x: f32, y: f32) -> Option<ColorPickerDragging> {
        for i in 0..3 {
            let track = Self::slider_track_bounds(pos, i);
            // Expand hit area a bit
            let expanded = Bounds::new(
                track.x - 5.0,
                track.y - 5.0,
                track.width + 10.0,
                track.height + 10.0,
            );
            if expanded.contains(x, y) {
                return Some(match i {
                    0 => ColorPickerDragging::Red,
                    1 => ColorPickerDragging::Green,
                    _ => ColorPickerDragging::Blue,
                });
            }
        }
        None
    }

    /// Update slider value from mouse position
    fn update_slider_value(&mut self, pos: (f32, f32), x: f32) {
        let slider_idx = match self.state.dragging {
            ColorPickerDragging::Red => 0,
            ColorPickerDragging::Green => 1,
            ColorPickerDragging::Blue => 2,
            ColorPickerDragging::None => return,
        };

        let track = Self::slider_track_bounds(pos, slider_idx);
        let normalized = ((x - track.x) / track.width).clamp(0.0, 1.0);
        let value = (normalized * 255.0) as u8;

        match self.state.dragging {
            ColorPickerDragging::Red => self.current_color[0] = value,
            ColorPickerDragging::Green => self.current_color[1] = value,
            ColorPickerDragging::Blue => self.current_color[2] = value,
            ColorPickerDragging::None => {}
        }
    }

    /// Calculate the actual overlay position, applying alignment and offsets.
    /// This ensures draw(), on_event(), and capture_bounds() all use consistent positioning.
    fn calculate_overlay_position(&self, layout_bounds: Bounds) -> (f32, f32) {
        let size = Self::picker_size();
        // Determine the anchor point (where the picker attaches to)
        let anchor = if self.overlay_position == (0.0, 0.0) {
            (
                layout_bounds.x + self.x_offset,
                layout_bounds.y + self.y_offset,
            )
        } else {
            (
                self.overlay_position.0 + self.x_offset,
                self.overlay_position.1 + self.y_offset,
            )
        };
        // Apply alignment
        match self.alignment {
            Alignment::Right | Alignment::Center => anchor,
            Alignment::Left => (anchor.0 - size.width, anchor.1),
        }
    }
}

impl<M> Default for ColorPicker<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Clone + 'static> Widget<M> for ColorPicker<M> {
    fn layout(&mut self, _available: Size) -> Size {
        // Return zero size - we render as overlay
        Size::ZERO
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        if !self.is_open {
            return;
        }

        // Use consistent position calculation (applies alignment and offsets)
        let pos = self.calculate_overlay_position(bounds);
        let size = Self::picker_size();
        let overlay_bounds = Bounds::new(pos.0, pos.1, size.width, size.height);

        // Register as overlay for event capture
        renderer.register_overlay(overlay_bounds);

        // Begin overlay rendering
        renderer.begin_overlay();

        // Corner radii for modern look
        const OVERLAY_RADIUS: f32 = 8.0;
        const CELL_RADIUS: f32 = 4.0;
        const PREVIEW_RADIUS: f32 = 4.0;

        // Draw shadow and background
        let theme = crate::theme::current_theme();
        renderer.draw_popup_shadow(overlay_bounds, OVERLAY_RADIUS);
        renderer.fill_rounded_rect(overlay_bounds, theme.popup_bg, OVERLAY_RADIUS);
        renderer.stroke_rounded_rect(overlay_bounds, theme.divider, OVERLAY_RADIUS, 1.0);

        // Draw palette cells (rounded)
        for (i, color) in COLOR_PALETTE.iter().enumerate() {
            let col = i % GRID_COLS;
            let row = i / GRID_COLS;
            let cell_x = pos.0 + PICKER_PADDING + col as f32 * (CELL_SIZE + CELL_SPACING);
            let cell_y = pos.1 + PICKER_PADDING + row as f32 * (CELL_SIZE + CELL_SPACING);
            let cell_bounds = Bounds::new(cell_x, cell_y, CELL_SIZE, CELL_SIZE);

            let cell_color = Color::rgb(
                color[0] as f32 / 255.0,
                color[1] as f32 / 255.0,
                color[2] as f32 / 255.0,
            );

            renderer.fill_rounded_rect(cell_bounds, cell_color, CELL_RADIUS);

            // Highlight hovered or matching current color
            let is_hovered = self.hovered_cell == Some(i);
            let is_current = *color == self.current_color;

            if is_current {
                renderer.stroke_rounded_rect(cell_bounds, Color::WHITE, CELL_RADIUS, 2.0);
            } else if is_hovered {
                renderer.stroke_rounded_rect(
                    cell_bounds,
                    Color::rgba(1.0, 1.0, 1.0, 0.5),
                    CELL_RADIUS,
                    1.0,
                );
            } else {
                renderer.stroke_rounded_rect(cell_bounds, theme.border, CELL_RADIUS, 1.0);
            }
        }

        // Draw RGB sliders with color preview
        let [r, g, b] = self.current_color;
        let sliders = [
            ("R", r, Color::rgb(1.0, 0.3, 0.3)),
            ("G", g, Color::rgb(0.3, 1.0, 0.3)),
            ("B", b, Color::rgb(0.3, 0.3, 1.0)),
        ];

        // Draw color preview rectangle next to the labels (rounded)
        let preview_x = pos.0 + PICKER_PADDING;
        let preview_y = Self::sliders_start_y(pos);
        let preview_height = 3.0 * SLIDER_ROW_HEIGHT - 4.0;
        let preview_bounds = Bounds::new(preview_x, preview_y, PREVIEW_WIDTH, preview_height);
        let preview_color = Color::rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        renderer.fill_rounded_rect(preview_bounds, preview_color, PREVIEW_RADIUS);
        renderer.stroke_rounded_rect(preview_bounds, theme.border, PREVIEW_RADIUS, 1.0);

        for (i, (label, value, color)) in sliders.iter().enumerate() {
            let track = Self::slider_track_bounds(pos, i);
            let label_x = pos.0 + PICKER_PADDING + PREVIEW_WIDTH + PREVIEW_GAP;
            let label_y = track.y;

            // Draw label
            renderer.text(
                label,
                label_x,
                label_y,
                DEFAULT_FONT_SIZE,
                theme.text_primary,
            );

            // Draw track background
            renderer.fill_rect(track, theme.slider_track);
            renderer.stroke_rect(track, theme.border, 1.0);

            // Draw gradient on track
            let gradient = Bounds::new(
                track.x + 1.0,
                track.y + 1.0,
                track.width - 2.0,
                track.height - 2.0,
            );
            renderer.fill_rect(gradient, *color);

            // Draw thumb
            let normalized = *value as f32 / 255.0;
            let thumb_x = track.x + normalized * track.width;
            let thumb_y = track.y + track.height / 2.0;
            renderer.fill_circle(thumb_x, thumb_y, SLIDER_THUMB_RADIUS, Color::WHITE);
            renderer.fill_circle(thumb_x, thumb_y, SLIDER_THUMB_RADIUS - 2.0, *color);

            // Draw value text
            let value_x = track.x + track.width + 5.0;
            renderer.text(
                &format!("{:3}", value),
                value_x,
                label_y,
                DEFAULT_FONT_SIZE,
                Color::TEXT_SECONDARY,
            );
        }

        renderer.end_overlay();
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> EventResult<M> {
        if !self.is_open {
            return EventResult::None;
        }

        // Use consistent position calculation (applies alignment and offsets)
        let pos = self.calculate_overlay_position(bounds);
        let size = Self::picker_size();
        let overlay_bounds = Bounds::new(pos.0, pos.1, size.width, size.height);

        // Helper to find palette cell at position
        let hit_test_palette = |x: f32, y: f32| -> Option<usize> {
            for i in 0..COLOR_PALETTE.len() {
                let col = i % GRID_COLS;
                let row = i / GRID_COLS;
                let cell_x = pos.0 + PICKER_PADDING + col as f32 * (CELL_SIZE + CELL_SPACING);
                let cell_y = pos.1 + PICKER_PADDING + row as f32 * (CELL_SIZE + CELL_SPACING);
                let cell = Bounds::new(cell_x, cell_y, CELL_SIZE, CELL_SIZE);
                if cell.contains(x, y) {
                    return Some(i);
                }
            }
            None
        };

        match event {
            Event::MouseMove { position, .. } => {
                let (x, y) = *position;

                // Update hover state for palette
                self.hovered_cell = hit_test_palette(x, y);

                // Handle slider dragging
                if self.state.is_dragging() {
                    self.update_slider_value(pos, x);
                    log::debug!(
                        "ColorPicker: dragging slider, color={:?}",
                        self.current_color
                    );
                    return self.on_change.call(self.current_color).into();
                }
                EventResult::None
            }

            Event::MousePress {
                button: MouseButton::Left,
                position,
                ..
            } => {
                let (x, y) = *position;

                // Check if clicking on a slider
                if let Some(slider) = Self::hit_test_slider(pos, x, y) {
                    log::debug!("ColorPicker: started dragging {:?}", slider);
                    self.state.start_drag(slider);
                    // Update value immediately
                    self.update_slider_value(pos, x);
                    // Emit state change AND color change
                    if let Some(msg) = self.on_state_change.call(self.state) {
                        return EventResult::Message(msg);
                    }
                    return self.on_change.call(self.current_color).into();
                }
                EventResult::None
            }

            Event::MouseRelease {
                button: MouseButton::Left,
                position,
                ..
            } => {
                let (x, y) = *position;

                // Stop any slider drag
                if self.state.is_dragging() {
                    log::debug!("ColorPicker: stopped dragging");
                    self.state.stop_drag();
                    // Emit state change AND final color change
                    if let Some(msg) = self.on_state_change.call(self.state) {
                        return EventResult::Message(msg);
                    }
                    return self.on_change.call(self.current_color).into();
                }

                // Check palette click - this DOES close the picker
                if let Some(index) = hit_test_palette(x, y) {
                    let color = COLOR_PALETTE[index];
                    log::debug!("ColorPicker: selected palette color {:?}", color);
                    return self.on_select.call(color).into();
                }

                EventResult::None
            }

            // Use OverlayCloseHelper for consistent close behavior
            _ if OverlayCloseHelper::should_close(&event, overlay_bounds) => {
                log::debug!("ColorPicker: closing via OverlayCloseHelper");
                self.on_close.clone().into()
            }

            _ => EventResult::None,
        }
    }

    fn has_active_overlay(&self) -> bool {
        self.is_open
    }

    fn has_active_drag(&self) -> bool {
        self.state.is_dragging()
    }

    fn capture_bounds(&self, layout_bounds: Bounds) -> Option<Bounds> {
        if self.is_open {
            // Use consistent position calculation (same as draw/on_event)
            let pos = self.calculate_overlay_position(layout_bounds);
            let size = Self::picker_size();
            Some(Bounds::new(pos.0, pos.1, size.width, size.height))
        } else {
            None
        }
    }
}
