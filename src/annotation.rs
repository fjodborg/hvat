//! Annotation data model and management.
//!
//! This module provides the core types for image annotations, including:
//! - Shape types (bounding boxes, polygons, points)
//! - Annotation labels and metadata
//! - Annotation storage and serialization

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::color_utils::hsv_to_rgb;
use crate::ui_constants::annotation as ann_const;

// ============================================================================
// Core Geometry Types
// ============================================================================

/// A 2D point in image coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Calculate distance to another point.
    pub fn distance_to(&self, other: &Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// An axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Top-left corner X coordinate
    pub x: f32,
    /// Top-left corner Y coordinate
    pub y: f32,
    /// Width of the box
    pub width: f32,
    /// Height of the box
    pub height: f32,
}

impl BoundingBox {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Create a bounding box from two corner points.
    pub fn from_corners(p1: Point, p2: Point) -> Self {
        let x = p1.x.min(p2.x);
        let y = p1.y.min(p2.y);
        let width = (p1.x - p2.x).abs();
        let height = (p1.y - p2.y).abs();
        Self { x, y, width, height }
    }

    /// Get the center point of the box.
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Check if a point is inside the box.
    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    /// Get the area of the box.
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    /// Get the top-left corner.
    pub fn top_left(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Get the bottom-right corner.
    pub fn bottom_right(&self) -> Point {
        Point::new(self.x + self.width, self.y + self.height)
    }
}

/// A polygon defined by a sequence of vertices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polygon {
    /// The vertices of the polygon in order.
    pub vertices: Vec<Point>,
    /// Whether the polygon is closed (last vertex connects to first).
    pub closed: bool,
}

impl Polygon {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            closed: false,
        }
    }

    /// Add a vertex to the polygon.
    pub fn push(&mut self, point: Point) {
        self.vertices.push(point);
    }

    /// Close the polygon.
    pub fn close(&mut self) {
        self.closed = true;
    }

    /// Check if the polygon is valid (has at least 3 vertices for a closed polygon).
    pub fn is_valid(&self) -> bool {
        if self.closed {
            self.vertices.len() >= 3
        } else {
            self.vertices.len() >= 2
        }
    }

    /// Get the bounding box of the polygon.
    pub fn bounding_box(&self) -> Option<BoundingBox> {
        if self.vertices.is_empty() {
            return None;
        }

        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for p in &self.vertices {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }

        Some(BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y))
    }

    /// Check if a point is inside the polygon (ray casting algorithm).
    pub fn contains(&self, point: &Point) -> bool {
        if !self.closed || self.vertices.len() < 3 {
            return false;
        }

        let mut inside = false;
        let n = self.vertices.len();

        let mut j = n - 1;
        for i in 0..n {
            let vi = &self.vertices[i];
            let vj = &self.vertices[j];

            if ((vi.y > point.y) != (vj.y > point.y))
                && (point.x < (vj.x - vi.x) * (point.y - vi.y) / (vj.y - vi.y) + vi.x)
            {
                inside = !inside;
            }
            j = i;
        }

        inside
    }
}

impl Default for Polygon {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Annotation Shape Types
// ============================================================================

/// The shape type of an annotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Shape {
    /// A single point marker.
    Point(Point),
    /// An axis-aligned bounding box.
    BoundingBox(BoundingBox),
    /// A polygon (closed or open polyline).
    Polygon(Polygon),
}

impl Shape {
    /// Get the bounding box of this shape.
    pub fn bounding_box(&self) -> Option<BoundingBox> {
        match self {
            Shape::Point(p) => Some(BoundingBox::new(p.x, p.y, 0.0, 0.0)),
            Shape::BoundingBox(b) => Some(*b),
            Shape::Polygon(poly) => poly.bounding_box(),
        }
    }

    /// Check if a point is inside/on this shape.
    pub fn contains(&self, point: &Point) -> bool {
        match self {
            Shape::Point(p) => p.distance_to(point) < 5.0, // Small hit radius
            Shape::BoundingBox(b) => b.contains(point),
            Shape::Polygon(poly) => poly.contains(point),
        }
    }

