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
    DEFAULT_BRIGHTNESS, DEFAULT_CONTRAST, DEFAULT_GAMMA, DEFAULT_HUE, DEFAULT_RED_BAND,
    DEFAULT_TEST_BANDS, DEFAULT_TEST_HEIGHT, DEFAULT_TEST_WIDTH, UNDO_HISTORY_SIZE,
};
use crate::data::HyperspectralData;
use crate::message::Message;
use crate::model::{
    Annotation, AnnotationShape, AnnotationTool, Category, DrawingState, MIN_POLYGON_VERTICES,
    POLYGON_CLOSE_THRESHOLD,
};
use crate::state::{AppSnapshot, GpuRenderState, ImageDataStore, LoadedImage, ProjectState};
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
            left_scroll_state: ScrollState::default(),

            band_selection_collapsed: CollapsibleState::expanded(),
            adjustments_collapsed: CollapsibleState::expanded(),
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

    /// Get the current image path (for per-image data storage).
    /// Returns a test image path if no project is loaded.
    pub(crate) fn current_image_path(&self) -> PathBuf {
        self.project
            .as_ref()
            .and_then(|p| p.current_image().cloned())
            .unwrap_or_else(|| PathBuf::from("__test_image__"))
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
                self.reset_band_sliders();
                self.reset_adjustment_sliders();

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
                // Selection tool: select/deselect annotations on click
                if event.kind == PointerEventKind::DragStart {
                    self.handle_selection_click(x, y);
                }
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

    /// Handle selection tool click - select annotation under cursor.
    fn handle_selection_click(&mut self, x: f32, y: f32) {
        let path = self.current_image_path();
        let image_data = self.image_data_store.get_or_create(&path);

        // Deselect all first
        for ann in &mut image_data.annotations {
            ann.selected = false;
        }

        // Find annotation under cursor (reverse order for top-most first)
        let selected_idx = image_data
            .annotations
            .iter()
            .enumerate()
            .rev()
            .find(|(_, ann)| ann.shape.contains_point(x, y))
            .map(|(idx, _)| idx);

        if let Some(idx) = selected_idx {
            let id = image_data.annotations[idx].id;
            image_data.annotations[idx].selected = true;
            log::info!("Selected annotation {}", id);
        }
    }

    /// Handle bounding box drawing.
    fn handle_bounding_box_draw(&mut self, x: f32, y: f32, kind: hvat_ui::PointerEventKind) {
        use hvat_ui::PointerEventKind;

        let path = self.current_image_path();
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
                // Finish bounding box
                if let Some(shape) = image_data.drawing_state.to_shape() {
                    let annotation = Annotation::new(
                        image_data.next_annotation_id,
                        shape,
                        self.selected_category,
                    );
                    image_data.next_annotation_id += 1;
                    image_data.annotations.push(annotation);
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
            }
        }
        image_data.drawing_state = DrawingState::Idle;
    }

    /// Create a point annotation.
    fn create_point_annotation(&mut self, x: f32, y: f32) {
        let path = self.current_image_path();
        let image_data = self.image_data_store.get_or_create(&path);

        let shape = AnnotationShape::Point { x, y };
        let annotation =
            Annotation::new(image_data.next_annotation_id, shape, self.selected_category);
        image_data.next_annotation_id += 1;
        image_data.annotations.push(annotation);
        log::info!("Created point annotation at ({:.1}, {:.1})", x, y);
    }
}

// ============================================================================
// Application Implementation
// ============================================================================

impl Application for HvatApp {
    type Message = Message;

    fn setup(&mut self, resources: &mut Resources<'_>) {
        log::info!("HVAT setup: generating hyperspectral test image...");

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

        // Main application view
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
                            .add_filter(
                                "images",
                                &["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"],
                            )
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
                self.settings_open = !self.settings_open;
                log::info!("Settings toggled: {}", self.settings_open);
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
            }
            Message::RemoveTag(tag) => {
                // Remove from global registry (affects all images)
                self.global_tags.retain(|t| t != &tag);
                // Also remove from all per-image selections
                self.image_data_store.remove_tag_from_all(&tag);
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
                let before_count = image_data.annotations.len();
                image_data.annotations.retain(|a| !a.selected);
                let deleted = before_count - image_data.annotations.len();
                if deleted > 0 {
                    log::info!("Deleted {} annotation(s)", deleted);
                }
            }
            Message::FinishPolygon => {
                self.finalize_polygon();
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
