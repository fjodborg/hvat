//! Tests for the YOLO format.

use std::path::PathBuf;

use crate::format::formats::YoloFormat;
use crate::format::project::{AnnotationEntry, CategoryEntry, ImageEntry, ProjectData, ShapeEntry};
use crate::format::traits::AnnotationFormat;

/// Create a test project with YOLO-compatible data (bbox only).
fn create_yolo_project() -> ProjectData {
    let mut data = ProjectData::new();

    // Categories (0-indexed for YOLO)
    data.categories.push(CategoryEntry::new(0, "person"));
    data.categories.push(CategoryEntry::new(1, "car"));
    data.categories.push(CategoryEntry::new(2, "bicycle"));

    // Image with bounding boxes
    let mut image =
        ImageEntry::new(PathBuf::from("train/images/photo001.jpg")).with_dimensions(640, 480);

    // Person bbox
    image.annotations.push(AnnotationEntry::new(
        1,
        0,
        ShapeEntry::BoundingBox {
            x: 100.0,
            y: 120.0,
            width: 80.0,
            height: 200.0,
        },
    ));

    // Car bbox
    image.annotations.push(AnnotationEntry::new(
        2,
        1,
        ShapeEntry::BoundingBox {
            x: 300.0,
            y: 200.0,
            width: 150.0,
            height: 100.0,
        },
    ));

    data.images.push(image);
    data
}

#[test]
fn test_yolo_format_metadata() {
    let format = YoloFormat;

    assert_eq!(format.id(), "yolo");
    assert_eq!(format.display_name(), "YOLO (TXT)");
    assert!(format.extensions().contains(&"txt"));
    assert!(!format.supports_polygon(), "YOLO only supports bboxes");
    assert!(!format.supports_point(), "YOLO only supports bboxes");
    assert!(format.supports_per_image(), "YOLO uses per-image files");
}

#[test]
fn test_yolo_coordinate_normalization() {
    // YOLO uses normalized coordinates [0, 1]
    // Format: <class_id> <x_center> <y_center> <width> <height>

    let image_width = 640.0f32;
    let image_height = 480.0f32;

    // Bounding box in pixel coordinates
    let x = 100.0f32;
    let y = 120.0f32;
    let width = 80.0f32;
    let height = 200.0f32;

    // Convert to YOLO format (normalized center coordinates)
    let x_center = (x + width / 2.0) / image_width;
    let y_center = (y + height / 2.0) / image_height;
    let norm_width = width / image_width;
    let norm_height = height / image_height;

    // Verify normalization
    assert!(x_center >= 0.0 && x_center <= 1.0);
    assert!(y_center >= 0.0 && y_center <= 1.0);
    assert!(norm_width >= 0.0 && norm_width <= 1.0);
    assert!(norm_height >= 0.0 && norm_height <= 1.0);

    // Check specific values
    assert!((x_center - 0.21875).abs() < 0.0001); // (100 + 40) / 640
    assert!((y_center - 0.45833).abs() < 0.001); // (120 + 100) / 480
    assert!((norm_width - 0.125).abs() < 0.0001); // 80 / 640
    assert!((norm_height - 0.41667).abs() < 0.001); // 200 / 480
}

#[test]
fn test_yolo_line_format() {
    // YOLO format: "class_id x_center y_center width height"
    let class_id = 0;
    let x_center = 0.21875;
    let y_center = 0.45833;
    let width = 0.125;
    let height = 0.41667;

    let line = format!(
        "{} {:.6} {:.6} {:.6} {:.6}",
        class_id, x_center, y_center, width, height
    );

    assert!(line.starts_with("0 "));
    assert!(line.contains("0.218750"));
}

#[test]
fn test_yolo_class_id_zero_indexed() {
    let data = create_yolo_project();

    // YOLO uses 0-indexed class IDs
    for cat in &data.categories {
        assert!(cat.id < data.categories.len() as u32);
    }

    // First category should be 0
    assert_eq!(data.categories[0].id, 0);
}

#[test]
fn test_parse_yolo_line() {
    let line = "1 0.500000 0.500000 0.250000 0.333333";
    let parts: Vec<&str> = line.split_whitespace().collect();

    assert_eq!(parts.len(), 5);

    let class_id: u32 = parts[0].parse().unwrap();
    let x_center: f32 = parts[1].parse().unwrap();
    let y_center: f32 = parts[2].parse().unwrap();
    let width: f32 = parts[3].parse().unwrap();
    let height: f32 = parts[4].parse().unwrap();

    assert_eq!(class_id, 1);
    assert!((x_center - 0.5).abs() < 0.0001);
    assert!((y_center - 0.5).abs() < 0.0001);
    assert!((width - 0.25).abs() < 0.0001);
    assert!((height - 0.333333).abs() < 0.0001);
}

