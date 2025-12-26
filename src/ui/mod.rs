//! UI building modules for HVAT application.
//!
//! Each module contains `impl HvatApp` blocks that extend the main
//! application struct with UI building methods.

mod context_menu;
mod export_dialog;
mod image_viewer;
mod left_sidebar;
mod right_sidebar;
pub(crate) mod settings;
mod topbar;

// Re-export APP_VERSION and GIT_HASH for use in lib.rs and topbar
pub(crate) use settings::{APP_VERSION, GIT_HASH};
