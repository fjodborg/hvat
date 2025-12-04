//! UI constants for consistent styling across the application.
//!
//! This module centralizes all hardcoded values for sizes, spacing,
//! thresholds, and other UI-related constants.

use hvat_ui::Color;

/// Text size constants for consistent typography hierarchy.
pub mod text {
    /// Large title (e.g., "HVAT" header)
    pub const TITLE_LARGE: f32 = 28.0;
    /// Page/section title (e.g., "Image Viewer", "Settings")
    pub const TITLE: f32 = 24.0;
    /// Application title in header
    pub const HEADER_TITLE: f32 = 20.0;
    /// Section headers and accented labels
    pub const SECTION_HEADER: f32 = 16.0;
    /// Body text and labels
    pub const BODY: f32 = 14.0;
    /// Normal text (alias for body)
    pub const NORMAL: f32 = 14.0;
    /// Small text, status messages, help text
    pub const SMALL: f32 = 12.0;
    /// Extra large for emphasis (e.g., counter display)
    pub const DISPLAY: f32 = 48.0;
}

/// Spacing constants for consistent layout.
pub mod spacing {
    /// Tight spacing for compact elements
    pub const TIGHT: f32 = 5.0;
    /// Standard spacing between related elements
    pub const STANDARD: f32 = 10.0;
    /// Normal spacing (alias for standard)
    pub const NORMAL: f32 = 10.0;
    /// Medium spacing for visual grouping
    pub const MEDIUM: f32 = 15.0;
    /// Large spacing between major sections
    pub const LARGE: f32 = 20.0;
    /// Extra large spacing for page-level separation
    pub const XLARGE: f32 = 30.0;
}

/// Padding constants for containers.
pub mod padding {
    /// Minimal padding (e.g., image border container)
    pub const MINIMAL: f32 = 4.0;
    /// Small padding (e.g., header)
    pub const SMALL: f32 = 5.0;
    /// Standard padding for content areas
    pub const STANDARD: f32 = 20.0;
    /// Large padding for main container
    pub const LARGE: f32 = 30.0;
}

/// Button dimension constants.
pub mod button {
    /// Navigation buttons in header
    pub const NAV_WIDTH: f32 = 100.0;
    /// Standard action buttons
    pub const STANDARD_WIDTH: f32 = 120.0;
    /// Small tool buttons
    pub const TOOL_WIDTH: f32 = 80.0;
    /// Compact action buttons
    pub const COMPACT_WIDTH: f32 = 70.0;
    /// Extra compact buttons
    pub const XCOMPACT_WIDTH: f32 = 60.0;
    /// Wide buttons (e.g., "Reset Image Settings")
    pub const WIDE_WIDTH: f32 = 180.0;
    /// Zoom control buttons
    pub const ZOOM_WIDTH: f32 = 90.0;
    /// Band reset button
    pub const BAND_RESET_WIDTH: f32 = 110.0;
    /// Default button height
    pub const DEFAULT_HEIGHT: f32 = 40.0;
}

/// Slider dimension constants.
pub mod slider {
    use hvat_ui::Length;

    /// Standard slider width for image adjustments
    pub const STANDARD_WIDTH: f32 = 200.0;
    /// Compact slider width for band selection
    pub const COMPACT_WIDTH: f32 = 150.0;

    /// Standard slider width as Length
    pub fn standard_length() -> Length {
        Length::Units(STANDARD_WIDTH)
    }

    /// Compact slider width as Length
    pub fn compact_length() -> Length {
        Length::Units(COMPACT_WIDTH)
    }
}

/// Image viewer dimension constants.
pub mod image_viewer {
    /// Default image display width
    pub const WIDTH: f32 = 600.0;
    /// Default image display height
    pub const HEIGHT: f32 = 400.0;
    /// Border width around image
    pub const BORDER_WIDTH: f32 = 2.0;
}

/// Sidebar dimension constants.
pub mod sidebar {
    /// Default sidebar width
    pub const WIDTH: f32 = 220.0;
    /// Spacing between sidebar and main content
    pub const GAP: f32 = 15.0;
}

