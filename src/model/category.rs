//! Category data model for annotation categories.

/// An annotation category with a name and color.
#[derive(Debug, Clone)]
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
