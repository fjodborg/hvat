//! Configuration file support for HVAT.
//!
//! This module provides serialization and deserialization of application settings,
//! allowing users to export and import their configuration.

use hvat_ui::KeyCode;
use serde::{Deserialize, Serialize};

use crate::keybindings::{KeyBindings, MAX_CATEGORY_HOTKEYS};
use crate::model::Category;

/// Current configuration file format version.
/// Increment this when making breaking changes to the config format.
pub const CONFIG_VERSION: u32 = 1;

/// Application configuration that can be exported and imported.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Version of the configuration file format
    pub version: u32,

    /// Application name (for identification)
    #[serde(default = "default_app_name")]
    pub app_name: String,

    /// User preferences
    pub preferences: UserPreferences,

    /// Keybinding configuration
    pub keybindings: KeyBindingsConfig,

    /// Category definitions
    pub categories: Vec<CategoryConfig>,
}

fn default_app_name() -> String {
    "HVAT".to_string()
}

/// User preferences section of the config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Dark theme enabled
    #[serde(default = "default_dark_theme")]
    pub dark_theme: bool,

    /// Default export folder path
    #[serde(default)]
    pub export_folder: String,

    /// Default import folder path
    #[serde(default)]
    pub import_folder: String,

    /// Number of images to preload before/after current
    #[serde(default = "default_gpu_preload_count")]
    pub gpu_preload_count: usize,
}

fn default_dark_theme() -> bool {
    true
}

fn default_gpu_preload_count() -> usize {
    crate::constants::DEFAULT_GPU_PRELOAD_COUNT
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            dark_theme: default_dark_theme(),
            export_folder: String::new(),
            import_folder: String::new(),
            gpu_preload_count: default_gpu_preload_count(),
        }
    }
}

/// Keybinding configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindingsConfig {
    /// Hotkey for Select tool
    pub tool_select: KeyCode,
    /// Hotkey for BoundingBox tool
    pub tool_bbox: KeyCode,
    /// Hotkey for Polygon tool
    pub tool_polygon: KeyCode,
    /// Hotkey for Point tool
    pub tool_point: KeyCode,

    /// Hotkeys for category selection (indices 0-9 map to categories 1-10)
    #[serde(default = "default_category_hotkeys")]
    pub category_hotkeys: Vec<Option<KeyCode>>,
}

fn default_category_hotkeys() -> Vec<Option<KeyCode>> {
    vec![
        Some(KeyCode::Key1),
        Some(KeyCode::Key2),
        Some(KeyCode::Key3),
        Some(KeyCode::Key4),
        Some(KeyCode::Key5),
        Some(KeyCode::Key6),
        Some(KeyCode::Key7),
        Some(KeyCode::Key8),
        Some(KeyCode::Key9),
        Some(KeyCode::Key0),
    ]
}

impl Default for KeyBindingsConfig {
    fn default() -> Self {
        Self {
            tool_select: KeyCode::G,
            tool_bbox: KeyCode::E,
            tool_polygon: KeyCode::R,
            tool_point: KeyCode::T,
            category_hotkeys: default_category_hotkeys(),
        }
    }
}

impl From<&KeyBindings> for KeyBindingsConfig {
    fn from(bindings: &KeyBindings) -> Self {
        Self {
            tool_select: bindings.tool_select,
            tool_bbox: bindings.tool_bbox,
            tool_polygon: bindings.tool_polygon,
            tool_point: bindings.tool_point,
            category_hotkeys: bindings.category_hotkeys.to_vec(),
        }
    }
}

impl KeyBindingsConfig {
    /// Convert back to KeyBindings, filling missing slots with None.
    pub fn to_keybindings(&self) -> KeyBindings {
        let mut category_hotkeys: [Option<KeyCode>; MAX_CATEGORY_HOTKEYS] =
            [None; MAX_CATEGORY_HOTKEYS];

        for (i, hotkey) in self
            .category_hotkeys
            .iter()
            .take(MAX_CATEGORY_HOTKEYS)
            .enumerate()
        {
            category_hotkeys[i] = *hotkey;
        }

        KeyBindings {
            tool_select: self.tool_select,
            tool_bbox: self.tool_bbox,
            tool_polygon: self.tool_polygon,
            tool_point: self.tool_point,
            category_hotkeys,
        }
    }
}

/// Category configuration for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryConfig {
    /// Unique identifier for the category
    pub id: u32,
    /// Display name of the category
    pub name: String,
    /// RGB color for the category
    pub color: [u8; 3],
}

impl From<&Category> for CategoryConfig {
    fn from(cat: &Category) -> Self {
        Self {
            id: cat.id,
            name: cat.name.clone(),
            color: cat.color,
        }
    }
}

impl From<CategoryConfig> for Category {
    fn from(config: CategoryConfig) -> Self {
        Category::new(config.id, &config.name, config.color)
    }
}

impl AppConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self {
            version: CONFIG_VERSION,
            app_name: default_app_name(),
            preferences: UserPreferences::default(),
            keybindings: KeyBindingsConfig::default(),
            categories: vec![
                CategoryConfig {
                    id: 1,
                    name: "Background".to_string(),
                    color: [100, 100, 100],
                },
                CategoryConfig {
                    id: 2,
                    name: "Object".to_string(),
                    color: [255, 100, 100],
                },
                CategoryConfig {
                    id: 3,
                    name: "Region".to_string(),
                    color: [100, 255, 100],
                },
            ],
        }
    }

    /// Serialize the configuration to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize configuration from JSON.
    pub fn from_json(json: &str) -> Result<Self, ConfigError> {
        let config: Self = serde_json::from_str(json)?;

        // Validate version compatibility
        if config.version > CONFIG_VERSION {
            return Err(ConfigError::VersionTooNew {
                file_version: config.version,
                supported_version: CONFIG_VERSION,
            });
        }

        Ok(config)
    }

    /// Get the default filename for config export.
    pub fn default_filename() -> &'static str {
        "hvat-config.json"
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when loading configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// JSON parsing error
    #[error("Failed to parse configuration: {0}")]
    ParseError(#[from] serde_json::Error),

    /// Configuration version is newer than supported
    #[error(
        "Configuration file version {file_version} is newer than supported version {supported_version}"
    )]
    VersionTooNew {
        file_version: u32,
        supported_version: u32,
    },

    /// I/O error when reading/writing config
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
