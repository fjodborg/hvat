//! Shared configuration types for widgets
//!
//! This module provides base configuration structures that can be reused
//! across multiple widgets to reduce code duplication.

use crate::renderer::Color;

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
        Self {
            background_color: Color::rgb(0.15, 0.15, 0.17),
            focused_background_color: Color::rgb(0.18, 0.18, 0.2),
            border_color: Color::BORDER,
            focused_border_color: Color::ACCENT,
            text_color: Color::TEXT_PRIMARY,
            cursor_color: Color::ACCENT,
            selection_color: Color::rgba(0.4, 0.6, 1.0, 0.3),
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
