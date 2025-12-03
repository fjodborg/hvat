//! Shared uniform types for GPU rendering pipelines.

use bytemuck::{Pod, Zeroable};

/// 4x4 transform matrix for pan/zoom operations.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TransformUniform {
    pub matrix: [[f32; 4]; 4],
}

impl TransformUniform {
    pub fn new() -> Self {
        Self {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Create transform from offset and zoom.
    pub fn from_transform(offset_x: f32, offset_y: f32, zoom: f32) -> Self {
        Self {
            matrix: [
                [zoom, 0.0, 0.0, 0.0],
                [0.0, zoom, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [offset_x, offset_y, 0.0, 1.0],
            ],
        }
    }
}

impl Default for TransformUniform {
    fn default() -> Self {
        Self::new()
    }
}

/// Image adjustment parameters for real-time image processing.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ImageAdjustments {
    /// Additive brightness adjustment.
    pub brightness: f32,
    /// Multiplicative contrast (1.0 = no change).
    pub contrast: f32,
    /// Gamma correction exponent (1.0 = no change).
    pub gamma: f32,
    /// Hue rotation in degrees.
    pub hue_shift: f32,
}

impl ImageAdjustments {
    pub fn new() -> Self {
        Self {
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            hue_shift: 0.0,
        }
    }
}

impl Default for ImageAdjustments {
    fn default() -> Self {
        Self::new()
    }
}
