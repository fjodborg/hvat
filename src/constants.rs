//! Global constants for the HVAT application

// =============================================================================
// Layout
// =============================================================================

/// Sidebar width for left and right sidebars
pub const SIDEBAR_WIDTH: f32 = 250.0;

/// Offset from sidebar width for content (accounts for padding/borders)
pub const SIDEBAR_CONTENT_OFFSET: f32 = 20.0;

/// Content width inside sidebars
pub const SIDEBAR_CONTENT_WIDTH: f32 = SIDEBAR_WIDTH - SIDEBAR_CONTENT_OFFSET;

// =============================================================================
// Test Image Defaults
// =============================================================================

/// Default test image width
pub const DEFAULT_TEST_WIDTH: u32 = 1024;

/// Default test image height
pub const DEFAULT_TEST_HEIGHT: u32 = 1024;

/// Default number of spectral bands for test images
pub const DEFAULT_TEST_BANDS: usize = 10;

// =============================================================================
// Undo System
// =============================================================================

/// Maximum entries in the application undo/redo stack
pub const UNDO_HISTORY_SIZE: usize = 50;

// =============================================================================
// Image Adjustment Slider Ranges
// =============================================================================

/// Brightness slider minimum value
pub const BRIGHTNESS_MIN: f32 = -1.0;

/// Brightness slider maximum value
pub const BRIGHTNESS_MAX: f32 = 1.0;

/// Brightness slider step size
pub const BRIGHTNESS_STEP: f32 = 0.01;

/// Contrast slider minimum value
pub const CONTRAST_MIN: f32 = 0.1;

/// Contrast slider maximum value
pub const CONTRAST_MAX: f32 = 3.0;

/// Contrast slider step size
pub const CONTRAST_STEP: f32 = 0.01;

/// Gamma slider minimum value (same as contrast)
pub const GAMMA_MIN: f32 = 0.1;

/// Gamma slider maximum value (same as contrast)
pub const GAMMA_MAX: f32 = 3.0;

/// Gamma slider step size
pub const GAMMA_STEP: f32 = 0.01;

/// Hue slider minimum value (degrees)
pub const HUE_MIN: f32 = 0.0;

/// Hue slider maximum value (degrees)
pub const HUE_MAX: f32 = 360.0;

/// Hue slider step size (degrees)
pub const HUE_STEP: f32 = 1.0;

// =============================================================================
// Default Adjustment Values
// =============================================================================

/// Default brightness value (no change)
pub const DEFAULT_BRIGHTNESS: f32 = 0.0;

/// Default contrast value (no change)
pub const DEFAULT_CONTRAST: f32 = 1.0;

/// Default gamma value (no change)
pub const DEFAULT_GAMMA: f32 = 1.0;

/// Default hue value (no change, in degrees)
pub const DEFAULT_HUE: f32 = 0.0;

// =============================================================================
// Default Band Selection
// =============================================================================

/// Default red band index
pub const DEFAULT_RED_BAND: usize = 0;

// =============================================================================
// GPU Preloading
// =============================================================================

/// Default number of images to preload in each direction (before and after current)
pub const DEFAULT_GPU_PRELOAD_COUNT: usize = 5;

/// Maximum preload count (prevents excessive GPU memory usage)
pub const MAX_GPU_PRELOAD_COUNT: usize = 10;

/// Number of texture rows to upload per tick during chunked GPU upload.
/// Higher values = faster uploads but more lag. Lower values = smoother but slower.
/// For a 4096x4096 RGBA texture:
///   - 64 rows = ~1MB per tick (~1ms blocking)
///   - 128 rows = ~2MB per tick (~2ms blocking)
///   - 256 rows = ~4MB per tick (~4ms blocking)
/// Set to 0 to upload entire layers at once (old behavior).
/// Used by both WASM and native for consistent preloading behavior.
pub const GPU_UPLOAD_ROWS_PER_TICK: u32 = 128;

/// Number of spectral bands packed into each RGBA texture layer.
/// Each layer stores 4 bands in R, G, B, A channels.
/// Note: WASM worker binary has its own copy of this constant.
#[cfg(not(target_arch = "wasm32"))]
pub const BANDS_PER_LAYER: usize = 4;

/// Minimum texture array layers for GPU textures.
/// WebGL2 has a bug where single-layer texture arrays don't work correctly.
/// This ensures at least 2 layers are always created as a workaround.
/// Also applied on native for consistency.
/// Note: WASM worker binary has its own copy of this constant.
#[cfg(not(target_arch = "wasm32"))]
pub const MIN_TEXTURE_LAYERS: u32 = 2;

/// Maximum number of in-flight decode requests for background preloading.
/// Limits memory usage by capping concurrent decodes.
#[cfg(not(target_arch = "wasm32"))]
pub const MAX_IN_FLIGHT_DECODES: usize = 3;

// =============================================================================
// Right Sidebar Sections
// =============================================================================

/// Maximum height for file list collapsible content (forces scrollbar)
pub const FILE_LIST_MAX_HEIGHT: f32 = 250.0;

/// Maximum height for thumbnails collapsible content (forces scrollbar)
pub const THUMBNAILS_MAX_HEIGHT: f32 = 200.0;

/// Thumbnail size (width and height) in pixels
pub const THUMBNAIL_SIZE: f32 = 64.0;

/// Spacing between thumbnails in the grid
pub const THUMBNAIL_SPACING: f32 = 4.0;

/// Maximum height for annotations panel collapsible content
pub const ANNOTATIONS_MAX_HEIGHT: f32 = 300.0;
