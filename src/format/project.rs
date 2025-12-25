//! Project data structures for import/export.
//!
//! This module defines the intermediate representation used by all format
//! converters. `ProjectData` serves as the common format that all annotation
//! formats convert to and from.
//!
//! # Versioning
//!
//! The HVAT format uses semantic versioning (MAJOR.MINOR.PATCH):
//!
//! - **Version 0.x.x**: Unstable development versions. The format may change
//!   in breaking ways between any 0.x releases. Do not rely on backwards
//!   compatibility for version 0 files.
//!
//! - **Version 1.x.x** (future): First stable release. Breaking changes will
//!   only occur in major version bumps (1.x -> 2.x). Minor versions add
//!   features in a backwards-compatible way.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::{Annotation, AnnotationShape, Category, Tag};
use crate::state::ImageData;

/// Complete project data for import/export.
///
/// This is the intermediate representation used by all format converters.
/// It contains all the information needed to fully represent a project's
/// annotations, categories, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectData {
    /// Format version for compatibility checking.
    pub version: String,

    /// Project folder path (may be empty for imported data).
    #[serde(default)]
    pub folder: PathBuf,

    /// List of images in the project with their annotations.
    pub images: Vec<ImageEntry>,

    /// Category definitions (for annotations).
    pub categories: Vec<CategoryEntry>,

    /// Tag definitions (for images).
    #[serde(default)]
    pub tags: Vec<TagEntry>,

    /// Project metadata (creation date, tool version, etc.).
    #[serde(default)]
    pub metadata: ProjectMetadata,
}

impl ProjectData {
    /// Current version of the project data format.
    ///
    /// Version 0.x.x indicates an unstable format that may change between releases.
    /// Do not rely on backwards compatibility for version 0 files.
    pub const CURRENT_VERSION: &'static str = "0.1.0";

    /// Major version number for compatibility checking.
    pub const VERSION_MAJOR: u32 = 0;

    /// Minor version number.
    pub const VERSION_MINOR: u32 = 1;

    /// Patch version number.
    pub const VERSION_PATCH: u32 = 0;

    /// Create a new empty project data structure.
    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION.to_string(),
            folder: PathBuf::new(),
            images: Vec::new(),
            categories: Vec::new(),
            tags: Vec::new(),
            metadata: ProjectMetadata::default(),
        }
    }

    /// Parse a version string into (major, minor, patch) components.
    ///
    /// Returns None if the version string is invalid.
    pub fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;
        Some((major, minor, patch))
    }

    /// Check if a version is compatible with the current version.
    ///
    /// For version 0.x.x (unstable), only exact minor version matches are compatible.
    /// For version 1.x.x+, any file with the same major version is compatible.
    pub fn is_version_compatible(file_version: &str) -> bool {
        let Some((file_major, file_minor, _)) = Self::parse_version(file_version) else {
            return false;
        };

        if Self::VERSION_MAJOR == 0 {
            // Unstable: require exact minor version match
            file_major == 0 && file_minor == Self::VERSION_MINOR
        } else {
            // Stable: same major version is compatible
            file_major == Self::VERSION_MAJOR
        }
    }

    /// Check if we can read a file but it might have compatibility issues.
    ///
    /// Returns true for any version 0.x file (we'll try to read it with warnings).
    pub fn is_version_readable(file_version: &str) -> bool {
        let Some((file_major, _, _)) = Self::parse_version(file_version) else {
            return false;
        };

        // We can attempt to read any version 0 file, but with warnings
        file_major == 0 || file_major == Self::VERSION_MAJOR
    }

    /// Get total annotation count across all images.
    pub fn total_annotations(&self) -> usize {
        self.images.iter().map(|i| i.annotations.len()).sum()
    }

    /// Check if the project has any annotations.
    pub fn has_annotations(&self) -> bool {
        self.images.iter().any(|i| !i.annotations.is_empty())
    }
}

impl Default for ProjectData {
    fn default() -> Self {
        Self::new()
    }
}

/// An image with its annotations and tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageEntry {
    /// Path to the image file (can be relative or absolute).
    pub path: PathBuf,

    /// Original filename (for display).
    pub filename: String,

    /// Image dimensions (width, height) if known.
    #[serde(default)]
    pub dimensions: Option<(u32, u32)>,

    /// Annotations on this image.
    pub annotations: Vec<AnnotationEntry>,

    /// Tag IDs selected for this image.
    #[serde(default)]
    pub tag_ids: HashSet<u32>,
}

