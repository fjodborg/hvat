//! UI building modules for HVAT application.
//!
//! Each module contains `impl HvatApp` blocks that extend the main
//! application struct with UI building methods.

mod image_viewer;
mod left_sidebar;
mod right_sidebar;
mod topbar;

// Re-export nothing - the impl blocks extend HvatApp directly
