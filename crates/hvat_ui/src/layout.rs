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
        Self { width, height }
    }

    /// Create a square size
    pub fn square(size: f32) -> Self {
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
    Start,
    Center,
    End,
}

impl Alignment {
    /// Calculate offset for aligning content within available space
    pub fn align(&self, available: f32, content: f32) -> f32 {
        match self {
            Alignment::Start => 0.0,
            Alignment::Center => (available - content) / 2.0,
            Alignment::End => available - content,
        }
    }
}
