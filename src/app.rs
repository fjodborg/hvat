//! HVAT Application - Hyperspectral Annotation Tool
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

use crate::constants::{DEFAULT_TEST_BANDS, DEFAULT_TEST_HEIGHT, DEFAULT_TEST_WIDTH};
use crate::data::HyperspectralData;
use crate::message::Message;
use crate::model::{AnnotationTool, Category};
use crate::state::{AppSnapshot, GpuRenderState, LoadedImage, ProjectState};
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
static PENDING_PICKER_RESULT: OnceLock<std::sync::Mutex<Option<AsyncPickerResult>>> = OnceLock::new();

fn pending_picker_state() -> &'static std::sync::Mutex<Option<AsyncPickerResult>> {
    PENDING_PICKER_RESULT.get_or_init(|| std::sync::Mutex::new(None))
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

    // GPU rendering state (initialized in setup())
    gpu_state: Option<GpuRenderState>,

    // Flag indicating we need to load a new image
    pending_image_load: bool,

    // Left sidebar states
    pub(crate) tools_collapsed: CollapsibleState,
    pub(crate) categories_collapsed: CollapsibleState,
    pub(crate) tags_collapsed: CollapsibleState,
    pub(crate) left_scroll_state: ScrollState,

    // Right sidebar states
    pub(crate) band_selection_collapsed: CollapsibleState,
    pub(crate) adjustments_collapsed: CollapsibleState,
    pub(crate) right_scroll_state: ScrollState,

    // Tool selection
    pub(crate) selected_tool: AnnotationTool,

    // Categories
    pub(crate) categories: Vec<Category>,
    pub(crate) selected_category: u32,

    // Image tags (per-image)
    pub(crate) image_tags: Vec<String>,
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

            project: None,

            hyperspectral: None,
            num_bands,
            band_selection: (0, 1, 2),

            gpu_state: None,
            pending_image_load: false,

            tools_collapsed: CollapsibleState::expanded(),
            categories_collapsed: CollapsibleState::expanded(),
            tags_collapsed: CollapsibleState::collapsed(),
            left_scroll_state: ScrollState::new(),

            band_selection_collapsed: CollapsibleState::expanded(),
            adjustments_collapsed: CollapsibleState::expanded(),
            right_scroll_state: ScrollState::new(),

            selected_tool: AnnotationTool::default(),

            categories: vec![
                Category::new(1, "Background", [100, 100, 100]),
                Category::new(2, "Object", [255, 100, 100]),
                Category::new(3, "Region", [100, 255, 100]),
            ],
            selected_category: 1,

            image_tags: vec!["needs-review".to_string()],
            tag_input_text: String::new(),
            tag_input_state: TextInputState::new(),

            red_band_slider: SliderState::new(0.0),
            green_band_slider: SliderState::new(1.0),
            blue_band_slider: SliderState::new(2.0),

            brightness_slider: SliderState::new(0.0),
            contrast_slider: SliderState::new(1.0),
            gamma_slider: SliderState::new(1.0),
            hue_slider: SliderState::new(0.0),

            undo_stack: Rc::new(RefCell::new(UndoStack::new(50))),

            window_height: 900.0,

            needs_gpu_render: true,
        }
    }

    /// Create a snapshot of current state for undo.
    pub(crate) fn snapshot(&self) -> AppSnapshot {
        AppSnapshot {
            red_band: self.band_selection.0,
            green_band: self.band_selection.1,
            blue_band: self.band_selection.2,
            brightness: self.brightness_slider.value,
            contrast: self.contrast_slider.value,
            gamma: self.gamma_slider.value,
            hue: self.hue_slider.value,
        }
    }

    /// Restore state from a snapshot.
    fn restore(&mut self, snapshot: &AppSnapshot) {
        self.band_selection = (snapshot.red_band, snapshot.green_band, snapshot.blue_band);
        self.red_band_slider.set_value(snapshot.red_band as f32);
        self.green_band_slider.set_value(snapshot.green_band as f32);
        self.blue_band_slider.set_value(snapshot.blue_band as f32);
        self.brightness_slider.set_value(snapshot.brightness);
        self.contrast_slider.set_value(snapshot.contrast);
        self.gamma_slider.set_value(snapshot.gamma);
        self.hue_slider.set_value(snapshot.hue);
        self.needs_gpu_render = true;
    }

    /// Handle keyboard events for undo/redo shortcuts.
    fn handle_key_event(event: &Event) -> Option<Message> {
        if let Event::KeyPress { key, modifiers, .. } = event {
            if modifiers.ctrl {
                match key {
                    KeyCode::Z if modifiers.shift => return Some(Message::Redo),
                    KeyCode::Z => return Some(Message::Undo),
                    KeyCode::Y => return Some(Message::Redo),
                    _ => {}
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

        let gpu_ctx = resources.gpu_context();

        match GpuRenderState::new(
            gpu_ctx,
            hyper,
            self.band_selection,
            self.image_adjustments(),
        ) {
            Ok(state) => {
                self.image_size = (hyper.width, hyper.height);
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

        let gpu_ctx = resources.gpu_context();

        gpu_state.render(
            gpu_ctx,
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

    /// Load an image file and reinitialize GPU state.
    fn load_image_file(&mut self, path: PathBuf, resources: &mut Resources<'_>) {
        log::info!("Loading image: {:?}", path);

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
                self.band_selection = (0, 1.min(self.num_bands - 1), 2.min(self.num_bands - 1));

                let max_band = (self.num_bands - 1) as f32;
                self.red_band_slider.set_value(0.0);
                self.green_band_slider.set_value(1.0_f32.min(max_band));
                self.blue_band_slider.set_value(2.0_f32.min(max_band));

                self.brightness_slider.set_value(0.0);
                self.contrast_slider.set_value(1.0);
                self.gamma_slider.set_value(1.0);
                self.hue_slider.set_value(0.0);

                self.texture_id = None;
                self.hyperspectral = Some(hyper);

                self.init_gpu_state(resources);
                self.render_to_texture(resources);

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
}

// ============================================================================
// Application Implementation
// ============================================================================

impl Application for HvatApp {
    type Message = Message;

    fn setup(&mut self, resources: &mut Resources<'_>) {
        log::info!("HVAT setup: generating hyperspectral test image...");

        let hyper = generate_test_hyperspectral(DEFAULT_TEST_WIDTH, DEFAULT_TEST_HEIGHT, DEFAULT_TEST_BANDS);
        self.hyperspectral = Some(hyper);
        self.num_bands = DEFAULT_TEST_BANDS;

        self.init_gpu_state(resources);
        self.render_to_texture(resources);

        log::info!("HVAT setup complete - GPU pipeline initialized");
    }

    fn view(&self) -> Element<Self::Message> {
        let topbar = self.build_topbar();
        let left_sidebar = self.build_left_sidebar();
        let center_viewer = self.build_image_viewer();
        let right_sidebar = self.build_right_sidebar();

        let main_row = Row::new(vec![left_sidebar, center_viewer, right_sidebar])
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
                        match ProjectState::from_folder(folder) {
                            Ok(project) => {
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
                    wasm_bindgen_futures::spawn_local(async {
                        let files = rfd::AsyncFileDialog::new()
                            .add_filter("images", &["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"])
                            .pick_files()
                            .await;

                        if let Some(file_handles) = files {
                            let mut loaded_images = Vec::new();

                            for handle in file_handles {
                                let name = handle.file_name();
                                let data = handle.read().await;
                                loaded_images.push(LoadedImage { name, data });
                            }

                            log::info!("WASM: loaded {} files", loaded_images.len());
                            if let Ok(mut pending) = pending_picker_state().lock() {
                                *pending = Some(AsyncPickerResult::Files(loaded_images));
                            }
                        }
                    });
                }
            }
            Message::FolderLoaded(project) => {
                log::info!("Folder loaded with {} images", project.images.len());
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
                log::info!("Settings toggle requested");
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
                let new_id = self.categories.len() as u32 + 1;
                self.categories
                    .push(Category::new(new_id, &format!("Category {}", new_id), [200, 200, 100]));
                log::info!("Added new category: {}", new_id);
            }

            // Left Sidebar - Tags
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
                    if !self.image_tags.contains(&tag) {
                        self.image_tags.push(tag.clone());
                        log::info!("Added tag: {}", tag);
                    }
                    self.tag_input_text.clear();
                    self.tag_input_state.cursor = 0;
                }
            }
            Message::RemoveTag(tag) => {
                self.image_tags.retain(|t| t != &tag);
                log::info!("Removed tag: {}", tag);
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
                self.brightness_slider.set_value(0.0);
                self.contrast_slider.set_value(1.0);
                self.gamma_slider.set_value(1.0);
                self.hue_slider.set_value(0.0);
                self.needs_gpu_render = true;
                log::info!("Adjustments reset");
            }

            // Right Sidebar Scroll
            Message::RightScrolled(state) => {
                self.right_scroll_state = state;
            }

            // Global Undo/Redo
            Message::Undo => {
                let current = self.snapshot();
                let prev = self.undo_stack.borrow_mut().undo(current);
                if let Some(prev) = prev {
                    self.restore(&prev);
                    log::info!("Undo performed");
                }
            }
            Message::Redo => {
                let current = self.snapshot();
                let next = self.undo_stack.borrow_mut().redo(current);
                if let Some(next) = next {
                    self.restore(&next);
                    log::info!("Redo performed");
                }
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
        }

        // Re-render to texture if band selection or adjustments changed
        if self.needs_gpu_render {
            self.render_to_texture(resources);
            self.needs_gpu_render = false;
            needs_rebuild = true;
        }

        needs_rebuild
    }

    fn on_event(&mut self, event: &Event) -> Option<Self::Message> {
        Self::handle_key_event(event)
    }

    fn on_resize(&mut self, _width: f32, height: f32) {
        self.window_height = height;
    }
}
