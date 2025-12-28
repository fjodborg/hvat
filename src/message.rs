//! Application message types for HVAT.
//!
//! All UI events and actions are represented as messages in the Elm architecture style.

use std::path::PathBuf;

use hvat_ui::prelude::*;
use hvat_ui::{FileTreeState, ImagePointerEvent, TooltipContent};

use crate::config::LogLevel;
use crate::keybindings::KeybindTarget;
use crate::model::AnnotationTool;
use crate::state::{LoadedImage, ProjectState};

/// Messages that can be sent to update application state.
#[derive(Clone)]
pub enum Message {
    // TopBar
    /// Open folder dialog requested
    OpenFolder,
    /// Navigate to previous image
    PrevImage,
    /// Navigate to next image
    NextImage,
    /// Undo last action
    Undo,
    /// Redo previously undone action
    Redo,
    /// Toggle settings panel
    ToggleSettings,
    /// Close settings and return to main view
    CloseSettings,
    /// Settings view scrolled
    SettingsScrolled(ScrollState),
    /// Dependencies section toggled in settings
    DependenciesToggled(CollapsibleState),
    /// Individual license section toggled (license name, new state)
    LicenseToggled(String, CollapsibleState),
    /// Settings section toggled (collapsible)
    SettingsSectionToggled(CollapsibleState),
    /// Appearance section toggled
    AppearanceSectionToggled(CollapsibleState),
    /// Keybindings section toggled
    KeybindingsSectionToggled(CollapsibleState),
    /// Folders section toggled
    FoldersSectionToggled(CollapsibleState),
    /// Performance section toggled
    PerformanceSectionToggled(CollapsibleState),
    /// Theme changed (true = dark, false = light)
    ThemeChanged(bool),
    /// Log level changed
    LogLevelChanged(LogLevel),
    /// Export folder path changed
    ExportFolderChanged(String, TextInputState),
    /// Import folder path changed
    ImportFolderChanged(String, TextInputState),
    /// Folder was selected and images discovered
    FolderLoaded(ProjectState),

    // Image Viewer
    /// Image viewer state changed (pan/zoom)
    ViewerChanged(ImageViewerState),

    // Left Sidebar - Tools
    /// Tools section toggled
    ToolsToggled(CollapsibleState),
    /// Annotation tool selected
    ToolSelected(AnnotationTool),

    // Left Sidebar - Categories
    /// Categories section toggled
    CategoriesToggled(CollapsibleState),
    /// Category selected by ID
    CategorySelected(u32),
    /// Category input text changed (for adding new categories)
    CategoryInputChanged(String, TextInputState),
    /// Add new category from input
    AddCategory,
    /// Start editing a category name (by ID)
    StartEditingCategory(u32),
    /// Category name input changed
    CategoryNameChanged(String, TextInputState),
    /// Finish editing category name (submit)
    FinishEditingCategory,
    /// Cancel editing category name
    CancelEditingCategory,
    /// Toggle color picker for a category (opens if closed, closes if open)
    ToggleCategoryColorPicker(u32),
    /// Close category color picker
    CloseCategoryColorPicker,
    /// Live color update from slider (doesn't close picker)
    CategoryColorLiveUpdate([u8; 3]),
    /// Apply color from palette selection (closes picker)
    CategoryColorApply([u8; 3]),
    /// Color picker state changed (drag state)
    ColorPickerStateChanged(ColorPickerState),
    /// Request to delete a category (shows confirmation)
    DeleteCategory(u32),
    /// Confirmed deletion of a category (actually performs delete)
    ConfirmDeleteCategory(u32),

    // Left Sidebar - Image Tags
    /// Tags section toggled
    TagsToggled(CollapsibleState),
    /// Tag selected by ID
    TagSelected(u32),
    /// Tag input text changed
    TagInputChanged(String, TextInputState),
    /// Add tag from input
    AddTag,
    /// Toggle tag on current image (on/off) by ID
    ToggleImageTag(u32),
    /// Delete tag by ID
    DeleteTag(u32),
    /// Start editing a tag name (by ID)
    StartEditingTag(u32),
    /// Tag name edit input changed
    TagNameChanged(String, TextInputState),
    /// Finish editing tag name (submit)
    FinishEditingTag,
    /// Cancel editing tag name
    CancelEditingTag,
    /// Toggle color picker for a tag
    ToggleTagColorPicker(u32),
    /// Close tag color picker
    CloseTagColorPicker,
    /// Live color update from tag color picker
    TagColorLiveUpdate([u8; 3]),
    /// Apply color from tag palette selection
    TagColorApply([u8; 3]),

    // Left Sidebar Scroll
    /// Left sidebar scrolled
    LeftScrolled(ScrollState),

    // Right Sidebar - Band Selection
    /// Band selection section toggled
    BandSelectionToggled(CollapsibleState),
    /// Red band slider changed
    RedBandChanged(SliderState),
    /// Green band slider changed
    GreenBandChanged(SliderState),
    /// Blue band slider changed
    BlueBandChanged(SliderState),

