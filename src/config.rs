//! Configuration file support for HVAT.
//!
//! This module provides serialization and deserialization of application settings,
//! allowing users to export and import their configuration.

use hvat_ui::KeyCode;
use serde::{Deserialize, Serialize};

/// Log level setting for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Show only errors
    Error,
    /// Show errors and warnings
    Warn,
    /// Show errors, warnings, and info messages
    #[default]
    Info,
    /// Show debug-level logging
    Debug,
    /// Show all log messages including trace
    Trace,
}

impl LogLevel {
    /// Get the display name for this log level.
    pub fn name(&self) -> &'static str {
        match self {
            LogLevel::Error => "Error",
            LogLevel::Warn => "Warn",
            LogLevel::Info => "Info",
            LogLevel::Debug => "Debug",
            LogLevel::Trace => "Trace",
        }
    }

    /// Get all log levels in order from least to most verbose.
    pub fn all() -> &'static [LogLevel] {
        &[
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
            LogLevel::Trace,
        ]
    }

    /// Convert to log crate's LevelFilter.
    pub fn to_level_filter(&self) -> log::LevelFilter {
        match self {
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

use crate::keybindings::{KeyBindings, MAX_CATEGORY_HOTKEYS};
use crate::model::{Category, default_categories};

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

    /// Log verbosity level
    #[serde(default)]
    pub log_level: LogLevel,
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
            log_level: LogLevel::default(),
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
            categories: default_categories()
                .iter()
                .map(CategoryConfig::from)
                .collect(),
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

    /// Get the default config file path for auto-load/save.
    /// Returns None on WASM (no filesystem access).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn default_path() -> Option<std::path::PathBuf> {
        // Try to use XDG config directory, fall back to home directory
        if let Some(config_dir) = dirs::config_dir() {
            Some(config_dir.join("hvat").join(Self::default_filename()))
        } else if let Some(home_dir) = dirs::home_dir() {
            Some(
                home_dir
                    .join(".config")
                    .join("hvat")
                    .join(Self::default_filename()),
            )
        } else {
            None
        }
    }

    /// Try to load configuration from the default path.
    /// Returns None if the file doesn't exist or can't be read.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_from_default_path() -> Option<Self> {
        let path = Self::default_path()?;
        if !path.exists() {
            log::debug!("No config file found at {:?}", path);
            return None;
        }

        match std::fs::read_to_string(&path) {
            Ok(json) => match Self::from_json(&json) {
                Ok(config) => {
                    log::info!("Loaded configuration from {:?}", path);
                    Some(config)
                }
                Err(e) => {
                    log::warn!("Failed to parse config file {:?}: {}", path, e);
                    None
                }
            },
            Err(e) => {
                log::warn!("Failed to read config file {:?}: {}", path, e);
                None
            }
        }
    }

    /// Save configuration to the default path.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_to_default_path(&self) -> Result<(), ConfigError> {
        let path = Self::default_path().ok_or_else(|| {
            ConfigError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine config directory",
            ))
        })?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = self.to_json()?;
        std::fs::write(&path, json)?;
        log::info!("Saved configuration to {:?}", path);
        Ok(())
    }

    /// LocalStorage key for WASM config persistence.
    #[cfg(target_arch = "wasm32")]
    const LOCALSTORAGE_KEY: &'static str = "hvat-config";

    /// Try to load configuration from localStorage (WASM only).
    /// Returns None if not found or can't be parsed.
    #[cfg(target_arch = "wasm32")]
    pub fn load_from_local_storage() -> Option<Self> {
        let window = web_sys::window()?;
        let storage = window.local_storage().ok()??;

        match storage.get_item(Self::LOCALSTORAGE_KEY) {
            Ok(Some(json)) => match Self::from_json(&json) {
                Ok(config) => {
                    log::info!("Loaded configuration from localStorage");
                    Some(config)
                }
                Err(e) => {
                    log::warn!("Failed to parse config from localStorage: {}", e);
                    None
                }
            },
            Ok(None) => {
                log::debug!("No config found in localStorage");
                None
            }
            Err(e) => {
                log::warn!("Failed to read from localStorage: {:?}", e);
                None
            }
        }
    }

    /// Save configuration to localStorage (WASM only).
    #[cfg(target_arch = "wasm32")]
    pub fn save_to_local_storage(&self) -> Result<(), ConfigError> {
        let window = web_sys::window()
            .ok_or_else(|| ConfigError::StorageError("No window object available".to_string()))?;

        let storage = window
            .local_storage()
            .map_err(|e| ConfigError::StorageError(format!("localStorage access error: {:?}", e)))?
            .ok_or_else(|| ConfigError::StorageError("localStorage not available".to_string()))?;

        let json = self.to_json()?;

        storage
            .set_item(Self::LOCALSTORAGE_KEY, &json)
            .map_err(|e| {
                ConfigError::StorageError(format!("Failed to save to localStorage: {:?}", e))
            })?;

        log::info!("Saved configuration to localStorage");
        Ok(())
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

    /// Storage error (localStorage in WASM)
    #[error("Storage error: {0}")]
    StorageError(String),
}
