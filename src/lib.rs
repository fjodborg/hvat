//! HVAT - Hyperspectral Annotation Tool
//!
//! A GPU-accelerated desktop and web application for hyperspectral image annotation.

// ============================================================================
// Core Modules
// ============================================================================

// Image caching abstraction (unified native/WASM)
mod image_cache;
pub use image_cache::{is_image_file, ImageCache, IMAGE_EXTENSIONS};

// Widget state management layer
mod widget_state;
pub use widget_state::{ImageViewState, ScrollState, SliderState, WidgetState};

// Zoom-to-cursor mathematics (extracted for testability)
mod zoom_math;
pub use zoom_math::Transform;

// Annotation system for image labeling
mod annotation;
pub use annotation::{
    Annotation, AnnotationStore, AnnotationTool, AnnotationToolBehavior, BoundingBox, Category,
    DrawingState, Point, Polygon, Shape,
};

// Undo/Redo system for annotations
mod undo;
pub use undo::{Command, UndoConfig, UndoStack, record_command, redo_command, undo_command};

// Hyperspectral image support
mod hyperspectral;
pub use hyperspectral::{
    generate_test_hyperspectral, BandSelection, HyperspectralImage,
};

// UI constants for consistent styling
pub mod ui_constants;

// Color utility functions (shared across modules)
pub mod color_utils;

// Annotation import/export formats
pub mod formats;

// Project file management
mod project;
pub use project::{Project, ProjectError, ProjectSettings, PROJECT_VERSION};

// ============================================================================
// Application Modules (modularized from hvat_app.rs)
// ============================================================================

// Message types and constructors
mod message;
pub use message::{
    AnnotationMessage, BandMessage, CounterMessage, ExportFormat, ImageLoadMessage,
    ImageSettingsMessage, ImageViewMessage, Message, NavigationMessage, PersistenceMode,
    ProjectMessage, Tab, UIMessage,
};

// Theme system
mod theme;
pub use theme::{Theme, ThemeChoice};

// View building functions
mod views;
pub use views::{build_overlay, view_counter, view_export_modal_content, view_home, view_image_viewer, view_settings};

// Message handlers
mod handlers;
pub use handlers::{
    handle_annotation, handle_band, handle_counter, handle_image_load, handle_image_settings,
    handle_image_view, handle_navigation, handle_project, handle_ui, AnnotationState,
    ImageLoadState, ProjectState,
};

// WASM file loading utilities
mod wasm_file;
pub use wasm_file::{open_wasm_file_picker, take_wasm_pending_files};

// ============================================================================
// Main Application
// ============================================================================

// HVAT application (shared between native and WASM)
mod hvat_app;
pub use hvat_app::{
    ExportState, FpsTracker, HvatApp, ImageSettings, ImageViewTransform, PersistenceState, Tag,
    TaggingState,
};

// ============================================================================
// Platform Entry Points
// ============================================================================

// WASM entry point
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;


// ============================================================================
// Public API
// ============================================================================

/// Initialize the HVAT library.
pub fn init() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        println!("HVAT - Hyperspectral Annotation Tool");
        println!("Use `cargo run --bin hvat-native` to run the native version");
    }
}

/// Generate a test image with a colorful gradient pattern.
pub fn generate_test_image(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            // Create a colorful test pattern
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            let b = (((x + y) as f32 / (width + height) as f32) * 255.0) as u8;

            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255); // Alpha
        }
    }

    data
}
