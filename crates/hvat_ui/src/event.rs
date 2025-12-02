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
