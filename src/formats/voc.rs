//! Pascal VOC format support.
//!
//! Pascal VOC uses one XML file per image with bounding box annotations.
//!
//! # Format Structure
//!
//! ```xml
//! <annotation>
//!   <folder>JPEGImages</folder>
//!   <filename>image1.jpg</filename>
//!   <size>
//!     <width>640</width>
//!     <height>480</height>
//!     <depth>3</depth>
//!   </size>
//!   <object>
//!     <name>person</name>
//!     <pose>Unspecified</pose>
//!     <truncated>0</truncated>
//!     <difficult>0</difficult>
//!     <bndbox>
//!       <xmin>100</xmin>
//!       <ymin>100</ymin>
//!       <xmax>200</xmax>
//!       <ymax>200</ymax>
//!     </bndbox>
//!   </object>
//! </annotation>
//! ```
//!
//! Note: Pascal VOC only supports bounding boxes. Points and polygons will be
//! skipped with a warning.

use super::{common::ImageInfo, AnnotationFormat, ExportResult, FormatError, ImportResult};
use crate::{AnnotationStore, BoundingBox, Category, Shape};
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pascal VOC format implementation.
#[derive(Debug, Clone, Default)]
pub struct VocFormat {
    /// Folder name for the images (default: "JPEGImages").
    pub folder: String,
}

impl VocFormat {
    /// Create a new Pascal VOC format handler.
    pub fn new() -> Self {
        Self {
            folder: "JPEGImages".to_string(),
        }
    }

    /// Set the folder name for images.
    pub fn with_folder(mut self, folder: impl Into<String>) -> Self {
        self.folder = folder.into();
        self
    }
}

impl AnnotationFormat for VocFormat {
    fn name(&self) -> &'static str {
        "Pascal VOC"
    }

    fn extensions(&self) -> &[&'static str] {
        &["xml"]
    }

    fn supports_shape(&self, shape: &Shape) -> bool {
        matches!(shape, Shape::BoundingBox(_))
    }

    fn export_dataset(
        &self,
        stores: &[(ImageInfo, &AnnotationStore)],
    ) -> Result<ExportResult, FormatError> {
        let mut result = ExportResult::new();

        // Collect all categories for name lookup
        let mut category_names: HashMap<u32, String> = HashMap::new();
        for (_, store) in stores {
            for cat in store.categories() {
                category_names.entry(cat.id).or_insert_with(|| cat.name.clone());
            }
        }

        // Process each image
        for (info, store) in stores {
            let mut voc_ann = VocAnnotation {
                folder: self.folder.clone(),
                filename: info.file_name.clone(),
                size: VocSize {
                    width: info.width as i32,
                    height: info.height as i32,
                    depth: 3,
                },
                objects: Vec::new(),
            };

            for ann in store.iter() {
                match &ann.shape {
                    Shape::BoundingBox(bbox) => {
                        let name = category_names
                            .get(&ann.category_id)
                            .map(|s| s.as_str())
                            .unwrap_or("object");

                        voc_ann.objects.push(VocObject {
                            name: name.to_string(),
                            pose: "Unspecified".to_string(),
                            truncated: 0,
                            difficult: 0,
                            bndbox: VocBndbox {
                                xmin: bbox.x as i32,
                                ymin: bbox.y as i32,
                                xmax: (bbox.x + bbox.width) as i32,
                                ymax: (bbox.y + bbox.height) as i32,
                            },
                        });
                    }
                    Shape::Polygon(_) => {
                        result.add_warning(format!(
                            "Skipped polygon annotation {} (VOC only supports bounding boxes)",
                            ann.id
                        ));
                    }
                    Shape::Point(_) => {
                        result.add_warning(format!(
                            "Skipped point annotation {} (VOC only supports bounding boxes)",
                            ann.id
                        ));
                    }
                }
            }

            // Generate XML
            let xml = to_string(&voc_ann).map_err(|e| FormatError::Xml(e.to_string()))?;

            // Add XML declaration and format nicely
            let formatted_xml = format_voc_xml(&xml);

            let xml_filename = format!("{}.xml", info.base_name());
            result.add_file(xml_filename, formatted_xml);
        }

        Ok(result)
    }

    fn import_dataset(
        &self,
        files: &HashMap<String, String>,
    ) -> Result<ImportResult, FormatError> {
        let mut result = ImportResult::new();

        // Track category names to IDs - start from 1 to avoid collision with default "Object" category
        let mut category_map: HashMap<String, u32> = HashMap::new();
        let mut next_category_id: u32 = 1;

        // Process XML files
        for (filename, content) in files {
            if !filename.ends_with(".xml") {
                continue;
            }

            // Skip if not valid XML
            if !content.trim_start().starts_with('<') {
                continue;
            }

            let voc_ann: VocAnnotation = from_str(content).map_err(|e| {
                FormatError::Xml(format!("Failed to parse {}: {}", filename, e))
            })?;

            let mut store = AnnotationStore::new();

            // Process objects
            for obj in &voc_ann.objects {
                // Get or create category
                let is_new_category = !category_map.contains_key(&obj.name);
                let category_id = *category_map.entry(obj.name.clone()).or_insert_with(|| {
                    let id = next_category_id;
                    next_category_id += 1;
                    id
                });

                // Add category to store and result if it's new
                if is_new_category {
                    let cat = Category::new(category_id, &obj.name);
                    store.add_category(cat.clone());
                    result.add_category(cat);
                } else if store.get_category(category_id).is_none() {
                    // Category exists in map but not in this store
                    store.add_category(Category::new(category_id, &obj.name));
                }

                // Create bounding box, clamping to ensure non-negative values
                let bbox = BoundingBox::new(
                    (obj.bndbox.xmin as f32).max(0.0),
                    (obj.bndbox.ymin as f32).max(0.0),
                    ((obj.bndbox.xmax - obj.bndbox.xmin) as f32).max(0.0),
                    ((obj.bndbox.ymax - obj.bndbox.ymin) as f32).max(0.0),
                );

                store.add(category_id, Shape::BoundingBox(bbox));
            }

            // Use the filename from the annotation, or derive from XML filename
            let image_name = if !voc_ann.filename.is_empty() {
                voc_ann.filename.clone()
            } else {
                filename.trim_end_matches(".xml").to_string()
            };

            result.add_annotations(image_name, store);
        }

        Ok(result)
    }
}

