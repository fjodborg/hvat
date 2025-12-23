//! COCO JSON format implementation.
//!
//! Implements the COCO (Common Objects in Context) annotation format,
//! which is widely used for object detection and segmentation tasks.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::format::error::FormatError;
use crate::format::project::{
    AnnotationEntry, CategoryEntry, ImageEntry, ProjectData, ProjectMetadata, ShapeEntry,
};
use crate::format::traits::{
    AnnotationFormat, ExportOptions, ExportResult, FormatWarning, ImportOptions,
};

/// COCO JSON format.
///
/// Supports:
/// - Bounding boxes (bbox)
/// - Polygons (segmentation)
/// - Points (as single-point segmentation)
/// - Categories with supercategories
///
/// Does not support:
/// - Per-image tags (COCO doesn't have this concept)
/// - Category colors (not part of standard COCO)
pub struct CocoFormat;

impl AnnotationFormat for CocoFormat {
    fn id(&self) -> &'static str {
        "coco"
    }

    fn display_name(&self) -> &'static str {
        "COCO (JSON)"
    }

    fn extensions(&self) -> &[&'static str] {
        &["json"]
    }

    fn supports_polygon(&self) -> bool {
        true
    }

    fn supports_point(&self) -> bool {
        true
    }

    fn supports_per_image(&self) -> bool {
        false
    }

    fn export(
        &self,
        data: &ProjectData,
        path: &Path,
        options: &ExportOptions,
    ) -> Result<ExportResult, FormatError> {
        log::info!("Exporting COCO annotations to {:?}", path);

        let (bytes, mut result) = self.export_to_bytes(data, options)?;
        std::fs::write(path, &bytes)?;
        result.files_created = vec![path.to_path_buf()];

        log::info!(
            "Exported {} images with {} annotations ({} warnings)",
            result.images_exported,
            result.annotations_exported,
            result.warnings.len()
        );

        Ok(result)
    }

    fn export_to_bytes(
        &self,
        data: &ProjectData,
        _options: &ExportOptions,
    ) -> Result<(Vec<u8>, ExportResult), FormatError> {
        log::info!("Exporting COCO annotations to bytes");

        let mut warnings = Vec::new();
        let mut coco = CocoDataset::new();

        // Convert categories
        for cat in &data.categories {
            coco.categories.push(CocoCategory {
                id: cat.id,
                name: cat.name.clone(),
                supercategory: cat.supercategory.clone().unwrap_or_else(|| "none".into()),
            });
        }

        // Convert images and annotations
        let mut annotation_id = 1u64;
        let mut annotations_exported = 0;

        for (img_idx, image) in data.images.iter().enumerate() {
            let image_id = (img_idx + 1) as u64;
            let (width, height) = image.dimensions.unwrap_or((0, 0));

            if width == 0 || height == 0 {
                warnings.push(
                    FormatWarning::warning(format!(
                        "Image '{}' has no dimensions, area calculations may be incorrect",
                        image.filename
                    ))
                    .with_image(&image.path),
                );
            }

            // Compute relative path from project folder to preserve structure
            let relative_path = if !data.folder.as_os_str().is_empty() {
                image
                    .path
                    .strip_prefix(&data.folder)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| image.path.to_string_lossy().to_string())
            } else {
                // When folder is empty (e.g., WASM), use the path directly
                // It's already a relative path in WASM case
                image.path.to_string_lossy().to_string()
            };
            // Normalize path separators for cross-platform compatibility
            let file_name = relative_path.replace('\\', "/");

            coco.images.push(CocoImage {
                id: image_id,
                file_name,
                width,
                height,
                license: None,
            });

            for ann in &image.annotations {
                match self.convert_annotation(ann, image_id, annotation_id, &mut warnings) {
                    Ok(coco_ann) => {
                        coco.annotations.push(coco_ann);
                        annotation_id += 1;
                        annotations_exported += 1;
                    }
                    Err(e) => {
                        warnings.push(
                            FormatWarning::error(format!("Failed to convert annotation: {}", e))
                                .with_image(&image.path),
                        );
                    }
                }
            }
        }

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&coco)?;

        log::info!(
            "Exported {} images with {} annotations ({} warnings)",
            data.images.len(),
            annotations_exported,
            warnings.len()
        );

        Ok((
            json.into_bytes(),
            ExportResult {
                images_exported: data.images.len(),
                annotations_exported,
                warnings,
                files_created: Vec::new(),
            },
        ))
    }

    fn import(&self, path: &Path, options: &ImportOptions) -> Result<ProjectData, FormatError> {
        log::info!("Importing COCO annotations from {:?}", path);

        let json = std::fs::read_to_string(path)?;
        let coco: CocoDataset = serde_json::from_str(&json)?;

        let mut data = ProjectData::new();

        // Set folder from options or derive from file path
        if let Some(ref base) = options.image_base_path {
            data.folder = base.clone();
        } else if let Some(parent) = path.parent() {
            data.folder = parent.to_path_buf();
        }

        for cat in &coco.categories {
            let mut entry = CategoryEntry::new(cat.id, &cat.name);
            if cat.supercategory != "none" {
                entry = entry.with_supercategory(&cat.supercategory);
            }
            data.categories.push(entry);
        }

        // Build image ID to index map
        let image_map: HashMap<u64, usize> = coco
            .images
            .iter()
            .enumerate()
            .map(|(idx, img)| (img.id, idx))
            .collect();

        // Convert images
        for coco_img in &coco.images {
            let path = data.folder.join(&coco_img.file_name);
            let mut entry = ImageEntry::new(path);
            entry.filename = coco_img.file_name.clone();
            if coco_img.width > 0 && coco_img.height > 0 {
                entry = entry.with_dimensions(coco_img.width, coco_img.height);
            }
            data.images.push(entry);
        }

        // Convert annotations
        for coco_ann in &coco.annotations {
            if let Some(&img_idx) = image_map.get(&coco_ann.image_id) {
                if let Some(shape) = self.convert_coco_annotation(coco_ann) {
                    let entry =
                        AnnotationEntry::new(coco_ann.id as u32, coco_ann.category_id, shape);
                    data.images[img_idx].annotations.push(entry);
                }
            }
        }

        data.metadata = ProjectMetadata::new();
        data.metadata.extra.insert(
            "imported_from".into(),
            serde_json::Value::String("coco".into()),
        );

        log::info!(
            "Imported {} images with {} annotations",
            data.images.len(),
            data.total_annotations()
        );

        Ok(data)
    }
}

