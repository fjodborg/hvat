//! Color utility functions shared across the application.
//!
//! This module provides color conversion and generation utilities
//! used by annotations, tags, and other UI components.

/// Convert HSV to RGB.
///
/// # Arguments
/// * `h` - Hue in degrees (0-360)
/// * `s` - Saturation (0.0-1.0)
/// * `v` - Value/brightness (0.0-1.0)
///
/// # Returns
/// RGB tuple with values in range 0.0-1.0
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r + m, g + m, b + m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsv_to_rgb_red() {
        let (r, g, b) = hsv_to_rgb(0.0, 1.0, 1.0);
        assert!((r - 1.0).abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!(b.abs() < 0.01);
    }

    #[test]
    fn test_hsv_to_rgb_green() {
        let (r, g, b) = hsv_to_rgb(120.0, 1.0, 1.0);
        assert!(r.abs() < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!(b.abs() < 0.01);
    }

    #[test]
    fn test_hsv_to_rgb_blue() {
        let (r, g, b) = hsv_to_rgb(240.0, 1.0, 1.0);
        assert!(r.abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!((b - 1.0).abs() < 0.01);
    }
}
