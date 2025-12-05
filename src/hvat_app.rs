//! The main HVAT application - shared between native and WASM builds.
//!
//! This file contains the core HvatApp struct and the Application trait implementation.
//! Logic is delegated to specialized modules:
//! - message: Message types and constructors
//! - theme: Theme system
//! - views: View building functions
//! - handlers: Message handlers
//! - wasm_file: WASM file loading

use crate::annotation::{AnnotationStore, DrawingState};
use crate::color_utils::hsv_to_rgb;
use crate::handlers::{
    handle_annotation, handle_band, handle_counter, handle_image_load, handle_image_settings,
    handle_image_view, handle_navigation, handle_ui, AnnotationState, ImageLoadState,
};
use crate::hyperspectral::{generate_test_hyperspectral, BandSelection, HyperspectralImage};
use crate::image_cache::ImageCache;
use crate::message::{ExportFormat, Message, PersistenceMode, Tab};
use crate::theme::Theme;
use crate::ui_constants::{annotation as ann_const, test_image};
use crate::views::{build_overlay, view_counter, view_export_modal_content, view_home, view_image_viewer, view_settings};
use crate::widget_state::WidgetState;
use hvat_ui::Overlay;

#[cfg(target_arch = "wasm32")]
use crate::message::ImageLoadMessage;
#[cfg(target_arch = "wasm32")]
use crate::wasm_file::take_wasm_pending_files;
use hvat_ui::widgets::{button, column, container, modal, row, scrollable, text, Element, ScrollDirection};
use hvat_ui::{Application, Color, HyperspectralImageHandle, ImageHandle};
use std::collections::{HashMap, HashSet};
use web_time::Instant;

/// Main application state.
pub struct HvatApp {
    // === Navigation ===
    current_tab: Tab,

    // === Counter demo ===
    counter: i32,

    // === Image viewer - transform state ===
    transform: ImageViewTransform,

    // === Image manipulation settings ===
    image_settings: ImageSettings,

    // === Settings ===
    show_debug_info: bool,
    theme: Theme,

    // === Image data ===
    /// The raw hyperspectral/multi-band image data (source of truth)
    hyperspectral_image: HyperspectralImage,
    /// GPU handle for the hyperspectral image (band data uploaded to GPU once)
    hyperspectral_handle: HyperspectralImageHandle,
    /// The rendered composite image (fallback for non-hyperspectral images)
    current_image: ImageHandle,
    /// Current band selection for RGB composite
    band_selection: BandSelection,
    /// Image cache for loading/preloading (unified native/WASM)
    image_cache: ImageCache,
    /// Current index in the loaded images list
    current_image_index: usize,
    /// Status message to display
    status_message: Option<String>,

    // === Transient UI state (drag states, hover, etc.) ===
    widget_state: WidgetState,

    // === Annotation system ===
    /// Annotation storage per image (keyed by image name/path)
    annotations_map: HashMap<String, AnnotationStore>,
    /// Global categories shared across all images
    categories: HashMap<u32, crate::annotation::Category>,
    /// Current drawing state (tool, in-progress points)
    drawing_state: DrawingState,
    /// Cached overlay - rebuilt only when annotations or drawing state changes
    cached_overlay: Overlay,
    /// Last image key used for cached overlay (invalidate on image change)
    cached_overlay_image_key: String,

    // === FPS tracking ===
    fps_tracker: FpsTracker,

    // === Persistence settings ===
    persistence: PersistenceState,

    // === Export settings ===
    export: ExportState,

    // === Image tagging ===
    tagging: TaggingState,
}

/// Stored image manipulation settings for per-image persistence.
#[derive(Clone, Copy, Debug)]
pub struct ImageSettings {
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub hue_shift: f32,
}

/// Image view transform state (zoom and pan).
#[derive(Clone, Copy, Debug)]
pub struct ImageViewTransform {
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
}

impl Default for ImageViewTransform {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
}

/// FPS tracking state.
#[derive(Debug)]
pub struct FpsTracker {
    pub frame_count: u32,
    pub last_fps_time: Instant,
    pub fps: f32,
}