impl ImageEntry {
    /// Create a new image entry.
    pub fn new(path: PathBuf) -> Self {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            path,
            filename,
            dimensions: None,
            annotations: Vec::new(),
            tag_ids: HashSet::new(),
        }
    }

    /// Set the image dimensions.
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.dimensions = Some((width, height));
        self
    }

    /// Add an annotation to this image.
    pub fn add_annotation(&mut self, annotation: AnnotationEntry) {
        self.annotations.push(annotation);
    }
}

/// An annotation entry with shape and category information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationEntry {
    /// Unique ID within the image.
    pub id: u32,

    /// Category ID this annotation belongs to.
    pub category_id: u32,

    /// The shape data.
    pub shape: ShapeEntry,

    /// Optional custom attributes/metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, serde_json::Value>,
}

impl AnnotationEntry {
    /// Create a new annotation entry.
    pub fn new(id: u32, category_id: u32, shape: ShapeEntry) -> Self {
        Self {
            id,
            category_id,
            shape,
            attributes: HashMap::new(),
        }
    }

    /// Create from an internal Annotation.
    pub fn from_annotation(annotation: &Annotation) -> Self {
        Self {
            id: annotation.id,
            category_id: annotation.category_id,
            shape: ShapeEntry::from_shape(&annotation.shape),
            attributes: HashMap::new(),
        }
    }

    /// Convert to an internal Annotation.
    pub fn to_annotation(&self) -> Annotation {
        Annotation::new(self.id, self.shape.to_shape(), self.category_id)
    }
}

/// Shape types with their coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ShapeEntry {
    /// Bounding box defined by top-left corner and size.
    #[serde(rename = "bbox")]
    BoundingBox {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },

    /// Single point marker.
    #[serde(rename = "point")]
    Point { x: f32, y: f32 },

    /// Polygon defined by vertices.
    #[serde(rename = "polygon")]
    Polygon { vertices: Vec<(f32, f32)> },
}

impl ShapeEntry {
    /// Create from an internal AnnotationShape.
    pub fn from_shape(shape: &AnnotationShape) -> Self {
        match shape {
            AnnotationShape::BoundingBox {
                x,
                y,
                width,
                height,
            } => ShapeEntry::BoundingBox {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            },
            AnnotationShape::Point { x, y } => ShapeEntry::Point { x: *x, y: *y },
            AnnotationShape::Polygon { vertices } => ShapeEntry::Polygon {
                vertices: vertices.clone(),
            },
        }
    }

    /// Convert to an internal AnnotationShape.
    pub fn to_shape(&self) -> AnnotationShape {
        match self {
            ShapeEntry::BoundingBox {
                x,
                y,
                width,
                height,
            } => AnnotationShape::BoundingBox {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            },
            ShapeEntry::Point { x, y } => AnnotationShape::Point { x: *x, y: *y },
            ShapeEntry::Polygon { vertices } => AnnotationShape::Polygon {
                vertices: vertices.clone(),
            },
        }
    }

    /// Get the shape type as a string (for error messages).
    pub fn shape_type(&self) -> &'static str {
        match self {
            ShapeEntry::BoundingBox { .. } => "bbox",
            ShapeEntry::Point { .. } => "point",
            ShapeEntry::Polygon { .. } => "polygon",
        }
    }

    /// Check if this is a bounding box.
    pub fn is_bbox(&self) -> bool {
        matches!(self, ShapeEntry::BoundingBox { .. })
    }

    /// Check if this is a point.
    pub fn is_point(&self) -> bool {
        matches!(self, ShapeEntry::Point { .. })
    }

    /// Check if this is a polygon.
    pub fn is_polygon(&self) -> bool {
        matches!(self, ShapeEntry::Polygon { .. })
    }
}

/// Category definition for export/import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryEntry {
    /// Unique identifier.
    pub id: u32,

    /// Display name.
    pub name: String,

    /// RGB color (optional for formats that don't support it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,

    /// Supercategory for COCO compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supercategory: Option<String>,
}

impl CategoryEntry {
    /// Create a new category entry.
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            color: None,
            supercategory: None,
        }
    }

    /// Create from an internal Category.
    pub fn from_category(category: &Category) -> Self {
        Self {
            id: category.id,
            name: category.name.clone(),
            color: Some(category.color),
            supercategory: None,
        }
    }

    /// Convert to an internal Category.
    pub fn to_category(&self) -> Category {
        Category::new(self.id, &self.name, self.color.unwrap_or([200, 200, 200]))
    }

    /// Set the color.
    pub fn with_color(mut self, color: [u8; 3]) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the supercategory.
    pub fn with_supercategory(mut self, supercategory: impl Into<String>) -> Self {
        self.supercategory = Some(supercategory.into());
        self
    }
}