    /// Hit test for drag handles.
    /// Returns the handle type if near a handle, None otherwise.
    pub fn hit_test_handle(&self, point: &Point, hit_radius: f32) -> Option<DragHandle> {
        match self {
            Shape::Point(p) => {
                if p.distance_to(point) < hit_radius {
                    Some(DragHandle::Point)
                } else {
                    None
                }
            }
            Shape::BoundingBox(b) => {
                // Check corners first (TL, TR, BR, BL)
                let corners = [
                    Point::new(b.x, b.y),                         // TL = 0
                    Point::new(b.x + b.width, b.y),               // TR = 1
                    Point::new(b.x + b.width, b.y + b.height),    // BR = 2
                    Point::new(b.x, b.y + b.height),              // BL = 3
                ];
                for (i, corner) in corners.iter().enumerate() {
                    if corner.distance_to(point) < hit_radius {
                        return Some(DragHandle::BoxCorner(i as u8));
                    }
                }
                // Check if inside the box (body drag)
                if b.contains(point) {
                    return Some(DragHandle::Body);
                }
                None
            }
            Shape::Polygon(poly) => {
                // Check vertices first
                for (i, vertex) in poly.vertices.iter().enumerate() {
                    if vertex.distance_to(point) < hit_radius {
                        return Some(DragHandle::PolygonVertex(i));
                    }
                }
                // Check if inside polygon (body drag)
                if poly.contains(point) {
                    return Some(DragHandle::Body);
                }
                None
            }
        }
    }

    /// Apply a drag delta to the shape based on the handle being dragged.
    /// Returns the modified shape.
    pub fn apply_drag(&self, handle: DragHandle, delta: Point) -> Shape {
        match (self, handle) {
            // Move entire shape
            (Shape::Point(p), DragHandle::Body | DragHandle::Point) => {
                Shape::Point(Point::new(p.x + delta.x, p.y + delta.y))
            }
            (Shape::BoundingBox(b), DragHandle::Body) => {
                Shape::BoundingBox(BoundingBox::new(
                    b.x + delta.x,
                    b.y + delta.y,
                    b.width,
                    b.height,
                ))
            }
            (Shape::Polygon(poly), DragHandle::Body) => {
                let mut new_poly = Polygon::new();
                for v in &poly.vertices {
                    new_poly.push(Point::new(v.x + delta.x, v.y + delta.y));
                }
                if poly.closed {
                    new_poly.close();
                }
                Shape::Polygon(new_poly)
            }
            // Resize bounding box corners
            (Shape::BoundingBox(b), DragHandle::BoxCorner(corner)) => {
                let (new_x, new_y, new_w, new_h) = match corner {
                    0 => { // TL: move x,y, adjust width/height
                        let new_x = b.x + delta.x;
                        let new_y = b.y + delta.y;
                        let new_w = (b.width - delta.x).max(1.0);
                        let new_h = (b.height - delta.y).max(1.0);
                        (new_x.min(b.x + b.width - 1.0), new_y.min(b.y + b.height - 1.0), new_w, new_h)
                    }
                    1 => { // TR: adjust width, move y, adjust height
                        let new_w = (b.width + delta.x).max(1.0);
                        let new_y = b.y + delta.y;
                        let new_h = (b.height - delta.y).max(1.0);
                        (b.x, new_y.min(b.y + b.height - 1.0), new_w, new_h)
                    }
                    2 => { // BR: adjust width and height
                        let new_w = (b.width + delta.x).max(1.0);
                        let new_h = (b.height + delta.y).max(1.0);
                        (b.x, b.y, new_w, new_h)
                    }
                    3 => { // BL: move x, adjust width and height
                        let new_x = b.x + delta.x;
                        let new_w = (b.width - delta.x).max(1.0);
                        let new_h = (b.height + delta.y).max(1.0);
                        (new_x.min(b.x + b.width - 1.0), b.y, new_w, new_h)
                    }
                    _ => (b.x, b.y, b.width, b.height),
                };
                Shape::BoundingBox(BoundingBox::new(new_x, new_y, new_w, new_h))
            }
            // Move polygon vertex
            (Shape::Polygon(poly), DragHandle::PolygonVertex(idx)) => {
                let mut new_poly = Polygon::new();
                for (i, v) in poly.vertices.iter().enumerate() {
                    if i == idx {
                        new_poly.push(Point::new(v.x + delta.x, v.y + delta.y));
                    } else {
                        new_poly.push(*v);
                    }
                }
                if poly.closed {
                    new_poly.close();
                }
                Shape::Polygon(new_poly)
            }
            // Default: return unchanged
            _ => self.clone(),
        }
    }

