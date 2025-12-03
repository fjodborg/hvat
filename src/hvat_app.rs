// The main HVAT application - shared between native and WASM builds

use crate::annotation::{AnnotationStore, AnnotationTool, Category, DrawingState, Point};
use crate::image_cache::ImageCache;
use crate::widget_state::WidgetState;
use hvat_ui::{
    widgets::*, Application, Color, Element, ImageAdjustments, ImageHandle, Length,
    Overlay, OverlayItem, OverlayShape,
};
use std::path::PathBuf;
use web_time::Instant;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Home,
    Counter,
    ImageViewer,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeChoice {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub struct Theme {
    choice: ThemeChoice,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            choice: ThemeChoice::Dark,
        }
    }

    pub fn light() -> Self {
        Self {
            choice: ThemeChoice::Light,
        }
    }

    pub fn background_color(&self) -> Color {
        match self.choice {
            ThemeChoice::Dark => Color::rgb(0.15, 0.15, 0.15),
            ThemeChoice::Light => Color::rgb(0.95, 0.95, 0.95),
        }
    }

    pub fn text_color(&self) -> Color {
        match self.choice {
            ThemeChoice::Dark => Color::rgb(0.9, 0.9, 0.9),
            ThemeChoice::Light => Color::rgb(0.1, 0.1, 0.1),
        }
    }

    pub fn accent_color(&self) -> Color {
        Color::rgb(0.3, 0.6, 0.9)
    }

    pub fn button_color(&self) -> Color {
        match self.choice {
            ThemeChoice::Dark => Color::rgb(0.25, 0.25, 0.25),
            ThemeChoice::Light => Color::rgb(0.85, 0.85, 0.85),
        }
    }
}

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
    annotations_map: std::collections::HashMap<String, AnnotationStore>,
    /// Current drawing state (tool, in-progress points)
    drawing_state: DrawingState,

    // === FPS tracking ===
    frame_count: u32,
    last_fps_time: Instant,
    fps: f32,
}

// ============================================================================
// Hierarchical Message System
// ============================================================================

/// Messages related to navigation between tabs/views.
#[derive(Debug, Clone)]
pub enum NavigationMessage {
    SwitchTab(Tab),
}

/// Messages for the counter demo.
#[derive(Debug, Clone)]
pub enum CounterMessage {
    Increment,
    Decrement,
    Reset,
}

/// Messages for image viewer controls (pan, zoom, drag).
#[derive(Debug, Clone)]
pub enum ImageViewMessage {
    // Button controls
    ZoomIn,
    ZoomOut,
    ResetView,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    // Widget callbacks
    Pan((f32, f32)),
    /// (new_zoom, cursor_x, cursor_y, widget_center_x, widget_center_y)
    ZoomAtPoint(f32, f32, f32, f32, f32),
    DragStart((f32, f32)),
    DragMove((f32, f32)),
    DragEnd,
}

/// Messages for image manipulation settings (brightness, contrast, etc.).
#[derive(Debug, Clone)]
pub enum ImageSettingsMessage {
    // Slider drag state
    SliderDragStart(SliderId),
    SliderDragEnd,
    // Value changes
    SetBrightness(f32),
    SetContrast(f32),
    SetGamma(f32),
    SetHueShift(f32),
    Reset,
}

/// Messages for image loading and navigation between images.
#[derive(Debug, Clone)]
pub enum ImageLoadMessage {
    LoadFolder,
    FolderLoaded(Vec<PathBuf>),
    NextImage,
    PreviousImage,
    ImageLoaded(ImageHandle),
    #[cfg(target_arch = "wasm32")]
    WasmFilesLoaded(Vec<(String, Vec<u8>)>),
}

/// Messages for UI state (scrolling, debug, theme).
#[derive(Debug, Clone)]
pub enum UIMessage {
    // Scrolling
    Scroll(f32),
    ScrollbarDragStart,
    ScrollbarDragEnd,
    // Settings
    ToggleDebugInfo,
    SetTheme(Theme),
}

/// Messages for annotation tools and operations.
#[derive(Debug, Clone)]
pub enum AnnotationMessage {
    // Tool selection
    SetTool(AnnotationTool),
    // Category management
    SetCategory(u32),
    AddCategory(String),
    // Drawing operations
    StartDrawing(f32, f32),
    ContinueDrawing(f32, f32),
    FinishDrawing,
    /// Force finish polygon (right-click or Enter) - closes polygon regardless of mouse state
    ForceFinishPolygon,
    CancelDrawing,
    // Selection
    SelectAnnotation(Option<u64>),
    DeleteSelected,
    // Import/Export
    ExportJson,
    ImportJson,
    ClearAll,
}

/// Top-level message enum that delegates to sub-message types.
/// This keeps the match arms organized and easier to maintain.
#[derive(Debug, Clone)]
pub enum Message {
    /// Navigation between tabs
    Navigation(NavigationMessage),
    /// Counter demo messages
    Counter(CounterMessage),
    /// Image viewer (pan/zoom/drag)
    ImageView(ImageViewMessage),
    /// Image manipulation settings
    ImageSettings(ImageSettingsMessage),
    /// Image loading and file management
    ImageLoad(ImageLoadMessage),
    /// UI state (scroll, theme, debug)
    UI(UIMessage),
    /// Annotation tools and operations
    Annotation(AnnotationMessage),
    /// FPS tick (called every frame)
    Tick,
}

// ============================================================================
// Convenience constructors for common messages
// ============================================================================

impl Message {
    // Navigation shortcuts
    pub fn switch_tab(tab: Tab) -> Self {
        Message::Navigation(NavigationMessage::SwitchTab(tab))
    }

    // Counter shortcuts
    pub fn increment() -> Self {
        Message::Counter(CounterMessage::Increment)
    }
    pub fn decrement() -> Self {
        Message::Counter(CounterMessage::Decrement)
    }

