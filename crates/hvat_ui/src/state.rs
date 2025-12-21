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

/// Generic drag interaction state
///
/// This enum represents a dragging state that can optionally hold data.
/// Use `()` as the type parameter for simple drag tracking without data.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DragState<T = ()> {
    /// Not dragging
    #[default]
    Idle,
    /// Dragging with optional data
    Dragging(T),
}

impl<T: Default> DragState<T> {
    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool {
        matches!(self, DragState::Dragging(_))
    }

    /// Start dragging with default data
    pub fn start_drag(&mut self) {
        *self = DragState::Dragging(T::default());
    }

    /// Stop dragging
    pub fn stop_drag(&mut self) {
        *self = DragState::Idle;
    }

    /// Get the drag data if dragging
    pub fn data(&self) -> Option<&T> {
        match self {
            DragState::Dragging(data) => Some(data),
            DragState::Idle => None,
        }
    }

    /// Get mutable drag data if dragging
    pub fn data_mut(&mut self) -> Option<&mut T> {
        match self {
            DragState::Dragging(data) => Some(data),
            DragState::Idle => None,
        }
    }
}

impl<T> DragState<T> {
    /// Start dragging with specific data
    pub fn start_drag_with(&mut self, data: T) {
        *self = DragState::Dragging(data);
    }
}

/// Pan drag data for image viewer
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PanDragData {
    /// Last mouse position (screen space)
    pub last_pos: (f32, f32),
}

/// Pan drag interaction state for image viewer
pub type PanDragState = DragState<PanDragData>;

/// Extension methods for PanDragState
pub trait PanDragExt {
    /// Get the last drag position if dragging
    fn last_pos(&self) -> Option<(f32, f32)>;
    /// Update last position during drag
    fn update_pos(&mut self, pos: (f32, f32));
}

impl PanDragExt for PanDragState {
    fn last_pos(&self) -> Option<(f32, f32)> {
        self.data().map(|d| d.last_pos)
    }

    fn update_pos(&mut self, pos: (f32, f32)) {
        if let Some(data) = self.data_mut() {
            data.last_pos = pos;
        }
    }
}

/// Interaction mode for the image viewer.
///
/// Determines how left mouse button interactions are interpreted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InteractionMode {
    /// Normal viewing mode - left click does nothing special, middle mouse pans
    #[default]
    View,
    /// Annotation mode - left click/drag reports pointer events for drawing
    Annotate,
}

/// Current pointer interaction state.
///
/// Tracks what interaction is currently in progress.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PointerState {
    /// No active pointer interaction
    #[default]
    Idle,
    /// Left mouse is being dragged for annotation drawing
    AnnotationDrag,
}

/// State for the image viewer widget
///
/// ## Zoom semantics
/// The zoom value represents `screen_pixels / image_pixel`:
/// - zoom = 1.0 (100%) means 1:1 pixel ratio (1 image pixel = 1 screen pixel)
/// - zoom = 2.0 (200%) means 1 image pixel = 2 screen pixels (enlarged)
/// - zoom = 0.5 (50%) means 2 image pixels = 1 screen pixel (shrunk)
#[derive(Debug, Clone)]
pub struct ImageViewerState {
    /// Pan offset in clip space (-1 to 1)
    pub pan: (f32, f32),
    /// Zoom level where 1.0 = 1:1 pixel ratio (100%)
    /// zoom = screen_pixels_per_image_pixel
    pub zoom: f32,
    /// Current fit mode
    pub fit_mode: FitMode,
    /// Drag interaction state for panning (middle mouse)
    pub drag: PanDragState,
    /// Current pointer interaction state (for annotation drawing)
    pub pointer_state: PointerState,
    /// Cached view bounds from last render (width, height)
    pub cached_view_size: Option<(f32, f32)>,
    /// Cached texture size (width, height)
    pub cached_texture_size: Option<(u32, u32)>,
}

