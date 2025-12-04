//! COCO (Common Objects in Context) format support.
//!
//! COCO is a widely-used format for object detection, segmentation, and keypoint detection.
//! It uses a single JSON file for the entire dataset.
//!
//! # Format Structure
//!
//! ```json
//! {
//!   "info": { "description": "Dataset", "version": "1.0" },
//!   "licenses": [],
//!   "images": [
//!     { "id": 1, "file_name": "image1.jpg", "width": 640, "height": 480 }
//!   ],
//!   "annotations": [
//!     {
//!       "id": 1,
//!       "image_id": 1,
//!       "category_id": 1,
//!       "bbox": [x, y, width, height],
//!       "segmentation": [[x1,y1,x2,y2,...]],
//!       "area": 1234.5,
//!       "iscrowd": 0
//!     }
//!   ],
//!   "categories": [
//!     { "id": 1, "name": "person", "supercategory": "human" }
//!   ]
//! }
//! ```

use super::{common::ImageInfo, AnnotationFormat, ExportResult, FormatError, ImportResult};
use crate::{
    AnnotationStore, BoundingBox, Category, Shape,
    formats::common::{polygon_to_flat_coords, flat_coords_to_polygon},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// COCO format implementation.
#[derive(Debug, Clone, Default)]
pub struct CocoFormat {
    /// Description for the dataset info section.
    pub description: String,
    /// Version string for the dataset info section.
    pub version: String,
}

impl CocoFormat {
    /// Create a new COCO format handler.
    pub fn new() -> Self {
        Self {
            description: "HVAT Export".to_string(),
            version: "1.0".to_string(),
        }
    }

    /// Set the dataset description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the dataset version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }
}

impl AnnotationFormat for CocoFormat {
    fn name(&self) -> &'static str {
        "COCO"
    }

    fn extensions(&self) -> &[&'static str] {
        &["json"]
    }

    fn supports_shape(&self, shape: &Shape) -> bool {
        match shape {
            Shape::BoundingBox(_) => true,
            Shape::Polygon(_) => true,
            Shape::Point(_) => false, // COCO doesn't natively support standalone points
        }
    }

    fn export_dataset(
        &self,
        stores: &[(ImageInfo, &AnnotationStore)],
    ) -> Result<ExportResult, FormatError> {
        let mut result = ExportResult::new();
        let mut coco = CocoDataset::new(&self.description, &self.version);

        // Collect all categories from all stores
        let mut category_map: HashMap<u32, Category> = HashMap::new();
        for (_, store) in stores {
            for cat in store.categories() {
                category_map.entry(cat.id).or_insert_with(|| cat.clone());
            }
        }

        // Add categories to COCO format
        for cat in category_map.values() {
            coco.categories.push(CocoCategory {
                id: cat.id as i64,
                name: cat.name.clone(),
                supercategory: "object".to_string(),
            });
        }

        // Process each image
        let mut annotation_id: i64 = 1;
        for (idx, (info, store)) in stores.iter().enumerate() {
            let image_id = info.id.unwrap_or((idx + 1) as u64) as i64;

            // Add image entry
            coco.images.push(CocoImage {
                id: image_id,
                file_name: info.file_name.clone(),
                width: info.width as i64,
                height: info.height as i64,
            });

            // Add annotations for this image
            for ann in store.iter() {
                match &ann.shape {
                    Shape::BoundingBox(bbox) => {
                        coco.annotations.push(CocoAnnotation {
                            id: annotation_id,
                            image_id,
                            category_id: ann.category_id as i64,
                            bbox: Some(vec![bbox.x as f64, bbox.y as f64, bbox.width as f64, bbox.height as f64]),
                            segmentation: None,
                            area: (bbox.width * bbox.height) as f64,
                            iscrowd: 0,
                        });
                        annotation_id += 1;
                    }
                    Shape::Polygon(poly) => {
                        let flat_coords: Vec<f64> = polygon_to_flat_coords(poly)
                            .iter()
                            .map(|&x| x as f64)
                            .collect();
                        let bbox = poly.bounding_box();
                        let area = bbox.map(|b| b.area() as f64).unwrap_or(0.0);

                        coco.annotations.push(CocoAnnotation {
                            id: annotation_id,
                            image_id,
                            category_id: ann.category_id as i64,
                            bbox: bbox.map(|b| vec![b.x as f64, b.y as f64, b.width as f64, b.height as f64]),
                            segmentation: Some(vec![flat_coords]),
                            area,
                            iscrowd: 0,
                        });
                        annotation_id += 1;
                    }
                    Shape::Point(_) => {
                        result.add_warning(format!(
                            "Skipped point annotation {} (COCO doesn't support standalone points)",
                            ann.id
                        ));
                    }
                }
            }
        }

        let json = serde_json::to_string_pretty(&coco)?;
        result.add_file("instances.json", json);

        Ok(result)
    }

    fn import_dataset(
        &self,
        files: &HashMap<String, String>,
    ) -> Result<ImportResult, FormatError> {
        let mut result = ImportResult::new();

        // Find the JSON file
        let json_content = files
            .values()
            .find(|content| content.trim_start().starts_with('{'))
            .ok_or_else(|| FormatError::MissingField("JSON file".to_string()))?;

        let coco: CocoDataset = serde_json::from_str(json_content)?;

        // Build category map
        let mut category_map: HashMap<i64, Category> = HashMap::new();
        for coco_cat in &coco.categories {
            let cat = Category::new(coco_cat.id as u32, &coco_cat.name);
            category_map.insert(coco_cat.id, cat.clone());
            result.add_category(cat);
        }

        // Build image ID to filename map
        let mut image_map: HashMap<i64, (String, u32, u32)> = HashMap::new();
        for img in &coco.images {
            image_map.insert(img.id, (img.file_name.clone(), img.width as u32, img.height as u32));
        }

        // Group annotations by image
        let mut annotations_by_image: HashMap<i64, Vec<&CocoAnnotation>> = HashMap::new();
        for ann in &coco.annotations {
            annotations_by_image
                .entry(ann.image_id)
                .or_default()
                .push(ann);
        }

        // Process annotations for each image
        for (image_id, anns) in annotations_by_image {
            let (file_name, _width, _height) = match image_map.get(&image_id) {
                Some(info) => info.clone(),
                None => {
                    result.add_warning(format!("Skipped annotations for unknown image_id {}", image_id));
                    continue;
                }
            };

            let mut store = AnnotationStore::new();

            // Add categories to the store
            for cat in category_map.values() {
                store.add_category(cat.clone());
            }

            for coco_ann in anns {
                // Try to import as polygon first, then as bbox
                let shape = if let Some(ref segs) = coco_ann.segmentation {
                    if let Some(seg) = segs.first() {
                        let f32_coords: Vec<f32> = seg.iter().map(|&x| x as f32).collect();
                        flat_coords_to_polygon(&f32_coords).map(Shape::Polygon)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let shape = shape.or_else(|| {
                    coco_ann.bbox.as_ref().map(|bbox| {
                        if bbox.len() >= 4 {
                            // Clamp coordinates to ensure non-negative values
                            Shape::BoundingBox(BoundingBox::new(
                                (bbox[0] as f32).max(0.0),
                                (bbox[1] as f32).max(0.0),
                                (bbox[2] as f32).max(0.0),
                                (bbox[3] as f32).max(0.0),
                            ))
                        } else {
                            Shape::BoundingBox(BoundingBox::new(0.0, 0.0, 0.0, 0.0))
                        }
                    })
                });

                match shape {
                    Some(s) => {
                        store.add(coco_ann.category_id as u32, s);
                    }
                    None => {
                        result.add_warning(format!(
                            "Skipped annotation {} (no bbox or segmentation)",
                            coco_ann.id
                        ));
                    }
                }
            }

            result.add_annotations(file_name, store);
        }

        Ok(result)
    }
}

// ============================================================================
// COCO JSON Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CocoDataset {
    info: CocoInfo,
    licenses: Vec<CocoLicense>,
    images: Vec<CocoImage>,
    annotations: Vec<CocoAnnotation>,
    categories: Vec<CocoCategory>,
}

impl CocoDataset {
    fn new(description: &str, version: &str) -> Self {
        Self {
            info: CocoInfo {
                description: description.to_string(),
                version: version.to_string(),
                year: 2024,
                contributor: "HVAT".to_string(),
                date_created: String::new(),
            },
            licenses: Vec::new(),
            images: Vec::new(),
            annotations: Vec::new(),
            categories: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CocoInfo {
    description: String,
    version: String,
    #[serde(default)]
    year: i32,
    #[serde(default)]
    contributor: String,
    #[serde(default)]
    date_created: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CocoLicense {
    id: i64,
    name: String,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CocoImage {
    id: i64,
    file_name: String,
    width: i64,
    height: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CocoAnnotation {
    id: i64,
    image_id: i64,
    category_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    bbox: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    segmentation: Option<Vec<Vec<f64>>>,
    area: f64,
    iscrowd: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CocoCategory {
    id: i64,
    name: String,
    #[serde(default = "default_supercategory")]
    supercategory: String,
}

fn default_supercategory() -> String {
    "object".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point, Polygon};

    fn create_test_store() -> AnnotationStore {
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(1, "car"));
        store.add_category(Category::new(2, "person"));
        store.add(1, Shape::BoundingBox(BoundingBox::new(10.0, 20.0, 100.0, 50.0)));

        let mut poly = Polygon::new();
        poly.push(Point::new(200.0, 200.0));
        poly.push(Point::new(300.0, 200.0));
        poly.push(Point::new(250.0, 300.0));
        poly.close();
        store.add(2, Shape::Polygon(poly));

        store
    }

    #[test]
    fn test_coco_export() {
        let format = CocoFormat::new();
        let store = create_test_store();
        let info = ImageInfo::new("test.jpg", 640, 480);

        let result = format.export_dataset(&[(info, &store)]).unwrap();
        assert!(result.files.contains_key("instances.json"));

        let json = &result.files["instances.json"];
        assert!(json.contains("\"car\""));
        assert!(json.contains("\"person\""));
        assert!(json.contains("\"bbox\""));
        assert!(json.contains("\"segmentation\""));
    }

    #[test]
    fn test_coco_import() {
        let coco_json = r#"{
            "info": { "description": "Test", "version": "1.0" },
            "licenses": [],
            "images": [
                { "id": 1, "file_name": "test.jpg", "width": 640, "height": 480 }
            ],
            "annotations": [
                {
                    "id": 1,
                    "image_id": 1,
                    "category_id": 1,
                    "bbox": [10, 20, 100, 50],
                    "area": 5000,
                    "iscrowd": 0
                }
            ],
            "categories": [
                { "id": 1, "name": "car", "supercategory": "vehicle" }
            ]
        }"#;

        let format = CocoFormat::new();
        let mut files = HashMap::new();
        files.insert("instances.json".to_string(), coco_json.to_string());

        let result = format.import_dataset(&files).unwrap();
        assert_eq!(result.categories.len(), 1);
        assert_eq!(result.annotations.len(), 1);
        assert!(result.annotations.contains_key("test.jpg"));

        let store = &result.annotations["test.jpg"];
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_coco_roundtrip() {
        let format = CocoFormat::new();
        let original_store = create_test_store();
        let info = ImageInfo::new("roundtrip.jpg", 640, 480);

        // Export
        let export_result = format.export_dataset(&[(info.clone(), &original_store)]).unwrap();
        let json = &export_result.files["instances.json"];

        // Import
        let mut files = HashMap::new();
        files.insert("instances.json".to_string(), json.clone());
        let import_result = format.import_dataset(&files).unwrap();

        // Verify
        assert_eq!(import_result.categories.len(), 3); // Including default "Object" category
        let imported_store = &import_result.annotations["roundtrip.jpg"];
        assert_eq!(imported_store.len(), original_store.len());
    }

    #[test]
    fn test_coco_skips_points() {
        let format = CocoFormat::new();
        let mut store = AnnotationStore::new();
        store.add(0, Shape::Point(Point::new(100.0, 100.0)));

        let info = ImageInfo::new("test.jpg", 640, 480);
        let result = format.export_dataset(&[(info, &store)]).unwrap();

        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("point"));
    }

    #[test]
    fn test_supports_shape() {
        let format = CocoFormat::new();
        assert!(format.supports_shape(&Shape::BoundingBox(BoundingBox::new(0.0, 0.0, 10.0, 10.0))));
        assert!(format.supports_shape(&Shape::Polygon(Polygon::new())));
        assert!(!format.supports_shape(&Shape::Point(Point::new(0.0, 0.0))));
    }
}