    // Image view shortcuts
    pub fn zoom_in() -> Self {
        Message::ImageView(ImageViewMessage::ZoomIn)
    }
    pub fn zoom_out() -> Self {
        Message::ImageView(ImageViewMessage::ZoomOut)
    }
    pub fn reset_view() -> Self {
        Message::ImageView(ImageViewMessage::ResetView)
    }
    pub fn image_drag_start(pos: (f32, f32)) -> Self {
        Message::ImageView(ImageViewMessage::DragStart(pos))
    }
    pub fn image_drag_move(pos: (f32, f32)) -> Self {
        Message::ImageView(ImageViewMessage::DragMove(pos))
    }
    pub fn image_drag_end() -> Self {
        Message::ImageView(ImageViewMessage::DragEnd)
    }
    pub fn image_zoom_at_point(new_zoom: f32, cursor_x: f32, cursor_y: f32, cx: f32, cy: f32) -> Self {
        Message::ImageView(ImageViewMessage::ZoomAtPoint(new_zoom, cursor_x, cursor_y, cx, cy))
    }

    // Image settings shortcuts
    pub fn slider_drag_start(id: SliderId) -> Self {
        Message::ImageSettings(ImageSettingsMessage::SliderDragStart(id))
    }
    pub fn slider_drag_end() -> Self {
        Message::ImageSettings(ImageSettingsMessage::SliderDragEnd)
    }
    pub fn set_brightness(v: f32) -> Self {
        Message::ImageSettings(ImageSettingsMessage::SetBrightness(v))
    }
    pub fn set_contrast(v: f32) -> Self {
        Message::ImageSettings(ImageSettingsMessage::SetContrast(v))
    }
    pub fn set_gamma(v: f32) -> Self {
        Message::ImageSettings(ImageSettingsMessage::SetGamma(v))
    }
    pub fn set_hue_shift(v: f32) -> Self {
        Message::ImageSettings(ImageSettingsMessage::SetHueShift(v))
    }
    pub fn reset_image_settings() -> Self {
        Message::ImageSettings(ImageSettingsMessage::Reset)
    }

    // Image load shortcuts
    pub fn load_folder() -> Self {
        Message::ImageLoad(ImageLoadMessage::LoadFolder)
    }
    pub fn next_image() -> Self {
        Message::ImageLoad(ImageLoadMessage::NextImage)
    }
    pub fn previous_image() -> Self {
        Message::ImageLoad(ImageLoadMessage::PreviousImage)
    }

    // UI shortcuts
    pub fn scroll(offset: f32) -> Self {
        Message::UI(UIMessage::Scroll(offset))
    }
    pub fn scrollbar_drag_start() -> Self {
        Message::UI(UIMessage::ScrollbarDragStart)
    }
    pub fn scrollbar_drag_end() -> Self {
        Message::UI(UIMessage::ScrollbarDragEnd)
    }
    pub fn toggle_debug_info() -> Self {
        Message::UI(UIMessage::ToggleDebugInfo)
    }
    pub fn set_theme(theme: Theme) -> Self {
        Message::UI(UIMessage::SetTheme(theme))
    }

    // Annotation shortcuts
    pub fn set_annotation_tool(tool: AnnotationTool) -> Self {
        Message::Annotation(AnnotationMessage::SetTool(tool))
    }
    pub fn set_annotation_category(id: u32) -> Self {
        Message::Annotation(AnnotationMessage::SetCategory(id))
    }
    pub fn start_drawing(x: f32, y: f32) -> Self {
        Message::Annotation(AnnotationMessage::StartDrawing(x, y))
    }
    pub fn continue_drawing(x: f32, y: f32) -> Self {
        Message::Annotation(AnnotationMessage::ContinueDrawing(x, y))
    }
    pub fn finish_drawing() -> Self {
        Message::Annotation(AnnotationMessage::FinishDrawing)
    }
    pub fn force_finish_polygon() -> Self {
        Message::Annotation(AnnotationMessage::ForceFinishPolygon)
    }
    pub fn cancel_drawing() -> Self {
        Message::Annotation(AnnotationMessage::CancelDrawing)
    }
    pub fn delete_selected_annotation() -> Self {
        Message::Annotation(AnnotationMessage::DeleteSelected)
    }
    pub fn export_annotations() -> Self {
        Message::Annotation(AnnotationMessage::ExportJson)
    }
    pub fn clear_annotations() -> Self {
        Message::Annotation(AnnotationMessage::ClearAll)
    }
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
            annotations_map: std::collections::HashMap::new(),
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
            Message::Navigation(msg) => self.handle_navigation(msg),
            Message::Counter(msg) => self.handle_counter(msg),
            Message::ImageView(msg) => self.handle_image_view(msg),
            Message::ImageSettings(msg) => self.handle_image_settings(msg),
            Message::ImageLoad(msg) => self.handle_image_load(msg),
            Message::UI(msg) => self.handle_ui(msg),
            Message::Annotation(msg) => self.handle_annotation(msg),
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
                text("HVAT")
                    .size(20.0)
                    .color(self.theme.accent_color()),
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
        let content = match self.current_tab {
            Tab::Home => self.view_home(text_color),
            Tab::Counter => self.view_counter(text_color),
            Tab::ImageViewer => self.view_image_viewer(text_color),
            Tab::Settings => self.view_settings(text_color),
        };

        // Wrap content in scrollable - use Fill to expand with window
        let scrollable_content = scrollable(Element::new(content))
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
    // ========================================================================
    // Annotation Helpers
    // ========================================================================

    /// Get the current image key for annotation storage.
    fn current_image_key(&self) -> String {
        self.image_cache.get_name(self.current_image_index).unwrap_or_else(|| "default".to_string())
    }

    /// Get annotations for the current image (creates empty store if needed).
    fn annotations(&self) -> &AnnotationStore {
        static EMPTY: std::sync::OnceLock<AnnotationStore> = std::sync::OnceLock::new();
        let key = self.current_image_key();
        self.annotations_map.get(&key).unwrap_or_else(|| EMPTY.get_or_init(AnnotationStore::new))
    }