/// Tag definition for export/import (image-level tags).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagEntry {
    /// Unique identifier.
    pub id: u32,

    /// Display name.
    pub name: String,

    /// RGB color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
}

impl TagEntry {
    /// Create a new tag entry.
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            color: None,
        }
    }

    /// Create from an internal Tag.
    pub fn from_tag(tag: &Tag) -> Self {
        Self {
            id: tag.id,
            name: tag.name.clone(),
            color: Some(tag.color),
        }
    }

    /// Convert to an internal Tag.
    pub fn to_tag(&self) -> Tag {
        Tag::new(self.id, &self.name, self.color.unwrap_or([100, 140, 180]))
    }

    /// Set the color.
    pub fn with_color(mut self, color: [u8; 3]) -> Self {
        self.color = Some(color);
        self
    }
}

/// Project metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Tool that created this file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// Creation timestamp (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last modified timestamp (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,

    /// Format-specific extra data.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ProjectMetadata {
    /// Create new metadata with the current timestamp.
    pub fn new() -> Self {
        Self {
            created_by: Some("HVAT".to_string()),
            created_at: Some(Self::current_timestamp()),
            modified_at: Some(Self::current_timestamp()),
            extra: HashMap::new(),
        }
    }

    /// Update the modified timestamp.
    pub fn touch(&mut self) {
        self.modified_at = Some(Self::current_timestamp());
    }

    /// Get the current timestamp as ISO 8601 string.
    fn current_timestamp() -> String {
        // Use web-time for cross-platform compatibility (native + WASM)
        let now = web_time::SystemTime::now();
        let duration = now
            .duration_since(web_time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();

        // Simple ISO 8601 format without external chrono dependency
        // Format: YYYY-MM-DDTHH:MM:SSZ (UTC)
        let days_since_epoch = secs / 86400;
        let secs_today = secs % 86400;
        let hours = secs_today / 3600;
        let mins = (secs_today % 3600) / 60;
        let secs_remaining = secs_today % 60;

        // Calculate year/month/day from days since epoch (1970-01-01)
        let (year, month, day) = days_to_ymd(days_since_epoch);

        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hours, mins, secs_remaining
        )
    }
}

/// Convert days since Unix epoch to year/month/day.
fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    // Simplified algorithm - good enough for timestamps
    let mut remaining_days = days as i64;
    let mut year = 1970i32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let leap = is_leap_year(year);
    let days_in_months: [i64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u32;
    for &days_in_month in &days_in_months {
        if remaining_days < days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    let day = remaining_days as u32 + 1;
    (year as u32, month, day)
}

/// Check if a year is a leap year.
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Conversion utilities for building ProjectData from app state.
impl ProjectData {
    /// Build project data from categories, tags, and image data.
    pub fn from_app_state(
        folder: PathBuf,
        image_paths: &[PathBuf],
        categories: &[Category],
        tags: &[Tag],
        get_image_data: impl Fn(&PathBuf) -> ImageData,
        get_dimensions: impl Fn(&PathBuf) -> Option<(u32, u32)>,
    ) -> Self {
        log::info!(
            "ProjectData::from_app_state: folder={:?}, {} images, {} categories, {} tags",
            folder,
            image_paths.len(),
            categories.len(),
            tags.len()
        );

        let mut data = Self::new();
        data.folder = folder;

        // Convert categories
        data.categories = categories
            .iter()
            .map(CategoryEntry::from_category)
            .collect();

        // Convert tags
        data.tags = tags.iter().map(TagEntry::from_tag).collect();

        // Convert images and annotations
        for (idx, path) in image_paths.iter().enumerate() {
            let image_data = get_image_data(path);
            let mut entry = ImageEntry::new(path.clone());

            if let Some((w, h)) = get_dimensions(path) {
                entry = entry.with_dimensions(w, h);
            }

            entry.tag_ids = image_data.selected_tag_ids;
            entry.annotations = image_data
                .annotations
                .iter()
                .map(AnnotationEntry::from_annotation)
                .collect();

            log::debug!(
                "  Image {}: {:?} ({} annotations)",
                idx,
                path,
                entry.annotations.len()
            );
            data.images.push(entry);
        }

        log::info!(
            "ProjectData::from_app_state: created {} image entries",
            data.images.len()
        );

        data.metadata = ProjectMetadata::new();
        data
    }
}
