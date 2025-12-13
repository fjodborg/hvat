//! Demo showcasing undo/redo functionality with counter, slider, and text input

use crate::event::{Event, KeyCode};
use crate::state::{SliderState, TextInputState};
use crate::{col, Element, Length};

/// A simple generic undo stack
#[derive(Debug, Clone)]
pub struct SimpleUndoStack<T: Clone> {
    /// Stack of states that can be undone
    undo_stack: Vec<T>,
    /// Stack of states that can be redone
    redo_stack: Vec<T>,
    /// Maximum history size
    max_history: usize,
}

impl<T: Clone> Default for SimpleUndoStack<T> {
    fn default() -> Self {
        Self::new(50)
    }
}

impl<T: Clone> SimpleUndoStack<T> {
    /// Create a new undo stack with max history size
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Push a state to the undo stack (called before making a change)
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
    /// The current state should be passed so it can be pushed to redo stack
    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo_stack.pop()?;
        self.redo_stack.push(current);
        Some(prev)
    }

    /// Redo: returns the next state, or None if nothing to redo
    /// The current state should be passed so it can be pushed to undo stack
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

/// Snapshot of entire demo state for global undo
#[derive(Debug, Clone)]
struct DemoSnapshot {
    counter: i32,
    slider_value: f32,
    text_value: String,
}

/// Undo demo state
pub struct UndoDemo {
    /// Current counter value
    pub counter: i32,
    /// Current slider value
    pub slider_state: SliderState,
    /// Text input value
    pub text_value: String,
    /// Text input state
    pub text_input_state: TextInputState,
    /// Global undo stack (for entire demo state)
    global_undo: SimpleUndoStack<DemoSnapshot>,
    /// Whether slider drag is in progress (don't record each frame)
    slider_dragging: bool,
    /// Whether slider input field is focused (for tracking edit start)
    slider_input_was_focused: bool,
    /// Whether text input is focused (for tracking edit start)
    text_input_was_focused: bool,
    /// Snapshot at start of drag or input focus
    drag_start_snapshot: Option<DemoSnapshot>,
}

/// Undo demo messages
#[derive(Clone)]
pub enum UndoMessage {
    Increment,
    Decrement,
    SliderChanged(SliderState),
    TextInputChanged(String, TextInputState),
    /// Global undo (Ctrl+Z)
    Undo,
    /// Global redo (Ctrl+Y or Ctrl+Shift+Z)
    Redo,
    ClearHistory,
}

impl Default for UndoDemo {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoDemo {
    pub fn new() -> Self {
        Self {
            counter: 0,
            slider_state: SliderState::new(50.0),
            text_value: String::new(),
            text_input_state: TextInputState::new(),
            global_undo: SimpleUndoStack::new(50),
            slider_dragging: false,
            slider_input_was_focused: false,
            text_input_was_focused: false,
            drag_start_snapshot: None,
        }
    }

    /// Create a snapshot of current state
    fn snapshot(&self) -> DemoSnapshot {
        DemoSnapshot {
            counter: self.counter,
            slider_value: self.slider_state.value,
            text_value: self.text_value.clone(),
        }
    }

    /// Restore from a snapshot
    fn restore(&mut self, snapshot: &DemoSnapshot) {
        self.counter = snapshot.counter;
        self.slider_state.set_value(snapshot.slider_value);
        self.text_value = snapshot.text_value.clone();
        // Reset cursor to end of text
        self.text_input_state.cursor = self.text_value.len();
        self.text_input_state.selection = None;
    }

