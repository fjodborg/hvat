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
use hvat_ui::{Application, Column, Element, Event, KeyCode, Resources, Row};

use crate::constants::{
    DEFAULT_BRIGHTNESS, DEFAULT_CONTRAST, DEFAULT_GAMMA, DEFAULT_GPU_PRELOAD_COUNT, DEFAULT_HUE,
    DEFAULT_RED_BAND, DEFAULT_TEST_BANDS, DEFAULT_TEST_HEIGHT, DEFAULT_TEST_WIDTH,
    MAX_GPU_PRELOAD_COUNT, UNDO_HISTORY_SIZE,
};
use crate::data::HyperspectralData;
use crate::format::{AutoSaveManager, ExportOptions, FormatRegistry, ProjectData};
use crate::message::Message;
use crate::model::{
    Annotation, AnnotationShape, AnnotationTool, Category, DrawingState, EditState,
    HANDLE_HIT_RADIUS, MIN_DRAG_DISTANCE, MIN_POLYGON_VERTICES, POLYGON_CLOSE_THRESHOLD,
};
use crate::state::{
    AppSnapshot, GpuRenderState, GpuTextureCache, ImageDataStore, LoadedImage, ProjectState,
    SharedGpuPipeline,
};
use crate::test_image::generate_test_hyperspectral;

// ============================================================================
// Async Picker State (for WASM file picker)
// ============================================================================

/// Result of async file picker (used for WASM).
pub enum AsyncPickerResult {
    /// Folder selected (native only - contains folder path)
    Folder(PathBuf),
    /// Files selected (WASM - contains loaded image data)
    Files(Vec<LoadedImage>),
}

// Global shared state for receiving async picker results
static PENDING_PICKER_RESULT: OnceLock<std::sync::Mutex<Option<AsyncPickerResult>>> =
    OnceLock::new();

fn pending_picker_state() -> &'static std::sync::Mutex<Option<AsyncPickerResult>> {
    PENDING_PICKER_RESULT.get_or_init(|| std::sync::Mutex::new(None))
}

