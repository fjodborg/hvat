//! Theme constants for consistent UI styling.
//!
//! This module provides centralized color constants and UI strings
//! to ensure consistency across widgets and enable easy theming.

use crate::Color;

/// Color constants for UI elements.
pub mod colors {
    use super::Color;

    // Button colors
    pub const BUTTON_NORMAL: Color = Color { r: 0.2, g: 0.3, b: 0.5, a: 1.0 };
    pub const BUTTON_HOVER: Color = Color { r: 0.3, g: 0.4, b: 0.6, a: 1.0 };

    // Icon button colors
    pub const ICON_BUTTON_NORMAL: Color = Color { r: 0.15, g: 0.15, b: 0.2, a: 1.0 };
    pub const ICON_BUTTON_HOVER: Color = Color { r: 0.25, g: 0.25, b: 0.3, a: 1.0 };
    pub const ICON_BUTTON_ACTIVE: Color = Color { r: 0.3, g: 0.5, b: 0.7, a: 1.0 };

    // Background colors
    pub const DARK_BG: Color = Color { r: 0.15, g: 0.18, b: 0.22, a: 1.0 };
    pub const MEDIUM_BG: Color = Color { r: 0.2, g: 0.25, b: 0.3, a: 1.0 };
    pub const LIGHT_BG: Color = Color { r: 0.25, g: 0.3, b: 0.35, a: 1.0 };

    // Container/panel colors
    pub const CONTAINER_BG: Color = Color { r: 0.2, g: 0.25, b: 0.3, a: 1.0 };
    pub const COLLAPSIBLE_HEADER: Color = Color { r: 0.2, g: 0.25, b: 0.3, a: 1.0 };
    pub const COLLAPSIBLE_HEADER_HOVER: Color = Color { r: 0.25, g: 0.3, b: 0.35, a: 1.0 };

    // Dropdown colors
    pub const DROPDOWN_BG: Color = Color { r: 0.2, g: 0.3, b: 0.4, a: 1.0 };
    pub const DROPDOWN_HOVER: Color = Color { r: 0.3, g: 0.4, b: 0.5, a: 1.0 };
    pub const DROPDOWN_MENU: Color = Color { r: 0.15, g: 0.2, b: 0.3, a: 1.0 };
    pub const DROPDOWN_OPTION_HOVER: Color = Color { r: 0.25, g: 0.35, b: 0.5, a: 1.0 };

    // Modal colors
    pub const MODAL_OVERLAY: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.6 };
    pub const MODAL_BG: Color = Color { r: 0.2, g: 0.3, b: 0.4, a: 1.0 };

    // Border/decoration colors
    pub const BORDER: Color = Color { r: 0.3, g: 0.35, b: 0.4, a: 1.0 };
    pub const BORDER_LIGHT: Color = Color { r: 0.4, g: 0.45, b: 0.5, a: 1.0 };

    // Slider colors
    pub const SLIDER_TRACK: Color = Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 };
    pub const SLIDER_FILL: Color = Color { r: 0.4, g: 0.6, b: 0.8, a: 1.0 };
    pub const SLIDER_THUMB: Color = Color { r: 0.8, g: 0.8, b: 0.8, a: 1.0 };

    // Text input colors
    pub const TEXT_INPUT_BG: Color = Color { r: 0.15, g: 0.15, b: 0.2, a: 1.0 };
    pub const TEXT_INPUT_BORDER: Color = Color { r: 0.3, g: 0.3, b: 0.4, a: 1.0 };
    pub const TEXT_INPUT_FOCUS_BORDER: Color = Color { r: 0.4, g: 0.6, b: 0.8, a: 1.0 };
}

/// UI string constants.
pub mod ui {
    /// Arrow indicators for collapsible sections.
    pub const ARROW_COLLAPSED: &str = "\u{25B6}"; // ▶
    pub const ARROW_EXPANDED: &str = "\u{25BC}";  // ▼
    pub const ARROW_UP: &str = "\u{25B2}";        // ▲
    pub const ARROW_DOWN: &str = "\u{25BC}";      // ▼

    /// Default sizing constants.
    pub const DEFAULT_PADDING: f32 = 6.0;
    pub const DEFAULT_SPACING: f32 = 4.0;
    pub const DEFAULT_BORDER_WIDTH: f32 = 1.0;
}
