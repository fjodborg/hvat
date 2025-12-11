//! Widget state types for stateful widgets

/// State for the image viewer widget
#[derive(Debug, Clone)]
pub struct ImageViewerState {
    /// Pan offset in clip space (-1 to 1)
    pub pan: (f32, f32),
    /// Zoom level (1.0 = 100%)
    pub zoom: f32,
    /// Current fit mode
    pub fit_mode: FitMode,
    /// Whether the widget is currently being dragged
    pub dragging: bool,
    /// Last mouse position during drag (screen space)
    pub last_drag_pos: Option<(f32, f32)>,
}

impl Default for ImageViewerState {
    fn default() -> Self {
        Self {
            pan: (0.0, 0.0),
            zoom: 1.0,
            fit_mode: FitMode::FitToView,
            dragging: false,
            last_drag_pos: None,
        }
    }
}

impl ImageViewerState {
    /// Create new state with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create state with specific zoom
    pub fn with_zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// Create state with specific fit mode
    pub fn with_fit_mode(mut self, mode: FitMode) -> Self {
        self.fit_mode = mode;
        self
    }

    /// Reset to default state
    pub fn reset(&mut self) {
        self.pan = (0.0, 0.0);
        self.zoom = 1.0;
        self.fit_mode = FitMode::FitToView;
    }

    /// Set to 1:1 pixel ratio
    pub fn set_one_to_one(&mut self) {
        self.fit_mode = FitMode::OneToOne;
        self.pan = (0.0, 0.0);
    }

    /// Set to fit to view
    pub fn set_fit_to_view(&mut self) {
        self.fit_mode = FitMode::FitToView;
        self.pan = (0.0, 0.0);
    }

    /// Pan by delta in clip space
    pub fn pan_by(&mut self, delta_x: f32, delta_y: f32) {
        self.pan.0 += delta_x;
        self.pan.1 += delta_y;
        // When manually panning, switch to manual mode
        self.fit_mode = FitMode::Manual;
    }

    /// Zoom at a specific point (in clip space)
    pub fn zoom_at(&mut self, cursor_x: f32, cursor_y: f32, factor: f32) {
        let new_zoom = (self.zoom * factor).clamp(0.1, 50.0);
        let zoom_ratio = new_zoom / self.zoom;

        // Adjust pan so point under cursor stays fixed
        let cursor_rel_x = cursor_x - self.pan.0;
        let cursor_rel_y = cursor_y - self.pan.1;
        self.pan.0 -= cursor_rel_x * (zoom_ratio - 1.0);
        self.pan.1 -= cursor_rel_y * (zoom_ratio - 1.0);

        self.zoom = new_zoom;
        self.fit_mode = FitMode::Manual;
    }

    /// Zoom in by a standard factor
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.25).clamp(0.1, 50.0);
        self.fit_mode = FitMode::Manual;
    }

    /// Zoom out by a standard factor
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.25).clamp(0.1, 50.0);
        self.fit_mode = FitMode::Manual;
    }
}

/// How the image should be fit to the viewer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FitMode {
    /// Manual zoom and pan
    Manual,
    /// Automatically fit to view (default)
    #[default]
    FitToView,
    /// 1:1 pixel ratio
    OneToOne,
}

/// State for scrollable containers
#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    /// Scroll offset (x, y)
    pub offset: (f32, f32),
    /// Velocity for momentum scrolling
    pub(crate) velocity: (f32, f32),
    /// Whether currently being dragged
    pub(crate) dragging: bool,
    /// Offset within thumb where drag started
    pub(crate) drag_start_offset: Option<f32>,
}

impl ScrollState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Scroll to a specific offset
    pub fn scroll_to(&mut self, x: f32, y: f32) {
        self.offset = (x, y);
    }

    /// Scroll by a delta
    pub fn scroll_by(&mut self, dx: f32, dy: f32) {
        self.offset.0 += dx;
        self.offset.1 += dy;
    }
}

/// State for dropdown widgets
#[derive(Debug, Clone, Default)]
pub struct DropdownState {
    /// Whether the dropdown is open
    pub is_open: bool,
    /// Search/filter text
    pub search_text: String,
    /// Currently highlighted option index
    pub highlighted: Option<usize>,
    /// Scroll offset for the popup list (in number of items)
    pub scroll_offset: usize,
}

