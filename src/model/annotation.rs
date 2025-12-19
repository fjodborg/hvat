//! Annotation tool types.

/// Annotation tools available in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationTool {
    /// Selection tool for selecting existing annotations
    Select,
    /// Bounding box annotation tool
    BoundingBox,
    /// Polygon annotation tool
    Polygon,
    /// Point annotation tool
    Point,
}

impl Default for AnnotationTool {
    fn default() -> Self {
        AnnotationTool::Select
    }
}

impl AnnotationTool {
    /// Get the display name for this tool.
    pub fn name(&self) -> &'static str {
        match self {
            AnnotationTool::Select => "Select",
            AnnotationTool::BoundingBox => "Bounding Box",
            AnnotationTool::Polygon => "Polygon",
            AnnotationTool::Point => "Point",
        }
    }

    /// Get all available annotation tools.
    pub fn all() -> &'static [AnnotationTool] {
        &[
            AnnotationTool::Select,
            AnnotationTool::BoundingBox,
            AnnotationTool::Polygon,
            AnnotationTool::Point,
        ]
    }
}