impl Default for FpsTracker {
    fn default() -> Self {
        Self {
            frame_count: 0,
            last_fps_time: Instant::now(),
            fps: 0.0,
        }
    }
}

impl FpsTracker {
    /// Update FPS tracking. Call once per frame.
    pub fn update(&mut self) {
        self.frame_count += 1;
        let elapsed = self.last_fps_time.elapsed();
        if elapsed.as_secs_f32() >= 1.0 {
            self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_count = 0;
            self.last_fps_time = Instant::now();
        }
    }
}

/// Settings persistence state.
#[derive(Debug, Default)]
pub struct PersistenceState {
    /// How band selection should persist across image navigation
    pub band_mode: PersistenceMode,
    /// How image settings (brightness, contrast, etc.) should persist
    pub image_settings_mode: PersistenceMode,
    /// Stored band selections per image (for PerImage mode)
    pub stored_band_selections: HashMap<String, BandSelection>,
    /// Stored image settings per image (for PerImage mode)
    pub stored_image_settings: HashMap<String, ImageSettings>,
}

/// Export dialog state.
#[derive(Debug)]
pub struct ExportState {
    /// Whether export dialog is visible
    pub dialog_open: bool,
    /// Currently selected export format
    pub format: ExportFormat,
}

impl Default for ExportState {
    fn default() -> Self {
        Self {
            dialog_open: false,
            format: ExportFormat::default(),
        }
    }
}

/// Image tagging state.
#[derive(Debug, Default)]
pub struct TaggingState {
    /// Available tag definitions
    pub available_tags: Vec<Tag>,
    /// Tags applied to each image (keyed by image name/path, value is set of tag IDs)
    pub image_tags: HashMap<String, HashSet<u32>>,
}

/// An image tag definition.
#[derive(Clone, Debug)]
pub struct Tag {
    pub id: u32,
    pub name: String,
    pub color: [f32; 4],
}

impl Tag {
    /// Create a new tag with auto-generated color.
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        // Generate distinct colors using golden angle
        let hue = (id as f32 * ann_const::GOLDEN_ANGLE) % 360.0;
        let (r, g, b) = hsv_to_rgb(hue, 0.7, 0.9);
        Self {
            id,
            name: name.into(),
            color: [r, g, b, 1.0],
        }
    }
}

impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            hue_shift: 0.0,
        }
    }
}

impl Application for HvatApp {
    type Message = Message;

    fn new() -> Self {
        // Create a test hyperspectral image
        log::info!(
            "Creating {}x{} test hyperspectral image ({} bands)...",
            test_image::WIDTH,
            test_image::HEIGHT,
            test_image::BANDS
        );
        let hyper_image = generate_test_hyperspectral(test_image::WIDTH, test_image::HEIGHT, test_image::BANDS);
        let band_selection = BandSelection::default_rgb();

        // Create GPU handle for hyperspectral rendering (band data uploaded once)
        let hyperspectral_handle = hyper_image.to_gpu_handle();

        // Create fallback composite for non-GPU path
        let current_image = hyper_image
            .to_rgb_composite(band_selection.red, band_selection.green, band_selection.blue)
            .expect("Failed to create initial composite");
        log::info!("Test image created (GPU-accelerated band compositing enabled)");

        Self {
            current_tab: Tab::Home,
            counter: 0,
            transform: ImageViewTransform::default(),
            image_settings: ImageSettings::default(),
            show_debug_info: false,
            theme: Theme::dark(),
            hyperspectral_image: hyper_image,
            hyperspectral_handle,
            current_image,
            band_selection,
            image_cache: ImageCache::new(1),
            current_image_index: 0,
            status_message: None,
            widget_state: WidgetState::new(),
            annotations_map: HashMap::new(),
            categories: {
                let mut cats = HashMap::new();
                cats.insert(0, crate::annotation::Category::new(0, "Object"));
                cats
            },
            drawing_state: DrawingState::new(),
            cached_overlay: Overlay::new(),
            cached_overlay_image_key: String::new(),
            fps_tracker: FpsTracker::default(),
            persistence: PersistenceState::default(),
            export: ExportState::default(),
            tagging: TaggingState::default(),
        }
    }

