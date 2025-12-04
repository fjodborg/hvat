//! Common utilities for annotation format conversions.

use crate::{BoundingBox, Point, Polygon};

/// Metadata about a single image in a dataset.
#[derive(Debug, Clone)]
pub struct ImageInfo {
    /// The filename of the image (e.g., "image001.jpg").
    pub file_name: String,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Optional numeric ID (for formats that use numeric IDs like COCO).
    pub id: Option<u64>,
}

impl ImageInfo {
    /// Create a new ImageInfo with the given filename and dimensions.
    pub fn new(file_name: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            file_name: file_name.into(),
            width,
            height,
            id: None,
        }
    }

    /// Set the numeric ID for this image.
    pub fn with_id(mut self, id: u64) -> Self {
        self.id = Some(id);
        self
    }

    /// Get the base name (without extension) of the image file.
    pub fn base_name(&self) -> &str {
        self.file_name
            .rsplit_once('.')
            .map(|(base, _)| base)
            .unwrap_or(&self.file_name)
    }
}

// ============================================================================
// Coordinate Conversion Utilities
// ============================================================================

/// Convert absolute pixel coordinates to normalized [0, 1] coordinates.
pub fn normalize_point(p: &Point, width: u32, height: u32) -> (f32, f32) {
    (p.x / width as f32, p.y / height as f32)
}

/// Convert normalized [0, 1] coordinates to absolute pixel coordinates.
/// Coordinates are clamped to ensure non-negative values.
pub fn denormalize_point(x: f32, y: f32, width: u32, height: u32) -> Point {
    Point::new((x * width as f32).max(0.0), (y * height as f32).max(0.0))
}

/// Convert a bounding box to normalized YOLO format (x_center, y_center, width, height).
pub fn bbox_to_yolo(bbox: &BoundingBox, img_width: u32, img_height: u32) -> (f32, f32, f32, f32) {
    let x_center = (bbox.x + bbox.width / 2.0) / img_width as f32;
    let y_center = (bbox.y + bbox.height / 2.0) / img_height as f32;
    let w = bbox.width / img_width as f32;
    let h = bbox.height / img_height as f32;
    (x_center, y_center, w, h)
}

/// Convert normalized YOLO format to bounding box.
///
/// YOLO format stores center-based normalized coordinates:
/// - x_center, y_center: center of box as fraction of image size (0.0-1.0)
/// - w, h: width and height as fraction of image size (0.0-1.0)
///
/// This function converts to top-left corner format with absolute pixel coordinates.
/// Coordinates are clamped to ensure non-negative values (handles edge cases where
/// box center minus half-width would be negative).
pub fn yolo_to_bbox(
    x_center: f32,
    y_center: f32,
    w: f32,
    h: f32,
    img_width: u32,
    img_height: u32,
) -> BoundingBox {
    let width = w * img_width as f32;
    let height = h * img_height as f32;
    // Calculate top-left corner, clamping to 0 to avoid negative coordinates
    let x = (x_center * img_width as f32 - width / 2.0).max(0.0);
    let y = (y_center * img_height as f32 - height / 2.0).max(0.0);
    BoundingBox::new(x, y, width, height)
}

/// Convert a polygon to normalized coordinates.
pub fn normalize_polygon(poly: &Polygon, width: u32, height: u32) -> Vec<(f32, f32)> {
    poly.vertices
        .iter()
        .map(|p| normalize_point(p, width, height))
        .collect()
}

/// Convert normalized coordinates to a polygon.
pub fn denormalize_polygon(coords: &[(f32, f32)], width: u32, height: u32) -> Polygon {
    let mut poly = Polygon::new();
    for (x, y) in coords {
        poly.push(denormalize_point(*x, *y, width, height));
    }
    poly.close();
    poly
}

/// Convert a polygon to a flat list of coordinates [x1, y1, x2, y2, ...].
pub fn polygon_to_flat_coords(poly: &Polygon) -> Vec<f32> {
    poly.vertices.iter().flat_map(|p| [p.x, p.y]).collect()
}

/// Convert a flat list of coordinates to a polygon.
/// Coordinates are clamped to ensure non-negative values.
pub fn flat_coords_to_polygon(coords: &[f32]) -> Option<Polygon> {
    if coords.len() < 6 || coords.len() % 2 != 0 {
        return None;
    }

    let mut poly = Polygon::new();
    for chunk in coords.chunks(2) {
        // Clamp coordinates to ensure non-negative values
        poly.push(Point::new(chunk[0].max(0.0), chunk[1].max(0.0)));
    }
    poly.close();
    Some(poly)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_denormalize_point() {
        let p = Point::new(100.0, 200.0);
        let (nx, ny) = normalize_point(&p, 400, 400);
        assert!((nx - 0.25).abs() < 0.001);
        assert!((ny - 0.5).abs() < 0.001);

        let restored = denormalize_point(nx, ny, 400, 400);
        assert!((restored.x - p.x).abs() < 0.001);
        assert!((restored.y - p.y).abs() < 0.001);
    }

    #[test]
    fn test_bbox_to_yolo() {
        let bbox = BoundingBox::new(100.0, 100.0, 200.0, 100.0);
        let (x_center, y_center, w, h) = bbox_to_yolo(&bbox, 640, 480);

        // Center should be at (200, 150) -> normalized (0.3125, 0.3125)
        assert!((x_center - 0.3125).abs() < 0.001);
        assert!((y_center - 0.3125).abs() < 0.001);
        assert!((w - 0.3125).abs() < 0.001);
        assert!((h - 0.2083).abs() < 0.01);
    }

    #[test]
    fn test_yolo_to_bbox() {
        let bbox = yolo_to_bbox(0.5, 0.5, 0.25, 0.25, 640, 480);
        assert!((bbox.x - 240.0).abs() < 1.0);
        assert!((bbox.y - 180.0).abs() < 1.0);
        assert!((bbox.width - 160.0).abs() < 1.0);
        assert!((bbox.height - 120.0).abs() < 1.0);
    }

    #[test]
    fn test_image_info_base_name() {
        let info = ImageInfo::new("image001.jpg", 640, 480);
        assert_eq!(info.base_name(), "image001");

        let info2 = ImageInfo::new("complex.name.png", 100, 100);
        assert_eq!(info2.base_name(), "complex.name");
    }

    #[test]
    fn test_polygon_flat_coords() {
        let mut poly = Polygon::new();
        poly.push(Point::new(0.0, 0.0));
        poly.push(Point::new(100.0, 0.0));
        poly.push(Point::new(100.0, 100.0));
        poly.close();

        let flat = polygon_to_flat_coords(&poly);
        assert_eq!(flat, vec![0.0, 0.0, 100.0, 0.0, 100.0, 100.0]);

        let restored = flat_coords_to_polygon(&flat).unwrap();
        assert_eq!(restored.vertices.len(), 3);
        assert!(restored.closed);
    }
}
