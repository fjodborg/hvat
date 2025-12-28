//! HVAT Application - Hyperspectral Vision Annotation Tool
//!
//! Main application implementing the three-column layout with:
//! - Left sidebar: Tools, Categories, Image Tags
//! - Center: Image viewer with hyperspectral composite
//! - Right sidebar: Band Selection, Image Adjustments
//!
//! GPU-accelerated rendering:
//! - Band data uploaded to GPU texture array once
//! - Band compositing done in fragment shader (instant band changes)
//! - Image adjustments (brightness, contrast, gamma, hue) also GPU-side

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;

use hvat_gpu::{BandSelectionUniform, ImageAdjustments};
use hvat_ui::prelude::*;
use hvat_ui::{
    Application, Column, Element, Event, FileTreeState, KeyCode, Resources, Row, TickResult,
    TooltipManager,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::constants::MAX_IN_FLIGHT_DECODES;
use crate::constants::{
    DEFAULT_BRIGHTNESS, DEFAULT_CONTRAST, DEFAULT_GAMMA, DEFAULT_HUE, DEFAULT_RED_BAND,
    DEFAULT_TEST_BANDS, DEFAULT_TEST_HEIGHT, DEFAULT_TEST_WIDTH, MAX_GPU_PRELOAD_COUNT,
    UNDO_HISTORY_SIZE,
};
use crate::data::HyperspectralData;
use crate::format::{AutoSaveManager, ExportOptions, FormatRegistry, ProjectData};
use crate::keybindings::{KeyBindings, KeybindTarget};
use crate::message::Message;
use crate::model::{
    Annotation, AnnotationShape, AnnotationTool, Category, DrawingState, EditState,
    HANDLE_HIT_RADIUS, MIN_DRAG_DISTANCE, MIN_POLYGON_VERTICES, POLYGON_CLOSE_THRESHOLD, Tag,
};
use crate::state::{
    AppSnapshot, GpuRenderState, GpuTextureCache, ImageDataStore, LoadedImage, ProjectState,
    SharedGpuPipeline,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::state::{DecodeResult, NativePreloadState, extract_images_from_zip_file, is_zip_path};
#[cfg(target_arch = "wasm32")]
use crate::state::{WasmPreloadState, extract_images_from_zip_bytes, is_zip_file};
use crate::test_image::generate_test_hyperspectral;

// ============================================================================
// Confirmation Dialog Target
// ============================================================================

/// What item is being confirmed for deletion.
#[derive(Debug, Clone)]
pub enum ConfirmTarget {
    /// Delete a category (id, name, annotation_count)
    Category(u32, String, usize),
    /// Delete a tag (id, name)
    Tag(u32, String),
}

// ============================================================================
// Async Picker State (for WASM file picker)
// ============================================================================

/// Result of async file picker (WASM only).
#[cfg(target_arch = "wasm32")]
pub enum AsyncPickerResult {
    /// Files selected (WASM - contains loaded image data)
    Files(Vec<LoadedImage>),
}

// Global shared state for receiving async picker results (WASM only)
#[cfg(target_arch = "wasm32")]
static PENDING_PICKER_RESULT: OnceLock<std::sync::Mutex<Option<AsyncPickerResult>>> =
    OnceLock::new();

#[cfg(target_arch = "wasm32")]
fn pending_picker_state() -> &'static std::sync::Mutex<Option<AsyncPickerResult>> {
    PENDING_PICKER_RESULT.get_or_init(|| std::sync::Mutex::new(None))
}

// Global shared state for receiving async config import results (WASM only)
static PENDING_CONFIG_RESULT: OnceLock<std::sync::Mutex<Option<String>>> = OnceLock::new();

fn pending_config_state() -> &'static std::sync::Mutex<Option<String>> {
    PENDING_CONFIG_RESULT.get_or_init(|| std::sync::Mutex::new(None))
}

