//! Per-image data storage for tags and annotations.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::{Annotation, AnnotationId, DrawingState, EditState};

/// Data associated with a specific image (tags, annotations, etc.)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImageData {
    /// Which tag IDs are selected/active for this image
    pub selected_tag_ids: HashSet<u32>,
    /// Annotations on this image
    pub annotations: Vec<Annotation>,
    /// Image dimensions (width, height) - stored when image is loaded
    #[serde(default)]
    pub dimensions: Option<(u32, u32)>,
    /// Current drawing state for this image (transient, not serialized)
    #[serde(skip)]
    pub drawing_state: DrawingState,
    /// Current edit state for modifying existing annotations (transient)
    #[serde(skip)]
    pub edit_state: EditState,
    /// Next annotation ID for this image
    pub next_annotation_id: AnnotationId,
    /// Last clicked annotation index (for cycling through overlapping annotations)
    #[serde(skip)]
    pub last_clicked_index: Option<usize>,
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

    /// Remove a tag from all images' selected tags by ID
    pub fn remove_tag_from_all(&mut self, tag_id: u32) {
        for image_data in self.data.values_mut() {
            image_data.selected_tag_ids.remove(&tag_id);
        }
    }

    /// Remove all annotations with a specific category ID from all images.
    /// Returns the total number of annotations removed.
    pub fn remove_annotations_by_category(&mut self, category_id: u32) -> usize {
        let mut total_removed = 0;
        for image_data in self.data.values_mut() {
            let before = image_data.annotations.len();
            image_data
                .annotations
                .retain(|a| a.category_id != category_id);
            total_removed += before - image_data.annotations.len();
        }
        total_removed
    }

    /// Iterate over all image data entries
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &ImageData)> {
        self.data.iter()
    }
}
