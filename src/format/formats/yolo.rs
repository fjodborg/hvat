//! YOLO TXT format implementation.
//!
//! Implements the YOLO annotation format, which uses one text file per image
//! with normalized bounding box coordinates.

use std::collections::HashMap;
use std::path::Path;

use crate::format::error::FormatError;
use crate::format::project::{
    AnnotationEntry, CategoryEntry, ImageEntry, ProjectData, ProjectMetadata, ShapeEntry,
};
use crate::format::traits::{
    AnnotationFormat, ExportOptions, ExportResult, FormatWarning, ImportOptions, WarningSeverity,
};

/// YOLO TXT format.
///
/// Supports:
/// - Bounding boxes only (normalized coordinates)
/// - Per-image annotation files
/// - classes.txt for category names
///
/// Does not support:
/// - Polygons (skipped with warning)
/// - Points (skipped with warning)
/// - Per-image tags
/// - Category colors
pub struct YoloFormat;

impl AnnotationFormat for YoloFormat {
    fn id(&self) -> &'static str {
        "yolo"
    }

    fn display_name(&self) -> &'static str {
        "YOLO (TXT)"
    }

    fn extensions(&self) -> &[&'static str] {
        &["txt"]
    }

    fn supports_polygon(&self) -> bool {
        false
    }

    fn supports_point(&self) -> bool {
        false
    }

    fn supports_per_image(&self) -> bool {
        true
    }

    fn export(
        &self,
        data: &ProjectData,
        path: &Path,
        _options: &ExportOptions,
    ) -> Result<ExportResult, FormatError> {
        log::info!("Exporting YOLO annotations to {:?}", path);

        let output_dir = path;
        std::fs::create_dir_all(output_dir)?;

        let mut warnings = Vec::new();
        let mut files_created = Vec::new();
        let mut annotations_exported = 0;

        // Write classes.txt
        let classes_path = output_dir.join("classes.txt");
        let classes_content: String = data
            .categories
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&classes_path, &classes_content)?;
        files_created.push(classes_path);

        // Build category_id -> index map
        let cat_to_idx: HashMap<u32, usize> = data
            .categories
            .iter()
            .enumerate()
            .map(|(idx, c)| (c.id, idx))
            .collect();

        // Write per-image annotation files
        for image in &data.images {
            let (width, height) = match image.dimensions {
                Some((w, h)) if w > 0 && h > 0 => (w as f32, h as f32),
                _ => {
                    warnings.push(
                        FormatWarning::error(format!(
                            "Skipping image '{}': dimensions required for YOLO format",
                            image.filename
                        ))
                        .with_image(&image.path),
                    );
                    continue;
                }
            };

            let stem = Path::new(&image.filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let txt_path = output_dir.join(format!("{}.txt", stem));

            let mut lines = Vec::new();
            for ann in &image.annotations {
                match &ann.shape {
                    ShapeEntry::BoundingBox {
                        x,
                        y,
                        width: w,
                        height: h,
                    } => {
                        let class_idx = match cat_to_idx.get(&ann.category_id) {
                            Some(&idx) => idx,
                            None => {
                                warnings.push(
                                    FormatWarning::warning(format!(
                                        "Unknown category ID {}, skipping annotation",
                                        ann.category_id
                                    ))
                                    .with_image(&image.path),
                                );
                                continue;
                            }
                        };

                        // Convert to YOLO normalized format
                        // YOLO uses center coordinates, normalized to [0, 1]
                        let cx = (x + w / 2.0) / width;
                        let cy = (y + h / 2.0) / height;
                        let nw = w / width;
                        let nh = h / height;

                        lines.push(format!(
                            "{} {:.6} {:.6} {:.6} {:.6}",
                            class_idx, cx, cy, nw, nh
                        ));
                        annotations_exported += 1;
                    }
                    ShapeEntry::Point { .. } => {
                        warnings.push(
                            FormatWarning::warning(
                                "Skipped point annotation (YOLO only supports bounding boxes)",
                            )
                            .with_image(&image.path),
                        );
                    }
                    ShapeEntry::Polygon { .. } => {
                        warnings.push(
                            FormatWarning::warning(
                                "Skipped polygon annotation (YOLO only supports bounding boxes)",
                            )
                            .with_image(&image.path),
                        );
                    }
                }
            }

            std::fs::write(&txt_path, lines.join("\n"))?;
            files_created.push(txt_path);
        }

        log::info!(
            "Exported {} images with {} annotations ({} warnings)",
            data.images.len(),
            annotations_exported,
            warnings.len()
        );

        Ok(ExportResult {
            images_exported: data.images.len(),
            annotations_exported,
            warnings,
            files_created,
        })
    }

    fn import(&self, path: &Path, options: &ImportOptions) -> Result<ProjectData, FormatError> {
        log::info!("Importing YOLO annotations from {:?}", path);

        let input_dir = path;
        if !input_dir.is_dir() {
            return Err(FormatError::invalid_format(
                "YOLO import requires a directory path",
            ));
        }

        let mut data = ProjectData::new();
        data.folder = input_dir.to_path_buf();

        // Read classes.txt
        let classes_path = input_dir.join("classes.txt");
        if classes_path.exists() {
            let content = std::fs::read_to_string(&classes_path)?;
            for (idx, line) in content.lines().enumerate() {
                let name = line.trim();
                if !name.is_empty() {
                    data.categories.push(CategoryEntry::new(idx as u32, name));
                }
            }
        }

        // Find all .txt files (excluding classes.txt)
        let txt_files: Vec<_> = std::fs::read_dir(input_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().map_or(false, |ext| ext == "txt")
                    && p.file_name().map_or(false, |n| n != "classes.txt")
            })
            .collect();

        // Import each annotation file
        for txt_path in txt_files {
            let stem = txt_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

            // Try to find corresponding image
            let image_path = find_image_for_stem(input_dir, stem);
            let mut entry = ImageEntry::new(image_path.clone());
            entry.filename = image_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(stem)
                .to_string();

            // Parse annotations
            let content = std::fs::read_to_string(&txt_path)?;
            let mut ann_id = 0u32;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if let Some(ann) = parse_yolo_line(line, ann_id) {
                    entry.annotations.push(ann);
                    ann_id += 1;
                }
            }

            data.images.push(entry);
        }

        data.metadata = ProjectMetadata::new();
        data.metadata.extra.insert(
            "imported_from".into(),
            serde_json::Value::String("yolo".into()),
        );
        data.metadata.extra.insert(
            "note".into(),
            serde_json::Value::String(
                "YOLO coordinates are normalized; image dimensions needed for pixel values".into(),
            ),
        );

        log::info!(
            "Imported {} images with {} annotations",
            data.images.len(),
            data.total_annotations()
        );

        Ok(data)
    }
}

