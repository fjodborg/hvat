//! SAM2 (Segment Anything Model 2) integration module.
//!
//! This module provides AI-assisted segmentation using SAM2.1 tiny ONNX model.
//! The integration is designed for minimal overhead:
//!
//! - The main app only contains lightweight state and message types
//! - The heavy encoder runs in a background thread (native) or main thread (WASM)
//! - The lightweight decoder runs on the main thread for real-time preview
//!
//! # Platform-specific backends
//!
//! - **Native**: Uses `ort` (ONNX Runtime) with auto-downloaded binaries
//! - **WASM**: Uses `tract-onnx` (pure Rust, no C++ dependencies)
//!
//! # Workflows
//!
//! 1. **BBox to Segment**: Draw bbox -> auto-segment -> refine with points
//! 2. **SAM2 Tool**: Select tool -> draw bbox/add points -> accept mask
//! 3. **Points Only**: Add positive/negative points until satisfied

mod contour;
mod state;

// Platform-specific encoder/decoder implementations
#[cfg(not(target_arch = "wasm32"))]
mod decoder;
#[cfg(not(target_arch = "wasm32"))]
mod encoder;

#[cfg(target_arch = "wasm32")]
mod decoder_wasm;
#[cfg(target_arch = "wasm32")]
mod encoder_wasm;

pub use contour::extract_contour;
pub use state::{ImageEmbeddings, SAM2Mask, SAM2Message, SAM2Prompts, SAM2Session, SAM2State};

// Re-export platform-specific types with the same names
#[cfg(not(target_arch = "wasm32"))]
pub use decoder::{SAM2Decoder, SAM2DecoderError};
#[cfg(not(target_arch = "wasm32"))]
pub use encoder::{ENCODER_INPUT_SIZE, SAM2Encoder, SAM2EncoderError};

#[cfg(target_arch = "wasm32")]
pub use decoder_wasm::{SAM2Decoder, SAM2DecoderError};
#[cfg(target_arch = "wasm32")]
pub use encoder_wasm::{ENCODER_INPUT_SIZE, SAM2Encoder, SAM2EncoderError};