impl DropdownState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.highlighted = Some(0);
        self.scroll_offset = 0;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.search_text.clear();
        self.scroll_offset = 0;
    }

    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Scroll the dropdown list by a delta (positive = down, negative = up)
    pub fn scroll_by(&mut self, delta: isize, max_items: usize, visible_items: usize) {
        if max_items <= visible_items {
            self.scroll_offset = 0;
            return;
        }

        let max_scroll = max_items.saturating_sub(visible_items);
        let new_offset = (self.scroll_offset as isize + delta).clamp(0, max_scroll as isize) as usize;
        self.scroll_offset = new_offset;
    }

    /// Ensure the highlighted item is visible within the scroll view
    pub fn ensure_highlighted_visible(&mut self, visible_items: usize) {
        if let Some(highlighted) = self.highlighted {
            // If highlighted is above visible area, scroll up
            if highlighted < self.scroll_offset {
                self.scroll_offset = highlighted;
            }
            // If highlighted is below visible area, scroll down
            else if highlighted >= self.scroll_offset + visible_items {
                self.scroll_offset = highlighted.saturating_sub(visible_items.saturating_sub(1));
            }
        }
    }
}

/// State for collapsible sections
#[derive(Debug, Clone)]
pub struct CollapsibleState {
    /// Whether the section is expanded
    pub is_expanded: bool,
    /// Animation progress (0.0 = collapsed, 1.0 = expanded)
    pub(crate) animation_progress: f32,
}

impl Default for CollapsibleState {
    fn default() -> Self {
        Self {
            is_expanded: false,
            animation_progress: 0.0,
        }
    }
}

impl CollapsibleState {
    pub fn new(expanded: bool) -> Self {
        Self {
            is_expanded: expanded,
            animation_progress: if expanded { 1.0 } else { 0.0 },
        }
    }

    pub fn expanded() -> Self {
        Self::new(true)
    }

    pub fn collapsed() -> Self {
        Self::new(false)
    }

    pub fn toggle(&mut self) {
        self.is_expanded = !self.is_expanded;
    }
}

/// State for text input fields
#[derive(Debug, Clone, Default)]
pub struct TextInputState {
    /// Cursor position (character index)
    pub cursor: usize,
    /// Selection range (start, end) if any
    pub selection: Option<(usize, usize)>,
    /// Whether the input is focused
    pub is_focused: bool,
}

impl TextInputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn focus(&mut self) {
        self.is_focused = true;
    }

    pub fn blur(&mut self) {
        self.is_focused = false;
        self.selection = None;
    }
}

/// A snapshot of text input state for undo
#[derive(Debug, Clone)]
pub struct TextSnapshot {
    /// The text content
    pub text: String,
    /// Cursor position
    pub cursor: usize,
}

/// State for slider widgets
#[derive(Debug, Clone)]
pub struct SliderState {
    /// Current value
    pub value: f32,
    /// Whether currently being dragged
    pub dragging: bool,
    /// Input field focus state (when show_input is enabled)
    pub input_focused: bool,
    /// Input field text (when show_input is enabled)
    pub input_text: String,
    /// Input field cursor position
    pub input_cursor: usize,
    /// Input field selection range
    pub input_selection: Option<(usize, usize)>,
    /// Text undo stack (for Ctrl+Z in input field)
    pub(crate) input_undo_stack: Vec<TextSnapshot>,
    /// Text redo stack (for Ctrl+Y in input field)
    pub(crate) input_redo_stack: Vec<TextSnapshot>,
}

impl Default for SliderState {
    fn default() -> Self {
        Self {
            value: 0.0,
            dragging: false,
            input_focused: false,
            input_text: String::new(),
            input_cursor: 0,
            input_selection: None,
            input_undo_stack: Vec::new(),
            input_redo_stack: Vec::new(),
        }
    }
}

impl SliderState {
    pub fn new(value: f32) -> Self {
        let text = Self::format_value(value);
        let cursor = text.len();
        Self {
            value,
            dragging: false,
            input_focused: false,
            input_text: text,
            input_cursor: cursor,
            input_selection: None,
            input_undo_stack: Vec::new(),
            input_redo_stack: Vec::new(),
        }
    }

