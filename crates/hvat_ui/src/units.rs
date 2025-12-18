//! Type-safe wrappers for numeric units
//!
//! This module provides newtype wrappers that add compile-time distinction between
//! different types of f32 values, preventing common bugs like mixing font sizes with
//! zoom levels or spacing values.

use std::fmt;

// =============================================================================
// FontSize
// =============================================================================

/// Font size in pixels
///
/// This newtype prevents accidentally using other f32 values (like spacing or zoom)
/// where a font size is expected.
///
/// # Example
///
/// ```ignore
/// use hvat_ui::FontSize;
///
/// let size = FontSize::new(14.0);
/// let larger = size.scaled(1.5); // 21px
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct FontSize(pub f32);

impl FontSize {
    /// Minimum recommended font size (readable text)
    pub const MIN: Self = Self(8.0);

    /// Default font size
    pub const DEFAULT: Self = Self(14.0);

    /// Large font size for headers
    pub const LARGE: Self = Self(18.0);

    /// Maximum practical font size
    pub const MAX: Self = Self(72.0);

    /// Create a new font size
    pub const fn new(size: f32) -> Self {
        Self(size)
    }

    /// Get the raw f32 value
    pub const fn value(self) -> f32 {
        self.0
    }

    /// Clamp to reasonable bounds
    pub fn clamp(self) -> Self {
        Self(self.0.clamp(Self::MIN.0, Self::MAX.0))
    }

    /// Scale by a factor
    pub fn scaled(self, factor: f32) -> Self {
        Self(self.0 * factor)
    }
}

impl Default for FontSize {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<f32> for FontSize {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl From<FontSize> for f32 {
    fn from(size: FontSize) -> f32 {
        size.0
    }
}

impl fmt::Display for FontSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}px", self.0)
    }
}

// =============================================================================
// Spacing
// =============================================================================

/// Spacing/padding value in pixels
///
/// Represents margins, padding, gaps, and other spacing values.
///
/// # Example
///
/// ```ignore
/// use hvat_ui::Spacing;
///
/// let gap = Spacing::SMALL;
/// let padding = Spacing::MEDIUM;
/// let total = gap + padding;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Spacing(pub f32);

impl Spacing {
    /// No spacing
    pub const ZERO: Self = Self(0.0);

    /// Extra small spacing (2px)
    pub const XSMALL: Self = Self(2.0);

    /// Small spacing (4px)
    pub const SMALL: Self = Self(4.0);

    /// Medium spacing (8px)
    pub const MEDIUM: Self = Self(8.0);

    /// Large spacing (16px)
    pub const LARGE: Self = Self(16.0);

    /// Extra large spacing (24px)
    pub const XLARGE: Self = Self(24.0);

    /// Create a new spacing value
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    /// Get the raw f32 value
    pub const fn value(self) -> f32 {
        self.0
    }

    /// Clamp to non-negative
    pub fn clamp(self) -> Self {
        Self(self.0.max(0.0))
    }

    /// Double the spacing
    pub fn doubled(self) -> Self {
        Self(self.0 * 2.0)
    }

    /// Half the spacing
    pub fn halved(self) -> Self {
        Self(self.0 / 2.0)
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self::MEDIUM
    }
}

impl From<f32> for Spacing {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl From<Spacing> for f32 {
    fn from(spacing: Spacing) -> f32 {
        spacing.0
    }
}

impl fmt::Display for Spacing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}px", self.0)
    }
}

// Arithmetic operations for Spacing
impl std::ops::Add for Spacing {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Spacing {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Mul<f32> for Spacing {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self(self.0 * rhs)
    }
}

impl std::ops::Div<f32> for Spacing {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self(self.0 / rhs)
    }
}

// =============================================================================
// ZoomLevel
// =============================================================================

/// Zoom level for image viewing
///
/// Represents the zoom factor where:
/// - 1.0 = 100% (1:1 pixel ratio, one image pixel = one screen pixel)
/// - 2.0 = 200% (one image pixel = two screen pixels, zoomed in)
/// - 0.5 = 50% (two image pixels = one screen pixel, zoomed out)
///
/// # Example
///
/// ```ignore
/// use hvat_ui::ZoomLevel;
///
/// let zoom = ZoomLevel::ONE_TO_ONE;
/// let zoomed_in = zoom.zoom_in();
/// let percentage = zoom.as_percentage(); // "100%"
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ZoomLevel(pub f32);

impl ZoomLevel {
    /// Minimum zoom level (10%)
    pub const MIN: Self = Self(0.1);

    /// Maximum zoom level (1000%)
    pub const MAX: Self = Self(10.0);

    /// 1:1 pixel ratio (100%)
    pub const ONE_TO_ONE: Self = Self(1.0);

    /// 50% zoom
    pub const HALF: Self = Self(0.5);

    /// 200% zoom
    pub const DOUBLE: Self = Self(2.0);

    /// Default zoom factor for zoom in/out
    pub const ZOOM_FACTOR: f32 = 1.1;

