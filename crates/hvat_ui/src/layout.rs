//! Layout types for widget positioning and sizing

/// A rectangle defining position and size
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Bounds {
    pub const ZERO: Bounds = Bounds {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        debug_assert!(width >= 0.0, "Bounds width cannot be negative: {}", width);
        debug_assert!(
            height >= 0.0,
            "Bounds height cannot be negative: {}",
            height
        );
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_size(size: Size) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: size.width,
            height: size.height,
        }
    }

    /// Check if a point is inside these bounds
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Get the right edge x coordinate
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Get the bottom edge y coordinate
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Get the center point
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Get the size as a Size struct
    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Shrink bounds by padding
    pub fn shrink(&self, padding: Padding) -> Self {
        Self {
            x: self.x + padding.left,
            y: self.y + padding.top,
            width: (self.width - padding.left - padding.right).max(0.0),
            height: (self.height - padding.top - padding.bottom).max(0.0),
        }
    }

    /// Expand bounds by padding
    pub fn expand(&self, padding: Padding) -> Self {
        Self {
            x: self.x - padding.left,
            y: self.y - padding.top,
            width: self.width + padding.left + padding.right,
            height: self.height + padding.top + padding.bottom,
        }
    }

    /// Intersect with another bounds
    pub fn intersect(&self, other: &Bounds) -> Option<Bounds> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        if right > x && bottom > y {
            Some(Bounds::new(x, y, right - x, bottom - y))
        } else {
            None
        }
    }

    /// Union with another bounds (smallest rectangle containing both)
    pub fn union(&self, other: &Bounds) -> Bounds {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Bounds::new(x, y, right - x, bottom - y)
    }

    /// Translate these bounds (relative coordinates) to absolute position within a container
    ///
    /// This is commonly used when positioning child widgets within a parent container.
    /// The child's bounds are relative to the parent (starting from 0,0), and this method
    /// translates them to absolute screen coordinates.
    ///
    /// # Example
    /// ```
    /// use hvat_ui::Bounds;
    ///
    /// let parent = Bounds::new(100.0, 200.0, 400.0, 300.0);
    /// let child_relative = Bounds::new(10.0, 20.0, 50.0, 30.0);
    /// let child_absolute = child_relative.translate_to(parent);
    ///
    /// assert_eq!(child_absolute.x, 110.0);  // 100 + 10
    /// assert_eq!(child_absolute.y, 220.0);  // 200 + 20
    /// assert_eq!(child_absolute.width, 50.0);
    /// assert_eq!(child_absolute.height, 30.0);
    /// ```
    pub fn translate_to(&self, container: Bounds) -> Bounds {
        Bounds::new(
            container.x + self.x,
            container.y + self.y,
            self.width,
            self.height,
        )
    }
}

/// A size without position
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Size = Size {
        width: 0.0,
        height: 0.0,
    };

    pub fn new(width: f32, height: f32) -> Self {
        debug_assert!(width >= 0.0, "Size width cannot be negative: {}", width);
        debug_assert!(height >= 0.0, "Size height cannot be negative: {}", height);
        Self { width, height }
    }

    /// Create a square size
    pub fn square(size: f32) -> Self {
        debug_assert!(size >= 0.0, "Square size cannot be negative: {}", size);
        Self {
            width: size,
            height: size,
        }
    }
}

/// Length specification for widget dimensions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    /// Fixed pixel size
    Fixed(f32),
    /// Fill available space with optional weight (default 1.0)
    Fill(f32),
    /// Shrink to fit content
    Shrink,
}

impl Length {
    /// Fill with default weight of 1.0
    pub fn fill() -> Self {
        Length::Fill(1.0)
    }

    /// Fill with specific weight
    pub fn fill_weighted(weight: f32) -> Self {
        Length::Fill(weight)
    }

    /// Resolve length given available space and content size
    pub fn resolve(&self, available: f32, content: f32) -> f32 {
        match self {
            Length::Fixed(px) => *px,
            Length::Fill(_) => available,
            Length::Shrink => content.min(available),
        }
    }