    /// Get mutable annotations for the current image (creates empty store if needed).
    fn annotations_mut(&mut self) -> &mut AnnotationStore {
        let key = self.current_image_key();
        self.annotations_map.entry(key).or_insert_with(AnnotationStore::new)
    }

    // ========================================================================
    // Message Handlers - Grouped by message category
    // ========================================================================

    fn handle_navigation(&mut self, msg: NavigationMessage) {
        match msg {
            NavigationMessage::SwitchTab(tab) => {
                log::debug!("🔄 Switching to tab: {:?}", tab);
                self.current_tab = tab;
            }
        }
    }

    fn handle_counter(&mut self, msg: CounterMessage) {
        match msg {
            CounterMessage::Increment => {
                self.counter += 1;
                log::debug!("➕ Counter incremented: {}", self.counter);
            }
            CounterMessage::Decrement => {
                self.counter -= 1;
                log::debug!("➖ Counter decremented: {}", self.counter);
            }
            CounterMessage::Reset => {
                self.counter = 0;
                log::debug!("🔄 Counter reset");
            }
        }
    }

    fn handle_image_view(&mut self, msg: ImageViewMessage) {
        match msg {
            ImageViewMessage::ZoomIn => {
                self.zoom = (self.zoom * 1.2).min(5.0);
                log::debug!("🔍 Zoom in: {:.2}x", self.zoom);
            }
            ImageViewMessage::ZoomOut => {
                self.zoom = (self.zoom / 1.2).max(0.2);
                log::debug!("🔍 Zoom out: {:.2}x", self.zoom);
            }
            ImageViewMessage::ResetView => {
                self.zoom = 1.0;
                self.pan_x = 0.0;
                self.pan_y = 0.0;
                log::debug!("🔄 View reset");
            }
            ImageViewMessage::PanLeft => {
                self.pan_x -= 10.0;
                log::debug!("⬅️  Pan left: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            ImageViewMessage::PanRight => {
                self.pan_x += 10.0;
                log::debug!("➡️  Pan right: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            ImageViewMessage::PanUp => {
                self.pan_y -= 10.0;
                log::debug!("⬆️  Pan up: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            ImageViewMessage::PanDown => {
                self.pan_y += 10.0;
                log::debug!("⬇️  Pan down: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            ImageViewMessage::Pan(pan) => {
                self.pan_x = pan.0;
                self.pan_y = pan.1;
            }
            ImageViewMessage::ZoomAtPoint(new_zoom, cursor_x, cursor_y, widget_center_x, widget_center_y) => {
                let old_zoom = self.zoom;
                let cursor_rel_x = cursor_x - widget_center_x;
                let cursor_rel_y = cursor_y - widget_center_y;
                let img_x = (cursor_rel_x - self.pan_x) / old_zoom;
                let img_y = (cursor_rel_y - self.pan_y) / old_zoom;
                self.zoom = new_zoom;
                self.pan_x = cursor_rel_x - img_x * new_zoom;
                self.pan_y = cursor_rel_y - img_y * new_zoom;
                log::debug!("🔍 Zoom-to-cursor: {:.2}x at ({:.1}, {:.1}), pan: ({:.1}, {:.1})",
                    self.zoom, cursor_x, cursor_y, self.pan_x, self.pan_y);
            }
            ImageViewMessage::DragStart(pos) => {
                self.widget_state.image.start_drag(pos);
                log::debug!("Pan drag started at ({:.1}, {:.1})", pos.0, pos.1);
            }
            ImageViewMessage::DragMove(pos) => {
                if let Some((dx, dy)) = self.widget_state.image.update_drag(pos) {
                    self.pan_x += dx;
                    self.pan_y += dy;
                    if dx.abs() > 1.0 || dy.abs() > 1.0 {
                        log::debug!("🖐️ Panning: delta({:.1}, {:.1}) -> pan({:.1}, {:.1})", dx, dy, self.pan_x, self.pan_y);
                    }
                }
            }
            ImageViewMessage::DragEnd => {
                self.widget_state.image.end_drag();
                log::debug!("Pan drag ended");
            }
        }
    }

    fn handle_image_settings(&mut self, msg: ImageSettingsMessage) {
        match msg {
            ImageSettingsMessage::SliderDragStart(id) => {
                self.widget_state.slider.start_drag(id);
                log::debug!("Slider drag started: {:?}", id);
            }
            ImageSettingsMessage::SliderDragEnd => {
                self.widget_state.slider.end_drag();
                log::debug!("Slider drag ended");
            }
            ImageSettingsMessage::SetBrightness(value) => {
                self.brightness = value;
                log::debug!("☀️  Brightness: {:.2}", self.brightness);
            }
            ImageSettingsMessage::SetContrast(value) => {
                self.contrast = value;
                log::debug!("🎛️  Contrast: {:.2}", self.contrast);
            }
            ImageSettingsMessage::SetGamma(value) => {
                self.gamma = value;
                log::debug!("📊 Gamma: {:.2}", self.gamma);
            }
            ImageSettingsMessage::SetHueShift(value) => {
                self.hue_shift = value;
                log::debug!("🎨 Hue shift: {:.2}", self.hue_shift);
            }
            ImageSettingsMessage::Reset => {
                self.brightness = 0.0;
                self.contrast = 1.0;
                self.gamma = 1.0;
                self.hue_shift = 0.0;
                log::debug!("🔄 Image settings reset");
            }
        }
    }

    fn handle_image_load(&mut self, msg: ImageLoadMessage) {
        match msg {
            ImageLoadMessage::LoadFolder => {
                log::info!("📂 Opening folder dialog...");
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        log::info!("📂 Selected folder: {:?}", folder);
                        match self.image_cache.load_from_folder(&folder) {
                            Ok(count) if count > 0 => {
                                self.current_image_index = 0;
                                self.status_message = Some(format!("Loaded {} images", count));
                                log::info!("📂 Found {} images", count);
                                self.load_current_image();
                            }
                            Ok(_) => {
                                self.status_message = Some("No images found in folder".to_string());
                                log::warn!("📂 No images found in folder");
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Error reading folder: {}", e));
                                log::error!("📂 Error reading folder: {}", e);
                            }
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.status_message = Some("Opening file picker...".to_string());
                    log::info!("📂 Opening WASM file picker...");
                    open_wasm_file_picker();
                }
            }
            ImageLoadMessage::FolderLoaded(_paths) => {
                // Deprecated - LoadFolder now handles loading directly
            }
            ImageLoadMessage::NextImage => {
                if !self.image_cache.is_empty() {
                    self.current_image_index = self.image_cache.next_index(self.current_image_index);
                    self.load_current_image();
                    self.zoom = 1.0;
                    self.pan_x = 0.0;
                    self.pan_y = 0.0;
                }
            }
            ImageLoadMessage::PreviousImage => {
                if !self.image_cache.is_empty() {
                    self.current_image_index = self.image_cache.prev_index(self.current_image_index);
                    self.load_current_image();
                    self.zoom = 1.0;
                    self.pan_x = 0.0;
                    self.pan_y = 0.0;
                }
            }
            ImageLoadMessage::ImageLoaded(handle) => {
                self.current_image = handle;
            }
            #[cfg(target_arch = "wasm32")]
            ImageLoadMessage::WasmFilesLoaded(files) => {
                log::info!("📂 WASM: {} files loaded (lazy - not decoded yet)", files.len());
                if files.is_empty() {
                    self.status_message = Some("No files selected".to_string());
                    return;
                }
                let count = self.image_cache.load_from_bytes(files);
                self.current_image_index = 0;
                self.status_message = Some(format!("Loaded {} images", count));
                self.load_current_image();
            }
        }
    }

    fn handle_ui(&mut self, msg: UIMessage) {
        match msg {
            UIMessage::Scroll(offset) => {
                self.widget_state.scroll.set_offset(offset);
                log::debug!("📜 Scroll offset: {:.1}", offset);
            }
            UIMessage::ScrollbarDragStart => {
                self.widget_state.scroll.start_drag();
                log::debug!("📜 Scrollbar drag started");
            }
            UIMessage::ScrollbarDragEnd => {
                self.widget_state.scroll.end_drag();
                log::debug!("📜 Scrollbar drag ended");
            }
            UIMessage::ToggleDebugInfo => {
                self.show_debug_info = !self.show_debug_info;
                log::debug!("🐛 Debug info: {}", if self.show_debug_info { "ON" } else { "OFF" });
            }
            UIMessage::SetTheme(theme) => {
                self.theme = theme.clone();
                log::debug!("🎨 Theme changed to: {:?}", self.theme.choice);
            }
        }
    }

    fn handle_annotation(&mut self, msg: AnnotationMessage) {
        match msg {
            AnnotationMessage::SetTool(tool) => {
                self.drawing_state.tool = tool;
                // Cancel any in-progress drawing when switching tools
                self.drawing_state.cancel();
                log::debug!("🖌️ Annotation tool: {:?}", tool);
            }
            AnnotationMessage::SetCategory(id) => {
                self.drawing_state.current_category = id;
                log::debug!("🏷️ Category: {}", id);
            }
            AnnotationMessage::AddCategory(name) => {
                let id = self.annotations().categories().count() as u32;
                self.annotations_mut().add_category(Category::new(id, name.clone()));
                log::debug!("🏷️ Added category: {} (id={})", name, id);
            }
            AnnotationMessage::StartDrawing(x, y) => {
                match self.drawing_state.tool {
                    AnnotationTool::Select => {
                        // Hit test for selection
                        let hit = self.annotations().hit_test(&Point::new(x, y));
                        self.annotations_mut().select(hit);
                        if let Some(id) = hit {
                            log::debug!("🔍 Selected annotation {}", id);
                        }
                    }
                    AnnotationTool::Point => {
                        // Point tool: create immediately on click
                        let category = self.drawing_state.current_category;
                        let id = self.annotations_mut().add(
                            category,
                            crate::annotation::Shape::Point(Point::new(x, y))
                        );
                        log::info!("✅ Created point annotation {} at ({:.1}, {:.1})", id, x, y);
                    }
                    AnnotationTool::BoundingBox => {
                        // Start drawing for bbox
                        self.drawing_state.start(Point::new(x, y));
                        log::debug!("✏️ Started bbox at ({:.1}, {:.1})", x, y);
                    }
                    AnnotationTool::Polygon => {
                        if self.drawing_state.is_drawing {
                            // Check if clicking near first point to close polygon
                            if self.drawing_state.points.len() >= 3 {
                                if let Some(first) = self.drawing_state.points.first() {
                                    let click_point = Point::new(x, y);
                                    // Close threshold in image coordinates (adjustable)
                                    const CLOSE_THRESHOLD: f32 = 15.0;
                                    if first.distance_to(&click_point) < CLOSE_THRESHOLD / self.zoom {
                                        // Close the polygon
                                        let category = self.drawing_state.current_category;
                                        if let Some(shape) = self.drawing_state.finish() {
                                            let id = self.annotations_mut().add(category, shape);
                                            log::info!("✅ Closed polygon annotation {} (category={})", id, category);
                                        }
                                        return;
                                    }
                                }
                            }
                            // Not closing - add another point
                            self.drawing_state.add_point(Point::new(x, y));
                            log::debug!("✏️ Added polygon point at ({:.1}, {:.1}), total: {}", x, y, self.drawing_state.points.len());
                        } else {
                            // Start new polygon
                            self.drawing_state.start(Point::new(x, y));
                            log::debug!("✏️ Started polygon at ({:.1}, {:.1})", x, y);
                        }
                    }
                }
            }
            AnnotationMessage::ContinueDrawing(x, y) => {
                if self.drawing_state.is_drawing {
                    match self.drawing_state.tool {
                        AnnotationTool::BoundingBox => {
                            // For bbox, we need exactly 2 points - add second if missing, else update
                            if self.drawing_state.points.len() == 1 {
                                self.drawing_state.add_point(Point::new(x, y));
                            } else {
                                self.drawing_state.update_last(Point::new(x, y));
                            }
                        }
                        AnnotationTool::Polygon => {
                            // For polygon, we don't add points on move - only on click
                            // (ContinueDrawing for polygon should just update preview)
                        }
                        _ => {}
                    }
                }
            }
            AnnotationMessage::FinishDrawing => {
                // This is called on mouse release - only finish bbox, NOT polygon
                // Polygon is finished via ForceFinishPolygon (right-click/Enter) or clicking first point
                match self.drawing_state.tool {
                    AnnotationTool::BoundingBox => {
                        let category = self.drawing_state.current_category;
                        if let Some(shape) = self.drawing_state.finish() {
                            let id = self.annotations_mut().add(category, shape);
                            log::info!("✅ Created bbox annotation {} (category={})", id, category);
                        }
                    }
                    AnnotationTool::Polygon => {
                        // Do nothing on mouse release for polygon - keep drawing
                        log::debug!("📝 Polygon continues (use right-click or click first point to close)");
                    }
                    _ => {
                        // Point tool handles creation in StartDrawing, Select doesn't create
                    }
                }
            }
            AnnotationMessage::ForceFinishPolygon => {
                // Called via right-click or Enter key - force close polygon if valid
                if self.drawing_state.tool == AnnotationTool::Polygon && self.drawing_state.is_drawing {
                    if self.drawing_state.points.len() >= 3 {
                        let category = self.drawing_state.current_category;
                        if let Some(shape) = self.drawing_state.finish() {
                            let id = self.annotations_mut().add(category, shape);
                            log::info!("✅ Created polygon annotation {} (category={})", id, category);
                        }
                    } else {
                        log::debug!("📝 Polygon needs at least 3 points, currently has {}", self.drawing_state.points.len());
                    }
                }
            }
            AnnotationMessage::CancelDrawing => {
                self.drawing_state.cancel();
                log::debug!("❌ Drawing cancelled");
            }
            AnnotationMessage::SelectAnnotation(id) => {
                self.annotations_mut().select(id);
                log::debug!("🔍 Selected annotation: {:?}", id);
            }
            AnnotationMessage::DeleteSelected => {
                if let Some(id) = self.annotations().selected() {
                    self.annotations_mut().remove(id);
                    log::info!("🗑️ Deleted annotation {}", id);
                }
            }
            AnnotationMessage::ExportJson => {
                match self.annotations().to_json() {
                    Ok(json) => {
                        log::info!("📤 Exported {} annotations to JSON", self.annotations().len());
                        // In a real app, we'd save to file or clipboard
                        // For now, just log a preview
                        if json.len() > 200 {
                            log::debug!("JSON preview: {}...", &json[..200]);
                        } else {
                            log::debug!("JSON: {}", json);
                        }
                        self.status_message = Some(format!("Exported {} annotations", self.annotations().len()));
                    }
                    Err(e) => {
                        log::error!("Failed to export JSON: {}", e);
                        self.status_message = Some(format!("Export failed: {}", e));
                    }
                }
            }
            AnnotationMessage::ImportJson => {
                // TODO: Implement file picker for importing
                log::info!("📥 Import not yet implemented");
                self.status_message = Some("Import not yet implemented".to_string());
            }
            AnnotationMessage::ClearAll => {
                let count = self.annotations().len();
                self.annotations_mut().clear();
                log::info!("🗑️ Cleared {} annotations", count);
                self.status_message = Some(format!("Cleared {} annotations", count));
            }
        }
    }

    // ========================================================================
    // View Methods
    // ========================================================================

    fn view_home(&self, text_color: Color) -> Column<'static, Message> {
        column()
            .push(Element::new(
                text("Welcome to HVAT")
                    .size(28.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("A GPU-accelerated hyperspectral image annotation tool")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("Features:")
                    .size(16.0)
                    .color(self.theme.accent_color()),
            ))
            .push(Element::new(
                text("• Fast GPU rendering with wgpu")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("• Cross-platform (native + WASM)")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("• Pan and zoom")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("• Custom UI framework")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("Navigate using the tabs above to explore features")
                    .size(14.0)
                    .color(self.theme.accent_color()),
            ))
            .spacing(20.0)
    }

    fn view_counter(&self, text_color: Color) -> Column<'static, Message> {
        column()
            .push(Element::new(
                text("Counter Demo")
                    .size(24.0)
                    .color(text_color),
            ))
            .push(Element::new(
                container(Element::new(
                    text(format!("{}", self.counter))
                        .size(48.0)
                        .color(self.theme.accent_color()),
                ))
                .padding(20.0),
            ))
            .push(Element::new(
                row()
                    .push(Element::new(
                        button("Increment")
                            .on_press(Message::increment())
                            .width(150.0),
                    ))
                    .push(Element::new(
                        button("Decrement")
                            .on_press(Message::decrement())
                            .width(150.0),
                    ))
                    .push(Element::new(
                        button("Reset")
                            .on_press(Message::Counter(CounterMessage::Reset))
                            .width(150.0),
                    ))
                    .spacing(15.0),
            ))
            .spacing(30.0)
    }

    /// Build an overlay from the current annotations and drawing state.
    fn build_overlay(&self) -> Overlay {
        let mut overlay = Overlay::new();

        // Add all annotations
        let annotations = self.annotations();
        for ann in annotations.iter() {
            // Get category color
            let cat_color = annotations
                .get_category(ann.category_id)
                .map(|c| Color::new(c.color[0], c.color[1], c.color[2], c.color[3]))
                .unwrap_or(Color::rgb(0.7, 0.7, 0.7));

            let shape = match &ann.shape {
                crate::annotation::Shape::Point(p) => OverlayShape::Point {
                    x: p.x,
                    y: p.y,
                    radius: 6.0,
                },
                crate::annotation::Shape::BoundingBox(b) => OverlayShape::Rect {
                    x: b.x,
                    y: b.y,
                    width: b.width,
                    height: b.height,
                },
                crate::annotation::Shape::Polygon(poly) => OverlayShape::Polygon {
                    vertices: poly.vertices.iter().map(|p| (p.x, p.y)).collect(),
                    closed: poly.closed,
                },
            };

            let selected = annotations.selected() == Some(ann.id);
            overlay.push(OverlayItem::new(shape, cat_color).selected(selected));
        }

        // Add preview for in-progress drawing
        if let Some(preview_shape) = self.drawing_state.preview() {
            let cat_color = annotations
                .get_category(self.drawing_state.current_category)
                .map(|c| Color::new(c.color[0], c.color[1], c.color[2], 0.5))
                .unwrap_or(Color::new(0.7, 0.7, 0.7, 0.5));

            let shape = match preview_shape {
                crate::annotation::Shape::Point(p) => OverlayShape::Point {
                    x: p.x,
                    y: p.y,
                    radius: 6.0,
                },
                crate::annotation::Shape::BoundingBox(b) => OverlayShape::Rect {
                    x: b.x,
                    y: b.y,
                    width: b.width,
                    height: b.height,
                },
                crate::annotation::Shape::Polygon(poly) => OverlayShape::Polygon {
                    vertices: poly.vertices.iter().map(|p| (p.x, p.y)).collect(),
                    closed: false, // Preview is always open
                },
            };

            overlay.set_preview(Some(OverlayItem::new(shape, cat_color)));
        }

        overlay
    }

    fn view_image_viewer(&self, text_color: Color) -> Column<'_, Message> {
        // Create image adjustments from current settings
        let adjustments = ImageAdjustments {
            brightness: self.brightness,
            contrast: self.contrast,
            gamma: self.gamma,
            hue_shift: self.hue_shift,
        };

        // Build the annotation overlay
        let overlay = self.build_overlay();

        // Create the pan/zoom image widget
        let image_widget = pan_zoom_image(self.current_image.clone())
            .pan((self.pan_x, self.pan_y))
            .zoom(self.zoom)
            .dragging(self.widget_state.image.is_dragging)
            .drawing(self.drawing_state.is_drawing)
            .adjustments(adjustments)
            .overlay(overlay)
            .width(Length::Units(600.0))
            .height(Length::Units(400.0))
            .on_drag_start(Message::image_drag_start)
            .on_drag_move(Message::image_drag_move)
            .on_drag_end(Message::image_drag_end)
            .on_zoom(Message::image_zoom_at_point)
            // Annotation callbacks
            .on_click(|(x, y)| Message::start_drawing(x, y))
            .on_draw_move(|(x, y)| Message::continue_drawing(x, y))
            .on_draw_end(Message::finish_drawing)
            .on_space(Message::force_finish_polygon);

        // Status text
        let status_text = self.status_message.as_deref().unwrap_or("No images loaded");

        column()
            .push(Element::new(
                text("Image Viewer")
                    .size(24.0)
                    .color(text_color),
            ))
            // File loading controls
            .push(Element::new(
                row()
                    .push(Element::new(
                        button("Load Folder")
                            .on_press(Message::load_folder())
                            .width(120.0),
                    ))
                    .push(Element::new(
                        button("< Prev")
                            .on_press(Message::previous_image())
                            .width(80.0),
                    ))
                    .push(Element::new(
                        button("Next >")
                            .on_press(Message::next_image())
                            .width(80.0),
                    ))
                    .push(Element::new(
                        text(status_text)
                            .size(12.0)
                            .color(text_color),
                    ))
                    .spacing(10.0),
            ))
            .push(Element::new(
                text(format!("Zoom: {:.2}x | Pan: ({:.0}, {:.0})", self.zoom, self.pan_x, self.pan_y))
                    .size(14.0)
                    .color(text_color),
            ))
            // Image display area with border
            .push(Element::new(
                container(Element::new(image_widget))
                    .padding(4.0)
                    .border(Color::rgb(0.4, 0.4, 0.4))
                    .border_width(2.0),
            ))
            .push(Element::new(
                text("Middle-click drag to pan, scroll to zoom")
                    .size(12.0)
                    .color(Color::rgb(0.6, 0.6, 0.6)),
            ))
            // Zoom/pan button controls
            .push(Element::new(
                row()
                    .push(Element::new(
                        button("Zoom In")
                            .on_press(Message::zoom_in())
                            .width(90.0),
                    ))
                    .push(Element::new(
                        button("Zoom Out")
                            .on_press(Message::zoom_out())
                            .width(90.0),
                    ))
                    .push(Element::new(
                        button("Reset View")
                            .on_press(Message::reset_view())
                            .width(90.0),
                    ))
                    .spacing(10.0),
            ))
            // Image manipulation controls with sliders
            .push(Element::new(
                text("Image Settings:")
                    .size(14.0)
                    .color(self.theme.accent_color()),
            ))
            // Brightness slider
            .push(Element::new(
                row()
                    .push(Element::new(
                        text(format!("Brightness: {:.2}", self.brightness))
                            .size(12.0)
                            .color(text_color),
                    ))
                    .push(Element::new(
                        slider(-1.0, 1.0, self.brightness)
                            .id(SliderId::Brightness)
                            .dragging(self.widget_state.slider.is_dragging(SliderId::Brightness))
                            .width(Length::Units(200.0))
                            .on_drag_start(Message::slider_drag_start)
                            .on_change(Message::set_brightness)
                            .on_drag_end(Message::slider_drag_end),
                    ))
                    .spacing(10.0),
            ))
            // Contrast slider
            .push(Element::new(
                row()
                    .push(Element::new(
                        text(format!("Contrast:   {:.2}", self.contrast))
                            .size(12.0)
                            .color(text_color),
                    ))
                    .push(Element::new(
                        slider(0.1, 3.0, self.contrast)
                            .id(SliderId::Contrast)
                            .dragging(self.widget_state.slider.is_dragging(SliderId::Contrast))
                            .width(Length::Units(200.0))
                            .on_drag_start(Message::slider_drag_start)
                            .on_change(Message::set_contrast)
                            .on_drag_end(Message::slider_drag_end),
                    ))
                    .spacing(10.0),
            ))
            // Gamma slider
            .push(Element::new(
                row()
                    .push(Element::new(
                        text(format!("Gamma:      {:.2}", self.gamma))
                            .size(12.0)
                            .color(text_color),
                    ))
                    .push(Element::new(
                        slider(0.1, 3.0, self.gamma)
                            .id(SliderId::Gamma)
                            .dragging(self.widget_state.slider.is_dragging(SliderId::Gamma))
                            .width(Length::Units(200.0))
                            .on_drag_start(Message::slider_drag_start)
                            .on_change(Message::set_gamma)
                            .on_drag_end(Message::slider_drag_end),
                    ))
                    .spacing(10.0),
            ))
            // Hue shift slider
            .push(Element::new(
                row()
                    .push(Element::new(
                        text(format!("Hue Shift:  {:.0}", self.hue_shift))
                            .size(12.0)
                            .color(text_color),
                    ))
                    .push(Element::new(
                        slider(-180.0, 180.0, self.hue_shift)
                            .id(SliderId::HueShift)
                            .dragging(self.widget_state.slider.is_dragging(SliderId::HueShift))
                            .width(Length::Units(200.0))
                            .on_drag_start(Message::slider_drag_start)
                            .on_change(Message::set_hue_shift)
                            .on_drag_end(Message::slider_drag_end),
                    ))
                    .spacing(10.0),
            ))
            // Reset button
            .push(Element::new(
                button("Reset Image Settings")
                    .on_press(Message::reset_image_settings())
                    .width(180.0),
            ))
            // Annotation toolbar
            .push(Element::new(
                text("Annotation Tools:")
                    .size(14.0)
                    .color(self.theme.accent_color()),
            ))
            .push(Element::new(self.view_annotation_toolbar(text_color)))
            // Annotation info
            .push(Element::new(
                text(format!(
                    "Annotations: {} | Tool: {:?} | {}",
                    self.annotations().len(),
                    self.drawing_state.tool,
                    if self.drawing_state.is_drawing { "Drawing..." } else { "Ready" }
                ))
                .size(12.0)
                .color(text_color),
            ))
            .spacing(8.0)
    }

    fn view_annotation_toolbar(&self, _text_color: Color) -> Row<'static, Message> {
        let tool = self.drawing_state.tool;

        // Tool selection buttons with visual indication of active tool
        let select_btn = if tool == AnnotationTool::Select {
            button("Select *").on_press(Message::set_annotation_tool(AnnotationTool::Select)).width(80.0)
        } else {
            button("Select").on_press(Message::set_annotation_tool(AnnotationTool::Select)).width(80.0)
        };

        let bbox_btn = if tool == AnnotationTool::BoundingBox {
            button("BBox *").on_press(Message::set_annotation_tool(AnnotationTool::BoundingBox)).width(80.0)
        } else {
            button("BBox").on_press(Message::set_annotation_tool(AnnotationTool::BoundingBox)).width(80.0)
        };

        let poly_btn = if tool == AnnotationTool::Polygon {
            button("Polygon *").on_press(Message::set_annotation_tool(AnnotationTool::Polygon)).width(80.0)
        } else {
            button("Polygon").on_press(Message::set_annotation_tool(AnnotationTool::Polygon)).width(80.0)
        };

        let point_btn = if tool == AnnotationTool::Point {
            button("Point *").on_press(Message::set_annotation_tool(AnnotationTool::Point)).width(80.0)
        } else {
            button("Point").on_press(Message::set_annotation_tool(AnnotationTool::Point)).width(80.0)
        };

        row()
            .push(Element::new(select_btn))
            .push(Element::new(bbox_btn))
            .push(Element::new(poly_btn))
            .push(Element::new(point_btn))
            .push(Element::new(
                button("Delete")
                    .on_press(Message::delete_selected_annotation())
                    .width(70.0),
            ))
            .push(Element::new(
                button("Export")
                    .on_press(Message::export_annotations())
                    .width(70.0),
            ))
            .push(Element::new(
                button("Clear")
                    .on_press(Message::clear_annotations())
                    .width(60.0),
            ))
            .spacing(5.0)
    }

    fn view_settings(&self, text_color: Color) -> Column<'static, Message> {
        column()
            .push(Element::new(
                text("Settings")
                    .size(24.0)
                    .color(text_color),
            ))
            .push(Element::new(
                container(Element::new(
                    column()
                        .push(Element::new(
                            text("Theme")
                                .size(16.0)
                                .color(self.theme.accent_color()),
                        ))
                        .push(Element::new(
                            row()
                                .push(Element::new(
                                    button("Dark Theme")
                                        .on_press(Message::set_theme(Theme::dark()))
                                        .width(120.0),
                                ))
                                .push(Element::new(
                                    button("Light Theme")
                                        .on_press(Message::set_theme(Theme::light()))
                                        .width(120.0),
                                ))
                                .spacing(10.0),
                        ))
                        .spacing(15.0),
                ))
                .padding(20.0),
            ))
            .push(Element::new(
                container(Element::new(
                    column()
                        .push(Element::new(
                            text("Debug")
                                .size(16.0)
                                .color(self.theme.accent_color()),
                        ))
                        .push(Element::new(
                            button(if self.show_debug_info {
                                "Hide Debug Info"
                            } else {
                                "Show Debug Info"
                            })
                            .on_press(Message::toggle_debug_info())
                            .width(150.0),
                        ))
                        .spacing(15.0),
                ))
                .padding(20.0),
            ))
            .spacing(20.0)
    }

