//! Error types for annotation format operations.

use std::fmt;

/// Errors that can occur during format import/export operations.
#[derive(Debug)]
pub enum FormatError {
    /// JSON parsing or serialization error.
    Json(serde_json::Error),
    /// XML parsing error (for Pascal VOC).
    Xml(String),
    /// Invalid or malformed data in the format.
    InvalidData(String),
    /// Missing required field.
    MissingField(String),
    /// Unsupported shape type for this format.
    UnsupportedShape(String),
    /// Invalid coordinate values.
    InvalidCoordinates(String),
    /// Category not found.
    CategoryNotFound(u32),
    /// IO error (for file operations).
    Io(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Json(e) => write!(f, "JSON error: {}", e),
            FormatError::Xml(msg) => write!(f, "XML error: {}", msg),
            FormatError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            FormatError::MissingField(field) => write!(f, "Missing required field: {}", field),
            FormatError::UnsupportedShape(msg) => write!(f, "Unsupported shape: {}", msg),
            FormatError::InvalidCoordinates(msg) => write!(f, "Invalid coordinates: {}", msg),
            FormatError::CategoryNotFound(id) => write!(f, "Category not found: {}", id),
            FormatError::Io(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for FormatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FormatError::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for FormatError {
    fn from(e: serde_json::Error) -> Self {
        FormatError::Json(e)
    }
}
