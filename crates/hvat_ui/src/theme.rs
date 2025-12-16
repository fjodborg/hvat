//! Centralized theme system for hvat_ui
//!
//! Provides dark and light themes with consistent color palettes.
//! All widgets should use theme colors instead of hardcoded values.

use crate::renderer::Color;

/// A complete color theme for the UI
#[derive(Debug, Clone)]
pub struct Theme {
    // ==========================================================================
    // Backgrounds
    // ==========================================================================
    /// Main application background
    pub background: Color,

    /// Surface/panel background (slightly lighter than background)
    pub surface: Color,

    /// Input field background (text input, number input, slider input)
    pub input_bg: Color,

    /// Input field background when focused
    pub input_bg_focused: Color,

    // ==========================================================================
    // Borders
    // ==========================================================================
    /// Default border color
    pub border: Color,

    /// Border color when element is focused
    pub border_focused: Color,

    // ==========================================================================
    // Text
    // ==========================================================================
    /// Primary text color (high contrast)
    pub text_primary: Color,

    /// Secondary text color (lower contrast, labels, placeholders)
    pub text_secondary: Color,

    /// Placeholder text color
    pub text_placeholder: Color,

    // ==========================================================================
    // Interactive Elements
    // ==========================================================================
    /// Button background
    pub button_bg: Color,

    /// Button background on hover
    pub button_hover: Color,

    /// Button background when pressed/active
    pub button_active: Color,

    // ==========================================================================
    // Accent & Selection
    // ==========================================================================
    /// Accent color (focus rings, highlights, active elements)
    pub accent: Color,

    /// Text selection background
    pub selection: Color,

    /// Text cursor color
    pub cursor: Color,

    // ==========================================================================
    // Scrollbars
    // ==========================================================================
    /// Scrollbar track background
    pub scrollbar_track: Color,

    /// Scrollbar thumb
    pub scrollbar_thumb: Color,

    /// Scrollbar thumb on hover
    pub scrollbar_thumb_hover: Color,

    /// Scrollbar thumb while dragging
    pub scrollbar_thumb_drag: Color,

    // ==========================================================================
    // Specialized
    // ==========================================================================
    /// Slider track (unfilled portion)
    pub slider_track: Color,

    /// Slider track (filled portion)
    pub slider_track_fill: Color,

    /// Slider thumb
    pub slider_thumb: Color,

    /// Slider thumb on hover
    pub slider_thumb_hover: Color,

    /// Dropdown popup background
    pub popup_bg: Color,

    /// Dropdown option hover background
    pub option_hover: Color,

    /// Header background (collapsible)
    pub header_bg: Color,

    /// Header background on hover
    pub header_hover: Color,

    /// Content background (collapsible content area)
    pub content_bg: Color,
}

impl Theme {
    /// Create the default dark theme
    pub fn dark() -> Self {
        Self {
            // Backgrounds
            background: Color::rgb(0.12, 0.12, 0.14),
            surface: Color::rgb(0.15, 0.15, 0.18),
            input_bg: Color::rgb(0.15, 0.15, 0.17),
            input_bg_focused: Color::rgb(0.18, 0.18, 0.2),

            // Borders
            border: Color::rgb(0.3, 0.3, 0.35),
            border_focused: Color::rgb(0.4, 0.6, 1.0),

            // Text
            text_primary: Color::rgb(0.9, 0.9, 0.92),
            text_secondary: Color::rgb(0.6, 0.6, 0.65),
            text_placeholder: Color::rgb(0.5, 0.5, 0.55),

            // Interactive
            button_bg: Color::rgb(0.2, 0.2, 0.24),
            button_hover: Color::rgb(0.28, 0.28, 0.32),
            button_active: Color::rgb(0.35, 0.35, 0.4),

            // Accent & Selection
            accent: Color::rgb(0.4, 0.6, 1.0),
            selection: Color::rgba(0.4, 0.6, 1.0, 0.3),
            cursor: Color::rgb(0.4, 0.6, 1.0),

            // Scrollbars
            scrollbar_track: Color::rgba(0.15, 0.15, 0.18, 0.5),
            scrollbar_thumb: Color::rgba(0.4, 0.4, 0.45, 0.8),
            scrollbar_thumb_hover: Color::rgba(0.5, 0.5, 0.55, 0.9),
            scrollbar_thumb_drag: Color::rgba(0.6, 0.6, 0.65, 1.0),

            // Slider
            slider_track: Color::rgb(0.2, 0.2, 0.24),
            slider_track_fill: Color::rgb(0.4, 0.6, 1.0),
            slider_thumb: Color::rgb(0.9, 0.9, 0.92),
            slider_thumb_hover: Color::rgb(1.0, 1.0, 1.0),

            // Dropdown/Popup
            popup_bg: Color::rgba(0.15, 0.15, 0.18, 0.98),
            option_hover: Color::rgba(0.25, 0.25, 0.3, 1.0),

            // Collapsible
            header_bg: Color::rgba(0.15, 0.15, 0.18, 1.0),
            header_hover: Color::rgba(0.2, 0.2, 0.24, 1.0),
            content_bg: Color::rgba(0.12, 0.12, 0.14, 1.0),
        }
    }

