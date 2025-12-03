//! View modules for HVAT application.
//!
//! Each view corresponds to a tab in the application:
//! - home: Welcome screen
//! - counter: Counter demo
//! - image_viewer: Main image viewing and annotation
//! - settings: Application settings
//! - helpers: Common UI building helpers

mod helpers;
mod home;
mod counter;
mod image_viewer;
mod settings;

// Re-export only the actively used helper for now
// Other helpers are available for future use
pub use helpers::tool_button;
pub use home::view_home;
pub use counter::view_counter;
pub use image_viewer::{view_image_viewer, view_annotation_toolbar, build_overlay};
pub use settings::view_settings;
