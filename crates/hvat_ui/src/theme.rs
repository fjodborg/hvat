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

    // ==========================================================================
    // Shadows & Elevation
    // ==========================================================================
    /// Small shadow (buttons, subtle depth)
    pub shadow_sm: Color,

    /// Medium shadow (panels, cards)
    pub shadow_md: Color,

    /// Large shadow (popups, modals)
    pub shadow_lg: Color,

    // ==========================================================================
    // Additional UI Elements
    // ==========================================================================
    /// Subtle accent background (selected items)
    pub accent_subtle: Color,

    /// Divider lines between sections
    pub divider: Color,

    /// Elevated surface background (slightly lighter than surface)
    pub elevated_bg: Color,
}

impl Theme {
    /// Create the default dark theme
    /// Modern SaaS-style palette with richer colors and better contrast
    pub fn dark() -> Self {
        Self {
            // Backgrounds - deeper, richer darks
            background: Color::rgb(0.09, 0.09, 0.11),
            surface: Color::rgb(0.13, 0.13, 0.16),
            input_bg: Color::rgb(0.11, 0.11, 0.14),
            input_bg_focused: Color::rgb(0.14, 0.14, 0.18),

            // Borders - softer, less harsh
            border: Color::rgb(0.20, 0.20, 0.26),
            border_focused: Color::rgb(0.40, 0.58, 0.98),

            // Text - crisper whites
            text_primary: Color::rgb(0.95, 0.95, 0.97),
            text_secondary: Color::rgb(0.58, 0.58, 0.65),
            text_placeholder: Color::rgb(0.42, 0.42, 0.50),

            // Interactive - cleaner buttons with better hover contrast
            button_bg: Color::rgb(0.18, 0.18, 0.22),
            button_hover: Color::rgb(0.26, 0.26, 0.32),
            button_active: Color::rgb(0.32, 0.32, 0.40),

            // Accent & Selection - slightly softer blue
            accent: Color::rgb(0.40, 0.58, 0.98),
            selection: Color::rgba(0.40, 0.58, 0.98, 0.25),
            cursor: Color::rgb(0.40, 0.58, 0.98),

            // Scrollbars - more subtle
            scrollbar_track: Color::rgba(0.12, 0.12, 0.15, 0.3),
            scrollbar_thumb: Color::rgba(0.35, 0.35, 0.42, 0.6),
            scrollbar_thumb_hover: Color::rgba(0.45, 0.45, 0.52, 0.75),
            scrollbar_thumb_drag: Color::rgba(0.55, 0.55, 0.62, 0.9),

            // Slider - darker track, vibrant fill
            slider_track: Color::rgb(0.14, 0.14, 0.18),
            slider_track_fill: Color::rgb(0.40, 0.58, 0.98),
            slider_thumb: Color::rgb(1.0, 1.0, 1.0),
            slider_thumb_hover: Color::rgb(0.40, 0.58, 0.98),

            // Dropdown/Popup - darker for depth
            popup_bg: Color::rgba(0.11, 0.11, 0.14, 0.98),
            option_hover: Color::rgba(0.22, 0.22, 0.28, 1.0),

            // Collapsible - darker headers for hierarchy
            header_bg: Color::rgba(0.11, 0.11, 0.14, 1.0),
            header_hover: Color::rgba(0.16, 0.16, 0.21, 1.0),
            content_bg: Color::rgba(0.09, 0.09, 0.11, 1.0),

            // Shadows - for elevation and depth
            shadow_sm: Color::rgba(0.0, 0.0, 0.0, 0.2),
            shadow_md: Color::rgba(0.0, 0.0, 0.0, 0.35),
            shadow_lg: Color::rgba(0.0, 0.0, 0.0, 0.5),

            // Additional UI elements
            accent_subtle: Color::rgba(0.40, 0.58, 0.98, 0.12),
            divider: Color::rgba(1.0, 1.0, 1.0, 0.06),
            elevated_bg: Color::rgb(0.15, 0.15, 0.19),
        }
    }

    /// Create a light theme
    /// Modern SaaS-style palette matching the dark theme aesthetic
    pub fn light() -> Self {
        Self {
            // Backgrounds - clean whites with subtle warmth
            background: Color::rgb(0.97, 0.97, 0.98),
            surface: Color::rgb(1.0, 1.0, 1.0),
            input_bg: Color::rgb(0.99, 0.99, 1.0),
            input_bg_focused: Color::rgb(1.0, 1.0, 1.0),

            // Borders - soft grays
            border: Color::rgb(0.82, 0.82, 0.86),
            border_focused: Color::rgb(0.35, 0.52, 0.92),

            // Text - rich blacks
            text_primary: Color::rgb(0.12, 0.12, 0.15),
            text_secondary: Color::rgb(0.45, 0.45, 0.52),
            text_placeholder: Color::rgb(0.62, 0.62, 0.68),

            // Interactive - subtle button backgrounds
            button_bg: Color::rgb(0.94, 0.94, 0.96),
            button_hover: Color::rgb(0.88, 0.88, 0.92),
            button_active: Color::rgb(0.82, 0.82, 0.88),

            // Accent & Selection - vibrant blue
            accent: Color::rgb(0.35, 0.52, 0.92),
            selection: Color::rgba(0.35, 0.52, 0.92, 0.2),
            cursor: Color::rgb(0.35, 0.52, 0.92),

            // Scrollbars - very subtle
            scrollbar_track: Color::rgba(0.88, 0.88, 0.90, 0.3),
            scrollbar_thumb: Color::rgba(0.55, 0.55, 0.60, 0.5),
            scrollbar_thumb_hover: Color::rgba(0.45, 0.45, 0.52, 0.65),
            scrollbar_thumb_drag: Color::rgba(0.38, 0.38, 0.45, 0.8),

            // Slider - subtle track
            slider_track: Color::rgb(0.88, 0.88, 0.90),
            slider_track_fill: Color::rgb(0.35, 0.52, 0.92),
            slider_thumb: Color::rgb(1.0, 1.0, 1.0),
            slider_thumb_hover: Color::rgb(0.35, 0.52, 0.92),

            // Dropdown/Popup - clean white with shadow effect implied by popup
            popup_bg: Color::rgba(1.0, 1.0, 1.0, 0.98),
            option_hover: Color::rgba(0.92, 0.94, 0.98, 1.0),

            // Collapsible - subtle hierarchy
            header_bg: Color::rgba(0.95, 0.95, 0.97, 1.0),
            header_hover: Color::rgba(0.91, 0.91, 0.94, 1.0),
            content_bg: Color::rgba(0.98, 0.98, 0.99, 1.0),

            // Shadows - lighter for light theme
            shadow_sm: Color::rgba(0.0, 0.0, 0.0, 0.08),
            shadow_md: Color::rgba(0.0, 0.0, 0.0, 0.15),
            shadow_lg: Color::rgba(0.0, 0.0, 0.0, 0.25),

            // Additional UI elements
            accent_subtle: Color::rgba(0.35, 0.52, 0.92, 0.1),
            divider: Color::rgba(0.0, 0.0, 0.0, 0.08),
            elevated_bg: Color::rgb(1.0, 1.0, 1.0),
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