    fn title(&self) -> String {
        "HVAT - Hyperspectral Annotation Tool".to_string()
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::Navigation(msg) => {
                handle_navigation(msg, &mut self.current_tab);
            }
            Message::Counter(msg) => {
                handle_counter(msg, &mut self.counter);
            }
            Message::ImageView(msg) => {
                // Handle ResetToOneToOne specially since it needs image dimensions
                if matches!(msg, crate::message::ImageViewMessage::ResetToOneToOne) {
                    // Use cached dimensions, or fall back to hyperspectral image dimensions
                    let dims = self.image_cache.get_dimensions(self.current_image_index)
                        .or_else(|| Some((self.hyperspectral_image.width, self.hyperspectral_image.height)));
                    if let Some((img_w, img_h)) = dims {
                        // Calculate zoom for 1:1 pixel ratio
                        // Use actual widget bounds if available, otherwise use defaults
                        use crate::ui_constants::image_viewer as img_const;
                        let (widget_w, widget_h) = self.widget_state.image.widget_bounds
                            .unwrap_or((img_const::WIDTH, img_const::HEIGHT));
                        let img_aspect = img_w as f32 / img_h as f32;
                        let widget_aspect = widget_w / widget_h;

                        // Fit scale: how much the image is scaled to fit widget at zoom=1.0
                        let fit_scale = if img_aspect > widget_aspect {
                            widget_w / img_w as f32
                        } else {
                            widget_h / img_h as f32
                        };

                        // For 1:1, we need actual_scale = 1.0
                        // actual_scale = fit_scale * zoom
                        // zoom = 1.0 / fit_scale
                        self.transform.zoom = (1.0 / fit_scale).clamp(crate::ui_constants::zoom::MIN, crate::ui_constants::zoom::MAX);
                        self.transform.pan_x = 0.0;
                        self.transform.pan_y = 0.0;
                        log::debug!("🔍 Reset to 1:1 pixel ratio: zoom = {:.2}x", self.transform.zoom);
                    }
                    return;
                }
                handle_image_view(
                    msg,
                    &mut self.transform.zoom,
                    &mut self.transform.pan_x,
                    &mut self.transform.pan_y,
                    &mut self.widget_state,
                );
            }
            Message::ImageSettings(msg) => {
                handle_image_settings(
                    msg,
                    &mut self.image_settings.brightness,
                    &mut self.image_settings.contrast,
                    &mut self.image_settings.gamma,
                    &mut self.image_settings.hue_shift,
                    &mut self.widget_state,
                );
            }
            Message::ImageLoad(msg) => {
                use crate::message::ImageLoadMessage;

                // Check if this is an image navigation message
                let is_navigation = matches!(
                    msg,
                    ImageLoadMessage::NextImage | ImageLoadMessage::PreviousImage
                );
                let is_load = matches!(
                    msg,
                    ImageLoadMessage::LoadFolder | ImageLoadMessage::FolderLoaded(_)
                );
                #[cfg(target_arch = "wasm32")]
                let is_load = is_load || matches!(msg, ImageLoadMessage::WasmFilesLoaded(_));

                // Before navigation: save current settings for PerImage mode
                if is_navigation {
                    self.save_current_settings();
                }

                // Remember if the target image is new (before we load it and mark it as viewed)
                let old_index = self.current_image_index;

                let mut state = ImageLoadState {
                    image_cache: &mut self.image_cache,
                    current_image_index: &mut self.current_image_index,
                    current_image: &mut self.current_image,
                    hyperspectral_image: &mut self.hyperspectral_image,
                    hyperspectral_handle: &mut self.hyperspectral_handle,
                    band_selection: &mut self.band_selection,
                    status_message: &mut self.status_message,
                    zoom: &mut self.transform.zoom,
                    pan_x: &mut self.transform.pan_x,
                    pan_y: &mut self.transform.pan_y,
                };
                handle_image_load(msg, &mut state);

                // After navigation: apply persistence logic
                let image_changed = old_index != self.current_image_index || is_load;
                if image_changed {
                    self.apply_settings_for_image();
                }

                // Rebuild overlay when image changes (annotations are per-image)
                self.rebuild_overlay_if_dirty();
            }
            Message::UI(msg) => {
                // Handle SubmitNewCategory specially since it needs access to global categories
                if matches!(msg, crate::message::UIMessage::SubmitNewCategory) {
                    let name = self.widget_state.category_input.new_category_name.trim().to_string();
                    if !name.is_empty() {
                        // Find next category ID from global categories
                        let next_id = self.categories.keys().max().copied().unwrap_or(0) + 1;
                        // Add category to global categories
                        self.add_category(crate::annotation::Category::new(next_id, &name));
                        log::info!("🏷️ Added new category: {} (id={})", name, next_id);
                    }
                    // Always clear input and unfocus after submit (even if empty)
                    self.widget_state.category_input.clear();
                }
                // Handle SubmitNewTag specially since it needs access to available_tags
                if matches!(msg, crate::message::UIMessage::SubmitNewTag) {
                    let name = self.widget_state.tag_input.new_tag_name.trim().to_string();
                    if !name.is_empty() {
                        // Find next tag ID
                        let next_id = self.tagging.available_tags.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                        // Add tag to available tags
                        self.tagging.available_tags.push(Tag::new(next_id, &name));
                        log::info!("🏷️ Added new tag: {} (id={})", name, next_id);
                    }
                    // Always clear input and unfocus after submit (even if empty)
                    self.widget_state.tag_input.clear();
                }
                handle_ui(
                    msg,
                    &mut self.widget_state,
                    &mut self.show_debug_info,
                    &mut self.theme,
                    &mut self.persistence.band_mode,
                    &mut self.persistence.image_settings_mode,
                );
            }
            Message::Tag(msg) => {
                use crate::message::TagMessage;
                match msg {
                    TagMessage::ToggleTagByHotkey(num) => {
                        // Map hotkey number (1-9) to tag ID based on sorted order
                        let mut tag_ids: Vec<u32> = self.tagging.available_tags.iter().map(|t| t.id).collect();
                        tag_ids.sort();
                        let index = (num as usize).saturating_sub(1);
                        if let Some(&tag_id) = tag_ids.get(index) {
                            let tags = self.current_image_tags_mut();
                            if tags.contains(&tag_id) {
                                tags.remove(&tag_id);
                                log::info!("🏷️ Removed tag {} from image (hotkey {})", tag_id, num);
                            } else {
                                tags.insert(tag_id);
                                log::info!("🏷️ Added tag {} to image (hotkey {})", tag_id, num);
                            }
                        }
                    }
                    TagMessage::ToggleTag(tag_id) => {
                        let tags = self.current_image_tags_mut();
                        if tags.contains(&tag_id) {
                            tags.remove(&tag_id);
                            log::info!("🏷️ Removed tag {} from image", tag_id);
                        } else {
                            tags.insert(tag_id);
                            log::info!("🏷️ Added tag {} to image", tag_id);
                        }
                    }
                    TagMessage::AddTag(name) => {
                        let next_id = self.tagging.available_tags.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                        self.tagging.available_tags.push(Tag::new(next_id, &name));
                        log::info!("🏷️ Added new tag: {} (id={})", name, next_id);
                    }
                }
            }
            Message::Annotation(msg) => {
                let image_key = self.current_image_key();
                let mut state = AnnotationState {
                    annotations_map: &mut self.annotations_map,
                    categories: &mut self.categories,
                    drawing_state: &mut self.drawing_state,
                    image_key,
                    zoom: self.transform.zoom,
                    status_message: &mut self.status_message,
                    export_dialog_open: &mut self.export.dialog_open,
                    export_format: &mut self.export.format,
                    widget_state: &mut self.widget_state,
                    image_cache: &self.image_cache,
                };
                handle_annotation(msg, &mut state);
                // Rebuild overlay if annotations changed
                self.rebuild_overlay_if_dirty();
            }
            Message::Band(msg) => {
                let num_bands = self.hyperspectral_image.num_bands();
                // With GPU-based compositing, band selection changes are instant
                // (just updates a uniform buffer), so no need to queue or throttle
                let _should_update = handle_band(
                    msg,
                    &mut self.band_selection,
                    num_bands,
                    &mut self.widget_state,
                );
                // Band selection is passed to the GPU widget each frame,
                // so changes are reflected immediately with no CPU work needed
            }
            Message::Project(msg) => {
                use crate::handlers::{handle_project, ProjectState};
                let mut state = ProjectState {
                    annotations_map: &self.annotations_map,
                    image_cache: &self.image_cache,
                    band_selection: self.band_selection,
                    image_settings: self.image_settings,
                    band_persistence: self.persistence.band_mode,
                    image_settings_persistence: self.persistence.image_settings_mode,
                    stored_band_selections: &self.persistence.stored_band_selections,
                    stored_image_settings: &self.persistence.stored_image_settings,
                    status_message: &mut self.status_message,
                };
                if let Some(project) = handle_project(msg, &mut state) {
                    self.apply_loaded_project(project);
                }
            }
            Message::Tick => {
                self.fps_tracker.update();
                // No composite generation needed - GPU handles band selection
            }
        }
    }

    fn tick(&self) -> Option<Self::Message> {
        // In WASM, check for pending files from file picker
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(files) = take_wasm_pending_files() {
                return Some(Message::ImageLoad(ImageLoadMessage::WasmFilesLoaded(files)));
            }
        }
        Some(Message::Tick)
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let _bg_color = self.theme.background_color();
        let text_color = self.theme.text_color();

        // Header with title and navigation
        let header_row = row()
            .push(Element::new(
                text("HVAT").size(20.0).color(self.theme.accent_color()),
            ))
            .push(Element::new(
                button("Home")
                    .on_press(Message::switch_tab(Tab::Home))
                    .width(100.0),
            ))
            .push(Element::new(
                button("Counter")
                    .on_press(Message::switch_tab(Tab::Counter))
                    .width(100.0),
            ))
            .push(Element::new(
                button("Image")
                    .on_press(Message::switch_tab(Tab::ImageViewer))
                    .width(100.0),
            ))
            .push(Element::new(
                button("Settings")
                    .on_press(Message::switch_tab(Tab::Settings))
                    .width(100.0),
            ))
            // FPS counter
            .push(Element::new(
                text(format!("FPS: {:.0}", self.fps_tracker.fps))
                    .size(14.0)
                    .color(Color::rgb(0.5, 0.8, 0.5)),
            ))
            .spacing(10.0);

        // Wrap header in container with background to cover any content that bleeds through
        let header = container(Element::new(header_row))
            .padding(5.0)
            .background(self.theme.background_color());

        // Content based on current tab
        let content: Element<'_, Message> = match self.current_tab {
            Tab::Home => Element::new(view_home(&self.theme, text_color)),
            Tab::Counter => Element::new(view_counter(&self.theme, text_color, self.counter)),
            Tab::ImageViewer => Element::new(view_image_viewer(
                &self.theme,
                text_color,
                &self.hyperspectral_handle,
                self.transform.zoom,
                self.transform.pan_x,
                self.transform.pan_y,
                self.image_settings.brightness,
                self.image_settings.contrast,
                self.image_settings.gamma,
                self.image_settings.hue_shift,
                &self.widget_state,
                &self.drawing_state,
                self.annotations(),
                self.categories().collect(),
                self.status_message.as_deref(),
                &self.band_selection,
                self.num_bands(),
                self.cached_overlay.clone(),
                self.persistence.band_mode,
                self.persistence.image_settings_mode,
                &self.tagging.available_tags,
                self.current_image_tags(),
                // Use cached dimensions, or fall back to hyperspectral image dimensions for test images
                self.image_cache.get_dimensions(self.current_image_index)
                    .or_else(|| Some((self.hyperspectral_image.width, self.hyperspectral_image.height))),
            )),
            Tab::Settings => Element::new(view_settings(
                &self.theme,
                text_color,
                self.show_debug_info,
            )),
        };

        // Wrap content in scrollable - supports both vertical and horizontal scrolling
        // fill_viewport enables children with Length::Fill to expand to fill the viewport
        let mut scrollable_content = scrollable(content)
            .direction(ScrollDirection::Both)
            .fill_viewport()
            .scroll_offset_y(self.widget_state.scroll.offset_y)
            .scroll_offset_x(self.widget_state.scroll.offset_x)
            .dragging_y(self.widget_state.scroll.is_dragging_y)
            .dragging_x(self.widget_state.scroll.is_dragging_x)
            .on_scroll_y(Message::scroll_y)
            .on_scroll_x(Message::scroll_x)
            .on_drag_start_y(Message::scrollbar_drag_start_y)
            .on_drag_end_y(Message::scrollbar_drag_end_y)
            .on_drag_start_x(Message::scrollbar_drag_start_x)
            .on_drag_end_x(Message::scrollbar_drag_end_x);

        // Pass drag start values for relative scrollbar dragging
        if let (Some(mouse_y), Some(scroll_y)) = (
            self.widget_state.scroll.drag_start_mouse_y,
            self.widget_state.scroll.drag_start_scroll_y,
        ) {
            scrollable_content = scrollable_content.drag_start_y(mouse_y, scroll_y);
        }
        if let (Some(mouse_x), Some(scroll_x)) = (
            self.widget_state.scroll.drag_start_mouse_x,
            self.widget_state.scroll.drag_start_scroll_x,
        ) {
            scrollable_content = scrollable_content.drag_start_x(mouse_x, scroll_x);
        }

        // Main app container wrapped with export modal
        // The modal renders as an overlay on top when visible
        let main_content = container(Element::new(
            column()
                .push(Element::new(header))
                .push(Element::new(scrollable_content))
                .spacing(20.0),
        ))
        .padding(30.0)
        .fill();

        // Wrap main content with export modal
        // Modal uses overlay rendering so it appears on top when visible
        Element::new(
            modal(
                Element::new(main_content),
                Element::new(view_export_modal_content(self.export.format)),
            )
            .visible(self.export.dialog_open)
            .width(320.0)
            .on_backdrop_click(Message::close_export_dialog),
        )
    }
}

