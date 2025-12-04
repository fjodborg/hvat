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
    /// Reset zoom to 1:1 pixel ratio (one screen pixel = one image pixel)
    ResetToOneToOne,
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
    /// Report widget bounds (width, height) for pixel ratio calculation
    ReportBounds(f32, f32),
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
    // Main content vertical scrolling
    ScrollY(f32),
    /// Scrollbar drag start with mouse Y position for relative dragging
    ScrollbarDragStartY(f32),
    ScrollbarDragEndY,
    // Main content horizontal scrolling
    ScrollX(f32),
    /// Scrollbar drag start with mouse X position for relative dragging
    ScrollbarDragStartX(f32),
    ScrollbarDragEndX,
    // Sidebar scrolling (independent)
    SidebarScrollY(f32),
    /// Sidebar scrollbar drag start with mouse Y position for relative dragging
    SidebarScrollbarDragStartY(f32),
    SidebarScrollbarDragEndY,
    // Settings
    ToggleDebugInfo,
    SetTheme(Theme),
    // Persistence modes for settings across image navigation
    SetBandPersistence(PersistenceMode),
    SetImageSettingsPersistence(PersistenceMode),
    // Dropdown state
    OpenBandPersistenceDropdown,
    CloseBandPersistenceDropdown,
    OpenImageSettingsPersistenceDropdown,
    CloseImageSettingsPersistenceDropdown,
    // Collapsible state
    ToggleImageSettingsCollapsed,
    ToggleBandSettingsCollapsed,
    // Category input state
    SetNewCategoryText(String),
    SetCategoryInputFocused(bool),
    SubmitNewCategory,
    // Tag input state
    SetNewTagText(String),
    SetTagInputFocused(bool),
    SubmitNewTag,
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

/// Available export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportFormat {
    #[default]
    Coco,
    Yolo,
    YoloSegmentation,
    Datumaro,
    PascalVoc,
}

impl ExportFormat {
    /// Get format name for display.
    pub fn name(&self) -> &'static str {
        match self {
            ExportFormat::Coco => "COCO",
            ExportFormat::Yolo => "YOLO",
            ExportFormat::YoloSegmentation => "YOLO Seg",
            ExportFormat::Datumaro => "Datumaro",
            ExportFormat::PascalVoc => "Pascal VOC",
        }
    }

    /// Get all available formats.
    pub fn all() -> &'static [ExportFormat] {
        &[
            ExportFormat::Coco,
            ExportFormat::Yolo,
            ExportFormat::YoloSegmentation,
            ExportFormat::Datumaro,
            ExportFormat::PascalVoc,
        ]
    }

    /// Get format from index.
    pub fn from_index(index: usize) -> Self {
        Self::all().get(index).copied().unwrap_or_default()
    }

    /// Get index of this format.
    pub fn index(&self) -> usize {
        Self::all().iter().position(|f| f == self).unwrap_or(0)
    }
}

/// Messages for annotation tools and operations.
#[derive(Debug, Clone)]
pub enum AnnotationMessage {
    // Tool selection
    SetTool(AnnotationTool),
    /// Tool shortcut key pressed (b=box, m=mask, p=point, s=select, ESC=cancel, DEL=delete)
    ToolShortcut(char),
    // Category management
    SetCategory(u32),
    /// Select category by hotkey number (1-9), maps to sorted category order
    SelectCategoryByHotkey(u8),
    AddCategory(String),
    // Drawing operations
    StartDrawing(f32, f32),
    ContinueDrawing(f32, f32),
    FinishDrawing,
    /// Force finish polygon (Space key) - closes polygon regardless of mouse state
    ForceFinishPolygon,
    CancelDrawing,
    // Selection and editing
    SelectAnnotation(Option<u64>),
    DeleteSelected,
    /// Start dragging selected annotation or handle
    StartDrag(f32, f32),
    /// Continue dragging (move)
    ContinueDrag(f32, f32),
    /// Finish dragging
    FinishDrag,
    // Import/Export
    /// Open export dialog with format selection
    OpenExportDialog,
    /// Close export dialog
    CloseExportDialog,
    /// Set the export format
    SetExportFormat(ExportFormat),
    /// Toggle export format dropdown
    ToggleExportFormatDropdown,
    /// Perform the export with the selected format
    PerformExport,
    /// Legacy: export to JSON (now opens dialog)
    ExportJson,
    ImportJson,
    ClearAll,
}

/// Messages for project file management.
#[derive(Debug, Clone)]
pub enum ProjectMessage {
    /// Save project to file
    SaveProject,
    /// Load project from file
    LoadProject,
    /// Project save completed
    #[cfg(not(target_arch = "wasm32"))]
    ProjectSaved(Result<std::path::PathBuf, String>),
    /// Project load completed
    #[cfg(not(target_arch = "wasm32"))]
    ProjectLoaded(Result<(std::path::PathBuf, crate::project::Project), String>),
    /// WASM: Download project as file
    #[cfg(target_arch = "wasm32")]
    DownloadProject,
    /// WASM: Project file uploaded
    #[cfg(target_arch = "wasm32")]
    ProjectUploaded(String, String), // (filename, json_content)
}