impl Default for ImageViewerState {
    fn default() -> Self {
        Self {
            pan: (0.0, 0.0),
            zoom: 1.0, // Will be updated by sync_with_bounds on first render
            fit_mode: FitMode::FitToView, // Indicates zoom needs to be calculated
            drag: PanDragState::default(),
            pointer_state: PointerState::default(),
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

    /// Update cached sizes and resolve any pending fit modes.
    ///
    /// This should be called by the widget whenever it has access to bounds,
    /// ensuring that `zoom` always reflects the actual displayed value.
    /// This fixes the issue where `FitToView` mode leaves `zoom` at a stale value
    /// until user interaction.
    pub fn sync_with_bounds(
        &mut self,
        view_width: f32,
        view_height: f32,
        tex_width: u32,
        tex_height: u32,
    ) {
        self.cached_view_size = Some((view_width, view_height));
        self.cached_texture_size = Some((tex_width, tex_height));

        // If we're in a deferred fit mode, resolve it now that we have bounds
        if self.fit_mode == FitMode::FitToView {
            self.zoom = Self::calculate_fit_zoom(view_width, view_height, tex_width, tex_height);
            self.fit_mode = FitMode::Manual;
        }
    }

    /// Reset to default state (fit to view)
    pub fn reset(&mut self) {
        self.set_fit_to_view();
    }

    /// Set to 1:1 pixel ratio (zoom = 1.0 = 100%)
    /// This is a one-time action - sets zoom to 1.0 and stays in Manual mode
    pub fn set_one_to_one(&mut self) {
        self.zoom = 1.0;
        self.fit_mode = FitMode::Manual;
        self.pan = (0.0, 0.0);
    }

    /// Set to fit to view - one-time action
    /// Calculates the zoom that makes the image fit, then stays in Manual mode
    /// Requires cached sizes to calculate the fit zoom
    pub fn set_fit_to_view(&mut self) {
        self.pan = (0.0, 0.0);
        if let (Some((view_w, view_h)), Some((tex_w, tex_h))) =
            (self.cached_view_size, self.cached_texture_size)
        {
            self.zoom = Self::calculate_fit_zoom(view_w, view_h, tex_w, tex_h);
            self.fit_mode = FitMode::Manual;
        } else {
            // No cached sizes yet - use FitToView mode temporarily
            // The widget will calculate and apply the zoom on first render
            self.fit_mode = FitMode::FitToView;
        }
    }

    /// Calculate the zoom value that makes the image fit the view
    /// Returns the zoom where the image exactly fills the viewport
    pub fn calculate_fit_zoom(view_w: f32, view_h: f32, tex_w: u32, tex_h: u32) -> f32 {
        if tex_w == 0 || tex_h == 0 {
            return 1.0;
        }
        let image_aspect = tex_w as f32 / tex_h as f32;
        let view_aspect = view_w / view_h;

        // Calculate zoom so image fits in view
        // zoom = screen_pixels_per_image_pixel
        // At fit: view_size = tex_size * zoom (for the constraining dimension)
        if image_aspect > view_aspect {
            // Image is wider - width is the constraint
            view_w / tex_w as f32
        } else {
            // Image is taller - height is the constraint
            view_h / tex_h as f32
        }
    }

    /// Get the effective zoom value based on current fit_mode and cached sizes
    /// This is useful for UI elements that need to display the current zoom percentage
    /// Returns the zoom value where 1.0 = 100% (1:1 pixel ratio)
    pub fn effective_zoom(&self) -> f32 {
        match self.fit_mode {
            FitMode::OneToOne => 1.0,
            FitMode::FitToView => {
                // FitToView is a temporary mode before the widget has rendered
                // Calculate fit zoom from cached sizes if available
                if let (Some((view_w, view_h)), Some((tex_w, tex_h))) =
                    (self.cached_view_size, self.cached_texture_size)
                {
                    Self::calculate_fit_zoom(view_w, view_h, tex_w, tex_h)
                } else {
                    self.zoom
                }
            }
            FitMode::Manual => self.zoom,
        }
    }

    /// Pan by delta in clip space
    pub fn pan_by(&mut self, delta_x: f32, delta_y: f32) {
        self.pan.0 += delta_x;
        self.pan.1 += delta_y;
        // When manually panning, switch to manual mode
        self.fit_mode = FitMode::Manual;
    }

    /// Zoom at a specific point (in clip space).
    ///
    /// Note: Call `sync_with_bounds()` before this to ensure zoom is properly initialized.
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

    /// Zoom in by a standard factor.
    ///
    /// Note: Call `sync_with_bounds()` before this to ensure zoom is properly initialized.
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * ZOOM_FACTOR).clamp(ZOOM_MIN, ZOOM_MAX);
        self.fit_mode = FitMode::Manual;
    }

