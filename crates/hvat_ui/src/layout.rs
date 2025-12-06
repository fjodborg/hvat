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

/// Measurement context - tells widgets how to interpret limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeasureContext {
    /// Normal layout - use finite bounds, fill widgets expand
    #[default]
    Normal,
    /// Content measurement for scrollable - report natural size, ignore fill behavior
    ContentMeasure,
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
    /// Context for how to interpret these limits
    pub context: MeasureContext,
}

impl Limits {
    /// Create limits with fixed size.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
            context: MeasureContext::Normal,
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
            context: MeasureContext::Normal,
        }
    }

    /// Fill all available space.
    pub fn fill() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
            context: MeasureContext::Normal,
        }
    }

    /// Create limits for measuring natural content size (used by scrollables).
    /// Children should report their intrinsic size, ignoring fill behavior.
    pub fn for_content_measure(max_width: f32, max_height: f32) -> Self {
        Self {
            min_width: 0.0,
            max_width,
            min_height: 0.0,
            max_height,
            context: MeasureContext::ContentMeasure,
        }
    }

    /// Check if we're in content measurement mode.
    pub fn is_content_measure(&self) -> bool {
        self.context == MeasureContext::ContentMeasure
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