    /// Handle keyboard events for undo/redo shortcuts
    /// Returns Some(message) if a shortcut was triggered
    pub fn handle_key_event(event: &Event) -> Option<UndoMessage> {
        if let Event::KeyPress { key, modifiers, .. } = event {
            if modifiers.ctrl {
                match key {
                    KeyCode::Z if modifiers.shift => {
                        // Ctrl+Shift+Z = Redo
                        log::debug!("Keyboard shortcut: Ctrl+Shift+Z (Redo)");
                        return Some(UndoMessage::Redo);
                    }
                    KeyCode::Z => {
                        // Ctrl+Z = Undo
                        log::debug!("Keyboard shortcut: Ctrl+Z (Undo)");
                        return Some(UndoMessage::Undo);
                    }
                    KeyCode::Y => {
                        // Ctrl+Y = Redo (Windows style)
                        log::debug!("Keyboard shortcut: Ctrl+Y (Redo)");
                        return Some(UndoMessage::Redo);
                    }
                    _ => {}
                }
            }
        }
        None
    }

    pub fn view<M: Clone + 'static>(
        &self,
        wrap: impl Fn(UndoMessage) -> M + Clone + 'static,
    ) -> Element<M> {
        let counter = self.counter;
        let slider_value = self.slider_state.value;
        let text_value = self.text_value.clone();
        let can_undo = self.global_undo.can_undo();
        let can_redo = self.global_undo.can_redo();
        let undo_count = self.global_undo.undo_count();
        let redo_count = self.global_undo.redo_count();

        let wrap1 = wrap.clone();
        let wrap2 = wrap.clone();
        let wrap3 = wrap.clone();
        let wrap4 = wrap.clone();
        let wrap5 = wrap.clone();
        let wrap6 = wrap.clone();
        let wrap7 = wrap.clone();

        col(move |c| {
            c.text("Undo/Redo Demo (Global)");
            c.text_sized("Demonstrates global undo/redo with keyboard shortcuts", 12.0);
            c.text("");

            // Counter section
            c.text("Counter:");
            c.row(|r| {
                r.button("-")
                    .width(Length::Fixed(40.0))
                    .on_click(wrap1(UndoMessage::Decrement));
                r.text(format!("  {}  ", counter));
                r.button("+")
                    .width(Length::Fixed(40.0))
                    .on_click(wrap2(UndoMessage::Increment));
            });
            c.text("");

            // Slider section
            c.text("Slider (records on release):");
            c.row(|r| {
                r.slider(0.0, 100.0)
                    .state(&self.slider_state)
                    .show_input(true)
                    .width(Length::Fixed(300.0))
                    .on_change({
                        let w = wrap3.clone();
                        move |s| w(UndoMessage::SliderChanged(s))
                    });
                r.text(format!("Value: {:.1}", slider_value));
            });
            c.text("");

            // Text input section
            c.text("Text Input (records on blur):");
            c.row(|r| {
                r.text_input()
                    .value(&self.text_value)
                    .placeholder("Type something...")
                    .state(&self.text_input_state)
                    .width(Length::Fixed(300.0))
                    .on_change({
                        let w = wrap7.clone();
                        move |s, state| w(UndoMessage::TextInputChanged(s, state))
                    });
            });
            c.text(format!("Text: \"{}\"", text_value));
            c.text("");

            // Undo/Redo controls
            c.text("Global Undo/Redo:");
            c.row(|r| {
                if can_undo {
                    r.button("Undo (Ctrl+Z)")
                        .width(Length::Fixed(120.0))
                        .on_click(wrap4(UndoMessage::Undo));
                } else {
                    r.button("Undo (Ctrl+Z)").width(Length::Fixed(120.0)).build();
                }
                if can_redo {
                    r.button("Redo (Ctrl+Y)")
                        .width(Length::Fixed(120.0))
                        .on_click(wrap5(UndoMessage::Redo));
                } else {
                    r.button("Redo (Ctrl+Y)").width(Length::Fixed(120.0)).build();
                }
                r.button("Clear History")
                    .width(Length::Fixed(120.0))
                    .on_click(wrap6(UndoMessage::ClearHistory));
            });
            c.text(format!("History: {} undo, {} redo steps", undo_count, redo_count));
            c.text("");

            // Instructions
            c.text_sized("How it works:", 14.0);
            c.text_sized("• Global undo tracks entire demo state (counter + slider + text)", 11.0);
            c.text_sized("• Counter changes are recorded immediately", 11.0);
            c.text_sized("• Slider changes are recorded when you release the mouse", 11.0);
            c.text_sized("• Text changes are recorded when you click outside (blur)", 11.0);
            c.text_sized("• Undo/Redo restores the full state snapshot", 11.0);
            c.text("");
            c.text_sized("Keyboard shortcuts:", 14.0);
            c.text_sized("• Ctrl+Z = Undo", 11.0);
            c.text_sized("• Ctrl+Y = Redo (Windows style)", 11.0);
            c.text_sized("• Ctrl+Shift+Z = Redo (Mac style)", 11.0);
        })
    }

