//! SAM2 state types and message definitions.
//!
//! This module contains the core types for SAM2 integration:
//! - [`SAM2State`]: The main state machine for SAM2 lifecycle
//! - [`SAM2Session`]: Active session for an encoded image
//! - [`SAM2Prompts`]: User prompts (points and bounding box)
//! - [`SAM2Mask`]: Predicted segmentation mask
//! - [`SAM2Message`]: Messages for SAM2 state updates

use std::path::PathBuf;
use std::sync::Arc;

/// Current state of SAM2 integration.
///
/// This is a state machine that tracks the SAM2 lifecycle from disabled
/// through downloading, loading, encoding, and active segmentation.
#[derive(Debug, Clone, Default)]
pub enum SAM2State {
    /// SAM2 not loaded (feature disabled or user preference off).
    #[default]
    Disabled,

    /// SAM2 WASM worker module is being downloaded.
    DownloadingWorker {
        /// Download progress (0.0 - 1.0).
        progress: f32,
    },

    /// SAM2 ONNX models are being downloaded.
    DownloadingModels {
        /// Download progress (0.0 - 1.0).
        progress: f32,
    },

    /// SAM2 models are loading into ONNX runtime.
    Loading,

    /// SAM2 ready but no image encoded yet.
    Ready,

    /// Encoding current image (running encoder in worker).
    Encoding {
        /// Path of the image being encoded.
        image_path: PathBuf,
    },

    /// Image encoded, ready for interactive segmentation.
    Active {
        /// The active SAM2 session.
        session: SAM2Session,
    },

    /// Error state.
    Error {
        /// Error message.
        message: String,
    },
}

impl SAM2State {
    /// Returns true if SAM2 is ready for use (Ready or Active state).
    pub fn is_ready(&self) -> bool {
        matches!(self, SAM2State::Ready | SAM2State::Active { .. })
    }

    /// Returns true if SAM2 is currently encoding an image.
    pub fn is_encoding(&self) -> bool {
        matches!(self, SAM2State::Encoding { .. })
    }

    /// Returns true if there's an active SAM2 session.
    pub fn is_active(&self) -> bool {
        matches!(self, SAM2State::Active { .. })
    }

    /// Returns the active session if in Active state.
    pub fn session(&self) -> Option<&SAM2Session> {
        match self {
            SAM2State::Active { session } => Some(session),
            _ => None,
        }
    }

    /// Returns a mutable reference to the active session if in Active state.
    pub fn session_mut(&mut self) -> Option<&mut SAM2Session> {
        match self {
            SAM2State::Active { session } => Some(session),
            _ => None,
        }
    }
}

/// Active SAM2 session for an image.
///
/// Contains the cached embeddings from the encoder and the current
/// prompts/mask for interactive refinement.
#[derive(Debug, Clone)]
pub struct SAM2Session {
    /// Path of the encoded image.
    pub image_path: PathBuf,

    /// Cached image embeddings (from encoder).
    /// These are large (~50MB) so we share via Arc.
    pub embeddings: Arc<ImageEmbeddings>,

    /// Original image dimensions (width, height).
    pub image_size: (u32, u32),

    /// Current prompts (points + optional box).
    pub prompts: SAM2Prompts,

    /// Current predicted mask (updated after each prompt change).
    pub mask: Option<SAM2Mask>,

    /// Whether mask is currently being computed by the decoder.
    pub computing: bool,

    /// Undo stack for prompts (previous states).
    prompts_undo: Vec<SAM2Prompts>,

    /// Redo stack for prompts (future states).
    prompts_redo: Vec<SAM2Prompts>,
}

/// Maximum number of prompt undo states to keep.
const MAX_PROMPT_UNDO_HISTORY: usize = 50;

impl SAM2Session {
    /// Creates a new SAM2 session with the given embeddings.
    pub fn new(
        image_path: PathBuf,
        embeddings: Arc<ImageEmbeddings>,
        image_size: (u32, u32),
    ) -> Self {
        Self {
            image_path,
            embeddings,
            image_size,
            prompts: SAM2Prompts::default(),
            mask: None,
            computing: false,
            prompts_undo: Vec::new(),
            prompts_redo: Vec::new(),
        }
    }

    /// Returns true if there are any prompts.
    pub fn has_prompts(&self) -> bool {
        !self.prompts.positive_points.is_empty()
            || !self.prompts.negative_points.is_empty()
            || self.prompts.bounding_box.is_some()
    }

    /// Pushes the current prompts state onto the undo stack before a change.
    /// Call this BEFORE modifying prompts.
    pub fn push_prompts_undo(&mut self) {
        self.prompts_undo.push(self.prompts.clone());
        // Clear redo stack when new action is taken
        self.prompts_redo.clear();
        // Limit undo history size
        if self.prompts_undo.len() > MAX_PROMPT_UNDO_HISTORY {
            self.prompts_undo.remove(0);
        }
    }

