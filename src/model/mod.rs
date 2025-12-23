//! Data models for HVAT application.

mod annotation;
mod category;

pub use annotation::{
    Annotation, AnnotationHandle, AnnotationId, AnnotationShape, AnnotationTool, DrawingState,
    EditState, HANDLE_HIT_RADIUS, MIN_DRAG_DISTANCE, MIN_POLYGON_VERTICES, POLYGON_CLOSE_THRESHOLD,
    PolygonHandle,
};
pub use category::{Category, default_categories};
