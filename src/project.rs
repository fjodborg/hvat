//! Project file management for HVAT.
//!
//! A project file stores the complete session state including:
//! - Image paths/sources
//! - All annotations for all images
//! - Band selections and image settings (per-image and global)
//! - Persistence mode settings
//!
//! # File Format
//!
//! Projects are stored as JSON files with the `.hvat` extension.
//!
//! ```json
//! {
//!   "version": "1.0",
//!   "images": ["image1.jpg", "image2.png", ...],
//!   "annotations": { "image1.jpg": {...}, ... },
//!   "settings": {
//!     "band_selection": { "red": 0, "green": 1, "blue": 2 },
//!     "image_settings": { "brightness": 0.0, ... },
//!     "band_persistence": "Constant",
//!     "image_settings_persistence": "Constant"
//!   },
//!   "per_image_settings": {
//!     "image1.jpg": { "band_selection": {...}, "image_settings": {...} }
//!   }
//! }
//! ```

use crate::annotation::AnnotationStore;
use crate::hyperspectral::BandSelection;
use crate::hvat_app::ImageSettings;
use crate::message::PersistenceMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current project file format version.
pub const PROJECT_VERSION: &str = "1.0";

/// A project file containing all session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Format version for forward compatibility.
    pub version: String,
    /// List of image filenames/paths in order.
    pub images: Vec<String>,
    /// Annotations per image (keyed by image filename).
    pub annotations: HashMap<String, AnnotationStore>,
    /// Global settings.
    pub settings: ProjectSettings,
    /// Per-image settings (band selection and image manipulation).
    #[serde(default)]
    pub per_image_settings: HashMap<String, PerImageSettings>,
}

/// Global project settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Current band selection for RGB composite.
    pub band_selection: BandSelectionData,
    /// Current image manipulation settings.
    pub image_settings: ImageSettingsData,
    /// How band selection persists across images.
    pub band_persistence: PersistenceModeData,
    /// How image settings persist across images.
    pub image_settings_persistence: PersistenceModeData,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            band_selection: BandSelectionData::default(),
            image_settings: ImageSettingsData::default(),
            band_persistence: PersistenceModeData::Constant,
            image_settings_persistence: PersistenceModeData::Constant,
        }
    }
}

/// Serializable band selection data.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BandSelectionData {
    pub red: usize,
    pub green: usize,
    pub blue: usize,
}

impl Default for BandSelectionData {
    fn default() -> Self {
        Self {
            red: 0,
            green: 1,
            blue: 2,
        }
    }
}

impl From<BandSelection> for BandSelectionData {
    fn from(bs: BandSelection) -> Self {
        Self {
            red: bs.red,
            green: bs.green,
            blue: bs.blue,
        }
    }
}

impl From<BandSelectionData> for BandSelection {
    fn from(data: BandSelectionData) -> Self {
        BandSelection::new(data.red, data.green, data.blue)
    }
}

/// Serializable image settings data.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ImageSettingsData {
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub hue_shift: f32,
}

impl Default for ImageSettingsData {
    fn default() -> Self {
        Self {
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            hue_shift: 0.0,
        }
    }
}

impl From<ImageSettings> for ImageSettingsData {
    fn from(is: ImageSettings) -> Self {
        Self {
            brightness: is.brightness,
            contrast: is.contrast,
            gamma: is.gamma,
            hue_shift: is.hue_shift,
        }
    }
}

impl From<ImageSettingsData> for ImageSettings {
    fn from(data: ImageSettingsData) -> Self {
        ImageSettings {
            brightness: data.brightness,
            contrast: data.contrast,
            gamma: data.gamma,
            hue_shift: data.hue_shift,
        }
    }
}

/// Serializable persistence mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistenceModeData {
    Reset,
    PerImage,
    Constant,
}

impl Default for PersistenceModeData {
    fn default() -> Self {
        Self::Constant
    }
}

impl From<PersistenceMode> for PersistenceModeData {
    fn from(pm: PersistenceMode) -> Self {
        match pm {
            PersistenceMode::Reset => Self::Reset,
            PersistenceMode::PerImage => Self::PerImage,
            PersistenceMode::Constant => Self::Constant,
        }
    }
}

impl From<PersistenceModeData> for PersistenceMode {
    fn from(data: PersistenceModeData) -> Self {
        match data {
            PersistenceModeData::Reset => Self::Reset,
            PersistenceModeData::PerImage => Self::PerImage,
            PersistenceModeData::Constant => Self::Constant,
        }
    }
}