impl CocoFormat {
    /// Convert an annotation entry to COCO format.
    fn convert_annotation(
        &self,
        ann: &AnnotationEntry,
        image_id: u64,
        annotation_id: u64,
        _warnings: &mut Vec<FormatWarning>,
    ) -> Result<CocoAnnotation, FormatError> {
        let (bbox, segmentation, area) = match &ann.shape {
            ShapeEntry::BoundingBox {
                x,
                y,
                width,
                height,
            } => {
                let bbox = Some([*x, *y, *width, *height]);
                let area = width * height;
                (bbox, None, area)
            }
            ShapeEntry::Point { x, y } => {
                // Represent point as a small polygon for COCO compatibility
                let seg = vec![vec![*x, *y]];
                (None, Some(seg), 0.0)
            }
            ShapeEntry::Polygon { vertices } => {
                // Convert vertices to flat array [x1, y1, x2, y2, ...]
                let flat: Vec<f32> = vertices.iter().flat_map(|(x, y)| [*x, *y]).collect();
                let area = polygon_area(vertices);
                let bbox = polygon_bbox(vertices);
                (bbox, Some(vec![flat]), area)
            }
        };

        Ok(CocoAnnotation {
            id: annotation_id,
            image_id,
            category_id: ann.category_id,
            bbox,
            segmentation,
            area,
            iscrowd: 0,
        })
    }