    /// Undoes the last prompt change.
    /// Returns true if undo was performed.
    pub fn undo_prompts(&mut self) -> bool {
        if let Some(previous) = self.prompts_undo.pop() {
            // Save current state to redo stack
            self.prompts_redo.push(self.prompts.clone());
            self.prompts = previous;
            // Clear mask since prompts changed
            self.mask = None;
            true
        } else {
            false
        }
    }

    /// Redoes the last undone prompt change.
    /// Returns true if redo was performed.
    pub fn redo_prompts(&mut self) -> bool {
        if let Some(next) = self.prompts_redo.pop() {
            // Save current state to undo stack
            self.prompts_undo.push(self.prompts.clone());
            self.prompts = next;
            // Clear mask since prompts changed
            self.mask = None;
            true
        } else {
            false
        }
    }

    /// Returns true if there are prompt changes to undo.
    pub fn can_undo_prompts(&self) -> bool {
        !self.prompts_undo.is_empty()
    }

    /// Returns true if there are prompt changes to redo.
    pub fn can_redo_prompts(&self) -> bool {
        !self.prompts_redo.is_empty()
    }

    /// Clears the prompt undo/redo history.
    /// Call this after accepting a mask to start fresh for the next segmentation.
    pub fn clear_prompts_history(&mut self) {
        self.prompts_undo.clear();
        self.prompts_redo.clear();
    }
}

/// Prompts for SAM2 decoder.
///
/// These are the user-provided hints that guide segmentation:
/// - Positive points: Foreground (include in mask)
/// - Negative points: Background (exclude from mask)
/// - Bounding box: Region of interest
#[derive(Debug, Clone, Default)]
pub struct SAM2Prompts {
    /// Positive points (foreground) in image coordinates.
    pub positive_points: Vec<(f32, f32)>,

    /// Negative points (background) in image coordinates.
    pub negative_points: Vec<(f32, f32)>,

    /// Optional bounding box (x, y, width, height) in image coordinates.
    pub bounding_box: Option<(f32, f32, f32, f32)>,
}

impl SAM2Prompts {
    /// Clears all prompts.
    pub fn clear(&mut self) {
        self.positive_points.clear();
        self.negative_points.clear();
        self.bounding_box = None;
    }

    /// Adds a positive point.
    pub fn add_positive_point(&mut self, x: f32, y: f32) {
        self.positive_points.push((x, y));
    }

    /// Adds a negative point.
    pub fn add_negative_point(&mut self, x: f32, y: f32) {
        self.negative_points.push((x, y));
    }

    /// Sets the bounding box.
    pub fn set_bounding_box(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.bounding_box = Some((x, y, width, height));
    }

    /// Clears the bounding box.
    pub fn clear_bounding_box(&mut self) {
        self.bounding_box = None;
    }

    /// Removes a point by index (positive points first, then negative).
    ///
    /// Returns true if a point was removed.
    pub fn remove_point(&mut self, index: usize) -> bool {
        if index < self.positive_points.len() {
            self.positive_points.remove(index);
            true
        } else {
            let neg_index = index - self.positive_points.len();
            if neg_index < self.negative_points.len() {
                self.negative_points.remove(neg_index);
                true
            } else {
                false
            }
        }
    }

    /// Finds and removes a point near the given coordinates.
    ///
    /// Returns the index of the removed point, or None if no point was found.
    pub fn find_and_remove_point(&mut self, x: f32, y: f32, tolerance: f32) -> Option<usize> {
        let tolerance_sq = tolerance * tolerance;

        // Check positive points
        for (i, (px, py)) in self.positive_points.iter().enumerate() {
            let dx = x - px;
            let dy = y - py;
            if dx * dx + dy * dy <= tolerance_sq {
                self.positive_points.remove(i);
                return Some(i);
            }
        }

        // Check negative points
        for (i, (px, py)) in self.negative_points.iter().enumerate() {
            let dx = x - px;
            let dy = y - py;
            if dx * dx + dy * dy <= tolerance_sq {
                self.negative_points.remove(i);
                return Some(self.positive_points.len() + i);
            }
        }

        None
    }

    /// Returns total number of points.
    pub fn total_points(&self) -> usize {
        self.positive_points.len() + self.negative_points.len()
    }
}

/// Predicted segmentation mask from SAM2 decoder.
#[derive(Debug, Clone)]
pub struct SAM2Mask {
    /// Binary mask data (1 = foreground, 0 = background).
    /// Stored as u8 for efficiency (0 or 255).
    pub data: Vec<u8>,

    /// Mask width in pixels.
    pub width: u32,

    /// Mask height in pixels.
    pub height: u32,

    /// Confidence score (0.0 - 1.0).
    pub score: f32,

