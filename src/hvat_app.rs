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
    handle_annotation, handle_counter, handle_image_load, handle_image_settings, handle_image_view,
    handle_navigation, handle_ui, AnnotationState, ImageLoadState,
};
use crate::image_cache::ImageCache;
use crate::message::{Message, Tab};
use crate::theme::Theme;
use crate::views::{view_counter, view_home, view_image_viewer, view_settings};
use crate::widget_state::WidgetState;

#[cfg(target_arch = "wasm32")]
use crate::message::ImageLoadMessage;
#[cfg(target_arch = "wasm32")]
use crate::wasm_file::take_wasm_pending_files;
use hvat_ui::widgets::{button, column, container, row, scrollable, text, Element};
use hvat_ui::{Application, Color, ImageHandle};
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
    /// The current image for the image viewer (either test image or loaded image)
    current_image: ImageHandle,
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

    // === FPS tracking ===
    frame_count: u32,
    last_fps_time: Instant,
    fps: f32,
}

impl Application for HvatApp {
    type Message = Message;

    fn new() -> Self {
        // Create a test image for initial display
        log::info!("Creating 512x512 test image...");
        let test_image = create_test_image(512, 512);
        log::info!("Test image created");

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
            current_image: test_image,
            image_cache: ImageCache::new(1), // Preload 1 image before and after
            current_image_index: 0,
            status_message: None,
            widget_state: WidgetState::new(),
            annotations_map: HashMap::new(),
            drawing_state: DrawingState::new(),
            frame_count: 0,
            last_fps_time: Instant::now(),
            fps: 0.0,
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
                let mut state = ImageLoadState {
                    image_cache: &mut self.image_cache,
                    current_image_index: &mut self.current_image_index,
                    current_image: &mut self.current_image,
                    status_message: &mut self.status_message,
                    zoom: &mut self.zoom,
                    pan_x: &mut self.pan_x,
                    pan_y: &mut self.pan_y,
                };
                handle_image_load(msg, &mut state);
            }
            Message::UI(msg) => {
                handle_ui(
                    msg,
                    &mut self.widget_state,
                    &mut self.show_debug_info,
                    &mut self.theme,
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
            }
            Message::Tick => {
                self.frame_count += 1;
                let elapsed = self.last_fps_time.elapsed();
                if elapsed.as_secs_f32() >= 1.0 {
                    self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
                    self.frame_count = 0;
                    self.last_fps_time = Instant::now();
                }
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
                &self.current_image,
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
            )),
            Tab::Settings => Element::new(view_settings(
                &self.theme,
                text_color,
                self.show_debug_info,
            )),
        };

        // Wrap content in scrollable - use Fill to expand with window
        let scrollable_content = scrollable(content)
            .scroll_offset(self.widget_state.scroll.offset)
            .dragging(self.widget_state.scroll.is_dragging)
            .on_scroll(Message::scroll)
            .on_drag_start(Message::scrollbar_drag_start)
            .on_drag_end(Message::scrollbar_drag_end);

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
}

/// Create a test image with a gradient pattern for demonstration.
fn create_test_image(width: u32, height: u32) -> ImageHandle {
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            // Create a colorful gradient pattern
            let fx = x as f32 / width as f32;
            let fy = y as f32 / height as f32;

            // Create a checkerboard pattern with gradients
            let checker = ((x / 32) + (y / 32)) % 2 == 0;

            let r = if checker {
                (fx * 255.0) as u8
            } else {
                ((1.0 - fx) * 255.0) as u8
            };
            let g = (fy * 255.0) as u8;
            let b = if checker {
                ((fx + fy) / 2.0 * 255.0) as u8
            } else {
                (((1.0 - fx) + (1.0 - fy)) / 2.0 * 255.0) as u8
            };

            // Add some circular pattern in the center
            let cx = (x as f32 - width as f32 / 2.0) / (width as f32 / 2.0);
            let cy = (y as f32 - height as f32 / 2.0) / (height as f32 / 2.0);
            let dist = (cx * cx + cy * cy).sqrt();

            let (r, g, b) = if dist < 0.3 {
                // Inner circle - bright color
                (255, 200, 100)
            } else if dist < 0.5 {
                // Ring
                (100, 150, 255)
            } else {
                (r, g, b)
            };

            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255); // Alpha
        }
    }

    ImageHandle::from_rgba8(data, width, height)
}
