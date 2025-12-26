//! Tests for the native HVAT JSON format.

use std::path::PathBuf;

use crate::format::project::{
    AnnotationEntry, CategoryEntry, ImageEntry, ProjectData, ProjectMetadata, ShapeEntry, TagEntry,
};

/// Create a minimal test project with basic data.
fn create_minimal_project() -> ProjectData {
    let mut data = ProjectData::new();
    data.categories.push(CategoryEntry::new(1, "person"));
    data.images.push(ImageEntry::new(PathBuf::from("test.png")));
    data
}

/// Create a comprehensive test project with all features.
fn create_full_project() -> ProjectData {
    let mut data = ProjectData::new();

    // Categories with colors
    data.categories
        .push(CategoryEntry::new(1, "person").with_color([255, 0, 0]));
    data.categories
        .push(CategoryEntry::new(2, "car").with_color([0, 255, 0]));
    data.categories.push(
        CategoryEntry::new(3, "building")
            .with_color([0, 0, 255])
            .with_supercategory("structure"),
    );

    // Tags
    data.tags = vec![
        TagEntry::new(1, "reviewed").with_color([100, 200, 100]),
        TagEntry::new(2, "difficult").with_color([200, 100, 100]),
        TagEntry::new(3, "needs-check").with_color([100, 100, 200]),
    ];

    // Image with various annotation types
    let mut image1 =
        ImageEntry::new(PathBuf::from("images/photo1.jpg")).with_dimensions(1920, 1080);

    image1.tag_ids.insert(1); // "reviewed"

    // Bounding box annotation
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

    // Point annotation
    image1.annotations.push(AnnotationEntry::new(
        2,
        2,
        ShapeEntry::Point { x: 500.0, y: 400.0 },
    ));

    // Polygon annotation
    image1.annotations.push(AnnotationEntry::new(
        3,
        3,
        ShapeEntry::Polygon {
            vertices: vec![
                (100.0, 100.0),
                (200.0, 100.0),
                (200.0, 200.0),
                (150.0, 250.0),
                (100.0, 200.0),
            ],
        },
    ));

    data.images.push(image1);

    // Second image without annotations
    let mut image2 =
        ImageEntry::new(PathBuf::from("images/photo2.jpg")).with_dimensions(3840, 2160);
    image2.tag_ids.insert(2); // "difficult"
    data.images.push(image2);

    data.metadata = ProjectMetadata::new();
    data
}

#[test]
fn test_project_data_new() {
    let data = ProjectData::new();

    assert_eq!(data.version, ProjectData::CURRENT_VERSION);
    assert!(data.images.is_empty());
    assert!(data.categories.is_empty());
    assert!(data.tags.is_empty());
}

#[test]
fn test_project_data_total_annotations() {
    let mut data = ProjectData::new();
    assert_eq!(data.total_annotations(), 0);

    let mut image = ImageEntry::new(PathBuf::from("test.png"));
    image.annotations.push(AnnotationEntry::new(
        1,
        1,
        ShapeEntry::Point { x: 0.0, y: 0.0 },
    ));
    image.annotations.push(AnnotationEntry::new(
        2,
        1,
        ShapeEntry::Point { x: 1.0, y: 1.0 },
    ));
    data.images.push(image);

    assert_eq!(data.total_annotations(), 2);

    let mut image2 = ImageEntry::new(PathBuf::from("test2.png"));
    image2.annotations.push(AnnotationEntry::new(
        1,
        1,
        ShapeEntry::Point { x: 2.0, y: 2.0 },
    ));
    data.images.push(image2);

    assert_eq!(data.total_annotations(), 3);
}

#[test]
fn test_project_data_has_annotations() {
    let mut data = ProjectData::new();
    assert!(!data.has_annotations());

    data.images.push(ImageEntry::new(PathBuf::from("test.png")));
    assert!(!data.has_annotations());

    data.images[0].annotations.push(AnnotationEntry::new(
        1,
        1,
        ShapeEntry::Point { x: 0.0, y: 0.0 },
    ));
    assert!(data.has_annotations());
}

#[test]
fn test_minimal_serialization() {
    let original = create_minimal_project();
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    let loaded: ProjectData = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(original.version, loaded.version);
    assert_eq!(original.categories.len(), loaded.categories.len());
    assert_eq!(original.images.len(), loaded.images.len());
}

