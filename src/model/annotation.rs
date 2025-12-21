//! Annotation tool types and data structures.

use serde::{Deserialize, Serialize};

/// Unique identifier for an annotation.
pub type AnnotationId = u32;

/// Minimum size (width/height) for a valid bounding box.
pub const MIN_BBOX_SIZE: f32 = 1.0;

/// Minimum number of vertices required for a valid polygon.
pub const MIN_POLYGON_VERTICES: usize = 3;

/// Hit radius for point annotation selection (in image pixels).
pub const POINT_HIT_RADIUS: f32 = 10.0;

/// Distance threshold for closing a polygon by clicking near the first vertex.
pub const POLYGON_CLOSE_THRESHOLD: f32 = 15.0;

/// Hit radius for annotation handles (in image pixels).
/// This is scaled by zoom level when checking.
pub const HANDLE_HIT_RADIUS: f32 = 8.0;

/// Handle type for bounding box corners and edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BBoxHandle {
    /// Top-left corner
    TopLeft,
    /// Top-right corner
    TopRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-right corner
    BottomRight,
    /// Top edge (for vertical resize)
    TopEdge,
    /// Bottom edge (for vertical resize)
    BottomEdge,
    /// Left edge (for horizontal resize)
    LeftEdge,
    /// Right edge (for horizontal resize)
    RightEdge,
    /// Center (for move)
    Center,
}

/// Handle type for polygon vertices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonHandle {
    /// A specific vertex index
    Vertex(usize),
    /// Center of the polygon (for move)
    Center,
}

/// Unified handle type for any annotation shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationHandle {
    /// Handle for a bounding box
    BBox(BBoxHandle),
    /// Handle for a polygon vertex or center
    Polygon(PolygonHandle),
    /// Point annotation (move the whole point)
    Point,
}

/// Minimum drag distance (in image pixels) before we consider it a real drag vs a click.
pub const MIN_DRAG_DISTANCE: f32 = 3.0;

/// State for editing an existing annotation.
#[derive(Debug, Clone)]
pub enum EditState {
    /// Not editing any annotation.
    Idle,
    /// Mouse down on a handle, but haven't moved enough to start editing yet.
    /// If mouse is released without enough movement, this becomes a click (for cycling).
    PotentialDrag {
        /// ID of the annotation that might be edited
        annotation_id: AnnotationId,
        /// The handle that was clicked
        handle: AnnotationHandle,
        /// Starting mouse position in image coordinates
        start_x: f32,
        start_y: f32,
        /// Original shape before the potential drag
        original_shape: AnnotationShape,
    },
    /// Actively dragging a handle to resize/move an annotation.
    DraggingHandle {
        /// ID of the annotation being edited
        annotation_id: AnnotationId,
        /// The handle being dragged
        handle: AnnotationHandle,
        /// Starting mouse position in image coordinates
        start_x: f32,
        start_y: f32,
        /// Original shape before the drag started (for calculating delta)
        original_shape: AnnotationShape,
    },
}

impl Default for EditState {
    fn default() -> Self {
        EditState::Idle
    }
}

impl EditState {
    /// Check if we're currently editing something (actively dragging).
    pub fn is_editing(&self) -> bool {
        matches!(self, EditState::DraggingHandle { .. })
    }

    /// Check if we have a potential drag in progress.
    pub fn is_potential_drag(&self) -> bool {
        matches!(self, EditState::PotentialDrag { .. })
    }

    /// Get the annotation ID being edited, if any.
    pub fn editing_annotation_id(&self) -> Option<AnnotationId> {
        match self {
            EditState::Idle => None,
            EditState::PotentialDrag { annotation_id, .. } => Some(*annotation_id),
            EditState::DraggingHandle { annotation_id, .. } => Some(*annotation_id),
        }
    }
}

/// Annotation tools available in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationTool {
    /// Selection tool for selecting existing annotations
    Select,
    /// Bounding box annotation tool
    BoundingBox,
    /// Polygon annotation tool
    Polygon,
    /// Point annotation tool
    Point,
}

impl Default for AnnotationTool {
    fn default() -> Self {
        AnnotationTool::Select
    }
}

