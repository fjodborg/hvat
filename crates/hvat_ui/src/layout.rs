// ============================================================================
// Type-Safe Size Types
// ============================================================================

/// A size value guaranteed to be finite and non-negative.
/// Used for natural_size() returns - can NEVER be infinity.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ConcreteSize(f32);

impl ConcreteSize {
    /// Zero size constant.
    pub const ZERO: Self = Self(0.0);

    /// Create a ConcreteSize if the value is finite and non-negative.
    pub fn new(value: f32) -> Option<Self> {
        if value.is_finite() && value >= 0.0 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Create a ConcreteSize without validation.
    /// Panics in debug builds if the value is invalid.
    pub fn new_unchecked(value: f32) -> Self {
        debug_assert!(
            value.is_finite() && value >= 0.0,
            "ConcreteSize must be finite and non-negative, got {}",
            value
        );
        // Clamp to valid range in release builds for safety
        Self(if value.is_finite() && value >= 0.0 { value } else { 0.0 })
    }

    /// Get the inner value.
    #[inline]
    pub fn get(self) -> f32 {
        self.0
    }

    /// Return the maximum of two sizes.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    /// Return the minimum of two sizes.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }
}

impl std::ops::Add for ConcreteSize {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for ConcreteSize {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl std::ops::Sub for ConcreteSize {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self((self.0 - rhs.0).max(0.0))
    }
}

impl Default for ConcreteSize {
    fn default() -> Self {
        Self::ZERO
    }
}

/// A 2D size with guaranteed finite, non-negative dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ConcreteSizeXY {
    pub width: ConcreteSize,
    pub height: ConcreteSize,
}

impl ConcreteSizeXY {
    /// Zero size constant.
    pub const ZERO: Self = Self {
        width: ConcreteSize::ZERO,
        height: ConcreteSize::ZERO,
    };

    /// Create a new ConcreteSizeXY.
    pub fn new(width: ConcreteSize, height: ConcreteSize) -> Self {
        Self { width, height }
    }

    /// Create from raw f32 values (unchecked).
    pub fn from_f32(width: f32, height: f32) -> Self {
        Self {
            width: ConcreteSize::new_unchecked(width),
            height: ConcreteSize::new_unchecked(height),
        }
    }
}

// ============================================================================
// Layout Context Markers
// ============================================================================

/// Marker type: layout context where Fill makes sense (bounded parent container).
/// Used as a type parameter to enable/disable Fill methods at compile time.
#[derive(Debug, Clone, Copy, Default)]
pub struct Bounded;

/// Marker type: layout context where Fill would be infinite (inside scrollable).
/// Used as a type parameter to disable Fill methods at compile time.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unbounded;

// ============================================================================
// Sizing Mode
// ============================================================================

/// Sizing mode for a single axis - indicates whether a widget has fixed size or wants to fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizingMode {
    /// Widget has a fixed/intrinsic size (the value in Layout bounds)
    Fixed,
    /// Widget wants to fill available space with relative weight (1.0 = equal share)
    Fill(f32),
}

impl Default for SizingMode {
    fn default() -> Self {
        SizingMode::Fixed
    }
}

/// Size constraints for widget layout.
///
/// Limits define the minimum and maximum size a widget can have.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl Limits {
    /// Create limits with fixed size.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }

    /// Create limits with a range of sizes.
    pub fn with_range(
        min_width: f32,
        max_width: f32,
        min_height: f32,
        max_height: f32,
    ) -> Self {
        Self {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }

    /// Fill all available space.
    pub fn fill() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }

    /// Get the width of these limits.
    pub fn width(&self) -> f32 {
        self.max_width
    }

    /// Get the height of these limits.
    pub fn height(&self) -> f32 {
        self.max_height
    }

    /// Resolve a size within these limits.
    pub fn resolve(&self, width: f32, height: f32) -> Size {
        Size {
            width: width.max(self.min_width).min(self.max_width),
            height: height.max(self.min_height).min(self.max_height),
        }
    }

    /// Check if limits are valid (min <= max, all non-negative).
    pub fn is_valid(&self) -> bool {
        self.min_width >= 0.0
            && self.min_height >= 0.0
            && self.min_width <= self.max_width
            && self.min_height <= self.max_height
    }

    /// Create validated limits, clamping invalid values.
    pub fn validated(mut self) -> Self {
        self.min_width = self.min_width.max(0.0);
        self.min_height = self.min_height.max(0.0);
        if self.max_width < self.min_width {
            self.max_width = self.min_width;
        }
        if self.max_height < self.min_height {
            self.max_height = self.min_height;
        }
        self
    }

    /// Check if width is bounded (finite max_width).
    pub fn is_width_bounded(&self) -> bool {
        self.max_width.is_finite()
    }

    /// Check if height is bounded (finite max_height).
    pub fn is_height_bounded(&self) -> bool {
        self.max_height.is_finite()
    }
}