    /// Create a new zoom level
    pub const fn new(zoom: f32) -> Self {
        Self(zoom)
    }

    /// Get the raw f32 value
    pub const fn value(self) -> f32 {
        self.0
    }

    /// Clamp to valid zoom range
    pub fn clamp(self) -> Self {
        Self(self.0.clamp(Self::MIN.0, Self::MAX.0))
    }

    /// Zoom in by the standard factor
    pub fn zoom_in(self) -> Self {
        Self(self.0 * Self::ZOOM_FACTOR).clamp()
    }

    /// Zoom out by the standard factor
    pub fn zoom_out(self) -> Self {
        Self(self.0 / Self::ZOOM_FACTOR).clamp()
    }

    /// Zoom by a custom factor
    pub fn zoom_by(self, factor: f32) -> Self {
        Self(self.0 * factor).clamp()
    }

    /// Get zoom as a percentage (e.g., 1.0 -> 100.0)
    pub fn as_percentage(self) -> f32 {
        self.0 * 100.0
    }

    /// Create from percentage (e.g., 100.0 -> 1.0)
    pub fn from_percentage(percentage: f32) -> Self {
        Self(percentage / 100.0).clamp()
    }

    /// Check if this is the 1:1 zoom level
    pub fn is_one_to_one(self) -> bool {
        (self.0 - 1.0).abs() < 0.001
    }
}

impl Default for ZoomLevel {
    fn default() -> Self {
        Self::ONE_TO_ONE
    }
}

impl From<f32> for ZoomLevel {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl From<ZoomLevel> for f32 {
    fn from(zoom: ZoomLevel) -> f32 {
        zoom.0
    }
}

impl fmt::Display for ZoomLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.0}%", self.as_percentage())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_size_clamp() {
        assert_eq!(FontSize::new(5.0).clamp(), FontSize::MIN);
        assert_eq!(FontSize::new(100.0).clamp(), FontSize::MAX);
        assert_eq!(FontSize::new(14.0).clamp(), FontSize::new(14.0));
    }

    #[test]
    fn font_size_scaled() {
        assert_eq!(FontSize::new(10.0).scaled(2.0), FontSize::new(20.0));
        assert_eq!(FontSize::new(20.0).scaled(0.5), FontSize::new(10.0));
    }

    #[test]
    fn spacing_arithmetic() {
        let a = Spacing::SMALL;
        let b = Spacing::MEDIUM;
        assert_eq!(a + b, Spacing::new(12.0));
        assert_eq!(b - a, Spacing::new(4.0));
        assert_eq!(a * 2.0, Spacing::new(8.0));
        assert_eq!(b / 2.0, Spacing::new(4.0));
    }

    #[test]
    fn spacing_modifiers() {
        let s = Spacing::MEDIUM;
        assert_eq!(s.doubled(), Spacing::new(16.0));
        assert_eq!(s.halved(), Spacing::new(4.0));
    }

    #[test]
    fn zoom_level_clamp() {
        assert_eq!(ZoomLevel::new(0.05).clamp(), ZoomLevel::MIN);
        assert_eq!(ZoomLevel::new(20.0).clamp(), ZoomLevel::MAX);
        assert_eq!(ZoomLevel::new(1.0).clamp(), ZoomLevel::ONE_TO_ONE);
    }

    #[test]
    fn zoom_level_zoom_in_out() {
        let zoom = ZoomLevel::ONE_TO_ONE;
        let zoomed_in = zoom.zoom_in();
        assert!(zoomed_in.value() > 1.0);

        let zoomed_out = zoom.zoom_out();
        assert!(zoomed_out.value() < 1.0);
    }

    #[test]
    fn zoom_level_percentage() {
        assert_eq!(ZoomLevel::ONE_TO_ONE.as_percentage(), 100.0);
        assert_eq!(ZoomLevel::HALF.as_percentage(), 50.0);
        assert_eq!(ZoomLevel::DOUBLE.as_percentage(), 200.0);

        assert_eq!(ZoomLevel::from_percentage(100.0), ZoomLevel::ONE_TO_ONE);
        assert_eq!(ZoomLevel::from_percentage(50.0), ZoomLevel::HALF);
    }

    #[test]
    fn zoom_level_is_one_to_one() {
        assert!(ZoomLevel::ONE_TO_ONE.is_one_to_one());
        assert!(ZoomLevel::new(1.0001).is_one_to_one()); // Within tolerance
        assert!(!ZoomLevel::HALF.is_one_to_one());
        assert!(!ZoomLevel::DOUBLE.is_one_to_one());
    }

    #[test]
    fn type_safety() {
        // This test demonstrates that these types can't be accidentally mixed
        let _font: FontSize = FontSize::DEFAULT;
        let _spacing: Spacing = Spacing::MEDIUM;
        let _zoom: ZoomLevel = ZoomLevel::ONE_TO_ONE;

        // These would not compile (type mismatch):
        // let _font: FontSize = spacing;
        // let _spacing: Spacing = zoom;
        // let _zoom: ZoomLevel = font;
    }
}
