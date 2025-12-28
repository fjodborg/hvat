//! Customizable keybindings for HVAT.
//!
//! This module defines keybinding configuration for annotation tools and category selection.
//! Keybindings can be customized through the settings UI.
//!
//! Note: Settings persistence is not yet implemented. Keybindings reset on app restart.

use hvat_ui::KeyCode;

use crate::model::AnnotationTool;

/// Target for keybind capture - which binding is being set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeybindTarget {
    /// Binding for an annotation tool
    Tool(AnnotationTool),
    /// Binding for a category by index (0-based)
    Category(usize),
}

/// Maximum number of categories that can have hotkeys (0-9 keys).
pub const MAX_CATEGORY_HOTKEYS: usize = 10;

/// Keybinding configuration for the application.
///
/// Note: Serde traits not derived because KeyCode doesn't implement them.
/// When settings persistence is added, we'll need to convert KeyCode to/from strings.
#[derive(Debug, Clone)]
pub struct KeyBindings {
    /// Hotkey for Select tool
    pub tool_select: KeyCode,
    /// Hotkey for BoundingBox tool
    pub tool_bbox: KeyCode,
    /// Hotkey for Polygon tool
    pub tool_polygon: KeyCode,
    /// Hotkey for Point tool
    pub tool_point: KeyCode,

    /// Hotkeys for category selection (indices 0-9 map to categories 1-10)
    /// None means no hotkey assigned for that slot
    pub category_hotkeys: [Option<KeyCode>; MAX_CATEGORY_HOTKEYS],
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            // Default tool hotkeys: G for Select, E for BBox, R for Polygon, T for Point
            tool_select: KeyCode::S,
            tool_bbox: KeyCode::E,
            tool_polygon: KeyCode::R,
            tool_point: KeyCode::T,

            // Default category hotkeys: 1-9, 0 for categories 1-10
            category_hotkeys: [
                Some(KeyCode::Key1), // Category at index 0
                Some(KeyCode::Key2), // Category at index 1
                Some(KeyCode::Key3), // Category at index 2
                Some(KeyCode::Key4), // Category at index 3
                Some(KeyCode::Key5), // Category at index 4
                Some(KeyCode::Key6), // Category at index 5
                Some(KeyCode::Key7), // Category at index 6
                Some(KeyCode::Key8), // Category at index 7
                Some(KeyCode::Key9), // Category at index 8
                Some(KeyCode::Key0), // Category at index 9
            ],
        }
    }
}

impl KeyBindings {
    /// Create new keybindings with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the tool that corresponds to a key press, if any.
    pub fn tool_for_key(&self, key: KeyCode) -> Option<AnnotationTool> {
        if key == self.tool_select {
            Some(AnnotationTool::Select)
        } else if key == self.tool_bbox {
            Some(AnnotationTool::BoundingBox)
        } else if key == self.tool_polygon {
            Some(AnnotationTool::Polygon)
        } else if key == self.tool_point {
            Some(AnnotationTool::Point)
        } else {
            None
        }
    }

    /// Get the category index (0-based) that corresponds to a key press, if any.
    pub fn category_index_for_key(&self, key: KeyCode) -> Option<usize> {
        self.category_hotkeys
            .iter()
            .enumerate()
            .find(|(_, hotkey)| **hotkey == Some(key))
            .map(|(index, _)| index)
    }

    /// Get the hotkey for a specific tool.
    pub fn key_for_tool(&self, tool: AnnotationTool) -> KeyCode {
        match tool {
            AnnotationTool::Select => self.tool_select,
            AnnotationTool::BoundingBox => self.tool_bbox,
            AnnotationTool::Polygon => self.tool_polygon,
            AnnotationTool::Point => self.tool_point,
        }
    }

    /// Get the hotkey for a category at a specific index, if any.
    pub fn key_for_category_index(&self, index: usize) -> Option<KeyCode> {
        self.category_hotkeys.get(index).copied().flatten()
    }

    /// Set the hotkey for a tool.
    pub fn set_tool_key(&mut self, tool: AnnotationTool, key: KeyCode) {
        match tool {
            AnnotationTool::Select => self.tool_select = key,
            AnnotationTool::BoundingBox => self.tool_bbox = key,
            AnnotationTool::Polygon => self.tool_polygon = key,
            AnnotationTool::Point => self.tool_point = key,
        }
    }