/// A 2D size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

/// A 2D point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// A rectangle defined by position and size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rectangle {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    pub fn position(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Create a new rectangle with padding applied (inset from all sides).
    /// Returns a smaller rectangle inside this one.
    pub fn with_padding(&self, padding: f32) -> Rectangle {
        Rectangle::new(
            self.x + padding,
            self.y + padding,
            (self.width - padding * 2.0).max(0.0),
            (self.height - padding * 2.0).max(0.0),
        )
    }

    /// Get the center point of this rectangle.
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Compute the intersection of two rectangles.
    /// Returns a rectangle representing the overlapping area.
    /// If there's no overlap, returns a zero-sized rectangle at the origin.
    pub fn intersect(&self, other: &Rectangle) -> Rectangle {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        let width = (x2 - x1).max(0.0);
        let height = (y2 - y1).max(0.0);

        Rectangle::new(x1, y1, width, height)
    }
}

/// The layout of a widget - its position, size, and sizing intent.
#[derive(Debug, Clone)]
pub struct Layout {
    bounds: Rectangle,
    /// How this widget wants to be sized horizontally
    width_mode: SizingMode,
    /// How this widget wants to be sized vertically
    height_mode: SizingMode,
}

impl Layout {
    /// Create a new layout with the given bounds (defaults to Fixed sizing).
    pub fn new(bounds: Rectangle) -> Self {
        Self {
            bounds,
            width_mode: SizingMode::Fixed,
            height_mode: SizingMode::Fixed,
        }
    }

    /// Create a layout that fills horizontally with fixed height.
    pub fn fill_width(bounds: Rectangle) -> Self {
        Self {
            bounds,
            width_mode: SizingMode::Fill(1.0),
            height_mode: SizingMode::Fixed,
        }
    }

    /// Create a layout that fills vertically with fixed width.
    pub fn fill_height(bounds: Rectangle) -> Self {
        Self {
            bounds,
            width_mode: SizingMode::Fixed,
            height_mode: SizingMode::Fill(1.0),
        }
    }

    /// Create a layout that fills both dimensions.
    pub fn fill_both(bounds: Rectangle) -> Self {
        Self {
            bounds,
            width_mode: SizingMode::Fill(1.0),
            height_mode: SizingMode::Fill(1.0),
        }
    }

    /// Builder: set width mode.
    pub fn with_width_mode(mut self, mode: SizingMode) -> Self {
        self.width_mode = mode;
        self
    }

    /// Builder: set height mode.
    pub fn with_height_mode(mut self, mode: SizingMode) -> Self {
        self.height_mode = mode;
        self
    }

    /// Get the bounds of this layout.
    pub fn bounds(&self) -> Rectangle {
        self.bounds
    }

    /// Get the position of this layout.
    pub fn position(&self) -> Point {
        self.bounds.position()
    }

    /// Get the size of this layout.
    pub fn size(&self) -> Size {
        self.bounds.size()
    }

    /// Check if this layout wants to fill horizontally.
    pub fn fills_width(&self) -> bool {
        matches!(self.width_mode, SizingMode::Fill(_))
    }

    /// Check if this layout wants to fill vertically.
    pub fn fills_height(&self) -> bool {
        matches!(self.height_mode, SizingMode::Fill(_))
    }

    /// Get the fill weight for width (returns 0 if not filling).
    pub fn width_fill_weight(&self) -> f32 {
        match self.width_mode {
            SizingMode::Fill(w) => w,
            SizingMode::Fixed => 0.0,
        }
    }

    /// Get the fill weight for height (returns 0 if not filling).
    pub fn height_fill_weight(&self) -> f32 {
        match self.height_mode {
            SizingMode::Fill(w) => w,
            SizingMode::Fixed => 0.0,
        }
    }

    /// Get the width sizing mode.
    pub fn width_mode(&self) -> SizingMode {
        self.width_mode
    }

    /// Get the height sizing mode.
    pub fn height_mode(&self) -> SizingMode {
        self.height_mode
    }
}
