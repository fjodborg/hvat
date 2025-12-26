//! Data structures and loaders for hyperspectral image data.
//!
//! This module provides:
//! - `HyperspectralData`: CPU-side representation of hyperspectral images
//! - `LoaderRegistry`: Extensible system for loading various file formats
//! - Built-in loaders for images (PNG, JPEG, etc.) and NumPy (.npy) files
//!
//! ## Adding New Formats
//!
//! To add support for a new format (e.g., ENVI, HDF5):
//!
//! 1. Create a new loader in `loaders/` implementing `HyperspectralLoader`
//! 2. Register it in `LoaderRegistry::new()`
//!
//! ```rust,ignore
//! use hvat::data::{HyperspectralLoader, LoaderError, HyperspectralData};
//!
//! pub struct MyFormatLoader;
//!
//! impl HyperspectralLoader for MyFormatLoader {
//!     fn id(&self) -> &'static str { "myformat" }
//!     fn display_name(&self) -> &'static str { "My Format" }
//!     fn extensions(&self) -> &'static [&'static str] { &["myf"] }
//!     fn can_load(&self, data: &[u8]) -> bool { /* check magic bytes */ }
//!     fn load(&self, data: &[u8]) -> Result<HyperspectralData, LoaderError> { /* ... */ }
//! }
//! ```

mod hyperspectral;
mod loader;
pub mod loaders;

pub use hyperspectral::HyperspectralData;
pub use loader::{HyperspectralLoader, LoaderError, LoaderRegistry};