    /// Zoom out by a standard factor.
    ///
    /// Note: Call `sync_with_bounds()` before this to ensure zoom is properly initialized.
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

/// Scroll thumb drag data
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScrollDragData {
    /// Offset within thumb where drag started
    pub thumb_offset: f32,
}

/// Scroll thumb drag interaction state
pub type ScrollDragState = DragState<ScrollDragData>;

/// Extension methods for ScrollDragState
pub trait ScrollDragExt {
    /// Get the thumb offset if dragging
    fn thumb_offset(&self) -> Option<f32>;
}

impl ScrollDragExt for ScrollDragState {
    fn thumb_offset(&self) -> Option<f32> {
        self.data().map(|d| d.thumb_offset)
    }
}

/// State for scrollable containers
///
/// This type is Copy since all fields are Copy.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollState {
    /// Scroll offset (x, y)
    pub offset: (f32, f32),
    /// Velocity for momentum scrolling
    pub(crate) velocity: (f32, f32),
    /// Drag interaction state for scrollbar thumb
    pub(crate) drag: ScrollDragState,
}

impl ScrollState {
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
        let new_offset =
            (self.scroll_offset as isize + delta).clamp(0, max_scroll as isize) as usize;
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
///
/// This type is Copy since it's just a single bool.
#[derive(Debug, Clone, Copy, Default)]
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

/// Which RGB slider is being dragged in a color picker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorPickerDragging {
    #[default]
    None,
    Red,
    Green,
    Blue,
}

/// State for color picker widgets
///
/// This type is Copy since it contains only simple types.
#[derive(Debug, Clone, Copy, Default)]
pub struct ColorPickerState {
    /// Which slider is currently being dragged
    pub dragging: ColorPickerDragging,
}

impl ColorPickerState {
    /// Check if any slider is being dragged
    pub fn is_dragging(&self) -> bool {
        self.dragging != ColorPickerDragging::None
    }

    /// Start dragging a slider
    pub fn start_drag(&mut self, slider: ColorPickerDragging) {
        self.dragging = slider;
    }

    /// Stop dragging
    pub fn stop_drag(&mut self) {
        self.dragging = ColorPickerDragging::None;
    }
}

/// State for text input fields
///
/// Note: Undo/redo is handled externally via `UndoStack<T>`. Use the `on_undo_point`
/// callback on the widget to know when to save an undo snapshot.
///
/// This type is Copy since it contains only small primitive types.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextInputState {
    /// Cursor position (character index)
    pub cursor: usize,
    /// Selection range (start, end) if any
    pub selection: Option<(usize, usize)>,
    /// Whether the input is focused
    pub is_focused: bool,
}

impl TextInputState {
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

/// Shared undo/redo history for text input fields
///
/// This encapsulates the common undo/redo logic used by SliderState and NumberInputState
/// to avoid code duplication.
#[derive(Debug, Clone, Default)]
pub struct TextUndoHistory {
    undo_stack: Vec<TextSnapshot>,
    redo_stack: Vec<TextSnapshot>,
}

impl TextUndoHistory {
    /// Push current state to undo stack (call before making changes)
    pub fn push(&mut self, snapshot: TextSnapshot) {
        self.undo_stack.push(snapshot);
        self.redo_stack.clear();
        while self.undo_stack.len() > UNDO_STACK_LIMIT {
            self.undo_stack.remove(0);
        }
    }

    /// Undo: returns previous state if available
    pub fn undo(&mut self, current: TextSnapshot) -> Option<TextSnapshot> {
        let prev = self.undo_stack.pop()?;
        self.redo_stack.push(current);
        Some(prev)
    }

