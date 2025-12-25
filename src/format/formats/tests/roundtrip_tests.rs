//! Round-trip conversion tests between formats.
//!
//! These tests verify that data can be converted between formats
//! without loss of information (where supported).

use std::path::PathBuf;

use crate::format::project::{
    AnnotationEntry, CategoryEntry, ImageEntry, ProjectData, ShapeEntry, TagEntry,
};

/// Create a project with all shape types for testing.
fn create_comprehensive_project() -> ProjectData {
    let mut data = ProjectData::new();

    // Categories
    data.categories
        .push(CategoryEntry::new(1, "person").with_color([255, 0, 0]));
    data.categories
        .push(CategoryEntry::new(2, "vehicle").with_color([0, 255, 0]));
    data.categories
        .push(CategoryEntry::new(3, "building").with_color([0, 0, 255]));

    // Tags
    data.tags = vec![
        TagEntry::new(1, "verified").with_color([100, 200, 100]),
        TagEntry::new(2, "needs-review").with_color([200, 200, 100]),
    ];

    // Image with all annotation types
    let mut image1 =
        ImageEntry::new(PathBuf::from("images/scene1.jpg")).with_dimensions(1920, 1080);
    image1.tag_ids.insert(1); // "verified"

    // Bounding boxes
    image1.annotations.push(AnnotationEntry::new(
        1,
        1,
        ShapeEntry::BoundingBox {
            x: 100.0,
            y: 200.0,
            width: 150.0,
            height: 300.0,
        },
    ));

    image1.annotations.push(AnnotationEntry::new(
        2,
        2,
        ShapeEntry::BoundingBox {
            x: 500.0,
            y: 100.0,
            width: 400.0,
            height: 200.0,
        },
    ));

    // Points
    image1.annotations.push(AnnotationEntry::new(
        3,
        1,
        ShapeEntry::Point { x: 960.0, y: 540.0 },
    ));

    // Polygons
    image1.annotations.push(AnnotationEntry::new(
        4,
        3,
        ShapeEntry::Polygon {
            vertices: vec![
                (800.0, 600.0),
                (1000.0, 600.0),
                (1100.0, 800.0),
                (900.0, 900.0),
                (700.0, 800.0),
            ],
        },
    ));

    data.images.push(image1);

    // Second image with only bounding boxes
    let mut image2 =
        ImageEntry::new(PathBuf::from("images/scene2.jpg")).with_dimensions(3840, 2160);

    image2.annotations.push(AnnotationEntry::new(
        1,
        2,
        ShapeEntry::BoundingBox {
            x: 1000.0,
            y: 500.0,
            width: 600.0,
            height: 400.0,
        },
    ));

    data.images.push(image2);

    data
}

#[test]
fn test_hvat_json_roundtrip() {
    let original = create_comprehensive_project();

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&original).expect("Failed to serialize");

    // Deserialize back
    let loaded: ProjectData = serde_json::from_str(&json).expect("Failed to deserialize");

    // Verify all data preserved
    assert_eq!(original.version, loaded.version);
    assert_eq!(original.categories.len(), loaded.categories.len());
    assert_eq!(original.tags.len(), loaded.tags.len());
    assert_eq!(original.images.len(), loaded.images.len());
    assert_eq!(original.total_annotations(), loaded.total_annotations());
}

#[test]
fn test_bbox_coordinates_preserved() {
    let original_bbox = ShapeEntry::BoundingBox {
        x: 123.456,
        y: 789.012,
        width: 345.678,
        height: 901.234,
    };

    let json = serde_json::to_string(&original_bbox).unwrap();
    let loaded: ShapeEntry = serde_json::from_str(&json).unwrap();

    if let ShapeEntry::BoundingBox {
        x,
        y,
        width,
        height,
    } = loaded
    {
        assert!((x - 123.456).abs() < 0.001);
        assert!((y - 789.012).abs() < 0.001);
        assert!((width - 345.678).abs() < 0.001);
        assert!((height - 901.234).abs() < 0.001);
    } else {
        panic!("Expected BoundingBox");
    }
}

#[test]
fn test_point_coordinates_preserved() {
    let original_point = ShapeEntry::Point {
        x: 1234.5678,
        y: 9012.3456,
    };

    let json = serde_json::to_string(&original_point).unwrap();
    let loaded: ShapeEntry = serde_json::from_str(&json).unwrap();

    if let ShapeEntry::Point { x, y } = loaded {
        assert!((x - 1234.5678).abs() < 0.001);
        assert!((y - 9012.3456).abs() < 0.001);
    } else {
        panic!("Expected Point");
    }
}

#[test]
fn test_polygon_vertices_preserved() {
    let original_vertices = vec![
        (100.5, 200.5),
        (300.75, 200.25),
        (350.125, 400.875),
        (150.0625, 450.9375),
    ];

    let original_polygon = ShapeEntry::Polygon {
        vertices: original_vertices.clone(),
    };

    let json = serde_json::to_string(&original_polygon).unwrap();
    let loaded: ShapeEntry = serde_json::from_str(&json).unwrap();

    if let ShapeEntry::Polygon { vertices } = loaded {
        assert_eq!(vertices.len(), original_vertices.len());
        for (orig, load) in original_vertices.iter().zip(vertices.iter()) {
            assert!((orig.0 - load.0).abs() < 0.0001);
            assert!((orig.1 - load.1).abs() < 0.0001);
        }
    } else {
        panic!("Expected Polygon");
    }
}