    /// Get the positions of all drag handles for this shape.
    /// Returns a vector of (handle_type, position) pairs.
    pub fn get_handles(&self) -> Vec<(DragHandle, Point)> {
        match self {
            Shape::Point(p) => vec![(DragHandle::Point, *p)],
            Shape::BoundingBox(b) => {
                vec![
                    (DragHandle::BoxCorner(0), Point::new(b.x, b.y)),
                    (DragHandle::BoxCorner(1), Point::new(b.x + b.width, b.y)),
                    (DragHandle::BoxCorner(2), Point::new(b.x + b.width, b.y + b.height)),
                    (DragHandle::BoxCorner(3), Point::new(b.x, b.y + b.height)),
                ]
            }
            Shape::Polygon(poly) => {
                poly.vertices
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (DragHandle::PolygonVertex(i), *v))
                    .collect()
            }
        }
    }
}

// ============================================================================
// Annotation Labels and Categories
// ============================================================================

/// A label category for annotations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    /// Unique identifier for the category.
    pub id: u32,
    /// Display name of the category.
    pub name: String,
    /// Color for rendering (RGBA).
    pub color: [f32; 4],
}

impl Category {
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        // Generate a default color based on the ID using golden angle for good distribution
        let hue = (id as f32 * ann_const::GOLDEN_ANGLE) % 360.0;
        let (r, g, b) = hsv_to_rgb(hue, 0.7, 0.9);
        Self {
            id,
            name: name.into(),
            color: [r, g, b, ann_const::DEFAULT_ALPHA],
        }
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

// ============================================================================
// Annotation
// ============================================================================

/// A single annotation on an image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    /// Unique identifier for this annotation.
    pub id: u64,
    /// The category/label of this annotation.
    pub category_id: u32,
    /// The shape of the annotation.
    pub shape: Shape,
    /// Optional metadata/attributes.
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl Annotation {
    /// Create a new annotation.
    pub fn new(id: u64, category_id: u32, shape: Shape) -> Self {
        Self {
            id,
            category_id,
            shape,
            attributes: HashMap::new(),
        }
    }

    /// Add an attribute to the annotation.
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

// ============================================================================
// Annotation Store
// ============================================================================

/// Storage for annotations on a single image.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnnotationStore {
    /// All annotations, keyed by their ID.
    annotations: HashMap<u64, Annotation>,
    /// Available categories.
    categories: HashMap<u32, Category>,
    /// Counter for generating unique annotation IDs.
    next_id: u64,
    /// Currently selected annotation ID.
    #[serde(skip)]
    selected_id: Option<u64>,
    /// Dirty flag - set when annotations or selection changes.
    /// Used to avoid rebuilding overlay every frame.
    #[serde(skip)]
    dirty: bool,
}

impl AnnotationStore {
    pub fn new() -> Self {
        let mut store = Self {
            annotations: HashMap::new(),
            categories: HashMap::new(),
            next_id: 1,
            selected_id: None,
            dirty: true, // Start dirty so first overlay build happens
        };
        // Add a default category
        store.add_category(Category::new(0, "Object"));
        store
    }

    /// Check if the store has been modified since last clear_dirty().
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear the dirty flag. Call after rebuilding the overlay.
    #[inline]
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Mark the store as dirty.
    #[inline]
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Add a category.
    pub fn add_category(&mut self, category: Category) {
        self.categories.insert(category.id, category);
        self.mark_dirty();
    }

    /// Get a category by ID.
    pub fn get_category(&self, id: u32) -> Option<&Category> {
        self.categories.get(&id)
    }

    /// Get all categories.
    pub fn categories(&self) -> impl Iterator<Item = &Category> {
        self.categories.values()
    }

    /// Add an annotation and return its ID.
    pub fn add(&mut self, category_id: u32, shape: Shape) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.annotations.insert(id, Annotation::new(id, category_id, shape));
        self.mark_dirty();
        id
    }

    /// Remove an annotation by ID.
    pub fn remove(&mut self, id: u64) -> Option<Annotation> {
        let removed = self.annotations.remove(&id);
        if removed.is_some() {
            self.mark_dirty();
        }
        if self.selected_id == Some(id) {
            self.selected_id = None;
        }
        removed
    }