    /// Redo: returns next state if available
    pub fn redo(&mut self, current: TextSnapshot) -> Option<TextSnapshot> {
        let next = self.redo_stack.pop()?;
        self.undo_stack.push(current);
        Some(next)
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

/// Text editing state with integrated undo/redo support
///
/// This struct combines text, cursor, selection, and undo stack into a single
/// reusable component that can be embedded in widget states.
#[derive(Debug, Clone)]
pub struct TextEditState {
    /// The text content
    pub text: String,
    /// Cursor position (character index)
    pub cursor: usize,
    /// Selection range (start, end) if any
    pub selection: Option<(usize, usize)>,
    /// Undo stack for text changes
    undo: UndoStack<TextSnapshot>,
}

impl Default for TextEditState {
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            selection: None,
            undo: UndoStack::default(),
        }
    }
}

impl TextEditState {
    /// Create a new text edit state with initial text
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.len();
        Self {
            text,
            cursor,
            selection: None,
            undo: UndoStack::default(),
        }
    }

    /// Create a snapshot of current state
    fn snapshot(&self) -> TextSnapshot {
        TextSnapshot {
            text: self.text.clone(),
            cursor: self.cursor,
        }
    }

    /// Push current state to undo stack (call BEFORE making changes)
    pub fn push_undo(&mut self) {
        self.undo.push(self.snapshot());
    }

    /// Undo the last change, returns true if successful
    pub fn undo(&mut self) -> bool {
        let current = self.snapshot();
        if let Some(prev) = self.undo.undo(current) {
            self.text = prev.text;
            self.cursor = prev.cursor;
            self.selection = None;
            true
        } else {
            false
        }
    }

    /// Redo the last undone change, returns true if successful
    pub fn redo(&mut self) -> bool {
        let current = self.snapshot();
        if let Some(next) = self.undo.redo(current) {
            self.text = next.text;
            self.cursor = next.cursor;
            self.selection = None;
            true
        } else {
            false
        }
    }

    /// Clear undo/redo history
    pub fn clear_history(&mut self) {
        self.undo.clear();
    }

    /// Set the text content, updating cursor to end
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor = self.text.len();
        self.selection = None;
    }
}

