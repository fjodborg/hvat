//! Annotation format import/export system.
//!
//! This module provides a trait-based system for importing and exporting
//! annotations in various formats. The system is designed to be extensible,
//! allowing new formats to be added by implementing the `AnnotationFormat` trait.
//!
//! ## Supported Formats
//!
//! - **HVAT JSON**: Native format with full fidelity (all shapes, colors, tags)
//! - **COCO JSON**: Industry standard for object detection/segmentation
//! - **YOLO TXT**: Simple per-image format for bounding boxes
//! - **Pascal VOC XML**: Classic per-image XML format for bounding boxes
//!
//! ## Usage
//!
//! ```rust,ignore
//! use hvat::format::{FormatRegistry, ExportOptions};
//!
//! // Get the format registry
//! let registry = FormatRegistry::new();
//!
//! // Export in COCO format
//! let format = registry.get("coco").unwrap();
//! let result = format.export(&project_data, path, &ExportOptions::default())?;
//! ```

mod auto_save;
mod error;
pub mod formats;
mod project;
mod registry;
mod traits;

pub use auto_save::AutoSaveManager;
pub use error::FormatError;
pub use project::{
    AnnotationEntry, CategoryEntry, ImageEntry, ProjectData, ProjectMetadata, ShapeEntry,
};
pub use registry::FormatRegistry;
pub use traits::{
    AnnotationFormat, ExportOptions, ExportResult, FormatWarning, ImportOptions, WarningSeverity,
};