    /// Get an annotation by ID.
    pub fn get(&self, id: u64) -> Option<&Annotation> {
        self.annotations.get(&id)
    }

    /// Get a mutable reference to an annotation by ID.
    pub fn get_mut(&mut self, id: u64) -> Option<&mut Annotation> {
        self.annotations.get_mut(&id)
    }

    /// Get all annotations.
    pub fn iter(&self) -> impl Iterator<Item = &Annotation> {
        self.annotations.values()
    }

    /// Get the number of annotations.
    pub fn len(&self) -> usize {
        self.annotations.len()
    }

    /// Check if there are no annotations.
    pub fn is_empty(&self) -> bool {
        self.annotations.is_empty()
    }

    /// Clear all annotations.
    pub fn clear(&mut self) {
        if !self.annotations.is_empty() {
            self.mark_dirty();
        }
        self.annotations.clear();
        self.selected_id = None;
    }

    /// Select an annotation.
    pub fn select(&mut self, id: Option<u64>) {
        if self.selected_id != id {
            self.selected_id = id;
            self.mark_dirty();
        }
    }

    /// Get the selected annotation ID.
    pub fn selected(&self) -> Option<u64> {
        self.selected_id
    }

    /// Find the annotation at a given point.
    pub fn hit_test(&self, point: &Point) -> Option<u64> {
        // Return the first annotation that contains the point
        // In a more sophisticated implementation, we'd consider z-order or area
        for ann in self.annotations.values() {
            if ann.shape.contains(point) {
                return Some(ann.id);
            }
        }
        None
    }

    /// Find all annotations at a given point (for cycling through overlapping annotations).
    /// Returns annotation IDs sorted by ID for consistent ordering.
    pub fn hit_test_all(&self, point: &Point) -> Vec<u64> {
        let mut hits: Vec<u64> = self.annotations.values()
            .filter(|ann| ann.shape.contains(point))
            .map(|ann| ann.id)
            .collect();
        hits.sort();
        hits
    }

    /// Find the next annotation to select when cycling through overlapping annotations.
    /// If current_id is Some, returns the next annotation after it in the hit list.
    /// If current_id is None or not in the list, returns the first annotation.
    pub fn cycle_selection(&self, point: &Point, current_id: Option<u64>) -> Option<u64> {
        let hits = self.hit_test_all(point);
        if hits.is_empty() {
            return None;
        }

        match current_id {
            Some(id) => {
                // Find current position and return next (wrapping around)
                if let Some(pos) = hits.iter().position(|&h| h == id) {
                    let next_pos = (pos + 1) % hits.len();
                    Some(hits[next_pos])
                } else {
                    // Current not in list, return first
                    Some(hits[0])
                }
            }
            None => Some(hits[0]),
        }
    }

    /// Hit test for drag handles on the selected annotation.
    /// Returns the handle type if a handle is hit, None otherwise.
    /// `hit_radius` is the distance in image coordinates to consider a hit.
    pub fn hit_test_handle(&self, point: &Point, hit_radius: f32) -> Option<DragHandle> {
        let ann_id = self.selected_id?;
        let ann = self.annotations.get(&ann_id)?;
        ann.shape.hit_test_handle(point, hit_radius)
    }

    /// Hit test for drag handles on ALL annotations (not just selected).
    /// Returns (annotation_id, handle_type) if a handle is hit.
    /// Checks handles before body, so corners are prioritized.
    /// `hit_radius` is the distance in image coordinates to consider a hit.
    pub fn hit_test_any_handle(&self, point: &Point, hit_radius: f32) -> Option<(u64, DragHandle)> {
        // First pass: check handles (corners/vertices) - these should have priority
        for ann in self.annotations.values() {
            if let Some(handle) = ann.shape.hit_test_handle(point, hit_radius) {
                // Only return non-Body handles in this pass (corners, vertices, points)
                if !matches!(handle, DragHandle::Body) {
                    return Some((ann.id, handle));
                }
            }
        }
        // Second pass: check bodies
        for ann in self.annotations.values() {
            if ann.shape.contains(point) {
                return Some((ann.id, DragHandle::Body));
            }
        }
        None
    }

    /// Update the shape of an annotation.
    pub fn update_shape(&mut self, id: u64, shape: Shape) {
        if let Some(ann) = self.annotations.get_mut(&id) {
            ann.shape = shape;
            self.mark_dirty();
        }
    }