impl HvatApp {
    /// Get the current image key for annotation storage.
    fn current_image_key(&self) -> String {
        self.image_cache
            .get_name(self.current_image_index)
            .unwrap_or_else(|| "default".to_string())
    }

    /// Get annotations for the current image.
    fn annotations(&self) -> &AnnotationStore {
        static EMPTY: std::sync::OnceLock<AnnotationStore> = std::sync::OnceLock::new();
        let key = self.current_image_key();
        self.annotations_map
            .get(&key)
            .unwrap_or_else(|| EMPTY.get_or_init(AnnotationStore::new))
    }

    /// Get all categories (global, shared across all images).
    pub fn categories(&self) -> impl Iterator<Item = &crate::annotation::Category> {
        self.categories.values()
    }

    /// Get a category by ID.
    pub fn get_category(&self, id: u32) -> Option<&crate::annotation::Category> {
        self.categories.get(&id)
    }

    /// Add a new category.
    pub fn add_category(&mut self, category: crate::annotation::Category) {
        self.categories.insert(category.id, category);
    }

    /// Get tags for the current image.
    fn current_image_tags(&self) -> &HashSet<u32> {
        static EMPTY: std::sync::OnceLock<HashSet<u32>> = std::sync::OnceLock::new();
        let key = self.current_image_key();
        self.tagging.image_tags
            .get(&key)
            .unwrap_or_else(|| EMPTY.get_or_init(HashSet::new))
    }

