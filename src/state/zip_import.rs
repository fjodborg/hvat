//! ZIP file import functionality.
//!
//! This module provides utilities for extracting images from ZIP archives
//! for both native and WASM targets.

#[cfg(target_arch = "wasm32")]
use std::io::Cursor;
use std::io::{Read, Seek};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use zip::ZipArchive;

use super::LoadedImage;
use super::project::is_supported_filename;

/// Check if a filename has a ZIP extension.
#[cfg(target_arch = "wasm32")]
pub fn is_zip_file(filename: &str) -> bool {
    filename.to_lowercase().ends_with(".zip")
}

/// Check if a path is a ZIP file.
#[cfg(not(target_arch = "wasm32"))]
pub fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("zip"))
        .unwrap_or(false)
}

/// Check if a filename has a supported extension (for ZIP entry filtering).
fn is_supported_entry(name: &str) -> bool {
    let lower = name.to_lowercase();
    // Skip hidden files and macOS metadata
    if lower.contains("__macosx") || lower.contains("/.") || lower.starts_with('.') {
        return false;
    }
    is_supported_filename(name)
}

/// Extract images from a ZIP archive reader.
///
/// This is the core extraction logic used by both platform-specific functions.
/// It accepts any type that implements `Read + Seek`, allowing it to work with
/// both in-memory data (`Cursor<&[u8]>`) and files (`std::fs::File`).
fn extract_images_from_archive<R: Read + Seek>(
    reader: R,
    archive_name: &str,
) -> Result<Vec<LoadedImage>, String> {
    let mut archive =
        ZipArchive::new(reader).map_err(|e| format!("Failed to read ZIP archive: {}", e))?;

    let mut images = Vec::new();
    let file_count = archive.len();

    log::debug!("ZIP '{}' contains {} entries", archive_name, file_count);

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

        // Check if it's a supported file
        if !is_supported_entry(&name) {
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
            archive_name
        ));
    }

    // Sort images by name for consistent ordering
    images.sort_by(|a, b| a.name.cmp(&b.name));

    log::info!(
        "Extracted {} images from ZIP '{}'",
        images.len(),
        archive_name
    );

    Ok(images)
}

/// Extract images from a ZIP file loaded as bytes (WASM).
///
/// Returns a list of `LoadedImage` with paths relative to the ZIP root.
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
    extract_images_from_archive(cursor, zip_filename)
}

/// Extract images from a ZIP file on disk (native only).
///
/// Returns a list of `LoadedImage` with paths relative to the ZIP root.
#[cfg(not(target_arch = "wasm32"))]
pub fn extract_images_from_zip_file(path: &Path) -> Result<Vec<LoadedImage>, String> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.zip");

    log::info!("Opening ZIP file: {:?}", path);

    let file = std::fs::File::open(path).map_err(|e| format!("Failed to open ZIP file: {}", e))?;

    extract_images_from_archive(file, filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_is_zip_file() {
        assert!(is_zip_file("archive.zip"));
        assert!(is_zip_file("archive.ZIP"));
        assert!(is_zip_file("my.archive.zip"));
        assert!(!is_zip_file("image.png"));
        assert!(!is_zip_file("zipfile.txt"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_is_zip_path() {
        use std::path::PathBuf;
        assert!(is_zip_path(&PathBuf::from("archive.zip")));
        assert!(is_zip_path(&PathBuf::from("archive.ZIP")));
        assert!(is_zip_path(&PathBuf::from("my.archive.zip")));
        assert!(!is_zip_path(&PathBuf::from("image.png")));
        assert!(!is_zip_path(&PathBuf::from("zipfile.txt")));
    }

    #[test]
    fn test_is_supported_entry() {
        assert!(is_supported_entry("image.png"));
        assert!(is_supported_entry("folder/image.jpg"));
        assert!(is_supported_entry("deep/folder/image.JPEG"));
        assert!(is_supported_entry("data.npy")); // NumPy files are now supported
        assert!(!is_supported_entry("__MACOSX/._image.png"));
        assert!(!is_supported_entry(".hidden.png"));
        assert!(!is_supported_entry("folder/.hidden.jpg"));
        assert!(!is_supported_entry("document.txt"));
    }
}