impl AnnotationTool {
    /// Get the display name for this tool.
    pub fn name(&self) -> &'static str {
        match self {
            AnnotationTool::Select => "Select",
            AnnotationTool::BoundingBox => "Bounding Box",
            AnnotationTool::Polygon => "Polygon",
            AnnotationTool::Point => "Point",
        }
    }

    /// Get all available annotation tools.
    pub fn all() -> &'static [AnnotationTool] {
        &[
            AnnotationTool::Select,
            AnnotationTool::BoundingBox,
            AnnotationTool::Polygon,
            AnnotationTool::Point,
        ]
    }

    /// Check if this tool is a drawing tool (not Select).
    pub fn is_drawing_tool(&self) -> bool {
        !matches!(self, AnnotationTool::Select)
    }
}

/// Shape data for an annotation (in image coordinates).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationShape {
    /// Bounding box defined by top-left corner and size.
    BoundingBox {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Single point marker.
    Point { x: f32, y: f32 },
    /// Polygon defined by vertices.
    Polygon { vertices: Vec<(f32, f32)> },
}

impl AnnotationShape {
    /// Create a normalized bounding box from two corner points.
    /// Returns None if the box is too small.
    pub fn bounding_box_from_corners(x1: f32, y1: f32, x2: f32, y2: f32) -> Option<Self> {
        let x = x1.min(x2);
        let y = y1.min(y2);
        let width = (x2 - x1).abs();
        let height = (y2 - y1).abs();

        if width > MIN_BBOX_SIZE && height > MIN_BBOX_SIZE {
            Some(AnnotationShape::BoundingBox {
                x,
                y,
                width,
                height,
            })
        } else {
            None
        }
    }