/// Parse a single YOLO annotation line.
fn parse_yolo_line(line: &str, id: u32) -> Option<AnnotationEntry> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    let class_id: u32 = parts[0].parse().ok()?;
    let cx: f32 = parts[1].parse().ok()?;
    let cy: f32 = parts[2].parse().ok()?;
    let w: f32 = parts[3].parse().ok()?;
    let h: f32 = parts[4].parse().ok()?;

    // YOLO stores center coordinates; convert to top-left
    // Note: These are normalized values [0, 1]; caller needs dimensions to convert
    let x = cx - w / 2.0;
    let y = cy - h / 2.0;

    Some(AnnotationEntry::new(
        id,
        class_id,
        ShapeEntry::BoundingBox {
            x,
            y,
            width: w,
            height: h,
        },
    ))
}

/// Find an image file matching the given stem in the directory.
fn find_image_for_stem(dir: &Path, stem: &str) -> std::path::PathBuf {
    const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"];

    for ext in IMAGE_EXTENSIONS {
        let path = dir.join(format!("{}.{}", stem, ext));
        if path.exists() {
            return path;
        }
    }

    // Fall back to a .png path even if it doesn't exist
    dir.join(format!("{}.png", stem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yolo_line() {
        let line = "0 0.5 0.5 0.2 0.3";
        let ann = parse_yolo_line(line, 1).unwrap();

        assert_eq!(ann.id, 1);
        assert_eq!(ann.category_id, 0);

        match ann.shape {
            ShapeEntry::BoundingBox {
                x,
                y,
                width,
                height,
            } => {
                assert!((x - 0.4).abs() < 0.001);
                assert!((y - 0.35).abs() < 0.001);
                assert!((width - 0.2).abs() < 0.001);
                assert!((height - 0.3).abs() < 0.001);
            }
            _ => panic!("Expected bounding box"),
        }
    }

    #[test]
    fn test_format_metadata() {
        let format = YoloFormat;
        assert_eq!(format.id(), "yolo");
        assert!(!format.supports_polygon());
        assert!(!format.supports_point());
        assert!(format.supports_per_image());
    }
}
