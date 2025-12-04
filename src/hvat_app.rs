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
use crate::handlers::{
    handle_annotation, handle_band, handle_counter, handle_image_load, handle_image_settings,
    handle_image_view, handle_navigation, handle_ui, AnnotationState, ImageLoadState,
};
use crate::hyperspectral::{generate_test_hyperspectral, BandSelection, HyperspectralImage};
use crate::image_cache::ImageCache;
use crate::message::{Message, PersistenceMode, Tab};
use crate::theme::Theme;
use crate::views::{build_overlay, view_counter, view_home, view_image_viewer, view_settings};
use crate::widget_state::WidgetState;
use hvat_ui::Overlay;

#[cfg(target_arch = "wasm32")]
use crate::message::ImageLoadMessage;
#[cfg(target_arch = "wasm32")]
use crate::wasm_file::take_wasm_pending_files;
use hvat_ui::widgets::{button, column, container, row, scrollable, text, Element, ScrollDirection};
use hvat_ui::{Application, BandSelectionUniform, Color, HyperspectralImageHandle, ImageHandle};
use std::collections::HashMap;
use web_time::Instant;

/// Main application state.
pub struct HvatApp {
    // === Navigation ===
    current_tab: Tab,

    // === Counter demo ===
    counter: i32,

    // === Image viewer - transform state ===
    zoom: f32,
    pan_x: f32,
    pan_y: f32,

    // === Image manipulation settings ===
    brightness: f32,
    contrast: f32,
    gamma: f32,
    hue_shift: f32,

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
    /// Current drawing state (tool, in-progress points)
    drawing_state: DrawingState,
    /// Cached overlay - rebuilt only when annotations or drawing state changes
    cached_overlay: Overlay,
    /// Last image key used for cached overlay (invalidate on image change)
    cached_overlay_image_key: String,

    // === FPS tracking ===
    frame_count: u32,
    last_fps_time: Instant,
    fps: f32,

    // === Persistence settings ===
    /// How band selection should persist across image navigation
    band_persistence: PersistenceMode,
    /// How image settings (brightness, contrast, etc.) should persist
    image_settings_persistence: PersistenceMode,
    /// Stored band selections per image (for PerImage mode)
    stored_band_selections: HashMap<String, BandSelection>,
    /// Stored image settings per image (for PerImage mode)
    stored_image_settings: HashMap<String, ImageSettings>,
}

