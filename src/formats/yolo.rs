//! YOLO format support.
//!
//! YOLO uses one `.txt` file per image plus a `classes.txt` file for category names.
//!
//! # Detection Format (Bounding Boxes)
//!
//! Each line in the label file:
//! ```text
//! <class_id> <x_center> <y_center> <width> <height>
//! ```
//!
//! All coordinates are normalized to [0, 1] relative to image size.
//!
//! # Segmentation Format (Polygons)
//!
//! Each line in the label file:
//! ```text
//! <class_id> <x1> <y1> <x2> <y2> ... <xn> <yn>
//! ```
//!
//! Polygon coordinates are also normalized to [0, 1].

use super::{
    common::{bbox_to_yolo, normalize_polygon, yolo_to_bbox, denormalize_polygon, ImageInfo},
    AnnotationFormat, ExportResult, FormatError, ImportResult,
};
use crate::{AnnotationStore, Category, Shape};
use std::collections::HashMap;

/// YOLO format variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YoloVariant {
    /// Standard detection format (bounding boxes only).
    Detection,
    /// Segmentation format (polygons and bounding boxes).
    Segmentation,
}

/// YOLO format implementation.
#[derive(Debug, Clone)]
pub struct YoloFormat {
    variant: YoloVariant,
}

impl YoloFormat {
    /// Create a YOLO detection format handler (bounding boxes only).
    pub fn detection() -> Self {
        Self {
            variant: YoloVariant::Detection,
        }
    }

    /// Create a YOLO segmentation format handler (polygons supported).
    pub fn segmentation() -> Self {
        Self {
            variant: YoloVariant::Segmentation,
        }
    }

    /// Get the current variant.
    pub fn variant(&self) -> YoloVariant {
        self.variant
    }
}

