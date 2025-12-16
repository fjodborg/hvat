//! Demo showcasing undo/redo functionality with counter, slider, and text input

use crate::event::{Event, KeyCode};
use crate::state::{SliderState, TextInputState, UndoStack};
use crate::{col, Element, Length};
use std::cell::RefCell;
use std::rc::Rc;

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
    /// Uses Rc<RefCell> for interior mutability so on_undo_point closures can save snapshots
    global_undo: Rc<RefCell<UndoStack<DemoSnapshot>>>,
}

/// Undo demo messages
#[derive(Clone)]
pub enum UndoMessage {
    Increment,
    Decrement,
    SliderChanged(SliderState),
    TextInputChanged(String, TextInputState),
    /// Save undo snapshot (called by widget when edit starts)
    SaveSnapshot,
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
            global_undo: Rc::new(RefCell::new(UndoStack::new(50))),
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
        let can_undo = self.global_undo.borrow().can_undo();
        let can_redo = self.global_undo.borrow().can_redo();
        let undo_count = self.global_undo.borrow().undo_count();
        let redo_count = self.global_undo.borrow().redo_count();

        let wrap1 = wrap.clone();
        let wrap2 = wrap.clone();
        let wrap3 = wrap.clone();
        let wrap4 = wrap.clone();
        let wrap5 = wrap.clone();
        let wrap6 = wrap.clone();
        let wrap7 = wrap.clone();

        // Create snapshot closures that capture the Rc<RefCell<>>
        let snapshot = self.snapshot();
        let undo_stack1 = Rc::clone(&self.global_undo);
        let snapshot1 = snapshot.clone();
        let undo_stack2 = Rc::clone(&self.global_undo);
        let snapshot2 = snapshot.clone();

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
            c.text("Slider (records on drag start):");
            c.row(|r| {
                r.slider(0.0, 100.0)
                    .state(&self.slider_state)
                    .show_input(true)
                    .width(Length::Fixed(300.0))
                    .on_change({
                        let w = wrap3.clone();
                        move |s| w(UndoMessage::SliderChanged(s))
                    })
                    .on_undo_point({
                        let undo_stack = Rc::clone(&undo_stack1);
                        let snap = snapshot1.clone();
                        move || {
                            log::debug!("on_undo_point: saving snapshot for slider");
                            undo_stack.borrow_mut().push(snap.clone());
                        }
                    })
                    .build();
                r.text(format!("Value: {:.1}", slider_value));
            });
            c.text("");

            // Text input section
            c.text("Text Input (records on focus):");
            c.row(|r| {
                r.text_input()
                    .value(&self.text_value)
                    .placeholder("Type something...")
                    .state(&self.text_input_state)
                    .width(Length::Fixed(300.0))
                    .on_change({
                        let w = wrap7.clone();
                        move |s, state| w(UndoMessage::TextInputChanged(s, state))
                    })
                    .on_undo_point({
                        let undo_stack = Rc::clone(&undo_stack2);
                        let snap = snapshot2.clone();
                        move || {
                            log::debug!("on_undo_point: saving snapshot for text input");
                            undo_stack.borrow_mut().push(snap.clone());
                        }
                    })
                    .build();
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
            c.text_sized("• Slider snapshots are saved when drag/input starts", 11.0);
            c.text_sized("• Text snapshots are saved when input gains focus", 11.0);
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
                self.global_undo.borrow_mut().push(self.snapshot());
                self.counter += 1;
                log::info!("Counter incremented to {}", self.counter);
            }
            UndoMessage::Decrement => {
                self.global_undo.borrow_mut().push(self.snapshot());
                self.counter -= 1;
                log::info!("Counter decremented to {}", self.counter);
            }
            UndoMessage::SliderChanged(state) => {
                self.slider_state = state;
            }
            UndoMessage::TextInputChanged(text, state) => {
                self.text_value = text;
                self.text_input_state = state;
            }
            UndoMessage::SaveSnapshot => {
                // This message is no longer used - snapshots are saved via on_undo_point callbacks
                self.global_undo.borrow_mut().push(self.snapshot());
                log::debug!("Saved undo snapshot (via message)");
            }
            UndoMessage::Undo => {
                let current = self.snapshot();
                let prev = self.global_undo.borrow_mut().undo(current);
                if let Some(prev) = prev {
                    self.restore(&prev);
                    log::info!("Undo: counter={}, slider={:.1}, text='{}'",
                        self.counter, self.slider_state.value, self.text_value);
                }
            }
            UndoMessage::Redo => {
                let current = self.snapshot();
                let next = self.global_undo.borrow_mut().redo(current);
                if let Some(next) = next {
                    self.restore(&next);
                    log::info!("Redo: counter={}, slider={:.1}, text='{}'",
                        self.counter, self.slider_state.value, self.text_value);
                }
            }
            UndoMessage::ClearHistory => {
                self.global_undo.borrow_mut().clear();
                log::info!("Cleared undo history");
            }
        }
    }
}
