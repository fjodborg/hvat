//! Hyperspectral image data model and band selection.
//!
//! Hyperspectral images contain multiple spectral bands (channels),
//! typically ranging from visible light through near-infrared.
//! This module provides:
//! - Multi-band image storage
//! - RGB composite generation from arbitrary band combinations
//! - Band selection state management

use hvat_ui::{ImageHandle, HyperspectralImageHandle};

/// A hyperspectral image with multiple spectral bands.
#[derive(Clone)]
pub struct HyperspectralImage {
    /// Raw pixel data for all bands, stored as [band0_pixels, band1_pixels, ...]
    /// Each band has width * height f32 values (normalized 0.0-1.0)
    bands: Vec<Vec<f32>>,
    /// Image width in pixels
    pub width: u32,
    /// Image height in u32
    pub height: u32,
    /// Optional wavelength labels for each band (e.g., "450nm", "550nm")
    pub band_labels: Vec<String>,
}

impl HyperspectralImage {
    /// Create a new hyperspectral image from band data.
    ///
    /// # Arguments
    /// * `bands` - Vector of bands, each band is a Vec<f32> with width*height values (0.0-1.0)
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    pub fn new(bands: Vec<Vec<f32>>, width: u32, height: u32) -> Self {
        let num_bands = bands.len();
        let band_labels = (0..num_bands)
            .map(|i| format!("Band {}", i + 1))
            .collect();

        Self {
            bands,
            width,
            height,
            band_labels,
        }
    }

    /// Create a hyperspectral image with custom band labels.
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.band_labels = labels;
        self
    }

    /// Create a hyperspectral image from RGBA data.
    /// Converts the RGBA image into 3 bands (R, G, B), ignoring alpha.
    pub fn from_rgba(data: &[u8], width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        let mut r_band = Vec::with_capacity(pixel_count);
        let mut g_band = Vec::with_capacity(pixel_count);
        let mut b_band = Vec::with_capacity(pixel_count);

        for i in 0..pixel_count {
            let offset = i * 4;
            r_band.push(data[offset] as f32 / 255.0);
            g_band.push(data[offset + 1] as f32 / 255.0);
            b_band.push(data[offset + 2] as f32 / 255.0);
            // Ignore alpha (offset + 3)
        }

        Self {
            bands: vec![r_band, g_band, b_band],
            width,
            height,
            band_labels: vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()],
        }
    }

    /// Get the number of spectral bands.
    pub fn num_bands(&self) -> usize {
        self.bands.len()
    }

    /// Get a reference to a specific band's data.
    pub fn band(&self, index: usize) -> Option<&[f32]> {
        self.bands.get(index).map(|v| v.as_slice())
    }

    /// Get the label for a specific band.
    pub fn band_label(&self, index: usize) -> Option<&str> {
        self.band_labels.get(index).map(|s| s.as_str())
    }

    /// Get a reference to all band data for GPU upload.
    pub fn bands(&self) -> &[Vec<f32>] {
        &self.bands
    }

    /// Create a HyperspectralImageHandle for GPU-based rendering.
    /// This uploads band data to the GPU once, then band selection changes
    /// only require updating a uniform buffer (instant).
    pub fn to_gpu_handle(&self) -> HyperspectralImageHandle {
        HyperspectralImageHandle::from_bands(self.bands.clone(), self.width, self.height)
    }

    /// Generate an RGB composite image from three bands.
    ///
    /// # Arguments
    /// * `r_band` - Band index for red channel
    /// * `g_band` - Band index for green channel
    /// * `b_band` - Band index for blue channel
    ///
    /// # Returns
    /// An ImageHandle containing the RGB composite, or None if band indices are invalid.
    pub fn to_rgb_composite(&self, r_band: usize, g_band: usize, b_band: usize) -> Option<ImageHandle> {
        log::debug!(
            "to_rgb_composite: r_band={}, g_band={}, b_band={}, num_bands={}",
            r_band, g_band, b_band, self.bands.len()
        );

        let r = self.bands.get(r_band)?;
        let g = self.bands.get(g_band)?;
        let b = self.bands.get(b_band)?;

        // Log if all bands are the same (should produce grayscale)
        if r_band == g_band && g_band == b_band {
            log::info!(
                "All bands are the same ({}), output will be grayscale",
                r_band
            );
        }

        let pixel_count = (self.width * self.height) as usize;
        let mut rgba_data = Vec::with_capacity(pixel_count * 4);

        for i in 0..pixel_count {
            // Convert from 0.0-1.0 to 0-255
            let r_val = (r[i].clamp(0.0, 1.0) * 255.0) as u8;
            let g_val = (g[i].clamp(0.0, 1.0) * 255.0) as u8;
            let b_val = (b[i].clamp(0.0, 1.0) * 255.0) as u8;

            rgba_data.push(r_val);
            rgba_data.push(g_val);
            rgba_data.push(b_val);
            rgba_data.push(255); // Alpha
        }

        Some(ImageHandle::from_rgba8(rgba_data, self.width, self.height))
    }

    /// Generate a grayscale image from a single band.
    pub fn to_grayscale(&self, band_index: usize) -> Option<ImageHandle> {
        let band = self.bands.get(band_index)?;

        let pixel_count = (self.width * self.height) as usize;
        let mut rgba_data = Vec::with_capacity(pixel_count * 4);

        for i in 0..pixel_count {
            let val = (band[i].clamp(0.0, 1.0) * 255.0) as u8;
            rgba_data.push(val);
            rgba_data.push(val);
            rgba_data.push(val);
            rgba_data.push(255);
        }

        Some(ImageHandle::from_rgba8(rgba_data, self.width, self.height))
    }
}