    /// Load the current image using the unified image cache (works on both native and WASM).
    fn load_current_image(&mut self) {
        let index = self.current_image_index;

        // Load the current image
        if let Some(handle) = self.image_cache.get_or_load(index) {
            self.current_image = handle;

            // Update status message
            let name = self.image_cache.get_name(index).unwrap_or_default();
            self.status_message = Some(format!(
                "Image {}/{}: {}",
                index + 1,
                self.image_cache.len(),
                name
            ));
        } else {
            self.status_message = Some("Failed to load image".to_string());
        }

        // Preload adjacent images
        self.image_cache.preload_adjacent(index);
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

// ============================================================================
// WASM File Loading
// ============================================================================

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Global storage for loaded files (raw bytes, not decoded) - the app polls this via tick()
    static WASM_PENDING_FILES: RefCell<Option<Vec<(String, Vec<u8>)>>> = const { RefCell::new(None) };
}

/// Check if there are pending files loaded from WASM file picker
#[cfg(target_arch = "wasm32")]
pub fn take_wasm_pending_files() -> Option<Vec<(String, Vec<u8>)>> {
    WASM_PENDING_FILES.with(|pending| pending.borrow_mut().take())
}

#[cfg(target_arch = "wasm32")]
fn set_wasm_pending_files(files: Vec<(String, Vec<u8>)>) {
    WASM_PENDING_FILES.with(|pending| {
        *pending.borrow_mut() = Some(files);
    });
}

#[cfg(target_arch = "wasm32")]
pub fn open_wasm_file_picker() {
    use web_sys::{Document, Event, FileReader, HtmlInputElement};

    let window = web_sys::window().expect("no window");
    let document: Document = window.document().expect("no document");

    // Create a hidden file input element
    let input: HtmlInputElement = document
        .create_element("input")
        .expect("failed to create input")
        .dyn_into()
        .expect("not an input element");

    input.set_type("file");
    input.set_accept("image/*");
    input.set_multiple(true);

    // Enable folder selection using webkitdirectory attribute
    // This is widely supported (Chrome, Edge, Firefox, Safari)
    input.set_attribute("webkitdirectory", "").expect("failed to set webkitdirectory");
    input.set_attribute("directory", "").expect("failed to set directory"); // Firefox fallback

    // Store raw file bytes as they load (lazy loading - no decoding here)
    let results: Rc<RefCell<Vec<(String, Vec<u8>)>>> = Rc::new(RefCell::new(Vec::new()));
    let total_files: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
    let loaded_files: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));

    // Clone for closure
    let results_clone = results.clone();
    let total_clone = total_files.clone();
    let loaded_clone = loaded_files.clone();

    // Handle file selection
    let onchange = Closure::wrap(Box::new(move |event: Event| {
        let input: HtmlInputElement = event
            .target()
            .expect("no target")
            .dyn_into()
            .expect("not input");

        if let Some(files) = input.files() {
            let count = files.length();
            if count == 0 {
                log::warn!("📂 No files selected");
                return;
            }

            // Filter to only image files using the centralized check
            let mut image_files = Vec::new();
            for i in 0..count {
                if let Some(file) = files.get(i) {
                    let name = file.name();
                    if crate::image_cache::is_image_file(&name) {
                        image_files.push(file);
                    }
                }
            }

            if image_files.is_empty() {
                log::warn!("📂 No image files found");
                set_wasm_pending_files(Vec::new());
                return;
            }

            let image_count = image_files.len();

            // Show warning for large folders (> 50 images)
            const LARGE_FOLDER_THRESHOLD: usize = 50;
            if image_count > LARGE_FOLDER_THRESHOLD {
                let window = web_sys::window().expect("no window");
                let message = format!(
                    "Warning: You selected {} images.\n\n\
                    Loading many images in the browser can use significant memory.\n\n\
                    For large datasets, the native desktop application is recommended.\n\n\
                    Continue anyway?",
                    image_count
                );

                // Use confirm() dialog - returns true if user clicks OK
                let confirmed = window.confirm_with_message(&message).unwrap_or(false);
                if !confirmed {
                    log::info!("📂 User cancelled loading {} images", image_count);
                    set_wasm_pending_files(Vec::new());
                    return;
                }
                log::info!("📂 User confirmed loading {} images", image_count);
            }

            *total_clone.borrow_mut() = image_count;
            log::info!("📂 Found {} image files (lazy loading - will decode on demand)", image_count);

            for file in image_files {
                let name = file.name();
                log::info!("📂 Reading file: {}", name);

                let reader = FileReader::new().expect("failed to create FileReader");

                let results_inner = results_clone.clone();
                let loaded_inner = loaded_clone.clone();
                let total_inner = total_clone.clone();
                let name_clone = name.clone();

                // Handle load complete - store raw bytes, no decoding
                let onload = Closure::wrap(Box::new(move |event: Event| {
                    let reader: FileReader = event
                        .target()
                        .expect("no target")
                        .dyn_into()
                        .expect("not FileReader");

                    if let Ok(result) = reader.result() {
                        let array = js_sys::Uint8Array::new(&result);
                        let bytes = array.to_vec();

                        log::info!("📂 File {} read: {} bytes (not decoded yet)", name_clone, bytes.len());

                        // Store raw bytes - decoding happens lazily when viewing
                        results_inner
                            .borrow_mut()
                            .push((name_clone.clone(), bytes));
                    }

                    // Check if all files loaded
                    *loaded_inner.borrow_mut() += 1;
                    let loaded = *loaded_inner.borrow();
                    let total = *total_inner.borrow();

                    if loaded >= total {
                        log::info!("📂 All {} files read (will decode on demand)", total);
                        let files = results_inner.borrow().clone();
                        set_wasm_pending_files(files);
                    }
                }) as Box<dyn FnMut(Event)>);

                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget(); // Leak the closure to keep it alive

                // Read as array buffer
                reader.read_as_array_buffer(&file).expect("failed to read");
            }
        }
    }) as Box<dyn FnMut(Event)>);

    input.set_onchange(Some(onchange.as_ref().unchecked_ref()));
    onchange.forget(); // Leak the closure to keep it alive

    // Trigger the file picker
    input.click();
}