/// Title bar constants for titled containers.
pub mod title_bar {
    use super::Color;

    /// Default title font size
    pub const FONT_SIZE: f32 = 11.0;
    /// Default horizontal padding
    pub const PADDING_H: f32 = 8.0;
    /// Default vertical padding
    pub const PADDING_V: f32 = 3.0;

    /// Default title bar background color
    pub const BG_COLOR: Color = Color {
        r: 0.15,
        g: 0.18,
        b: 0.22,
        a: 0.95,
    };

    /// Default title text color
    pub const TEXT_COLOR: Color = Color {
        r: 0.8,
        g: 0.85,
        b: 0.9,
        a: 1.0,
    };
}

/// Interaction threshold constants.
pub mod threshold {
    /// Minimum pan delta to trigger update (pixels)
    pub const PAN_CHANGE: f32 = 0.1;
    /// Minimum drag movement to apply (pixels)
    pub const DRAG_MOVEMENT: f32 = 0.5;
    /// Minimum zoom change to trigger update
    pub const ZOOM_CHANGE: f32 = 0.001;
    /// Epsilon for float comparison in image settings
    pub const FLOAT_EPSILON: f32 = 0.001;
    /// Scroll position change threshold
    pub const SCROLL_CHANGE: f32 = 0.1;
    /// Polygon close distance threshold (image coords)
    pub const POLYGON_CLOSE: f32 = 15.0;
    /// Point hit test radius for selection
    pub const POINT_HIT_RADIUS: f32 = 5.0;
}

/// Zoom constants.
pub mod zoom {
    /// Zoom increment/decrement factor
    pub const FACTOR: f32 = 1.2;
    /// Maximum zoom level
    pub const MAX: f32 = 5.0;
    /// Minimum zoom level
    pub const MIN: f32 = 0.2;
    /// Pan step size for keyboard/button navigation
    pub const PAN_STEP: f32 = 10.0;
}

/// Annotation constants.
pub mod annotation {
    /// Default point marker radius
    pub const POINT_RADIUS: f32 = 6.0;
    /// Golden angle for category color generation (degrees)
    pub const GOLDEN_ANGLE: f32 = 137.5;
    /// Default category color alpha
    pub const DEFAULT_ALPHA: f32 = 0.7;
    /// Preview color alpha (semi-transparent)
    pub const PREVIEW_ALPHA: f32 = 0.5;
}

/// Common UI colors (beyond theme colors).
pub mod colors {
    use super::Color;

    /// Accent color for highlights and headers
    pub const ACCENT: Color = Color {
        r: 0.3,
        g: 0.6,
        b: 0.9,
        a: 1.0,
    };

    /// Default gray for annotations without category
    pub const DEFAULT_GRAY: Color = Color {
        r: 0.7,
        g: 0.7,
        b: 0.7,
        a: 1.0,
    };

    /// Muted text color for help/instructions
    pub const MUTED_TEXT: Color = Color {
        r: 0.6,
        g: 0.6,
        b: 0.6,
        a: 1.0,
    };

    /// Border color for containers
    pub const BORDER: Color = Color {
        r: 0.4,
        g: 0.4,
        b: 0.4,
        a: 1.0,
    };

    /// FPS counter color (green tint)
    pub const FPS_TEXT: Color = Color {
        r: 0.5,
        g: 0.8,
        b: 0.5,
        a: 1.0,
    };

    /// Red channel label color
    pub const CHANNEL_RED: Color = Color {
        r: 1.0,
        g: 0.3,
        b: 0.3,
        a: 1.0,
    };

    /// Green channel label color
    pub const CHANNEL_GREEN: Color = Color {
        r: 0.3,
        g: 1.0,
        b: 0.3,
        a: 1.0,
    };

    /// Blue channel label color
    pub const CHANNEL_BLUE: Color = Color {
        r: 0.3,
        g: 0.3,
        b: 1.0,
        a: 1.0,
    };
}
