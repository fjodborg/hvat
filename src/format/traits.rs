//! Trait definitions for annotation format implementations.

use std::path::{Path, PathBuf};

use crate::format::error::FormatError;
use crate::format::project::ProjectData;

/// Trait for annotation format import/export implementations.
///
/// Each format (HVAT JSON, COCO, YOLO, Pascal VOC) implements this trait
/// to provide bidirectional conversion between HVAT's internal representation
/// and the external format.
pub trait AnnotationFormat: Send + Sync {
    /// Unique identifier for this format (e.g., "hvat", "coco", "yolo", "voc").
    fn id(&self) -> &'static str;

    /// Human-readable name for UI display.
    fn display_name(&self) -> &'static str;

    /// File extensions this format uses (e.g., `["json"]` for COCO).
    fn extensions(&self) -> &[&'static str];

    /// Whether this format supports polygon annotations.
    fn supports_polygon(&self) -> bool;

    /// Whether this format supports point annotations.
    fn supports_point(&self) -> bool;

    /// Whether this format supports per-image export (vs single project file).
    fn supports_per_image(&self) -> bool;

    /// Export project data to the specified path.
    ///
    /// For single-file formats, `path` is the output file.
    /// For per-image formats, `path` is the output directory.
    fn export(
        &self,
        data: &ProjectData,
        path: &Path,
        options: &ExportOptions,
    ) -> Result<ExportResult, FormatError>;

    /// Import project data from the specified path.
    ///
    /// For single-file formats, `path` is the input file.
    /// For per-image formats, `path` is the input directory.
    fn import(&self, path: &Path, options: &ImportOptions) -> Result<ProjectData, FormatError>;
}

/// Options for export operations.
#[derive(Debug, Clone, Default)]
pub struct ExportOptions {
    /// Whether to export as per-image files (if format supports it).
    pub per_image: bool,

    /// Base path for relative image references.
    pub image_base_path: Option<PathBuf>,

    /// Whether to include tags in export (if format supports it).
    pub include_tags: bool,

    /// Whether to include category colors (if format supports it).
    pub include_colors: bool,
}

impl ExportOptions {
    /// Create new export options with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set per-image export mode.
    pub fn per_image(mut self, per_image: bool) -> Self {
        self.per_image = per_image;
        self
    }

    /// Set base path for relative image references.
    pub fn image_base_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.image_base_path = Some(path.into());
        self
    }

    /// Include tags in export.
    pub fn include_tags(mut self, include: bool) -> Self {
        self.include_tags = include;
        self
    }

    /// Include category colors in export.
    pub fn include_colors(mut self, include: bool) -> Self {
        self.include_colors = include;
        self
    }
}

/// Options for import operations.
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Base path to resolve relative image paths.
    pub image_base_path: Option<PathBuf>,

    /// Whether to merge with existing data or replace.
    pub merge: bool,

    /// Filter to specific category names (empty = all).
    pub category_filter: Vec<String>,
}

impl ImportOptions {
    /// Create new import options with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set base path for resolving relative image paths.
    pub fn image_base_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.image_base_path = Some(path.into());
        self
    }

    /// Set merge mode (true = merge with existing, false = replace).
    pub fn merge(mut self, merge: bool) -> Self {
        self.merge = merge;
        self
    }

    /// Filter to specific category names.
    pub fn category_filter(mut self, categories: Vec<String>) -> Self {
        self.category_filter = categories;
        self
    }
}

/// Result of an export operation.
#[derive(Debug, Default)]
pub struct ExportResult {
    /// Number of images exported.
    pub images_exported: usize,

    /// Number of annotations exported.
    pub annotations_exported: usize,

    /// Warnings generated during export (e.g., skipped shapes).
    pub warnings: Vec<FormatWarning>,

    /// Files created during export.
    pub files_created: Vec<PathBuf>,
}

impl ExportResult {
    /// Create a new export result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a warning to the result.
    pub fn add_warning(&mut self, warning: FormatWarning) {
        self.warnings.push(warning);
    }

    /// Check if there were any warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Check if there were any errors (severe warnings).
    pub fn has_errors(&self) -> bool {
        self.warnings
            .iter()
            .any(|w| matches!(w.severity, WarningSeverity::Error))
    }
}

/// Warning generated during format conversion.
#[derive(Debug, Clone)]
pub struct FormatWarning {
    /// Path of the image this warning relates to (if applicable).
    pub image_path: Option<PathBuf>,

    /// Human-readable warning message.
    pub message: String,

    /// Severity level of the warning.
    pub severity: WarningSeverity,
}

impl FormatWarning {
    /// Create a new warning.
    pub fn new(message: impl Into<String>, severity: WarningSeverity) -> Self {
        Self {
            image_path: None,
            message: message.into(),
            severity,
        }
    }

    /// Create an info-level warning.
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, WarningSeverity::Info)
    }

    /// Create a warning-level warning.
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, WarningSeverity::Warning)
    }

    /// Create an error-level warning.
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, WarningSeverity::Error)
    }

    /// Set the image path this warning relates to.
    pub fn with_image(mut self, path: impl Into<PathBuf>) -> Self {
        self.image_path = Some(path.into());
        self
    }
}

/// Severity level for format warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningSeverity {
    /// Informational message, not a problem.
    Info,
    /// Warning that something was skipped or modified.
    Warning,
    /// Error that may affect data integrity.
    Error,
}
