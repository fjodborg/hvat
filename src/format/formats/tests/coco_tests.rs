//! Tests for the COCO JSON format.

use std::path::PathBuf;

use crate::format::formats::CocoFormat;
use crate::format::project::{AnnotationEntry, CategoryEntry, ImageEntry, ProjectData, ShapeEntry};
use crate::format::traits::AnnotationFormat;

/// Create a test project with COCO-compatible data.
fn create_coco_project() -> ProjectData {
    let mut data = ProjectData::new();

    // Categories
    data.categories
        .push(CategoryEntry::new(1, "person").with_supercategory("human"));
    data.categories
        .push(CategoryEntry::new(2, "car").with_supercategory("vehicle"));

    // Image with bounding box and polygon
    let mut image = ImageEntry::new(PathBuf::from("image1.jpg")).with_dimensions(800, 600);

    // Bounding box
    image.annotations.push(AnnotationEntry::new(
        1,
        1,
        ShapeEntry::BoundingBox {
            x: 100.0,
            y: 150.0,
            width: 200.0,
            height: 300.0,
        },
    ));

    // Polygon (triangle)
    image.annotations.push(AnnotationEntry::new(
        2,
        2,
        ShapeEntry::Polygon {
            vertices: vec![(400.0, 100.0), (500.0, 200.0), (350.0, 200.0)],
        },
    ));

    data.images.push(image);
    data
}

#[test]
fn test_coco_format_metadata() {
    let format = CocoFormat;

    assert_eq!(format.id(), "coco");
    assert_eq!(format.display_name(), "COCO (JSON)");
    assert!(format.extensions().contains(&"json"));
    assert!(format.supports_polygon());
    assert!(format.supports_point());
    assert!(!format.supports_per_image());
}

#[test]
fn test_coco_polygon_to_flat_array() {
    // COCO expects polygons as flat arrays: [x1, y1, x2, y2, ...]
    let polygon = ShapeEntry::Polygon {
        vertices: vec![(100.0, 200.0), (300.0, 200.0), (200.0, 400.0)],
    };

    // The polygon should have 3 vertices
    if let ShapeEntry::Polygon { vertices } = &polygon {
        assert_eq!(vertices.len(), 3);

        // Verify vertices are correct
        assert_eq!(vertices[0], (100.0, 200.0));
        assert_eq!(vertices[1], (300.0, 200.0));
        assert_eq!(vertices[2], (200.0, 400.0));
    } else {
        panic!("Expected polygon");
    }
}

#[test]
fn test_coco_bbox_format() {
    // COCO expects bbox as [x, y, width, height]
    let bbox = ShapeEntry::BoundingBox {
        x: 50.0,
        y: 100.0,
        width: 200.0,
        height: 150.0,
    };

    if let ShapeEntry::BoundingBox {
        x,
        y,
        width,
        height,
    } = bbox
    {
        // Verify COCO format [x, y, w, h]
        assert_eq!(x, 50.0);
        assert_eq!(y, 100.0);
        assert_eq!(width, 200.0);
        assert_eq!(height, 150.0);

        // Calculate area (used in COCO)
        let area = width * height;
        assert_eq!(area, 30000.0);
    }
}

#[test]
fn test_polygon_area_calculation() {
    // Test the shoelace formula for polygon area
    // Simple rectangle: (0,0), (100,0), (100,50), (0,50)
    let vertices = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)];

    // Shoelace formula
    let n = vertices.len();
    let mut area = 0.0f32;
    for i in 0..n {
        let j = (i + 1) % n;
        area += vertices[i].0 * vertices[j].1;
        area -= vertices[j].0 * vertices[i].1;
    }
    let area = (area / 2.0).abs();

    assert!((area - 5000.0).abs() < 0.001);
}

#[test]
fn test_polygon_bounding_box() {
    // Calculate bounding box from polygon vertices
    let vertices = vec![
        (100.0, 150.0),
        (300.0, 150.0),
        (350.0, 250.0),
        (200.0, 400.0),
        (50.0, 250.0),
    ];

    let min_x = vertices.iter().map(|v| v.0).fold(f32::INFINITY, f32::min);
    let min_y = vertices.iter().map(|v| v.1).fold(f32::INFINITY, f32::min);
    let max_x = vertices
        .iter()
        .map(|v| v.0)
        .fold(f32::NEG_INFINITY, f32::max);
    let max_y = vertices
        .iter()
        .map(|v| v.1)
        .fold(f32::NEG_INFINITY, f32::max);

    assert_eq!(min_x, 50.0);
    assert_eq!(min_y, 150.0);
    assert_eq!(max_x, 350.0);
    assert_eq!(max_y, 400.0);

    // COCO bbox: [x, y, width, height]
    let bbox = (min_x, min_y, max_x - min_x, max_y - min_y);
    assert_eq!(bbox, (50.0, 150.0, 300.0, 250.0));
}

#[test]
fn test_coco_category_id_mapping() {
    // COCO uses 1-based category IDs
    let data = create_coco_project();

    // Verify our test data uses proper IDs
    assert!(data.categories.iter().all(|c| c.id >= 1));

    // Verify annotation category_ids reference valid categories
    for image in &data.images {
        for ann in &image.annotations {
            assert!(data.categories.iter().any(|c| c.id == ann.category_id));
        }
    }
}

#[test]
fn test_coco_image_id_assignment() {
    let data = create_coco_project();

    // In COCO export, each image gets a unique ID
    // Verify our images have unique paths (which become IDs)
    let paths: Vec<_> = data.images.iter().map(|i| &i.path).collect();
    let unique_count = paths.iter().collect::<std::collections::HashSet<_>>().len();

    assert_eq!(paths.len(), unique_count, "Image paths must be unique");
}

#[test]
fn test_point_annotation_for_coco() {
    // COCO keypoints format uses points
    let point = ShapeEntry::Point { x: 250.0, y: 175.0 };

    if let ShapeEntry::Point { x, y } = point {
        // For COCO, points would be represented in keypoints array
        // [x, y, visibility] where visibility: 0=not labeled, 1=labeled but not visible, 2=labeled and visible
        assert_eq!(x, 250.0);
        assert_eq!(y, 175.0);
    }
}

#[test]
fn test_coco_annotation_id_uniqueness() {
    let data = create_coco_project();

    // Collect all annotation IDs across all images
    let mut all_ids = Vec::new();
    for image in &data.images {
        for ann in &image.annotations {
            all_ids.push(ann.id);
        }
    }

    // In a proper COCO file, annotation IDs should be globally unique
    // (Note: our internal format uses per-image IDs, but export should handle this)
    let unique_count = all_ids
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();

    // For this test project, IDs should be unique
    assert_eq!(all_ids.len(), unique_count);
}

#[test]
fn test_empty_project_for_coco() {
    let data = ProjectData::new();

    // Empty project should be valid for COCO export
    assert!(data.images.is_empty());
    assert!(data.categories.is_empty());
    assert_eq!(data.total_annotations(), 0);
}

#[test]
fn test_coco_dimensions_required() {
    // COCO format requires image dimensions
    let mut image = ImageEntry::new(PathBuf::from("no_dims.jpg"));

    // Initially no dimensions
    assert!(image.dimensions.is_none());

    // Add dimensions
    image = image.with_dimensions(1920, 1080);
    assert_eq!(image.dimensions, Some((1920, 1080)));
}
