//! Hierarchical message system for HVAT application.
//!
//! Messages are organized into categories for maintainability:
//! - NavigationMessage: Tab switching
//! - CounterMessage: Counter demo
//! - ImageViewMessage: Pan/zoom/drag
//! - ImageSettingsMessage: Brightness/contrast/gamma/hue
//! - ImageLoadMessage: File loading
//! - UIMessage: Scroll/theme/debug
//! - AnnotationMessage: Annotation tools and operations

use crate::annotation::AnnotationTool;

/// Persistence mode for settings (bands, image adjustments) across image navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PersistenceMode {
    /// Reset to defaults when switching images
    Reset,
    /// Store/restore settings per image (each image remembers its own settings)
    /// For new images without stored settings, uses current settings as starting point
    PerImage,
    /// Keep current settings constant across all images
    #[default]
    Constant,
}

use crate::theme::Theme;
use hvat_ui::widgets::SliderId;
use hvat_ui::ImageHandle;
use std::path::PathBuf;

/// Application tabs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Home,
    Counter,
    ImageViewer,
    Settings,
}

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
    // Vertical scrolling
    ScrollY(f32),
    ScrollbarDragStartY,
    ScrollbarDragEndY,
    // Horizontal scrolling
    ScrollX(f32),
    ScrollbarDragStartX,
    ScrollbarDragEndX,
    // Settings
    ToggleDebugInfo,
    SetTheme(Theme),
    // Persistence modes for settings across image navigation
    SetBandPersistence(PersistenceMode),
    SetImageSettingsPersistence(PersistenceMode),
}

/// Messages for hyperspectral band selection.
#[derive(Debug, Clone)]
pub enum BandMessage {
    /// Set the red channel band index (during drag - doesn't regenerate composite)
    SetRedBand(usize),
    /// Set the green channel band index (during drag - doesn't regenerate composite)
    SetGreenBand(usize),
    /// Set the blue channel band index (during drag - doesn't regenerate composite)
    SetBlueBand(usize),
    /// Start dragging red band slider with initial value (starts drag state AND sets value)
    StartRedBand(usize),
    /// Start dragging green band slider with initial value (starts drag state AND sets value)
    StartGreenBand(usize),
    /// Start dragging blue band slider with initial value (starts drag state AND sets value)
    StartBlueBand(usize),
    /// Apply band changes and regenerate composite (called on drag end)
    ApplyBands,
    /// Reset to default RGB (bands 0, 1, 2)
    ResetBands,
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
    /// Force finish polygon (Space key) - closes polygon regardless of mouse state
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
    /// Hyperspectral band selection
    Band(BandMessage),
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
    /// Slider drag start - ignores the initial value (used for non-band sliders)
    pub fn slider_drag_start(id: SliderId, _value: f32) -> Self {
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

    // UI shortcuts - vertical scrolling
    pub fn scroll_y(offset: f32) -> Self {
        Message::UI(UIMessage::ScrollY(offset))
    }
    pub fn scrollbar_drag_start_y() -> Self {
        Message::UI(UIMessage::ScrollbarDragStartY)
    }
    pub fn scrollbar_drag_end_y() -> Self {
        Message::UI(UIMessage::ScrollbarDragEndY)
    }
    // UI shortcuts - horizontal scrolling
    pub fn scroll_x(offset: f32) -> Self {
        Message::UI(UIMessage::ScrollX(offset))
    }
    pub fn scrollbar_drag_start_x() -> Self {
        Message::UI(UIMessage::ScrollbarDragStartX)
    }
    pub fn scrollbar_drag_end_x() -> Self {
        Message::UI(UIMessage::ScrollbarDragEndX)
    }
    pub fn toggle_debug_info() -> Self {
        Message::UI(UIMessage::ToggleDebugInfo)
    }
    pub fn set_theme(theme: Theme) -> Self {
        Message::UI(UIMessage::SetTheme(theme))
    }
    pub fn set_band_persistence(mode: PersistenceMode) -> Self {
        Message::UI(UIMessage::SetBandPersistence(mode))
    }
    pub fn set_image_settings_persistence(mode: PersistenceMode) -> Self {
        Message::UI(UIMessage::SetImageSettingsPersistence(mode))
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

    // Band selection shortcuts
    pub fn set_red_band(band: usize) -> Self {
        Message::Band(BandMessage::SetRedBand(band))
    }
    pub fn set_green_band(band: usize) -> Self {
        Message::Band(BandMessage::SetGreenBand(band))
    }
    pub fn set_blue_band(band: usize) -> Self {
        Message::Band(BandMessage::SetBlueBand(band))
    }
    pub fn start_red_band(band: usize) -> Self {
        Message::Band(BandMessage::StartRedBand(band))
    }
    pub fn start_green_band(band: usize) -> Self {
        Message::Band(BandMessage::StartGreenBand(band))
    }
    pub fn start_blue_band(band: usize) -> Self {
        Message::Band(BandMessage::StartBlueBand(band))
    }
    pub fn apply_bands() -> Self {
        Message::Band(BandMessage::ApplyBands)
    }
    pub fn reset_bands() -> Self {
        Message::Band(BandMessage::ResetBands)
    }
}
