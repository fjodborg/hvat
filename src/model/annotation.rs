//! Annotation tool types and data structures.

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

/// Annotation tools available in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
}

/// A completed annotation with metadata.
#[derive(Debug, Clone)]
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