    /// Push current text state to undo stack (call before making changes)
    pub fn push_text_undo(&mut self) {
        self.input_undo_stack.push(TextSnapshot {
            text: self.input_text.clone(),
            cursor: self.input_cursor,
        });
        // Clear redo stack on new change
        self.input_redo_stack.clear();
        // Limit undo history to 50 entries
        while self.input_undo_stack.len() > 50 {
            self.input_undo_stack.remove(0);
        }
    }

    /// Undo text change (Ctrl+Z)
    pub fn text_undo(&mut self) -> bool {
        if let Some(snapshot) = self.input_undo_stack.pop() {
            // Save current state to redo stack
            self.input_redo_stack.push(TextSnapshot {
                text: self.input_text.clone(),
                cursor: self.input_cursor,
            });
            self.input_text = snapshot.text;
            self.input_cursor = snapshot.cursor;
            self.input_selection = None;
            true
        } else {
            false
        }
    }

    /// Redo text change (Ctrl+Y or Ctrl+Shift+Z)
    pub fn text_redo(&mut self) -> bool {
        if let Some(snapshot) = self.input_redo_stack.pop() {
            // Save current state to undo stack
            self.input_undo_stack.push(TextSnapshot {
                text: self.input_text.clone(),
                cursor: self.input_cursor,
            });
            self.input_text = snapshot.text;
            self.input_cursor = snapshot.cursor;
            self.input_selection = None;
            true
        } else {
            false
        }
    }

    /// Clear text undo/redo history
    pub fn clear_text_history(&mut self) {
        self.input_undo_stack.clear();
        self.input_redo_stack.clear();
    }

    /// Set the value
    pub fn set_value(&mut self, value: f32) {
        self.value = value;
        if !self.input_focused {
            self.input_text = Self::format_value(value);
            self.input_cursor = self.input_text.len();
        }
    }

    /// Format value for input field display
    fn format_value(value: f32) -> String {
        // If it's close to an integer, display as integer
        if (value - value.round()).abs() < 0.0001 {
            format!("{}", value.round() as i32)
        } else {
            // Otherwise display with up to 3 decimal places, trimming trailing zeros
            let formatted = format!("{:.3}", value);
            formatted.trim_end_matches('0').trim_end_matches('.').to_string()
        }
    }

    /// Sync input text from value (call when not focused)
    pub fn sync_input_from_value(&mut self) {
        if !self.input_focused {
            self.input_text = Self::format_value(self.value);
            self.input_cursor = self.input_text.len();
            self.input_selection = None;
        }
    }
}

/// State for number input fields
#[derive(Debug, Clone)]
pub struct NumberInputState {
    /// The current text being edited
    pub text: String,
    /// Cursor position (character index)
    pub cursor: usize,
    /// Whether the input is focused
    pub is_focused: bool,
    /// Selection range (start, end) if any
    pub selection: Option<(usize, usize)>,
}

impl Default for NumberInputState {
    fn default() -> Self {
        Self {
            text: String::from("0"),
            cursor: 1,
            is_focused: false,
            selection: None,
        }
    }
}

impl NumberInputState {
    pub fn new(value: f32) -> Self {
        let text = format_number(value);
        let cursor = text.len();
        Self {
            text,
            cursor,
            is_focused: false,
            selection: None,
        }
    }

    /// Parse the current text as a number
    pub fn value(&self) -> Option<f32> {
        self.text.parse().ok()
    }

    /// Set the value (updates text)
    pub fn set_value(&mut self, value: f32) {
        self.text = format_number(value);
        self.cursor = self.text.len();
        self.selection = None;
    }

    /// Focus the input
    pub fn focus(&mut self) {
        self.is_focused = true;
        // Select all text when focusing
        if !self.text.is_empty() {
            self.selection = Some((0, self.text.len()));
            self.cursor = self.text.len();
        }
    }

    /// Blur the input
    pub fn blur(&mut self) {
        self.is_focused = false;
        self.selection = None;
    }
}

/// Format a number for display, avoiding excessive decimal places
fn format_number(value: f32) -> String {
    // If it's close to an integer, display as integer
    if (value - value.round()).abs() < 0.0001 {
        format!("{}", value.round() as i32)
    } else {
        // Otherwise display with up to 3 decimal places, trimming trailing zeros
        let formatted = format!("{:.3}", value);
        formatted.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
