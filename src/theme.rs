//! Theme system for HVAT application.
//!
//! Provides dark and light theme support with customizable colors.

use hvat_ui::Color;

/// Theme choice - dark or light mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeChoice {
    Dark,
    Light,
}

/// Application theme with color definitions.
#[derive(Debug, Clone)]
pub struct Theme {
    pub choice: ThemeChoice,
}

impl Theme {
    /// Create a dark theme.
    pub fn dark() -> Self {
        Self {
            choice: ThemeChoice::Dark,
        }
    }

    /// Create a light theme.
    pub fn light() -> Self {
        Self {
            choice: ThemeChoice::Light,
        }
    }

    /// Get the background color for this theme.
    pub fn background_color(&self) -> Color {
        match self.choice {
            ThemeChoice::Dark => Color::rgb(0.15, 0.15, 0.15),
            ThemeChoice::Light => Color::rgb(0.95, 0.95, 0.95),
        }
    }

    /// Get the text color for this theme.
    pub fn text_color(&self) -> Color {
        match self.choice {
            ThemeChoice::Dark => Color::rgb(0.9, 0.9, 0.9),
            ThemeChoice::Light => Color::rgb(0.1, 0.1, 0.1),
        }
    }

    /// Get the accent color (same for both themes).
    pub fn accent_color(&self) -> Color {
        Color::rgb(0.3, 0.6, 0.9)
    }

    /// Get the button color for this theme.
    pub fn button_color(&self) -> Color {
        match self.choice {
            ThemeChoice::Dark => Color::rgb(0.25, 0.25, 0.25),
            ThemeChoice::Light => Color::rgb(0.85, 0.85, 0.85),
        }
    }

    /// Get the secondary text color (for less prominent text).
    pub fn secondary_text_color(&self) -> Color {
        match self.choice {
            ThemeChoice::Dark => Color::rgb(0.6, 0.6, 0.6),
            ThemeChoice::Light => Color::rgb(0.4, 0.4, 0.4),
        }
    }

    /// Get the border color for this theme.
    pub fn border_color(&self) -> Color {
        match self.choice {
            ThemeChoice::Dark => Color::rgb(0.4, 0.4, 0.4),
            ThemeChoice::Light => Color::rgb(0.7, 0.7, 0.7),
        }
    }
}