    /// Check if a point is inside this shape.
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        match self {
            AnnotationShape::BoundingBox {
                x: bx,
                y: by,
                width,
                height,
            } => x >= *bx && x <= bx + width && y >= *by && y <= by + height,
            AnnotationShape::Point { x: px, y: py } => {
                let dx = x - px;
                let dy = y - py;
                (dx * dx + dy * dy).sqrt() < POINT_HIT_RADIUS
            }
            AnnotationShape::Polygon { vertices } => {
                // Point-in-polygon test using ray casting algorithm
                if vertices.len() < MIN_POLYGON_VERTICES {
                    return false;
                }
                let mut inside = false;
                let mut j = vertices.len() - 1;
                for i in 0..vertices.len() {
                    let (xi, yi) = vertices[i];
                    let (xj, yj) = vertices[j];
                    if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
                        inside = !inside;
                    }
                    j = i;
                }
                inside
            }
        }
    }

    /// Check if a point hits a handle on this shape.
    /// Returns the handle type if a hit is detected.
    /// `hit_radius` should be scaled appropriately for the current zoom level.
    pub fn hit_test_handle(&self, x: f32, y: f32, hit_radius: f32) -> Option<AnnotationHandle> {
        match self {
            AnnotationShape::BoundingBox {
                x: bx,
                y: by,
                width,
                height,
            } => {
                let left = *bx;
                let right = bx + width;
                let top = *by;
                let bottom = by + height;
                let cx = bx + width / 2.0;
                let cy = by + height / 2.0;

                // Check corners first (highest priority)
                let corners = [
                    (left, top, BBoxHandle::TopLeft),
                    (right, top, BBoxHandle::TopRight),
                    (left, bottom, BBoxHandle::BottomLeft),
                    (right, bottom, BBoxHandle::BottomRight),
                ];

                for (hx, hy, handle) in corners {
                    if point_distance(x, y, hx, hy) <= hit_radius {
                        return Some(AnnotationHandle::BBox(handle));
                    }
                }

                // Check edges (midpoints)
                let edges = [
                    (cx, top, BBoxHandle::TopEdge),
                    (cx, bottom, BBoxHandle::BottomEdge),
                    (left, cy, BBoxHandle::LeftEdge),
                    (right, cy, BBoxHandle::RightEdge),
                ];

                for (hx, hy, handle) in edges {
                    if point_distance(x, y, hx, hy) <= hit_radius {
                        return Some(AnnotationHandle::BBox(handle));
                    }
                }

                // Check center (for move)
                if point_distance(x, y, cx, cy) <= hit_radius {
                    return Some(AnnotationHandle::BBox(BBoxHandle::Center));
                }

                // Check if inside the box (also move)
                if self.contains_point(x, y) {
                    return Some(AnnotationHandle::BBox(BBoxHandle::Center));
                }

                None
            }
            AnnotationShape::Point { x: px, y: py } => {
                if point_distance(x, y, *px, *py) <= hit_radius.max(POINT_HIT_RADIUS) {
                    Some(AnnotationHandle::Point)
                } else {
                    None
                }
            }
            AnnotationShape::Polygon { vertices } => {
                // Check vertices first
                for (i, (vx, vy)) in vertices.iter().enumerate() {
                    if point_distance(x, y, *vx, *vy) <= hit_radius {
                        return Some(AnnotationHandle::Polygon(PolygonHandle::Vertex(i)));
                    }
                }

                // Check center (for move)
                if let Some((cx, cy)) = polygon_centroid(vertices) {
                    if point_distance(x, y, cx, cy) <= hit_radius {
                        return Some(AnnotationHandle::Polygon(PolygonHandle::Center));
                    }
                }

                // Check if inside polygon (also move)
                if self.contains_point(x, y) {
                    return Some(AnnotationHandle::Polygon(PolygonHandle::Center));
                }

                None
            }
        }
    }

    /// Apply a delta movement to this shape.
    pub fn translate(&mut self, dx: f32, dy: f32) {
        match self {
            AnnotationShape::BoundingBox { x, y, .. } => {
                *x += dx;
                *y += dy;
            }
            AnnotationShape::Point { x, y } => {
                *x += dx;
                *y += dy;
            }
            AnnotationShape::Polygon { vertices } => {
                for (vx, vy) in vertices.iter_mut() {
                    *vx += dx;
                    *vy += dy;
                }
            }
        }
    }

    /// Apply a handle drag operation given the original shape, handle, and current position.
    /// Returns a new shape with the modification applied.
    pub fn apply_handle_drag(
        original: &AnnotationShape,
        handle: &AnnotationHandle,
        start_x: f32,
        start_y: f32,
        current_x: f32,
        current_y: f32,
    ) -> Option<AnnotationShape> {
        let dx = current_x - start_x;
        let dy = current_y - start_y;

        match (original, handle) {
            (
                AnnotationShape::BoundingBox {
                    x,
                    y,
                    width,
                    height,
                },
                AnnotationHandle::BBox(bbox_handle),
            ) => {
                let (new_x, new_y, new_w, new_h) = match bbox_handle {
                    BBoxHandle::TopLeft => (x + dx, y + dy, width - dx, height - dy),
                    BBoxHandle::TopRight => (*x, y + dy, width + dx, height - dy),
                    BBoxHandle::BottomLeft => (x + dx, *y, width - dx, height + dy),
                    BBoxHandle::BottomRight => (*x, *y, width + dx, height + dy),
                    BBoxHandle::TopEdge => (*x, y + dy, *width, height - dy),
                    BBoxHandle::BottomEdge => (*x, *y, *width, height + dy),
                    BBoxHandle::LeftEdge => (x + dx, *y, width - dx, *height),
                    BBoxHandle::RightEdge => (*x, *y, width + dx, *height),
                    BBoxHandle::Center => (x + dx, y + dy, *width, *height),
                };

                // Normalize to ensure positive width/height
                let (final_x, final_w) = if new_w < 0.0 {
                    (new_x + new_w, -new_w)
                } else {
                    (new_x, new_w)
                };
                let (final_y, final_h) = if new_h < 0.0 {
                    (new_y + new_h, -new_h)
                } else {
                    (new_y, new_h)
                };

                // Enforce minimum size
                if final_w >= MIN_BBOX_SIZE && final_h >= MIN_BBOX_SIZE {
                    Some(AnnotationShape::BoundingBox {
                        x: final_x,
                        y: final_y,
                        width: final_w,
                        height: final_h,
                    })
                } else {
                    None
                }
            }
            (AnnotationShape::Point { x, y }, AnnotationHandle::Point) => {
                Some(AnnotationShape::Point {
                    x: x + dx,
                    y: y + dy,
                })
            }
            (AnnotationShape::Polygon { vertices }, AnnotationHandle::Polygon(poly_handle)) => {
                let mut new_vertices = vertices.clone();
                match poly_handle {
                    PolygonHandle::Vertex(idx) => {
                        if *idx < new_vertices.len() {
                            new_vertices[*idx].0 += dx;
                            new_vertices[*idx].1 += dy;
                        }
                    }
                    PolygonHandle::Center => {
                        for (vx, vy) in new_vertices.iter_mut() {
                            *vx += dx;
                            *vy += dy;
                        }
                    }
                }
                Some(AnnotationShape::Polygon {
                    vertices: new_vertices,
                })
            }
            _ => None, // Mismatched shape and handle types
        }
    }

    /// Get the bounding box of this shape (min_x, min_y, max_x, max_y).
    pub fn bounding_box(&self) -> (f32, f32, f32, f32) {
        match self {
            AnnotationShape::BoundingBox {
                x,
                y,
                width,
                height,
            } => (*x, *y, x + width, y + height),
            AnnotationShape::Point { x, y } => (*x, *y, *x, *y),
            AnnotationShape::Polygon { vertices } => {
                if vertices.is_empty() {
                    return (0.0, 0.0, 0.0, 0.0);
                }
                let mut min_x = f32::MAX;
                let mut min_y = f32::MAX;
                let mut max_x = f32::MIN;
                let mut max_y = f32::MIN;
                for (vx, vy) in vertices {
                    min_x = min_x.min(*vx);
                    min_y = min_y.min(*vy);
                    max_x = max_x.max(*vx);
                    max_y = max_y.max(*vy);
                }
                (min_x, min_y, max_x, max_y)
            }
        }
    }
}

