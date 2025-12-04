//! Annotation format import/export module.
//!
//! This module provides support for various annotation formats commonly used
//! in machine learning and computer vision:
//!
//! - **COCO**: Microsoft COCO JSON format (single JSON for entire dataset)
//! - **YOLO**: Ultralytics YOLO format (one `.txt` per image + `classes.txt`)
//! - **Datumaro**: Intel Datumaro format (JSON-based, multi-purpose)
//! - **Pascal VOC**: Pascal Visual Object Classes XML format
//!
//! # Architecture
//!
//! All formats implement the [`AnnotationFormat`] trait, which provides a
//! dataset-oriented API for import/export. This means:
//!
//! - Export takes all images at once and produces the format's output files
//! - Import takes the format's files and produces per-image annotations
//!
//! File I/O is handled by the caller (making WASM compatible), while this
//! module only handles string↔annotation conversion.
//!
//! # Example
//!
//! ```ignore
//! use hvat::formats::{AnnotationFormat, YoloFormat, ImageInfo};
//!
//! // Export to YOLO format
//! let format = YoloFormat::detection();
//! let images = vec![
//!     (ImageInfo::new("img1.jpg", 640, 480), &store1),
//!     (ImageInfo::new("img2.jpg", 640, 480), &store2),
//! ];
//! let result = format.export_dataset(&images)?;
//!
//! // result.files contains: "classes.txt", "img1.txt", "img2.txt"
//! ```

mod common;
mod error;

// Format implementations
mod coco;
mod datumaro;
mod voc;
mod yolo;

// Public API
pub use common::ImageInfo;
pub use error::FormatError;

pub use coco::CocoFormat;
pub use datumaro::DatumaroFormat;
pub use voc::VocFormat;
pub use yolo::YoloFormat;

use crate::{AnnotationStore, Category, Shape};
use std::collections::HashMap;

/// Result of exporting annotations to a format.
#[derive(Debug, Clone)]
pub struct ExportResult {
    /// Filename → file content.
    ///
    /// For dataset formats (COCO, Datumaro): typically one file.
    /// For per-image formats (YOLO, VOC): one file per image + metadata files.
    pub files: HashMap<String, String>,
    /// Warnings encountered during export (skipped shapes, etc.).
    pub warnings: Vec<String>,
}

impl ExportResult {
    /// Create a new empty export result.
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    /// Add a file to the export result.
    pub fn add_file(&mut self, name: impl Into<String>, content: impl Into<String>) {
        self.files.insert(name.into(), content.into());
    }

    /// Add a warning to the export result.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Check if the export produced any files.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

impl Default for ExportResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of importing annotations from a format.
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// Per-image annotations (keyed by image filename).
    pub annotations: HashMap<String, AnnotationStore>,
    /// Merged categories from all images.
    pub categories: Vec<Category>,
    /// Warnings encountered during import.
    pub warnings: Vec<String>,
}

impl ImportResult {
    /// Create a new empty import result.
    pub fn new() -> Self {
        Self {
            annotations: HashMap::new(),
            categories: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add annotations for an image.
    pub fn add_annotations(&mut self, image_name: impl Into<String>, store: AnnotationStore) {
        self.annotations.insert(image_name.into(), store);
    }

    /// Add a category.
    pub fn add_category(&mut self, category: Category) {
        // Avoid duplicates
        if !self.categories.iter().any(|c| c.id == category.id) {
            self.categories.push(category);
        }
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Get the total number of annotations across all images.
    pub fn total_annotations(&self) -> usize {
        self.annotations.values().map(|s| s.len()).sum()
    }
}

impl Default for ImportResult {
    fn default() -> Self {
        Self::new()
    }
}

/// A format that can import/export annotations.
///
/// Formats operate on datasets (multiple images) to support:
/// - Shared category lists (YOLO's `classes.txt`, COCO's categories array)
/// - Per-image annotation files (YOLO `.txt`, VOC `.xml`)
/// - Single-file datasets (COCO JSON, Datumaro JSON)
pub trait AnnotationFormat {
    /// Human-readable name of the format (e.g., "YOLO", "COCO", "Pascal VOC").
    fn name(&self) -> &'static str;

    /// File extension(s) this format uses (e.g., ["json"] or ["txt"]).
    fn extensions(&self) -> &[&'static str];

    /// Whether this format supports the given shape type.
    fn supports_shape(&self, shape: &Shape) -> bool;

    /// Export annotations for multiple images.
    ///
    /// # Arguments
    /// * `stores` - List of (image info, annotation store) pairs
    ///
    /// # Returns
    /// An `ExportResult` containing all output files and any warnings.
    fn export_dataset(
        &self,
        stores: &[(ImageInfo, &AnnotationStore)],
    ) -> Result<ExportResult, FormatError>;

    /// Import annotations from format files.
    ///
    /// # Arguments
    /// * `files` - Map of filename → file content
    ///
    /// # Returns
    /// An `ImportResult` containing per-image annotations and categories.
    fn import_dataset(
        &self,
        files: &HashMap<String, String>,
    ) -> Result<ImportResult, FormatError>;
}

/// Get a list of all available format names.
pub fn available_formats() -> Vec<&'static str> {
    vec!["COCO", "YOLO", "YOLO Segmentation", "Datumaro", "Pascal VOC"]
}

/// Create a format by name.
pub fn format_by_name(name: &str) -> Option<Box<dyn AnnotationFormat>> {
    match name.to_lowercase().as_str() {
        "coco" => Some(Box::new(CocoFormat::new())),
        "yolo" => Some(Box::new(YoloFormat::detection())),
        "yolo segmentation" | "yolo-seg" => Some(Box::new(YoloFormat::segmentation())),
        "datumaro" => Some(Box::new(DatumaroFormat::new())),
        "pascal voc" | "voc" => Some(Box::new(VocFormat::new())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_result() {
        let mut result = ExportResult::new();
        assert!(result.is_empty());

        result.add_file("test.txt", "content");
        result.add_warning("test warning");

        assert!(!result.is_empty());
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_import_result() {
        let mut result = ImportResult::new();
        result.add_category(Category::new(0, "test"));
        result.add_annotations("img.jpg", AnnotationStore::new());

        assert_eq!(result.categories.len(), 1);
        assert_eq!(result.annotations.len(), 1);
    }

    #[test]
    fn test_format_by_name() {
        assert!(format_by_name("coco").is_some());
        assert!(format_by_name("COCO").is_some());
        assert!(format_by_name("yolo").is_some());
        assert!(format_by_name("datumaro").is_some());
        assert!(format_by_name("pascal voc").is_some());
        assert!(format_by_name("voc").is_some());
        assert!(format_by_name("unknown").is_none());
    }

    #[test]
    fn test_available_formats() {
        let formats = available_formats();
        assert!(formats.contains(&"COCO"));
        assert!(formats.contains(&"YOLO"));
        assert!(formats.contains(&"Datumaro"));
        assert!(formats.contains(&"Pascal VOC"));
    }
}
