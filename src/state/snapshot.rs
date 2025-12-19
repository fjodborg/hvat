//! Application state snapshot for undo/redo functionality.

/// Snapshot of application state for undo/redo.
///
/// Contains the current band selection and image adjustments
/// that can be restored when undoing or redoing actions.
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
}