#[test]
fn test_category_colors_preserved() {
    let original = CategoryEntry::new(42, "test-category").with_color([128, 64, 192]);

    let json = serde_json::to_string(&original).unwrap();
    let loaded: CategoryEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.id, 42);
    assert_eq!(loaded.name, "test-category");
    assert_eq!(loaded.color, Some([128, 64, 192]));
}

#[test]
fn test_image_tag_ids_preserved() {
    let mut original = ImageEntry::new(PathBuf::from("test.jpg"));
    original.tag_ids.insert(1);
    original.tag_ids.insert(2);
    original.tag_ids.insert(3);

    let json = serde_json::to_string(&original).unwrap();
    let loaded: ImageEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.tag_ids.len(), 3);
    assert!(loaded.tag_ids.contains(&1));
    assert!(loaded.tag_ids.contains(&2));
    assert!(loaded.tag_ids.contains(&3));
}

#[test]
fn test_annotation_id_preserved() {
    let original = AnnotationEntry::new(12345, 67, ShapeEntry::Point { x: 100.0, y: 200.0 });

    let json = serde_json::to_string(&original).unwrap();
    let loaded: AnnotationEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.id, 12345);
    assert_eq!(loaded.category_id, 67);
}

#[test]
fn test_empty_project_roundtrip() {
    let original = ProjectData::new();

    let json = serde_json::to_string(&original).unwrap();
    let loaded: ProjectData = serde_json::from_str(&json).unwrap();

    assert_eq!(original.version, loaded.version);
    assert!(loaded.images.is_empty());
    assert!(loaded.categories.is_empty());
    assert!(loaded.tags.is_empty());
}

#[test]
fn test_large_coordinates_preserved() {
    // Test with coordinates typical of large satellite images
    let bbox = ShapeEntry::BoundingBox {
        x: 50000.0,
        y: 75000.0,
        width: 10000.0,
        height: 15000.0,
    };

    let json = serde_json::to_string(&bbox).unwrap();
    let loaded: ShapeEntry = serde_json::from_str(&json).unwrap();

    if let ShapeEntry::BoundingBox {
        x,
        y,
        width,
        height,
    } = loaded
    {
        assert_eq!(x, 50000.0);
        assert_eq!(y, 75000.0);
        assert_eq!(width, 10000.0);
        assert_eq!(height, 15000.0);
    }
}

#[test]
fn test_small_coordinates_preserved() {
    // Test with very small/precise coordinates
    let point = ShapeEntry::Point {
        x: 0.123456789,
        y: 0.987654321,
    };

    let json = serde_json::to_string(&point).unwrap();
    let loaded: ShapeEntry = serde_json::from_str(&json).unwrap();

    if let ShapeEntry::Point { x, y } = loaded {
        // f32 precision is about 6-7 significant digits
        assert!((x - 0.123456789).abs() < 0.0000001);
        assert!((y - 0.987654321).abs() < 0.0000001);
    }
}

#[test]
fn test_unicode_category_names_preserved() {
    let original = CategoryEntry::new(1, "人物"); // Chinese for "person"

    let json = serde_json::to_string(&original).unwrap();
    let loaded: CategoryEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.name, "人物");
}

#[test]
fn test_unicode_tag_names_preserved() {
    // Test unicode tag names in TagEntry
    let tags = vec![
        TagEntry::new(1, "已验证"), // Chinese for "verified"
        TagEntry::new(2, "élève"),  // French
        TagEntry::new(3, "日本語"), // Japanese
    ];

    let json = serde_json::to_string(&tags).unwrap();
    let loaded: Vec<TagEntry> = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded[0].name, "已验证");
    assert_eq!(loaded[1].name, "élève");
    assert_eq!(loaded[2].name, "日本語");
}

#[test]
fn test_path_with_special_characters() {
    let original = ImageEntry::new(PathBuf::from("data/2023-01-15/image (1).jpg"));

    let json = serde_json::to_string(&original).unwrap();
    let loaded: ImageEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.path, PathBuf::from("data/2023-01-15/image (1).jpg"));
}

#[test]
fn test_multiple_images_order_preserved() {
    let mut data = ProjectData::new();

    for i in 0..10 {
        data.images.push(ImageEntry::new(PathBuf::from(format!(
            "image_{:03}.jpg",
            i
        ))));
    }

    let json = serde_json::to_string(&data).unwrap();
    let loaded: ProjectData = serde_json::from_str(&json).unwrap();

    for (i, image) in loaded.images.iter().enumerate() {
        assert_eq!(image.path, PathBuf::from(format!("image_{:03}.jpg", i)));
    }
}

#[test]
fn test_annotation_order_preserved() {
    let mut image = ImageEntry::new(PathBuf::from("test.jpg"));

    for i in 0..5 {
        image.annotations.push(AnnotationEntry::new(
            i,
            1,
            ShapeEntry::Point {
                x: i as f32 * 100.0,
                y: i as f32 * 100.0,
            },
        ));
    }

    let json = serde_json::to_string(&image).unwrap();
    let loaded: ImageEntry = serde_json::from_str(&json).unwrap();

    for (i, ann) in loaded.annotations.iter().enumerate() {
        assert_eq!(ann.id, i as u32);
    }
}
