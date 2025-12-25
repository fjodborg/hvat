//! Tag data model for image-level tags.
//!
//! Tags are similar to categories but are applied to entire images,
//! not individual annotations.

use serde::{Deserialize, Serialize};

/// An image tag with a name and color.
/// Tags are applied to entire images, unlike categories which are for annotations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tag {
    /// Unique identifier for the tag
    pub id: u32,
    /// Display name of the tag
    pub name: String,
    /// RGB color for the tag
    pub color: [u8; 3],
}

impl Tag {
    /// Create a new tag with the given ID, name, and color.
    pub fn new(id: u32, name: &str, color: [u8; 3]) -> Self {
        Self {
            id,
            name: name.to_string(),
            color,
        }
    }
}

/// Default tags for new projects.
pub fn default_tags() -> Vec<Tag> {
    vec![
        Tag::new(1, "Review", [100, 140, 180]),
        Tag::new(2, "Complete", [100, 180, 100]),
        Tag::new(3, "Problem", [180, 100, 100]),
    ]
}