    // Right Sidebar - Image Adjustments
    /// Adjustments section toggled
    AdjustmentsToggled(CollapsibleState),
    /// Brightness slider changed
    BrightnessChanged(SliderState),
    /// Contrast slider changed
    ContrastChanged(SliderState),
    /// Gamma slider changed
    GammaChanged(SliderState),
    /// Hue slider changed
    HueChanged(SliderState),
    /// Reset all adjustments to defaults
    ResetAdjustments,

    // Right Sidebar - File Explorer (replaces File List)
    /// File explorer section toggled
    FileExplorerToggled(CollapsibleState),
    /// File explorer scrolled (internal scroll within collapsible)
    FileExplorerScrolled(ScrollState),
    /// Folder expand/collapse toggled (folder path)
    FileExplorerFolderToggle(String),
    /// File tree expansion state changed
    FileExplorerStateChanged(FileTreeState),
    /// File selected from explorer (by path string)
    FileExplorerSelect(String),

    // Right Sidebar - Thumbnails
    /// Thumbnails section toggled
    ThumbnailsToggled(CollapsibleState),
    /// Thumbnails scrolled (internal scroll within collapsible)
    ThumbnailsScrolled(ScrollState),
    /// Thumbnail clicked (by index)
    ThumbnailSelect(usize),

    // Right Sidebar - Annotations Panel
    /// Annotations section toggled
    AnnotationsToggled(CollapsibleState),
    /// Annotations list scrolled
    AnnotationsScrolled(ScrollState),
    /// Toggle category visibility filter (hide/show annotations of a category)
    ToggleCategoryFilter(u32),
    /// Select an annotation by ID (for highlighting/scrolling to it)
    SelectAnnotation(u32),

    // Right Sidebar Scroll
    /// Right sidebar scrolled
    RightScrolled(ScrollState),

    // Image Viewer - Annotations
    /// Pointer event for annotation drawing
    ImagePointer(ImagePointerEvent),
    /// Cancel current annotation drawing (Escape key)
    CancelAnnotation,
    /// Delete selected annotation
    DeleteAnnotation,
    /// Finish polygon annotation (close the shape)
    FinishPolygon,
    /// Change the category of the selected annotation
    ChangeSelectedAnnotationCategory(u32),

    // Settings - GPU Preloading
    /// GPU preload count slider changed
    GpuPreloadCountChanged(SliderState),

    // Import/Export
    /// Show export format selection dialog
    ShowExportDialog,
    /// Close export dialog
    CloseExportDialog,
    /// Export annotations in a specific format (format id)
    ExportAnnotations(String),
    /// Import annotations from file
    ImportAnnotations,
    /// Export completed successfully
    ExportCompleted(usize, usize), // (images, annotations)
    /// Export failed with error message
    ExportFailed(String),
    /// Import completed successfully
    ImportCompleted(usize, usize), // (images, annotations)
    /// Import failed with error message
    ImportFailed(String),
    /// Auto-save triggered
    AutoSave,
    /// Auto-save completed
    AutoSaveCompleted,
    /// Auto-save failed with error
    AutoSaveFailed(String),

    // Drag-Drop Events
    /// Files/folders were dropped on the window (native: filesystem paths)
    FilesDropped(Vec<PathBuf>),
    /// File data dropped (WASM: contains actual file bytes since we can't read from disk)
    FileDataDropped(Vec<LoadedImage>),
    /// Files are being dragged over the window (for visual feedback)
    FileHoverStarted,
    /// File drag hover ended
    FileHoverEnded,

    // Keybinding Configuration
    /// Start capturing a key for rebinding
    StartCapturingKeybind(KeybindTarget),
    /// Cancel key capture
    CancelCapturingKeybind,
    /// A key was captured for rebinding
    KeyCaptured(KeyCode),
    /// Reset keybindings to defaults
    ResetKeybindings,

    // Configuration Import/Export
    /// Export configuration to JSON file
    ExportConfig,
    /// Import configuration from JSON file
    ImportConfig,
    /// Configuration data loaded (from WASM file picker or native)
    ConfigLoaded(String),
    /// Configuration import completed successfully
    ConfigImportCompleted,
    /// Configuration import failed with error message
    ConfigImportFailed(String),

    // Tooltip Events
    /// Request to show a tooltip (id, content, trigger_bounds, mouse_position)
    TooltipRequest(String, TooltipContent, Bounds, (f32, f32)),
    /// Clear tooltip if it matches the given ID (mouse left the trigger area)
    TooltipClear(String),
    /// Update mouse position for current tooltip (for tracking movement)
    TooltipMouseMove((f32, f32)),
    /// Tooltip became visible (after idle timer expired)
    TooltipBecameVisible,

    // Context Menu Events
    /// Open context menu at position (position, annotation_id if clicked on annotation)
    OpenContextMenu((f32, f32), Option<u32>),
    /// Close context menu without selection
    CloseContextMenu,
    /// Context menu item selected (item id)
    ContextMenuSelect(String),

    // Confirmation Dialog Events
    /// Confirmation dialog confirmed (user clicked confirm button)
    ConfirmDialogConfirm,
    /// Confirmation dialog cancelled (user clicked cancel or pressed Escape)
    ConfirmDialogCancel,
}
