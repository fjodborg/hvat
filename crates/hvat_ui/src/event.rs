use crate::Point;

/// Events that widgets can respond to.
#[derive(Debug, Clone)]
pub enum Event {
    /// Mouse button pressed.
    MousePressed {
        button: MouseButton,
        position: Point,
    },
    /// Mouse button released.
    MouseReleased {
        button: MouseButton,
        position: Point,
    },
    /// Mouse moved.
    MouseMoved { position: Point },
    /// Mouse wheel scrolled.
    MouseWheel { delta: f32, position: Point },
    /// Keyboard key pressed.
    KeyPressed { key: Key, modifiers: Modifiers },
    /// Keyboard key released.
    KeyReleased { key: Key, modifiers: Modifiers },
    /// Window was resized (width, height).
    WindowResized { width: f32, height: f32 },
}

/// Result of handling an event, indicating whether it was consumed.
///
/// This allows widgets to signal whether an event was handled and should
/// stop propagating to other widgets, or if it should continue.
#[derive(Debug, Clone)]
pub enum EventResult<Message> {
    /// Event was not handled, no message produced.
    Ignored,
    /// Event was handled but no message produced (stops propagation).
    Consumed,
    /// Event produced a message (stops propagation).
    Message(Message),
}

impl<Message> EventResult<Message> {
    /// Returns true if this result indicates the event was handled.
    pub fn is_handled(&self) -> bool {
        !matches!(self, EventResult::Ignored)
    }

    /// Extract the message if one was produced.
    pub fn into_message(self) -> Option<Message> {
        match self {
            EventResult::Message(msg) => Some(msg),
            _ => None,
        }
    }

    /// Map the message type.
    pub fn map<F, B>(self, f: F) -> EventResult<B>
    where
        F: FnOnce(Message) -> B,
    {
        match self {
            EventResult::Ignored => EventResult::Ignored,
            EventResult::Consumed => EventResult::Consumed,
            EventResult::Message(msg) => EventResult::Message(f(msg)),
        }
    }
}

impl<Message> From<Option<Message>> for EventResult<Message> {
    /// Convert from legacy Option<Message> format for backwards compatibility.
    fn from(opt: Option<Message>) -> Self {
        match opt {
            Some(msg) => EventResult::Message(msg),
            None => EventResult::Ignored,
        }
    }
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

/// Keyboard keys (simplified set for now).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Escape,
    Backspace,
    Delete,
    Tab,
    Space,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}

/// Keyboard modifiers.
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}
