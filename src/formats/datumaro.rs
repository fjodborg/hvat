//! Datumaro format support.
//!
//! Datumaro is Intel's dataset format that supports multiple annotation types
//! including bounding boxes, polygons, points, masks, and more.
//!
//! # Format Structure
//!
//! ```json
//! {
//!   "info": {},
//!   "categories": {
//!     "label": {
//!       "labels": [{ "name": "cat", "parent": "", "attributes": [] }]
//!     }
//!   },
//!   "items": [
//!     {
//!       "id": "image1",
//!       "annotations": [
//!         {
//!           "id": 0,
//!           "type": "bbox",
//!           "label_id": 0,
//!           "bbox": [x, y, w, h],
//!           "attributes": {}
//!         }
//!       ]
//!     }
//!   ]
//! }
//! ```

use super::{common::ImageInfo, AnnotationFormat, ExportResult, FormatError, ImportResult};
use crate::{
    formats::common::{flat_coords_to_polygon, polygon_to_flat_coords},
    AnnotationStore, BoundingBox, Category, Point, Shape,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Datumaro format implementation.
#[derive(Debug, Clone, Default)]
pub struct DatumaroFormat;

impl DatumaroFormat {
    /// Create a new Datumaro format handler.
    pub fn new() -> Self {
        Self
    }
}

impl AnnotationFormat for DatumaroFormat {
    fn name(&self) -> &'static str {
        "Datumaro"
    }

    fn extensions(&self) -> &[&'static str] {
        &["json"]
    }

    fn supports_shape(&self, _shape: &Shape) -> bool {
        // Datumaro supports all shape types
        true
    }

    fn export_dataset(
        &self,
        stores: &[(ImageInfo, &AnnotationStore)],
    ) -> Result<ExportResult, FormatError> {
        let mut result = ExportResult::new();
        let mut dataset = DatumaroDataset::new();

        // Collect all categories
        let mut category_map: HashMap<u32, Category> = HashMap::new();
        for (_, store) in stores {
            for cat in store.categories() {
                category_map.entry(cat.id).or_insert_with(|| cat.clone());
            }
        }

        // Add labels to dataset
        let mut label_ids: Vec<u32> = category_map.keys().copied().collect();
        label_ids.sort();

        // Build label_id -> index mapping
        let label_index: HashMap<u32, usize> = label_ids
            .iter()
            .enumerate()
            .map(|(idx, &id)| (id, idx))
            .collect();

        for id in &label_ids {
            if let Some(cat) = category_map.get(id) {
                dataset.categories.label.labels.push(DatumaroLabel {
                    name: cat.name.clone(),
                    parent: String::new(),
                    attributes: Vec::new(),
                });
            }
        }

        // Process each image
        for (info, store) in stores {
            let mut item = DatumaroItem {
                id: info.base_name().to_string(),
                annotations: Vec::new(),
            };

            let mut ann_id: i64 = 0;
            for ann in store.iter() {
                let label_idx = label_index.get(&ann.category_id).copied().unwrap_or(0);

                let datum_ann = match &ann.shape {
                    Shape::BoundingBox(bbox) => DatumaroAnnotation {
                        id: ann_id,
                        annotation_type: "bbox".to_string(),
                        label_id: label_idx as i64,
                        bbox: Some(vec![
                            bbox.x as f64,
                            bbox.y as f64,
                            bbox.width as f64,
                            bbox.height as f64,
                        ]),
                        points: None,
                        attributes: ann
                            .attributes
                            .iter()
                            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                            .collect(),
                    },
                    Shape::Polygon(poly) => {
                        let coords: Vec<f64> =
                            polygon_to_flat_coords(poly).iter().map(|&x| x as f64).collect();
                        DatumaroAnnotation {
                            id: ann_id,
                            annotation_type: "polygon".to_string(),
                            label_id: label_idx as i64,
                            bbox: None,
                            points: Some(coords),
                            attributes: ann
                                .attributes
                                .iter()
                                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                                .collect(),
                        }
                    }
                    Shape::Point(p) => DatumaroAnnotation {
                        id: ann_id,
                        annotation_type: "points".to_string(),
                        label_id: label_idx as i64,
                        bbox: None,
                        points: Some(vec![p.x as f64, p.y as f64]),
                        attributes: ann
                            .attributes
                            .iter()
                            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                            .collect(),
                    },
                };

                item.annotations.push(datum_ann);
                ann_id += 1;
            }

            dataset.items.push(item);
        }

        let json = serde_json::to_string_pretty(&dataset)?;
        result.add_file("default.json", json);

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

        let dataset: DatumaroDataset = serde_json::from_str(json_content)?;

        // Build label index -> category mapping
        let mut categories: Vec<Category> = Vec::new();
        for (idx, label) in dataset.categories.label.labels.iter().enumerate() {
            let cat = Category::new(idx as u32, &label.name);
            categories.push(cat.clone());
            result.add_category(cat);
        }

        // Process items
        for item in &dataset.items {
            let mut store = AnnotationStore::new();

            // Add categories to store
            for cat in &categories {
                store.add_category(cat.clone());
            }

            for ann in &item.annotations {
                let category_id = ann.label_id as u32;

                let shape = match ann.annotation_type.as_str() {
                    "bbox" => {
                        if let Some(ref bbox) = ann.bbox {
                            if bbox.len() >= 4 {
                                // Clamp coordinates to ensure non-negative values
                                Some(Shape::BoundingBox(BoundingBox::new(
                                    (bbox[0] as f32).max(0.0),
                                    (bbox[1] as f32).max(0.0),
                                    (bbox[2] as f32).max(0.0),
                                    (bbox[3] as f32).max(0.0),
                                )))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    "polygon" => {
                        if let Some(ref points) = ann.points {
                            let f32_points: Vec<f32> = points.iter().map(|&x| x as f32).collect();
                            flat_coords_to_polygon(&f32_points).map(Shape::Polygon)
                        } else {
                            None
                        }
                    }
                    "points" | "point" => {
                        if let Some(ref points) = ann.points {
                            if points.len() >= 2 {
                                // Clamp coordinates to ensure non-negative values
                                Some(Shape::Point(Point::new(
                                    (points[0] as f32).max(0.0),
                                    (points[1] as f32).max(0.0),
                                )))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    other => {
                        result.add_warning(format!(
                            "Skipped annotation with unknown type '{}' in item '{}'",
                            other, item.id
                        ));
                        None
                    }
                };

                if let Some(s) = shape {
                    let ann_id = store.add(category_id, s);

                    // Add attributes
                    if let Some(annotation) = store.get_mut(ann_id) {
                        for (key, value) in &ann.attributes {
                            let value_str = match value {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            annotation.attributes.insert(key.clone(), value_str);
                        }
                    }
                }
            }

            result.add_annotations(item.id.clone(), store);
        }

        Ok(result)
    }
}

// ============================================================================
// Datumaro JSON Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatumaroDataset {
    #[serde(default)]
    info: DatumaroInfo,
    categories: DatumaroCategories,
    items: Vec<DatumaroItem>,
}

impl DatumaroDataset {
    fn new() -> Self {
        Self {
            info: DatumaroInfo::default(),
            categories: DatumaroCategories::default(),
            items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DatumaroInfo {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DatumaroCategories {
    label: DatumaroLabelCategory,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DatumaroLabelCategory {
    labels: Vec<DatumaroLabel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatumaroLabel {
    name: String,
    #[serde(default)]
    parent: String,
    #[serde(default)]
    attributes: Vec<DatumaroAttribute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatumaroAttribute {
    name: String,
    #[serde(rename = "type")]
    attr_type: String,
    #[serde(default)]
    values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatumaroItem {
    id: String,
    annotations: Vec<DatumaroAnnotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatumaroAnnotation {
    id: i64,
    #[serde(rename = "type")]
    annotation_type: String,
    label_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    bbox: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    points: Option<Vec<f64>>,
    #[serde(default)]
    attributes: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Polygon;

    fn create_test_store() -> AnnotationStore {
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(0, "car"));
        store.add_category(Category::new(1, "person"));

        store.add(
            0,
            Shape::BoundingBox(BoundingBox::new(10.0, 20.0, 100.0, 50.0)),
        );

        let mut poly = Polygon::new();
        poly.push(Point::new(200.0, 200.0));
        poly.push(Point::new(300.0, 200.0));
        poly.push(Point::new(250.0, 300.0));
        poly.close();
        store.add(1, Shape::Polygon(poly));

        store.add(0, Shape::Point(Point::new(400.0, 400.0)));

        store
    }

    #[test]
    fn test_datumaro_export() {
        let format = DatumaroFormat::new();
        let store = create_test_store();
        let info = ImageInfo::new("test.jpg", 640, 480);

        let result = format.export_dataset(&[(info, &store)]).unwrap();
        assert!(result.files.contains_key("default.json"));

        let json = &result.files["default.json"];
        assert!(json.contains("\"car\""));
        assert!(json.contains("\"person\""));
        assert!(json.contains("\"bbox\""));
        assert!(json.contains("\"polygon\""));
        assert!(json.contains("\"points\""));
    }

    #[test]
    fn test_datumaro_import() {
        let json = r#"{
            "info": {},
            "categories": {
                "label": {
                    "labels": [
                        { "name": "car", "parent": "", "attributes": [] },
                        { "name": "person", "parent": "", "attributes": [] }
                    ]
                }
            },
            "items": [
                {
                    "id": "test",
                    "annotations": [
                        {
                            "id": 0,
                            "type": "bbox",
                            "label_id": 0,
                            "bbox": [10, 20, 100, 50],
                            "attributes": {}
                        },
                        {
                            "id": 1,
                            "type": "points",
                            "label_id": 1,
                            "points": [400, 400],
                            "attributes": {}
                        }
                    ]
                }
            ]
        }"#;

        let format = DatumaroFormat::new();
        let mut files = HashMap::new();
        files.insert("default.json".to_string(), json.to_string());

        let result = format.import_dataset(&files).unwrap();

        assert_eq!(result.categories.len(), 2);
        assert!(result.annotations.contains_key("test"));

        let store = &result.annotations["test"];
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_datumaro_roundtrip() {
        let format = DatumaroFormat::new();
        let original_store = create_test_store();
        let info = ImageInfo::new("roundtrip.jpg", 640, 480);

        // Export
        let export_result = format
            .export_dataset(&[(info.clone(), &original_store)])
            .unwrap();
        let json = &export_result.files["default.json"];

        // Import
        let mut files = HashMap::new();
        files.insert("default.json".to_string(), json.clone());
        let import_result = format.import_dataset(&files).unwrap();

        // Verify
        let imported_store = &import_result.annotations["roundtrip"];
        assert_eq!(imported_store.len(), original_store.len());
    }

    #[test]
    fn test_supports_all_shapes() {
        let format = DatumaroFormat::new();
        assert!(format.supports_shape(&Shape::BoundingBox(BoundingBox::new(0.0, 0.0, 10.0, 10.0))));
        assert!(format.supports_shape(&Shape::Polygon(Polygon::new())));
        assert!(format.supports_shape(&Shape::Point(Point::new(0.0, 0.0))));
    }

    #[test]
    fn test_preserves_attributes() {
        let format = DatumaroFormat::new();
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(0, "object"));

        // Add annotation with attribute via the builder
        let id = store.add(0, Shape::Point(Point::new(100.0, 100.0)));
        if let Some(ann) = store.get_mut(id) {
            ann.attributes.insert("confidence".to_string(), "0.95".to_string());
        }

        let info = ImageInfo::new("test.jpg", 640, 480);
        let export_result = format.export_dataset(&[(info, &store)]).unwrap();
        let json = &export_result.files["default.json"];

        assert!(json.contains("confidence"));
        assert!(json.contains("0.95"));
    }
}