    /// Update the category of an annotation.
    pub fn set_category(&mut self, id: u64, category_id: u32) {
        if let Some(ann) = self.annotations.get_mut(&id) {
            if ann.category_id != category_id {
                ann.category_id = category_id;
                self.mark_dirty();
            }
        }
    }

    // ========================================================================
    // Import/Export
    // ========================================================================

    /// Export annotations to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Export annotations to compact JSON string.
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Import annotations from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Export annotations to COCO-style JSON format.
    /// This format is commonly used for object detection datasets.
    pub fn to_coco_json(&self, image_id: u64, image_width: u32, image_height: u32) -> Result<String, serde_json::Error> {
        let coco = CocoExport {
            image_id,
            image_width,
            image_height,
            annotations: self.annotations.values().map(|ann| {
                let bbox = ann.shape.bounding_box();
                CocoAnnotation {
                    id: ann.id,
                    image_id,
                    category_id: ann.category_id,
                    bbox: bbox.map(|b| [b.x, b.y, b.width, b.height]),
                    segmentation: match &ann.shape {
                        Shape::Polygon(poly) => {
                            Some(vec![poly.vertices.iter()
                                .flat_map(|p| vec![p.x, p.y])
                                .collect()])
                        }
                        _ => None,
                    },
                    area: bbox.map(|b| b.area()).unwrap_or(0.0),
                    iscrowd: 0,
                }
            }).collect(),
            categories: self.categories.values().map(|cat| {
                CocoCategory {
                    id: cat.id,
                    name: cat.name.clone(),
                }
            }).collect(),
        };
        serde_json::to_string_pretty(&coco)
    }
}

/// COCO format export structures
#[derive(Serialize)]
struct CocoExport {
    image_id: u64,
    image_width: u32,
    image_height: u32,
    annotations: Vec<CocoAnnotation>,
    categories: Vec<CocoCategory>,
}

#[derive(Serialize)]
struct CocoAnnotation {
    id: u64,
    image_id: u64,
    category_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    bbox: Option<[f32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    segmentation: Option<Vec<Vec<f32>>>,
    area: f32,
    iscrowd: u8,
}

#[derive(Serialize)]
struct CocoCategory {
    id: u32,
    name: String,
}

// ============================================================================
// Annotation Tool State
// ============================================================================

/// The current annotation drawing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnnotationTool {
    /// No tool selected (selection mode).
    #[default]
    Select,
    /// Drawing bounding boxes.
    BoundingBox,
    /// Drawing polygons.
    Polygon,
    /// Placing point markers.
    Point,
}

/// Trait defining behavior and metadata for annotation tools.
///
/// This trait centralizes tool-specific information that was previously
/// scattered across multiple match statements in the codebase.
pub trait AnnotationToolBehavior {
    /// Get the icon name for caching purposes.
    fn icon_name(&self) -> &'static str;

    /// Get the tooltip text including keyboard shortcut.
    fn tooltip(&self) -> &'static str;

    /// Get the keyboard shortcut character.
    fn hotkey(&self) -> char;
}

impl AnnotationToolBehavior for AnnotationTool {
    fn icon_name(&self) -> &'static str {
        match self {
            Self::Select => "cursor",
            Self::BoundingBox => "bounding-box",
            Self::Polygon => "hexagon",
            Self::Point => "geo-alt",
        }
    }

    fn tooltip(&self) -> &'static str {
        match self {
            Self::Select => "Select and edit annotations (s)",
            Self::BoundingBox => "Draw bounding boxes (b)",
            Self::Polygon => "Draw polygon masks (m)",
            Self::Point => "Place point markers (p)",
        }
    }

    fn hotkey(&self) -> char {
        match self {
            Self::Select => 's',
            Self::BoundingBox => 'b',
            Self::Polygon => 'm',
            Self::Point => 'p',
        }
    }
}

/// Which part of an annotation is being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragHandle {
    /// Dragging the whole annotation (move)
    Body,
    /// Dragging a corner of a bounding box (0=TL, 1=TR, 2=BR, 3=BL)
    BoxCorner(u8),
    /// Dragging an edge of a bounding box (0=top, 1=right, 2=bottom, 3=left)
    BoxEdge(u8),
    /// Dragging a vertex of a polygon
    PolygonVertex(usize),
    /// Dragging a point annotation
    Point,
}

