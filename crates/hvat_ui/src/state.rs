//! Widget state types for stateful widgets

use crate::constants::{format_number, UNDO_STACK_LIMIT, ZOOM_FACTOR, ZOOM_MAX, ZOOM_MIN};
use std::cell::RefCell;
use std::rc::Rc;

/// Generic undo stack for any value type
///
/// This can be used to implement undo/redo for any cloneable type:
/// - Text input values
/// - Slider values
/// - Number inputs
/// - Custom application state
///
/// # Example
/// ```
/// use hvat_ui::UndoStack;
///
/// let mut stack: UndoStack<String> = UndoStack::new(50);
///
/// // Before making a change, push current state
/// stack.push("hello".to_string());
///
/// // Now the value is "hello world"
/// let current = "hello world".to_string();
///
/// // Undo returns the previous state
/// if let Some(previous) = stack.undo(current) {
///     assert_eq!(previous, "hello");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct UndoStack<T: Clone> {
    /// Stack of states that can be undone
    undo_stack: Vec<T>,
    /// Stack of states that can be redone
    redo_stack: Vec<T>,
    /// Maximum history size
    max_history: usize,
}

impl<T: Clone> Default for UndoStack<T> {
    fn default() -> Self {
        Self::new(UNDO_STACK_LIMIT)
    }
}

impl<T: Clone> UndoStack<T> {
    /// Create a new undo stack with specified max history size
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Push a state to the undo stack (call this BEFORE making a change)
    ///
    /// This clears the redo stack since a new change invalidates the redo history.
    pub fn push(&mut self, state: T) {
        self.undo_stack.push(state);
        self.redo_stack.clear();

        // Limit history size
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Undo: returns the previous state, or None if nothing to undo
    ///
    /// The current state must be passed so it can be pushed to the redo stack.
    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo_stack.pop()?;
        self.redo_stack.push(current);
        Some(prev)
    }

    /// Redo: returns the next state, or None if nothing to redo
    ///
    /// The current state must be passed so it can be pushed to the undo stack.
    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo_stack.pop()?;
        self.undo_stack.push(current);
        Some(next)
    }

    /// Get number of undo steps available
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get number of redo steps available
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

/// Pan drag interaction state for image viewer
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PanDragState {
    /// Not dragging
    #[default]
    Idle,
    /// Dragging with last mouse position (screen space)
    Dragging { last_pos: (f32, f32) },
}

impl PanDragState {
    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool {
        matches!(self, PanDragState::Dragging { .. })
    }

    /// Get the last drag position if dragging
    pub fn last_pos(&self) -> Option<(f32, f32)> {
        match self {
            PanDragState::Dragging { last_pos } => Some(*last_pos),
            PanDragState::Idle => None,
        }
    }

    /// Start dragging with the given position
    pub fn start_drag(&mut self, pos: (f32, f32)) {
        *self = PanDragState::Dragging { last_pos: pos };
    }

    /// Update last position during drag
    pub fn update_pos(&mut self, pos: (f32, f32)) {
        if let PanDragState::Dragging { last_pos } = self {
            *last_pos = pos;
        }
    }

    /// Stop dragging
    pub fn stop_drag(&mut self) {
        *self = PanDragState::Idle;
    }
}

/// State for the image viewer widget
#[derive(Debug, Clone)]
pub struct ImageViewerState {
    /// Pan offset in clip space (-1 to 1)
    pub pan: (f32, f32),
    /// Zoom level (1.0 = fit to view, actual pixel ratio depends on image/view size)
    pub zoom: f32,
    /// Current fit mode - used temporarily when switching modes
    /// After the ImageViewer processes this, fit_mode is set back to Manual
    pub fit_mode: FitMode,
    /// Drag interaction state for panning
    pub drag: PanDragState,
    /// Cached view bounds from last render (width, height)
    /// Used to calculate 1:1 zoom from outside the widget
    pub cached_view_size: Option<(f32, f32)>,
    /// Cached texture size (width, height)
    /// Used to calculate 1:1 zoom from outside the widget
    pub cached_texture_size: Option<(u32, u32)>,
}