/// Slider thumb drag interaction state (alias for DragState<()>)
pub type SliderDragState = DragState<()>;

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
    /// Text undo/redo history (for Ctrl+Z/Ctrl+Y in input field)
    pub(crate) input_history: TextUndoHistory,
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
            input_history: TextUndoHistory::default(),
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
            input_history: TextUndoHistory::default(),
        }
    }

    /// Push current text state to undo stack (call before making changes)
    pub fn push_text_undo(&mut self) {
        self.input_history.push(TextSnapshot {
            text: self.input_text.clone(),
            cursor: self.input_cursor,
        });
    }

    /// Undo text change (Ctrl+Z)
    pub fn text_undo(&mut self) -> bool {
        let current = TextSnapshot {
            text: self.input_text.clone(),
            cursor: self.input_cursor,
        };
        if let Some(snapshot) = self.input_history.undo(current) {
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
        let current = TextSnapshot {
            text: self.input_text.clone(),
            cursor: self.input_cursor,
        };
        if let Some(snapshot) = self.input_history.redo(current) {
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
        self.input_history.clear();
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
    /// Text undo/redo history (for Ctrl+Z/Ctrl+Y)
    pub(crate) history: TextUndoHistory,
}

impl Default for NumberInputState {
    fn default() -> Self {
        Self {
            text: String::from("0"),
            cursor: 1,
            is_focused: false,
            selection: None,
            history: TextUndoHistory::default(),
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
            history: TextUndoHistory::default(),
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
        self.history.push(TextSnapshot {
            text: self.text.clone(),
            cursor: self.cursor,
        });
    }

    /// Undo text change (Ctrl+Z) - returns true if undo was performed
    pub fn undo(&mut self) -> bool {
        let current = TextSnapshot {
            text: self.text.clone(),
            cursor: self.cursor,
        };
        if let Some(snapshot) = self.history.undo(current) {
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
        let current = TextSnapshot {
            text: self.text.clone(),
            cursor: self.cursor,
        };
        if let Some(snapshot) = self.history.redo(current) {
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
        self.history.clear();
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

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // UndoStack Tests
    // =========================================================================

    #[test]
    fn undo_stack_new_empty() {
        let stack: UndoStack<i32> = UndoStack::new(10);
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 0);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn undo_stack_push_and_undo() {
        let mut stack = UndoStack::new(10);

        // Push state 1
        stack.push(1);
        assert_eq!(stack.undo_count(), 1);
        assert!(stack.can_undo());

        // Current state is 2, undo should return 1
        let prev = stack.undo(2);
        assert_eq!(prev, Some(1));
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 1);
    }

    #[test]
    fn undo_stack_redo() {
        let mut stack = UndoStack::new(10);

        stack.push(1);
        let prev = stack.undo(2).unwrap();
        assert_eq!(prev, 1);

        // Redo should return 2
        let next = stack.redo(1);
        assert_eq!(next, Some(2));
        assert_eq!(stack.undo_count(), 1);
        assert_eq!(stack.redo_count(), 0);
    }

    #[test]
    fn undo_stack_push_clears_redo() {
        let mut stack = UndoStack::new(10);

        stack.push(1);
        stack.undo(2);
        assert!(stack.can_redo());

        // Push a new state - this should clear redo stack
        stack.push(3);
        assert!(!stack.can_redo());
        assert_eq!(stack.redo_count(), 0);
    }

    #[test]
    fn undo_stack_max_history() {
        let mut stack = UndoStack::new(3);

        stack.push(1);
        stack.push(2);
        stack.push(3);
        stack.push(4); // Should remove 1

        assert_eq!(stack.undo_count(), 3);

        // Undo should get 4, then 3, then 2 (1 was removed)
        assert_eq!(stack.undo(5), Some(4));
        assert_eq!(stack.undo(4), Some(3));
        assert_eq!(stack.undo(3), Some(2));
        assert_eq!(stack.undo(2), None); // 1 was removed due to max_history
    }

    #[test]
    fn undo_stack_clear() {
        let mut stack = UndoStack::new(10);

        stack.push(1);
        stack.push(2);
        stack.undo(3);

        assert!(stack.can_undo());
        assert!(stack.can_redo());

        stack.clear();

        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 0);
    }

    // =========================================================================
    // DragState Tests
    // =========================================================================

    #[test]
    fn drag_state_default_is_idle() {
        let drag: DragState<i32> = DragState::default();
        assert!(!drag.is_dragging());
        assert_eq!(drag.data(), None);
    }

    #[test]
    fn drag_state_start_drag() {
        let mut drag: DragState<i32> = DragState::Idle;
        drag.start_drag();
        assert!(drag.is_dragging());
        assert_eq!(drag.data(), Some(&0)); // i32::default() is 0
    }

    #[test]
    fn drag_state_start_drag_with_data() {
        let mut drag: DragState<i32> = DragState::Idle;
        drag.start_drag_with(42);
        assert!(drag.is_dragging());
        assert_eq!(drag.data(), Some(&42));
    }

    #[test]
    fn drag_state_stop_drag() {
        let mut drag = DragState::Dragging(100);
        drag.stop_drag();
        assert!(!drag.is_dragging());
        assert_eq!(drag.data(), None);
    }

    #[test]
    fn drag_state_data_mut() {
        let mut drag = DragState::Dragging(10);
        if let Some(data) = drag.data_mut() {
            *data = 20;
        }
        assert_eq!(drag.data(), Some(&20));
    }

    // =========================================================================
    // PanDragState Tests
    // =========================================================================

    #[test]
    fn pan_drag_state_last_pos() {
        let drag = DragState::Dragging(PanDragData {
            last_pos: (100.0, 200.0),
        });
        assert_eq!(drag.last_pos(), Some((100.0, 200.0)));
    }

    #[test]
    fn pan_drag_state_update_pos() {
        let mut drag = DragState::Dragging(PanDragData {
            last_pos: (0.0, 0.0),
        });
        drag.update_pos((50.0, 75.0));
        assert_eq!(drag.last_pos(), Some((50.0, 75.0)));
    }

    // =========================================================================
    // TextUndoHistory Tests
    // =========================================================================

    #[test]
    fn text_undo_history_push_and_undo() {
        let mut history = TextUndoHistory::default();

        let snap1 = TextSnapshot {
            text: "hello".to_string(),
            cursor: 5,
        };
        history.push(snap1);

        let current = TextSnapshot {
            text: "hello world".to_string(),
            cursor: 11,
        };

        let prev = history.undo(current).unwrap();
        assert_eq!(prev.text, "hello");
        assert_eq!(prev.cursor, 5);
    }

    #[test]
    fn text_undo_history_redo() {
        let mut history = TextUndoHistory::default();

        let snap1 = TextSnapshot {
            text: "a".to_string(),
            cursor: 1,
        };
        history.push(snap1);

        let snap2 = TextSnapshot {
            text: "ab".to_string(),
            cursor: 2,
        };

        let prev = history.undo(snap2.clone()).unwrap();
        assert_eq!(prev.text, "a");

        let next = history.redo(prev).unwrap();
        assert_eq!(next.text, "ab");
        assert_eq!(next.cursor, 2);
    }

    #[test]
    fn text_undo_history_max_limit() {
        let mut history = TextUndoHistory::default();

        // Push more than UNDO_STACK_LIMIT snapshots
        for i in 0..UNDO_STACK_LIMIT + 5 {
            history.push(TextSnapshot {
                text: i.to_string(),
                cursor: 0,
            });
        }

        // Should only keep UNDO_STACK_LIMIT items
        // The first 5 should have been removed
        let current = TextSnapshot {
            text: "current".to_string(),
            cursor: 0,
        };

        let mut count = 0;
        let mut temp_history = history.clone();
        let mut temp_current = current.clone();

        while temp_history.undo(temp_current.clone()).is_some() {
            count += 1;
            temp_current = TextSnapshot {
                text: format!("temp{}", count),
                cursor: 0,
            };
            if count > UNDO_STACK_LIMIT + 10 {
                break; // Safety check
            }
        }

        assert_eq!(count, UNDO_STACK_LIMIT);
    }

    // =========================================================================
    // ImageViewerState Tests
    // =========================================================================

    #[test]
    fn image_viewer_state_default() {
        let state = ImageViewerState::default();
        assert_eq!(state.pan, (0.0, 0.0));
        assert_eq!(state.zoom, 1.0);
        assert_eq!(state.fit_mode, FitMode::FitToView);
    }

    #[test]
    fn image_viewer_state_zoom_in() {
        let mut state = ImageViewerState::new();
        let original_zoom = state.zoom;
        state.zoom_in();
        assert!(state.zoom > original_zoom);
        assert_eq!(state.fit_mode, FitMode::Manual);
    }

    #[test]
    fn image_viewer_state_zoom_out() {
        let mut state = ImageViewerState::new();
        state.zoom = 2.0;
        state.zoom_out();
        assert!(state.zoom < 2.0);
        assert_eq!(state.fit_mode, FitMode::Manual);
    }

    #[test]
    fn image_viewer_state_zoom_clamp() {
        let mut state = ImageViewerState::new();

        // Zoom in repeatedly - should clamp to ZOOM_MAX
        for _ in 0..100 {
            state.zoom_in();
        }
        assert_eq!(state.zoom, ZOOM_MAX);

        // Zoom out repeatedly - should clamp to ZOOM_MIN
        for _ in 0..100 {
            state.zoom_out();
        }
        assert_eq!(state.zoom, ZOOM_MIN);
    }

    #[test]
    fn image_viewer_state_calculate_fit_zoom() {
        // Image 800x600, view 400x300 -> zoom should be 0.5
        let zoom = ImageViewerState::calculate_fit_zoom(400.0, 300.0, 800, 600);
        assert_eq!(zoom, 0.5);

        // Image 200x200, view 400x400 -> zoom should be 2.0
        let zoom = ImageViewerState::calculate_fit_zoom(400.0, 400.0, 200, 200);
        assert_eq!(zoom, 2.0);

        // Wide image: 1600x400, view 800x600 -> constrained by width
        let zoom = ImageViewerState::calculate_fit_zoom(800.0, 600.0, 1600, 400);
        assert_eq!(zoom, 0.5); // 800 / 1600
    }

    #[test]
    fn image_viewer_state_pan_by() {
        let mut state = ImageViewerState::new();
        state.pan_by(10.0, 20.0);
        assert_eq!(state.pan, (10.0, 20.0));
        assert_eq!(state.fit_mode, FitMode::Manual);

        state.pan_by(5.0, -10.0);
        assert_eq!(state.pan, (15.0, 10.0));
    }

    // =========================================================================
    // CollapsibleState Tests
    // =========================================================================

    #[test]
    fn collapsible_state_new() {
        let expanded = CollapsibleState::new(true);
        assert!(expanded.is_expanded);

        let collapsed = CollapsibleState::new(false);
        assert!(!collapsed.is_expanded);
    }

    #[test]
    fn collapsible_state_toggle() {
        let mut state = CollapsibleState::expanded();
        assert!(state.is_expanded);

        state.toggle();
        assert!(!state.is_expanded);

        state.toggle();
        assert!(state.is_expanded);
    }

    // =========================================================================
    // DropdownState Tests
    // =========================================================================

    #[test]
    fn dropdown_state_open_close() {
        let mut state = DropdownState::default();
        assert!(!state.is_open);

        state.open();
        assert!(state.is_open);
        assert_eq!(state.highlighted, Some(0));

        state.close();
        assert!(!state.is_open);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn dropdown_state_toggle() {
        let mut state = DropdownState::default();

        state.toggle();
        assert!(state.is_open);

        state.toggle();
        assert!(!state.is_open);
    }

    #[test]
    fn dropdown_state_scroll_by() {
        let mut state = DropdownState::default();

        // 20 items, 10 visible -> max scroll = 10
        state.scroll_by(5, 20, 10);
        assert_eq!(state.scroll_offset, 5);

        state.scroll_by(10, 20, 10);
        assert_eq!(state.scroll_offset, 10); // Clamped to max

        state.scroll_by(-5, 20, 10);
        assert_eq!(state.scroll_offset, 5);

        state.scroll_by(-20, 20, 10);
        assert_eq!(state.scroll_offset, 0); // Clamped to min
    }

    #[test]
    fn dropdown_state_ensure_highlighted_visible() {
        let mut state = DropdownState::default();
        state.scroll_offset = 5;
        state.highlighted = Some(2); // Above visible area

        state.ensure_highlighted_visible(5);
        assert_eq!(state.scroll_offset, 2); // Scrolled up

        state.highlighted = Some(10); // Below visible area (5-9 visible)
        state.ensure_highlighted_visible(5);
        assert_eq!(state.scroll_offset, 6); // Scrolled down
    }

    // =========================================================================
    // NumberInputState Tests
    // =========================================================================

    #[test]
    fn number_input_state_new() {
        let state = NumberInputState::new(42.5);
        assert!(state.text.contains("42.5"));
        assert!(!state.is_focused);
    }

    #[test]
    fn number_input_state_value() {
        let state = NumberInputState::new(123.45);
        assert_eq!(state.value(), Some(123.45));

        let mut invalid = NumberInputState::new(0.0);
        invalid.text = "not a number".to_string();
        assert_eq!(invalid.value(), None);
    }

    #[test]
    fn number_input_state_set_value() {
        let mut state = NumberInputState::new(0.0);
        state.set_value(99.9);
        assert!(state.text.contains("99.9"));
        assert_eq!(state.selection, None);
    }

    #[test]
    fn number_input_state_focus_blur() {
        let mut state = NumberInputState::new(0.0);

        state.focus();
        assert!(state.is_focused);
        assert!(state.selection.is_some()); // Focuses select all

        state.blur();
        assert!(!state.is_focused);
        assert_eq!(state.selection, None);
    }

    #[test]
    fn number_input_state_undo_redo() {
        let mut state = NumberInputState::new(10.0);

        state.push_undo();
        state.text = "20".to_string();
        state.cursor = 2;

        let undone = state.undo();
        assert!(undone);
        assert!(state.text.contains("10"));

        let redone = state.redo();
        assert!(redone);
        assert_eq!(state.text, "20");
    }
}
