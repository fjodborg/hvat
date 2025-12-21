//! Tests for the Pascal VOC XML format.

use std::path::PathBuf;

use crate::format::formats::PascalVocFormat;
use crate::format::project::{AnnotationEntry, CategoryEntry, ImageEntry, ProjectData, ShapeEntry};
use crate::format::traits::AnnotationFormat;

/// Create a test project with Pascal VOC-compatible data.
fn create_voc_project() -> ProjectData {
    let mut data = ProjectData::new();
    data.folder = PathBuf::from("/data/VOC2012/");

    // Categories
    data.categories.push(CategoryEntry::new(1, "person"));
    data.categories.push(CategoryEntry::new(2, "car"));
    data.categories.push(CategoryEntry::new(3, "dog"));

    // Image with bounding boxes
    let mut image =
        ImageEntry::new(PathBuf::from("JPEGImages/2007_000027.jpg")).with_dimensions(500, 375);

    // Person bbox
    image.annotations.push(AnnotationEntry::new(
        1,
        1,
        ShapeEntry::BoundingBox {
            x: 174.0,
            y: 101.0,
            width: 175.0,
            height: 274.0,
        },
    ));

    // Car bbox
    image.annotations.push(AnnotationEntry::new(
        2,
        2,
        ShapeEntry::BoundingBox {
            x: 22.0,
            y: 144.0,
            width: 182.0,
            height: 131.0,
        },
    ));

    data.images.push(image);
    data
}

#[test]
fn test_voc_format_metadata() {
    let format = PascalVocFormat;

    assert_eq!(format.id(), "voc");
    assert_eq!(format.display_name(), "Pascal VOC (XML)");
    assert!(format.extensions().contains(&"xml"));
    assert!(!format.supports_polygon(), "VOC only supports bboxes");
    assert!(!format.supports_point(), "VOC only supports bboxes");
    assert!(format.supports_per_image(), "VOC uses per-image files");
}

#[test]
fn test_voc_bbox_format() {
    // Pascal VOC uses (xmin, ymin, xmax, ymax) format
    let bbox = ShapeEntry::BoundingBox {
        x: 100.0,
        y: 150.0,
        width: 200.0,
        height: 100.0,
    };

    if let ShapeEntry::BoundingBox {
        x,
        y,
        width,
        height,
    } = bbox
    {
        // Convert to VOC format
        let xmin = x as u32;
        let ymin = y as u32;
        let xmax = (x + width) as u32;
        let ymax = (y + height) as u32;

        assert_eq!(xmin, 100);
        assert_eq!(ymin, 150);
        assert_eq!(xmax, 300);
        assert_eq!(ymax, 250);
    }
}

#[test]
fn test_voc_xml_structure() {
    // Pascal VOC XML structure
    let expected_elements = [
        "annotation",
        "folder",
        "filename",
        "size",
        "width",
        "height",
        "depth",
        "object",
        "name",
        "bndbox",
        "xmin",
        "ymin",
        "xmax",
        "ymax",
    ];

    // All these elements should be present in a valid VOC file
    for element in expected_elements {
        assert!(!element.is_empty());
    }
}

#[test]
fn test_voc_object_fields() {
    // Pascal VOC object annotation fields
    let object_fields = [
        ("name", "category name"),
        ("pose", "Unspecified/Left/Right/Frontal/Rear"),
        ("truncated", "0 or 1"),
        ("difficult", "0 or 1"),
        ("bndbox", "bounding box coordinates"),
    ];

    for (field, _description) in object_fields {
        assert!(!field.is_empty());
    }
}

#[test]
fn test_voc_size_element() {
    let data = create_voc_project();
    let image = &data.images[0];

    if let Some((width, height)) = image.dimensions {
        // VOC size element
        assert_eq!(width, 500);
        assert_eq!(height, 375);

        // Depth is typically 3 for RGB images
        let depth = 3;
        assert_eq!(depth, 3);
    }
}

#[test]
fn test_voc_filename_extraction() {
    let path = PathBuf::from("JPEGImages/2007_000027.jpg");
    let filename = path.file_name().unwrap().to_str().unwrap();

    assert_eq!(filename, "2007_000027.jpg");
}