// ============================================================================
// WASM Folder Picker using File System Access API
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod wasm_folder_picker {
    use crate::state::{LoadedImage, is_image_filename};
    use hvat_ui::read_file_async;
    use js_sys::Reflect;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
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

            // Collect all files
            let mut web_files: Vec<web_sys::File> = Vec::new();
            for i in 0..file_count {
                if let Some(file) = files.get(i) {
                    let name = file.name();
                    if is_image_filename(&name) {
                        web_files.push(file);
                    } else {
                        log::debug!("Skipping non-image file: {}", name);
                    }
                }
            }

            log::info!("Found {} image files", web_files.len());

            // Clean up input element
            if let Some(parent) = input_clone.parent_element() {
                parent.remove_child(&input_clone).ok();
            }

            if web_files.is_empty() {
                log::warn!("No image files found in selected folder");
                return;
            }

            // Read files asynchronously
            let cb = callback_clone.clone();
            wasm_bindgen_futures::spawn_local(async move {
                use js_sys::Reflect;
                use wasm_bindgen::JsValue;

                let mut loaded_images: Vec<LoadedImage> = Vec::new();

                for file in web_files {
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
                log::info!("Successfully read {} image files", loaded_images.len());

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
    pub(crate) file_list_collapsed: CollapsibleState,
    pub(crate) file_list_scroll_state: ScrollState,
    pub(crate) thumbnails_collapsed: CollapsibleState,
    pub(crate) thumbnails_scroll_state: ScrollState,
    pub(crate) right_scroll_state: ScrollState,

    // Tool selection
    pub(crate) selected_tool: AnnotationTool,

    // Note: Annotations are now stored per-image in ImageDataStore

    // Categories
    pub(crate) categories: Vec<Category>,
    pub(crate) selected_category: u32,
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
    // Global tag registry (tags exist across all images)
    pub(crate) global_tags: Vec<String>,
    // Tag input UI state (not per-image)
    pub(crate) tag_input_text: String,
    pub(crate) tag_input_state: TextInputState,

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

    // Format system
    /// Format registry with all supported formats
    pub(crate) format_registry: FormatRegistry,
    /// Auto-save manager for automatic project persistence
    pub(crate) auto_save: AutoSaveManager,
    /// Path to the current project file (for auto-save)
    pub(crate) project_file_path: Option<PathBuf>,
    /// Whether the export dialog is open
    pub(crate) export_dialog_open: bool,

    // Drag-Drop State
    /// Whether files are being dragged over the window
    pub(crate) drag_hover_active: bool,
    /// Pending dropped files from WASM (collects DroppedFileData events)
    pub(crate) pending_wasm_files: Vec<LoadedImage>,
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
                        Err(e) => {
                            log::warn!("Failed to load test folder: {}", e);
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
            gpu_cache: GpuTextureCache::new(DEFAULT_GPU_PRELOAD_COUNT),

            pending_image_load: false,
            pending_preload: false,

            gpu_preload_count: DEFAULT_GPU_PRELOAD_COUNT,
            gpu_preload_slider: SliderState::new(DEFAULT_GPU_PRELOAD_COUNT as f32),

            tools_collapsed: CollapsibleState::expanded(),
            categories_collapsed: CollapsibleState::expanded(),
            tags_collapsed: CollapsibleState::collapsed(),
            left_scroll_state: ScrollState::default(),

            band_selection_collapsed: CollapsibleState::expanded(),
            adjustments_collapsed: CollapsibleState::expanded(),
            file_list_collapsed: CollapsibleState::expanded(),
            file_list_scroll_state: ScrollState::default(),
            thumbnails_collapsed: CollapsibleState::collapsed(),
            thumbnails_scroll_state: ScrollState::default(),
            right_scroll_state: ScrollState::default(),

            selected_tool: AnnotationTool::default(),

            // Annotations are stored per-image in image_data_store
            categories: vec![
                Category::new(1, "Background", [100, 100, 100]),
                Category::new(2, "Object", [255, 100, 100]),
                Category::new(3, "Region", [100, 255, 100]),
            ],
            selected_category: 1,
            editing_category: None,
            category_name_input: String::new(),
            category_name_input_state: TextInputState::default(),
            color_picker_category: None,
            color_picker_state: ColorPickerState::default(),

            image_data_store: ImageDataStore::new(),
            global_tags: Vec::new(),
            tag_input_text: String::new(),
            tag_input_state: TextInputState::default(),

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
            dependencies_collapsed: CollapsibleState::collapsed(),
            license_collapsed: std::collections::HashMap::new(),

            dark_theme: true, // Default to dark theme
            export_folder: String::new(),
            export_folder_state: TextInputState::default(),
            import_folder: String::new(),
            import_folder_state: TextInputState::default(),

            format_registry: FormatRegistry::new(),
            auto_save: AutoSaveManager::new(),
            project_file_path: None,
            export_dialog_open: false,

            drag_hover_active: false,
            pending_wasm_files: Vec::new(),
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

    /// Find a category by ID and return a mutable reference.
    fn find_category_mut(&mut self, id: u32) -> Option<&mut Category> {
        self.categories.iter_mut().find(|c| c.id == id)
    }

    /// Handle keyboard events for undo/redo and annotation shortcuts.
    fn handle_key_event(event: &Event) -> Option<Message> {
        if let Event::KeyPress { key, modifiers, .. } = event {
            // Ctrl+key shortcuts
            if modifiers.ctrl {
                match key {
                    KeyCode::Z if modifiers.shift => return Some(Message::Redo),
                    KeyCode::Z => return Some(Message::Undo),
                    KeyCode::Y => return Some(Message::Redo),
                    _ => {}
                }
            }

            // Non-modifier shortcuts for annotations
            match key {
                KeyCode::Escape => return Some(Message::CancelAnnotation),
                KeyCode::Delete | KeyCode::Backspace => return Some(Message::DeleteAnnotation),
                KeyCode::Enter => return Some(Message::FinishPolygon),
                _ => {}
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

    /// Handle native file drop (files and/or folders).
    /// Supports dropping folders (scanned recursively) and/or multiple image files.
    #[cfg(not(target_arch = "wasm32"))]
    fn handle_native_file_drop(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }

        log::info!("Native file drop: {} paths", paths.len());
        for path in &paths {
            log::debug!(
                "  - {:?} (is_dir={}, is_file={})",
                path,
                path.is_dir(),
                path.is_file()
            );
        }

        // Use the new from_paths method which handles both files and folders recursively
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
        // We need project info to know the new current index
        if let Some(ref project) = self.project {
            let to_keep = self
                .gpu_cache
                .paths_to_keep(&project.images, project.current_index);
            self.gpu_cache.retain_only(&to_keep);
        }

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

        // Not cached - load from disk/memory
        let hyper_result = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                HyperspectralData::from_image_file(&path)
            }

            #[cfg(target_arch = "wasm32")]
            {
                if let Some(ref project) = self.project {
                    project
                        .loaded_images
                        .iter()
                        .find(|img| PathBuf::from(&img.name) == path)
                        .ok_or_else(|| format!("Image data not found for {:?}", path))
                        .and_then(|loaded_img| HyperspectralData::from_bytes(&loaded_img.data))
                } else {
                    Err("No project loaded".to_string())
                }
            }
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

    /// Preload ONE adjacent image into the GPU cache.
    ///
    /// Called each tick to progressively preload images without blocking the UI.
    /// Returns true if there are more images to preload.
    fn do_preloading_step(&mut self, resources: &mut Resources<'_>) -> bool {
        let Some(ref project) = self.project else {
            return false;
        };
        let Some(ref pipeline) = self.shared_pipeline else {
            return false;
        };

        if project.images.is_empty() || self.gpu_preload_count == 0 {
            return false;
        }

        // Evict out-of-range entries to free GPU memory (do this once per navigation)
        let to_keep = self
            .gpu_cache
            .paths_to_keep(&project.images, project.current_index);
        self.gpu_cache.retain_only(&to_keep);

        // Get paths to preload (not cached, within range)
        let to_preload = self
            .gpu_cache
            .paths_to_preload(&project.images, project.current_index);

        if to_preload.is_empty() {
            log::debug!(
                "Preloading complete: {} images in GPU cache",
                self.gpu_cache.len()
            );
            return false;
        }

        // Preload just ONE image per tick to avoid blocking the UI
        let path = &to_preload[0];
        log::info!("Preloading (1 of {}): {:?}", to_preload.len(), path);

        let gpu_ctx = resources.gpu_context();

        let hyper_result = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                HyperspectralData::from_image_file(path)
            }

            #[cfg(target_arch = "wasm32")]
            {
                project
                    .loaded_images
                    .iter()
                    .find(|img| PathBuf::from(&img.name) == *path)
                    .ok_or_else(|| format!("Image not found: {:?}", path))
                    .and_then(|img| HyperspectralData::from_bytes(&img.data))
            }
        };

        match hyper_result {
            Ok(hyper) => {
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
        use hvat_ui::PointerEventKind;

        // Update viewer state to persist pointer_state changes
        self.viewer_state = event.viewer_state.clone();

        let x = event.image_x;
        let y = event.image_y;

        log::trace!(
            "ImagePointer: tool={:?}, pos=({:.1}, {:.1}), kind={:?}",
            self.selected_tool,
            x,
            y,
            event.kind
        );

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
            // Record potential drag - we don't start editing until there's actual movement
            log::debug!(
                "Potential drag on annotation {}, handle={:?}",
                annotation_id,
                handle
            );

            let image_data = self.image_data_store.get_or_create(&path);
            image_data.edit_state = EditState::PotentialDrag {
                annotation_id,
                handle,
                start_x: x,
                start_y: y,
                original_shape,
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
        if let DrawingState::Polygon { vertices } = &image_data.drawing_state {
            if vertices.len() >= MIN_POLYGON_VERTICES {
                let (first_x, first_y) = vertices[0];
                let dist = ((x - first_x).powi(2) + (y - first_y).powi(2)).sqrt();
                if dist < POLYGON_CLOSE_THRESHOLD {
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
                // Wrong drawing state, reset
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

        ProjectData::from_app_state(
            folder,
            &image_paths,
            &self.categories,
            &self.global_tags,
            |path| self.image_data_store.get(path),
            |path| self.get_image_dimensions(path),
        )
    }

    /// Get image dimensions for a path (from hyperspectral data if current image).
    fn get_image_dimensions(&self, path: &PathBuf) -> Option<(u32, u32)> {
        // If this is the current image, we have dimensions
        if let Some(ref project) = self.project {
            if project.current_image() == Some(path) {
                if self.image_size.0 > 0 && self.image_size.1 > 0 {
                    return Some(self.image_size);
                }
            }
        }
        // Otherwise we'd need to load the image to get dimensions
        // For now, return None - dimensions will be filled during export if needed
        None
    }

    /// Apply imported ProjectData to app state.
    pub fn apply_project_data(&mut self, data: ProjectData, merge: bool) {
        if !merge {
            // Clear existing data
            self.categories.clear();
            self.global_tags.clear();
            self.image_data_store = ImageDataStore::new();
        }

        // Apply categories
        for cat_entry in &data.categories {
            let exists = self.categories.iter().any(|c| c.id == cat_entry.id);
            if !exists {
                self.categories.push(cat_entry.to_category());
            }
        }

        // Apply global tags
        for tag in &data.global_tags {
            if !self.global_tags.contains(tag) {
                self.global_tags.push(tag.clone());
            }
        }

        // Apply image annotations
        for image_entry in &data.images {
            let image_data = self.image_data_store.get_or_create(&image_entry.path);

            if !merge {
                image_data.annotations.clear();
                image_data.selected_tags.clear();
            }

            for ann_entry in &image_entry.annotations {
                image_data.annotations.push(ann_entry.to_annotation());
            }

            image_data.selected_tags.extend(image_entry.tags.clone());

            // Update next_annotation_id
            if let Some(max_id) = image_data.annotations.iter().map(|a| a.id).max() {
                image_data.next_annotation_id = max_id + 1;
            }
        }

        log::info!(
            "Applied project data: {} categories, {} images, {} annotations",
            data.categories.len(),
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
        log::info!("HVAT setup: initializing GPU pipeline...");

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
            Message::ThemeChanged(dark) => {
                self.dark_theme = dark;
                log::info!("Theme changed to: {}", if dark { "dark" } else { "light" });
            }
            Message::ExportFolderChanged(text, state) => {
                self.export_folder = text;
                self.export_folder_state = state;
            }
            Message::ImportFolderChanged(text, state) => {
                self.import_folder = text;
                self.import_folder_state = state;
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
                self.selected_tool = tool;
                log::info!("Tool selected: {:?}", tool);
            }

            // Left Sidebar - Categories
            Message::CategoriesToggled(state) => {
                self.categories_collapsed = state;
            }
            Message::CategorySelected(id) => {
                self.selected_category = id;
                log::info!("Category selected: {}", id);
            }
            Message::AddCategory => {
                let new_id = self.categories.iter().map(|c| c.id).max().unwrap_or(0) + 1;
                self.categories.push(Category::new(
                    new_id,
                    &format!("Category {}", new_id),
                    [200, 200, 100],
                ));
                self.auto_save.mark_dirty();
                log::info!("Added new category: {}", new_id);
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
                self.category_name_input = text;
                self.category_name_input_state = state;
            }
            Message::FinishEditingCategory => {
                if let Some(id) = self.editing_category {
                    if !self.category_name_input.is_empty() {
                        if let Some(cat) = self.categories.iter_mut().find(|c| c.id == id) {
                            cat.name = self.category_name_input.clone();
                            self.auto_save.mark_dirty();
                            log::info!("Renamed category {} to '{}'", id, cat.name);
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
                    self.color_picker_category = Some(id);
                    log::info!("Opened color picker for category: {}", id);
                }
            }
            Message::CloseCategoryColorPicker => {
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
                    }
                }
                self.color_picker_category = None;
                self.color_picker_state = ColorPickerState::default();
            }
            Message::ColorPickerStateChanged(state) => {
                log::debug!("Color picker state changed: {:?}", state);
                self.color_picker_state = state;
            }

            // Left Sidebar - Tags (global registry with per-image selection)
            Message::TagsToggled(state) => {
                self.tags_collapsed = state;
            }
            Message::TagInputChanged(text, state) => {
                self.tag_input_text = text;
                self.tag_input_state = state;
            }
            Message::AddTag => {
                if !self.tag_input_text.is_empty() {
                    let tag = self.tag_input_text.clone();
                    // Add to global tag registry if not already present
                    if !self.global_tags.contains(&tag) {
                        self.global_tags.push(tag.clone());
                        log::info!("Added tag '{}' to global registry", tag);
                    }
                    // Also select it for the current image
                    let path = self.current_image_path();
                    let image_data = self.image_data_store.get_or_create(&path);
                    image_data.selected_tags.insert(tag.clone());
                    self.auto_save.mark_dirty();
                    log::info!("Selected tag '{}' for image {:?}", tag, path);

                    self.tag_input_text.clear();
                    self.tag_input_state.cursor = 0;
                }
            }
            Message::ToggleTag(tag) => {
                let path = self.current_image_path();
                let image_data = self.image_data_store.get_or_create(&path);
                if image_data.selected_tags.contains(&tag) {
                    image_data.selected_tags.remove(&tag);
                    log::info!("Deselected tag '{}' on image {:?}", tag, path);
                } else {
                    image_data.selected_tags.insert(tag.clone());
                    log::info!("Selected tag '{}' on image {:?}", tag, path);
                }
                self.auto_save.mark_dirty();
            }
            Message::RemoveTag(tag) => {
                // Remove from global registry (affects all images)
                self.global_tags.retain(|t| t != &tag);
                // Also remove from all per-image selections
                self.image_data_store.remove_tag_from_all(&tag);
                self.auto_save.mark_dirty();
                log::info!("Removed tag '{}' from global registry", tag);
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

            // Right Sidebar - File List
            Message::FileListToggled(state) => {
                self.file_list_collapsed = state;
            }
            Message::FileListScrolled(state) => {
                self.file_list_scroll_state = state;
            }
            Message::FileListSelect(index) => {
                if let Some(ref mut project) = self.project {
                    if index < project.images.len() {
                        project.current_index = index;
                        self.pending_image_load = true;
                        log::info!("File list: selected image {}", index);
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

            // Right Sidebar Scroll
            Message::RightScrolled(state) => {
                self.right_scroll_state = state;
            }

            // Global Undo/Redo
            // Priority: annotation undo first, then slider/adjustment undo
            Message::Undo => {
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

            // Settings - GPU Preloading
            Message::GpuPreloadCountChanged(state) => {
                self.gpu_preload_slider = state;
                let count = (self.gpu_preload_slider.value as usize).min(MAX_GPU_PRELOAD_COUNT);
                self.gpu_preload_count = count;
                self.gpu_cache.set_preload_count(count);
                log::info!("GPU preload count changed to: {}", count);

                // Trigger preloading with new count if we have a project
                if count > 0 && self.project.is_some() {
                    self.pending_preload = true;
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
                        let loaded_images = std::mem::take(&mut self.pending_wasm_files);
                        log::info!("Processing {} dropped files from WASM", loaded_images.len());

                        match ProjectState::from_loaded_images(loaded_images) {
                            Ok(project) => {
                                self.gpu_cache.clear();
                                log::info!(
                                    "Loaded project with {} images from drop",
                                    project.images.len()
                                );
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
        }
    }

    fn tick_with_resources(&mut self, resources: &mut Resources<'_>) -> bool {
        let mut needs_rebuild = false;

        // Check for async picker result
        if let Ok(mut pending) = pending_picker_state().lock() {
            if let Some(result) = pending.take() {
                match result {
                    #[cfg(not(target_arch = "wasm32"))]
                    AsyncPickerResult::Folder(folder) => {
                        log::info!("Processing folder from async picker: {:?}", folder);
                        match ProjectState::from_folder(folder) {
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
                    #[cfg(target_arch = "wasm32")]
                    AsyncPickerResult::Files(loaded_images) => {
                        log::info!("Processing {} files from async picker", loaded_images.len());
                        match ProjectState::from_loaded_images(loaded_images) {
                            Ok(project) => {
                                // Clear GPU cache when folder changes
                                self.gpu_cache.clear();
                                log::info!("Loaded project with {} images", project.images.len());
                                self.project = Some(project);
                                self.pending_image_load = true;
                            }
                            Err(e) => {
                                log::error!("Failed to load project: {}", e);
                            }
                        }
                    }
                    #[allow(unreachable_patterns)]
                    _ => {
                        log::warn!("Unexpected picker result type for current platform");
                    }
                }
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
            // But return true if preloading is pending so the framework keeps calling us
            return needs_rebuild || self.pending_preload;
        }

        // Re-render to texture if band selection or adjustments changed
        if self.needs_gpu_render {
            self.render_to_texture(resources);
            self.needs_gpu_render = false;
            needs_rebuild = true;
            // Don't preload in the same tick as a render
            return needs_rebuild;
        }

        // Handle progressive preloading of adjacent images (one per tick)
        // Only runs when no image load or render is pending
        if self.pending_preload && self.gpu_preload_count > 0 {
            // Preload one image per tick; keep pending_preload=true if more to do
            self.pending_preload = self.do_preloading_step(resources);

            // Return true if there's more preloading to do, so the framework
            // keeps calling tick_with_resources() without requiring user input
            if self.pending_preload {
                return true;
            }
        }

        // Auto-save check (native only, not in WASM)
        #[cfg(not(target_arch = "wasm32"))]
        if self.auto_save.should_save() {
            self.do_auto_save();
        }

        needs_rebuild
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
            _ => {}
        }

        // Handle keyboard events
        Self::handle_key_event(event)
    }

    fn on_resize(&mut self, _width: f32, height: f32) {
        self.window_height = height;
    }
}