    /// Extracted polygon contour (for annotation).
    /// This is computed from the binary mask using marching squares.
    pub contour: Vec<(f32, f32)>,
}

impl SAM2Mask {
    /// Creates a new mask with the given data.
    pub fn new(data: Vec<u8>, width: u32, height: u32, score: f32) -> Self {
        Self {
            data,
            width,
            height,
            score,
            contour: Vec::new(),
        }
    }

    /// Creates a mask with a pre-computed contour.
    pub fn with_contour(
        data: Vec<u8>,
        width: u32,
        height: u32,
        score: f32,
        contour: Vec<(f32, f32)>,
    ) -> Self {
        Self {
            data,
            width,
            height,
            score,
            contour,
        }
    }

    /// Returns the mask value at the given coordinates.
    ///
    /// Returns 0 if out of bounds.
    pub fn get(&self, x: u32, y: u32) -> u8 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        self.data[(y * self.width + x) as usize]
    }

    /// Returns true if the given point is inside the mask (foreground).
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let xi = x as u32;
        let yi = y as u32;
        self.get(xi, yi) > 127
    }
}

/// Image embeddings from SAM2 encoder.
///
/// These are the intermediate representations computed by the encoder
/// that allow fast mask generation with the decoder.
#[derive(Debug)]
pub struct ImageEmbeddings {
    /// Main image embedding tensor data (flattened) [1, 256, 64, 64].
    pub data: Vec<f32>,

    /// Embedding tensor shape [batch, channels, height, width].
    pub shape: [usize; 4],

    /// High-resolution features level 0 (flattened) [1, 32, 256, 256].
    pub high_res_feats_0: Vec<f32>,

    /// High-resolution features level 0 shape.
    pub high_res_feats_0_shape: [usize; 4],

    /// High-resolution features level 1 (flattened) [1, 64, 128, 128].
    pub high_res_feats_1: Vec<f32>,

    /// High-resolution features level 1 shape.
    pub high_res_feats_1_shape: [usize; 4],

    /// Original image dimensions (width, height) for coordinate scaling.
    pub original_size: (u32, u32),
}

impl ImageEmbeddings {
    /// Creates new embeddings with the given data and shape.
    pub fn new(
        data: Vec<f32>,
        shape: [usize; 4],
        high_res_feats_0: Vec<f32>,
        high_res_feats_0_shape: [usize; 4],
        high_res_feats_1: Vec<f32>,
        high_res_feats_1_shape: [usize; 4],
        original_size: (u32, u32),
    ) -> Self {
        Self {
            data,
            shape,
            high_res_feats_0,
            high_res_feats_0_shape,
            high_res_feats_1,
            high_res_feats_1_shape,
            original_size,
        }
    }

    /// Returns the total number of elements in the embeddings.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the embeddings are empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// SAM2-specific messages for state updates.
///
/// These messages are used to drive the SAM2 state machine and
/// communicate between the main thread, worker, and UI.
#[derive(Clone)]
pub enum SAM2Message {
    // === Lifecycle ===
    /// Enable or disable SAM2 mode.
    ToggleEnabled(bool),

    /// Worker download progress update (0.0 - 1.0).
    WorkerDownloadProgress(f32),

    /// Model download progress update (0.0 - 1.0).
    ModelDownloadProgress(f32),

    /// Models loaded successfully, SAM2 ready.
    ModelsLoaded,

    /// Download or load failed with error message.
    LoadError(String),

    // === Encoding ===
    /// Start encoding the current image.
    StartEncoding,

    /// Encoding completed with embeddings.
    EncodingComplete(Arc<ImageEmbeddings>),

    /// Encoding failed with error message.
    EncodingFailed(String),

    // === Interactive Prompting ===
    /// Add a positive point (foreground) at image coordinates.
    AddPositivePoint(f32, f32),

    /// Add a negative point (background) at image coordinates.
    AddNegativePoint(f32, f32),

    /// Remove point at the given index.
    RemovePoint(usize),

    /// Remove point near the given coordinates.
    RemovePointAt(f32, f32),

    /// Set bounding box prompt (x, y, width, height).
    SetBoundingBox(f32, f32, f32, f32),

    /// Clear the bounding box.
    ClearBoundingBox,

    /// Clear all prompts.
    ClearPrompts,

    // === Mask Updates ===
    /// Decoder computed new mask.
    MaskComputed(SAM2Mask),

    /// Decoder failed with error message.
    MaskFailed(String),

    // === Undo/Redo ===
    /// Undo the last prompt change.
    UndoPrompts,

    /// Redo the last undone prompt change.
    RedoPrompts,

    // === Finalization ===
    /// Accept current mask as polygon annotation.
    AcceptMask,

