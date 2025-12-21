//! Centralized constants for hvat_ui
//!
//! All magic numbers and repeated constants are defined here for consistency
//! and easy maintenance.

use crate::layout::Padding;

// =============================================================================
// Typography
// =============================================================================

/// Default font size used across most widgets
pub const DEFAULT_FONT_SIZE: f32 = 14.0;

/// Smaller font size for secondary text (e.g., slider input)
pub const SMALL_FONT_SIZE: f32 = 12.0;

/// Approximate character width as a ratio of font size
/// Used for text measurement approximation
pub const CHAR_WIDTH_FACTOR: f32 = 0.6;

/// Line height as a ratio of font size
pub const LINE_HEIGHT_FACTOR: f32 = 1.2;

// =============================================================================
// Layout & Spacing
// =============================================================================

/// Default spacing between children in Column/Row
pub const DEFAULT_SPACING: f32 = 8.0;

/// Compact vertical padding (text input, number input)
pub const PADDING_COMPACT: f32 = 6.0;

/// Standard horizontal padding
pub const PADDING_STANDARD: f32 = 8.0;

/// Comfortable padding (buttons)
pub const PADDING_COMFORTABLE: f32 = 16.0;

/// Default padding for text input fields
pub const TEXT_INPUT_PADDING: Padding = Padding {
    top: PADDING_COMPACT,
    right: PADDING_STANDARD,
    bottom: PADDING_COMPACT,
    left: PADDING_STANDARD,
};

/// Default padding for buttons
pub const BUTTON_PADDING: Padding = Padding {
    top: PADDING_STANDARD,
    right: PADDING_COMFORTABLE,
    bottom: PADDING_STANDARD,
    left: PADDING_COMFORTABLE,
};

/// Compact padding for small/icon buttons
pub const BUTTON_PADDING_COMPACT: Padding = Padding {
    top: PADDING_COMPACT,
    right: PADDING_STANDARD,
    bottom: PADDING_COMPACT,
    left: PADDING_STANDARD,
};

/// Standard row item height for consistent sizing
/// Derived from: line_height(DEFAULT_FONT_SIZE) + PADDING_COMPACT * 2.0
pub const ROW_ITEM_HEIGHT: f32 = DEFAULT_FONT_SIZE * LINE_HEIGHT_FACTOR + PADDING_COMPACT * 2.0;

/// Default content padding inside collapsible sections
/// This ensures borders of child elements are visible
pub const COLLAPSIBLE_CONTENT_PADDING: f32 = 2.0;

/// Offset to align color picker with swatch (compensates for content padding)
pub const COLOR_PICKER_SWATCH_OFFSET: f32 = -COLLAPSIBLE_CONTENT_PADDING;

// =============================================================================
// Text Input
// =============================================================================

/// Width of the text cursor
pub const CURSOR_WIDTH: f32 = 1.0;

/// Maximum entries in undo/redo stacks
pub const UNDO_STACK_LIMIT: usize = 50;

// =============================================================================
// Scrollbar
// =============================================================================

/// Default scrollbar width
pub const SCROLLBAR_WIDTH: f32 = 8.0;

/// Compact scrollbar width (dropdown, collapsible)
pub const SCROLLBAR_WIDTH_COMPACT: f32 = 6.0;

/// Minimum scrollbar thumb size
pub const SCROLLBAR_MIN_THUMB: f32 = 20.0;

/// Padding around scrollbar thumb
pub const SCROLLBAR_PADDING: f32 = 2.0;

// =============================================================================
// Slider
// =============================================================================

/// Default slider height
pub const SLIDER_HEIGHT: f32 = 24.0;

/// Slider track height
pub const SLIDER_TRACK_HEIGHT: f32 = 4.0;

/// Slider thumb radius
pub const SLIDER_THUMB_RADIUS: f32 = 8.0;

/// Slider input field width
pub const SLIDER_INPUT_WIDTH: f32 = 60.0;

/// Slider input field spacing from slider
pub const SLIDER_INPUT_SPACING: f32 = 8.0;

/// Slider input field internal padding
pub const SLIDER_INPUT_PADDING: f32 = 4.0;

// =============================================================================
// Number Input
// =============================================================================

/// Width of increment/decrement buttons
pub const NUMBER_INPUT_BUTTON_WIDTH: f32 = 20.0;

/// Default width for number input
pub const NUMBER_INPUT_DEFAULT_WIDTH: f32 = 120.0;

// =============================================================================
// Dropdown
// =============================================================================

/// Horizontal padding for dropdown text
pub const DROPDOWN_TEXT_PADDING_X: f32 = 8.0;

/// Width of the dropdown arrow indicator
pub const DROPDOWN_ARROW_WIDTH: f32 = 20.0;

/// Default option height in dropdown
pub const DROPDOWN_OPTION_HEIGHT: f32 = 28.0;

/// Default max visible options before scrolling
pub const DROPDOWN_MAX_VISIBLE_OPTIONS: usize = 8;

/// Default dropdown width
pub const DROPDOWN_DEFAULT_WIDTH: f32 = 200.0;

// =============================================================================
// Collapsible
// =============================================================================

/// Header height for collapsible sections
pub const COLLAPSIBLE_HEADER_HEIGHT: f32 = 32.0;

/// Horizontal padding in collapsible header
pub const COLLAPSIBLE_HEADER_PADDING_X: f32 = 8.0;

/// Chevron icon size
pub const COLLAPSIBLE_ICON_SIZE: f32 = 12.0;

/// Margin after chevron icon
pub const COLLAPSIBLE_ICON_MARGIN: f32 = 8.0;