    /// Check if this is a fill length
    pub fn is_fill(&self) -> bool {
        matches!(self, Length::Fill(_))
    }

    /// Get the fill weight, or 0 if not fill
    pub fn fill_weight(&self) -> f32 {
        match self {
            Length::Fill(w) => *w,
            _ => 0.0,
        }
    }
}

impl Default for Length {
    fn default() -> Self {
        Length::Shrink
    }
}

impl From<f32> for Length {
    fn from(px: f32) -> Self {
        Length::Fixed(px)
    }
}

/// Padding around a widget
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Padding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Padding {
    pub const ZERO: Padding = Padding {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    /// Create uniform padding on all sides
    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Create padding with horizontal and vertical values
    pub fn axes(horizontal: f32, vertical: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Create padding with individual values
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Total horizontal padding
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// Total vertical padding
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

impl From<f32> for Padding {
    fn from(value: f32) -> Self {
        Padding::all(value)
    }
}

impl From<[f32; 2]> for Padding {
    fn from([h, v]: [f32; 2]) -> Self {
        Padding::axes(h, v)
    }
}

impl From<[f32; 4]> for Padding {
    fn from([top, right, bottom, left]: [f32; 4]) -> Self {
        Padding::new(top, right, bottom, left)
    }
}

/// Alignment options for positioning children
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

impl Alignment {
    /// Calculate offset for aligning content within available space
    pub fn align(&self, available: f32, content: f32) -> f32 {
        match self {
            Alignment::Left => 0.0,
            Alignment::Center => (available - content) / 2.0,
            Alignment::Right => available - content,
        }
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Bounds Tests
    // =========================================================================

    #[test]
    fn bounds_new() {
        let b = Bounds::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(b.x, 10.0);
        assert_eq!(b.y, 20.0);
        assert_eq!(b.width, 100.0);
        assert_eq!(b.height, 50.0);
    }

    #[test]
    fn bounds_from_size() {
        let size = Size::new(200.0, 150.0);
        let b = Bounds::from_size(size);
        assert_eq!(b.x, 0.0);
        assert_eq!(b.y, 0.0);
        assert_eq!(b.width, 200.0);
        assert_eq!(b.height, 150.0);
    }

    #[test]
    fn bounds_contains() {
        let b = Bounds::new(10.0, 20.0, 100.0, 50.0);

        // Inside
        assert!(b.contains(50.0, 40.0));
        assert!(b.contains(10.0, 20.0)); // Top-left edge

        // Outside
        assert!(!b.contains(5.0, 40.0)); // Left of bounds
        assert!(!b.contains(120.0, 40.0)); // Right of bounds (at x + width)
        assert!(!b.contains(50.0, 15.0)); // Above bounds
        assert!(!b.contains(50.0, 75.0)); // Below bounds (at y + height)
    }

    #[test]
    fn bounds_right_bottom() {
        let b = Bounds::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(b.right(), 110.0); // 10 + 100
        assert_eq!(b.bottom(), 70.0); // 20 + 50
    }

    #[test]
    fn bounds_center() {
        let b = Bounds::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(b.center(), (50.0, 25.0));

        let b2 = Bounds::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(b2.center(), (60.0, 45.0)); // (10 + 50, 20 + 25)
    }

    #[test]
    fn bounds_size() {
        let b = Bounds::new(10.0, 20.0, 100.0, 50.0);
        let size = b.size();
        assert_eq!(size.width, 100.0);
        assert_eq!(size.height, 50.0);
    }

    #[test]
    fn bounds_shrink() {
        let b = Bounds::new(10.0, 20.0, 100.0, 80.0);
        let padding = Padding::new(5.0, 10.0, 15.0, 20.0);
        let shrunk = b.shrink(padding);

        assert_eq!(shrunk.x, 30.0); // 10 + 20 (left)
        assert_eq!(shrunk.y, 25.0); // 20 + 5 (top)
        assert_eq!(shrunk.width, 70.0); // 100 - 20 - 10
        assert_eq!(shrunk.height, 60.0); // 80 - 5 - 15
    }

    #[test]
    fn bounds_shrink_negative_clamped() {
        let b = Bounds::new(0.0, 0.0, 10.0, 10.0);
        let huge_padding = Padding::all(20.0); // Would result in negative size
        let shrunk = b.shrink(huge_padding);

        // Should clamp to 0
        assert_eq!(shrunk.width, 0.0);
        assert_eq!(shrunk.height, 0.0);
    }

    #[test]
    fn bounds_expand() {
        let b = Bounds::new(50.0, 50.0, 100.0, 80.0);
        let padding = Padding::all(10.0);
        let expanded = b.expand(padding);

        assert_eq!(expanded.x, 40.0); // 50 - 10
        assert_eq!(expanded.y, 40.0); // 50 - 10
        assert_eq!(expanded.width, 120.0); // 100 + 10 + 10
        assert_eq!(expanded.height, 100.0); // 80 + 10 + 10
    }

    #[test]
    fn bounds_intersect_overlapping() {
        let b1 = Bounds::new(0.0, 0.0, 100.0, 100.0);
        let b2 = Bounds::new(50.0, 50.0, 100.0, 100.0);

        let intersection = b1.intersect(&b2).unwrap();
        assert_eq!(intersection.x, 50.0);
        assert_eq!(intersection.y, 50.0);
        assert_eq!(intersection.width, 50.0);
        assert_eq!(intersection.height, 50.0);
    }

    #[test]
    fn bounds_intersect_non_overlapping() {
        let b1 = Bounds::new(0.0, 0.0, 50.0, 50.0);
        let b2 = Bounds::new(100.0, 100.0, 50.0, 50.0);

        assert_eq!(b1.intersect(&b2), None);
    }

    #[test]
    fn bounds_union() {
        let b1 = Bounds::new(0.0, 0.0, 50.0, 50.0);
        let b2 = Bounds::new(100.0, 100.0, 50.0, 50.0);

        let union = b1.union(&b2);
        assert_eq!(union.x, 0.0);
        assert_eq!(union.y, 0.0);
        assert_eq!(union.width, 150.0); // 0 to 150
        assert_eq!(union.height, 150.0); // 0 to 150
    }

    #[test]
    fn bounds_translate_to() {
        let parent = Bounds::new(100.0, 200.0, 400.0, 300.0);
        let child = Bounds::new(10.0, 20.0, 50.0, 30.0);

        let translated = child.translate_to(parent);
        assert_eq!(translated.x, 110.0); // 100 + 10
        assert_eq!(translated.y, 220.0); // 200 + 20
        assert_eq!(translated.width, 50.0); // Unchanged
        assert_eq!(translated.height, 30.0); // Unchanged
    }

    // =========================================================================
    // Size Tests
    // =========================================================================

    #[test]
    fn size_new() {
        let s = Size::new(100.0, 50.0);
        assert_eq!(s.width, 100.0);
        assert_eq!(s.height, 50.0);
    }

    #[test]
    fn size_square() {
        let s = Size::square(75.0);
        assert_eq!(s.width, 75.0);
        assert_eq!(s.height, 75.0);
    }

    #[test]
    fn size_zero() {
        assert_eq!(Size::ZERO.width, 0.0);
        assert_eq!(Size::ZERO.height, 0.0);
    }

    // =========================================================================
    // Length Tests
    // =========================================================================

    #[test]
    fn length_fixed_resolve() {
        let length = Length::Fixed(100.0);
        assert_eq!(length.resolve(200.0, 50.0), 100.0);
        assert_eq!(length.resolve(50.0, 150.0), 100.0); // Always returns fixed value
    }

    #[test]
    fn length_fill_resolve() {
        let length = Length::Fill(1.0);
        assert_eq!(length.resolve(200.0, 50.0), 200.0); // Uses available
        assert_eq!(length.resolve(100.0, 150.0), 100.0);
    }

    #[test]
    fn length_shrink_resolve() {
        let length = Length::Shrink;
        assert_eq!(length.resolve(200.0, 50.0), 50.0); // Uses content
        assert_eq!(length.resolve(100.0, 150.0), 100.0); // Clamped to available
    }

    #[test]
    fn length_is_fill() {
        assert!(Length::Fill(1.0).is_fill());
        assert!(Length::fill().is_fill());
        assert!(!Length::Fixed(100.0).is_fill());
        assert!(!Length::Shrink.is_fill());
    }

    #[test]
    fn length_fill_weight() {
        assert_eq!(Length::Fill(2.5).fill_weight(), 2.5);
        assert_eq!(Length::Fixed(100.0).fill_weight(), 0.0);
        assert_eq!(Length::Shrink.fill_weight(), 0.0);
    }

    #[test]
    fn length_from_f32() {
        let length: Length = 150.0.into();
        assert_eq!(length, Length::Fixed(150.0));
    }

    // =========================================================================
    // Padding Tests
    // =========================================================================

    #[test]
    fn padding_all() {
        let p = Padding::all(10.0);
        assert_eq!(p.top, 10.0);
        assert_eq!(p.right, 10.0);
        assert_eq!(p.bottom, 10.0);
        assert_eq!(p.left, 10.0);
    }

    #[test]
    fn padding_axes() {
        let p = Padding::axes(20.0, 10.0); // horizontal, vertical
        assert_eq!(p.top, 10.0);
        assert_eq!(p.right, 20.0);
        assert_eq!(p.bottom, 10.0);
        assert_eq!(p.left, 20.0);
    }

    #[test]
    fn padding_new() {
        let p = Padding::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(p.top, 1.0);
        assert_eq!(p.right, 2.0);
        assert_eq!(p.bottom, 3.0);
        assert_eq!(p.left, 4.0);
    }

    #[test]
    fn padding_horizontal_vertical() {
        let p = Padding::new(10.0, 15.0, 20.0, 25.0);
        assert_eq!(p.horizontal(), 40.0); // 15 + 25
        assert_eq!(p.vertical(), 30.0); // 10 + 20
    }

    #[test]
    fn padding_from_f32() {
        let p: Padding = 5.0.into();
        assert_eq!(p, Padding::all(5.0));
    }

    #[test]
    fn padding_from_array_2() {
        let p: Padding = [10.0, 20.0].into(); // [horizontal, vertical]
        assert_eq!(p.top, 20.0);
        assert_eq!(p.right, 10.0);
        assert_eq!(p.bottom, 20.0);
        assert_eq!(p.left, 10.0);
    }

    #[test]
    fn padding_from_array_4() {
        let p: Padding = [1.0, 2.0, 3.0, 4.0].into(); // [top, right, bottom, left]
        assert_eq!(p.top, 1.0);
        assert_eq!(p.right, 2.0);
        assert_eq!(p.bottom, 3.0);
        assert_eq!(p.left, 4.0);
    }

    #[test]
    fn padding_zero() {
        assert_eq!(Padding::ZERO.top, 0.0);
        assert_eq!(Padding::ZERO.right, 0.0);
        assert_eq!(Padding::ZERO.bottom, 0.0);
        assert_eq!(Padding::ZERO.left, 0.0);
    }

    // =========================================================================
    // Alignment Tests
    // =========================================================================

    #[test]
    fn alignment_left() {
        let align = Alignment::Left;
        assert_eq!(align.align(100.0, 50.0), 0.0);
        assert_eq!(align.align(200.0, 80.0), 0.0);
    }

    #[test]
    fn alignment_center() {
        let align = Alignment::Center;
        assert_eq!(align.align(100.0, 50.0), 25.0); // (100 - 50) / 2
        assert_eq!(align.align(200.0, 80.0), 60.0); // (200 - 80) / 2
    }

    #[test]
    fn alignment_right() {
        let align = Alignment::Right;
        assert_eq!(align.align(100.0, 50.0), 50.0); // 100 - 50
        assert_eq!(align.align(200.0, 80.0), 120.0); // 200 - 80
    }

    #[test]
    fn alignment_default() {
        assert_eq!(Alignment::default(), Alignment::Left);
    }
}
