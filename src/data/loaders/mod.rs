//! Built-in hyperspectral data loaders.
//!
//! This module contains implementations of the `HyperspectralLoader` trait
//! for various file formats.

mod image_loader;
mod npy_loader;

pub use image_loader::ImageLoader;
pub use npy_loader::NpyLoader;
