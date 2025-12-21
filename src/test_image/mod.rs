//! Test hyperspectral image generation.
//!
//! Provides functions to generate test hyperspectral images with various patterns
//! for testing and demonstration purposes.

mod ascii_art;
mod font;

use crate::data::HyperspectralData;

use ascii_art::render_ascii_art;
use font::render_test_text;

/// Embedded HVAT logo icon (1024x1024 PNG format for high-quality display)
const HVAT_ICON: &[u8] = include_bytes!("../../assets/icon-1024.png");

/// Loaded and scaled HVAT icon data (R, G, B channels)
struct HvatIconData {
    red: Vec<f32>,
    green: Vec<f32>,
    blue: Vec<f32>,
    width: u32,
    height: u32,
}

impl HvatIconData {
    /// Load the HVAT icon and scale it to the target dimensions
    fn load(target_width: u32, target_height: u32) -> Option<Self> {
        use image::GenericImageView;

        let img = match image::load_from_memory(HVAT_ICON) {
            Ok(img) => img,
            Err(e) => {
                log::warn!("Failed to load HVAT icon: {}", e);
                return None;
            }
        };

        // Scale the icon to fit the target dimensions while maintaining aspect ratio
        let (icon_w, icon_h) = img.dimensions();
        let scale = (target_width as f32 / icon_w as f32)
            .min(target_height as f32 / icon_h as f32)
            .min(1.0); // Don't upscale beyond original size

        let scaled_w = (icon_w as f32 * scale) as u32;
        let scaled_h = (icon_h as f32 * scale) as u32;

        let scaled = image::imageops::resize(
            &img.to_rgba8(),
            scaled_w,
            scaled_h,
            image::imageops::FilterType::Lanczos3,
        );

        // Calculate offset to center the icon
        let offset_x = (target_width.saturating_sub(scaled_w)) / 2;
        let offset_y = (target_height.saturating_sub(scaled_h)) / 2;

        let pixel_count = (target_width * target_height) as usize;
        let mut red = vec![0.0f32; pixel_count];
        let mut green = vec![0.0f32; pixel_count];
        let mut blue = vec![0.0f32; pixel_count];

        // Copy scaled icon into centered position
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let pixel = scaled.get_pixel(x, y);
                let target_x = x + offset_x;
                let target_y = y + offset_y;

                if target_x < target_width && target_y < target_height {
                    let idx = (target_y * target_width + target_x) as usize;
                    let alpha = pixel[3] as f32 / 255.0;

                    // Premultiply with alpha for proper blending
                    red[idx] = (pixel[0] as f32 / 255.0) * alpha;
                    green[idx] = (pixel[1] as f32 / 255.0) * alpha;
                    blue[idx] = (pixel[2] as f32 / 255.0) * alpha;
                }
            }
        }

        log::info!(
            "Loaded HVAT icon: {}x{} scaled to {}x{}, centered in {}x{}",
            icon_w,
            icon_h,
            scaled_w,
            scaled_h,
            target_width,
            target_height
        );

        Some(Self {
            red,
            green,
            blue,
            width: target_width,
            height: target_height,
        })
    }

    /// Get the value for a specific channel at the given coordinates
    fn get_channel(&self, channel: usize, x: u32, y: u32) -> f32 {
        if x >= self.width || y >= self.height {
            return 0.0;
        }
        let idx = (y * self.width + x) as usize;
        match channel {
            0 => self.red[idx],
            1 => self.green[idx],
            2 => self.blue[idx],
            _ => 0.0,
        }
    }
}

/// Generate a test hyperspectral image with different patterns per band.
///
/// Each band has a unique visual pattern to help distinguish them when
/// switching between bands in the viewer:
/// - Band 0: HVAT logo Red channel
/// - Band 1: HVAT logo Green channel
/// - Band 2: HVAT logo Blue channel
/// - Band 3: Text overlay with horizontal gradient background
/// - Band 4: Vertical gradient
/// - Band 5: Checkerboard pattern
/// - Band 6: Circular pattern (radial gradient)
/// - Band 7: Concentric rings
/// - Band 8: Noise-like pattern
/// - Band 9: ASCII art (Braille character rendering)
/// - Band 10+: Offset gradients
pub fn generate_test_hyperspectral(width: u32, height: u32, num_bands: usize) -> HyperspectralData {
    log::info!(
        "Generating test hyperspectral image: {}x{} with {} bands",
        width,
        height,
        num_bands
    );

    // Load the HVAT icon for the first 3 bands
    let icon_data = HvatIconData::load(width, height);

    let pixel_count = (width * height) as usize;
    let mut bands = Vec::with_capacity(num_bands);

    for band_idx in 0..num_bands {
        let mut band_data = Vec::with_capacity(pixel_count);

        for y in 0..height {
            for x in 0..width {
                let value = generate_band_pixel(band_idx, x, y, width, height, icon_data.as_ref());
                band_data.push(value);
            }
        }

        bands.push(band_data);
    }

    // Update labels to reflect the new band assignments
    let labels: Vec<String> = (0..num_bands)
        .map(|i| match i {
            0 => "Red (HVAT)".to_string(),
            1 => "Green (HVAT)".to_string(),
            2 => "Blue (HVAT)".to_string(),
            _ => {
                let wavelength = 400 + i * 50;
                format!("{}nm", wavelength)
            }
        })
        .collect();

    log::info!("Hyperspectral image generation complete");

    HyperspectralData::new(bands, width, height, labels)
}

/// Generate the pixel value for a specific band at the given coordinates.
fn generate_band_pixel(
    band_idx: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    icon_data: Option<&HvatIconData>,
) -> f32 {
    let fx = x as f32 / width as f32;
    let fy = y as f32 / height as f32;

    match band_idx {
        // First 3 bands: HVAT logo RGB channels
        0 => icon_data.map_or(fx, |icon| icon.get_channel(0, x, y)),
        1 => icon_data.map_or(fy, |icon| icon.get_channel(1, x, y)),
        2 => icon_data.map_or((fx + fy) / 2.0, |icon| icon.get_channel(2, x, y)),

        // Band 3: Text overlay with horizontal gradient background
        3 => {
            let text_value = render_test_text(x, y, width, height);
            if text_value > 0.0 {
                text_value
            } else {
                fx * 0.3
            }
        }

        // Band 4: Vertical gradient
        4 => fy,

        // Band 5: Checkerboard
        5 => {
            let checker = ((x / 32) + (y / 32)) % 2 == 0;
            if checker { 0.8 } else { 0.2 }
        }

        // Band 6: Circular pattern
        6 => {
            let cx = fx - 0.5;
            let cy = fy - 0.5;
            let dist = (cx * cx + cy * cy).sqrt();
            (1.0 - dist * 2.0).clamp(0.0, 1.0)
        }

        // Band 7: Concentric rings
        7 => {
            let cx = fx - 0.5;
            let cy = fy - 0.5;
            let dist = (cx * cx + cy * cy).sqrt();
            ((dist * 20.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0)
        }

        // Band 8: Noise-like pattern
        8 => {
            let noise = ((x as f32 * 12.9898 + y as f32 * 78.233).sin() * 43758.5453).fract();
            noise * 0.5 + fy * 0.5
        }

        // Band 9: ASCII art
        9 => render_ascii_art(x, y, width, height),

        // Band 10+: Offset gradients
        _ => {
            let offset = (band_idx as f32 * 0.1) % 1.0;
            ((fx + offset) % 1.0 + fy) / 2.0
        }
    }
}
