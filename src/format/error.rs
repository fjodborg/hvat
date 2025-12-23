//! Error types for annotation format operations.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during annotation format operations.
#[derive(Error, Debug)]
pub enum FormatError {
    /// I/O error during file operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing or serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// XML parsing or serialization error
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),

    /// Invalid format structure or content
    #[error("Invalid format: {message}")]
    InvalidFormat {
        /// Description of the format error
        message: String,
    },

    /// Required field is missing
    #[error("Missing required field: {field}")]
    MissingField {
        /// Name of the missing field
        field: String,
    },

    /// Annotation shape type not supported by this format
    #[error("Unsupported shape type '{shape}' for format '{format}'")]
    UnsupportedShape {
        /// The shape type that was encountered
        shape: String,
        /// The format that doesn't support this shape
        format: String,
    },

    /// Image file not found at expected path
    #[error("Image not found: {path:?}")]
    ImageNotFound {
        /// Path where the image was expected
        path: PathBuf,
    },

    /// Category ID referenced but not defined
    #[error("Category not found: {id}")]
    CategoryNotFound {
        /// The missing category ID
        id: u32,
    },

    /// Invalid coordinate values
    #[error("Invalid coordinates: {message}")]
    InvalidCoordinates {
        /// Description of the coordinate error
        message: String,
    },

    /// Version mismatch between expected and found
    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch {
        /// Expected version string
        expected: String,
        /// Found version string
        found: String,
    },

    /// Image dimensions required but not available
    #[error(
        "Image dimensions required for format '{format}' but not available for image '{image}'"
    )]
    MissingDimensions {
        /// The format requiring dimensions
        format: String,
        /// The image missing dimensions
        image: String,
    },

    /// Operation not supported by this format
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
}

impl FormatError {
    /// Create an invalid format error with a message.
    pub fn invalid_format(message: impl Into<String>) -> Self {
        Self::InvalidFormat {
            message: message.into(),
        }
    }

    /// Create a missing field error.
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
        }
    }

    /// Create an unsupported shape error.
    pub fn unsupported_shape(shape: impl Into<String>, format: impl Into<String>) -> Self {
        Self::UnsupportedShape {
            shape: shape.into(),
            format: format.into(),
        }
    }

    /// Create an invalid coordinates error.
    pub fn invalid_coordinates(message: impl Into<String>) -> Self {
        Self::InvalidCoordinates {
            message: message.into(),
        }
    }
}