    /// Create a light theme
    pub fn light() -> Self {
        Self {
            // Backgrounds
            background: Color::rgb(0.96, 0.96, 0.97),
            surface: Color::rgb(1.0, 1.0, 1.0),
            input_bg: Color::rgb(1.0, 1.0, 1.0),
            input_bg_focused: Color::rgb(0.98, 0.98, 1.0),

            // Borders
            border: Color::rgb(0.8, 0.8, 0.82),
            border_focused: Color::rgb(0.3, 0.5, 0.9),

            // Text
            text_primary: Color::rgb(0.1, 0.1, 0.12),
            text_secondary: Color::rgb(0.4, 0.4, 0.45),
            text_placeholder: Color::rgb(0.6, 0.6, 0.65),

            // Interactive
            button_bg: Color::rgb(0.92, 0.92, 0.94),
            button_hover: Color::rgb(0.86, 0.86, 0.9),
            button_active: Color::rgb(0.8, 0.8, 0.85),

            // Accent & Selection
            accent: Color::rgb(0.3, 0.5, 0.9),
            selection: Color::rgba(0.3, 0.5, 0.9, 0.25),
            cursor: Color::rgb(0.3, 0.5, 0.9),

            // Scrollbars
            scrollbar_track: Color::rgba(0.9, 0.9, 0.92, 0.5),
            scrollbar_thumb: Color::rgba(0.6, 0.6, 0.65, 0.6),
            scrollbar_thumb_hover: Color::rgba(0.5, 0.5, 0.55, 0.7),
            scrollbar_thumb_drag: Color::rgba(0.4, 0.4, 0.45, 0.8),

            // Slider
            slider_track: Color::rgb(0.85, 0.85, 0.88),
            slider_track_fill: Color::rgb(0.3, 0.5, 0.9),
            slider_thumb: Color::rgb(1.0, 1.0, 1.0),
            slider_thumb_hover: Color::rgb(0.95, 0.95, 0.98),

            // Dropdown/Popup
            popup_bg: Color::rgba(1.0, 1.0, 1.0, 0.98),
            option_hover: Color::rgba(0.9, 0.92, 0.96, 1.0),

            // Collapsible
            header_bg: Color::rgba(0.94, 0.94, 0.96, 1.0),
            header_hover: Color::rgba(0.9, 0.9, 0.93, 1.0),
            content_bg: Color::rgba(0.98, 0.98, 0.99, 1.0),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

use std::sync::OnceLock;

/// Global theme singleton for convenience
/// Widgets can use this to get default colors without requiring theme to be passed
static CURRENT_THEME: OnceLock<Theme> = OnceLock::new();

/// Set the global theme (can only be called once)
///
/// Returns `Err` with the provided theme if a theme has already been set.
pub fn set_theme(theme: Theme) -> Result<(), Theme> {
    CURRENT_THEME.set(theme)
}

/// Get the current global theme (or dark theme if not set)
pub fn current_theme() -> &'static Theme {
    CURRENT_THEME.get_or_init(Theme::dark)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme_colors_valid() {
        let theme = Theme::dark();

        // All colors should have valid RGB values (0.0 to 1.0)
        assert!(theme.background.r >= 0.0 && theme.background.r <= 1.0);
        assert!(theme.text_primary.r >= 0.0 && theme.text_primary.r <= 1.0);
        assert!(theme.accent.r >= 0.0 && theme.accent.r <= 1.0);

        // Alpha values should be valid
        assert!(theme.selection.a >= 0.0 && theme.selection.a <= 1.0);
        assert!(theme.scrollbar_track.a >= 0.0 && theme.scrollbar_track.a <= 1.0);
    }

    #[test]
    fn test_light_theme_colors_valid() {
        let theme = Theme::light();

        assert!(theme.background.r >= 0.0 && theme.background.r <= 1.0);
        assert!(theme.text_primary.r >= 0.0 && theme.text_primary.r <= 1.0);
        assert!(theme.accent.r >= 0.0 && theme.accent.r <= 1.0);
    }

    #[test]
    fn test_dark_theme_contrast() {
        let theme = Theme::dark();

        // Text should be lighter than background (dark theme)
        assert!(theme.text_primary.r > theme.background.r);
        assert!(theme.text_primary.g > theme.background.g);
        assert!(theme.text_primary.b > theme.background.b);
    }

    #[test]
    fn test_light_theme_contrast() {
        let theme = Theme::light();

        // Text should be darker than background (light theme)
        assert!(theme.text_primary.r < theme.background.r);
        assert!(theme.text_primary.g < theme.background.g);
        assert!(theme.text_primary.b < theme.background.b);
    }

    #[test]
    fn test_default_is_dark() {
        let default = Theme::default();
        let dark = Theme::dark();

        // Default should match dark theme
        assert!((default.background.r - dark.background.r).abs() < 0.001);
        assert!((default.text_primary.r - dark.text_primary.r).abs() < 0.001);
    }
}