    /// Convert a COCO annotation to a shape entry.
    fn convert_coco_annotation(&self, ann: &CocoAnnotation) -> Option<ShapeEntry> {
        // Prefer segmentation if available
        if let Some(ref seg) = ann.segmentation {
            if let Some(first_seg) = seg.first() {
                if first_seg.len() == 2 {
                    // Single point
                    return Some(ShapeEntry::Point {
                        x: first_seg[0],
                        y: first_seg[1],
                    });
                } else if first_seg.len() >= 6 {
                    // Polygon (at least 3 vertices)
                    let vertices: Vec<(f32, f32)> = first_seg
                        .chunks(2)
                        .filter_map(|chunk| {
                            if chunk.len() == 2 {
                                Some((chunk[0], chunk[1]))
                            } else {
                                None
                            }
                        })
                        .collect();
                    return Some(ShapeEntry::Polygon { vertices });
                }
            }
        }

        // Fall back to bounding box
        if let Some(bbox) = ann.bbox {
            return Some(ShapeEntry::BoundingBox {
                x: bbox[0],
                y: bbox[1],
                width: bbox[2],
                height: bbox[3],
            });
        }

        None
    }
}

/// Calculate the area of a polygon using the shoelace formula.
fn polygon_area(vertices: &[(f32, f32)]) -> f32 {
    if vertices.len() < 3 {
        return 0.0;
    }

    let mut area = 0.0;
    let n = vertices.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += vertices[i].0 * vertices[j].1;
        area -= vertices[j].0 * vertices[i].1;
    }
    (area / 2.0).abs()
}

/// Calculate the bounding box of a polygon.
fn polygon_bbox(vertices: &[(f32, f32)]) -> Option<[f32; 4]> {
    if vertices.is_empty() {
        return None;
    }

    let min_x = vertices.iter().map(|(x, _)| *x).fold(f32::MAX, f32::min);
    let max_x = vertices.iter().map(|(x, _)| *x).fold(f32::MIN, f32::max);
    let min_y = vertices.iter().map(|(_, y)| *y).fold(f32::MAX, f32::min);
    let max_y = vertices.iter().map(|(_, y)| *y).fold(f32::MIN, f32::max);

    Some([min_x, min_y, max_x - min_x, max_y - min_y])
}

// COCO format structures

#[derive(Debug, Serialize, Deserialize)]
struct CocoDataset {
    info: CocoInfo,
    images: Vec<CocoImage>,
    annotations: Vec<CocoAnnotation>,
    categories: Vec<CocoCategory>,
    #[serde(default)]
    licenses: Vec<CocoLicense>,
}

impl CocoDataset {
    fn new() -> Self {
        Self {
            info: CocoInfo::default(),
            images: Vec::new(),
            annotations: Vec::new(),
            categories: Vec::new(),
            licenses: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CocoInfo {
    #[serde(default)]
    year: u32,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    contributor: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    date_created: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CocoImage {
    id: u64,
    file_name: String,
    width: u32,
    height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CocoAnnotation {
    id: u64,
    image_id: u64,
    category_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    bbox: Option<[f32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    segmentation: Option<Vec<Vec<f32>>>,
    area: f32,
    iscrowd: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct CocoCategory {
    id: u32,
    name: String,
    supercategory: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CocoLicense {
    id: u32,
    name: String,
    url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polygon_area() {
        // Unit square
        let square = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert!((polygon_area(&square) - 1.0).abs() < 0.001);

        // Triangle
        let triangle = vec![(0.0, 0.0), (2.0, 0.0), (1.0, 2.0)];
        assert!((polygon_area(&triangle) - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_polygon_bbox() {
        let polygon = vec![(10.0, 20.0), (50.0, 20.0), (50.0, 80.0), (10.0, 80.0)];
        let bbox = polygon_bbox(&polygon).unwrap();
        assert_eq!(bbox[0], 10.0); // x
        assert_eq!(bbox[1], 20.0); // y
        assert_eq!(bbox[2], 40.0); // width
        assert_eq!(bbox[3], 60.0); // height
    }

    #[test]
    fn test_format_metadata() {
        let format = CocoFormat;
        assert_eq!(format.id(), "coco");
        assert!(format.supports_polygon());
        assert!(format.supports_point());
        assert!(!format.supports_per_image());
    }
}