// =============================================================================
// Image Viewer
// =============================================================================

/// Zoom factor per scroll step
pub const ZOOM_FACTOR: f32 = 1.25;

/// Minimum zoom level (10% = 1 image pixel shown as 0.1 screen pixels)
pub const ZOOM_MIN: f32 = 0.1;

/// Maximum zoom level (5000% = 1 image pixel shown as 50 screen pixels)
pub const ZOOM_MAX: f32 = 50.0;

/// Pan speed in clip space units per key press
pub const PAN_SPEED: f32 = 0.1;

/// Control button size
pub const IMAGE_VIEWER_CONTROL_SIZE: f32 = 28.0;

// =============================================================================
// Scrolling
// =============================================================================

/// Default scroll speed multiplier for scroll wheel
pub const SCROLL_SPEED: f32 = 1.0;

// =============================================================================
// Tolerances & Math
// =============================================================================

/// Epsilon for float comparison (close to integer check)
pub const FLOAT_EPSILON: f32 = 0.0001;

/// Maximum decimal places for number display
pub const MAX_DECIMAL_PLACES: usize = 3;

/// Default step divider for calculating step from range
pub const DEFAULT_STEP_DIVIDER: f32 = 100.0;

/// Big step multiplier (PageUp/PageDown)
pub const BIG_STEP_MULTIPLIER: f32 = 10.0;

/// Fine step multiplier (Shift+Arrow)
pub const FINE_STEP_MULTIPLIER: f32 = 0.1;

/// Thumb hit area multiplier for easier clicking
pub const THUMB_HIT_AREA_MULTIPLIER: f32 = 1.5;

/// Tolerance for detecting fill behavior in flex layout
/// When a child's size is within this tolerance of the available size,
/// it's treated as a "fill" child that should expand to fill remaining space.
pub const FILL_DETECTION_TOLERANCE: f32 = 1.0;

/// Default width for input widgets (slider, text input, etc.)
/// Used as fallback when Length::Shrink is specified
pub const DEFAULT_INPUT_WIDTH: f32 = 200.0;

// =============================================================================
// Renderer Buffer Capacities
// =============================================================================

/// Initial capacity for color vertex buffer
/// Sized for ~256 quads (each quad = 4 vertices)
pub const RENDERER_COLOR_VERTEX_CAPACITY: usize = 1024;

/// Initial capacity for color index buffer
/// Sized for ~256 quads (each quad = 6 indices: 2 triangles)
pub const RENDERER_COLOR_INDEX_CAPACITY: usize = 2048;

/// Initial capacity for overlay vertex buffer
/// Overlays typically have fewer primitives than main content
pub const RENDERER_OVERLAY_VERTEX_CAPACITY: usize = 256;

/// Initial capacity for overlay index buffer
pub const RENDERER_OVERLAY_INDEX_CAPACITY: usize = 512;

/// Initial capacity for text render requests per frame
pub const RENDERER_TEXT_REQUEST_CAPACITY: usize = 64;

/// Initial capacity for overlay text requests per frame
pub const RENDERER_OVERLAY_TEXT_REQUEST_CAPACITY: usize = 16;

/// Initial capacity for texture render requests per frame
pub const RENDERER_TEXTURE_REQUEST_CAPACITY: usize = 8;

/// Initial capacity for clip stack depth
pub const RENDERER_CLIP_STACK_CAPACITY: usize = 8;

/// Initial capacity for text buffer cache entries
pub const RENDERER_TEXT_CACHE_CAPACITY: usize = 128;

// =============================================================================
// Helper Functions
// =============================================================================

/// Calculate approximate character width for a given font size
#[inline]
pub fn char_width(font_size: f32) -> f32 {
    font_size * CHAR_WIDTH_FACTOR
}

/// Calculate approximate line height for a given font size
#[inline]
pub fn line_height(font_size: f32) -> f32 {
    font_size * LINE_HEIGHT_FACTOR
}

/// Format a number, avoiding excessive decimal places
/// If close to integer, displays as integer
/// Otherwise displays with up to MAX_DECIMAL_PLACES, trimming trailing zeros
pub fn format_number(value: f32) -> String {
    if (value - value.round()).abs() < FLOAT_EPSILON {
        format!("{}", value.round() as i32)
    } else {
        let formatted = format!("{:.3}", value);
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_width() {
        assert!((char_width(14.0) - 8.4).abs() < 0.001);
        assert!((char_width(12.0) - 7.2).abs() < 0.001);
    }

    #[test]
    fn test_line_height() {
        assert!((line_height(14.0) - 16.8).abs() < 0.001);
    }

    #[test]
    fn test_format_number_integer() {
        assert_eq!(format_number(5.0), "5");
        assert_eq!(format_number(5.00001), "5");
        assert_eq!(format_number(-10.0), "-10");
    }

    #[test]
    fn test_format_number_decimal() {
        assert_eq!(format_number(3.14159), "3.142");
        assert_eq!(format_number(1.5), "1.5");
        assert_eq!(format_number(2.25), "2.25");
    }

    #[test]
    fn test_constants_are_positive() {
        assert!(DEFAULT_FONT_SIZE > 0.0);
        assert!(CHAR_WIDTH_FACTOR > 0.0);
        assert!(DEFAULT_SPACING > 0.0);
        assert!(CURSOR_WIDTH > 0.0);
        assert!(UNDO_STACK_LIMIT > 0);
        assert!(SCROLLBAR_WIDTH > 0.0);
        assert!(SCROLLBAR_MIN_THUMB > 0.0);
        assert!(ZOOM_MIN > 0.0);
        assert!(ZOOM_MAX > ZOOM_MIN);
    }
}
