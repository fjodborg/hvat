//! Event types for user input handling

/// Keyboard key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Navigation
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,

    // Editing
    Backspace,
    Delete,
    Insert,
    Enter,
    Tab,
    Escape,
    Space,

    // Modifiers (for detecting key state)
    Shift,
    Control,
    Alt,
    Super,

    // Symbols
    Minus,
    Equal,
    Plus,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Grave,

    // Other
    Unknown,
}

impl KeyCode {
    /// Convert from winit KeyCode
    pub fn from_winit(key: winit::keyboard::KeyCode) -> Self {
        use winit::keyboard::KeyCode as WK;
        match key {
            WK::KeyA => KeyCode::A,
            WK::KeyB => KeyCode::B,
            WK::KeyC => KeyCode::C,
            WK::KeyD => KeyCode::D,
            WK::KeyE => KeyCode::E,
            WK::KeyF => KeyCode::F,
            WK::KeyG => KeyCode::G,
            WK::KeyH => KeyCode::H,
            WK::KeyI => KeyCode::I,
            WK::KeyJ => KeyCode::J,
            WK::KeyK => KeyCode::K,
            WK::KeyL => KeyCode::L,
            WK::KeyM => KeyCode::M,
            WK::KeyN => KeyCode::N,
            WK::KeyO => KeyCode::O,
            WK::KeyP => KeyCode::P,
            WK::KeyQ => KeyCode::Q,
            WK::KeyR => KeyCode::R,
            WK::KeyS => KeyCode::S,
            WK::KeyT => KeyCode::T,
            WK::KeyU => KeyCode::U,
            WK::KeyV => KeyCode::V,
            WK::KeyW => KeyCode::W,
            WK::KeyX => KeyCode::X,
            WK::KeyY => KeyCode::Y,
            WK::KeyZ => KeyCode::Z,
            WK::Digit0 => KeyCode::Key0,
            WK::Digit1 => KeyCode::Key1,
            WK::Digit2 => KeyCode::Key2,
            WK::Digit3 => KeyCode::Key3,
            WK::Digit4 => KeyCode::Key4,
            WK::Digit5 => KeyCode::Key5,
            WK::Digit6 => KeyCode::Key6,
            WK::Digit7 => KeyCode::Key7,
            WK::Digit8 => KeyCode::Key8,
            WK::Digit9 => KeyCode::Key9,
            WK::F1 => KeyCode::F1,
            WK::F2 => KeyCode::F2,
            WK::F3 => KeyCode::F3,
            WK::F4 => KeyCode::F4,
            WK::F5 => KeyCode::F5,
            WK::F6 => KeyCode::F6,
            WK::F7 => KeyCode::F7,
            WK::F8 => KeyCode::F8,
            WK::F9 => KeyCode::F9,
            WK::F10 => KeyCode::F10,
            WK::F11 => KeyCode::F11,
            WK::F12 => KeyCode::F12,
            WK::ArrowUp => KeyCode::Up,
            WK::ArrowDown => KeyCode::Down,
            WK::ArrowLeft => KeyCode::Left,
            WK::ArrowRight => KeyCode::Right,
            WK::Home => KeyCode::Home,
            WK::End => KeyCode::End,
            WK::PageUp => KeyCode::PageUp,
            WK::PageDown => KeyCode::PageDown,
            WK::Backspace => KeyCode::Backspace,
            WK::Delete => KeyCode::Delete,
            WK::Insert => KeyCode::Insert,
            WK::Enter => KeyCode::Enter,
            WK::Tab => KeyCode::Tab,
            WK::Escape => KeyCode::Escape,
            WK::Space => KeyCode::Space,
            WK::ShiftLeft | WK::ShiftRight => KeyCode::Shift,
            WK::ControlLeft | WK::ControlRight => KeyCode::Control,
            WK::AltLeft | WK::AltRight => KeyCode::Alt,
            WK::SuperLeft | WK::SuperRight => KeyCode::Super,
            WK::Minus => KeyCode::Minus,
            WK::Equal => KeyCode::Equal,
            WK::BracketLeft => KeyCode::BracketLeft,
            WK::BracketRight => KeyCode::BracketRight,
            WK::Backslash => KeyCode::Backslash,
            WK::Semicolon => KeyCode::Semicolon,
            WK::Quote => KeyCode::Quote,
            WK::Comma => KeyCode::Comma,
            WK::Period => KeyCode::Period,
            WK::Slash => KeyCode::Slash,
            WK::Backquote => KeyCode::Grave,
            _ => KeyCode::Unknown,
        }
    }
}

/// Keyboard modifiers state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl KeyModifiers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_winit(modifiers: winit::event::Modifiers) -> Self {
        let state = modifiers.state();
        Self {
            shift: state.shift_key(),
            ctrl: state.control_key(),
            alt: state.alt_key(),
            super_key: state.super_key(),
        }
    }
}

/// Mouse button types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

impl MouseButton {
    pub fn from_winit(button: winit::event::MouseButton) -> Self {
        match button {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Other(n) => MouseButton::Other(n),
            winit::event::MouseButton::Back => MouseButton::Other(4),
            winit::event::MouseButton::Forward => MouseButton::Other(5),
        }
    }
}

/// UI events that widgets can handle
#[derive(Debug, Clone)]
pub enum Event {
    /// Mouse button pressed
    MousePress {
        button: MouseButton,
        position: (f32, f32),
        modifiers: KeyModifiers,
    },

    /// Mouse button released
    MouseRelease {
        button: MouseButton,
        position: (f32, f32),
        modifiers: KeyModifiers,
    },

    /// Mouse moved
    MouseMove {
        position: (f32, f32),
        modifiers: KeyModifiers,
    },

    /// Mouse scroll wheel
    MouseScroll {
        delta: (f32, f32),
        position: (f32, f32),
        modifiers: KeyModifiers,
    },

    /// Key pressed
    KeyPress {
        key: KeyCode,
        modifiers: KeyModifiers,
    },

    /// Key released
    KeyRelease {
        key: KeyCode,
        modifiers: KeyModifiers,
    },

    /// Text input (for text fields)
    TextInput { text: String },

    /// Widget gained focus
    FocusGained,

    /// Widget lost focus
    FocusLost,

    /// Global mouse press event - sent to all widgets regardless of bounds
    /// Used to blur focused widgets when clicking elsewhere
    GlobalMousePress {
        button: MouseButton,
        position: (f32, f32),
    },
}

impl Event {
    /// Get the position if this is a mouse event
    pub fn position(&self) -> Option<(f32, f32)> {
        match self {
            Event::MousePress { position, .. }
            | Event::MouseRelease { position, .. }
            | Event::MouseMove { position, .. }
            | Event::MouseScroll { position, .. }
            | Event::GlobalMousePress { position, .. } => Some(*position),
            _ => None,
        }
    }

    /// Get the modifiers for this event
    pub fn modifiers(&self) -> KeyModifiers {
        match self {
            Event::MousePress { modifiers, .. }
            | Event::MouseRelease { modifiers, .. }
            | Event::MouseMove { modifiers, .. }
            | Event::MouseScroll { modifiers, .. }
            | Event::KeyPress { modifiers, .. }
            | Event::KeyRelease { modifiers, .. } => *modifiers,
            _ => KeyModifiers::default(),
        }
    }
}
