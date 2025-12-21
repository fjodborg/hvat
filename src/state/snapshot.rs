//! Application state snapshot for undo/redo functionality.

use std::path::PathBuf;

use crate::model::Annotation;

/// Snapshot of application state for undo/redo.
///
/// Contains the current band selection, image adjustments, and optionally
/// annotation state that can be restored when undoing or redoing actions.
#[derive(Debug, Clone)]
pub struct AppSnapshot {
    /// Red band index
    pub red_band: usize,
    /// Green band index
    pub green_band: usize,
    /// Blue band index
    pub blue_band: usize,
    /// Brightness adjustment value
    pub brightness: f32,
    /// Contrast adjustment value
    pub contrast: f32,
    /// Gamma adjustment value
    pub gamma: f32,
    /// Hue shift adjustment value
    pub hue: f32,
    /// Optional annotation state (only present for annotation changes)
    pub annotations: Option<AnnotationState>,
}

/// Annotation state for a specific image.
#[derive(Debug, Clone)]
pub struct AnnotationState {
    /// The image path these annotations belong to
    pub image_path: PathBuf,
    /// The annotations at this point in time
    pub annotations: Vec<Annotation>,
    /// The next annotation ID at this point
    pub next_annotation_id: u32,
}
