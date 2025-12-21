//! Test hyperspectral image generation.
//!
//! Provides functions to generate test hyperspectral images with various patterns
//! for testing and demonstration purposes.

mod ascii_art;
mod font;

use crate::data::HyperspectralData;

use ascii_art::render_ascii_art;
use font::render_test_text;

/// Generate a test hyperspectral image with different patterns per band.
///
/// Each band has a unique visual pattern to help distinguish them when
/// switching between bands in the viewer:
/// - Band 0: Text overlay with horizontal gradient background
/// - Band 1: Vertical gradient
/// - Band 2: Diagonal gradient
/// - Band 3: Inverse horizontal gradient
/// - Band 4: Checkerboard pattern
/// - Band 5: Circular pattern (radial gradient)
/// - Band 6: Concentric rings
/// - Band 7: Noise-like pattern
/// - Band 8: ASCII art (Braille character rendering)
/// - Band 9+: Offset gradients
pub fn generate_test_hyperspectral(width: u32, height: u32, num_bands: usize) -> HyperspectralData {
    log::info!(
        "Generating test hyperspectral image: {}x{} with {} bands",
        width,
        height,
        num_bands
    );
    let pixel_count = (width * height) as usize;
    let mut bands = Vec::with_capacity(num_bands);

    for band_idx in 0..num_bands {
        let mut band_data = Vec::with_capacity(pixel_count);

        for y in 0..height {
            for x in 0..width {
                let value = generate_band_pixel(band_idx, x, y, width, height);
                band_data.push(value);
            }
        }

        bands.push(band_data);
    }

    let labels: Vec<String> = (0..num_bands)
        .map(|i| {
            let wavelength = 400 + i * 50;
            format!("{}nm", wavelength)
        })
        .collect();

    log::info!("Hyperspectral image generation complete");

    HyperspectralData::new(bands, width, height, labels)
}

/// Generate the pixel value for a specific band at the given coordinates.
fn generate_band_pixel(band_idx: usize, x: u32, y: u32, width: u32, height: u32) -> f32 {
    let fx = x as f32 / width as f32;
    let fy = y as f32 / height as f32;

    match band_idx {
        0 => {
            // Band 0: Display text in center
            let text_value = render_test_text(x, y, width, height);
            if text_value > 0.0 {
                text_value
            } else {
                fx * 0.3
            }
        }
        1 => fy,              // Vertical gradient
        2 => (fx + fy) / 2.0, // Diagonal
        3 => 1.0 - fx,        // Inverse horizontal
        4 => {
            // Checkerboard
            let checker = ((x / 32) + (y / 32)) % 2 == 0;
            if checker { 0.8 } else { 0.2 }
        }
        5 => {
            // Circular pattern
            let cx = fx - 0.5;
            let cy = fy - 0.5;
            let dist = (cx * cx + cy * cy).sqrt();
            (1.0 - dist * 2.0).clamp(0.0, 1.0)
        }
        6 => {
            // Concentric rings
            let cx = fx - 0.5;
            let cy = fy - 0.5;
            let dist = (cx * cx + cy * cy).sqrt();
            ((dist * 20.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0)
        }
        7 => {
            // Noise-like pattern
            let noise = ((x as f32 * 12.9898 + y as f32 * 78.233).sin() * 43758.5453).fract();
            noise * 0.5 + fy * 0.5
        }
        8 => render_ascii_art(x, y, width, height), // ASCII art
        _ => {
            let offset = (band_idx as f32 * 0.1) % 1.0;
            ((fx + offset) % 1.0 + fy) / 2.0
        }
    }
}