// ============================================================================
// Typestate Pattern: EditingPhase
// ============================================================================

/// Typestate enum representing the current editing phase.
///
/// This replaces the boolean flags in EditingState with explicit states,
/// making invalid state combinations impossible at compile time.
#[derive(Debug, Clone)]
pub enum EditingPhase {
    /// Not editing any annotation
    Idle,
    /// Dragging an annotation (mouse is down)
    Dragging {
        /// Which annotation is being edited
        annotation_id: u64,
        /// Which handle/part is being dragged
        handle: DragHandle,
        /// Starting point of the drag in image coordinates
        drag_start: Point,
        /// Original shape before editing (for undo/preview)
        original_shape: Shape,
        /// Whether the annotation was already selected when drag started
        was_already_selected: bool,
        /// Whether the mouse has actually moved since drag started
        has_moved: bool,
    },
}

impl Default for EditingPhase {
    fn default() -> Self {
        Self::Idle
    }
}

impl EditingPhase {
    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool {
        matches!(self, Self::Dragging { .. })
    }

    /// Get the annotation ID if dragging
    pub fn annotation_id(&self) -> Option<u64> {
        match self {
            Self::Dragging { annotation_id, .. } => Some(*annotation_id),
            Self::Idle => None,
        }
    }

    /// Get the drag handle if dragging
    pub fn handle(&self) -> Option<DragHandle> {
        match self {
            Self::Dragging { handle, .. } => Some(*handle),
            Self::Idle => None,
        }
    }

    /// Get the drag start point if dragging
    pub fn drag_start(&self) -> Option<Point> {
        match self {
            Self::Dragging { drag_start, .. } => Some(*drag_start),
            Self::Idle => None,
        }
    }

    /// Get reference to the original shape if dragging
    pub fn original_shape(&self) -> Option<&Shape> {
        match self {
            Self::Dragging { original_shape, .. } => Some(original_shape),
            Self::Idle => None,
        }
    }

    /// Check if the drag has moved
    pub fn has_moved(&self) -> bool {
        match self {
            Self::Dragging { has_moved, .. } => *has_moved,
            Self::Idle => false,
        }
    }

    /// Check if was already selected when drag started
    pub fn was_already_selected(&self) -> bool {
        match self {
            Self::Dragging { was_already_selected, .. } => *was_already_selected,
            Self::Idle => false,
        }
    }
}

/// State for editing an existing annotation.
#[derive(Debug, Clone, Default)]
pub struct EditingState {
    /// Current editing phase (typestate)
    phase: EditingPhase,
}

impl EditingState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool {
        self.phase.is_dragging()
    }

    /// Get the annotation ID if dragging
    pub fn annotation_id(&self) -> Option<u64> {
        self.phase.annotation_id()
    }

    /// Get the drag handle if dragging
    pub fn handle(&self) -> Option<DragHandle> {
        self.phase.handle()
    }

    /// Get the drag start point if dragging
    pub fn drag_start(&self) -> Option<Point> {
        self.phase.drag_start()
    }

    /// Get reference to the original shape if dragging
    pub fn original_shape(&self) -> Option<&Shape> {
        self.phase.original_shape()
    }

    /// Check if the drag has moved
    pub fn has_moved(&self) -> bool {
        self.phase.has_moved()
    }

    /// Check if was already selected when drag started
    pub fn was_already_selected(&self) -> bool {
        self.phase.was_already_selected()
    }

    /// Start dragging an annotation
    pub fn start_drag(&mut self, ann_id: u64, handle: DragHandle, point: Point, original: Shape, was_already_selected: bool) {
        self.phase = EditingPhase::Dragging {
            annotation_id: ann_id,
            handle,
            drag_start: point,
            original_shape: original,
            was_already_selected,
            has_moved: false,
        };
    }

    /// Mark that the drag has actually moved
    pub fn mark_moved(&mut self) {
        if let EditingPhase::Dragging { has_moved, .. } = &mut self.phase {
            *has_moved = true;
        }
    }

    /// Check if this was just a click (no movement)
    pub fn was_click(&self) -> bool {
        !self.has_moved()
    }

    /// Finish dragging
    pub fn finish_drag(&mut self) {
        self.phase = EditingPhase::Idle;
    }

    /// Cancel dragging (restore original)
    pub fn cancel_drag(&mut self) -> Option<(u64, Shape)> {
        let result = match &self.phase {
            EditingPhase::Dragging { annotation_id, original_shape, .. } => {
                Some((*annotation_id, original_shape.clone()))
            }
            EditingPhase::Idle => None,
        };
        self.phase = EditingPhase::Idle;
        result
    }
}

