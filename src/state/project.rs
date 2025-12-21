//! Project state management for loaded folders and images.

use std::path::PathBuf;

/// Supported image extensions
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"];

/// Check if a filename (string) has a supported image extension.
/// Works with both full paths and just filenames.
pub fn is_image_filename(name: &str) -> bool {
    let lower = name.to_lowercase();
    IMAGE_EXTENSIONS
        .iter()
        .any(|ext| lower.ends_with(&format!(".{}", ext)))
}

/// Check if a path has a supported image extension
fn is_image_file(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Image data loaded from a file (used for WASM where we can't access filesystem).
#[derive(Clone, Debug)]
pub struct LoadedImage {
    /// Filename of the image
    pub name: String,
    /// Raw image data bytes
    pub data: Vec<u8>,
}

/// State for a loaded project (folder with images).
#[derive(Clone, Debug)]
pub struct ProjectState {
    /// Path to the project folder (may be empty string on WASM)
    pub folder: PathBuf,
    /// List of image files in the folder (can be from subfolders)
    pub images: Vec<PathBuf>,
    /// Current image index
    pub current_index: usize,
    /// In-memory image data for WASM (where we can't access filesystem)
    pub loaded_images: Vec<LoadedImage>,
}

impl ProjectState {
    /// Discover image files in a folder, non-recursively (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_folder(folder: PathBuf) -> Result<Self, String> {
        let mut images: Vec<PathBuf> = std::fs::read_dir(&folder)
            .map_err(|e| format!("Failed to read folder: {}", e))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && is_image_file(path))
            .collect();

        if images.is_empty() {
            return Err("No image files found in folder".to_string());
        }

        // Sort by filename for consistent ordering
        images.sort();

        Ok(Self {
            folder,
            images,
            current_index: 0,
            loaded_images: Vec::new(),
        })
    }

    /// Discover image files in a folder recursively (native only).
    /// Scans all subdirectories and includes images from nested folders.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_folder_recursive(folder: PathBuf) -> Result<Self, String> {
        let mut images: Vec<PathBuf> = Vec::new();
        Self::scan_folder_recursive(&folder, &mut images)?;

        if images.is_empty() {
            return Err("No image files found in folder or subfolders".to_string());
        }

        // Sort by full path for consistent ordering
        images.sort();

        log::info!(
            "Recursively scanned folder {:?}: found {} images",
            folder,
            images.len()
        );

        Ok(Self {
            folder,
            images,
            current_index: 0,
            loaded_images: Vec::new(),
        })
    }

    /// Recursively scan a folder for image files (native only).
    #[cfg(not(target_arch = "wasm32"))]
    fn scan_folder_recursive(folder: &PathBuf, images: &mut Vec<PathBuf>) -> Result<(), String> {
        let entries = std::fs::read_dir(folder)
            .map_err(|e| format!("Failed to read folder {:?}: {}", folder, e))?;

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.is_file() && is_image_file(&path) {
                images.push(path);
            } else if path.is_dir() {
                // Recursively scan subdirectory
                if let Err(e) = Self::scan_folder_recursive(&path, images) {
                    log::warn!("Failed to scan subdirectory {:?}: {}", path, e);
                    // Continue scanning other directories
                }
            }
        }

        Ok(())
    }

    /// Create project from a list of paths (files and/or folders).
    /// Files are added directly, folders are scanned recursively.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_paths(paths: Vec<PathBuf>) -> Result<Self, String> {
        if paths.is_empty() {
            return Err("No paths provided".to_string());
        }

        let mut images: Vec<PathBuf> = Vec::new();

        for path in &paths {
            if path.is_file() && is_image_file(path) {
                images.push(path.clone());
            } else if path.is_dir() {
                if let Err(e) = Self::scan_folder_recursive(path, &mut images) {
                    log::warn!("Failed to scan folder {:?}: {}", path, e);
                }
            }
        }

        if images.is_empty() {
            return Err("No image files found in dropped items".to_string());
        }

        // Sort by full path for consistent ordering
        images.sort();

        // Determine the root folder (common parent or first folder)
        let folder = if paths.len() == 1 && paths[0].is_dir() {
            paths[0].clone()
        } else {
            // Use the parent of the first image as the folder
            images[0]
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        };

        log::info!(
            "Created project from {} paths: {} images, root folder: {:?}",
            paths.len(),
            images.len(),
            folder
        );

        Ok(Self {
            folder,
            images,
            current_index: 0,
            loaded_images: Vec::new(),
        })
    }

    /// Create project from loaded image data (WASM only).
    #[cfg(target_arch = "wasm32")]
    pub fn from_loaded_images(loaded_images: Vec<LoadedImage>) -> Result<Self, String> {
        if loaded_images.is_empty() {
            return Err("No images loaded".to_string());
        }

        // Create virtual paths from image names
        let images: Vec<PathBuf> = loaded_images
            .iter()
            .map(|img| PathBuf::from(&img.name))
            .collect();

        Ok(Self {
            folder: PathBuf::from(""),
            images,
            current_index: 0,
            loaded_images,
        })
    }

    /// Get the current image path.
    pub fn current_image(&self) -> Option<&PathBuf> {
        self.images.get(self.current_index)
    }

    /// Get the current image filename for display.
    /// If the image is in a subfolder, shows the relative path from the project folder.
    pub fn current_name(&self) -> String {
        let Some(path) = self.current_image() else {
            return "Unknown".to_string();
        };

        // Try to get relative path from project folder
        if !self.folder.as_os_str().is_empty() {
            if let Ok(relative) = path.strip_prefix(&self.folder) {
                if let Some(s) = relative.to_str() {
                    return s.to_string();
                }
            }
        }

        // Fall back to just the filename
        path.file_name()
            .and_then(|n| n.to_str())
            .map(String::from)
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Move to the next image, wrapping around.
    pub fn next(&mut self) {
        if !self.images.is_empty() {
            self.current_index = (self.current_index + 1) % self.images.len();
        }
    }

    /// Move to the previous image, wrapping around.
    pub fn prev(&mut self) {
        if !self.images.is_empty() {
            self.current_index = if self.current_index == 0 {
                self.images.len() - 1
            } else {
                self.current_index - 1
            };
        }
    }

    /// Get progress string like "3/15".
    pub fn progress(&self) -> String {
        format!("{}/{}", self.current_index + 1, self.images.len())
    }
}
