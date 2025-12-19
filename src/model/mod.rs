//! Data models for HVAT application.

mod annotation;
mod category;

pub use annotation::{
    Annotation, AnnotationId, AnnotationShape, AnnotationTool, DrawingState,
    MIN_POLYGON_VERTICES, POLYGON_CLOSE_THRESHOLD,
};
pub use category::Category;
