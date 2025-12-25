//! Shared configuration types for widgets
//!
//! This module provides base configuration structures that can be reused
//! across multiple widgets to reduce code duplication.

use crate::renderer::Color;
use crate::theme::current_theme;

/// Base configuration for text input fields (shared by TextInput, NumberInput, Slider)
///
/// Contains the common color fields used for background, borders, text, cursor,
/// and selection highlighting in all text-editable widgets.
#[derive(Debug, Clone)]
pub struct BaseInputConfig {
    /// Background color
    pub background_color: Color,
    /// Background color when focused
    pub focused_background_color: Color,
    /// Border color
    pub border_color: Color,
    /// Border color when focused
    pub focused_border_color: Color,
    /// Text color
    pub text_color: Color,
    /// Cursor color
    pub cursor_color: Color,
    /// Selection background color
    pub selection_color: Color,
}

impl Default for BaseInputConfig {
    fn default() -> Self {
        let theme = current_theme();
        Self {
            background_color: theme.input_bg,
            focused_background_color: theme.input_bg_focused,
            border_color: theme.border,
            focused_border_color: theme.border_focused,
            text_color: theme.text_primary,
            cursor_color: theme.cursor,
            selection_color: theme.selection,
        }
    }
}

impl BaseInputConfig {
    /// Get the appropriate background color based on focus state
    #[inline]
    pub fn background(&self, focused: bool) -> Color {
        if focused {
            self.focused_background_color
        } else {
            self.background_color
        }
    }

    /// Get the appropriate border color based on focus state
    #[inline]
    pub fn border(&self, focused: bool) -> Color {
        if focused {
            self.focused_border_color
        } else {
            self.border_color
        }
    }
}