// ============================================================================
// Typestate Pattern: DrawingPhase
// ============================================================================

/// Typestate enum representing the current drawing phase.
///
/// This replaces the boolean `is_drawing` flag with explicit states.
#[derive(Debug, Clone)]
pub enum DrawingPhase {
    /// Not currently drawing
    Idle,
    /// Actively drawing a shape
    Drawing {
        /// Points collected during the current drawing operation
        points: Vec<Point>,
    },
}

impl Default for DrawingPhase {
    fn default() -> Self {
        Self::Idle
    }
}

/// State for the current drawing operation.
#[derive(Debug, Clone, Default)]
pub struct DrawingState {
    /// The active tool.
    pub tool: AnnotationTool,
    /// The category to assign to new annotations.
    pub current_category: u32,
    /// Current drawing phase (typestate)
    phase: DrawingPhase,
    /// Editing state for modifying existing annotations
    pub editing: EditingState,
}

// Legacy compatibility: expose fields that handlers expect
impl DrawingState {
    /// Check if currently drawing (legacy compatibility)
    pub fn is_drawing(&self) -> bool {
        matches!(self.phase, DrawingPhase::Drawing { .. })
    }

    /// Get points slice (legacy compatibility)
    pub fn points(&self) -> &[Point] {
        match &self.phase {
            DrawingPhase::Drawing { points } => points,
            DrawingPhase::Idle => &[],
        }
    }
}

