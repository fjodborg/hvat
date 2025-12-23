//! Native HVAT JSON format implementation.
//!
//! This is the primary format for HVAT projects, providing full fidelity
//! for all features including polygons, points, tags, and category colors.
//!
//! # Versioning
//!
//! The format uses semantic versioning. Currently at version 0.x.x (unstable):
//! - Version 0 files may not be compatible between different minor versions
//! - Do not rely on backwards compatibility until version 1.0.0
//!
//! See [`ProjectData`] for more details on version compatibility.

use std::path::Path;

use crate::format::error::FormatError;
use crate::format::project::ProjectData;
use crate::format::traits::{AnnotationFormat, ExportOptions, ExportResult, ImportOptions};

/// Native HVAT JSON format.
///
/// This format provides full fidelity for all HVAT features:
/// - All shape types (bounding box, point, polygon)
/// - Category colors and names
/// - Per-image tags
/// - Global tags
/// - Project metadata
pub struct HvatJsonFormat;

impl AnnotationFormat for HvatJsonFormat {
    fn id(&self) -> &'static str {
        "hvat"
    }

    fn display_name(&self) -> &'static str {
        "HVAT Project (JSON)"
    }

    fn extensions(&self) -> &[&'static str] {
        &["hvat.json", "hvat"]
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
        log::info!("Exporting HVAT project to {:?}", path);

        let (bytes, mut result) = self.export_to_bytes(data, options)?;
        std::fs::write(path, &bytes)?;
        result.files_created = vec![path.to_path_buf()];

        log::info!(
            "Exported {} images with {} annotations",
            result.images_exported,
            result.annotations_exported
        );

        Ok(result)
    }

    fn export_to_bytes(
        &self,
        data: &ProjectData,
        _options: &ExportOptions,
    ) -> Result<(Vec<u8>, ExportResult), FormatError> {
        log::info!("Exporting HVAT project to bytes");

        // Serialize with pretty printing for readability
        let json = serde_json::to_string_pretty(data)?;
        let annotations_count = data.total_annotations();

        log::info!(
            "Exported {} images with {} annotations",
            data.images.len(),
            annotations_count
        );

        Ok((
            json.into_bytes(),
            ExportResult {
                images_exported: data.images.len(),
                annotations_exported: annotations_count,
                warnings: Vec::new(),
                files_created: Vec::new(),
            },
        ))
    }

    fn import(&self, path: &Path, _options: &ImportOptions) -> Result<ProjectData, FormatError> {
        log::info!("Importing HVAT project from {:?}", path);

        let json = std::fs::read_to_string(path)?;
        let data: ProjectData = serde_json::from_str(&json)?;

        // Validate version compatibility
        if !ProjectData::is_version_readable(&data.version) {
            return Err(FormatError::VersionMismatch {
                expected: ProjectData::CURRENT_VERSION.to_string(),
                found: data.version.clone(),
            });
        }

        if !ProjectData::is_version_compatible(&data.version) {
            log::warn!(
                "Project version {} may not be fully compatible with current version {} \
                 (version 0.x.x is unstable - format may have changed)",
                data.version,
                ProjectData::CURRENT_VERSION
            );
        }

        log::info!(
            "Imported {} images with {} annotations (format version {})",
            data.images.len(),
            data.total_annotations(),
            data.version
        );

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::project::{CategoryEntry, ImageEntry};
    use std::path::PathBuf;

    fn create_test_project() -> ProjectData {
        let mut data = ProjectData::new();
        data.categories.push(CategoryEntry::new(0, "Test Category"));
        data.images.push(ImageEntry::new(PathBuf::from("test.png")));
        data
    }

    #[test]
    fn test_format_metadata() {
        let format = HvatJsonFormat;
        assert_eq!(format.id(), "hvat");
        assert!(format.supports_polygon());
        assert!(format.supports_point());
        assert!(!format.supports_per_image());
    }

    #[test]
    fn test_roundtrip_serialization() {
        let original = create_test_project();
        let json = serde_json::to_string(&original).unwrap();
        let loaded: ProjectData = serde_json::from_str(&json).unwrap();

        assert_eq!(original.version, loaded.version);
        assert_eq!(original.categories.len(), loaded.categories.len());
        assert_eq!(original.images.len(), loaded.images.len());
    }

    #[test]
    fn test_version_parsing() {
        assert_eq!(ProjectData::parse_version("0.1.0"), Some((0, 1, 0)));
        assert_eq!(ProjectData::parse_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(ProjectData::parse_version("10.20.30"), Some((10, 20, 30)));
        assert_eq!(ProjectData::parse_version("invalid"), None);
        assert_eq!(ProjectData::parse_version("1.2"), None);
        assert_eq!(ProjectData::parse_version("1.2.3.4"), None);
    }

    #[test]
    fn test_version_compatibility() {
        // Current version should be compatible
        assert!(ProjectData::is_version_compatible(
            ProjectData::CURRENT_VERSION
        ));

        // Same minor version in v0 should be compatible
        assert!(ProjectData::is_version_compatible("0.1.0"));
        assert!(ProjectData::is_version_compatible("0.1.5"));

        // Different minor version in v0 should NOT be compatible (unstable)
        assert!(!ProjectData::is_version_compatible("0.2.0"));
        assert!(!ProjectData::is_version_compatible("0.0.1"));

        // Future v1 should not be compatible with v0
        assert!(!ProjectData::is_version_compatible("1.0.0"));
    }

    #[test]
    fn test_version_readable() {
        // All v0 files should be readable (with warnings)
        assert!(ProjectData::is_version_readable("0.1.0"));
        assert!(ProjectData::is_version_readable("0.2.0"));
        assert!(ProjectData::is_version_readable("0.0.1"));

        // Invalid versions should not be readable
        assert!(!ProjectData::is_version_readable("invalid"));
    }

    #[test]
    fn test_current_version_is_v0() {
        // Verify we're in unstable v0
        assert_eq!(ProjectData::VERSION_MAJOR, 0);
        assert!(ProjectData::CURRENT_VERSION.starts_with("0."));
    }
}