    /// Set the hotkey for a category index.
    pub fn set_category_key(&mut self, index: usize, key: Option<KeyCode>) {
        if index < MAX_CATEGORY_HOTKEYS {
            self.category_hotkeys[index] = key;
        }
    }

    /// Check if a key is already used by any binding.
    /// Returns a description of what it's used for, if anything.
    pub fn key_conflict(
        &self,
        key: KeyCode,
        exclude_tool: Option<AnnotationTool>,
    ) -> Option<String> {
        // Check tool bindings
        if exclude_tool != Some(AnnotationTool::Select) && key == self.tool_select {
            return Some("Select tool".to_string());
        }
        if exclude_tool != Some(AnnotationTool::BoundingBox) && key == self.tool_bbox {
            return Some("Bounding Box tool".to_string());
        }
        if exclude_tool != Some(AnnotationTool::Polygon) && key == self.tool_polygon {
            return Some("Polygon tool".to_string());
        }
        if exclude_tool != Some(AnnotationTool::Point) && key == self.tool_point {
            return Some("Point tool".to_string());
        }

        // Check category bindings
        for (i, hotkey) in self.category_hotkeys.iter().enumerate() {
            if *hotkey == Some(key) {
                return Some(format!("Category {}", i + 1));
            }
        }

        None
    }
}

/// Convert a KeyCode to a display string.
pub fn key_to_string(key: KeyCode) -> &'static str {
    match key {
        KeyCode::A => "A",
        KeyCode::B => "B",
        KeyCode::C => "C",
        KeyCode::D => "D",
        KeyCode::E => "E",
        KeyCode::F => "F",
        KeyCode::G => "G",
        KeyCode::H => "H",
        KeyCode::I => "I",
        KeyCode::J => "J",
        KeyCode::K => "K",
        KeyCode::L => "L",
        KeyCode::M => "M",
        KeyCode::N => "N",
        KeyCode::O => "O",
        KeyCode::P => "P",
        KeyCode::Q => "Q",
        KeyCode::R => "R",
        KeyCode::S => "S",
        KeyCode::T => "T",
        KeyCode::U => "U",
        KeyCode::V => "V",
        KeyCode::W => "W",
        KeyCode::X => "X",
        KeyCode::Y => "Y",
        KeyCode::Z => "Z",
        KeyCode::Key0 => "0",
        KeyCode::Key1 => "1",
        KeyCode::Key2 => "2",
        KeyCode::Key3 => "3",
        KeyCode::Key4 => "4",
        KeyCode::Key5 => "5",
        KeyCode::Key6 => "6",
        KeyCode::Key7 => "7",
        KeyCode::Key8 => "8",
        KeyCode::Key9 => "9",
        KeyCode::F1 => "F1",
        KeyCode::F2 => "F2",
        KeyCode::F3 => "F3",
        KeyCode::F4 => "F4",
        KeyCode::F5 => "F5",
        KeyCode::F6 => "F6",
        KeyCode::F7 => "F7",
        KeyCode::F8 => "F8",
        KeyCode::F9 => "F9",
        KeyCode::F10 => "F10",
        KeyCode::F11 => "F11",
        KeyCode::F12 => "F12",
        KeyCode::Space => "Space",
        KeyCode::Tab => "Tab",
        KeyCode::Minus => "-",
        KeyCode::Equal => "=",
        KeyCode::Plus => "+",
        KeyCode::BracketLeft => "[",
        KeyCode::BracketRight => "]",
        KeyCode::Backslash => "\\",
        KeyCode::Semicolon => ";",
        KeyCode::Quote => "'",
        KeyCode::Comma => ",",
        KeyCode::Period => ".",
        KeyCode::Slash => "/",
        KeyCode::Grave => "`",
        _ => "?",
    }
}

/// Convert an optional KeyCode to a display string.
pub fn optional_key_to_string(key: Option<KeyCode>) -> &'static str {
    match key {
        Some(k) => key_to_string(k),
        None => "-",
    }
}