    /// Get mutable tags for the current image.
    fn current_image_tags_mut(&mut self) -> &mut HashSet<u32> {
        let key = self.current_image_key();
        self.tagging.image_tags.entry(key).or_insert_with(HashSet::new)
    }

    /// Rebuild overlay if annotations are dirty or image changed.
    fn rebuild_overlay_if_dirty(&mut self) {
        let image_key = self.current_image_key();

        // Check if image changed (invalidates cache)
        let image_changed = self.cached_overlay_image_key != image_key;

        // Check if annotations are dirty
        let annotations_dirty = self
            .annotations_map
            .get(&image_key)
            .map(|a| a.is_dirty())
            .unwrap_or(false);

        // Also check drawing state - preview needs updating during drawing
        let drawing_active = self.drawing_state.is_drawing();

        if image_changed || annotations_dirty || drawing_active {
            // Rebuild overlay (pass global categories for color lookup)
            self.cached_overlay = build_overlay(self.annotations(), &self.drawing_state, &self.categories);
            self.cached_overlay_image_key = image_key.clone();

            // Clear dirty flag
            if let Some(store) = self.annotations_map.get_mut(&image_key) {
                store.clear_dirty();
            }
        }
    }

    /// Get the number of bands in the hyperspectral image.
    fn num_bands(&self) -> usize {
        self.hyperspectral_image.num_bands()
    }