    pub fn update(&mut self, message: UndoMessage) {
        match message {
            UndoMessage::Increment => {
                // Record current state before change
                self.global_undo.push(self.snapshot());
                self.counter += 1;
                log::info!("Counter incremented to {}", self.counter);
            }
            UndoMessage::Decrement => {
                self.global_undo.push(self.snapshot());
                self.counter -= 1;
                log::info!("Counter decremented to {}", self.counter);
            }
            UndoMessage::SliderChanged(state) => {
                // Track drag state to only record on release
                let was_dragging = self.slider_dragging;
                self.slider_dragging = state.dragging;

                // Track input field focus state
                let was_input_focused = self.slider_input_was_focused;
                self.slider_input_was_focused = state.input_focused;

                // Handle drag start/end
                if !was_dragging && state.dragging {
                    // Just started dragging - save snapshot
                    self.drag_start_snapshot = Some(self.snapshot());
                } else if was_dragging && !state.dragging {
                    // Just released drag - record to undo if value changed
                    if let Some(snapshot) = self.drag_start_snapshot.take() {
                        if (snapshot.slider_value - state.value).abs() > 0.001 {
                            self.global_undo.push(snapshot);
                            log::info!("Recorded slider drag change to {:.1}", state.value);
                        }
                    }
                }

                // Handle input field focus start/end
                if !was_input_focused && state.input_focused {
                    // Just focused input field - save snapshot
                    self.drag_start_snapshot = Some(self.snapshot());
                } else if was_input_focused && !state.input_focused {
                    // Just unfocused input field - record to undo if value changed
                    if let Some(snapshot) = self.drag_start_snapshot.take() {
                        if (snapshot.slider_value - state.value).abs() > 0.001 {
                            self.global_undo.push(snapshot);
                            log::info!("Recorded slider input change to {:.1}", state.value);
                        }
                    }
                }

                self.slider_state = state;
            }
            UndoMessage::TextInputChanged(text, state) => {
                // Track focus state to record on blur
                let was_focused = self.text_input_was_focused;
                self.text_input_was_focused = state.is_focused;

                // Handle focus start
                if !was_focused && state.is_focused {
                    // Just focused - save snapshot
                    self.drag_start_snapshot = Some(self.snapshot());
                }
                // Handle blur (focus end)
                else if was_focused && !state.is_focused {
                    // Just blurred - record to undo if text changed
                    if let Some(snapshot) = self.drag_start_snapshot.take() {
                        if snapshot.text_value != text {
                            self.global_undo.push(snapshot);
                            log::info!("Recorded text change to '{}'", text);
                        }
                    }
                }

                self.text_value = text;
                self.text_input_state = state;
            }
            UndoMessage::Undo => {
                let current = self.snapshot();
                if let Some(prev) = self.global_undo.undo(current) {
                    self.restore(&prev);
                    log::info!("Undo: counter={}, slider={:.1}, text='{}'",
                        self.counter, self.slider_state.value, self.text_value);
                }
            }
            UndoMessage::Redo => {
                let current = self.snapshot();
                if let Some(next) = self.global_undo.redo(current) {
                    self.restore(&next);
                    log::info!("Redo: counter={}, slider={:.1}, text='{}'",
                        self.counter, self.slider_state.value, self.text_value);
                }
            }
            UndoMessage::ClearHistory => {
                self.global_undo.clear();
                log::info!("Cleared undo history");
            }
        }
    }
}