/// Calculate distance between two points.
fn point_distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    (dx * dx + dy * dy).sqrt()
}

/// Calculate the centroid of a polygon.
fn polygon_centroid(vertices: &[(f32, f32)]) -> Option<(f32, f32)> {
    if vertices.is_empty() {
        return None;
    }
    let sum_x: f32 = vertices.iter().map(|(x, _)| x).sum();
    let sum_y: f32 = vertices.iter().map(|(_, y)| y).sum();
    let n = vertices.len() as f32;
    Some((sum_x / n, sum_y / n))
}

/// A completed annotation with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    /// Unique identifier.
    pub id: AnnotationId,
    /// The shape geometry.
    pub shape: AnnotationShape,
    /// Category ID this annotation belongs to.
    pub category_id: u32,
    /// Whether this annotation is currently selected.
    pub selected: bool,
}

impl Annotation {
    /// Create a new annotation with the given shape and category.
    pub fn new(id: AnnotationId, shape: AnnotationShape, category_id: u32) -> Self {
        Self {
            id,
            shape,
            category_id,
            selected: false,
        }
    }
}

/// State for an annotation currently being drawn.
#[derive(Debug, Clone)]
pub enum DrawingState {
    /// Not currently drawing anything.
    Idle,
    /// Drawing a bounding box - stores the starting corner.
    BoundingBox {
        start_x: f32,
        start_y: f32,
        current_x: f32,
        current_y: f32,
    },
    /// Drawing a polygon - stores vertices added so far.
    Polygon { vertices: Vec<(f32, f32)> },
}

impl Default for DrawingState {
    fn default() -> Self {
        DrawingState::Idle
    }
}

impl DrawingState {
    /// Check if we're currently drawing something.
    pub fn is_drawing(&self) -> bool {
        !matches!(self, DrawingState::Idle)
    }

    /// Convert to an AnnotationShape if the drawing is complete enough.
    pub fn to_shape(&self) -> Option<AnnotationShape> {
        match self {
            DrawingState::Idle => None,
            DrawingState::BoundingBox {
                start_x,
                start_y,
                current_x,
                current_y,
            } => AnnotationShape::bounding_box_from_corners(
                *start_x, *start_y, *current_x, *current_y,
            ),
            DrawingState::Polygon { vertices } => {
                if vertices.len() >= MIN_POLYGON_VERTICES {
                    Some(AnnotationShape::Polygon {
                        vertices: vertices.clone(),
                    })
                } else {
                    None
                }
            }
        }
    }
}