/// Per-image settings (stored when using PerImage persistence mode).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerImageSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub band_selection: Option<BandSelectionData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_settings: Option<ImageSettingsData>,
}

impl Project {
    /// Create a new empty project.
    pub fn new() -> Self {
        Self {
            version: PROJECT_VERSION.to_string(),
            images: Vec::new(),
            annotations: HashMap::new(),
            settings: ProjectSettings::default(),
            per_image_settings: HashMap::new(),
        }
    }

    /// Save the project to a JSON string.
    pub fn to_json(&self) -> Result<String, ProjectError> {
        serde_json::to_string_pretty(self).map_err(ProjectError::SerializationError)
    }

    /// Load a project from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, ProjectError> {
        let project: Self = serde_json::from_str(json).map_err(ProjectError::DeserializationError)?;

        // Version check for future compatibility
        if project.version != PROJECT_VERSION {
            log::warn!(
                "Project version mismatch: expected {}, got {}",
                PROJECT_VERSION,
                project.version
            );
        }

        Ok(project)
    }

    /// Save the project to a file (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), ProjectError> {
        let json = self.to_json()?;
        std::fs::write(path, json).map_err(ProjectError::IoError)
    }

    /// Load a project from a file (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, ProjectError> {
        let json = std::fs::read_to_string(path).map_err(ProjectError::IoError)?;
        Self::from_json(&json)
    }

    /// Add an image to the project.
    pub fn add_image(&mut self, name: String) {
        if !self.images.contains(&name) {
            self.images.push(name);
        }
    }

    /// Set annotations for an image.
    pub fn set_annotations(&mut self, image_name: String, store: AnnotationStore) {
        self.annotations.insert(image_name, store);
    }

    /// Get annotations for an image (if any).
    pub fn get_annotations(&self, image_name: &str) -> Option<&AnnotationStore> {
        self.annotations.get(image_name)
    }

    /// Set per-image band selection.
    pub fn set_per_image_band_selection(&mut self, image_name: String, selection: BandSelection) {
        self.per_image_settings
            .entry(image_name)
            .or_default()
            .band_selection = Some(selection.into());
    }

    /// Set per-image settings.
    pub fn set_per_image_settings(&mut self, image_name: String, settings: ImageSettings) {
        self.per_image_settings
            .entry(image_name)
            .or_default()
            .image_settings = Some(settings.into());
    }

    /// Get per-image band selection (if any).
    pub fn get_per_image_band_selection(&self, image_name: &str) -> Option<BandSelection> {
        self.per_image_settings
            .get(image_name)
            .and_then(|s| s.band_selection)
            .map(|data| data.into())
    }

    /// Get per-image settings (if any).
    pub fn get_per_image_settings(&self, image_name: &str) -> Option<ImageSettings> {
        self.per_image_settings
            .get(image_name)
            .and_then(|s| s.image_settings)
            .map(|data| data.into())
    }

    /// Get the number of images in the project.
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// Get the number of annotated images.
    pub fn annotated_image_count(&self) -> usize {
        self.annotations.values().filter(|s| !s.is_empty()).count()
    }

    /// Get total annotation count across all images.
    pub fn total_annotation_count(&self) -> usize {
        self.annotations.values().map(|s| s.len()).sum()
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during project operations.
#[derive(Debug)]
pub enum ProjectError {
    /// JSON serialization error.
    SerializationError(serde_json::Error),
    /// JSON deserialization error.
    DeserializationError(serde_json::Error),
    /// File I/O error.
    #[cfg(not(target_arch = "wasm32"))]
    IoError(std::io::Error),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError(e) => write!(f, "Failed to serialize project: {}", e),
            Self::DeserializationError(e) => write!(f, "Failed to deserialize project: {}", e),
            #[cfg(not(target_arch = "wasm32"))]
            Self::IoError(e) => write!(f, "File I/O error: {}", e),
        }
    }
}

