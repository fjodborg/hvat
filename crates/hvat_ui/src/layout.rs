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

/// The layout of a widget - its position and size.
#[derive(Debug, Clone)]
pub struct Layout {
    bounds: Rectangle,
}

impl Layout {
    /// Create a new layout with the given bounds.
    pub fn new(bounds: Rectangle) -> Self {
        Self { bounds }
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
}