    /// Cancel SAM2 session without creating annotation.
    CancelSession,
}

impl std::fmt::Debug for SAM2Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SAM2Message::ToggleEnabled(enabled) => {
                write!(f, "SAM2Message::ToggleEnabled({enabled})")
            }
            SAM2Message::WorkerDownloadProgress(p) => {
                write!(f, "SAM2Message::WorkerDownloadProgress({p:.2})")
            }
            SAM2Message::ModelDownloadProgress(p) => {
                write!(f, "SAM2Message::ModelDownloadProgress({p:.2})")
            }
            SAM2Message::ModelsLoaded => write!(f, "SAM2Message::ModelsLoaded"),
            SAM2Message::LoadError(msg) => write!(f, "SAM2Message::LoadError({msg:?})"),
            SAM2Message::StartEncoding => write!(f, "SAM2Message::StartEncoding"),
            SAM2Message::EncodingComplete(_) => write!(f, "SAM2Message::EncodingComplete(...)"),
            SAM2Message::EncodingFailed(msg) => write!(f, "SAM2Message::EncodingFailed({msg:?})"),
            SAM2Message::AddPositivePoint(x, y) => {
                write!(f, "SAM2Message::AddPositivePoint({x:.1}, {y:.1})")
            }
            SAM2Message::AddNegativePoint(x, y) => {
                write!(f, "SAM2Message::AddNegativePoint({x:.1}, {y:.1})")
            }
            SAM2Message::RemovePoint(idx) => write!(f, "SAM2Message::RemovePoint({idx})"),
            SAM2Message::RemovePointAt(x, y) => {
                write!(f, "SAM2Message::RemovePointAt({x:.1}, {y:.1})")
            }
            SAM2Message::SetBoundingBox(x, y, w, h) => {
                write!(
                    f,
                    "SAM2Message::SetBoundingBox({x:.1}, {y:.1}, {w:.1}, {h:.1})"
                )
            }
            SAM2Message::ClearBoundingBox => write!(f, "SAM2Message::ClearBoundingBox"),
            SAM2Message::ClearPrompts => write!(f, "SAM2Message::ClearPrompts"),
            SAM2Message::MaskComputed(_) => write!(f, "SAM2Message::MaskComputed(...)"),
            SAM2Message::MaskFailed(msg) => write!(f, "SAM2Message::MaskFailed({msg:?})"),
            SAM2Message::UndoPrompts => write!(f, "SAM2Message::UndoPrompts"),
            SAM2Message::RedoPrompts => write!(f, "SAM2Message::RedoPrompts"),
            SAM2Message::AcceptMask => write!(f, "SAM2Message::AcceptMask"),
            SAM2Message::CancelSession => write!(f, "SAM2Message::CancelSession"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompts_add_remove() {
        let mut prompts = SAM2Prompts::default();

        prompts.add_positive_point(10.0, 20.0);
        prompts.add_positive_point(30.0, 40.0);
        prompts.add_negative_point(50.0, 60.0);

        assert_eq!(prompts.positive_points.len(), 2);
        assert_eq!(prompts.negative_points.len(), 1);
        assert_eq!(prompts.total_points(), 3);

        // Remove second positive point
        assert!(prompts.remove_point(1));
        assert_eq!(prompts.positive_points.len(), 1);
        assert_eq!(prompts.positive_points[0], (10.0, 20.0));

        // Remove negative point (now at index 1)
        assert!(prompts.remove_point(1));
        assert_eq!(prompts.negative_points.len(), 0);

        // Try to remove non-existent
        assert!(!prompts.remove_point(5));
    }

    #[test]
    fn test_prompts_find_and_remove() {
        let mut prompts = SAM2Prompts::default();

        prompts.add_positive_point(100.0, 100.0);
        prompts.add_negative_point(200.0, 200.0);

        // Find and remove positive point
        let removed = prompts.find_and_remove_point(101.0, 99.0, 5.0);
        assert_eq!(removed, Some(0));
        assert_eq!(prompts.positive_points.len(), 0);

        // Find and remove negative point
        let removed = prompts.find_and_remove_point(199.0, 201.0, 5.0);
        assert_eq!(removed, Some(0)); // Now at index 0 after positive points
        assert_eq!(prompts.negative_points.len(), 0);

        // Try to find non-existent
        let removed = prompts.find_and_remove_point(500.0, 500.0, 5.0);
        assert!(removed.is_none());
    }

    #[test]
    fn test_mask_contains() {
        let mut data = vec![0u8; 100 * 100];
        // Set a 10x10 square in the middle to foreground
        for y in 45..55 {
            for x in 45..55 {
                data[y * 100 + x] = 255;
            }
        }

        let mask = SAM2Mask::new(data, 100, 100, 0.9);

        assert!(mask.contains(50.0, 50.0));
        assert!(!mask.contains(0.0, 0.0));
        assert!(!mask.contains(99.0, 99.0));
    }
}
