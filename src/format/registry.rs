//! Format registry for discovering and accessing annotation formats.

use std::collections::HashMap;

use crate::format::formats::{CocoFormat, HvatJsonFormat, PascalVocFormat, YoloFormat};
use crate::format::traits::AnnotationFormat;

/// Registry of available annotation formats.
///
/// This provides a central location to discover and access format implementations.
/// All built-in formats are registered automatically on creation.
pub struct FormatRegistry {
    formats: HashMap<&'static str, Box<dyn AnnotationFormat>>,
}

impl FormatRegistry {
    /// Create a new registry with all built-in formats registered.
    pub fn new() -> Self {
        let mut registry = Self {
            formats: HashMap::new(),
        };

        // Register all built-in formats
        registry.register(Box::new(HvatJsonFormat));
        registry.register(Box::new(CocoFormat));
        registry.register(Box::new(YoloFormat));
        registry.register(Box::new(PascalVocFormat));

        registry
    }

    /// Register a format implementation.
    pub fn register(&mut self, format: Box<dyn AnnotationFormat>) {
        self.formats.insert(format.id(), format);
    }

    /// Get a format by its ID.
    pub fn get(&self, id: &str) -> Option<&dyn AnnotationFormat> {
        self.formats.get(id).map(|f| f.as_ref())
    }

    /// Find formats by file extension.
    pub fn by_extension(&self, ext: &str) -> Vec<&dyn AnnotationFormat> {
        self.formats
            .values()
            .filter(|f| f.extensions().iter().any(|e| e.ends_with(ext)))
            .map(|f| f.as_ref())
            .collect()
    }

    /// Get all registered formats.
    pub fn all(&self) -> Vec<&dyn AnnotationFormat> {
        self.formats.values().map(|f| f.as_ref()).collect()
    }

    /// Get all format IDs.
    pub fn ids(&self) -> Vec<&'static str> {
        self.formats.keys().copied().collect()
    }

    /// Get the native HVAT format.
    pub fn native(&self) -> &dyn AnnotationFormat {
        self.get("hvat")
            .expect("Native format should always be registered")
    }

    /// Get formats that support polygon annotations.
    pub fn polygon_formats(&self) -> Vec<&dyn AnnotationFormat> {
        self.all()
            .into_iter()
            .filter(|f| f.supports_polygon())
            .collect()
    }

    /// Get formats that support per-image export.
    pub fn per_image_formats(&self) -> Vec<&dyn AnnotationFormat> {
        self.all()
            .into_iter()
            .filter(|f| f.supports_per_image())
            .collect()
    }
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_formats() {
        let registry = FormatRegistry::new();

        assert!(registry.get("hvat").is_some());
        assert!(registry.get("coco").is_some());
        assert!(registry.get("yolo").is_some());
        assert!(registry.get("voc").is_some());
    }

    #[test]
    fn test_native_format() {
        let registry = FormatRegistry::new();
        let native = registry.native();

        assert_eq!(native.id(), "hvat");
        assert!(native.supports_polygon());
        assert!(native.supports_point());
    }

    #[test]
    fn test_polygon_formats() {
        let registry = FormatRegistry::new();
        let polygon_formats = registry.polygon_formats();

        assert!(polygon_formats.iter().any(|f| f.id() == "hvat"));
        assert!(polygon_formats.iter().any(|f| f.id() == "coco"));
        assert!(!polygon_formats.iter().any(|f| f.id() == "yolo"));
    }

    #[test]
    fn test_per_image_formats() {
        let registry = FormatRegistry::new();
        let per_image = registry.per_image_formats();

        assert!(per_image.iter().any(|f| f.id() == "yolo"));
        assert!(per_image.iter().any(|f| f.id() == "voc"));
        assert!(!per_image.iter().any(|f| f.id() == "coco"));
    }
}
