//! Widget configuration structs for customizable appearance and behavior.
//!
//! These configuration structs centralize hardcoded values and make widgets more customizable.

use crate::Color;

/// Scroll direction for scrollable containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollDirection {
    /// Only vertical scrolling (default)
    #[default]
    Vertical,
    /// Only horizontal scrolling
    Horizontal,
    /// Both vertical and horizontal scrolling
    Both,
}

impl ScrollDirection {
    /// Check if vertical scrolling is enabled.
    pub fn has_vertical(&self) -> bool {
        matches!(self, ScrollDirection::Vertical | ScrollDirection::Both)
    }

    /// Check if horizontal scrolling is enabled.
    pub fn has_horizontal(&self) -> bool {
        matches!(self, ScrollDirection::Horizontal | ScrollDirection::Both)
    }
}

/// Configuration for slider widget appearance.
#[derive(Debug, Clone)]
pub struct SliderConfig {
    /// Height of the slider track
    pub track_height: f32,
    /// Diameter of the thumb
    pub thumb_size: f32,
    /// Total widget height
    pub widget_height: f32,
    /// Default track color
    pub track_color: Color,
    /// Default fill/progress color
    pub fill_color: Color,
    /// Default thumb color
    pub thumb_color: Color,
}

impl Default for SliderConfig {
    fn default() -> Self {
        Self {
            track_height: 6.0,
            thumb_size: 16.0,
            widget_height: 24.0,
            track_color: Color::rgb(0.3, 0.3, 0.3),
            fill_color: Color::rgb(0.3, 0.6, 0.9),
            thumb_color: Color::WHITE,
        }
    }
}

impl SliderConfig {
    /// Create a new slider configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the track height.
    pub fn track_height(mut self, height: f32) -> Self {
        self.track_height = height;
        self
    }

    /// Set the thumb size.
    pub fn thumb_size(mut self, size: f32) -> Self {
        self.thumb_size = size;
        self
    }

    /// Set the total widget height.
    pub fn widget_height(mut self, height: f32) -> Self {
        self.widget_height = height;
        self
    }

    /// Set the track color.
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self
    }

    /// Set the fill color.
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = color;
        self
    }

    /// Set the thumb color.
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = color;
        self
    }
}

/// Configuration for scrollbar appearance.
#[derive(Debug, Clone)]
pub struct ScrollbarConfig {
    /// Width of the scrollbar track
    pub width: f32,
    /// Padding around scrollbar
    pub padding: f32,
    /// Minimum thumb height
    pub min_thumb_height: f32,
    /// Track background color
    pub track_color: Color,
    /// Thumb color when idle
    pub thumb_color: Color,
    /// Thumb color when dragging
    pub thumb_active_color: Color,
}

impl Default for ScrollbarConfig {
    fn default() -> Self {
        Self {
            width: 12.0,
            padding: 2.0,
            min_thumb_height: 30.0,
            track_color: Color::rgb(0.25, 0.25, 0.25),
            thumb_color: Color::rgb(0.45, 0.45, 0.45),
            thumb_active_color: Color::rgb(0.7, 0.7, 0.7),
        }
    }
}

impl ScrollbarConfig {
    /// Create a new scrollbar configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the total area reserved for the scrollbar (width + padding on both sides).
    pub fn total_area(&self) -> f32 {
        self.width + self.padding * 2.0
    }

    /// Set the scrollbar width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Set the padding around the scrollbar.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set the minimum thumb height.
    pub fn min_thumb_height(mut self, height: f32) -> Self {
        self.min_thumb_height = height;
        self
    }

    /// Set the track color.
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self
    }

    /// Set the thumb color when idle.
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = color;
        self
    }

    /// Set the thumb color when active/dragging.
    pub fn thumb_active_color(mut self, color: Color) -> Self {
        self.thumb_active_color = color;
        self
    }
}

/// Configuration for button appearance.
#[derive(Debug, Clone)]
pub struct ButtonConfig {
    /// Default button height
    pub default_height: f32,
    /// Horizontal padding
    pub padding_horizontal: f32,
    /// Vertical padding
    pub padding_vertical: f32,
    /// Normal background color
    pub background_color: Color,
    /// Background color on hover
    pub hover_color: Color,
    /// Text color
    pub text_color: Color,
    /// Text size
    pub text_size: f32,
}

impl Default for ButtonConfig {
    fn default() -> Self {
        Self {
            default_height: 30.0,
            padding_horizontal: 10.0,
            padding_vertical: 5.0,
            background_color: Color::rgb(0.3, 0.3, 0.3),
            hover_color: Color::rgb(0.4, 0.4, 0.4),
            text_color: Color::WHITE,
            text_size: 14.0,
        }
    }
}

impl ButtonConfig {
    /// Create a new button configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default button height.
    pub fn default_height(mut self, height: f32) -> Self {
        self.default_height = height;
        self
    }

    /// Set the horizontal padding.
    pub fn padding_horizontal(mut self, padding: f32) -> Self {
        self.padding_horizontal = padding;
        self
    }

    /// Set the vertical padding.
    pub fn padding_vertical(mut self, padding: f32) -> Self {
        self.padding_vertical = padding;
        self
    }

    /// Set the background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Set the hover color.
    pub fn hover_color(mut self, color: Color) -> Self {
        self.hover_color = color;
        self
    }

    /// Set the text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set the text size.
    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = size;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slider_config_default() {
        let config = SliderConfig::default();
        assert_eq!(config.track_height, 6.0);
        assert_eq!(config.thumb_size, 16.0);
        assert_eq!(config.widget_height, 24.0);
    }

    #[test]
    fn test_scrollbar_config_total_area() {
        let config = ScrollbarConfig::default();
        assert_eq!(config.total_area(), 16.0); // 12 + 2*2
    }

    #[test]
    fn test_button_config_builder() {
        let config = ButtonConfig::new()
            .default_height(40.0)
            .text_size(16.0);
        assert_eq!(config.default_height, 40.0);
        assert_eq!(config.text_size, 16.0);
    }
}