impl AnnotationFormat for YoloFormat {
    fn name(&self) -> &'static str {
        match self.variant {
            YoloVariant::Detection => "YOLO",
            YoloVariant::Segmentation => "YOLO Segmentation",
        }
    }

    fn extensions(&self) -> &[&'static str] {
        &["txt"]
    }

    fn supports_shape(&self, shape: &Shape) -> bool {
        match (self.variant, shape) {
            (_, Shape::BoundingBox(_)) => true,
            (YoloVariant::Segmentation, Shape::Polygon(_)) => true,
            (YoloVariant::Detection, Shape::Polygon(_)) => false, // Will convert to bbox
            (_, Shape::Point(_)) => false,
        }
    }

    fn export_dataset(
        &self,
        stores: &[(ImageInfo, &AnnotationStore)],
    ) -> Result<ExportResult, FormatError> {
        let mut result = ExportResult::new();

        // Collect all categories across all stores
        let mut category_map: HashMap<u32, String> = HashMap::new();
        for (_, store) in stores {
            for cat in store.categories() {
                category_map.entry(cat.id).or_insert_with(|| cat.name.clone());
            }
        }

        // Generate classes.txt
        let mut class_ids: Vec<u32> = category_map.keys().copied().collect();
        class_ids.sort();
        let classes_content: String = class_ids
            .iter()
            .filter_map(|id| category_map.get(id))
            .map(|name| name.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        result.add_file("classes.txt", classes_content);

        // Build class index map (category_id -> line index in classes.txt)
        let class_index: HashMap<u32, usize> = class_ids
            .iter()
            .enumerate()
            .map(|(idx, &id)| (id, idx))
            .collect();

        // Process each image
        for (info, store) in stores {
            let mut lines: Vec<String> = Vec::new();

            for ann in store.iter() {
                let class_idx = match class_index.get(&ann.category_id) {
                    Some(&idx) => idx,
                    None => {
                        result.add_warning(format!(
                            "Skipped annotation {} (unknown category {})",
                            ann.id, ann.category_id
                        ));
                        continue;
                    }
                };

                match &ann.shape {
                    Shape::BoundingBox(bbox) => {
                        let (x, y, w, h) = bbox_to_yolo(bbox, info.width, info.height);
                        lines.push(format!("{} {:.6} {:.6} {:.6} {:.6}", class_idx, x, y, w, h));
                    }
                    Shape::Polygon(poly) => {
                        if self.variant == YoloVariant::Segmentation {
                            let coords = normalize_polygon(poly, info.width, info.height);
                            let coord_str: String = coords
                                .iter()
                                .map(|(x, y)| format!("{:.6} {:.6}", x, y))
                                .collect::<Vec<_>>()
                                .join(" ");
                            lines.push(format!("{} {}", class_idx, coord_str));
                        } else {
                            // Convert polygon to bbox for detection format
                            if let Some(bbox) = poly.bounding_box() {
                                let (x, y, w, h) = bbox_to_yolo(&bbox, info.width, info.height);
                                lines.push(format!("{} {:.6} {:.6} {:.6} {:.6}", class_idx, x, y, w, h));
                                result.add_warning(format!(
                                    "Converted polygon annotation {} to bounding box",
                                    ann.id
                                ));
                            }
                        }
                    }
                    Shape::Point(_) => {
                        result.add_warning(format!(
                            "Skipped point annotation {} (YOLO doesn't support points)",
                            ann.id
                        ));
                    }
                }
            }

            // Write label file (use base name + .txt)
            let label_filename = format!("{}.txt", info.base_name());
            result.add_file(label_filename, lines.join("\n"));
        }

        Ok(result)
    }

    fn import_dataset(
        &self,
        files: &HashMap<String, String>,
    ) -> Result<ImportResult, FormatError> {
        let mut result = ImportResult::new();

        // Parse classes.txt
        let classes: Vec<String> = files
            .get("classes.txt")
            .map(|content| {
                content
                    .lines()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        // Create categories
        for (idx, name) in classes.iter().enumerate() {
            result.add_category(Category::new(idx as u32, name));
        }

        // If no classes.txt, we'll create categories on-the-fly
        let mut dynamic_categories: HashMap<usize, Category> = HashMap::new();

        // Process label files
        for (filename, content) in files {
            if filename == "classes.txt" || !filename.ends_with(".txt") {
                continue;
            }

            // Derive image filename (we don't know the extension, so store just the base)
            let base_name = filename.trim_end_matches(".txt");
            let mut store = AnnotationStore::new();

            // Add known categories to store
            for cat in &result.categories {
                store.add_category(cat.clone());
            }

            for (line_num, line) in content.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                // Parse class ID
                let class_idx: usize = match parts[0].parse() {
                    Ok(idx) => idx,
                    Err(_) => {
                        result.add_warning(format!(
                            "{}:{}: Invalid class ID '{}'",
                            filename, line_num + 1, parts[0]
                        ));
                        continue;
                    }
                };

                // Ensure category exists - use dynamic_categories to track what we've added
                let category_id = class_idx as u32;
                let is_new_category = !dynamic_categories.contains_key(&class_idx);
                let cat = dynamic_categories
                    .entry(class_idx)
                    .or_insert_with(|| Category::new(category_id, format!("class_{}", class_idx)));

                if is_new_category {
                    store.add_category(cat.clone());
                    result.add_category(cat.clone());
                } else if store.get_category(category_id).is_none() {
                    store.add_category(cat.clone());
                }

                // Parse coordinates
                let coords: Vec<f32> = parts[1..]
                    .iter()
                    .filter_map(|s| s.parse().ok())
                    .collect();

                let shape = if coords.len() == 4 {
                    // Bounding box format: x_center y_center width height
                    // We need image dimensions to denormalize, but we don't have them
                    // Store as normalized bbox (will need to be denormalized later with image info)
                    // For now, use 1.0 as placeholder dimensions
                    let bbox = yolo_to_bbox(coords[0], coords[1], coords[2], coords[3], 1, 1);
                    Some(Shape::BoundingBox(bbox))
                } else if coords.len() >= 6 && coords.len() % 2 == 0 {
                    // Polygon format: x1 y1 x2 y2 ... xn yn
                    let points: Vec<(f32, f32)> = coords
                        .chunks(2)
                        .map(|c| (c[0], c[1]))
                        .collect();
                    let poly = denormalize_polygon(&points, 1, 1);
                    Some(Shape::Polygon(poly))
                } else {
                    result.add_warning(format!(
                        "{}:{}: Invalid coordinate count ({})",
                        filename, line_num + 1, coords.len()
                    ));
                    None
                };

                if let Some(s) = shape {
                    store.add(category_id, s);
                }
            }

            // Store annotations with base name as key
            // Caller will need to match with actual image files
            result.add_annotations(base_name.to_string(), store);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundingBox, Point, Polygon};

    fn create_test_store() -> AnnotationStore {
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(0, "car"));
        store.add_category(Category::new(1, "person"));
        store.add(0, Shape::BoundingBox(BoundingBox::new(100.0, 100.0, 200.0, 100.0)));
        store.add(1, Shape::BoundingBox(BoundingBox::new(300.0, 200.0, 50.0, 100.0)));
        store
    }

    #[test]
    fn test_yolo_detection_export() {
        let format = YoloFormat::detection();
        let store = create_test_store();
        let info = ImageInfo::new("test.jpg", 640, 480);

        let result = format.export_dataset(&[(info, &store)]).unwrap();

        assert!(result.files.contains_key("classes.txt"));
        assert!(result.files.contains_key("test.txt"));

        let classes = &result.files["classes.txt"];
        assert!(classes.contains("car"));
        assert!(classes.contains("person"));

        let labels = &result.files["test.txt"];
        let lines: Vec<&str> = labels.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_yolo_segmentation_export() {
        let format = YoloFormat::segmentation();
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(0, "shape"));

        let mut poly = Polygon::new();
        poly.push(Point::new(100.0, 100.0));
        poly.push(Point::new(200.0, 100.0));
        poly.push(Point::new(150.0, 200.0));
        poly.close();
        store.add(0, Shape::Polygon(poly));

        let info = ImageInfo::new("test.jpg", 640, 480);
        let result = format.export_dataset(&[(info, &store)]).unwrap();

        let labels = &result.files["test.txt"];
        // Should have more than 5 values (class + 3 x,y pairs)
        let parts: Vec<&str> = labels.split_whitespace().collect();
        assert!(parts.len() >= 7); // class_id + 6 coords
    }

    #[test]
    fn test_yolo_detection_converts_polygon() {
        let format = YoloFormat::detection();
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(0, "shape"));

        let mut poly = Polygon::new();
        poly.push(Point::new(100.0, 100.0));
        poly.push(Point::new(200.0, 100.0));
        poly.push(Point::new(150.0, 200.0));
        poly.close();
        store.add(0, Shape::Polygon(poly));

        let info = ImageInfo::new("test.jpg", 640, 480);
        let result = format.export_dataset(&[(info, &store)]).unwrap();

        // Should have a warning about conversion
        assert!(result.warnings.iter().any(|w| w.contains("Converted")));

        // Should still have valid output (as bbox)
        let labels = &result.files["test.txt"];
        let parts: Vec<&str> = labels.split_whitespace().collect();
        assert_eq!(parts.len(), 5); // class_id + 4 bbox values
    }

    #[test]
    fn test_yolo_import() {
        let format = YoloFormat::detection();

        let classes = "car\nperson";
        let labels = "0 0.5 0.5 0.25 0.25\n1 0.75 0.75 0.1 0.2";

        let mut files = HashMap::new();
        files.insert("classes.txt".to_string(), classes.to_string());
        files.insert("image1.txt".to_string(), labels.to_string());

        let result = format.import_dataset(&files).unwrap();

        assert_eq!(result.categories.len(), 2);
        assert!(result.annotations.contains_key("image1"));

        let store = &result.annotations["image1"];
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_yolo_import_without_classes() {
        let format = YoloFormat::detection();

        let labels = "0 0.5 0.5 0.25 0.25\n1 0.75 0.75 0.1 0.2";

        let mut files = HashMap::new();
        files.insert("image1.txt".to_string(), labels.to_string());

        let result = format.import_dataset(&files).unwrap();

        // Should create dynamic categories
        assert_eq!(result.categories.len(), 2);
        assert!(result.categories.iter().any(|c| c.name == "class_0"));
        assert!(result.categories.iter().any(|c| c.name == "class_1"));
    }

    #[test]
    fn test_supports_shape() {
        let detection = YoloFormat::detection();
        let segmentation = YoloFormat::segmentation();

        // Both support bounding boxes
        assert!(detection.supports_shape(&Shape::BoundingBox(BoundingBox::new(0.0, 0.0, 10.0, 10.0))));
        assert!(segmentation.supports_shape(&Shape::BoundingBox(BoundingBox::new(0.0, 0.0, 10.0, 10.0))));

        // Only segmentation supports polygons
        assert!(!detection.supports_shape(&Shape::Polygon(Polygon::new())));
        assert!(segmentation.supports_shape(&Shape::Polygon(Polygon::new())));

        // Neither supports points
        assert!(!detection.supports_shape(&Shape::Point(Point::new(0.0, 0.0))));
        assert!(!segmentation.supports_shape(&Shape::Point(Point::new(0.0, 0.0))));
    }
}
