//! Overlay shapes for rendering on top of images.
//!
//! This module provides simple shape types that can be drawn as overlays.
//! These are decoupled from application-specific annotation types.

use crate::Color;

/// A shape that can be drawn as an overlay on an image.
#[derive(Debug, Clone)]
pub enum OverlayShape {
    /// A point marker (filled circle).
    Point {
        /// X coordinate in image space
        x: f32,
        /// Y coordinate in image space
        y: f32,
        /// Radius in screen pixels
        radius: f32,
    },
    /// A rectangle (bounding box).
    Rect {
        /// Left edge X coordinate in image space
        x: f32,
        /// Top edge Y coordinate in image space
        y: f32,
        /// Width in image space
        width: f32,
        /// Height in image space
        height: f32,
    },
    /// A polygon defined by vertices.
    Polygon {
        /// Vertices in image space
        vertices: Vec<(f32, f32)>,
        /// Whether the polygon is closed
        closed: bool,
    },
    /// A line segment.
    Line {
        /// Start X in image space
        x1: f32,
        /// Start Y in image space
        y1: f32,
        /// End X in image space
        x2: f32,
        /// End Y in image space
        y2: f32,
    },
}

/// An overlay item with shape and styling.
#[derive(Debug, Clone)]
pub struct OverlayItem {
    /// The shape to draw
    pub shape: OverlayShape,
    /// Fill color (for points and rects)
    pub color: Color,
    /// Whether this item is selected
    pub selected: bool,
}

impl OverlayItem {
    /// Create a new overlay item.
    pub fn new(shape: OverlayShape, color: Color) -> Self {
        Self {
            shape,
            color,
            selected: false,
        }
    }

    /// Mark this item as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// A collection of overlay items to render.
#[derive(Debug, Clone, Default)]
pub struct Overlay {
    /// Items to render
    pub items: Vec<OverlayItem>,
    /// Optional preview shape (for drawing in progress)
    pub preview: Option<OverlayItem>,
}

impl Overlay {
    /// Create a new empty overlay.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an item to the overlay.
    pub fn push(&mut self, item: OverlayItem) {
        self.items.push(item);
    }

    /// Set the preview shape.
    pub fn set_preview(&mut self, preview: Option<OverlayItem>) {
        self.preview = preview;
    }

    /// Check if the overlay is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.preview.is_none()
    }
}