/// Stored image manipulation settings for per-image persistence.
#[derive(Clone, Copy, Debug)]
pub struct ImageSettings {
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub hue_shift: f32,
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
        // Create a test hyperspectral image (8 bands)
        log::info!("Creating 4096x4096 test hyperspectral image (8 bands)...");
        let width = 4096;
        let height = 4096;
        let hyper_image = generate_test_hyperspectral(width, height, 8);
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
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            hue_shift: 0.0,
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
            drawing_state: DrawingState::new(),
            cached_overlay: Overlay::new(),
            cached_overlay_image_key: String::new(),
            frame_count: 0,
            last_fps_time: Instant::now(),
            fps: 0.0,
            band_persistence: PersistenceMode::default(),
            image_settings_persistence: PersistenceMode::default(),
            stored_band_selections: HashMap::new(),
            stored_image_settings: HashMap::new(),
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
                handle_image_view(
                    msg,
                    &mut self.zoom,
                    &mut self.pan_x,
                    &mut self.pan_y,
                    &mut self.widget_state,
                );
            }
            Message::ImageSettings(msg) => {
                handle_image_settings(
                    msg,
                    &mut self.brightness,
                    &mut self.contrast,
                    &mut self.gamma,
                    &mut self.hue_shift,
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
                    zoom: &mut self.zoom,
                    pan_x: &mut self.pan_x,
                    pan_y: &mut self.pan_y,
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
                handle_ui(
                    msg,
                    &mut self.widget_state,
                    &mut self.show_debug_info,
                    &mut self.theme,
                    &mut self.band_persistence,
                    &mut self.image_settings_persistence,
                );
            }
            Message::Annotation(msg) => {
                let image_key = self.current_image_key();
                let mut state = AnnotationState {
                    annotations_map: &mut self.annotations_map,
                    drawing_state: &mut self.drawing_state,
                    image_key,
                    zoom: self.zoom,
                    status_message: &mut self.status_message,
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
            Message::Tick => {
                self.frame_count += 1;
                let elapsed = self.last_fps_time.elapsed();
                if elapsed.as_secs_f32() >= 1.0 {
                    self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
                    self.frame_count = 0;
                    self.last_fps_time = Instant::now();
                }
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
                text(format!("FPS: {:.0}", self.fps))
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
                self.zoom,
                self.pan_x,
                self.pan_y,
                self.brightness,
                self.contrast,
                self.gamma,
                self.hue_shift,
                &self.widget_state,
                &self.drawing_state,
                self.annotations(),
                self.status_message.as_deref(),
                &self.band_selection,
                self.num_bands(),
                self.cached_overlay.clone(),
                self.band_persistence,
                self.image_settings_persistence,
            )),
            Tab::Settings => Element::new(view_settings(
                &self.theme,
                text_color,
                self.show_debug_info,
            )),
        };

        // Wrap content in scrollable - supports both vertical and horizontal scrolling
        let scrollable_content = scrollable(content)
            .direction(ScrollDirection::Both)
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

        Element::new(
            container(Element::new(
                column()
                    .push(Element::new(header))
                    .push(Element::new(scrollable_content))
                    .spacing(20.0),
            ))
            .padding(30.0)
            .fill(),
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

    /// Get mutable annotations for the current image.
    fn annotations_mut(&mut self) -> &mut AnnotationStore {
        let key = self.current_image_key();
        self.annotations_map
            .entry(key)
            .or_insert_with(AnnotationStore::new)
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
        let drawing_active = self.drawing_state.is_drawing;

        if image_changed || annotations_dirty || drawing_active {
            // Rebuild overlay
            self.cached_overlay = build_overlay(self.annotations(), &self.drawing_state);
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
        self.stored_band_selections.insert(key.clone(), self.band_selection);

        // Save image settings
        self.stored_image_settings.insert(key, ImageSettings {
            brightness: self.brightness,
            contrast: self.contrast,
            gamma: self.gamma,
            hue_shift: self.hue_shift,
        });
    }

    /// Apply settings for the new image based on persistence modes.
    fn apply_settings_for_image(&mut self) {
        let key = self.current_image_key();

        // Apply band selection based on mode
        match self.band_persistence {
            PersistenceMode::Reset => {
                // Reset to defaults
                self.band_selection = BandSelection::default_rgb();
            }
            PersistenceMode::PerImage => {
                // Restore stored settings if available, otherwise keep current (for new images)
                if let Some(stored) = self.stored_band_selections.get(&key) {
                    self.band_selection = *stored;
                }
                // For new images without stored settings, keep current settings as starting point
            }
            PersistenceMode::Constant => {
                // Keep current settings (do nothing)
            }
        }

        // Apply image settings based on mode
        match self.image_settings_persistence {
            PersistenceMode::Reset => {
                self.brightness = 0.0;
                self.contrast = 1.0;
                self.gamma = 1.0;
                self.hue_shift = 0.0;
            }
            PersistenceMode::PerImage => {
                // Restore stored settings if available, otherwise keep current (for new images)
                if let Some(stored) = self.stored_image_settings.get(&key) {
                    self.brightness = stored.brightness;
                    self.contrast = stored.contrast;
                    self.gamma = stored.gamma;
                    self.hue_shift = stored.hue_shift;
                }
                // For new images without stored settings, keep current settings as starting point
            }
            PersistenceMode::Constant => {
                // Keep current settings (do nothing)
            }
        }
    }
}
