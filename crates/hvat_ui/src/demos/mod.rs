//! Reusable demo components for examples and web demo

mod basic;
mod collapsible;
mod dropdown;
mod image_viewer;
mod scrollable;

pub use basic::{BasicDemo, BasicMessage};
pub use collapsible::{CollapsibleDemo, CollapsibleMessage};
pub use dropdown::{DropdownDemo, DropdownMessage, COUNTRY_OPTIONS, SIMPLE_OPTIONS};
pub use image_viewer::{create_test_pattern, ImageViewerDemo, ImageViewerMessage};
pub use scrollable::{ScrollableDemo, ScrollableMessage};