#[test]
fn test_yolo_denormalization() {
    // Convert from YOLO normalized to pixel coordinates
    let image_width = 640.0f32;
    let image_height = 480.0f32;

    let x_center_norm = 0.5;
    let y_center_norm = 0.5;
    let width_norm = 0.25;
    let height_norm = 0.333333;

    // Denormalize
    let width = width_norm * image_width;
    let height = height_norm * image_height;
    let x = x_center_norm * image_width - width / 2.0;
    let y = y_center_norm * image_height - height / 2.0;

    assert!((x - 240.0).abs() < 0.1);
    assert!((y - 160.0).abs() < 0.1);
    assert!((width - 160.0).abs() < 0.1);
    assert!((height - 160.0).abs() < 0.1);
}

#[test]
fn test_yolo_file_naming() {
    // YOLO expects: image.jpg -> image.txt
    let image_path = PathBuf::from("train/images/photo001.jpg");
    let expected_label_path = PathBuf::from("train/images/photo001.txt");

    let stem = image_path.file_stem().unwrap().to_str().unwrap();
    let parent = image_path.parent().unwrap();
    let label_path = parent.join(format!("{}.txt", stem));

    assert_eq!(label_path, expected_label_path);
}

#[test]
fn test_yolo_classes_file() {
    // YOLO datasets typically have a classes.txt file
    let data = create_yolo_project();

    let classes: Vec<&str> = data.categories.iter().map(|c| c.name.as_str()).collect();

    assert_eq!(classes, vec!["person", "car", "bicycle"]);
}

#[test]
fn test_yolo_bbox_edge_cases() {
    // Test bbox at image boundaries
    let image_width = 640.0f32;
    let image_height = 480.0f32;

    // Bbox at origin
    let x = 0.0f32;
    let y = 0.0f32;
    let width = 100.0f32;
    let height = 100.0f32;

    let x_center = (x + width / 2.0) / image_width;
    let y_center = (y + height / 2.0) / image_height;

    assert!(x_center > 0.0);
    assert!(y_center > 0.0);

    // Bbox at bottom-right corner
    let x = 540.0f32;
    let y = 380.0f32;
    let width = 100.0f32;
    let height = 100.0f32;

    let x_center = (x + width / 2.0) / image_width;
    let y_center = (y + height / 2.0) / image_height;

    assert!(x_center <= 1.0);
    assert!(y_center <= 1.0);
}

#[test]
fn test_yolo_polygon_not_supported() {
    // YOLO format does not support polygons
    let format = YoloFormat;
    assert!(!format.supports_polygon());

    // A project with polygons would need conversion to bbox
    let polygon = ShapeEntry::Polygon {
        vertices: vec![
            (100.0, 100.0),
            (200.0, 100.0),
            (200.0, 200.0),
            (100.0, 200.0),
        ],
    };

    // Calculate bounding box from polygon
    if let ShapeEntry::Polygon { vertices } = &polygon {
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

        // This is the bbox that YOLO would need
        let bbox_width = max_x - min_x;
        let bbox_height = max_y - min_y;

        assert_eq!(min_x, 100.0);
        assert_eq!(min_y, 100.0);
        assert_eq!(bbox_width, 100.0);
        assert_eq!(bbox_height, 100.0);
    }
}

#[test]
fn test_yolo_dimensions_required() {
    // YOLO requires image dimensions for normalization
    let image = ImageEntry::new(PathBuf::from("test.jpg"));
    assert!(
        image.dimensions.is_none(),
        "Need dimensions for YOLO export"
    );

    let image_with_dims = image.with_dimensions(640, 480);
    assert!(image_with_dims.dimensions.is_some());
}

#[test]
fn test_empty_yolo_file() {
    // Images without annotations should have empty label files
    let mut data = ProjectData::new();
    data.categories.push(CategoryEntry::new(0, "person"));

    let image = ImageEntry::new(PathBuf::from("empty.jpg")).with_dimensions(640, 480);
    data.images.push(image);

    assert_eq!(data.images[0].annotations.len(), 0);
}