    /// Save current settings for the current image (for PerImage mode).
    fn save_current_settings(&mut self) {
        let key = self.current_image_key();

        // Save band selection
        self.persistence.stored_band_selections.insert(key.clone(), self.band_selection);

        // Save image settings
        self.persistence.stored_image_settings.insert(key, self.image_settings);
    }

    /// Apply settings for the new image based on persistence modes.
    fn apply_settings_for_image(&mut self) {
        let key = self.current_image_key();

        // Apply band selection based on mode
        match self.persistence.band_mode {
            PersistenceMode::Reset => {
                // Reset to defaults
                self.band_selection = BandSelection::default_rgb();
            }
            PersistenceMode::PerImage => {
                // Restore stored settings if available, otherwise keep current (for new images)
                if let Some(stored) = self.persistence.stored_band_selections.get(&key) {
                    self.band_selection = *stored;
                }
                // For new images without stored settings, keep current settings as starting point
            }
            PersistenceMode::Constant => {
                // Keep current settings (do nothing)
            }
        }

        // Check if band selection is valid for the current image
        // If any band index is out of bounds, reset to default RGB and warn
        let num_bands = self.hyperspectral_image.num_bands();
        if self.band_selection.red >= num_bands
            || self.band_selection.green >= num_bands
            || self.band_selection.blue >= num_bands
        {
            log::warn!(
                "Band selection ({}, {}, {}) out of range for {}-band image, resetting to default RGB",
                self.band_selection.red,
                self.band_selection.green,
                self.band_selection.blue,
                num_bands
            );
            self.band_selection = BandSelection::default_rgb();
        }

        // Apply image settings based on mode
        match self.persistence.image_settings_mode {
            PersistenceMode::Reset => {
                self.image_settings = ImageSettings::default();
            }
            PersistenceMode::PerImage => {
                // Restore stored settings if available, otherwise keep current (for new images)
                if let Some(stored) = self.persistence.stored_image_settings.get(&key) {
                    self.image_settings = *stored;
                }
                // For new images without stored settings, keep current settings as starting point
            }
            PersistenceMode::Constant => {
                // Keep current settings (do nothing)
            }
        }
    }