impl std::error::Error for ProjectError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::{BoundingBox, Category, Shape};

    #[test]
    fn test_project_new() {
        let project = Project::new();
        assert_eq!(project.version, PROJECT_VERSION);
        assert!(project.images.is_empty());
        assert!(project.annotations.is_empty());
    }

    #[test]
    fn test_project_add_image() {
        let mut project = Project::new();
        project.add_image("test.jpg".to_string());
        project.add_image("test2.png".to_string());
        project.add_image("test.jpg".to_string()); // Duplicate should be ignored

        assert_eq!(project.image_count(), 2);
    }

    #[test]
    fn test_project_annotations() {
        let mut project = Project::new();
        project.add_image("test.jpg".to_string());

        let mut store = AnnotationStore::new();
        store.add_category(Category::new(1, "car"));
        store.add(1, Shape::BoundingBox(BoundingBox::new(10.0, 20.0, 100.0, 50.0)));

        project.set_annotations("test.jpg".to_string(), store);

        assert_eq!(project.annotated_image_count(), 1);
        assert_eq!(project.total_annotation_count(), 1);

        let retrieved = project.get_annotations("test.jpg").unwrap();
        assert_eq!(retrieved.len(), 1);
    }

    #[test]
    fn test_project_serialization_roundtrip() {
        let mut project = Project::new();
        project.add_image("image1.jpg".to_string());
        project.add_image("image2.png".to_string());

        // Add annotations
        let mut store = AnnotationStore::new();
        store.add_category(Category::new(1, "person"));
        store.add(1, Shape::BoundingBox(BoundingBox::new(50.0, 50.0, 200.0, 300.0)));
        project.set_annotations("image1.jpg".to_string(), store);

        // Set settings
        project.settings.band_selection = BandSelectionData { red: 2, green: 1, blue: 0 };
        project.settings.image_settings.brightness = 0.5;
        project.settings.band_persistence = PersistenceModeData::PerImage;

        // Set per-image settings
        project.set_per_image_band_selection("image1.jpg".to_string(), BandSelection::new(3, 2, 1));
        project.set_per_image_settings("image1.jpg".to_string(), ImageSettings {
            brightness: 0.2,
            contrast: 1.1,
            gamma: 0.9,
            hue_shift: 10.0,
        });

        // Serialize
        let json = project.to_json().expect("Failed to serialize");

        // Deserialize
        let loaded = Project::from_json(&json).expect("Failed to deserialize");

        // Verify
        assert_eq!(loaded.version, PROJECT_VERSION);
        assert_eq!(loaded.images.len(), 2);
        assert_eq!(loaded.images[0], "image1.jpg");
        assert_eq!(loaded.images[1], "image2.png");

        // Check annotations
        let ann = loaded.get_annotations("image1.jpg").unwrap();
        assert_eq!(ann.len(), 1);

        // Check settings
        assert_eq!(loaded.settings.band_selection.red, 2);
        assert_eq!(loaded.settings.band_selection.green, 1);
        assert_eq!(loaded.settings.band_selection.blue, 0);
        assert_eq!(loaded.settings.image_settings.brightness, 0.5);
        assert_eq!(loaded.settings.band_persistence, PersistenceModeData::PerImage);

        // Check per-image settings
        let pis_bands = loaded.get_per_image_band_selection("image1.jpg").unwrap();
        assert_eq!(pis_bands.red, 3);
        assert_eq!(pis_bands.green, 2);
        assert_eq!(pis_bands.blue, 1);

        let pis_settings = loaded.get_per_image_settings("image1.jpg").unwrap();
        assert_eq!(pis_settings.brightness, 0.2);
        assert_eq!(pis_settings.contrast, 1.1);
    }

    #[test]
    fn test_band_selection_conversion() {
        let bs = BandSelection::new(5, 3, 1);
        let data: BandSelectionData = bs.into();
        let back: BandSelection = data.into();

        assert_eq!(bs.red, back.red);
        assert_eq!(bs.green, back.green);
        assert_eq!(bs.blue, back.blue);
    }

    #[test]
    fn test_persistence_mode_conversion() {
        let modes = [PersistenceMode::Reset, PersistenceMode::PerImage, PersistenceMode::Constant];

        for mode in modes {
            let data: PersistenceModeData = mode.into();
            let back: PersistenceMode = data.into();
            assert_eq!(mode, back);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_file_save_load() {
        use std::fs;

        let mut project = Project::new();
        project.add_image("test.jpg".to_string());

        let temp_path = std::path::Path::new("/tmp/test_project.hvat");

        // Save
        project.save_to_file(temp_path).expect("Failed to save");

        // Load
        let loaded = Project::load_from_file(temp_path).expect("Failed to load");

        assert_eq!(loaded.images.len(), 1);
        assert_eq!(loaded.images[0], "test.jpg");

        // Cleanup
        let _ = fs::remove_file(temp_path);
    }
}
