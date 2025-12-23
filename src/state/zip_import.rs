//! ZIP file import functionality.
//!
//! This module provides utilities for extracting images from ZIP archives
//! for both native and WASM targets.

#[cfg(target_arch = "wasm32")]
use std::io::Cursor;
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use zip::ZipArchive;

use super::LoadedImage;
use super::project::IMAGE_EXTENSIONS;

/// Check if a filename has a ZIP extension.
#[cfg(target_arch = "wasm32")]
pub fn is_zip_file(filename: &str) -> bool {
    filename.to_lowercase().ends_with(".zip")
}

/// Check if a path is a ZIP file.
#[cfg(not(target_arch = "wasm32"))]
pub fn is_zip_path(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("zip"))
        .unwrap_or(false)
}

/// Check if a filename has a supported image extension (for ZIP entry filtering).
fn is_image_entry(name: &str) -> bool {
    let lower = name.to_lowercase();
    // Skip hidden files and macOS metadata
    if lower.contains("__macosx") || lower.contains("/.") || lower.starts_with('.') {
        return false;
    }
    IMAGE_EXTENSIONS
        .iter()
        .any(|ext| lower.ends_with(&format!(".{}", ext)))
}

/// Extract images from a ZIP file loaded as bytes.
/// Returns a list of `LoadedImage` with paths relative to the ZIP root.
///
/// This function works on both native and WASM targets since it operates
/// on in-memory data.
#[cfg(target_arch = "wasm32")]
pub fn extract_images_from_zip_bytes(
    zip_data: &[u8],
    zip_filename: &str,
) -> Result<Vec<LoadedImage>, String> {
    log::info!(
        "Extracting images from ZIP '{}' ({} bytes)",
        zip_filename,
        zip_data.len()
    );

    let cursor = Cursor::new(zip_data);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| format!("Failed to read ZIP archive: {}", e))?;

    let mut images = Vec::new();
    let file_count = archive.len();

    log::debug!("ZIP contains {} entries", file_count);

    for i in 0..file_count {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry {}: {}", i, e))?;

        let name = file.name().to_string();

        // Skip directories
        if file.is_dir() {
            log::trace!("Skipping directory: {}", name);
            continue;
        }

        // Check if it's an image file
        if !is_image_entry(&name) {
            log::trace!("Skipping non-image: {}", name);
            continue;
        }

        // Read file contents
        let mut data = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut data)
            .map_err(|e| format!("Failed to read '{}' from ZIP: {}", name, e))?;

        log::debug!("Extracted image '{}' ({} bytes)", name, data.len());

        images.push(LoadedImage { name, data });
    }

    if images.is_empty() {
        return Err(format!(
            "No image files found in ZIP archive '{}'",
            zip_filename
        ));
    }

    // Sort images by name for consistent ordering
    images.sort_by(|a, b| a.name.cmp(&b.name));

    log::info!(
        "Extracted {} images from ZIP '{}'",
        images.len(),
        zip_filename
    );

    Ok(images)
}

/// Extract images from a ZIP file on disk (native only).
/// Returns a list of `LoadedImage` with paths relative to the ZIP root.
#[cfg(not(target_arch = "wasm32"))]
pub fn extract_images_from_zip_file(path: &PathBuf) -> Result<Vec<LoadedImage>, String> {
    log::info!("Opening ZIP file: {:?}", path);

    let file = std::fs::File::open(path).map_err(|e| format!("Failed to open ZIP file: {}", e))?;

    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP archive: {}", e))?;

    let mut images = Vec::new();
    let file_count = archive.len();

    log::debug!("ZIP contains {} entries", file_count);

    for i in 0..file_count {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry {}: {}", i, e))?;

        let name = file.name().to_string();

        // Skip directories
        if file.is_dir() {
            log::trace!("Skipping directory: {}", name);
            continue;
        }

        // Check if it's an image file
        if !is_image_entry(&name) {
            log::trace!("Skipping non-image: {}", name);
            continue;
        }

        // Read file contents
        let mut data = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut data)
            .map_err(|e| format!("Failed to read '{}' from ZIP: {}", name, e))?;

        log::debug!("Extracted image '{}' ({} bytes)", name, data.len());

        images.push(LoadedImage { name, data });
    }

    if images.is_empty() {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.zip");
        return Err(format!(
            "No image files found in ZIP archive '{}'",
            filename
        ));
    }

    // Sort images by name for consistent ordering
    images.sort_by(|a, b| a.name.cmp(&b.name));

    log::info!("Extracted {} images from ZIP file", images.len());

    Ok(images)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_zip_file() {
        assert!(is_zip_file("archive.zip"));
        assert!(is_zip_file("archive.ZIP"));
        assert!(is_zip_file("my.archive.zip"));
        assert!(!is_zip_file("image.png"));
        assert!(!is_zip_file("zipfile.txt"));
    }

    #[test]
    fn test_is_image_entry() {
        assert!(is_image_entry("image.png"));
        assert!(is_image_entry("folder/image.jpg"));
        assert!(is_image_entry("deep/folder/image.JPEG"));
        assert!(!is_image_entry("__MACOSX/._image.png"));
        assert!(!is_image_entry(".hidden.png"));
        assert!(!is_image_entry("folder/.hidden.jpg"));
        assert!(!is_image_entry("document.txt"));
    }
}
