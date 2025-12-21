//! Per-image data storage for tags and annotations.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::model::{Annotation, AnnotationId, DrawingState};

/// Data associated with a specific image (tags, annotations, etc.)
#[derive(Clone, Debug, Default)]
pub struct ImageData {
    /// Which global tags are selected/active for this image
    pub selected_tags: HashSet<String>,
    /// Annotations on this image
    pub annotations: Vec<Annotation>,
    /// Current drawing state for this image
    pub drawing_state: DrawingState,
    /// Next annotation ID for this image
    pub next_annotation_id: AnnotationId,
}

/// Storage for per-image data, keyed by image path
#[derive(Clone, Debug, Default)]
pub struct ImageDataStore {
    /// Map from image path to its data
    data: HashMap<PathBuf, ImageData>,
}

impl ImageDataStore {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Get data for an image, creating default if not exists
    pub fn get_or_create(&mut self, path: &PathBuf) -> &mut ImageData {
        self.data
            .entry(path.clone())
            .or_insert_with(ImageData::default)
    }

    /// Get data for an image (read-only), returns default if not exists
    pub fn get(&self, path: &PathBuf) -> ImageData {
        self.data.get(path).cloned().unwrap_or_default()
    }

    /// Check if data exists for an image
    #[allow(dead_code)]
    pub fn contains(&self, path: &PathBuf) -> bool {
        self.data.contains_key(path)
    }

    /// Get mutable reference to image data, if it exists
    #[allow(dead_code)]
    pub fn get_mut(&mut self, path: &PathBuf) -> Option<&mut ImageData> {
        self.data.get_mut(path)
    }

    /// Ensure data exists for an image path
    #[allow(dead_code)]
    pub fn ensure(&mut self, path: &PathBuf) {
        self.data
            .entry(path.clone())
            .or_insert_with(ImageData::default);
    }

    /// Remove a tag from all images' selected tags
    pub fn remove_tag_from_all(&mut self, tag: &str) {
        for image_data in self.data.values_mut() {
            image_data.selected_tags.remove(tag);
        }
    }
}