/// Format the XML output with proper indentation.
fn format_voc_xml(xml: &str) -> String {
    // Add XML declaration
    let mut formatted = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");

    // Simple indentation - not perfect but functional
    let mut indent: usize = 0;
    let mut in_text = false;
    let mut chars = xml.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '<' => {
                if let Some(&next) = chars.peek() {
                    if next == '/' {
                        indent = indent.saturating_sub(1);
                        if !in_text {
                            formatted.push('\n');
                            formatted.push_str(&"  ".repeat(indent));
                        }
                    } else if !in_text {
                        formatted.push('\n');
                        formatted.push_str(&"  ".repeat(indent));
                    }
                }
                formatted.push(c);
                in_text = true;
            }
            '>' => {
                formatted.push(c);
                if let Some(prev) = xml.as_bytes().get(formatted.len().saturating_sub(2)).map(|b| *b as char) {
                    if prev != '/' {
                        // Check if this is an opening tag (not a closing tag)
                        let tag_content: String = formatted.chars().rev().skip(1).take_while(|&c| c != '<').collect();
                        if !tag_content.starts_with('/') {
                            indent += 1;
                        }
                    }
                }
                in_text = false;
            }
            _ => {
                formatted.push(c);
            }
        }
    }

    formatted
}

// ============================================================================
// Pascal VOC XML Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "annotation")]
struct VocAnnotation {
    folder: String,
    filename: String,
    size: VocSize,
    #[serde(rename = "object", default)]
    objects: Vec<VocObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VocSize {
    width: i32,
    height: i32,
    depth: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VocObject {
    name: String,
    #[serde(default = "default_pose")]
    pose: String,
    #[serde(default)]
    truncated: i32,
    #[serde(default)]
    difficult: i32,
    bndbox: VocBndbox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VocBndbox {
    xmin: i32,
    ymin: i32,
    xmax: i32,
    ymax: i32,
}

fn default_pose() -> String {
    "Unspecified".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point, Polygon};

    fn create_test_store() -> AnnotationStore {
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(0, "car"));
        store.add_category(Category::new(1, "person"));
        store.add(
            0,
            Shape::BoundingBox(BoundingBox::new(100.0, 150.0, 200.0, 100.0)),
        );
        store.add(
            1,
            Shape::BoundingBox(BoundingBox::new(300.0, 200.0, 50.0, 120.0)),
        );
        store
    }

    #[test]
    fn test_voc_export() {
        let format = VocFormat::new();
        let store = create_test_store();
        let info = ImageInfo::new("test.jpg", 640, 480);

        let result = format.export_dataset(&[(info, &store)]).unwrap();
        assert!(result.files.contains_key("test.xml"));

        let xml = &result.files["test.xml"];
        // Check for key content - quick-xml may format differently
        assert!(xml.contains("annotation"), "Missing annotation tag in: {}", xml);
        assert!(xml.contains("test.jpg"), "Missing filename in: {}", xml);
        assert!(xml.contains("640"), "Missing width in: {}", xml);
        assert!(xml.contains("480"), "Missing height in: {}", xml);
        assert!(xml.contains("car"), "Missing car name in: {}", xml);
        assert!(xml.contains("person"), "Missing person name in: {}", xml);
        assert!(xml.contains("100"), "Missing xmin in: {}", xml);
        assert!(xml.contains("300"), "Missing xmax in: {}", xml);
    }

    #[test]
    fn test_voc_import() {
        // Use XML without leading whitespace/newlines that can confuse parsers
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?><annotation><folder>JPEGImages</folder><filename>test.jpg</filename><size><width>640</width><height>480</height><depth>3</depth></size><object><name>car</name><pose>Unspecified</pose><truncated>0</truncated><difficult>0</difficult><bndbox><xmin>100</xmin><ymin>150</ymin><xmax>300</xmax><ymax>250</ymax></bndbox></object></annotation>"#;

        // First test direct parsing
        let parsed: VocAnnotation = from_str(xml).expect("Failed to parse XML");
        assert_eq!(parsed.filename, "test.jpg");
        assert_eq!(parsed.objects.len(), 1);
        assert_eq!(parsed.objects[0].name, "car");

        let format = VocFormat::new();
        let mut files = HashMap::new();
        files.insert("test.xml".to_string(), xml.to_string());

        let result = format.import_dataset(&files).unwrap();

        assert_eq!(result.categories.len(), 1, "Expected 1 category, got {:?}", result.categories);
        assert!(result.annotations.contains_key("test.jpg"), "Missing test.jpg, got keys: {:?}", result.annotations.keys().collect::<Vec<_>>());

        let store = &result.annotations["test.jpg"];
        assert_eq!(store.len(), 1);

        // Check the bounding box values
        let ann = store.iter().next().unwrap();
        if let Shape::BoundingBox(bbox) = &ann.shape {
            assert_eq!(bbox.x, 100.0);
            assert_eq!(bbox.y, 150.0);
            assert_eq!(bbox.width, 200.0);
            assert_eq!(bbox.height, 100.0);
        } else {
            panic!("Expected bounding box");
        }
    }

    #[test]
    fn test_voc_roundtrip() {
        let format = VocFormat::new();
        let original_store = create_test_store();
        let info = ImageInfo::new("roundtrip.jpg", 640, 480);

        // Export
        let export_result = format
            .export_dataset(&[(info.clone(), &original_store)])
            .unwrap();
        let xml = &export_result.files["roundtrip.xml"];

        // Import
        let mut files = HashMap::new();
        files.insert("roundtrip.xml".to_string(), xml.clone());
        let import_result = format.import_dataset(&files).unwrap();

        // Verify
        let imported_store = &import_result.annotations["roundtrip.jpg"];
        assert_eq!(imported_store.len(), original_store.len());
    }

    #[test]
    fn test_voc_skips_polygons_and_points() {
        let format = VocFormat::new();
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(0, "test"));

        // Add polygon
        let mut poly = Polygon::new();
        poly.push(Point::new(100.0, 100.0));
        poly.push(Point::new(200.0, 100.0));
        poly.push(Point::new(150.0, 200.0));
        poly.close();
        store.add(0, Shape::Polygon(poly));

        // Add point
        store.add(0, Shape::Point(Point::new(300.0, 300.0)));

        let info = ImageInfo::new("test.jpg", 640, 480);
        let result = format.export_dataset(&[(info, &store)]).unwrap();

        // Should have warnings for both
        assert_eq!(result.warnings.len(), 2);
        assert!(result.warnings.iter().any(|w| w.contains("polygon")));
        assert!(result.warnings.iter().any(|w| w.contains("point")));

        // XML should have no objects
        let xml = &result.files["test.xml"];
        assert!(!xml.contains("<object>"));
    }

    #[test]
    fn test_supports_shape() {
        let format = VocFormat::new();
        assert!(format.supports_shape(&Shape::BoundingBox(BoundingBox::new(0.0, 0.0, 10.0, 10.0))));
        assert!(!format.supports_shape(&Shape::Polygon(Polygon::new())));
        assert!(!format.supports_shape(&Shape::Point(Point::new(0.0, 0.0))));
    }
}
