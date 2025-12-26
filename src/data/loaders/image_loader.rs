//! Loader for standard image formats (PNG, JPEG, BMP, TIFF, WebP).
//!
//! Converts RGB images to 3-band hyperspectral data.

use crate::data::HyperspectralData;
use crate::data::loader::{HyperspectralLoader, LoaderError};

/// Loader for standard image formats.
///
/// Supports PNG, JPEG, BMP, TIFF, and WebP formats.
/// Extracts RGB channels as 3 separate bands normalized to 0.0-1.0.
pub struct ImageLoader;

impl HyperspectralLoader for ImageLoader {
    fn id(&self) -> &'static str {
        "image"
    }

    fn display_name(&self) -> &'static str {
        "Standard Image (RGB)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"]
    }

    fn can_load(&self, data: &[u8]) -> bool {
        // Check common image magic bytes
        if data.len() < 8 {
            return false;
        }

        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return true;
        }

        // JPEG: FF D8 FF
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return true;
        }

        // BMP: 42 4D (BM)
        if data.starts_with(&[0x42, 0x4D]) {
            return true;
        }

        // TIFF: 49 49 2A 00 (little endian) or 4D 4D 00 2A (big endian)
        if data.starts_with(&[0x49, 0x49, 0x2A, 0x00])
            || data.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
        {
            return true;
        }

        // WebP: RIFF....WEBP
        if data.len() >= 12
            && data.starts_with(&[0x52, 0x49, 0x46, 0x46])
            && &data[8..12] == b"WEBP"
        {
            return true;
        }

        false
    }

    fn load(&self, data: &[u8]) -> Result<HyperspectralData, LoaderError> {
        let img = image::load_from_memory(data)
            .map_err(|e| LoaderError::new(format!("Failed to decode image: {}", e)))?
            .to_rgba8();

        let width = img.width();
        let height = img.height();
        let pixel_count = (width * height) as usize;

        let mut r_band = Vec::with_capacity(pixel_count);
        let mut g_band = Vec::with_capacity(pixel_count);
        let mut b_band = Vec::with_capacity(pixel_count);

        for pixel in img.pixels() {
            r_band.push(f32::from(pixel[0]) / 255.0);
            g_band.push(f32::from(pixel[1]) / 255.0);
            b_band.push(f32::from(pixel[2]) / 255.0);
        }

        log::trace!(
            "ImageLoader: loaded {}x{} image as 3 bands (RGB)",
            width,
            height
        );

        Ok(HyperspectralData::new(
            vec![r_band, g_band, b_band],
            width,
            height,
            vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()],
        ))
    }

    fn priority(&self) -> i32 {
        // Standard images have lower priority than specialized formats
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_metadata() {
        let loader = ImageLoader;
        assert_eq!(loader.id(), "image");
        assert!(loader.extensions().contains(&"png"));
        assert!(loader.extensions().contains(&"jpg"));
    }

    #[test]
    fn test_magic_detection_png() {
        let loader = ImageLoader;
        let png_magic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(loader.can_load(&png_magic));
    }

    #[test]
    fn test_magic_detection_jpeg() {
        let loader = ImageLoader;
        let jpeg_magic = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert!(loader.can_load(&jpeg_magic));
    }

    #[test]
    fn test_magic_detection_invalid() {
        let loader = ImageLoader;
        let random_data = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        assert!(!loader.can_load(&random_data));
    }
}