impl DrawingState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new drawing operation.
    pub fn start(&mut self, point: Point) {
        self.phase = DrawingPhase::Drawing {
            points: vec![point],
        };
    }

    /// Add a point to the current drawing.
    pub fn add_point(&mut self, point: Point) {
        if let DrawingPhase::Drawing { points } = &mut self.phase {
            points.push(point);
        }
    }

    /// Update the last point (for dragging).
    pub fn update_last(&mut self, point: Point) {
        if let DrawingPhase::Drawing { points } = &mut self.phase {
            if let Some(last) = points.last_mut() {
                *last = point;
            }
        }
    }

    /// Finish the current drawing and return the shape.
    pub fn finish(&mut self) -> Option<Shape> {
        let DrawingPhase::Drawing { points } = &self.phase else {
            return None;
        };

        let shape = match self.tool {
            AnnotationTool::Select => None,
            AnnotationTool::Point => {
                points.first().map(|p| Shape::Point(*p))
            }
            AnnotationTool::BoundingBox => {
                if points.len() >= 2 {
                    let bbox = BoundingBox::from_corners(points[0], *points.last().unwrap());
                    if bbox.area() > 0.0 {
                        Some(Shape::BoundingBox(bbox))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            AnnotationTool::Polygon => {
                if points.len() >= 3 {
                    let mut poly = Polygon::new();
                    for p in points {
                        poly.push(*p);
                    }
                    poly.close();
                    Some(Shape::Polygon(poly))
                } else {
                    None
                }
            }
        };

        self.phase = DrawingPhase::Idle;
        shape
    }

    /// Cancel the current drawing.
    pub fn cancel(&mut self) {
        self.phase = DrawingPhase::Idle;
    }

    /// Get the preview shape for the current drawing.
    pub fn preview(&self) -> Option<Shape> {
        let DrawingPhase::Drawing { points } = &self.phase else {
            return None;
        };

        match self.tool {
            AnnotationTool::Select => None,
            AnnotationTool::Point => points.first().map(|p| Shape::Point(*p)),
            AnnotationTool::BoundingBox => {
                if points.len() >= 2 {
                    Some(Shape::BoundingBox(BoundingBox::from_corners(
                        points[0],
                        *points.last().unwrap(),
                    )))
                } else {
                    None
                }
            }
            AnnotationTool::Polygon => {
                if !points.is_empty() {
                    let mut poly = Polygon::new();
                    for p in points {
                        poly.push(*p);
                    }
                    // Don't close for preview
                    Some(Shape::Polygon(poly))
                } else {
                    None
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert!((p1.distance_to(&p2) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_bounding_box_from_corners() {
        let bbox = BoundingBox::from_corners(Point::new(10.0, 20.0), Point::new(50.0, 80.0));
        assert_eq!(bbox.x, 10.0);
        assert_eq!(bbox.y, 20.0);
        assert_eq!(bbox.width, 40.0);
        assert_eq!(bbox.height, 60.0);

        // Test with reversed corners
        let bbox2 = BoundingBox::from_corners(Point::new(50.0, 80.0), Point::new(10.0, 20.0));
        assert_eq!(bbox, bbox2);
    }

    #[test]
    fn test_bounding_box_contains() {
        let bbox = BoundingBox::new(10.0, 10.0, 100.0, 100.0);
        assert!(bbox.contains(&Point::new(50.0, 50.0)));
        assert!(bbox.contains(&Point::new(10.0, 10.0))); // Edge
        assert!(!bbox.contains(&Point::new(5.0, 50.0)));
    }

    #[test]
    fn test_polygon_contains() {
        // Create a square polygon
        let mut poly = Polygon::new();
        poly.push(Point::new(0.0, 0.0));
        poly.push(Point::new(100.0, 0.0));
        poly.push(Point::new(100.0, 100.0));
        poly.push(Point::new(0.0, 100.0));
        poly.close();

        assert!(poly.contains(&Point::new(50.0, 50.0)));
        assert!(!poly.contains(&Point::new(150.0, 50.0)));
    }

    #[test]
    fn test_annotation_store() {
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(1, "Car"));
        store.add_category(Category::new(2, "Person"));

        let id1 = store.add(1, Shape::BoundingBox(BoundingBox::new(10.0, 10.0, 50.0, 50.0)));
        let id2 = store.add(2, Shape::Point(Point::new(100.0, 100.0)));

        assert_eq!(store.len(), 2);
        assert!(store.get(id1).is_some());
        assert!(store.get(id2).is_some());

        store.remove(id1);
        assert_eq!(store.len(), 1);
        assert!(store.get(id1).is_none());
    }

    #[test]
    fn test_hit_test() {
        let mut store = AnnotationStore::new();
        let id = store.add(0, Shape::BoundingBox(BoundingBox::new(10.0, 10.0, 50.0, 50.0)));

        assert_eq!(store.hit_test(&Point::new(30.0, 30.0)), Some(id));
        assert_eq!(store.hit_test(&Point::new(100.0, 100.0)), None);
    }

    #[test]
    fn test_drawing_state_bbox() {
        let mut state = DrawingState::new();
        state.tool = AnnotationTool::BoundingBox;

        state.start(Point::new(10.0, 10.0));
        state.add_point(Point::new(50.0, 50.0));

        let shape = state.finish();
        assert!(shape.is_some());

        if let Some(Shape::BoundingBox(bbox)) = shape {
            assert_eq!(bbox.x, 10.0);
            assert_eq!(bbox.y, 10.0);
            assert_eq!(bbox.width, 40.0);
            assert_eq!(bbox.height, 40.0);
        } else {
            panic!("Expected BoundingBox shape");
        }
    }

    #[test]
    fn test_category_color_generation() {
        let c1 = Category::new(1, "A");
        let c2 = Category::new(2, "B");
        // Categories should have different colors
        assert_ne!(c1.color, c2.color);
    }

    #[test]
    fn test_json_export_import() {
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(1, "Car"));
        store.add(1, Shape::BoundingBox(BoundingBox::new(10.0, 20.0, 30.0, 40.0)));

        // Export to JSON
        let json = store.to_json().expect("Failed to export JSON");
        assert!(json.contains("\"Car\""));
        assert!(json.contains("BoundingBox"));

        // Import from JSON
        let imported = AnnotationStore::from_json(&json).expect("Failed to import JSON");
        assert_eq!(imported.len(), 1);
        assert!(imported.get_category(1).is_some());
    }

    #[test]
    fn test_coco_export() {
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(1, "Car"));
        store.add(1, Shape::BoundingBox(BoundingBox::new(10.0, 20.0, 100.0, 50.0)));

        let coco_json = store.to_coco_json(1, 640, 480).expect("Failed to export COCO");
        assert!(coco_json.contains("\"image_id\": 1"));
        assert!(coco_json.contains("\"category_id\": 1"));
        assert!(coco_json.contains("\"bbox\""));
        assert!(coco_json.contains("\"area\""));
    }
}
