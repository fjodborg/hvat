//! Reusable demo components for examples and web demo

mod basic;
mod collapsible;
mod comprehensive;
mod dropdown;
mod image_viewer;
mod input_widgets;
mod scrollable;
mod undo;

pub use basic::{BasicDemo, BasicMessage};
pub use collapsible::{CollapsibleDemo, CollapsibleMessage};
pub use comprehensive::{
    create_gradient_pattern, ComprehensiveDemo, ComprehensiveMessage, Tool,
};
pub use dropdown::{DropdownDemo, DropdownMessage, COUNTRY_OPTIONS, SIMPLE_OPTIONS};
pub use image_viewer::{create_test_pattern, ImageViewerDemo, ImageViewerMessage};
pub use input_widgets::{InputWidgetsDemo, InputWidgetsMessage};
pub use scrollable::{ScrollableDemo, ScrollableMessage};
pub use undo::{UndoDemo, UndoMessage};
