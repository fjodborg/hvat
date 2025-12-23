//! Project state management for loaded folders and images.

use std::collections::BTreeMap;
use std::path::PathBuf;

use hvat_ui::FileTreeNode;

/// Supported image extensions
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"];

/// Check if a filename (string) has a supported image extension.
/// Works with both full paths and just filenames.
#[cfg(target_arch = "wasm32")]
pub fn is_image_filename(name: &str) -> bool {
    let lower = name.to_lowercase();
    IMAGE_EXTENSIONS
        .iter()
        .any(|ext| lower.ends_with(&format!(".{}", ext)))
}

/// Check if a path has a supported image extension
#[cfg(not(target_arch = "wasm32"))]
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

    /// Create project from loaded image data.
    ///
    /// Used for:
    /// - WASM: Images loaded from browser file picker or drag-drop
    /// - Native: Images extracted from ZIP archives
    pub fn from_loaded_images(loaded_images: Vec<LoadedImage>) -> Result<Self, String> {
        if loaded_images.is_empty() {
            return Err("No images loaded".to_string());
        }

        log::info!(
            "from_loaded_images: creating project from {} loaded images",
            loaded_images.len()
        );

        // Create virtual paths from image names
        let images: Vec<PathBuf> = loaded_images
            .iter()
            .map(|img| PathBuf::from(&img.name))
            .collect();

        log::info!("from_loaded_images: created {} image paths", images.len());
        for (i, img) in images.iter().enumerate() {
            log::debug!("  Image {}: {:?}", i, img);
        }

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

    /// Build a file tree from the loaded images.
    ///
    /// Creates a hierarchical tree structure from the flat list of image paths,
    /// organized by folder. Files at the root level are included as root nodes.
    ///
    /// Returns a list of root-level nodes (can be folders or files).
    pub fn build_file_tree(&self) -> Vec<FileTreeNode> {
        // Group images by their relative folder path
        let mut folder_files: BTreeMap<String, Vec<(String, usize)>> = BTreeMap::new();

        for (idx, path) in self.images.iter().enumerate() {
            // Get relative path from project folder
            let relative = if !self.folder.as_os_str().is_empty() {
                path.strip_prefix(&self.folder)
                    .ok()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| path.clone())
            } else {
                path.clone()
            };

            // Extract parent folder and filename
            let (folder_path, filename) = if let Some(parent) = relative.parent() {
                let folder_str = parent.to_str().unwrap_or("").to_string();
                let filename = relative
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                (folder_str, filename)
            } else {
                let filename = relative
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                (String::new(), filename)
            };

            folder_files
                .entry(folder_path)
                .or_default()
                .push((filename, idx));
        }

        // Build the tree structure
        // First, collect all unique folder paths and create a hierarchy
        let mut root_nodes: Vec<FileTreeNode> = Vec::new();
        let mut folder_nodes: BTreeMap<String, FileTreeNode> = BTreeMap::new();

        // Process folders in order (BTreeMap sorts keys)
        for (folder_path, files) in &folder_files {
            if folder_path.is_empty() {
                // Root-level files
                for (filename, idx) in files {
                    let file_path = filename.clone();
                    root_nodes.push(FileTreeNode::File {
                        path: file_path,
                        name: filename.clone(),
                        index: Some(*idx),
                    });
                }
            } else {
                // Files in a subfolder - we need to create folder hierarchy
                // Split path into components and build nested folders
                let components: Vec<&str> = folder_path.split(['/', '\\']).collect();

                // Build path incrementally to create nested folders
                let mut current_path = String::new();
                for (i, component) in components.iter().enumerate() {
                    if i > 0 {
                        current_path.push('/');
                    }
                    current_path.push_str(component);

                    // Create folder node if it doesn't exist
                    if !folder_nodes.contains_key(&current_path) {
                        folder_nodes.insert(
                            current_path.clone(),
                            FileTreeNode::Folder {
                                path: current_path.clone(),
                                name: component.to_string(),
                                children: Vec::new(),
                            },
                        );
                    }
                }

                // Add files to the deepest folder
                if let Some(FileTreeNode::Folder { children, .. }) =
                    folder_nodes.get_mut(folder_path)
                {
                    for (filename, idx) in files {
                        let file_path = format!("{}/{}", folder_path, filename);
                        children.push(FileTreeNode::File {
                            path: file_path,
                            name: filename.clone(),
                            index: Some(*idx),
                        });
                    }
                }
            }
        }

        // Now build the tree by nesting folders appropriately
        // Folders with no parent go into root_nodes
        // Folders with parents get added to their parent's children

        // Sort folder paths by depth (shallow first) for proper nesting
        let mut folder_paths: Vec<String> = folder_nodes.keys().cloned().collect();
        folder_paths.sort_by_key(|p| p.matches('/').count());

        // Process deepest folders first (reverse order) to build up the tree
        for folder_path in folder_paths.iter().rev() {
            if let Some(folder_node) = folder_nodes.remove(folder_path) {
                // Find parent path
                if let Some(last_sep) = folder_path.rfind('/') {
                    let parent_path = &folder_path[..last_sep];
                    if let Some(FileTreeNode::Folder { children, .. }) =
                        folder_nodes.get_mut(parent_path)
                    {
                        children.push(folder_node);
                    }
                } else {
                    // Top-level folder, add to root
                    root_nodes.push(folder_node);
                }
            }
        }

        // Sort root nodes: folders first, then files, both alphabetically
        root_nodes.sort_by(|a, b| match (a, b) {
            (
                FileTreeNode::Folder { name: a_name, .. },
                FileTreeNode::Folder { name: b_name, .. },
            ) => a_name.cmp(b_name),
            (FileTreeNode::File { name: a_name, .. }, FileTreeNode::File { name: b_name, .. }) => {
                a_name.cmp(b_name)
            }
            (FileTreeNode::Folder { .. }, FileTreeNode::File { .. }) => std::cmp::Ordering::Less,
            (FileTreeNode::File { .. }, FileTreeNode::Folder { .. }) => std::cmp::Ordering::Greater,
        });

        root_nodes
    }

    /// Get the relative path of the current image (for tree selection).
    pub fn current_relative_path(&self) -> Option<String> {
        let path = self.current_image()?;

        if !self.folder.as_os_str().is_empty() {
            if let Ok(relative) = path.strip_prefix(&self.folder) {
                return relative.to_str().map(String::from);
            }
        }

        path.to_str().map(String::from)
    }
}
