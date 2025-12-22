//! Application message types for HVAT.
//!
//! All UI events and actions are represented as messages in the Elm architecture style.

use std::path::PathBuf;

use hvat_ui::ImagePointerEvent;
use hvat_ui::prelude::*;

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
    /// Theme changed (true = dark, false = light)
    ThemeChanged(bool),
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
    /// Add new category
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

    // Left Sidebar - Image Tags
    /// Tags section toggled
    TagsToggled(CollapsibleState),
    /// Tag input text changed
    TagInputChanged(String, TextInputState),
    /// Add tag from input
    AddTag,
    /// Toggle tag selection (on/off)
    ToggleTag(String),
    /// Remove tag by value
    RemoveTag(String),

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

    // Right Sidebar - File List
    /// File list section toggled
    FileListToggled(CollapsibleState),
    /// File list scrolled (internal scroll within collapsible)
    FileListScrolled(ScrollState),
    /// File selected from list (by index)
    FileListSelect(usize),

    // Right Sidebar - Thumbnails
    /// Thumbnails section toggled
    ThumbnailsToggled(CollapsibleState),
    /// Thumbnails scrolled (internal scroll within collapsible)
    ThumbnailsScrolled(ScrollState),
    /// Thumbnail clicked (by index)
    ThumbnailSelect(usize),

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
}
