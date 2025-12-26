//! Loader for NumPy `.npy` files.
//!
//! Supports loading hyperspectral data stored as NumPy arrays.
//! Handles various array shapes and data types.

use std::io::Cursor;

use ndarray::ArrayD;
use ndarray_npy::ReadNpyExt;

use crate::data::HyperspectralData;
use crate::data::loader::{HyperspectralLoader, LoaderError};

/// Loader for NumPy `.npy` files.
///
/// **Expected array shapes** (width, height convention):
/// - 2D `(W, H)`: Single-band grayscale
/// - 3D `(B, W, H)`: Bands-first format (common for hyperspectral)
/// - 3D `(W, H, B)`: Channels-last format (common in image processing)
///
/// Supported data types: `f32`, `f64`, `u8`, `u16`, `i16`, `i32`.
/// Values are normalized to 0.0-1.0 range based on data type.
pub struct NpyLoader;

impl NpyLoader {
    /// NumPy magic bytes: \x93NUMPY
    const MAGIC: &'static [u8] = &[0x93, b'N', b'U', b'M', b'P', b'Y'];

    /// Convert array to HyperspectralData, handling different shapes.
    ///
    /// Uses (width, height) convention throughout:
    /// - 2D: shape[0] = width, shape[1] = height
    /// - 3D bands-first: shape[0] = bands, shape[1] = width, shape[2] = height
    /// - 3D channels-last: shape[0] = width, shape[1] = height, shape[2] = bands
    fn array_to_hyperspectral<T>(array: ArrayD<T>) -> Result<HyperspectralData, LoaderError>
    where
        T: NumericConvert + Copy,
    {
        let shape = array.shape();
        log::debug!("NpyLoader: array shape = {:?}", shape);

        match shape.len() {
            2 => {
                // 2D array: (W, H) - single band grayscale
                let width = shape[0] as u32;
                let height = shape[1] as u32;

                let band: Vec<f32> = array.iter().map(|&v| v.to_normalized_f32()).collect();

                log::info!(
                    "NpyLoader: loaded {}x{} as single-band grayscale",
                    width,
                    height
                );

                Ok(HyperspectralData::new(
                    vec![band],
                    width,
                    height,
                    vec!["Band 1".to_string()],
                ))
            }
            3 => {
                // 3D array: need to determine if (B, W, H) or (W, H, B)
                // Heuristic: if first dimension is small (<= 100) and third is larger,
                // assume (B, W, H). Otherwise assume (W, H, B).
                let (num_bands, width, height) = if shape[0] <= 100 && shape[2] > shape[0] {
                    // (B, W, H) format - bands first
                    (shape[0], shape[1] as u32, shape[2] as u32)
                } else if shape[2] <= 100 && shape[0] > shape[2] {
                    // (W, H, B) format - channels last
                    (shape[2], shape[0] as u32, shape[1] as u32)
                } else {
                    // Ambiguous - default to (B, W, H) as it's more common for hyperspectral
                    log::warn!(
                        "NpyLoader: ambiguous 3D shape {:?}, assuming (bands, width, height)",
                        shape
                    );
                    (shape[0], shape[1] as u32, shape[2] as u32)
                };

                let pixel_count = (width * height) as usize;
                let mut bands = Vec::with_capacity(num_bands);
                let mut labels = Vec::with_capacity(num_bands);

                // Determine layout
                let is_bands_first = shape[0] <= 100 && shape[2] > shape[0];

                for b in 0..num_bands {
                    let mut band_data = Vec::with_capacity(pixel_count);

                    if is_bands_first || (shape[2] > 100 && shape[0] <= 100) {
                        // (B, W, H) - bands first layout
                        for w in 0..width as usize {
                            for h in 0..height as usize {
                                let idx = b * (width as usize * height as usize)
                                    + w * height as usize
                                    + h;
                                band_data.push(array.as_slice().unwrap()[idx].to_normalized_f32());
                            }
                        }
                    } else {
                        // (W, H, B) - channels last layout
                        for w in 0..width as usize {
                            for h in 0..height as usize {
                                let idx = w * (height as usize * num_bands) + h * num_bands + b;
                                band_data.push(array.as_slice().unwrap()[idx].to_normalized_f32());
                            }
                        }
                    }

                    bands.push(band_data);
                    labels.push(format!("Band {}", b + 1));
                }

                log::info!(
                    "NpyLoader: loaded {}x{} with {} bands",
                    width,
                    height,
                    num_bands
                );

                Ok(HyperspectralData::new(bands, width, height, labels))
            }
            _ => Err(LoaderError::new(format!(
                "Unsupported array dimensions: {} (expected 2 or 3)",
                shape.len()
            ))),
        }
    }
}

