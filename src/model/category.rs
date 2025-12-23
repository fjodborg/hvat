//! Category data model for annotation categories.

use serde::{Deserialize, Serialize};

/// An annotation category with a name and color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    /// Unique identifier for the category
    pub id: u32,
    /// Display name of the category
    pub name: String,
    /// RGB color for the category
    pub color: [u8; 3],
}

impl Category {
    /// Create a new category with the given ID, name, and color.
    pub fn new(id: u32, name: &str, color: [u8; 3]) -> Self {
        Self {
            id,
            name: name.to_string(),
            color,
        }
    }
}

/// Default categories for new projects.
pub fn default_categories() -> Vec<Category> {
    vec![
        Category::new(1, "Background", [100, 100, 100]),
        Category::new(2, "Object", [255, 100, 100]),
        Category::new(3, "Region", [100, 255, 100]),
    ]
}