/// Band selection state for RGB composite display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandSelection {
    /// Band index for red channel (0-based)
    pub red: usize,
    /// Band index for green channel (0-based)
    pub green: usize,
    /// Band index for blue channel (0-based)
    pub blue: usize,
}

impl BandSelection {
    /// Create a new band selection.
    pub fn new(red: usize, green: usize, blue: usize) -> Self {
        Self { red, green, blue }
    }

    /// Create a default RGB selection (bands 0, 1, 2).
    pub fn default_rgb() -> Self {
        Self::new(0, 1, 2)
    }

    /// Clamp band indices to valid range.
    pub fn clamp(&self, num_bands: usize) -> Self {
        let max_index = num_bands.saturating_sub(1);
        Self {
            red: self.red.min(max_index),
            green: self.green.min(max_index),
            blue: self.blue.min(max_index),
        }
    }
}

impl Default for BandSelection {
    fn default() -> Self {
        Self::default_rgb()
    }
}

/// Generate a fake 8-channel hyperspectral test image.
///
/// Creates an image with different patterns for each band to demonstrate
/// band selection and RGB composite functionality.
pub fn generate_test_hyperspectral(width: u32, height: u32, num_bands: usize) -> HyperspectralImage {
    let pixel_count = (width * height) as usize;
    let mut bands = Vec::with_capacity(num_bands);

    for band_idx in 0..num_bands {
        let mut band_data = Vec::with_capacity(pixel_count);

        for y in 0..height {
            for x in 0..width {
                // Create different patterns for each band
                let fx = x as f32 / width as f32;
                let fy = y as f32 / height as f32;

                let value = match band_idx {
                    0 => {
                        // Band 0: Horizontal gradient (simulates blue-ish visible)
                        fx
                    }
                    1 => {
                        // Band 1: Vertical gradient (simulates green visible)
                        fy
                    }
                    2 => {
                        // Band 2: Diagonal gradient (simulates red visible)
                        (fx + fy) / 2.0
                    }
                    3 => {
                        // Band 3: Inverse horizontal (simulates red edge)
                        1.0 - fx
                    }
                    4 => {
                        // Band 4: Checkerboard pattern (simulates near-infrared)
                        let checker = ((x / 32) + (y / 32)) % 2 == 0;
                        if checker { 0.8 } else { 0.2 }
                    }
                    5 => {
                        // Band 5: Circular pattern (simulates mid-infrared)
                        let cx = fx - 0.5;
                        let cy = fy - 0.5;
                        let dist = (cx * cx + cy * cy).sqrt();
                        (1.0 - dist * 2.0).clamp(0.0, 1.0)
                    }
                    6 => {
                        // Band 6: Concentric rings (simulates far-infrared)
                        let cx = fx - 0.5;
                        let cy = fy - 0.5;
                        let dist = (cx * cx + cy * cy).sqrt();
                        ((dist * 20.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0)
                    }
                    7 => {
                        // Band 7: Noise-like pattern (simulates thermal)
                        let noise = ((x as f32 * 12.9898 + y as f32 * 78.233).sin() * 43758.5453).fract();
                        noise * 0.5 + fy * 0.5
                    }
                    _ => {
                        // Additional bands: gradient with offset
                        let offset = (band_idx as f32 * 0.1) % 1.0;
                        ((fx + offset) % 1.0 + fy) / 2.0
                    }
                };

                band_data.push(value);
            }
        }

        bands.push(band_data);
    }

    // Create wavelength-like labels
    let labels: Vec<String> = (0..num_bands)
        .map(|i| {
            let wavelength = 400 + i * 50; // Simulate 400nm to 750nm+ range
            format!("{}nm", wavelength)
        })
        .collect();

    HyperspectralImage::new(bands, width, height).with_labels(labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperspectral_creation() {
        let img = generate_test_hyperspectral(64, 64, 8);
        assert_eq!(img.num_bands(), 8);
        assert_eq!(img.width, 64);
        assert_eq!(img.height, 64);
    }

    #[test]
    fn test_band_access() {
        let img = generate_test_hyperspectral(32, 32, 4);
        assert!(img.band(0).is_some());
        assert!(img.band(3).is_some());
        assert!(img.band(4).is_none());
    }

    #[test]
    fn test_rgb_composite() {
        let img = generate_test_hyperspectral(32, 32, 8);
        let composite = img.to_rgb_composite(0, 1, 2);
        assert!(composite.is_some());
    }

    #[test]
    fn test_band_selection_clamp() {
        let selection = BandSelection::new(10, 5, 20);
        let clamped = selection.clamp(8);
        assert_eq!(clamped.red, 7);
        assert_eq!(clamped.green, 5);
        assert_eq!(clamped.blue, 7);
    }

    #[test]
    fn test_band_labels() {
        let img = generate_test_hyperspectral(32, 32, 4);
        assert_eq!(img.band_label(0), Some("400nm"));
        assert_eq!(img.band_label(1), Some("450nm"));
        assert_eq!(img.band_label(2), Some("500nm"));
        assert_eq!(img.band_label(3), Some("550nm"));
    }

    #[test]
    fn test_same_band_produces_grayscale() {
        // When all RGB channels map to the same band, output should be grayscale
        let img = generate_test_hyperspectral(32, 32, 8);

        // Use band 1 for all channels
        let composite = img.to_rgb_composite(1, 1, 1).expect("Should create composite");
        let data = composite.data();

        // Check that R, G, B values are equal for each pixel (grayscale)
        let pixel_count = (32 * 32) as usize;
        let mut grayscale_pixels = 0;
        for i in 0..pixel_count {
            let r = data[i * 4];
            let g = data[i * 4 + 1];
            let b = data[i * 4 + 2];
            if r == g && g == b {
                grayscale_pixels += 1;
            }
        }

        // All pixels should be grayscale
        assert_eq!(grayscale_pixels, pixel_count,
            "Expected all {} pixels to be grayscale, but only {} were",
            pixel_count, grayscale_pixels);
    }
}