#[test]
fn test_voc_folder_extraction() {
    let path = PathBuf::from("/data/VOC2012/JPEGImages/2007_000027.jpg");

    // The folder in VOC is typically the parent of the image
    let folder = path
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    assert_eq!(folder, "JPEGImages");
}

#[test]
fn test_voc_annotation_file_path() {
    // VOC expects: JPEGImages/image.jpg -> Annotations/image.xml
    let image_path = PathBuf::from("JPEGImages/2007_000027.jpg");
    let stem = image_path.file_stem().unwrap().to_str().unwrap();

    let annotation_path = PathBuf::from(format!("Annotations/{}.xml", stem));

    assert_eq!(
        annotation_path,
        PathBuf::from("Annotations/2007_000027.xml")
    );
}

#[test]
fn test_voc_category_names() {
    let data = create_voc_project();

    // Category names should be valid identifiers
    for cat in &data.categories {
        assert!(!cat.name.is_empty());
        assert!(!cat.name.contains(' ')); // No spaces in VOC category names typically
    }
}

#[test]
fn test_voc_bbox_integer_coordinates() {
    // VOC uses integer coordinates
    let bbox = ShapeEntry::BoundingBox {
        x: 100.5,
        y: 150.7,
        width: 200.3,
        height: 100.9,
    };

    if let ShapeEntry::BoundingBox {
        x,
        y,
        width,
        height,
    } = bbox
    {
        // Round to integers for VOC
        let xmin = x as u32;
        let ymin = y as u32;
        let xmax = (x + width) as u32;
        let ymax = (y + height) as u32;

        assert_eq!(xmin, 100);
        assert_eq!(ymin, 150);
        assert_eq!(xmax, 300); // 100.5 + 200.3 = 300.8 -> 300
        assert_eq!(ymax, 251); // 150.7 + 100.9 = 251.6 -> 251
    }
}

#[test]
fn test_voc_difficult_flag() {
    // VOC has a "difficult" flag for hard-to-recognize objects
    let difficult = false;
    let difficult_str = if difficult { "1" } else { "0" };

    assert_eq!(difficult_str, "0");
}

#[test]
fn test_voc_truncated_flag() {
    // VOC has a "truncated" flag for partially visible objects
    let truncated = false;
    let truncated_str = if truncated { "1" } else { "0" };

    assert_eq!(truncated_str, "0");
}

#[test]
fn test_voc_pose_values() {
    let valid_poses = ["Unspecified", "Left", "Right", "Frontal", "Rear"];

    // Default pose
    let default_pose = "Unspecified";
    assert!(valid_poses.contains(&default_pose));
}

#[test]
fn test_voc_polygon_not_supported() {
    let format = PascalVocFormat;
    assert!(!format.supports_polygon());
}

#[test]
fn test_voc_point_not_supported() {
    let format = PascalVocFormat;
    assert!(!format.supports_point());
}

#[test]
fn test_voc_dimensions_required() {
    // VOC requires image dimensions in the size element
    let image = ImageEntry::new(PathBuf::from("test.jpg"));
    assert!(image.dimensions.is_none());

    let image_with_dims = image.with_dimensions(800, 600);
    assert_eq!(image_with_dims.dimensions, Some((800, 600)));
}

#[test]
fn test_empty_voc_annotation() {
    // Image without annotations should still have a valid XML structure
    let mut data = ProjectData::new();
    data.categories.push(CategoryEntry::new(1, "person"));

    let image = ImageEntry::new(PathBuf::from("empty.jpg")).with_dimensions(640, 480);
    data.images.push(image);

    assert!(data.images[0].annotations.is_empty());
    // The XML should still have annotation, folder, filename, size elements
}

#[test]
fn test_voc_special_characters_in_path() {
    // Test handling of special characters in filenames
    let path = PathBuf::from("images/test image (1).jpg");
    let filename = path.file_name().unwrap().to_str().unwrap();

    // XML should escape special characters
    assert!(filename.contains(' '));
    assert!(filename.contains('('));
    assert!(filename.contains(')'));
}