impl Default for ImageViewerState {
    fn default() -> Self {
        Self {
            pan: (0.0, 0.0),
            zoom: 1.0,
            fit_mode: FitMode::FitToView,
            drag: PanDragState::default(),
            cached_view_size: None,
            cached_texture_size: None,
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

    /// Reset to default state (fit to view)
    pub fn reset(&mut self) {
        self.set_fit_to_view();
    }

    /// Set to 1:1 pixel ratio
    /// If cached view/texture sizes are available, calculates the actual zoom value.
    /// Otherwise sets fit_mode to OneToOne for the widget to calculate later.
    pub fn set_one_to_one(&mut self) {
        self.pan = (0.0, 0.0);
        if let (Some((view_w, view_h)), Some((tex_w, tex_h))) = (self.cached_view_size, self.cached_texture_size) {
            // Calculate 1:1 zoom directly
            self.zoom = Self::calculate_one_to_one_zoom(view_w, view_h, tex_w, tex_h);
            self.fit_mode = FitMode::Manual;
        } else {
            // No cached sizes - let widget calculate on next event
            self.fit_mode = FitMode::OneToOne;
        }
    }

    /// Set to fit to view (zoom = 1.0)
    pub fn set_fit_to_view(&mut self) {
        self.zoom = 1.0;
        self.fit_mode = FitMode::Manual;
        self.pan = (0.0, 0.0);
    }

    /// Calculate the zoom value for 1:1 pixel mapping
    pub fn calculate_one_to_one_zoom(view_w: f32, view_h: f32, tex_w: u32, tex_h: u32) -> f32 {
        if tex_w == 0 || tex_h == 0 {
            return 1.0;
        }
        let image_aspect = tex_w as f32 / tex_h as f32;
        let view_aspect = view_w / view_h;

        if image_aspect > view_aspect {
            view_w / tex_w as f32
        } else {
            view_h / tex_h as f32
        }
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
        let new_zoom = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
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
        self.zoom = (self.zoom * ZOOM_FACTOR).clamp(ZOOM_MIN, ZOOM_MAX);
        self.fit_mode = FitMode::Manual;
    }

    /// Zoom out by a standard factor
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / ZOOM_FACTOR).clamp(ZOOM_MIN, ZOOM_MAX);
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

/// Scroll thumb drag interaction state
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ScrollDragState {
    /// Not dragging
    #[default]
    Idle,
    /// Dragging the scrollbar thumb, with offset within thumb where drag started
    Dragging { thumb_offset: f32 },
}

impl ScrollDragState {
    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool {
        matches!(self, ScrollDragState::Dragging { .. })
    }

    /// Get the thumb offset if dragging
    pub fn thumb_offset(&self) -> Option<f32> {
        match self {
            ScrollDragState::Dragging { thumb_offset } => Some(*thumb_offset),
            ScrollDragState::Idle => None,
        }
    }

    /// Start dragging with the given thumb offset
    pub fn start_drag(&mut self, offset: f32) {
        *self = ScrollDragState::Dragging { thumb_offset: offset };
    }

    /// Stop dragging
    pub fn stop_drag(&mut self) {
        *self = ScrollDragState::Idle;
    }
}

/// State for scrollable containers
#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    /// Scroll offset (x, y)
    pub offset: (f32, f32),
    /// Velocity for momentum scrolling
    pub(crate) velocity: (f32, f32),
    /// Drag interaction state for scrollbar thumb
    pub(crate) drag: ScrollDragState,
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
    /// Whether the popup should open upward (calculated when opening)
    pub opens_upward: bool,
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
        self.opens_upward = false;
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
#[derive(Debug, Clone, Default)]
pub struct CollapsibleState {
    /// Whether the section is expanded
    pub is_expanded: bool,
}

impl CollapsibleState {
    pub fn new(expanded: bool) -> Self {
        Self {
            is_expanded: expanded,
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
///
/// Note: Undo/redo is handled externally via `UndoStack<T>`. Use the `on_undo_point`
/// callback on the widget to know when to save an undo snapshot.
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

/// A snapshot of text input state for undo (used externally by demos)
#[derive(Debug, Clone)]
pub struct TextSnapshot {
    /// The text content
    pub text: String,
    /// Cursor position
    pub cursor: usize,
}

/// Slider thumb drag interaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SliderDragState {
    /// Not dragging
    #[default]
    Idle,
    /// Dragging the slider thumb
    Dragging,
}

impl SliderDragState {
    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool {
        matches!(self, SliderDragState::Dragging)
    }

    /// Start dragging
    pub fn start_drag(&mut self) {
        *self = SliderDragState::Dragging;
    }

    /// Stop dragging
    pub fn stop_drag(&mut self) {
        *self = SliderDragState::Idle;
    }
}

/// State for slider widgets
///
/// Note: Undo/redo is handled externally via `UndoStack<T>`. Use the `on_undo_point`
/// callback on the widget to know when to save an undo snapshot.
#[derive(Debug, Clone)]
pub struct SliderState {
    /// Current value
    pub value: f32,
    /// Drag interaction state
    pub drag: SliderDragState,
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
            drag: SliderDragState::default(),
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
            drag: SliderDragState::default(),
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
        // Limit undo history
        while self.input_undo_stack.len() > UNDO_STACK_LIMIT {
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
        format_number(value)
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
///
/// Note: Undo/redo is handled externally via `UndoStack<T>`. Use the `on_undo_point`
/// callback on the widget to know when to save an undo snapshot.
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
    /// Text undo stack (for Ctrl+Z)
    pub(crate) undo_stack: Vec<TextSnapshot>,
    /// Text redo stack (for Ctrl+Y/Ctrl+Shift+Z)
    pub(crate) redo_stack: Vec<TextSnapshot>,
}

impl Default for NumberInputState {
    fn default() -> Self {
        Self {
            text: String::from("0"),
            cursor: 1,
            is_focused: false,
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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

    /// Push current text state to undo stack (call before making changes)
    pub fn push_undo(&mut self) {
        self.undo_stack.push(TextSnapshot {
            text: self.text.clone(),
            cursor: self.cursor,
        });
        // Clear redo stack on new change
        self.redo_stack.clear();
        // Limit undo history
        while self.undo_stack.len() > UNDO_STACK_LIMIT {
            self.undo_stack.remove(0);
        }
    }

    /// Undo text change (Ctrl+Z) - returns true if undo was performed
    pub fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.undo_stack.pop() {
            // Save current state to redo stack
            self.redo_stack.push(TextSnapshot {
                text: self.text.clone(),
                cursor: self.cursor,
            });
            self.text = snapshot.text;
            self.cursor = snapshot.cursor;
            self.selection = None;
            true
        } else {
            false
        }
    }

    /// Redo text change (Ctrl+Y or Ctrl+Shift+Z) - returns true if redo was performed
    pub fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.redo_stack.pop() {
            // Save current state to undo stack
            self.undo_stack.push(TextSnapshot {
                text: self.text.clone(),
                cursor: self.cursor,
            });
            self.text = snapshot.text;
            self.cursor = snapshot.cursor;
            self.selection = None;
            true
        } else {
            false
        }
    }

    /// Clear undo/redo history
    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

// =============================================================================
// Undo Context Helper
// =============================================================================

/// Helper for creating `on_undo_point` callbacks in view functions.
///
/// This eliminates the need to manually clone `Rc<RefCell<UndoStack<T>>>` and
/// snapshot values multiple times when building widget trees.
///
/// # Example
///
/// **Before (verbose):**
/// ```ignore
/// let undo_stack1 = Rc::clone(&self.undo_stack);
/// let undo_stack2 = Rc::clone(&self.undo_stack);
/// let snap1 = snapshot.clone();
/// let snap2 = snapshot.clone();
///
/// slider().on_undo_point({
///     let stack = undo_stack1;
///     let snap = snap1;
///     move || stack.borrow_mut().push(snap.clone())
/// })
/// ```
///
/// **After (clean):**
/// ```ignore
/// let undo_ctx = UndoContext::new(Rc::clone(&self.undo_stack), self.snapshot());
///
/// slider().on_undo_point(undo_ctx.callback())
/// ```
#[derive(Clone)]
pub struct UndoContext<T: Clone + 'static> {
    stack: Rc<RefCell<UndoStack<T>>>,
    snapshot: T,
}

impl<T: Clone + 'static> UndoContext<T> {
    /// Create a new undo context with the given stack and snapshot.
    ///
    /// The snapshot should represent the current state BEFORE any edits.
    /// Each call to `callback()` will create a closure that pushes a clone
    /// of this snapshot to the undo stack.
    pub fn new(stack: Rc<RefCell<UndoStack<T>>>, snapshot: T) -> Self {
        Self { stack, snapshot }
    }

    /// Create an `on_undo_point` callback.
    ///
    /// This can be called multiple times - each call creates a new closure
    /// that captures clones of the stack and snapshot.
    pub fn callback(&self) -> impl Fn() + 'static {
        let stack = Rc::clone(&self.stack);
        let snap = self.snapshot.clone();
        move || {
            log::debug!("UndoContext: pushing undo snapshot");
            stack.borrow_mut().push(snap.clone());
        }
    }

    /// Create a callback with a custom message for logging.
    pub fn callback_with_label(&self, label: &'static str) -> impl Fn() + 'static {
        let stack = Rc::clone(&self.stack);
        let snap = self.snapshot.clone();
        move || {
            log::debug!("UndoContext [{}]: pushing undo snapshot", label);
            stack.borrow_mut().push(snap.clone());
        }
    }
}