impl HyperspectralLoader for NpyLoader {
    fn id(&self) -> &'static str {
        "npy"
    }

    fn display_name(&self) -> &'static str {
        "NumPy Array (.npy)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["npy"]
    }

    fn can_load(&self, data: &[u8]) -> bool {
        data.len() >= Self::MAGIC.len() && data.starts_with(Self::MAGIC)
    }

    fn load(&self, data: &[u8]) -> Result<HyperspectralData, LoaderError> {
        let mut cursor = Cursor::new(data);

        // Try different numeric types in order of likelihood
        // f32 is most common for scientific data
        if let Ok(array) = ArrayD::<f32>::read_npy(&mut cursor) {
            return Self::array_to_hyperspectral(array);
        }

        // Reset cursor and try f64
        cursor.set_position(0);
        if let Ok(array) = ArrayD::<f64>::read_npy(&mut cursor) {
            return Self::array_to_hyperspectral(array);
        }

        // Reset cursor and try u8 (common for image-like data)
        cursor.set_position(0);
        if let Ok(array) = ArrayD::<u8>::read_npy(&mut cursor) {
            return Self::array_to_hyperspectral(array);
        }

        // Reset cursor and try u16 (common for 16-bit imagery)
        cursor.set_position(0);
        if let Ok(array) = ArrayD::<u16>::read_npy(&mut cursor) {
            return Self::array_to_hyperspectral(array);
        }

        // Reset cursor and try i16 (signed 16-bit)
        cursor.set_position(0);
        if let Ok(array) = ArrayD::<i16>::read_npy(&mut cursor) {
            return Self::array_to_hyperspectral(array);
        }

        // Reset cursor and try i32
        cursor.set_position(0);
        if let Ok(array) = ArrayD::<i32>::read_npy(&mut cursor) {
            return Self::array_to_hyperspectral(array);
        }

        Err(LoaderError::new(
            "Failed to read NumPy array: unsupported dtype or invalid format",
        ))
    }

    fn priority(&self) -> i32 {
        // NumPy files have higher priority than generic images
        // since they're specifically for scientific data
        10
    }
}

/// Trait for converting numeric types to normalized f32.
trait NumericConvert {
    fn to_normalized_f32(self) -> f32;
}

impl NumericConvert for f32 {
    fn to_normalized_f32(self) -> f32 {
        // Assume already in 0-1 range or close to it
        // If values are > 1, they'll be clamped by the GPU shader
        self
    }
}

impl NumericConvert for f64 {
    fn to_normalized_f32(self) -> f32 {
        self as f32
    }
}

impl NumericConvert for u8 {
    fn to_normalized_f32(self) -> f32 {
        f32::from(self) / 255.0
    }
}

impl NumericConvert for u16 {
    fn to_normalized_f32(self) -> f32 {
        f32::from(self) / 65535.0
    }
}

impl NumericConvert for i16 {
    fn to_normalized_f32(self) -> f32 {
        // Map -32768..32767 to 0..1
        (f32::from(self) + 32768.0) / 65535.0
    }
}

impl NumericConvert for i32 {
    fn to_normalized_f32(self) -> f32 {
        // Map i32 range to 0..1
        ((self as f64 + 2_147_483_648.0) / 4_294_967_295.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_metadata() {
        let loader = NpyLoader;
        assert_eq!(loader.id(), "npy");
        assert!(loader.extensions().contains(&"npy"));
        assert_eq!(loader.priority(), 10);
    }

    #[test]
    fn test_magic_detection() {
        let loader = NpyLoader;

        // Valid NumPy magic
        let valid_magic = [0x93, b'N', b'U', b'M', b'P', b'Y', 0x01, 0x00];
        assert!(loader.can_load(&valid_magic));

        // Invalid data
        let invalid = [0x89, 0x50, 0x4E, 0x47]; // PNG magic
        assert!(!loader.can_load(&invalid));
    }

    #[test]
    fn test_numeric_convert_u8() {
        assert!((0u8.to_normalized_f32() - 0.0).abs() < f32::EPSILON);
        assert!((255u8.to_normalized_f32() - 1.0).abs() < f32::EPSILON);
        assert!((128u8.to_normalized_f32() - 0.502).abs() < 0.01);
    }

    #[test]
    fn test_numeric_convert_u16() {
        assert!((0u16.to_normalized_f32() - 0.0).abs() < f32::EPSILON);
        assert!((65535u16.to_normalized_f32() - 1.0).abs() < f32::EPSILON);
    }

    /// Integration test loading actual .npy files
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_load_npy_files() {
        use std::path::Path;

        let loader = NpyLoader;
        let test_files = [
            ("/tmp/test_hyperspectral_bhw.npy", 4, 100, 100), // (B, H, W)
            ("/tmp/test_hyperspectral_hwb.npy", 4, 100, 100), // (H, W, B)
            ("/tmp/test_grayscale.npy", 1, 100, 100),         // (H, W)
            ("/tmp/test_u8.npy", 4, 100, 100),                // u8 dtype
        ];

        for (path, expected_bands, expected_width, expected_height) in test_files {
            let path = Path::new(path);
            if !path.exists() {
                eprintln!("Skipping test - file not found: {}", path.display());
                continue;
            }

            let data = std::fs::read(path).expect("Failed to read test file");
            let result = loader.load(&data);

            match result {
                Ok(hyperspectral) => {
                    assert_eq!(
                        hyperspectral.bands.len(),
                        expected_bands,
                        "Wrong band count for {}",
                        path.display()
                    );
                    assert_eq!(
                        hyperspectral.width,
                        expected_width,
                        "Wrong width for {}",
                        path.display()
                    );
                    assert_eq!(
                        hyperspectral.height,
                        expected_height,
                        "Wrong height for {}",
                        path.display()
                    );

                    // Check that values are in valid range
                    for (i, band) in hyperspectral.bands.iter().enumerate() {
                        for &val in band {
                            assert!(
                                (0.0..=1.0).contains(&val),
                                "Band {} has out-of-range value {} in {}",
                                i,
                                val,
                                path.display()
                            );
                        }
                    }

                    println!(
                        "✓ Loaded {} - {}x{} with {} bands",
                        path.display(),
                        hyperspectral.width,
                        hyperspectral.height,
                        hyperspectral.bands.len()
                    );
                }
                Err(e) => {
                    panic!("Failed to load {}: {}", path.display(), e);
                }
            }
        }
    }
}