#[test]
fn test_full_serialization() {
    let original = create_full_project();
    let json = serde_json::to_string_pretty(&original).expect("Failed to serialize");
    let loaded: ProjectData = serde_json::from_str(&json).expect("Failed to deserialize");

    // Verify categories
    assert_eq!(original.categories.len(), loaded.categories.len());
    for (orig, load) in original.categories.iter().zip(loaded.categories.iter()) {
        assert_eq!(orig.id, load.id);
        assert_eq!(orig.name, load.name);
        assert_eq!(orig.color, load.color);
        assert_eq!(orig.supercategory, load.supercategory);
    }

    // Verify tags
    assert_eq!(original.tags.len(), loaded.tags.len());
    for (orig, load) in original.tags.iter().zip(loaded.tags.iter()) {
        assert_eq!(orig.id, load.id);
        assert_eq!(orig.name, load.name);
        assert_eq!(orig.color, load.color);
    }

    // Verify images
    assert_eq!(original.images.len(), loaded.images.len());
    for (orig, load) in original.images.iter().zip(loaded.images.iter()) {
        assert_eq!(orig.path, load.path);
        assert_eq!(orig.dimensions, load.dimensions);
        assert_eq!(orig.tag_ids, load.tag_ids);
        assert_eq!(orig.annotations.len(), load.annotations.len());
    }
}

#[test]
fn test_shape_entry_bbox() {
    let shape = ShapeEntry::BoundingBox {
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 50.0,
    };

    assert!(shape.is_bbox());
    assert!(!shape.is_point());
    assert!(!shape.is_polygon());
    assert_eq!(shape.shape_type(), "bbox");

    let json = serde_json::to_string(&shape).expect("Failed to serialize");
    assert!(json.contains("\"type\":\"bbox\""));
}

#[test]
fn test_shape_entry_point() {
    let shape = ShapeEntry::Point { x: 100.0, y: 200.0 };

    assert!(!shape.is_bbox());
    assert!(shape.is_point());
    assert!(!shape.is_polygon());
    assert_eq!(shape.shape_type(), "point");

    let json = serde_json::to_string(&shape).expect("Failed to serialize");
    assert!(json.contains("\"type\":\"point\""));
}

#[test]
fn test_shape_entry_polygon() {
    let shape = ShapeEntry::Polygon {
        vertices: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
    };

    assert!(!shape.is_bbox());
    assert!(!shape.is_point());
    assert!(shape.is_polygon());
    assert_eq!(shape.shape_type(), "polygon");

    let json = serde_json::to_string(&shape).expect("Failed to serialize");
    assert!(json.contains("\"type\":\"polygon\""));
}

#[test]
fn test_category_entry_builders() {
    let cat = CategoryEntry::new(1, "test")
        .with_color([128, 64, 32])
        .with_supercategory("parent");

    assert_eq!(cat.id, 1);
    assert_eq!(cat.name, "test");
    assert_eq!(cat.color, Some([128, 64, 32]));
    assert_eq!(cat.supercategory, Some("parent".to_string()));
}

#[test]
fn test_image_entry_builders() {
    let image = ImageEntry::new(PathBuf::from("test.jpg")).with_dimensions(1920, 1080);

    assert_eq!(image.path, PathBuf::from("test.jpg"));
    assert_eq!(image.filename, "test.jpg");
    assert_eq!(image.dimensions, Some((1920, 1080)));
    assert!(image.annotations.is_empty());
    assert!(image.tag_ids.is_empty());
}

#[test]
fn test_version_in_json() {
    let data = ProjectData::new();
    let json = serde_json::to_string(&data).expect("Failed to serialize");

    assert!(json.contains(&format!("\"version\":\"{}\"", ProjectData::CURRENT_VERSION)));
}

#[test]
fn test_empty_optional_fields_not_serialized() {
    let data = ProjectData::new();
    let json = serde_json::to_string(&data).expect("Failed to serialize");

    // Empty metadata fields should be skipped
    let loaded: serde_json::Value = serde_json::from_str(&json).unwrap();
    let metadata = &loaded["metadata"];

    // created_by should be present (set by ProjectMetadata::new)
    // but extra should be empty and thus skipped
    if let Some(extra) = metadata.get("extra") {
        assert!(extra.as_object().map(|o| o.is_empty()).unwrap_or(true));
    }
}

#[test]
fn test_deserialize_with_missing_optional_fields() {
    // Minimal valid JSON without optional fields
    let json = r#"{
        "version": "0.1.0",
        "images": [],
        "categories": []
    }"#;

    let data: ProjectData = serde_json::from_str(json).expect("Failed to deserialize");

    assert_eq!(data.version, "0.1.0");
    assert!(data.tags.is_empty());
    assert!(data.folder.as_os_str().is_empty());
}

#[test]
fn test_annotation_entry_conversion() {
    let entry = AnnotationEntry::new(
        42,
        5,
        ShapeEntry::BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        },
    );

    let annotation = entry.to_annotation();

    assert_eq!(annotation.id, 42);
    assert_eq!(annotation.category_id, 5);

    // Convert back
    let entry2 = AnnotationEntry::from_annotation(&annotation);

    assert_eq!(entry.id, entry2.id);
    assert_eq!(entry.category_id, entry2.category_id);
}