/// Messages for image tagging operations.
#[derive(Debug, Clone)]
pub enum TagMessage {
    /// Toggle a tag on the current image by hotkey (Ctrl+1-9)
    ToggleTagByHotkey(u8),
    /// Toggle a specific tag on the current image
    ToggleTag(u32),
    /// Add a new tag definition
    AddTag(String),
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
    /// Image tagging
    Tag(TagMessage),
    /// Project file management
    Project(ProjectMessage),
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
    pub fn reset_to_one_to_one() -> Self {
        Message::ImageView(ImageViewMessage::ResetToOneToOne)
    }
    pub fn report_widget_bounds(width: f32, height: f32) -> Self {
        Message::ImageView(ImageViewMessage::ReportBounds(width, height))
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
    pub fn scrollbar_drag_start_y(mouse_y: f32) -> Self {
        Message::UI(UIMessage::ScrollbarDragStartY(mouse_y))
    }
    pub fn scrollbar_drag_end_y() -> Self {
        Message::UI(UIMessage::ScrollbarDragEndY)
    }
    // UI shortcuts - horizontal scrolling
    pub fn scroll_x(offset: f32) -> Self {
        Message::UI(UIMessage::ScrollX(offset))
    }
    pub fn scrollbar_drag_start_x(mouse_x: f32) -> Self {
        Message::UI(UIMessage::ScrollbarDragStartX(mouse_x))
    }
    pub fn scrollbar_drag_end_x() -> Self {
        Message::UI(UIMessage::ScrollbarDragEndX)
    }
    // UI shortcuts - sidebar scrolling
    pub fn sidebar_scroll_y(offset: f32) -> Self {
        Message::UI(UIMessage::SidebarScrollY(offset))
    }
    pub fn sidebar_scrollbar_drag_start_y(mouse_y: f32) -> Self {
        Message::UI(UIMessage::SidebarScrollbarDragStartY(mouse_y))
    }
    pub fn sidebar_scrollbar_drag_end_y() -> Self {
        Message::UI(UIMessage::SidebarScrollbarDragEndY)
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
    // Dropdown shortcuts
    pub fn open_band_persistence_dropdown() -> Self {
        Message::UI(UIMessage::OpenBandPersistenceDropdown)
    }
    pub fn close_band_persistence_dropdown() -> Self {
        Message::UI(UIMessage::CloseBandPersistenceDropdown)
    }
    pub fn open_image_settings_persistence_dropdown() -> Self {
        Message::UI(UIMessage::OpenImageSettingsPersistenceDropdown)
    }
    pub fn close_image_settings_persistence_dropdown() -> Self {
        Message::UI(UIMessage::CloseImageSettingsPersistenceDropdown)
    }
    // Collapsible shortcuts
    pub fn toggle_image_settings_collapsed() -> Self {
        Message::UI(UIMessage::ToggleImageSettingsCollapsed)
    }
    pub fn toggle_band_settings_collapsed() -> Self {
        Message::UI(UIMessage::ToggleBandSettingsCollapsed)
    }
    // Category input shortcuts
    pub fn set_new_category_text(text: String) -> Self {
        Message::UI(UIMessage::SetNewCategoryText(text))
    }
    pub fn set_category_input_focused(focused: bool) -> Self {
        Message::UI(UIMessage::SetCategoryInputFocused(focused))
    }
    pub fn submit_new_category() -> Self {
        Message::UI(UIMessage::SubmitNewCategory)
    }
    // Tag input shortcuts
    pub fn set_new_tag_text(text: String) -> Self {
        Message::UI(UIMessage::SetNewTagText(text))
    }
    pub fn set_tag_input_focused(focused: bool) -> Self {
        Message::UI(UIMessage::SetTagInputFocused(focused))
    }
    pub fn submit_new_tag() -> Self {
        Message::UI(UIMessage::SubmitNewTag)
    }

    // Tag shortcuts
    pub fn toggle_tag_by_hotkey(num: u8) -> Self {
        Message::Tag(TagMessage::ToggleTagByHotkey(num))
    }
    pub fn toggle_tag(id: u32) -> Self {
        Message::Tag(TagMessage::ToggleTag(id))
    }

    // Annotation shortcuts
    pub fn set_annotation_tool(tool: AnnotationTool) -> Self {
        Message::Annotation(AnnotationMessage::SetTool(tool))
    }
    pub fn tool_shortcut(key: char) -> Self {
        Message::Annotation(AnnotationMessage::ToolShortcut(key))
    }
    pub fn set_annotation_category(id: u32) -> Self {
        Message::Annotation(AnnotationMessage::SetCategory(id))
    }
    /// Select category by hotkey (1-9 maps to sorted category order)
    pub fn select_category_by_hotkey(num: u8) -> Self {
        Message::Annotation(AnnotationMessage::SelectCategoryByHotkey(num))
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
        Message::Annotation(AnnotationMessage::OpenExportDialog)
    }
    pub fn open_export_dialog() -> Self {
        Message::Annotation(AnnotationMessage::OpenExportDialog)
    }
    pub fn close_export_dialog() -> Self {
        Message::Annotation(AnnotationMessage::CloseExportDialog)
    }
    pub fn set_export_format(format: ExportFormat) -> Self {
        Message::Annotation(AnnotationMessage::SetExportFormat(format))
    }
    pub fn toggle_export_format_dropdown() -> Self {
        Message::Annotation(AnnotationMessage::ToggleExportFormatDropdown)
    }
    pub fn perform_export() -> Self {
        Message::Annotation(AnnotationMessage::PerformExport)
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

    // Project shortcuts
    pub fn save_project() -> Self {
        Message::Project(ProjectMessage::SaveProject)
    }
    pub fn load_project() -> Self {
        Message::Project(ProjectMessage::LoadProject)
    }
}