    /// Apply a loaded project to the application state.
    fn apply_loaded_project(&mut self, project: crate::project::Project) {
        // Apply annotations
        self.annotations_map.clear();
        for (image_name, mut store) in project.annotations {
            // Mark as dirty since deserialization skips the dirty flag
            store.mark_dirty();
            self.annotations_map.insert(image_name, store);
        }

        // Apply global settings
        self.band_selection = project.settings.band_selection.into();
        self.image_settings = project.settings.image_settings.into();

        // Apply persistence modes
        self.persistence.band_mode = project.settings.band_persistence.into();
        self.persistence.image_settings_mode = project.settings.image_settings_persistence.into();

        // Apply per-image settings
        self.persistence.stored_band_selections.clear();
        self.persistence.stored_image_settings.clear();
        for (image_name, settings) in project.per_image_settings {
            if let Some(band_sel) = settings.band_selection {
                self.persistence.stored_band_selections.insert(image_name.clone(), band_sel.into());
            }
            if let Some(img_set) = settings.image_settings {
                self.persistence.stored_image_settings.insert(image_name, img_set.into());
            }
        }

        // Note: We don't load image files here - the user can load them separately
        // The project just stores filenames for reference
        log::info!(
            "Applied project: {} annotations across {} images",
            self.annotations_map.values().map(|s| s.len()).sum::<usize>(),
            self.annotations_map.len()
        );

        // Force overlay rebuild by invalidating cached key
        self.cached_overlay_image_key = String::new();
        // Rebuild overlay for current image
        self.rebuild_overlay_if_dirty();
    }
}