// ============================================================================
// WASM Folder Picker using File System Access API
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod wasm_folder_picker {
    use crate::state::{
        LoadedImage, extract_images_from_zip_bytes, is_image_filename, is_zip_file,
    };
    use hvat_ui::read_file_async;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;

    /// Show folder picker using hidden input with webkitdirectory attribute.
    /// Works in Firefox, Chrome, and Edge.
    pub fn show_folder_picker_via_input(on_complete: impl Fn(Vec<LoadedImage>) + 'static) {
        let window = match web_sys::window() {
            Some(w) => w,
            None => {
                log::error!("No window object");
                return;
            }
        };
        let document = match window.document() {
            Some(d) => d,
            None => {
                log::error!("No document object");
                return;
            }
        };

        // Create hidden input element
        let input: web_sys::HtmlInputElement = match document.create_element("input") {
            Ok(el) => match el.dyn_into() {
                Ok(input) => input,
                Err(_) => {
                    log::error!("Failed to cast to HtmlInputElement");
                    return;
                }
            },
            Err(e) => {
                log::error!("Failed to create input element: {:?}", e);
                return;
            }
        };

        input.set_type("file");
        // webkitdirectory enables folder selection in Firefox, Chrome, and Edge
        input.set_attribute("webkitdirectory", "").ok();
        input.set_attribute("directory", "").ok();
        input.set_multiple(true);
        input.style().set_property("display", "none").ok();

        // Append to body temporarily
        if let Some(body) = document.body() {
            if let Err(e) = body.append_child(&input) {
                log::error!("Failed to append input to body: {:?}", e);
                return;
            }
        }

        // Create callback for when files are selected
        let input_clone = input.clone();
        let callback = Rc::new(RefCell::new(Some(on_complete)));
        let callback_clone = callback.clone();

        let change_closure = Closure::once(Box::new(move |_event: web_sys::Event| {
            log::info!("Folder input change event fired");

            // Get the files from the input
            let files = match input_clone.files() {
                Some(f) => f,
                None => {
                    log::warn!("No files in input");
                    // Clean up
                    if let Some(parent) = input_clone.parent_element() {
                        parent.remove_child(&input_clone).ok();
                    }
                    return;
                }
            };

            let file_count = files.length();
            log::info!("Selected {} files from folder", file_count);

            if file_count == 0 {
                // Clean up
                if let Some(parent) = input_clone.parent_element() {
                    parent.remove_child(&input_clone).ok();
                }
                return;
            }

            // Collect all files (images and ZIP files)
            let mut image_files: Vec<web_sys::File> = Vec::new();
            let mut zip_files: Vec<web_sys::File> = Vec::new();

            for i in 0..file_count {
                if let Some(file) = files.get(i) {
                    let name = file.name();
                    if is_zip_file(&name) {
                        log::info!("Found ZIP file: {}", name);
                        zip_files.push(file);
                    } else if is_image_filename(&name) {
                        image_files.push(file);
                    } else {
                        log::debug!("Skipping non-image file: {}", name);
                    }
                }
            }

            log::info!(
                "Found {} image files and {} ZIP files",
                image_files.len(),
                zip_files.len()
            );

            // Clean up input element
            if let Some(parent) = input_clone.parent_element() {
                parent.remove_child(&input_clone).ok();
            }

            if image_files.is_empty() && zip_files.is_empty() {
                log::warn!("No image or ZIP files found in selected folder");
                return;
            }

            // Read files asynchronously
            let cb = callback_clone.clone();
            wasm_bindgen_futures::spawn_local(async move {
                use js_sys::Reflect;
                use wasm_bindgen::JsValue;

                let mut loaded_images: Vec<LoadedImage> = Vec::new();

                // Process ZIP files first - extract images from them
                for file in zip_files {
                    let name = file.name();
                    match read_file_async(&file).await {
                        Ok(data) => {
                            log::info!("Read ZIP file: {} ({} bytes)", name, data.len());
                            match extract_images_from_zip_bytes(&data, &name) {
                                Ok(mut extracted) => {
                                    log::info!(
                                        "Extracted {} images from ZIP '{}'",
                                        extracted.len(),
                                        name
                                    );
                                    loaded_images.append(&mut extracted);
                                }
                                Err(e) => {
                                    log::error!("Failed to extract ZIP '{}': {}", name, e);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to read ZIP file {}: {}", name, e);
                        }
                    }
                }

                // Process regular image files
                for file in image_files {
                    let name = file.name();
                    // Try to get the relative path (webkitRelativePath) via JS reflection
                    // since web-sys may not expose it directly
                    let display_name =
                        Reflect::get(&file, &JsValue::from_str("webkitRelativePath"))
                            .ok()
                            .and_then(|v| v.as_string())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| name.clone());

                    match read_file_async(&file).await {
                        Ok(data) => {
                            log::debug!("Read file: {} ({} bytes)", display_name, data.len());
                            loaded_images.push(LoadedImage {
                                name: display_name,
                                data,
                            });
                        }
                        Err(e) => {
                            log::error!("Failed to read file {}: {}", name, e);
                        }
                    }
                }

                // Sort by name for consistent ordering
                loaded_images.sort_by(|a, b| a.name.cmp(&b.name));
                log::info!(
                    "Successfully loaded {} image files total",
                    loaded_images.len()
                );

                // Call the completion callback
                if let Some(callback) = cb.borrow_mut().take() {
                    callback(loaded_images);
                }
            });
        }));

        input.set_onchange(Some(change_closure.as_ref().unchecked_ref()));
        change_closure.forget(); // Don't drop the closure

        // Trigger the file picker
        log::info!("Opening folder picker dialog...");
        input.click();
    }
}

// ============================================================================
// HVAT Application State
// ============================================================================

/// Main HVAT application state.
pub struct HvatApp {
    // Image viewer
    pub(crate) viewer_state: ImageViewerState,
    pub(crate) texture_id: Option<TextureId>,
    pub(crate) image_size: (u32, u32),

    // Project state (loaded folder with images)
    pub(crate) project: Option<ProjectState>,

    // Hyperspectral data (CPU-side, used for initial upload to GPU)
    hyperspectral: Option<HyperspectralData>,
    pub(crate) num_bands: usize,
    band_selection: (usize, usize, usize), // R, G, B band indices

    // GPU rendering state
    /// Shared GPU pipeline (created once in setup, reused for all images)
    shared_pipeline: Option<SharedGpuPipeline>,
    /// Per-image GPU state (band textures + render target)
    gpu_state: Option<GpuRenderState>,
    /// Path of the currently displayed image (for returning to cache)
    current_gpu_image_path: Option<PathBuf>,
    /// GPU texture cache for preloaded images
    gpu_cache: GpuTextureCache,

    // Flags for pending operations
    /// Flag indicating we need to load a new image
    pending_image_load: bool,
    /// Flag indicating we should preload adjacent images
    pending_preload: bool,
    /// Flag indicating the last preload step did GPU work (upload_and_cache)
    /// Used to select between ContinueWork (did work, need redraw) and ScheduleTick (just polling)
    preload_did_gpu_work: bool,

    // GPU preload settings
    /// Number of images to preload in each direction (before and after current)
    pub(crate) gpu_preload_count: usize,
    /// Slider state for preload count in settings
    pub(crate) gpu_preload_slider: SliderState,

    // Left sidebar states
    pub(crate) tools_collapsed: CollapsibleState,
    pub(crate) categories_collapsed: CollapsibleState,
    pub(crate) tags_collapsed: CollapsibleState,
    pub(crate) left_scroll_state: ScrollState,

    // Right sidebar states
    pub(crate) band_selection_collapsed: CollapsibleState,
    pub(crate) adjustments_collapsed: CollapsibleState,
    pub(crate) file_explorer_collapsed: CollapsibleState,
    pub(crate) file_explorer_scroll_state: ScrollState,
    pub(crate) file_explorer_state: FileTreeState,
    pub(crate) thumbnails_collapsed: CollapsibleState,
    pub(crate) thumbnails_scroll_state: ScrollState,
    pub(crate) right_scroll_state: ScrollState,

    // Tool selection
    pub(crate) selected_tool: AnnotationTool,

    // Note: Annotations are now stored per-image in ImageDataStore

    // Categories
    pub(crate) categories: Vec<Category>,
    pub(crate) selected_category: u32,
    /// Category input text for adding new categories
    pub(crate) category_input_text: String,
    /// Category input state for adding new categories
    pub(crate) category_input_state: TextInputState,
    /// Category currently being edited (for renaming)
    pub(crate) editing_category: Option<u32>,
    /// Text input for category name editing
    pub(crate) category_name_input: String,
    /// State for category name text input
    pub(crate) category_name_input_state: TextInputState,
    /// Category ID with open color picker
    pub(crate) color_picker_category: Option<u32>,
    /// Color picker state (drag tracking)
    pub(crate) color_picker_state: ColorPickerState,

    // Per-image data (tags selection, annotations, etc.)
    pub(crate) image_data_store: ImageDataStore,
    // Global tag registry (tags exist across all images, like categories but for images)
    pub(crate) tags: Vec<Tag>,
    /// Currently selected tag ID (for new tag assignments)
    pub(crate) selected_tag: u32,
    // Tag input UI state (not per-image)
    pub(crate) tag_input_text: String,
    pub(crate) tag_input_state: TextInputState,
    /// Tag currently being edited (by ID)
    pub(crate) editing_tag: Option<u32>,
    /// Text input for tag name editing
    pub(crate) tag_name_input: String,
    /// State for tag name text input
    pub(crate) tag_name_input_state: TextInputState,
    /// Tag ID with open color picker
    pub(crate) color_picker_tag: Option<u32>,

    // Band sliders
    pub(crate) red_band_slider: SliderState,
    pub(crate) green_band_slider: SliderState,
    pub(crate) blue_band_slider: SliderState,

    // Adjustment sliders
    pub(crate) brightness_slider: SliderState,
    pub(crate) contrast_slider: SliderState,
    pub(crate) gamma_slider: SliderState,
    pub(crate) hue_slider: SliderState,

    // Undo system
    pub(crate) undo_stack: Rc<RefCell<UndoStack<AppSnapshot>>>,

    // Window size (for dropdown positioning)
    window_height: f32,

    // Flag to trigger GPU re-render (band/adjustment change)
    needs_gpu_render: bool,

    // Settings view
    pub(crate) settings_open: bool,
    pub(crate) settings_scroll_state: ScrollState,
    pub(crate) settings_section_collapsed: CollapsibleState,
    pub(crate) appearance_section_collapsed: CollapsibleState,
    pub(crate) keybindings_section_collapsed: CollapsibleState,
    pub(crate) folders_section_collapsed: CollapsibleState,
    pub(crate) performance_section_collapsed: CollapsibleState,
    pub(crate) dependencies_collapsed: CollapsibleState,
    /// Collapsed state for each license type in the dependencies view
    pub(crate) license_collapsed: std::collections::HashMap<String, CollapsibleState>,

    // User preferences
    /// Dark theme enabled (true = dark, false = light)
    pub(crate) dark_theme: bool,
    /// Default export folder path
    pub(crate) export_folder: String,
    pub(crate) export_folder_state: TextInputState,
    /// Default import folder path
    pub(crate) import_folder: String,
    pub(crate) import_folder_state: TextInputState,
    /// Customizable keybindings
    pub(crate) keybindings: KeyBindings,
    /// Whether we're currently capturing a key for rebinding
    pub(crate) capturing_keybind: Option<KeybindTarget>,
    /// Current log level setting
    pub(crate) log_level: crate::config::LogLevel,

    // Format system
    /// Format registry with all supported formats
    pub(crate) format_registry: FormatRegistry,
    /// Auto-save manager for automatic project persistence
    pub(crate) auto_save: AutoSaveManager,
    /// Path to the current project file (for auto-save, native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) project_file_path: Option<PathBuf>,
    /// Whether the export dialog is open
    pub(crate) export_dialog_open: bool,

    // Drag-Drop State
    /// Whether files are being dragged over the window
    pub(crate) drag_hover_active: bool,
    /// Pending dropped files from WASM (collects DroppedFileData events)
    pub(crate) pending_wasm_files: Vec<LoadedImage>,

    // Annotations Panel State (right sidebar)
    /// Annotations section collapsible state
    pub(crate) annotations_collapsed: CollapsibleState,
    /// Annotations list scroll state
    pub(crate) annotations_scroll_state: ScrollState,
    /// Set of category IDs that are hidden (filtered out from display)
    pub(crate) hidden_categories: std::collections::HashSet<u32>,

    // Tooltip system
    /// Tooltip manager for hover-triggered tooltips
    pub(crate) tooltip_manager: TooltipManager,
    /// Current window size (for tooltip boundary detection)
    pub(crate) window_size: (f32, f32),

    // Context menu state
    /// Whether the context menu is open
    pub(crate) context_menu_open: bool,
    /// Position where the context menu was opened (screen coordinates)
    pub(crate) context_menu_position: (f32, f32),
    /// Annotation ID that was right-clicked (if any)
    pub(crate) context_menu_annotation_id: Option<u32>,

    // Confirmation dialog state
    /// What is being confirmed (None = dialog closed)
    pub(crate) confirm_dialog_target: Option<ConfirmTarget>,

    // WASM preloading state (Web Worker + chunked uploads)
    /// Groups all WASM-specific preloading fields
    #[cfg(target_arch = "wasm32")]
    wasm_preload: WasmPreloadState,

    // Native preloading state (background thread + chunked uploads)
    /// Groups all native-specific preloading fields
    #[cfg(not(target_arch = "wasm32"))]
    native_preload: Option<NativePreloadState>,
}

impl Default for HvatApp {
    fn default() -> Self {
        Self::new()
    }
}

impl HvatApp {
    /// Create a new HVAT application instance.
    pub fn new() -> Self {
        let num_bands = DEFAULT_TEST_BANDS;

        // Load saved configuration or use defaults
        #[cfg(not(target_arch = "wasm32"))]
        let config = crate::config::AppConfig::load_from_default_path().unwrap_or_default();
        #[cfg(target_arch = "wasm32")]
        let config = crate::config::AppConfig::load_from_local_storage().unwrap_or_default();

        // Clamp GPU preload count to valid range
        let gpu_preload_count = config
            .preferences
            .gpu_preload_count
            .min(MAX_GPU_PRELOAD_COUNT);

        Self {
            viewer_state: ImageViewerState::new(),
            texture_id: None,
            image_size: (0, 0),

            // DEBUG: Load test folder on startup
            project: {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    use std::path::PathBuf;
                    match crate::state::ProjectState::from_folder(PathBuf::from(
                        "/home/fjod/Pictures",
                    )) {
                        Ok(p) => Some(p),
                        Err(_e) => {
                            // Can't log here - logger not initialized yet
                            None
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                None
            },

            hyperspectral: None,
            num_bands,
            band_selection: (0, 1, 2),

            shared_pipeline: None,
            gpu_state: None,
            current_gpu_image_path: None,
            gpu_cache: GpuTextureCache::new(gpu_preload_count),

            pending_image_load: false,
            pending_preload: false,
            preload_did_gpu_work: false,

            gpu_preload_count,
            gpu_preload_slider: SliderState::new(gpu_preload_count as f32),

            tools_collapsed: CollapsibleState::expanded(),
            categories_collapsed: CollapsibleState::expanded(),
            tags_collapsed: CollapsibleState::collapsed(),
            left_scroll_state: ScrollState::default(),

            band_selection_collapsed: CollapsibleState::expanded(),
            adjustments_collapsed: CollapsibleState::expanded(),
            file_explorer_collapsed: CollapsibleState::expanded(),
            file_explorer_scroll_state: ScrollState::default(),
            file_explorer_state: FileTreeState::new(),
            thumbnails_collapsed: CollapsibleState::collapsed(),
            thumbnails_scroll_state: ScrollState::default(),
            right_scroll_state: ScrollState::default(),

            selected_tool: AnnotationTool::default(),

            categories: config.categories.into_iter().map(|c| c.into()).collect(),
            selected_category: 1,
            category_input_text: String::new(),
            category_input_state: TextInputState::default(),
            editing_category: None,
            category_name_input: String::new(),
            category_name_input_state: TextInputState::default(),
            color_picker_category: None,
            color_picker_state: ColorPickerState::default(),

            image_data_store: ImageDataStore::new(),
            tags: config.tags.into_iter().map(|t| t.into()).collect(),
            selected_tag: 1, // Default to first tag
            tag_input_text: String::new(),
            tag_input_state: TextInputState::default(),
            editing_tag: None,
            tag_name_input: String::new(),
            tag_name_input_state: TextInputState::default(),
            color_picker_tag: None,

            red_band_slider: SliderState::new(0.0),
            green_band_slider: SliderState::new(1.0),
            blue_band_slider: SliderState::new(2.0),

            brightness_slider: SliderState::new(0.0),
            contrast_slider: SliderState::new(1.0),
            gamma_slider: SliderState::new(1.0),
            hue_slider: SliderState::new(0.0),

            undo_stack: Rc::new(RefCell::new(UndoStack::new(UNDO_HISTORY_SIZE))),

            window_height: 900.0,

            needs_gpu_render: true,

            settings_open: false,
            settings_scroll_state: ScrollState::default(),
            settings_section_collapsed: CollapsibleState::expanded(),
            appearance_section_collapsed: CollapsibleState::expanded(),
            keybindings_section_collapsed: CollapsibleState::collapsed(),
            folders_section_collapsed: CollapsibleState::collapsed(),
            performance_section_collapsed: CollapsibleState::expanded(),
            dependencies_collapsed: CollapsibleState::collapsed(),
            license_collapsed: std::collections::HashMap::new(),

            dark_theme: config.preferences.dark_theme,
            export_folder: config.preferences.export_folder,
            export_folder_state: TextInputState::default(),
            import_folder: config.preferences.import_folder,
            import_folder_state: TextInputState::default(),
            keybindings: config.keybindings.to_keybindings(),
            capturing_keybind: None,
            log_level: config.preferences.log_level,

            format_registry: FormatRegistry::new(),
            auto_save: AutoSaveManager::new(),
            #[cfg(not(target_arch = "wasm32"))]
            project_file_path: None,
            export_dialog_open: false,

            drag_hover_active: false,
            pending_wasm_files: Vec::new(),

            annotations_collapsed: CollapsibleState::expanded(),
            annotations_scroll_state: ScrollState::default(),
            hidden_categories: std::collections::HashSet::new(),

            // Tooltip system
            tooltip_manager: TooltipManager::new(),
            window_size: (1920.0, 1080.0), // Default, updated on resize

            // Context menu
            context_menu_open: false,
            context_menu_position: (0.0, 0.0),
            context_menu_annotation_id: None,

            // Confirmation dialog
            confirm_dialog_target: None,

            // WASM preloading state
            #[cfg(target_arch = "wasm32")]
            wasm_preload: WasmPreloadState::new(),

            // Native preloading state
            #[cfg(not(target_arch = "wasm32"))]
            native_preload: NativePreloadState::new().ok(),
        }
    }

    /// Create a snapshot of current state for undo (sliders only, no annotations).
    pub(crate) fn snapshot(&self) -> AppSnapshot {
        AppSnapshot {
            red_band: self.band_selection.0,
            green_band: self.band_selection.1,
            blue_band: self.band_selection.2,
            brightness: self.brightness_slider.value,
            contrast: self.contrast_slider.value,
            gamma: self.gamma_slider.value,
            hue: self.hue_slider.value,
            annotations: None,
        }
    }

    /// Create a snapshot with annotation state included.
    pub(crate) fn snapshot_with_annotations(&self) -> AppSnapshot {
        let path = self.current_image_path();
        let image_data = self.image_data_store.get(&path);

        AppSnapshot {
            red_band: self.band_selection.0,
            green_band: self.band_selection.1,
            blue_band: self.band_selection.2,
            brightness: self.brightness_slider.value,
            contrast: self.contrast_slider.value,
            gamma: self.gamma_slider.value,
            hue: self.hue_slider.value,
            annotations: Some(crate::state::AnnotationState {
                image_path: path,
                annotations: image_data.annotations.clone(),
                next_annotation_id: image_data.next_annotation_id,
            }),
        }
    }

    /// Restore state from a snapshot.
    fn restore(&mut self, snapshot: &AppSnapshot) {
        // Restore slider state
        self.band_selection = (snapshot.red_band, snapshot.green_band, snapshot.blue_band);
        self.red_band_slider.set_value(snapshot.red_band as f32);
        self.green_band_slider.set_value(snapshot.green_band as f32);
        self.blue_band_slider.set_value(snapshot.blue_band as f32);
        self.brightness_slider.set_value(snapshot.brightness);
        self.contrast_slider.set_value(snapshot.contrast);
        self.gamma_slider.set_value(snapshot.gamma);
        self.hue_slider.set_value(snapshot.hue);
        self.needs_gpu_render = true;

        // Restore annotation state if present
        if let Some(ref ann_state) = snapshot.annotations {
            let image_data = self.image_data_store.get_or_create(&ann_state.image_path);
            image_data.annotations = ann_state.annotations.clone();
            image_data.next_annotation_id = ann_state.next_annotation_id;
            self.auto_save.mark_dirty();
            log::info!(
                "Restored {} annotations for {:?}",
                ann_state.annotations.len(),
                ann_state.image_path
            );
        }
    }

    /// Get the current image path (for per-image data storage).
    /// Returns a test image path if no project is loaded.
    pub(crate) fn current_image_path(&self) -> PathBuf {
        self.project
            .as_ref()
            .and_then(|p| p.current_image().cloned())
            .unwrap_or_else(|| PathBuf::from("__test_image__"))
    }

    /// Push an undo point with annotation state to the unified undo stack.
    /// Call this before any annotation modification.
    fn push_annotation_undo_point(&self) {
        let snapshot = self.snapshot_with_annotations();
        self.undo_stack.borrow_mut().push(snapshot);
        log::debug!("Pushed annotation undo point");
    }

    /// Reset adjustment sliders to default values.
    fn reset_adjustment_sliders(&mut self) {
        self.brightness_slider.set_value(DEFAULT_BRIGHTNESS);
        self.contrast_slider.set_value(DEFAULT_CONTRAST);
        self.gamma_slider.set_value(DEFAULT_GAMMA);
        self.hue_slider.set_value(DEFAULT_HUE);
    }

    /// Reset band sliders to default values, clamped to available bands.
    fn reset_band_sliders(&mut self) {
        let max_band = (self.num_bands - 1) as f32;
        let green_band = ((DEFAULT_RED_BAND + 1) as f32).min(max_band);
        let blue_band = ((DEFAULT_RED_BAND + 2) as f32).min(max_band);

        self.red_band_slider.set_value(DEFAULT_RED_BAND as f32);
        self.green_band_slider.set_value(green_band);
        self.blue_band_slider.set_value(blue_band);

        self.band_selection = (DEFAULT_RED_BAND, green_band as usize, blue_band as usize);
    }

    /// Update GPU cache: evict out-of-range entries and return list of paths to preload.
    ///
    /// This is the unified cache management method called by both WASM and native preloading.
    fn update_gpu_cache_and_get_preload_list(&mut self) -> Vec<PathBuf> {
        let Some(ref project) = self.project else {
            return Vec::new();
        };

        // Evict out-of-range entries to free GPU memory
        let to_keep = self
            .gpu_cache
            .paths_to_keep(&project.images, project.current_index);
        self.gpu_cache.retain_only(&to_keep);

        // Get paths that need to be preloaded (not cached, within range)
        self.gpu_cache
            .paths_to_preload(&project.images, project.current_index)
    }

    /// Evict out-of-range entries from GPU cache (without returning preload list).
    fn evict_gpu_cache(&mut self) {
        let Some(ref project) = self.project else {
            return;
        };

        let to_keep = self
            .gpu_cache
            .paths_to_keep(&project.images, project.current_index);
        self.gpu_cache.retain_only(&to_keep);
    }

    /// Get paths that need to be preloaded (not cached, within range).
    /// Only used in WASM preloading (native uses update_gpu_cache_and_get_preload_list).
    #[cfg(target_arch = "wasm32")]
    fn get_preload_list(&self) -> Vec<PathBuf> {
        let Some(ref project) = self.project else {
            return Vec::new();
        };

        self.gpu_cache
            .paths_to_preload(&project.images, project.current_index)
    }

    /// Check if any text input field is currently focused.
    fn any_text_input_focused(&self) -> bool {
        // Text input fields
        self.category_name_input_state.is_focused
            || self.tag_input_state.is_focused
            || self.export_folder_state.is_focused
            || self.import_folder_state.is_focused
            // Slider text inputs
            || self.gpu_preload_slider.input_focused
            || self.red_band_slider.input_focused
            || self.green_band_slider.input_focused
            || self.blue_band_slider.input_focused
            || self.brightness_slider.input_focused
            || self.contrast_slider.input_focused
            || self.gamma_slider.input_focused
            || self.hue_slider.input_focused
    }

    /// Handle keyboard events for undo/redo, annotation shortcuts, and custom keybindings.
    fn handle_key_event(&self, event: &Event) -> Option<Message> {
        if let Event::KeyPress { key, modifiers, .. } = event {
            // If we're capturing a keybind, route to key capture handler
            if self.capturing_keybind.is_some() {
                // Escape cancels capture, any other key sets the binding
                if *key == KeyCode::Escape {
                    return Some(Message::CancelCapturingKeybind);
                }
                return Some(Message::KeyCaptured(*key));
            }

            // If a text input is focused, don't process hotkeys (let the text input handle them)
            // Exception: Escape should still work to cancel/unfocus
            if self.any_text_input_focused() {
                // Only allow Escape through to cancel annotation/unfocus
                if *key == KeyCode::Escape {
                    return Some(Message::CancelAnnotation);
                }
                // All other keys go to the text input
                return None;
            }

            // Ctrl+key shortcuts (hardcoded, not customizable)
            if modifiers.ctrl {
                match key {
                    KeyCode::Z if modifiers.shift => return Some(Message::Redo),
                    KeyCode::Z => return Some(Message::Undo),
                    KeyCode::Y => return Some(Message::Redo),
                    _ => {}
                }
            }

            // Non-modifier shortcuts for annotations (hardcoded)
            match key {
                KeyCode::Escape => return Some(Message::CancelAnnotation),
                KeyCode::Delete | KeyCode::Backspace => return Some(Message::DeleteAnnotation),
                KeyCode::Enter => return Some(Message::FinishPolygon),
                _ => {}
            }

            // Only process custom keybindings when no modifiers are pressed
            if !modifiers.ctrl && !modifiers.alt && !modifiers.super_key {
                // Check for tool hotkeys
                if let Some(tool) = self.keybindings.tool_for_key(*key) {
                    return Some(Message::ToolSelected(tool));
                }

                // Check for category hotkeys
                if let Some(index) = self.keybindings.category_index_for_key(*key) {
                    // Map index to category ID
                    if let Some(category) = self.categories.get(index) {
                        // If an annotation is selected, change its category instead
                        let image_data = self.image_data_store.get(&self.current_image_path());
                        let has_selected = image_data.annotations.iter().any(|a| a.selected);
                        if has_selected {
                            return Some(Message::ChangeSelectedAnnotationCategory(category.id));
                        } else {
                            return Some(Message::CategorySelected(category.id));
                        }
                    }
                }
            }
        }
        None
    }

    /// Build current band selection uniform from state.
    fn band_selection_uniform(&self) -> BandSelectionUniform {
        BandSelectionUniform {
            red_band: self.band_selection.0 as u32,
            green_band: self.band_selection.1 as u32,
            blue_band: self.band_selection.2 as u32,
            num_bands: self.num_bands as u32,
        }
    }

    /// Build current image adjustments from slider state.
    fn image_adjustments(&self) -> ImageAdjustments {
        ImageAdjustments {
            brightness: self.brightness_slider.value,
            contrast: self.contrast_slider.value,
            gamma: self.gamma_slider.value,
            hue_shift: self.hue_slider.value,
        }
    }

    /// Initialize GPU pipeline and upload band data.
    fn init_gpu_state(&mut self, resources: &mut Resources<'_>) {
        let Some(ref hyper) = self.hyperspectral else {
            log::error!("Cannot init GPU state: no hyperspectral data");
            return;
        };
        let Some(ref pipeline) = self.shared_pipeline else {
            log::error!("Cannot init GPU state: no shared pipeline");
            return;
        };

        let gpu_ctx = resources.gpu_context();

        match GpuRenderState::new(
            gpu_ctx,
            pipeline,
            hyper,
            self.band_selection,
            self.image_adjustments(),
        ) {
            Ok(state) => {
                self.image_size = (state.width, state.height);
                self.gpu_state = Some(state);
                self.needs_gpu_render = true;
                log::info!("GPU state initialized successfully");
            }
            Err(e) => {
                log::error!("Failed to initialize GPU state: {:?}", e);
            }
        }
    }

    /// Render hyperspectral composite to the render target texture.
    fn render_to_texture(&mut self, resources: &mut Resources<'_>) {
        let Some(ref gpu_state) = self.gpu_state else {
            return;
        };
        let Some(ref pipeline) = self.shared_pipeline else {
            return;
        };

        let gpu_ctx = resources.gpu_context();

        gpu_state.render(
            gpu_ctx,
            pipeline,
            self.band_selection_uniform(),
            self.image_adjustments(),
        );

        // Register render target with UI renderer if not already done
        if self.texture_id.is_none() {
            let id = resources.register_texture(&gpu_state.render_target);
            self.texture_id = Some(id);
            log::info!("Registered render target texture with UI renderer");
        }

        log::debug!(
            "GPU render: bands ({}, {}, {}), brightness={:.2}, contrast={:.2}, gamma={:.2}, hue={:.0}",
            self.band_selection.0,
            self.band_selection.1,
            self.band_selection.2,
            self.brightness_slider.value,
            self.contrast_slider.value,
            self.gamma_slider.value,
            self.hue_slider.value,
        );
    }

    // =========================================================================
    // Configuration Export/Import
    // =========================================================================

    /// Build the current configuration from app state.
    fn build_config(&self) -> crate::config::AppConfig {
        use crate::config::{
            AppConfig, CategoryConfig, KeyBindingsConfig, TagConfig, UserPreferences,
        };

        AppConfig {
            version: crate::config::CONFIG_VERSION,
            app_name: "HVAT".to_string(),
            preferences: UserPreferences {
                dark_theme: self.dark_theme,
                export_folder: self.export_folder.clone(),
                import_folder: self.import_folder.clone(),
                gpu_preload_count: self.gpu_preload_count,
                log_level: self.log_level,
            },
            keybindings: KeyBindingsConfig::from(&self.keybindings),
            categories: self.categories.iter().map(CategoryConfig::from).collect(),
            tags: self.tags.iter().map(TagConfig::from).collect(),
        }
    }

    /// Auto-save configuration to persistent storage.
    /// On native: saves to ~/.config/hvat/hvat-config.json
    /// On WASM: saves to browser localStorage
    fn auto_save_config(&self) {
        let config = self.build_config();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(e) = config.save_to_default_path() {
                log::warn!("Failed to auto-save config: {}", e);
            } else {
                log::debug!("Auto-saved configuration");
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Err(e) = config.save_to_local_storage() {
                log::warn!("Failed to auto-save config to localStorage: {}", e);
            } else {
                log::debug!("Auto-saved configuration to localStorage");
            }
        }
    }

    /// Delete a category and all annotations using it.
    fn delete_category_internal(&mut self, id: u32, cat_name: &str) {
        // Remove category from list
        if let Some(pos) = self.categories.iter().position(|c| c.id == id) {
            self.categories.remove(pos);

            // Remove all annotations using this category from all images
            let removed_count = self.image_data_store.remove_annotations_by_category(id);

            // If the deleted category was selected, select the first available
            if self.selected_category == id {
                self.selected_category = self.categories.first().map(|c| c.id).unwrap_or(0);
            }

            self.auto_save.mark_dirty();
            log::info!(
                "Deleted category '{}' (id={}) and {} annotation(s)",
                cat_name,
                id,
                removed_count
            );
            self.auto_save_config();
        }
    }

    fn delete_tag_internal(&mut self, id: u32, tag_name: &str) {
        // Remove tag from list
        if let Some(pos) = self.tags.iter().position(|t| t.id == id) {
            self.tags.remove(pos);

            // Remove from all per-image selections
            self.image_data_store.remove_tag_from_all(id);

            // If the deleted tag was selected, select the first available
            if self.selected_tag == id {
                self.selected_tag = self.tags.first().map(|t| t.id).unwrap_or(0);
            }

            self.auto_save.mark_dirty();
            log::info!("Deleted tag '{}' (id={})", tag_name, id);
            self.auto_save_config();
        }
    }

    /// Export configuration to a JSON file.
    fn export_config(&self) {
        let config = self.build_config();
        let json = match config.to_json() {
            Ok(json) => json,
            Err(e) => {
                log::error!("Failed to serialize config: {}", e);
                return;
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(crate::config::AppConfig::default_filename())
                .add_filter("JSON", &["json"])
                .save_file()
            {
                if let Err(e) = std::fs::write(&path, &json) {
                    log::error!("Failed to write config file: {}", e);
                } else {
                    log::info!("Configuration exported to {:?}", path);
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Save to localStorage for persistence
            if let Err(e) = config.save_to_local_storage() {
                log::warn!("Failed to save config to localStorage: {}", e);
            }

            // Download file via browser
            self.download_file_wasm(
                crate::config::AppConfig::default_filename(),
                "application/json",
                json.as_bytes(),
            );
        }
    }

    /// Download a file in WASM by creating a temporary anchor element.
    #[cfg(target_arch = "wasm32")]
    fn download_file_wasm(&self, filename: &str, mime_type: &str, data: &[u8]) {
        use wasm_bindgen::JsCast;

        let window = match web_sys::window() {
            Some(w) => w,
            None => {
                log::error!("No window object for download");
                return;
            }
        };
        let document = match window.document() {
            Some(d) => d,
            None => {
                log::error!("No document for download");
                return;
            }
        };

        // Create blob from data
        let array = js_sys::Uint8Array::from(data);
        let blob_parts = js_sys::Array::new();
        blob_parts.push(&array.buffer());

        let blob_options = web_sys::BlobPropertyBag::new();
        blob_options.set_type(mime_type);

        let blob =
            match web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &blob_options)
            {
                Ok(b) => b,
                Err(e) => {
                    log::error!("Failed to create blob: {:?}", e);
                    return;
                }
            };

        // Create object URL
        let url = match web_sys::Url::create_object_url_with_blob(&blob) {
            Ok(u) => u,
            Err(e) => {
                log::error!("Failed to create object URL: {:?}", e);
                return;
            }
        };

        // Create anchor element for download
        let anchor: web_sys::HtmlAnchorElement = match document.create_element("a") {
            Ok(el) => match el.dyn_into() {
                Ok(a) => a,
                Err(_) => {
                    log::error!("Failed to cast to anchor");
                    let _ = web_sys::Url::revoke_object_url(&url);
                    return;
                }
            },
            Err(e) => {
                log::error!("Failed to create anchor: {:?}", e);
                let _ = web_sys::Url::revoke_object_url(&url);
                return;
            }
        };

        anchor.set_href(&url);
        anchor.set_download(filename);

        // Some browsers require the anchor to be in the DOM
        let body = match document.body() {
            Some(b) => b,
            None => {
                log::error!("No document body for download");
                let _ = web_sys::Url::revoke_object_url(&url);
                return;
            }
        };

        // Temporarily add to DOM, click, then remove
        if let Err(e) = body.append_child(&anchor) {
            log::error!("Failed to append anchor to body: {:?}", e);
            let _ = web_sys::Url::revoke_object_url(&url);
            return;
        }

        anchor.click();

        // Remove from DOM
        let _ = body.remove_child(&anchor);

        // Schedule URL revocation after download has started
        // Use setTimeout to delay revocation - the download needs time to begin
        let url_clone = url.clone();
        let closure = wasm_bindgen::closure::Closure::once(Box::new(move || {
            let _ = web_sys::Url::revoke_object_url(&url_clone);
            log::debug!("Revoked object URL for download");
        }) as Box<dyn FnOnce()>);

        if let Err(e) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            1000, // 1 second delay
        ) {
            log::warn!("Failed to schedule URL revocation: {:?}", e);
            // Clean up immediately if setTimeout fails
            let _ = web_sys::Url::revoke_object_url(&url);
        } else {
            // Prevent closure from being dropped before timeout fires
            closure.forget();
        }

        log::info!("Configuration download initiated: {}", filename);
    }

    /// Import configuration from a JSON file.
    fn import_config(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file()
            {
                match std::fs::read_to_string(&path) {
                    Ok(json) => {
                        self.apply_config_from_json(&json);
                    }
                    Err(e) => {
                        log::error!("Failed to read config file: {}", e);
                    }
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.show_file_picker_wasm();
        }
    }

    /// Show file picker for importing config in WASM.
    #[cfg(target_arch = "wasm32")]
    fn show_file_picker_wasm(&self) {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::prelude::*;

        let window = match web_sys::window() {
            Some(w) => w,
            None => {
                log::error!("No window object");
                return;
            }
        };
        let document = match window.document() {
            Some(d) => d,
            None => {
                log::error!("No document object");
                return;
            }
        };

        // Create hidden input element
        let input: web_sys::HtmlInputElement = match document.create_element("input") {
            Ok(el) => match el.dyn_into() {
                Ok(i) => i,
                Err(_) => {
                    log::error!("Failed to cast to input");
                    return;
                }
            },
            Err(e) => {
                log::error!("Failed to create input: {:?}", e);
                return;
            }
        };

        input.set_type("file");
        input.set_accept(".json,application/json");
        input.style().set_property("display", "none").ok();

        // Append to body
        if let Some(body) = document.body() {
            let _ = body.append_child(&input);
        }

        // Use the global pending config state
        let input_clone = input.clone();
        let change_closure = Closure::once(Box::new(move |_event: web_sys::Event| {
            let files = match input_clone.files() {
                Some(f) => f,
                None => return,
            };

            if files.length() == 0 {
                return;
            }

            let file = match files.get(0) {
                Some(f) => f,
                None => return,
            };

            // Read file contents
            wasm_bindgen_futures::spawn_local(async move {
                match hvat_ui::read_file_async(&file).await {
                    Ok(data) => {
                        match String::from_utf8(data) {
                            Ok(json) => {
                                // Store in pending config state
                                if let Ok(mut pending) = pending_config_state().lock() {
                                    *pending = Some(json);
                                }
                            }
                            Err(e) => {
                                log::error!("Config file is not valid UTF-8: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read config file: {}", e);
                    }
                }
            });

            // Clean up
            if let Some(parent) = input_clone.parent_element() {
                let _ = parent.remove_child(&input_clone);
            }
        }));

        input.set_onchange(Some(change_closure.as_ref().unchecked_ref()));
        change_closure.forget();

        input.click();
    }

    /// Apply configuration from JSON string.
    fn apply_config_from_json(&mut self, json: &str) {
        use crate::config::AppConfig;

        log::info!("Applying config from JSON ({} bytes)", json.len());

        match AppConfig::from_json(json) {
            Ok(config) => {
                log::info!(
                    "Parsed config version {}: dark_theme={}, gpu_preload={}, {} categories",
                    config.version,
                    config.preferences.dark_theme,
                    config.preferences.gpu_preload_count,
                    config.categories.len()
                );

                // Log current state before applying
                log::debug!(
                    "Before apply: dark_theme={}, gpu_preload={}, {} categories",
                    self.dark_theme,
                    self.gpu_preload_count,
                    self.categories.len()
                );

                // Apply preferences
                self.dark_theme = config.preferences.dark_theme;
                self.export_folder = config.preferences.export_folder;
                self.import_folder = config.preferences.import_folder;
                self.gpu_preload_count = config
                    .preferences
                    .gpu_preload_count
                    .min(MAX_GPU_PRELOAD_COUNT);
                self.gpu_preload_slider = SliderState::new(self.gpu_preload_count as f32);
                self.gpu_cache.set_preload_count(self.gpu_preload_count);
                self.log_level = config.preferences.log_level;
                log::set_max_level(self.log_level.to_level_filter());

                // Apply keybindings
                self.keybindings = config.keybindings.to_keybindings();

                // Apply categories
                self.categories = config.categories.into_iter().map(|c| c.into()).collect();

                // Apply tags
                self.tags = config.tags.into_iter().map(|t| t.into()).collect();

                // Ensure we have at least one category and a valid selection
                if self.categories.is_empty() {
                    self.categories
                        .push(Category::new(1, "Default", [100, 100, 100]));
                }
                if !self
                    .categories
                    .iter()
                    .any(|c| c.id == self.selected_category)
                {
                    self.selected_category = self.categories[0].id;
                }

                // Ensure we have at least one tag and a valid selection
                if self.tags.is_empty() {
                    self.tags.push(Tag::new(1, "Default", [100, 140, 180]));
                }
                if !self.tags.iter().any(|t| t.id == self.selected_tag) {
                    self.selected_tag = self.tags[0].id;
                }

                // Log state after applying
                log::info!(
                    "After apply: dark_theme={}, gpu_preload={}, {} categories",
                    self.dark_theme,
                    self.gpu_preload_count,
                    self.categories.len()
                );
                for (i, cat) in self.categories.iter().enumerate() {
                    log::debug!(
                        "  Category {}: id={}, name='{}', color={:?}",
                        i,
                        cat.id,
                        cat.name,
                        cat.color
                    );
                }

                // Save imported config to localStorage for persistence (WASM only)
                #[cfg(target_arch = "wasm32")]
                {
                    let new_config = self.build_config();
                    if let Err(e) = new_config.save_to_local_storage() {
                        log::warn!("Failed to save imported config to localStorage: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to parse configuration: {}", e);
            }
        }
    }

    /// Handle native file drop (files and/or folders).
    /// Supports dropping folders (scanned recursively), multiple image files, and ZIP archives.
    #[cfg(not(target_arch = "wasm32"))]
    fn handle_native_file_drop(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }

        log::info!("Native file drop: {} paths", paths.len());
        for path in &paths {
            log::debug!(
                "  - {:?} (is_dir={}, is_file={}, is_zip={})",
                path,
                path.is_dir(),
                path.is_file(),
                is_zip_path(path)
            );
        }

        // Check if any dropped file is a ZIP archive
        let zip_files: Vec<&PathBuf> = paths.iter().filter(|p| is_zip_path(p)).collect();

        if !zip_files.is_empty() {
            // Handle ZIP files - extract images from the first ZIP file
            // (Multiple ZIP files at once is unusual, so we use the first one)
            let zip_path = zip_files[0];
            log::info!("Processing ZIP file: {:?}", zip_path);

            match extract_images_from_zip_file(zip_path) {
                Ok(loaded_images) => {
                    self.gpu_cache.clear();
                    log::info!("Extracted {} images from ZIP archive", loaded_images.len());

                    // Create project from loaded images (like WASM does)
                    match ProjectState::from_loaded_images(loaded_images) {
                        Ok(project) => {
                            log::info!(
                                "Loaded project with {} images from ZIP",
                                project.images.len()
                            );
                            // Extract dimensions from all loaded images for export
                            self.extract_dimensions_from_loaded_images(&project);
                            self.project = Some(project);
                            self.pending_image_load = true;
                        }
                        Err(e) => {
                            log::error!("Failed to create project from ZIP contents: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to extract ZIP archive: {}", e);
                }
            }
            return;
        }

        // No ZIP files - use the regular from_paths method which handles both files and folders
        match ProjectState::from_paths(paths) {
            Ok(project) => {
                self.gpu_cache.clear();
                log::info!(
                    "Loaded project with {} images from drop, folder: {:?}",
                    project.images.len(),
                    project.folder
                );
                self.project = Some(project);
                self.pending_image_load = true;
            }
            Err(e) => {
                log::error!("Failed to load dropped items: {}", e);
            }
        }
    }

    /// Load an image file and reinitialize GPU state.
    ///
    /// If the image is already in the GPU cache, uses the cached textures.
    /// Otherwise, loads from disk and uploads to GPU.
    fn load_image_file(&mut self, path: PathBuf, resources: &mut Resources<'_>) {
        log::info!(
            "Loading image: {:?} (cache_size={}, contains={})",
            path,
            self.gpu_cache.len(),
            self.gpu_cache.contains(&path)
        );

        if self.shared_pipeline.is_none() {
            log::error!("Cannot load image: no shared pipeline");
            return;
        }

        // Return current GPU state to cache before loading new image
        // This preserves the GPU textures for instant switching back
        if let (Some(old_state), Some(old_path)) =
            (self.gpu_state.take(), self.current_gpu_image_path.take())
        {
            // Only cache if it's not the same image we're about to load
            if old_path != path {
                log::debug!("Returning {:?} to GPU cache", old_path);
                self.gpu_cache.insert(old_path, old_state.into_cached());
            }
        }

        // Evict out-of-range entries immediately to free GPU memory
        self.evict_gpu_cache();

        // Check if image is cached in GPU
        if let Some(cached) = self.gpu_cache.take(&path) {
            log::info!("*** CACHE HIT *** Using cached GPU data for {:?}", path);

            self.num_bands = cached.num_bands;
            self.reset_band_sliders();
            self.reset_adjustment_sliders();
            self.texture_id = None;

            let gpu_ctx = resources.gpu_context();
            let pipeline = self.shared_pipeline.as_ref().unwrap();

            match GpuRenderState::from_cached(
                gpu_ctx,
                pipeline,
                cached,
                self.band_selection,
                self.image_adjustments(),
            ) {
                Ok(state) => {
                    self.image_size = (state.width, state.height);
                    self.gpu_state = Some(state);
                    self.current_gpu_image_path = Some(path.clone());
                    // Store dimensions in image data for export
                    let image_data = self.image_data_store.get_or_create(&path);
                    image_data.dimensions = Some(self.image_size);
                    // Render immediately so image displays in same frame
                    self.render_to_texture(resources);
                    self.needs_gpu_render = false;
                    self.pending_preload = true; // Trigger preloading for adjacent images
                    log::info!(
                        "Loaded from cache: {}x{} with {} bands",
                        self.image_size.0,
                        self.image_size.1,
                        self.num_bands
                    );
                }
                Err(e) => {
                    log::error!("Failed to create state from cache: {:?}", e);
                }
            }
            return;
        }

        // Not cached - load from disk/memory using unified API
        let hyper_result = if let Some(ref project) = self.project {
            project
                .get_image_data(&path)
                .and_then(|data| HyperspectralData::from_bytes(&data))
        } else {
            Err("No project loaded".to_string())
        };

        match hyper_result {
            Ok(hyper) => {
                self.num_bands = hyper.bands.len();
                self.reset_band_sliders();
                self.reset_adjustment_sliders();

                self.texture_id = None;
                self.hyperspectral = Some(hyper);

                self.init_gpu_state(resources);
                self.current_gpu_image_path = Some(path.clone());
                // Store dimensions in image data for export
                let image_data = self.image_data_store.get_or_create(&path);
                image_data.dimensions = Some(self.image_size);
                self.render_to_texture(resources);
                self.pending_preload = true; // Trigger preloading for adjacent images

                log::info!(
                    "Loaded image: {}x{} with {} bands",
                    self.image_size.0,
                    self.image_size.1,
                    self.num_bands
                );
            }
            Err(e) => {
                log::error!("Failed to load image {:?}: {}", path, e);
            }
        }
    }

    /// Preload adjacent images into the GPU cache.
    ///
    /// Called each tick to progressively preload images without blocking the UI.
    /// Returns true if there are more images to preload.
    ///
    /// On WASM: Uses a Web Worker for async decoding, then chunked GPU uploads
    /// to spread texture upload work across frames (one layer per tick).
    /// On Native: Decodes synchronously (one image per tick).
    #[cfg(target_arch = "wasm32")]
    fn do_preloading_step(&mut self, resources: &mut Resources<'_>) -> bool {
        use crate::state::DecodeResult;

        // Early return checks - avoid borrowing self.project across function
        if self.project.is_none() || self.shared_pipeline.is_none() {
            return false;
        }
        if self
            .project
            .as_ref()
            .map(|p| p.images.is_empty())
            .unwrap_or(true)
            || self.gpu_preload_count == 0
        {
            return false;
        }

        // Try to use the Web Worker for async decoding
        if self.wasm_preload.decoder_worker.is_none() {
            // Fallback to sync if worker failed to spawn
            return self.do_preloading_step_sync(resources);
        }

        // Flush any queued messages (worker may have just become ready)
        if let Some(ref mut worker) = self.wasm_preload.decoder_worker {
            worker.flush_queue();
        }

        // Evict out-of-range entries to free GPU memory
        self.evict_gpu_cache();

        let gpu_ctx = resources.gpu_context();
        let mut did_gpu_work = false;

        // Step 1: Process completed decodes from worker → queue for chunked upload
        let worker_result = self
            .wasm_preload
            .decoder_worker
            .as_mut()
            .and_then(|w| w.take_one_result());
        if let Some(result) = worker_result {
            match result {
                DecodeResult::Decoded(decoded) => {
                    // Don't queue if already in cache or already queued
                    if !self.gpu_cache.contains(&decoded.path)
                        && !self
                            .wasm_preload
                            .chunked_upload_queue
                            .is_queued(&decoded.path)
                    {
                        log::info!(
                            "Worker decoded {:?} ({}x{}, {} layers pre-packed), queueing for GPU upload",
                            decoded.path,
                            decoded.width,
                            decoded.height,
                            decoded.layers.len()
                        );
                        // Use queue_prepacked since worker already did the RGBA packing
                        self.wasm_preload.chunked_upload_queue.queue_prepacked(
                            decoded.path,
                            decoded.width,
                            decoded.height,
                            decoded.num_bands,
                            decoded.num_layers,
                            decoded.layers,
                            &gpu_ctx.device,
                        );
                    }
                }
                DecodeResult::Error(err) => {
                    log::warn!("Worker decode failed for {:?}: {}", err.path, err.error);
                }
            }
        }

        // Step 2: Process ONE texture layer from chunked queue (GPU work, ~1-2ms)
        if self
            .wasm_preload
            .chunked_upload_queue
            .process_one_layer(gpu_ctx)
        {
            did_gpu_work = true;
        }

        // Step 3: Move completed chunked uploads to GPU cache
        let pipeline = self.shared_pipeline.as_ref().unwrap();
        for completed in self.wasm_preload.chunked_upload_queue.take_completed() {
            log::info!(
                "Moving completed upload to cache: {:?} ({}x{}, {} bands)",
                completed.path,
                completed.width,
                completed.height,
                completed.num_bands
            );
            self.gpu_cache.insert_from_texture(
                gpu_ctx,
                completed.path,
                completed.texture,
                completed.width,
                completed.height,
                completed.num_bands,
                completed.num_layers,
                pipeline.band_texture_layout(),
            );
        }

        // Get paths that still need to be preloaded
        let to_preload = self.get_preload_list();

        // Check if we're done: no paths to preload, no pending work anywhere
        let chunked_pending = self.wasm_preload.chunked_upload_queue.has_pending()
            || self.wasm_preload.chunked_upload_queue.has_completed();
        let worker_pending = self
            .wasm_preload
            .decoder_worker
            .as_ref()
            .map(|w| w.pending_count() > 0 || w.has_results())
            .unwrap_or(false);
        if to_preload.is_empty() && !worker_pending && !chunked_pending {
            log::debug!(
                "Preloading complete: {} images in GPU cache",
                self.gpu_cache.len()
            );
            self.preload_did_gpu_work = did_gpu_work;
            return false;
        }

        // Request new decodes (limit in-flight to avoid memory pressure)
        const MAX_IN_FLIGHT: usize = 3;
        let project = self.project.as_ref().unwrap();
        for path in &to_preload {
            let pending_count = self
                .wasm_preload
                .decoder_worker
                .as_ref()
                .map(|w| w.pending_count())
                .unwrap_or(MAX_IN_FLIGHT);
            if pending_count >= MAX_IN_FLIGHT {
                break;
            }
            // Skip if already cached, already pending in worker, or already in chunked queue
            let is_pending = self
                .wasm_preload
                .decoder_worker
                .as_ref()
                .map(|w| w.is_pending(path))
                .unwrap_or(false);
            if self.gpu_cache.contains(path)
                || is_pending
                || self.wasm_preload.chunked_upload_queue.is_queued(path)
            {
                continue;
            }
            // Find image data and send to worker
            if let Ok(data) = project.get_image_data(path) {
                if let Some(ref mut worker) = self.wasm_preload.decoder_worker {
                    log::debug!("Requesting worker decode for {:?}", path);
                    worker.request_decode(path.clone(), data);
                }
            }
        }

        // Store whether we did GPU work this tick (for TickResult selection)
        self.preload_did_gpu_work = did_gpu_work;

        // Check if worker has pending work
        let worker_has_work = self
            .wasm_preload
            .decoder_worker
            .as_ref()
            .map(|w| w.pending_count() > 0 || w.has_results())
            .unwrap_or(false);

        // Continue if work is pending anywhere in the pipeline
        !to_preload.is_empty() || worker_has_work || chunked_pending
    }

    /// Native preloading: async decode in background thread + chunked GPU upload.
    ///
    /// Uses the same three-stage pipeline as WASM:
    /// 1. Background thread decodes + packs RGBA
    /// 2. Chunked upload queue spreads GPU uploads across frames
    /// 3. Cache insertion when complete
    #[cfg(not(target_arch = "wasm32"))]
    fn do_preloading_step(&mut self, resources: &mut Resources<'_>) -> bool {
        // Early return checks
        if self.project.is_none() || self.shared_pipeline.is_none() {
            return false;
        }

        if self
            .project
            .as_ref()
            .map(|p| p.images.is_empty())
            .unwrap_or(true)
            || self.gpu_preload_count == 0
        {
            return false;
        }

        // If no native preload state, fall back to sync
        let Some(ref mut native_preload) = self.native_preload else {
            log::debug!("No native preload state, using sync fallback");
            return self.do_preloading_step_sync(resources);
        };

        let gpu_ctx = resources.gpu_context();
        let pipeline = self.shared_pipeline.as_ref().unwrap();

        // Step 1: Process decoder results -> queue for chunked upload
        if let Some(result) = native_preload.decoder.take_one_result() {
            match result {
                DecodeResult::Decoded(img) => {
                    log::debug!(
                        "Decoder finished {:?}: {}x{} with {} layers",
                        img.path,
                        img.width,
                        img.height,
                        img.num_layers
                    );
                    native_preload.chunked_upload_queue.queue_prepacked(
                        img.path,
                        img.width,
                        img.height,
                        img.num_bands,
                        img.num_layers,
                        img.layers,
                        &gpu_ctx.device,
                    );
                }
                DecodeResult::Error(err) => {
                    log::warn!("Decode error for {:?}: {}", err.path, err.error);
                }
            }
        }

        // Step 2: Process one chunk of GPU upload
        let did_gpu_work = native_preload
            .chunked_upload_queue
            .process_one_layer(gpu_ctx);
        self.preload_did_gpu_work = did_gpu_work;

        // Step 3: Move completed uploads to cache
        for completed in native_preload.chunked_upload_queue.take_completed() {
            self.gpu_cache.insert_from_texture(
                gpu_ctx,
                completed.path,
                completed.texture,
                completed.width,
                completed.height,
                completed.num_bands,
                completed.num_layers,
                pipeline.band_texture_layout(),
            );
        }

        // Step 4: Request new decodes (limit in-flight)
        let to_preload = self.update_gpu_cache_and_get_preload_list();

        // Re-borrow native_preload after update_gpu_cache_and_get_preload_list
        let Some(ref mut native_preload) = self.native_preload else {
            return false;
        };

        for path in &to_preload {
            let pending_count = native_preload.decoder.pending_count();
            if pending_count >= MAX_IN_FLIGHT_DECODES {
                break;
            }

            // Skip if already cached, being decoded, or queued for upload
            if self.gpu_cache.contains(path)
                || native_preload.decoder.is_pending(path)
                || native_preload.chunked_upload_queue.is_queued(path)
            {
                continue;
            }

            // Load image data and send to decoder
            if let Some(data) = self
                .project
                .as_ref()
                .and_then(|p| p.get_image_data(path).ok())
            {
                log::debug!("Requesting decode for {:?}", path);
                native_preload.decoder.request_decode(path.clone(), data);
            }
        }

        // Check if there's more work to do
        let chunked_pending = native_preload.chunked_upload_queue.has_pending()
            || native_preload.chunked_upload_queue.has_completed();
        let decoder_has_work = native_preload.decoder.pending_count() > 0;

        !to_preload.is_empty() || decoder_has_work || chunked_pending
    }

    /// Synchronous preloading fallback (used if background thread/worker fails to spawn).
    fn do_preloading_step_sync(&mut self, resources: &mut Resources<'_>) -> bool {
        // Early return checks
        if self.project.is_none() || self.shared_pipeline.is_none() {
            return false;
        }

        if self
            .project
            .as_ref()
            .map(|p| p.images.is_empty())
            .unwrap_or(true)
            || self.gpu_preload_count == 0
        {
            return false;
        }

        // Evict out-of-range entries and get paths to preload
        let to_preload = self.update_gpu_cache_and_get_preload_list();

        if to_preload.is_empty() {
            log::debug!(
                "Preloading complete: {} images in GPU cache",
                self.gpu_cache.len()
            );
            return false;
        }

        // Preload just ONE image per tick to avoid blocking the UI
        let path = to_preload[0].clone();
        log::info!("Preloading sync (1 of {}): {:?}", to_preload.len(), path);

        let gpu_ctx = resources.gpu_context();

        // Use unified API that works for both WASM (in-memory) and native (filesystem)
        let hyper_result = self
            .project
            .as_ref()
            .unwrap()
            .get_image_data(&path)
            .and_then(|data| HyperspectralData::from_bytes(&data));

        match hyper_result {
            Ok(hyper) => {
                let pipeline = self.shared_pipeline.as_ref().unwrap();
                self.gpu_cache.upload_and_cache(
                    gpu_ctx,
                    path.clone(),
                    &hyper,
                    pipeline.band_texture_layout(),
                );
            }
            Err(e) => {
                log::warn!("Failed to load image for preloading {:?}: {}", path, e);
            }
        }

        // Return true if there are more images to preload
        to_preload.len() > 1
    }

    /// Handle pointer events for annotation drawing.
    fn handle_pointer_event(&mut self, event: hvat_ui::ImagePointerEvent) {
        use hvat_ui::{MouseButton, PointerEventKind};

        // Update viewer state to persist pointer_state changes
        self.viewer_state = event.viewer_state.clone();

        let x = event.image_x;
        let y = event.image_y;

        log::trace!(
            "ImagePointer: tool={:?}, pos=({:.1}, {:.1}), kind={:?}, button={:?}",
            self.selected_tool,
            x,
            y,
            event.kind,
            event.button
        );

        // Handle right-click: open context menu
        if event.button == MouseButton::Right && event.kind == PointerEventKind::Click {
            // Check if we clicked on a polygon vertex (for vertex removal - priority action)
            if self.selected_tool == AnnotationTool::Select {
                if self.try_remove_polygon_vertex(x, y) {
                    return;
                }
            }

            // Open context menu at the click position
            // Only use the selected annotation for category changes (matches sidebar behavior)
            // Hovering an annotation does NOT make it the target - must be explicitly selected
            let path = self.current_image_path();
            let selected_annotation_id = self
                .image_data_store
                .get(&path)
                .annotations
                .iter()
                .find(|a| a.selected)
                .map(|a| a.id);
            let screen_pos = (event.screen_x, event.screen_y);
            log::info!(
                "Right-click at image ({:.1}, {:.1}), screen ({:.0}, {:.0}), selected={:?}",
                x,
                y,
                screen_pos.0,
                screen_pos.1,
                selected_annotation_id
            );

            self.context_menu_open = true;
            self.context_menu_position = screen_pos;
            self.context_menu_annotation_id = selected_annotation_id;
            return;
        }

        // Handle left-click events
        match self.selected_tool {
            AnnotationTool::Select => {
                self.handle_select_tool(x, y, event.kind);
            }
            AnnotationTool::BoundingBox => {
                self.handle_bounding_box_draw(x, y, event.kind);
            }
            AnnotationTool::Polygon => {
                self.handle_polygon_draw(x, y, event.kind);
            }
            AnnotationTool::Point => {
                if event.kind == PointerEventKind::DragStart {
                    self.create_point_annotation(x, y);
                }
            }
        }
    }

    /// Get the hit radius scaled for current zoom level.
    fn scaled_hit_radius(&self) -> f32 {
        // Scale the hit radius inversely with zoom so handles are easier to grab at any zoom
        let zoom = self.viewer_state.effective_zoom();
        HANDLE_HIT_RADIUS / zoom.max(0.1)
    }

    /// Handle Select tool - selection, modification, and cycling.
    fn handle_select_tool(&mut self, x: f32, y: f32, kind: hvat_ui::PointerEventKind) {
        use hvat_ui::PointerEventKind;

        match kind {
            PointerEventKind::DragStart => {
                self.handle_selection_drag_start(x, y);
            }
            PointerEventKind::DragMove => {
                self.handle_selection_drag_move(x, y);
            }
            PointerEventKind::DragEnd => {
                self.handle_selection_drag_end(x, y);
            }
            PointerEventKind::Click => {
                // Click is handled via DragStart/DragEnd
            }
        }
    }

    /// Handle drag start in select mode - check for handle hit or start selection.
    fn handle_selection_drag_start(&mut self, x: f32, y: f32) {
        let path = self.current_image_path();
        let hit_radius = self.scaled_hit_radius();

        // First, check if we're clicking on a handle of the currently selected annotation
        let selected_handle = {
            let image_data = self.image_data_store.get(&path);
            image_data
                .annotations
                .iter()
                .find(|ann| ann.selected)
                .and_then(|ann| {
                    ann.shape
                        .hit_test_handle(x, y, hit_radius)
                        .map(|handle| (ann.id, ann.shape.clone(), handle))
                })
        };

        if let Some((annotation_id, original_shape, handle)) = selected_handle {
            use crate::model::{AnnotationHandle, PolygonHandle};

            // Determine the effective handle and shape for dragging
            // Edge clicks insert a vertex first, then drag the new vertex
            let (effective_handle, effective_shape) = if let AnnotationHandle::Polygon(
                PolygonHandle::Edge { index: edge_idx },
            ) = handle
            {
                if let Some(new_shape) = original_shape.insert_polygon_vertex(edge_idx, x, y) {
                    self.push_annotation_undo_point();
                    let image_data = self.image_data_store.get_or_create(&path);
                    if let Some(ann) = image_data
                        .annotations
                        .iter_mut()
                        .find(|a| a.id == annotation_id)
                    {
                        ann.shape = new_shape.clone();
                        self.auto_save.mark_dirty();
                    }
                    log::info!(
                        "Inserted vertex at edge {} of polygon {}, now at index {}",
                        edge_idx,
                        annotation_id,
                        edge_idx + 1
                    );
                    (
                        AnnotationHandle::Polygon(PolygonHandle::Vertex(edge_idx + 1)),
                        new_shape,
                    )
                } else {
                    return;
                }
            } else {
                (handle, original_shape)
            };

            // Polygon vertices start dragging immediately (precise control needed)
            // Other handles use PotentialDrag to differentiate click from drag
            let immediate_drag = matches!(
                effective_handle,
                AnnotationHandle::Polygon(PolygonHandle::Vertex(_))
            );

            log::debug!(
                "Handle interaction on annotation {}: {:?}, immediate={}",
                annotation_id,
                effective_handle,
                immediate_drag
            );

            if immediate_drag {
                self.push_annotation_undo_point();
            }

            let image_data = self.image_data_store.get_or_create(&path);
            image_data.edit_state = if immediate_drag {
                EditState::DraggingHandle {
                    annotation_id,
                    handle: effective_handle,
                    start_x: x,
                    start_y: y,
                    original_shape: effective_shape,
                }
            } else {
                EditState::PotentialDrag {
                    annotation_id,
                    handle: effective_handle,
                    start_x: x,
                    start_y: y,
                    original_shape: effective_shape,
                }
            };
            return;
        }

        // Not clicking on a selected annotation's handle - do selection/cycling
        self.handle_selection_click_with_cycling(x, y);
    }

    /// Handle selection click with cycling through overlapping annotations.
    fn handle_selection_click_with_cycling(&mut self, x: f32, y: f32) {
        let path = self.current_image_path();
        let image_data = self.image_data_store.get_or_create(&path);

        // Find all annotations under cursor
        let hit_indices: Vec<usize> = image_data
            .annotations
            .iter()
            .enumerate()
            .filter(|(_, ann)| ann.shape.contains_point(x, y))
            .map(|(idx, _)| idx)
            .collect();

        if hit_indices.is_empty() {
            // Clicked on empty space - deselect all
            for ann in &mut image_data.annotations {
                ann.selected = false;
            }
            image_data.last_clicked_index = None;
            log::debug!("No annotation at click position, deselected all");
            return;
        }

        // Find which one to select (cycle through overlapping annotations)
        let next_idx = if let Some(last_idx) = image_data.last_clicked_index {
            // If we previously clicked here and there are multiple overlapping,
            // cycle to the next one
            if hit_indices.contains(&last_idx) {
                // Find position of last_idx in hit_indices and get next
                let pos = hit_indices.iter().position(|&i| i == last_idx).unwrap_or(0);
                let next_pos = (pos + 1) % hit_indices.len();
                hit_indices[next_pos]
            } else {
                // Last click was elsewhere, start fresh with top-most (last in render order)
                *hit_indices.last().unwrap()
            }
        } else {
            // No previous click, select top-most
            *hit_indices.last().unwrap()
        };

        // Deselect all and select the chosen one
        for ann in &mut image_data.annotations {
            ann.selected = false;
        }
        image_data.annotations[next_idx].selected = true;
        image_data.last_clicked_index = Some(next_idx);

        let id = image_data.annotations[next_idx].id;
        log::info!(
            "Selected annotation {} (cycling: {} overlapping)",
            id,
            hit_indices.len()
        );
    }

    /// Handle drag move in select mode - update annotation if editing.
    fn handle_selection_drag_move(&mut self, x: f32, y: f32) {
        let path = self.current_image_path();

        // Check if we need to transition from PotentialDrag to DraggingHandle
        let should_start_editing = {
            let image_data = self.image_data_store.get(&path);
            if let EditState::PotentialDrag {
                start_x, start_y, ..
            } = &image_data.edit_state
            {
                let dx = x - start_x;
                let dy = y - start_y;
                let distance = (dx * dx + dy * dy).sqrt();
                distance >= MIN_DRAG_DISTANCE
            } else {
                false
            }
        };

        if should_start_editing {
            // Transition to DraggingHandle and push undo point
            self.push_annotation_undo_point();

            let image_data = self.image_data_store.get_or_create(&path);
            if let EditState::PotentialDrag {
                annotation_id,
                handle,
                start_x,
                start_y,
                original_shape,
            } = image_data.edit_state.clone()
            {
                log::info!(
                    "Starting handle drag on annotation {}, handle={:?}",
                    annotation_id,
                    handle
                );
                image_data.edit_state = EditState::DraggingHandle {
                    annotation_id,
                    handle,
                    start_x,
                    start_y,
                    original_shape,
                };
            }
        }

        // Now handle the actual drag if we're in DraggingHandle state
        let image_data = self.image_data_store.get_or_create(&path);
        if let EditState::DraggingHandle {
            annotation_id,
            handle,
            start_x,
            start_y,
            ref original_shape,
        } = image_data.edit_state.clone()
        {
            // Apply the handle drag to get new shape
            if let Some(new_shape) =
                AnnotationShape::apply_handle_drag(&original_shape, &handle, start_x, start_y, x, y)
            {
                // Find and update the annotation
                if let Some(ann) = image_data
                    .annotations
                    .iter_mut()
                    .find(|a| a.id == annotation_id)
                {
                    ann.shape = new_shape;
                    self.auto_save.mark_dirty();
                }
            }
        }
    }

    /// Handle drag end in select mode - finalize editing or trigger cycling.
    fn handle_selection_drag_end(&mut self, x: f32, y: f32) {
        let path = self.current_image_path();

        // Check what state we're in
        let was_potential_drag = {
            let image_data = self.image_data_store.get(&path);
            image_data.edit_state.is_potential_drag()
        };

        if was_potential_drag {
            // Mouse was released without enough movement - treat as a click for cycling
            let image_data = self.image_data_store.get_or_create(&path);
            image_data.edit_state = EditState::Idle;

            // Now do cycling
            self.handle_selection_click_with_cycling(x, y);
        } else {
            let image_data = self.image_data_store.get_or_create(&path);
            if image_data.edit_state.is_editing() {
                log::info!("Finished editing annotation");
                image_data.edit_state = EditState::Idle;
            }
        }
    }

    /// Try to remove a polygon vertex at the given position.
    /// Returns true if a vertex was removed, false otherwise.
    /// This is used to give priority to vertex removal over opening context menu.
    fn try_remove_polygon_vertex(&mut self, x: f32, y: f32) -> bool {
        use crate::model::{AnnotationHandle, PolygonHandle};

        let path = self.current_image_path();
        let hit_radius = self.scaled_hit_radius();

        // Find the selected polygon annotation
        let selected_polygon_info = {
            let image_data = self.image_data_store.get(&path);
            image_data
                .annotations
                .iter()
                .enumerate()
                .find(|(_, ann)| ann.selected && ann.shape.is_polygon())
                .map(|(idx, ann)| (idx, ann.id, ann.shape.clone()))
        };

        let Some((ann_idx, ann_id, shape)) = selected_polygon_info else {
            return false;
        };

        // Hit-test against the polygon's handles - only care about vertices
        let Some(handle) = shape.hit_test_handle(x, y, hit_radius) else {
            return false;
        };

        if let AnnotationHandle::Polygon(PolygonHandle::Vertex(vertex_idx)) = handle {
            log::info!(
                "Right-click on vertex {} of polygon {} - attempting removal",
                vertex_idx,
                ann_id
            );

            if let Some(new_shape) = shape.remove_polygon_vertex(vertex_idx) {
                // Push undo point before modifying
                self.push_annotation_undo_point();

                // Apply the change
                let image_data = self.image_data_store.get_or_create(&path);
                if let Some(ann) = image_data.annotations.get_mut(ann_idx) {
                    ann.shape = new_shape;
                    self.auto_save.mark_dirty();
                    log::info!(
                        "Removed vertex {} from polygon {} (now has {} vertices)",
                        vertex_idx,
                        ann_id,
                        ann.shape.polygon_vertices().map(|v| v.len()).unwrap_or(0)
                    );
                    return true;
                }
            }
        }

        false
    }

    /// Find an annotation at the given image coordinates.
    /// Returns the annotation ID if found, None otherwise.
    #[allow(dead_code)] // Kept for future hover/tooltip features
    fn find_annotation_at(&self, x: f32, y: f32) -> Option<u32> {
        let path = self.current_image_path();
        let image_data = self.image_data_store.get(&path);

        // First check if there's a selected annotation that contains the point
        for ann in &image_data.annotations {
            if ann.selected && ann.shape.contains_point(x, y) {
                return Some(ann.id);
            }
        }

        // If no selected annotation at point, check all annotations
        for ann in &image_data.annotations {
            if ann.shape.contains_point(x, y) {
                return Some(ann.id);
            }
        }

        None
    }

    /// Handle bounding box drawing.
    fn handle_bounding_box_draw(&mut self, x: f32, y: f32, kind: hvat_ui::PointerEventKind) {
        use hvat_ui::PointerEventKind;

        let path = self.current_image_path();

        // For DragEnd, check if we'll create an annotation and push undo point first
        if kind == PointerEventKind::DragEnd {
            let image_data = self.image_data_store.get(&path);
            if image_data.drawing_state.to_shape().is_some() {
                // Push undo point before creating annotation
                self.push_annotation_undo_point();
            }
        }

        let image_data = self.image_data_store.get_or_create(&path);

        match kind {
            PointerEventKind::DragStart => {
                // Deselect all annotations when starting to draw
                for ann in &mut image_data.annotations {
                    ann.selected = false;
                }
                // Start new bounding box
                image_data.drawing_state = DrawingState::BoundingBox {
                    start_x: x,
                    start_y: y,
                    current_x: x,
                    current_y: y,
                };
                log::info!("BoundingBox: STARTED at ({:.1}, {:.1})", x, y);
            }
            PointerEventKind::DragMove => {
                // Update current position
                if let DrawingState::BoundingBox {
                    current_x,
                    current_y,
                    ..
                } = &mut image_data.drawing_state
                {
                    *current_x = x;
                    *current_y = y;
                    log::debug!("BoundingBox: MOVE to ({:.1}, {:.1})", x, y);
                } else {
                    log::warn!(
                        "BoundingBox: MOVE but drawing_state is {:?}",
                        image_data.drawing_state
                    );
                }
            }
            PointerEventKind::DragEnd => {
                log::info!(
                    "BoundingBox: END at ({:.1}, {:.1}), state={:?}",
                    x,
                    y,
                    image_data.drawing_state
                );
                // Finish bounding box (undo point already pushed above)
                if let Some(shape) = image_data.drawing_state.to_shape() {
                    let annotation = Annotation::new(
                        image_data.next_annotation_id,
                        shape,
                        self.selected_category,
                    );
                    image_data.next_annotation_id += 1;
                    image_data.annotations.push(annotation);
                    self.auto_save.mark_dirty();
                    log::info!(
                        "Created bounding box annotation (total: {})",
                        image_data.annotations.len()
                    );
                } else {
                    log::warn!("BoundingBox: END but to_shape() returned None");
                }
                image_data.drawing_state = DrawingState::Idle;
            }
            PointerEventKind::Click => {
                // Single click without drag - ignore for bounding box
            }
        }
    }

    /// Handle polygon drawing.
    fn handle_polygon_draw(&mut self, x: f32, y: f32, kind: hvat_ui::PointerEventKind) {
        use hvat_ui::PointerEventKind;

        // Polygon only responds to DragStart (click to add vertex)
        if kind != PointerEventKind::DragStart {
            return;
        }

        let path = self.current_image_path();
        let image_data = self.image_data_store.get_or_create(&path);

        log::debug!(
            "Polygon: click at ({:.1}, {:.1}), state={:?}",
            x,
            y,
            image_data.drawing_state
        );

        // Check if we should close an existing polygon
        // Scale the close threshold by zoom: when zoomed in, require more precision
        let zoom = self.viewer_state.zoom;
        let close_threshold = POLYGON_CLOSE_THRESHOLD / zoom;

        if let DrawingState::Polygon { vertices } = &image_data.drawing_state {
            if vertices.len() >= MIN_POLYGON_VERTICES {
                let (first_x, first_y) = vertices[0];
                let dist = ((x - first_x).powi(2) + (y - first_y).powi(2)).sqrt();
                if dist < close_threshold {
                    self.finalize_polygon();
                    return;
                }
            }
        }

        // Handle adding vertices or starting new polygon
        // Need to re-borrow after potential finalize_polygon call
        let path = self.current_image_path();
        let image_data = self.image_data_store.get_or_create(&path);

        match &mut image_data.drawing_state {
            DrawingState::Idle => {
                // Deselect all annotations when starting to draw
                for ann in &mut image_data.annotations {
                    ann.selected = false;
                }
                image_data.drawing_state = DrawingState::Polygon {
                    vertices: vec![(x, y)],
                };
                log::info!("Polygon: started at ({:.1}, {:.1})", x, y);
            }
            DrawingState::Polygon { vertices } => {
                vertices.push((x, y));
                log::debug!(
                    "Polygon: added vertex {} at ({:.1}, {:.1})",
                    vertices.len(),
                    x,
                    y
                );
            }
            _ => {
                // Wrong drawing state, reset - also deselect
                for ann in &mut image_data.annotations {
                    ann.selected = false;
                }
                image_data.drawing_state = DrawingState::Polygon {
                    vertices: vec![(x, y)],
                };
                log::info!("Polygon: reset and started at ({:.1}, {:.1})", x, y);
            }
        }
    }

    /// Finalize the current polygon drawing and create an annotation.
    fn finalize_polygon(&mut self) {
        let path = self.current_image_path();

        // Check if we can create a polygon and push undo point first
        let should_create = {
            let image_data = self.image_data_store.get(&path);
            if let DrawingState::Polygon { vertices } = &image_data.drawing_state {
                vertices.len() >= MIN_POLYGON_VERTICES
            } else {
                false
            }
        };

        if should_create {
            // Push undo point before creating annotation
            self.push_annotation_undo_point();
        }

        let image_data = self.image_data_store.get_or_create(&path);

        if let DrawingState::Polygon { vertices } = &image_data.drawing_state.clone() {
            if vertices.len() >= MIN_POLYGON_VERTICES {
                let shape = AnnotationShape::Polygon {
                    vertices: vertices.clone(),
                };
                let annotation =
                    Annotation::new(image_data.next_annotation_id, shape, self.selected_category);
                image_data.next_annotation_id += 1;
                log::info!(
                    "Polygon created with {} vertices (total: {})",
                    vertices.len(),
                    image_data.annotations.len() + 1
                );
                image_data.annotations.push(annotation);
                self.auto_save.mark_dirty();
            }
        }
        image_data.drawing_state = DrawingState::Idle;
    }

    /// Create a point annotation.
    fn create_point_annotation(&mut self, x: f32, y: f32) {
        // Push undo point before creating annotation
        self.push_annotation_undo_point();

        let path = self.current_image_path();
        let image_data = self.image_data_store.get_or_create(&path);

        // Deselect all annotations when creating a point
        for ann in &mut image_data.annotations {
            ann.selected = false;
        }

        let shape = AnnotationShape::Point { x, y };
        let annotation =
            Annotation::new(image_data.next_annotation_id, shape, self.selected_category);
        image_data.next_annotation_id += 1;
        image_data.annotations.push(annotation);
        self.auto_save.mark_dirty();
        log::info!("Created point annotation at ({:.1}, {:.1})", x, y);
    }

    // =========================================================================
    // Format System Integration
    // =========================================================================

    /// Convert current app state to ProjectData for export.
    pub fn to_project_data(&self) -> ProjectData {
        let folder = self
            .project
            .as_ref()
            .map(|p| p.folder.clone())
            .unwrap_or_default();

        let image_paths: Vec<PathBuf> = self
            .project
            .as_ref()
            .map(|p| p.images.clone())
            .unwrap_or_default();

        log::info!(
            "to_project_data: folder={:?}, {} images, {} categories, {} tags",
            folder,
            image_paths.len(),
            self.categories.len(),
            self.tags.len()
        );

        let data = ProjectData::from_app_state(
            folder,
            &image_paths,
            &self.categories,
            &self.tags,
            |path| self.image_data_store.get(path),
            |path| self.get_image_dimensions(path),
        );

        log::info!(
            "to_project_data: exported {} images with {} total annotations",
            data.images.len(),
            data.total_annotations()
        );

        data
    }

    /// Get image dimensions for a path (from hyperspectral data if current image).
    fn get_image_dimensions(&self, path: &PathBuf) -> Option<(u32, u32)> {
        // First check stored dimensions in image data (set when image was loaded)
        let image_data = self.image_data_store.get(path);
        if let Some(dims) = image_data.dimensions {
            return Some(dims);
        }

        // Fall back to current image dimensions if this is the current image
        if let Some(ref project) = self.project {
            if project.current_image() == Some(path) {
                if self.image_size.0 > 0 && self.image_size.1 > 0 {
                    return Some(self.image_size);
                }
            }
        }
        None
    }

    /// Extract and store image dimensions from loaded image bytes.
    /// This ensures all images have dimensions available for export.
    ///
    /// Uses `image::ImageReader::into_dimensions()` which reads only the image header
    /// without decoding pixel data - much faster than full decode.
    ///
    /// Used for:
    /// - WASM: Images loaded from browser file picker or drag-drop
    /// - Native: Images extracted from ZIP archives
    fn extract_dimensions_from_loaded_images(&mut self, project: &ProjectState) {
        log::info!(
            "Extracting dimensions from {} loaded images (header-only)",
            project.loaded_images.len()
        );

        for loaded_img in &project.loaded_images {
            let path = PathBuf::from(&loaded_img.name);

            // Use ImageReader to get dimensions from header only (no full decode)
            let cursor = std::io::Cursor::new(&loaded_img.data);
            let result = image::ImageReader::new(cursor)
                .with_guessed_format()
                .map_err(|e| e.to_string())
                .and_then(|reader| reader.into_dimensions().map_err(|e| e.to_string()));

            match result {
                Ok((width, height)) => {
                    let image_data = self.image_data_store.get_or_create(&path);
                    image_data.dimensions = Some((width, height));
                    log::debug!("Extracted dimensions for {:?}: {}x{}", path, width, height);
                }
                Err(e) => {
                    log::warn!("Failed to read dimensions for {:?}: {}", path, e);
                }
            }
        }

        log::info!(
            "Finished extracting dimensions for {} images",
            project.loaded_images.len()
        );
    }

    /// Apply imported ProjectData to app state.
    pub fn apply_project_data(&mut self, data: ProjectData, merge: bool) {
        if !merge {
            // Clear existing data
            self.categories.clear();
            self.tags.clear();
            self.image_data_store = ImageDataStore::new();
        }

        // Apply categories
        for cat_entry in &data.categories {
            let exists = self.categories.iter().any(|c| c.id == cat_entry.id);
            if !exists {
                self.categories.push(cat_entry.to_category());
            }
        }

        // Apply tags
        for tag_entry in &data.tags {
            let exists = self.tags.iter().any(|t| t.id == tag_entry.id);
            if !exists {
                self.tags.push(tag_entry.to_tag());
            }
        }

        // Apply image annotations
        for image_entry in &data.images {
            let image_data = self.image_data_store.get_or_create(&image_entry.path);

            if !merge {
                image_data.annotations.clear();
                image_data.selected_tag_ids.clear();
            }

            for ann_entry in &image_entry.annotations {
                image_data.annotations.push(ann_entry.to_annotation());
            }

            image_data
                .selected_tag_ids
                .extend(image_entry.tag_ids.clone());

            // Update next_annotation_id
            if let Some(max_id) = image_data.annotations.iter().map(|a| a.id).max() {
                image_data.next_annotation_id = max_id + 1;
            }
        }

        log::info!(
            "Applied project data: {} categories, {} tags, {} images, {} annotations",
            data.categories.len(),
            data.tags.len(),
            data.images.len(),
            data.total_annotations()
        );
    }

    /// Perform auto-save of project data.
    #[cfg(not(target_arch = "wasm32"))]
    fn do_auto_save(&mut self) {
        let path = if let Some(ref p) = self.project_file_path {
            p.clone()
        } else if let Some(ref project) = self.project {
            project.folder.join(".hvat_project.json")
        } else {
            log::debug!("Auto-save skipped: no project loaded");
            return;
        };

        log::info!("Auto-saving to {:?}", path);

        let data = self.to_project_data();
        let format = self.format_registry.native();

        match format.export(&data, &path, &ExportOptions::default()) {
            Ok(result) => {
                log::info!(
                    "Auto-save complete: {} images, {} annotations",
                    result.images_exported,
                    result.annotations_exported
                );
                self.auto_save.mark_saved();
                self.project_file_path = Some(path);
            }
            Err(e) => {
                log::error!("Auto-save failed: {:?}", e);
                self.auto_save.mark_save_failed();
            }
        }
    }
}

// ============================================================================
// Application Implementation
// ============================================================================

impl Application for HvatApp {
    type Message = Message;

    fn setup(&mut self, resources: &mut Resources<'_>) {
        // Apply configured log level at startup
        log::set_max_level(self.log_level.to_level_filter());
        log::info!(
            "HVAT setup: log level set to {}, initializing GPU pipeline...",
            self.log_level.name()
        );

        // Create shared pipeline (once, reusable for all images)
        let gpu_ctx = resources.gpu_context();
        self.shared_pipeline = Some(SharedGpuPipeline::new(gpu_ctx));

        // Generate test image
        log::info!("Generating hyperspectral test image...");
        let hyper = generate_test_hyperspectral(
            DEFAULT_TEST_WIDTH,
            DEFAULT_TEST_HEIGHT,
            DEFAULT_TEST_BANDS,
        );
        self.hyperspectral = Some(hyper);
        self.num_bands = DEFAULT_TEST_BANDS;

        self.init_gpu_state(resources);
        self.render_to_texture(resources);

        log::info!("HVAT setup complete - GPU pipeline initialized");
    }

    fn view(&self) -> Element<Self::Message> {
        // Show settings view when settings_open is true
        if self.settings_open {
            return self.build_settings_view();
        }

        // Show export dialog when export_dialog_open is true
        if self.export_dialog_open {
            return self.build_export_dialog();
        }

        // Main application view
        let topbar = self.build_topbar();
        let left_sidebar = self.build_left_sidebar();
        let center_viewer = self.build_image_viewer();
        let right_sidebar = self.build_right_sidebar();

        let main_row = Row::new(vec![left_sidebar, center_viewer, right_sidebar])
            .spacing(0.0)
            .width(Length::Fill(1.0))
            .height(Length::Fill(1.0));

        let mut ctx = hvat_ui::Context::new();
        ctx.add(topbar);
        ctx.add(Element::new(main_row));

        // Add context menu overlay if open
        if self.context_menu_open {
            ctx.add(self.build_context_menu());
        }

        // Add tooltip overlay if visible (rendered last for highest z-order)
        if let Some((content, pos)) = self.tooltip_manager.visible_tooltip() {
            ctx.add(hvat_ui::tooltip_overlay_with_size(
                content.clone(),
                pos,
                self.window_size,
            ));
        }

        // Add confirmation dialog overlay if open (highest z-order)
        if let Some(ref target) = self.confirm_dialog_target {
            ctx.add(self.build_confirm_dialog(target));
        }

        Element::new(Column::new(ctx.take()))
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            // TopBar
            Message::OpenFolder => {
                log::info!("Open folder requested");
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        match ProjectState::from_folder_recursive(folder) {
                            Ok(project) => {
                                // Clear GPU cache when folder changes
                                self.gpu_cache.clear();

                                log::info!(
                                    "Loaded project with {} images from {:?}",
                                    project.images.len(),
                                    project.folder
                                );
                                self.project = Some(project);
                                self.pending_image_load = true;
                            }
                            Err(e) => {
                                log::error!("Failed to load project: {}", e);
                            }
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    // Use webkitdirectory input for folder selection
                    // Works in Firefox, Chrome, and Edge
                    wasm_folder_picker::show_folder_picker_via_input(|loaded_images| {
                        if !loaded_images.is_empty() {
                            log::info!("WASM: loaded {} files from folder", loaded_images.len());
                            if let Ok(mut pending) = pending_picker_state().lock() {
                                *pending = Some(AsyncPickerResult::Files(loaded_images));
                            }
                        } else {
                            log::warn!("No image files found in selected folder");
                        }
                    });
                }
            }
            Message::FolderLoaded(project) => {
                log::info!("Folder loaded with {} images", project.images.len());
                // Clear GPU cache when folder changes
                self.gpu_cache.clear();
                self.project = Some(project);
                self.pending_image_load = true;
            }
            Message::PrevImage => {
                if let Some(ref mut project) = self.project {
                    project.prev();
                    self.pending_image_load = true;
                    log::info!("Previous image: {}", project.current_name());
                }
            }
            Message::NextImage => {
                if let Some(ref mut project) = self.project {
                    project.next();
                    self.pending_image_load = true;
                    log::info!("Next image: {}", project.current_name());
                }
            }
            Message::ToggleSettings => {
                self.settings_open = !self.settings_open;
                log::info!("****** Settings toggled: {} ******", self.settings_open);
            }
            Message::CloseSettings => {
                self.settings_open = false;
                log::info!("Settings closed");
            }
            Message::SettingsScrolled(state) => {
                self.settings_scroll_state = state;
            }
            Message::DependenciesToggled(state) => {
                self.dependencies_collapsed = state;
            }
            Message::LicenseToggled(license, state) => {
                self.license_collapsed.insert(license, state);
            }
            Message::SettingsSectionToggled(state) => {
                self.settings_section_collapsed = state;
            }
            Message::AppearanceSectionToggled(state) => {
                self.appearance_section_collapsed = state;
            }
            Message::KeybindingsSectionToggled(state) => {
                self.keybindings_section_collapsed = state;
            }
            Message::FoldersSectionToggled(state) => {
                self.folders_section_collapsed = state;
            }
            Message::PerformanceSectionToggled(state) => {
                self.performance_section_collapsed = state;
            }
            Message::ThemeChanged(dark) => {
                self.dark_theme = dark;
                log::info!("Theme changed to: {}", if dark { "dark" } else { "light" });
                self.auto_save_config();
            }
            Message::LogLevelChanged(level) => {
                self.log_level = level;
                // Update the global log level
                log::set_max_level(level.to_level_filter());
                log::info!("Log level changed to: {}", level.name());
                self.auto_save_config();
            }
            Message::ExportFolderChanged(text, state) => {
                // Only save when focus is lost (not on every keystroke)
                let was_focused = self.export_folder_state.is_focused;
                let now_focused = state.is_focused;
                self.export_folder = text;
                self.export_folder_state = state;
                if was_focused && !now_focused {
                    self.auto_save_config();
                }
            }
            Message::ImportFolderChanged(text, state) => {
                // Only save when focus is lost (not on every keystroke)
                let was_focused = self.import_folder_state.is_focused;
                let now_focused = state.is_focused;
                self.import_folder = text;
                self.import_folder_state = state;
                if was_focused && !now_focused {
                    self.auto_save_config();
                }
            }

            // Image Viewer
            Message::ViewerChanged(state) => {
                self.viewer_state = state;
            }

            // Left Sidebar - Tools
            Message::ToolsToggled(state) => {
                self.tools_collapsed = state;
            }
            Message::ToolSelected(tool) => {
                // Deselect all annotations when switching tools
                let path = self.current_image_path();
                let image_data = self.image_data_store.get_or_create(&path);
                for ann in &mut image_data.annotations {
                    ann.selected = false;
                }
                self.selected_tool = tool;
                log::info!("Tool selected: {:?}", tool);
            }

            // Left Sidebar - Categories
            Message::CategoriesToggled(state) => {
                self.categories_collapsed = state;
            }
            Message::CategorySelected(id) => {
                // Check if there are selected annotations - if so, change their category
                // but do NOT change the default category
                let path = self.current_image_path();
                let image_data = self.image_data_store.get_or_create(&path);
                let has_selected = image_data.annotations.iter().any(|a| a.selected);
                if has_selected {
                    // Push undo point before changing category
                    self.push_annotation_undo_point();
                    let image_data = self.image_data_store.get_or_create(&path);
                    let mut changed_count = 0;
                    for annotation in &mut image_data.annotations {
                        if annotation.selected {
                            annotation.category_id = id;
                            changed_count += 1;
                        }
                    }
                    self.auto_save.mark_dirty();
                    log::info!(
                        "Changed category of {} annotation(s) to {} (default unchanged)",
                        changed_count,
                        id
                    );
                } else {
                    // No annotation selected - change the default category for new annotations
                    self.selected_category = id;
                    log::info!("Default category changed to: {}", id);
                }
            }
            Message::CategoryInputChanged(text, state) => {
                self.category_input_text = text;
                self.category_input_state = state;
            }
            Message::AddCategory => {
                if !self.category_input_text.is_empty() {
                    let new_id = self.categories.iter().map(|c| c.id).max().unwrap_or(0) + 1;
                    self.categories.push(Category::new(
                        new_id,
                        &self.category_input_text,
                        [200, 200, 100],
                    ));
                    self.auto_save.mark_dirty();
                    log::info!(
                        "Added new category '{}': {}",
                        self.category_input_text,
                        new_id
                    );
                    self.auto_save_config();
                    self.category_input_text.clear();
                    self.category_input_state.cursor = 0;
                }
            }
            Message::StartEditingCategory(id) => {
                if let Some(cat) = self.categories.iter().find(|c| c.id == id) {
                    self.editing_category = Some(id);
                    self.category_name_input = cat.name.clone();
                    self.category_name_input_state = TextInputState::default();
                    self.category_name_input_state.is_focused = true;
                    log::info!("Started editing category: {}", id);
                }
            }
            Message::CategoryNameChanged(text, state) => {
                // If focus was lost (clicked elsewhere), cancel editing
                if self.category_name_input_state.is_focused && !state.is_focused {
                    self.editing_category = None;
                    self.category_name_input.clear();
                    log::info!("Category editing cancelled (focus lost)");
                } else {
                    self.category_name_input = text;
                    self.category_name_input_state = state;
                }
            }
            Message::FinishEditingCategory => {
                if let Some(id) = self.editing_category {
                    if !self.category_name_input.is_empty() {
                        if let Some(cat) = self.categories.iter_mut().find(|c| c.id == id) {
                            cat.name = self.category_name_input.clone();
                            self.auto_save.mark_dirty();
                            log::info!("Renamed category {} to '{}'", id, cat.name);
                            self.auto_save_config();
                        }
                    }
                }
                self.editing_category = None;
                self.category_name_input.clear();
            }
            Message::CancelEditingCategory => {
                self.editing_category = None;
                self.category_name_input.clear();
                log::info!("Cancelled category editing");
            }
            Message::ToggleCategoryColorPicker(id) => {
                // Toggle: if already open for this category, close it; otherwise open for this category
                if self.color_picker_category == Some(id) {
                    self.color_picker_category = None;
                    log::info!("Closed color picker for category: {}", id);
                } else {
                    // Close any other open picker (tag or different category)
                    self.color_picker_tag = None;
                    self.color_picker_category = Some(id);
                    log::info!("Opened color picker for category: {}", id);
                }
            }
            Message::CloseCategoryColorPicker => {
                // Save config when color picker is closed (captures final color from live updates)
                if self.color_picker_category.is_some() {
                    self.auto_save_config();
                }
                self.color_picker_category = None;
                self.color_picker_state = ColorPickerState::default();
                log::info!("Closed color picker");
            }
            Message::CategoryColorLiveUpdate(color) => {
                // Update color but don't close picker (live preview while dragging sliders)
                if let Some(id) = self.color_picker_category {
                    if let Some(cat) = self.categories.iter_mut().find(|c| c.id == id) {
                        cat.color = color;
                    }
                }
            }
            Message::CategoryColorApply(color) => {
                // Apply color and close picker (from palette selection)
                if let Some(id) = self.color_picker_category {
                    if let Some(cat) = self.categories.iter_mut().find(|c| c.id == id) {
                        cat.color = color;
                        self.auto_save.mark_dirty();
                        log::info!("Applied color for category {}: {:?}", id, color);
                        self.auto_save_config();
                    }
                }
                self.color_picker_category = None;
                self.color_picker_state = ColorPickerState::default();
            }
            Message::ColorPickerStateChanged(state) => {
                log::debug!("Color picker state changed: {:?}", state);
                self.color_picker_state = state;
            }
            Message::DeleteCategory(id) => {
                // Find category and open confirmation dialog
                if let Some(cat) = self.categories.iter().find(|c| c.id == id) {
                    let cat_name = cat.name.clone();

                    // Count annotations using this category
                    let annotation_count: usize = self
                        .image_data_store
                        .iter()
                        .map(|(_, data)| {
                            data.annotations
                                .iter()
                                .filter(|a| a.category_id == id)
                                .count()
                        })
                        .sum();

                    // Open the in-app confirmation dialog
                    self.confirm_dialog_target = Some(ConfirmTarget::Category(
                        id,
                        cat_name.clone(),
                        annotation_count,
                    ));
                    log::debug!(
                        "Opening confirmation dialog for category '{}' (id={}, {} annotations)",
                        cat_name,
                        id,
                        annotation_count
                    );
                }
            }
            Message::ConfirmDeleteCategory(id) => {
                // Direct deletion without confirmation (used internally)
                if let Some(cat) = self.categories.iter().find(|c| c.id == id) {
                    let cat_name = cat.name.clone();
                    self.delete_category_internal(id, &cat_name);
                }
            }

            // Left Sidebar - Tags (global registry with per-image selection)
            Message::TagsToggled(state) => {
                self.tags_collapsed = state;
            }
            Message::TagSelected(id) => {
                self.selected_tag = id;
                log::info!("Selected tag ID {}", id);
            }
            Message::TagInputChanged(text, state) => {
                self.tag_input_text = text;
                self.tag_input_state = state;
            }
            Message::AddTag => {
                if !self.tag_input_text.is_empty() {
                    let name = self.tag_input_text.trim().to_string();
                    // Check if tag with same name already exists
                    let exists = self.tags.iter().any(|t| t.name == name);
                    if !exists {
                        // Generate new ID
                        let new_id = self.tags.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                        // Use a default color (blue-gray)
                        let new_tag = Tag::new(new_id, &name, [100, 140, 180]);
                        self.tags.push(new_tag);
                        self.selected_tag = new_id;
                        // Also apply it to the current image
                        let path = self.current_image_path();
                        let image_data = self.image_data_store.get_or_create(&path);
                        image_data.selected_tag_ids.insert(new_id);
                        self.auto_save.mark_dirty();
                        log::info!("Added tag '{}' with ID {}", name, new_id);
                    }
                    self.tag_input_text.clear();
                    self.tag_input_state.cursor = 0;
                }
            }
            Message::ToggleImageTag(tag_id) => {
                let path = self.current_image_path();
                let image_data = self.image_data_store.get_or_create(&path);
                if image_data.selected_tag_ids.contains(&tag_id) {
                    image_data.selected_tag_ids.remove(&tag_id);
                    log::info!("Removed tag ID {} from image {:?}", tag_id, path);
                } else {
                    image_data.selected_tag_ids.insert(tag_id);
                    log::info!("Added tag ID {} to image {:?}", tag_id, path);
                }
                self.auto_save.mark_dirty();
            }
            Message::DeleteTag(tag_id) => {
                // Find tag and open confirmation dialog
                if let Some(tag) = self.tags.iter().find(|t| t.id == tag_id) {
                    let tag_name = tag.name.clone();

                    // Open the in-app confirmation dialog
                    self.confirm_dialog_target = Some(ConfirmTarget::Tag(tag_id, tag_name.clone()));
                    log::debug!(
                        "Opening confirmation dialog for tag '{}' (id={})",
                        tag_name,
                        tag_id
                    );
                }
            }
            Message::StartEditingTag(tag_id) => {
                if let Some(tag) = self.tags.iter().find(|t| t.id == tag_id) {
                    self.editing_tag = Some(tag_id);
                    self.tag_name_input = tag.name.clone();
                    self.tag_name_input_state = TextInputState::default();
                    self.tag_name_input_state.is_focused = true;
                    log::info!("Started editing tag ID {}", tag_id);
                }
            }
            Message::TagNameChanged(text, state) => {
                // If focus was lost (clicked elsewhere), cancel editing
                if self.tag_name_input_state.is_focused && !state.is_focused {
                    self.editing_tag = None;
                    self.tag_name_input.clear();
                    log::info!("Tag editing cancelled (focus lost)");
                } else {
                    self.tag_name_input = text;
                    self.tag_name_input_state = state;
                }
            }
            Message::FinishEditingTag => {
                if let Some(tag_id) = self.editing_tag.take() {
                    let new_name = self.tag_name_input.trim().to_string();
                    if !new_name.is_empty() {
                        if let Some(tag) = self.tags.iter_mut().find(|t| t.id == tag_id) {
                            let old_name = tag.name.clone();
                            if new_name != old_name {
                                tag.name = new_name.clone();
                                self.auto_save.mark_dirty();
                                log::info!(
                                    "Renamed tag ID {} from '{}' to '{}'",
                                    tag_id,
                                    old_name,
                                    new_name
                                );
                            }
                        }
                    }
                }
                self.tag_name_input.clear();
                self.tag_name_input_state = TextInputState::default();
            }
            Message::CancelEditingTag => {
                self.editing_tag = None;
                self.tag_name_input.clear();
                self.tag_name_input_state = TextInputState::default();
                log::info!("Cancelled tag editing");
            }
            Message::ToggleTagColorPicker(tag_id) => {
                if self.color_picker_tag == Some(tag_id) {
                    self.color_picker_tag = None;
                } else {
                    self.color_picker_tag = Some(tag_id);
                    // Close category color picker if open
                    self.color_picker_category = None;
                }
            }
            Message::CloseTagColorPicker => {
                self.color_picker_tag = None;
                self.color_picker_state = ColorPickerState::default();
            }
            Message::TagColorLiveUpdate(color) => {
                // Live update the tag color while slider is being dragged
                if let Some(tag_id) = self.color_picker_tag {
                    if let Some(tag) = self.tags.iter_mut().find(|t| t.id == tag_id) {
                        tag.color = color;
                    }
                }
            }
            Message::TagColorApply(color) => {
                // Apply color and close picker
                if let Some(tag_id) = self.color_picker_tag {
                    if let Some(tag) = self.tags.iter_mut().find(|t| t.id == tag_id) {
                        tag.color = color;
                        self.auto_save.mark_dirty();
                        log::info!("Applied color {:?} to tag ID {}", color, tag_id);
                    }
                    self.color_picker_tag = None;
                    self.color_picker_state = ColorPickerState::default();
                }
            }

            // Left Sidebar Scroll
            Message::LeftScrolled(state) => {
                self.left_scroll_state = state;
            }

            // Right Sidebar - Band Selection
            Message::BandSelectionToggled(state) => {
                self.band_selection_collapsed = state;
            }
            Message::RedBandChanged(state) => {
                self.red_band_slider = state;
                self.band_selection.0 = self.red_band_slider.value as usize;
                self.needs_gpu_render = true;
                log::debug!("Red band: {}", self.band_selection.0);
            }
            Message::GreenBandChanged(state) => {
                self.green_band_slider = state;
                self.band_selection.1 = self.green_band_slider.value as usize;
                self.needs_gpu_render = true;
                log::debug!("Green band: {}", self.band_selection.1);
            }
            Message::BlueBandChanged(state) => {
                self.blue_band_slider = state;
                self.band_selection.2 = self.blue_band_slider.value as usize;
                self.needs_gpu_render = true;
                log::debug!("Blue band: {}", self.band_selection.2);
            }

            // Right Sidebar - Adjustments
            Message::AdjustmentsToggled(state) => {
                self.adjustments_collapsed = state;
            }
            Message::BrightnessChanged(state) => {
                self.brightness_slider = state;
                self.needs_gpu_render = true;
            }
            Message::ContrastChanged(state) => {
                self.contrast_slider = state;
                self.needs_gpu_render = true;
            }
            Message::GammaChanged(state) => {
                self.gamma_slider = state;
                self.needs_gpu_render = true;
            }
            Message::HueChanged(state) => {
                self.hue_slider = state;
                self.needs_gpu_render = true;
            }
            Message::ResetAdjustments => {
                self.reset_adjustment_sliders();
                self.needs_gpu_render = true;
                log::info!("Adjustments reset");
            }

            // Right Sidebar - File Explorer
            Message::FileExplorerToggled(state) => {
                self.file_explorer_collapsed = state;
            }
            Message::FileExplorerScrolled(state) => {
                self.file_explorer_scroll_state = state;
            }
            Message::FileExplorerFolderToggle(folder_path) => {
                log::debug!("File explorer: toggling folder '{}'", folder_path);
                self.file_explorer_state.toggle(&folder_path);
            }
            Message::FileExplorerStateChanged(state) => {
                self.file_explorer_state = state;
            }
            Message::FileExplorerSelect(file_path) => {
                // Find the index of the selected file by matching the path
                if let Some(ref mut project) = self.project {
                    // Try to find the file by its relative path
                    for (index, image_path) in project.images.iter().enumerate() {
                        let relative = if !project.folder.as_os_str().is_empty() {
                            image_path
                                .strip_prefix(&project.folder)
                                .ok()
                                .and_then(|p| p.to_str())
                                .map(String::from)
                        } else {
                            image_path.to_str().map(String::from)
                        };

                        if relative.as_ref() == Some(&file_path) {
                            project.current_index = index;
                            self.pending_image_load = true;
                            log::info!("File explorer: selected '{}' (index {})", file_path, index);
                            break;
                        }
                    }
                }
            }

            // Right Sidebar - Thumbnails
            Message::ThumbnailsToggled(state) => {
                self.thumbnails_collapsed = state;
            }
            Message::ThumbnailsScrolled(state) => {
                self.thumbnails_scroll_state = state;
            }
            Message::ThumbnailSelect(index) => {
                if let Some(ref mut project) = self.project {
                    if index < project.images.len() {
                        project.current_index = index;
                        self.pending_image_load = true;
                        log::info!("Thumbnail: selected image {}", index);
                    }
                }
            }

            // Right Sidebar - Annotations Panel
            Message::AnnotationsToggled(state) => {
                self.annotations_collapsed = state;
            }
            Message::AnnotationsScrolled(state) => {
                self.annotations_scroll_state = state;
            }
            Message::ToggleCategoryFilter(category_id) => {
                if self.hidden_categories.contains(&category_id) {
                    self.hidden_categories.remove(&category_id);
                    log::info!("Category {} is now visible", category_id);
                } else {
                    self.hidden_categories.insert(category_id);
                    log::info!("Category {} is now hidden", category_id);
                }
            }
            Message::SelectAnnotation(annotation_id) => {
                let path = self.current_image_path();
                let image_data = self.image_data_store.get_or_create(&path);
                // Toggle selection: if already selected, deselect; otherwise select exclusively
                let was_selected = image_data
                    .annotations
                    .iter()
                    .find(|a| a.id == annotation_id)
                    .map(|a| a.selected)
                    .unwrap_or(false);

                // Deselect all first
                for ann in &mut image_data.annotations {
                    ann.selected = false;
                }

                // Then select the clicked one (unless it was already selected)
                if !was_selected {
                    if let Some(ann) = image_data
                        .annotations
                        .iter_mut()
                        .find(|a| a.id == annotation_id)
                    {
                        ann.selected = true;
                        log::info!("Selected annotation #{}", annotation_id);
                    }
                } else {
                    log::info!("Deselected annotation #{}", annotation_id);
                }
            }

            // Right Sidebar Scroll
            Message::RightScrolled(state) => {
                self.right_scroll_state = state;
            }

            // Global Undo/Redo
            // Priority: in-progress polygon undo (remove last vertex), then annotation undo, then slider/adjustment undo
            Message::Undo => {
                // First check if we're drawing a polygon - undo should remove the last vertex
                let path = self.current_image_path();
                let image_data = self.image_data_store.get_or_create(&path);
                if let DrawingState::Polygon { vertices } = &mut image_data.drawing_state {
                    if vertices.len() > 1 {
                        let removed = vertices.pop();
                        log::info!(
                            "Polygon: undo removed vertex at {:?}, {} vertices remaining",
                            removed,
                            vertices.len()
                        );
                        return;
                    } else if vertices.len() == 1 {
                        // Last vertex - cancel the polygon entirely
                        image_data.drawing_state = DrawingState::Idle;
                        log::info!("Polygon: undo cancelled polygon (removed last vertex)");
                        return;
                    }
                }

                // Unified undo - snapshots may contain slider state, annotation state, or both
                let current = self.snapshot_with_annotations();
                let prev = self.undo_stack.borrow_mut().undo(current);
                if let Some(prev) = prev {
                    self.restore(&prev);
                    log::info!("Undo performed");
                }
            }
            Message::Redo => {
                // Unified redo - snapshots may contain slider state, annotation state, or both
                let current = self.snapshot_with_annotations();
                let next = self.undo_stack.borrow_mut().redo(current);
                if let Some(next) = next {
                    self.restore(&next);
                    log::info!("Redo performed");
                }
            }

            // Annotation Drawing
            Message::ImagePointer(event) => {
                self.handle_pointer_event(event);
            }
            Message::CancelAnnotation => {
                let path = self.current_image_path();
                let image_data = self.image_data_store.get_or_create(&path);
                if image_data.drawing_state.is_drawing() {
                    image_data.drawing_state = DrawingState::Idle;
                    log::info!("Annotation cancelled");
                }
            }
            Message::DeleteAnnotation => {
                // Remove selected annotations
                let path = self.current_image_path();
                let image_data = self.image_data_store.get_or_create(&path);
                // Check if there are any selected annotations to delete
                let has_selected = image_data.annotations.iter().any(|a| a.selected);
                if has_selected {
                    // Push undo point before deleting
                    self.push_annotation_undo_point();
                    let image_data = self.image_data_store.get_or_create(&path);
                    let before_count = image_data.annotations.len();
                    image_data.annotations.retain(|a| !a.selected);
                    let deleted = before_count - image_data.annotations.len();
                    self.auto_save.mark_dirty();
                    log::info!("Deleted {} annotation(s)", deleted);
                }
            }
            Message::FinishPolygon => {
                self.finalize_polygon();
            }
            Message::ChangeSelectedAnnotationCategory(category_id) => {
                let path = self.current_image_path();
                let image_data = self.image_data_store.get_or_create(&path);
                // Check if there are any selected annotations
                let has_selected = image_data.annotations.iter().any(|a| a.selected);
                if has_selected {
                    // Push undo point before changing category
                    self.push_annotation_undo_point();
                    let image_data = self.image_data_store.get_or_create(&path);
                    let mut changed_count = 0;
                    for annotation in &mut image_data.annotations {
                        if annotation.selected {
                            annotation.category_id = category_id;
                            changed_count += 1;
                        }
                    }
                    self.auto_save.mark_dirty();
                    // Note: We do NOT change selected_category here - that's the default for new annotations
                    log::info!(
                        "Changed category of {} annotation(s) to {} (default unchanged)",
                        changed_count,
                        category_id
                    );
                }
            }

            // Settings - GPU Preloading
            Message::GpuPreloadCountChanged(state) => {
                // Only auto-save when drag ends to avoid excessive writes
                let was_dragging = self.gpu_preload_slider.drag.is_dragging();
                let now_dragging = state.drag.is_dragging();

                self.gpu_preload_slider = state;
                let count = (self.gpu_preload_slider.value as usize).min(MAX_GPU_PRELOAD_COUNT);
                self.gpu_preload_count = count;
                self.gpu_cache.set_preload_count(count);
                log::info!("GPU preload count changed to: {}", count);

                // Trigger preloading with new count if we have a project
                if count > 0 && self.project.is_some() {
                    self.pending_preload = true;
                }

                // Auto-save when drag ends (not during drag to avoid excessive writes)
                if was_dragging && !now_dragging {
                    self.auto_save_config();
                }
            }

            // Import/Export
            Message::ShowExportDialog => {
                self.export_dialog_open = true;
                log::info!("Export dialog opened");
            }
            Message::CloseExportDialog => {
                self.export_dialog_open = false;
                log::info!("Export dialog closed");
            }
            Message::ExportAnnotations(format_id) => {
                log::info!("Export requested in format: {}", format_id);
                self.export_dialog_open = false;

                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some(format) = self.format_registry.get(&format_id) {
                        // Determine file extension for picker
                        let ext = format.extensions().first().copied().unwrap_or("json");
                        let default_name = format!("annotations.{}", ext);

                        // Use per-image export for YOLO/VOC (pick folder), single file for others
                        if format.supports_per_image() {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                let data = self.to_project_data();
                                match format.export(&data, &folder, &ExportOptions::default()) {
                                    Ok(result) => {
                                        log::info!(
                                            "Exported {} images with {} annotations to {:?}",
                                            result.images_exported,
                                            result.annotations_exported,
                                            folder
                                        );
                                        for warning in &result.warnings {
                                            log::warn!("Export warning: {}", warning.message);
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Export failed: {:?}", e);
                                    }
                                }
                            }
                        } else {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name(&default_name)
                                .save_file()
                            {
                                let data = self.to_project_data();
                                match format.export(&data, &path, &ExportOptions::default()) {
                                    Ok(result) => {
                                        log::info!(
                                            "Exported {} images with {} annotations to {:?}",
                                            result.images_exported,
                                            result.annotations_exported,
                                            path
                                        );
                                    }
                                    Err(e) => {
                                        log::error!("Export failed: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(format) = self.format_registry.get(&format_id) {
                        let data = self.to_project_data();

                        match format.export_to_bytes(&data, &ExportOptions::default()) {
                            Ok((bytes, result)) => {
                                // Per-image formats export as ZIP, single-file as their native format
                                let (filename, mime_type) = if format.supports_per_image() {
                                    (
                                        format!("annotations-{}.zip", format.id()),
                                        "application/zip",
                                    )
                                } else {
                                    let ext =
                                        format.extensions().first().copied().unwrap_or("json");
                                    (format!("annotations.{}", ext), "application/json")
                                };

                                log::info!(
                                    "Exporting {} images with {} annotations as download ({})",
                                    result.images_exported,
                                    result.annotations_exported,
                                    filename
                                );
                                for warning in &result.warnings {
                                    log::warn!("Export warning: {}", warning.message);
                                }
                                self.download_file_wasm(&filename, mime_type, &bytes);
                            }
                            Err(e) => {
                                log::error!("Export failed: {:?}", e);
                            }
                        }
                    }
                }
            }
            Message::ImportAnnotations => {
                log::info!("Import requested");
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Annotations", &["json", "hvat", "hvat.json"])
                        .pick_file()
                    {
                        // Try to detect format from extension
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        let format =
                            if ext == "hvat" || path.to_string_lossy().ends_with(".hvat.json") {
                                self.format_registry.get("hvat")
                            } else {
                                // Try COCO for generic JSON
                                self.format_registry.get("coco")
                            };

                        if let Some(format) = format {
                            match format.import(&path, &crate::format::ImportOptions::default()) {
                                Ok(data) => {
                                    let images = data.images.len();
                                    let annotations = data.total_annotations();
                                    self.apply_project_data(data, false);
                                    log::info!(
                                        "Imported {} images with {} annotations from {:?}",
                                        images,
                                        annotations,
                                        path
                                    );
                                }
                                Err(e) => {
                                    log::error!("Import failed: {:?}", e);
                                }
                            }
                        }
                    }
                }
            }
            Message::ExportCompleted(images, annotations) => {
                log::info!(
                    "Export completed: {} images, {} annotations",
                    images,
                    annotations
                );
            }
            Message::ExportFailed(error) => {
                log::error!("Export failed: {}", error);
            }
            Message::ImportCompleted(images, annotations) => {
                log::info!(
                    "Import completed: {} images, {} annotations",
                    images,
                    annotations
                );
            }
            Message::ImportFailed(error) => {
                log::error!("Import failed: {}", error);
            }
            Message::AutoSave => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.do_auto_save();
                }
            }
            Message::AutoSaveCompleted => {
                log::debug!("Auto-save completed");
            }
            Message::AutoSaveFailed(error) => {
                log::error!("Auto-save failed: {}", error);
            }

            // Drag-Drop Events
            #[allow(unused_variables)]
            Message::FilesDropped(paths) => {
                log::info!("Files dropped: {} paths", paths.len());

                #[cfg(target_arch = "wasm32")]
                {
                    // Check if we have pending WASM files (this means we're on WASM)
                    if !self.pending_wasm_files.is_empty() {
                        // Use the pending file data directly (WASM can't read from disk)
                        let pending_files = std::mem::take(&mut self.pending_wasm_files);
                        log::info!("Processing {} dropped files from WASM", pending_files.len());

                        // Check if any dropped file is a ZIP archive
                        let zip_files: Vec<&LoadedImage> = pending_files
                            .iter()
                            .filter(|f| is_zip_file(&f.name))
                            .collect();

                        let loaded_images = if !zip_files.is_empty() {
                            // Handle ZIP files - extract images from the first ZIP
                            let zip_file = zip_files[0];
                            log::info!("Processing ZIP file: {}", zip_file.name);

                            match extract_images_from_zip_bytes(&zip_file.data, &zip_file.name) {
                                Ok(extracted) => {
                                    log::info!(
                                        "Extracted {} images from ZIP archive",
                                        extracted.len()
                                    );
                                    extracted
                                }
                                Err(e) => {
                                    log::error!("Failed to extract ZIP archive: {}", e);
                                    return;
                                }
                            }
                        } else {
                            // No ZIP files - use files directly
                            pending_files
                        };

                        match ProjectState::from_loaded_images(loaded_images) {
                            Ok(project) => {
                                self.gpu_cache.clear();
                                log::info!(
                                    "Loaded project with {} images from drop",
                                    project.images.len()
                                );
                                // Extract dimensions from all loaded images for export
                                self.extract_dimensions_from_loaded_images(&project);
                                self.project = Some(project);
                                self.pending_image_load = true;
                            }
                            Err(e) => {
                                log::error!("Failed to load dropped images: {}", e);
                            }
                        }
                    } else {
                        log::warn!("FilesDropped received but no pending WASM files");
                    }
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    // Native: read files from filesystem
                    self.handle_native_file_drop(paths);
                }
            }
            Message::FileDataDropped(_loaded_images) => {
                // This is handled in on_event by storing to pending_wasm_files
                // The FilesDropped message will then process them
                log::debug!("FileDataDropped - handled via pending_wasm_files");
            }
            Message::FileHoverStarted => {
                self.drag_hover_active = true;
                log::debug!("Drag hover started");
            }
            Message::FileHoverEnded => {
                self.drag_hover_active = false;
                log::debug!("Drag hover ended");
            }

            // Keybinding configuration
            Message::StartCapturingKeybind(target) => {
                self.capturing_keybind = Some(target);
                log::info!("Started capturing keybind for {:?}", target);
            }
            Message::CancelCapturingKeybind => {
                self.capturing_keybind = None;
                log::info!("Cancelled keybind capture");
            }
            Message::KeyCaptured(key) => {
                if let Some(target) = self.capturing_keybind.take() {
                    match target {
                        KeybindTarget::Tool(tool) => {
                            self.keybindings.set_tool_key(tool, key);
                            log::info!("Set {:?} tool hotkey to {:?}", tool, key);
                        }
                        KeybindTarget::Category(index) => {
                            self.keybindings.set_category_key(index, Some(key));
                            log::info!("Set category {} hotkey to {:?}", index + 1, key);
                        }
                    }
                    self.auto_save_config();
                }
            }
            Message::ResetKeybindings => {
                self.keybindings = KeyBindings::default();
                self.capturing_keybind = None;
                log::info!("Reset keybindings to defaults");
                self.auto_save_config();
            }

            // Configuration Export/Import
            Message::ExportConfig => {
                log::info!("Exporting configuration...");
                self.export_config();
            }
            Message::ImportConfig => {
                log::info!("Importing configuration...");
                self.import_config();
            }
            Message::ConfigLoaded(json) => {
                log::info!("Configuration data loaded, parsing...");
                self.apply_config_from_json(&json);
            }
            Message::ConfigImportCompleted => {
                log::info!("Configuration import completed successfully");
            }
            Message::ConfigImportFailed(error) => {
                log::error!("Configuration import failed: {}", error);
            }

            // Tooltip Events
            Message::TooltipRequest(id, content, bounds, mouse_pos) => {
                let request = hvat_ui::TooltipRequest::new(id, content, bounds);
                self.tooltip_manager.request(request, mouse_pos);
            }
            Message::TooltipClear(id) => {
                self.tooltip_manager.clear_if_id(&id);
            }
            Message::TooltipMouseMove(pos) => {
                // Check if mouse is still within trigger bounds
                if let Some(bounds) = self.tooltip_manager.trigger_bounds() {
                    if !bounds.contains(pos.0, pos.1) {
                        // Mouse left the trigger area - clear tooltip
                        self.tooltip_manager.clear();
                    } else if let Some(current_id) = self.tooltip_manager.current_id() {
                        // Still within bounds - update position if visible
                        if let Some((content, _)) = self.tooltip_manager.visible_tooltip() {
                            let content_clone = content.clone();
                            let id_clone = current_id.to_string();
                            let request =
                                hvat_ui::TooltipRequest::new(id_clone, content_clone, bounds);
                            self.tooltip_manager.request(request, pos);
                        }
                    }
                }
            }
            Message::TooltipBecameVisible => {
                // Tooltip became visible via idle timer - just trigger rebuild
                log::debug!("Tooltip became visible via idle timer");
            }

            // Context Menu Events
            Message::OpenContextMenu(position, annotation_id) => {
                log::debug!(
                    "Opening context menu at {:?}, annotation_id={:?}",
                    position,
                    annotation_id
                );
                self.context_menu_open = true;
                self.context_menu_position = position;
                self.context_menu_annotation_id = annotation_id;
            }
            Message::CloseContextMenu => {
                log::debug!("Closing context menu");
                self.context_menu_open = false;
                self.context_menu_annotation_id = None;
            }
            Message::ContextMenuSelect(item_id) => {
                log::debug!("Context menu item selected: {}", item_id);
                self.context_menu_open = false;

                // Handle category selection (item_id is "category_{id}")
                if let Some(category_id_str) = item_id.strip_prefix("category_") {
                    if let Ok(category_id) = category_id_str.parse::<u32>() {
                        // If we have an annotation selected, change its category
                        if let Some(ann_id) = self.context_menu_annotation_id {
                            let path = self.current_image_path();

                            // Push undo point before modifying
                            self.push_annotation_undo_point();

                            // Find and update the annotation
                            if let Some(image_data) = self.image_data_store.get_mut(&path) {
                                for annotation in &mut image_data.annotations {
                                    if annotation.id == ann_id {
                                        annotation.category_id = category_id;
                                        log::info!(
                                            "Changed annotation {} category to {}",
                                            ann_id,
                                            category_id
                                        );
                                        break;
                                    }
                                }
                            }
                        } else {
                            // No annotation - just select this category for new annotations
                            self.selected_category = category_id;
                            log::info!("Selected category {} for new annotations", category_id);
                        }
                    }
                }

                self.context_menu_annotation_id = None;
            }

            // Confirmation Dialog Events
            Message::ConfirmDialogConfirm => {
                if let Some(target) = self.confirm_dialog_target.take() {
                    match target {
                        ConfirmTarget::Category(id, name, _) => {
                            log::info!("Confirmed deletion of category '{}' (ID {})", name, id);
                            self.delete_category_internal(id, &name);
                        }
                        ConfirmTarget::Tag(id, name) => {
                            log::info!("Confirmed deletion of tag '{}' (ID {})", name, id);
                            self.delete_tag_internal(id, &name);
                        }
                    }
                }
            }
            Message::ConfirmDialogCancel => {
                if let Some(target) = self.confirm_dialog_target.take() {
                    match target {
                        ConfirmTarget::Category(_, name, _) => {
                            log::info!("Cancelled deletion of category '{}'", name);
                        }
                        ConfirmTarget::Tag(_, name) => {
                            log::info!("Cancelled deletion of tag '{}'", name);
                        }
                    }
                }
            }
        }
    }

    fn tick_with_resources(&mut self, resources: &mut Resources<'_>) -> TickResult {
        let mut needs_rebuild = false;

        // Check for pending tooltip FIRST - we need to return RequestIdleTimer
        // even if preloading or other work is happening. The framework will set
        // the timer and continue processing other work.
        let tooltip_pending =
            self.tooltip_manager.current_id().is_some() && !self.tooltip_manager.is_visible();

        // Check for async picker result (WASM only)
        #[cfg(target_arch = "wasm32")]
        if let Ok(mut pending) = pending_picker_state().lock() {
            if let Some(result) = pending.take() {
                match result {
                    AsyncPickerResult::Files(loaded_images) => {
                        log::info!("Processing {} files from async picker", loaded_images.len());
                        match ProjectState::from_loaded_images(loaded_images) {
                            Ok(project) => {
                                // Clear GPU cache when folder changes
                                self.gpu_cache.clear();
                                log::info!("Loaded project with {} images", project.images.len());
                                // Extract dimensions from all loaded images for export
                                self.extract_dimensions_from_loaded_images(&project);
                                self.project = Some(project);
                                self.pending_image_load = true;
                            }
                            Err(e) => {
                                log::error!("Failed to load project: {}", e);
                            }
                        }
                    }
                }
            }
        }

        // Check for async config import result (WASM)
        if let Ok(mut pending) = pending_config_state().lock() {
            if let Some(json) = pending.take() {
                log::info!("Processing config from async import");
                self.apply_config_from_json(&json);
                needs_rebuild = true;
            }
        }

        // Load new image if pending
        if self.pending_image_load {
            self.pending_image_load = false;
            if let Some(ref project) = self.project {
                if let Some(path) = project.current_image() {
                    self.load_image_file(path.clone(), resources);
                    needs_rebuild = true;
                }
            }
            // Don't preload in the same tick - let the frame render first
            // Return appropriate result based on what work was done
            // Note: Tooltip timer takes priority if pending
            return if tooltip_pending {
                TickResult::RequestIdleTimer(self.tooltip_manager.delay())
            } else if needs_rebuild {
                TickResult::NeedsRebuild
            } else if self.pending_preload {
                TickResult::ContinueWork
            } else {
                TickResult::Idle
            };
        }

        // Re-render to texture if band selection or adjustments changed
        if self.needs_gpu_render {
            self.render_to_texture(resources);
            self.needs_gpu_render = false;
            needs_rebuild = true;
            // Don't preload in the same tick as a render
            // Note: Tooltip timer takes priority if pending
            return if tooltip_pending {
                TickResult::RequestIdleTimer(self.tooltip_manager.delay())
            } else if needs_rebuild {
                TickResult::NeedsRebuild
            } else {
                TickResult::Idle
            };
        }

        // Handle progressive preloading of adjacent images (one per tick)
        // Only runs when no image load or render is pending
        if self.pending_preload && self.gpu_preload_count > 0 {
            // Preload one image per tick; keep pending_preload=true if more to do
            self.pending_preload = self.do_preloading_step(resources);

            if self.pending_preload {
                // Note: Tooltip timer takes priority if pending
                if tooltip_pending {
                    return TickResult::RequestIdleTimer(self.tooltip_manager.delay());
                }
                // Choose tick result based on whether GPU work was done:
                // - ContinueWork: Did GPU upload, framework should redraw
                // - ScheduleTick: Just polling worker, skip expensive UI redraw
                if self.preload_did_gpu_work {
                    return TickResult::ContinueWork;
                } else {
                    return TickResult::ScheduleTick;
                }
            }
        }

        // Auto-save check (native only, not in WASM)
        #[cfg(not(target_arch = "wasm32"))]
        if self.auto_save.should_save() {
            self.do_auto_save();
        }

        if needs_rebuild {
            TickResult::NeedsRebuild
        } else if tooltip_pending {
            // Tooltip is pending but not visible yet - request idle timer to show it
            // The framework will fire Event::IdleTimer after this duration of inactivity
            TickResult::RequestIdleTimer(self.tooltip_manager.delay())
        } else {
            TickResult::Idle
        }
    }

    fn on_event(&mut self, event: &Event) -> Option<Self::Message> {
        // Handle drag-drop events
        match event {
            Event::FilesDropped { paths } => {
                log::info!("on_event: FilesDropped with {} paths", paths.len());
                return Some(Message::FilesDropped(paths.clone()));
            }
            Event::DroppedFileData { name, data } => {
                // Store dropped file data for WASM processing
                // This is handled before FilesDropped in WASM
                log::info!("on_event: DroppedFileData for {}", name);
                // We'll collect these and process them when FilesDropped arrives
                // For now, store them in pending state
                self.pending_wasm_files.push(LoadedImage {
                    name: name.clone(),
                    data: data.clone(),
                });
                return None; // Don't return a message, wait for FilesDropped
            }
            Event::FileHoverStarted { .. } => {
                log::debug!("on_event: FileHoverStarted");
                return Some(Message::FileHoverStarted);
            }
            Event::FileHoverEnded => {
                log::debug!("on_event: FileHoverEnded");
                return Some(Message::FileHoverEnded);
            }
            Event::MouseMove { position, .. } => {
                // Track mouse movement for tooltip boundary checking
                // Only intercept if tooltip is VISIBLE - if still pending, let widgets
                // continue receiving events so they can update the tooltip request.
                // This fixes the issue where tooltips wouldn't appear until mouse moved
                // again after the initial hover.
                if self.tooltip_manager.is_visible() {
                    return Some(Message::TooltipMouseMove(*position));
                }
            }
            Event::CursorLeft => {
                // Clear tooltip when cursor leaves the window
                if self.tooltip_manager.current_id().is_some() {
                    self.tooltip_manager.clear();
                }
            }
            Event::IdleTimer => {
                // Idle timer expired - make pending tooltip visible
                if self.tooltip_manager.current_id().is_some() && !self.tooltip_manager.is_visible()
                {
                    self.tooltip_manager.force_show();
                    return Some(Message::TooltipBecameVisible);
                }
            }
            _ => {}
        }

        // Handle keyboard events
        self.handle_key_event(event)
    }

    fn on_resize(&mut self, width: f32, height: f32) {
        self.window_height = height;
        self.window_size = (width, height);
    }

    fn is_text_input_focused(&self) -> bool {
        self.any_text_input_focused()
    }

    fn needs_immediate_rebuild(&self) -> bool {
        // Request immediate rebuild during drawing or editing operations
        // so we can show real-time previews and handle position updates
        let path = self.current_image_path();
        let image_data = self.image_data_store.get(&path);
        image_data.drawing_state.is_drawing() || image_data.edit_state.is_editing()
    }
}
