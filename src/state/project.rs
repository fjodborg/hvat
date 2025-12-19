//! Project state management for loaded folders and images.

use std::path::PathBuf;

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
    /// List of image files in the folder
    pub images: Vec<PathBuf>,
    /// Current image index
    pub current_index: usize,
    /// In-memory image data for WASM (where we can't access filesystem)
    pub loaded_images: Vec<LoadedImage>,
}

impl ProjectState {
    /// Discover image files in a folder (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_folder(folder: PathBuf) -> Result<Self, String> {
        let mut images: Vec<PathBuf> = std::fs::read_dir(&folder)
            .map_err(|e| format!("Failed to read folder: {}", e))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| {
                        matches!(
                            ext.to_lowercase().as_str(),
                            "png" | "jpg" | "jpeg" | "bmp" | "tiff" | "tif" | "webp"
                        )
                    })
                    .unwrap_or(false)
            })
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
    pub fn current_name(&self) -> String {
        self.current_image()
            .and_then(|p| p.file_name())
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
