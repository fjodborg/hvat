//! Application message types for HVAT.
//!
//! All UI events and actions are represented as messages in the Elm architecture style.

use hvat_ui::prelude::*;

use crate::model::AnnotationTool;
use crate::state::ProjectState;

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

    // Left Sidebar - Image Tags
    /// Tags section toggled
    TagsToggled(CollapsibleState),
    /// Tag input text changed
    TagInputChanged(String, TextInputState),
    /// Add tag from input
    AddTag,
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

    // Right Sidebar Scroll
    /// Right sidebar scrolled
    RightScrolled(ScrollState),
}
