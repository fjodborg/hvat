//! Image viewer UI component.

use hvat_ui::prelude::*;
use hvat_ui::{AnnotationOverlay, Column, Context, Element, OverlayShape};

use crate::app::HvatApp;
use crate::message::Message;
use crate::model::{AnnotationShape, AnnotationTool, DrawingState};

impl From<&AnnotationShape> for OverlayShape {
    fn from(shape: &AnnotationShape) -> Self {
        match shape {
            AnnotationShape::BoundingBox {
                x,
                y,
                width,
                height,
            } => OverlayShape::BoundingBox {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            },
            AnnotationShape::Point { x, y } => OverlayShape::Point { x: *x, y: *y },
            AnnotationShape::Polygon { vertices } => OverlayShape::Polygon {
                vertices: vertices.clone(),
                closed: true,
            },
        }
    }
}

impl HvatApp {
    /// Build the central image viewer.
    pub(crate) fn build_image_viewer(&self) -> Element<Message> {
        let viewer_state = self.viewer_state.clone();
        let texture_id = self.texture_id;
        let texture_size = self.image_size;

        let overlays = self.build_overlays();
        let interaction_mode = match self.selected_tool {
            AnnotationTool::Select => InteractionMode::Annotate, // Select also needs pointer events
            #[cfg(feature = "sam2")]
            AnnotationTool::SAM2Segment => InteractionMode::Annotate, // SAM2 needs pointer events
            _ if self.selected_tool.is_drawing_tool() => InteractionMode::Annotate,
            _ => InteractionMode::View,
        };

        let mut ctx = Context::new();

        if let Some(tex_id) = texture_id {
            ctx.image_viewer(tex_id, texture_size.0, texture_size.1)
                .state(&viewer_state)
                .show_controls(true)
                .width(Length::Fill(1.0))
                .height(Length::Fill(1.0))
                .on_change(Message::ViewerChanged)
                .on_pointer(Message::ImagePointer)
                .interaction_mode(interaction_mode)
                .overlays(overlays)
                .build();
        } else {
            ctx.image_viewer_empty()
                .state(&viewer_state)
                .show_controls(true)
                .width(Length::Fill(1.0))
                .height(Length::Fill(1.0))
                .build();
        }

        Element::new(Column::new(ctx.take()))
    }

    /// Build annotation overlays from current annotations and drawing state.
    /// Annotations with hidden categories are filtered out from rendering.
    fn build_overlays(&self) -> Vec<AnnotationOverlay> {
        let path = self.current_image_path();
        let image_data = self.image_data_store.get(&path);

        // Filter out annotations whose categories are hidden
        let mut overlays: Vec<_> = image_data
            .annotations
            .iter()
            .filter(|ann| !self.hidden_categories.contains(&ann.category_id))
            .map(|ann| AnnotationOverlay {
                shape: (&ann.shape).into(),
                color: self.get_category_color(ann.category_id),
                line_width: 2.0,
                selected: ann.selected,
            })
            .collect();

        // Add drawing preview if active
        if let Some(preview) = self.drawing_preview(&image_data.drawing_state) {
            let color = self.get_category_color(self.selected_category);
            overlays.push(AnnotationOverlay {
                shape: preview,
                color: [color[0], color[1], color[2], color[3] * 0.7],
                line_width: 2.0,
                selected: false,
            });
        }

        // Add SAM2 point overlays if SAM2 is active
        #[cfg(feature = "sam2")]
        self.add_sam2_overlays(&mut overlays);

        log::info!(
            "build_overlays: returning {} overlays total",
            overlays.len()
        );
        overlays
    }

    /// Add SAM2 point overlays (positive = green, negative = red).
    #[cfg(feature = "sam2")]
    fn add_sam2_overlays(&self, overlays: &mut Vec<AnnotationOverlay>) {
        use crate::sam2::SAM2State;

        // Debug: Log current SAM2 state
        log::info!(
            "SAM2 overlay check: state={:?}",
            std::mem::discriminant(&self.sam2_state)
        );

        if let SAM2State::Active { session } = &self.sam2_state {
            // Only render SAM2 overlays if the session is for the current image
            let current_path = self.current_image_path();
            if session.image_path != current_path {
                log::debug!("SAM2 session is for different image, not rendering overlays");
                return;
            }

            log::info!(
                "SAM2 Active: {} positive, {} negative points",
                session.prompts.positive_points.len(),
                session.prompts.negative_points.len()
            );
            // Add positive points (green)
            for (x, y) in &session.prompts.positive_points {
                log::info!("SAM2: Adding overlay for positive point at ({}, {})", x, y);
                overlays.push(AnnotationOverlay {
                    shape: OverlayShape::Point { x: *x, y: *y },
                    color: [0.0, 0.8, 0.2, 1.0], // Green
                    line_width: 3.0,
                    selected: false,
                });
            }

            // Add negative points (red)
            for (x, y) in &session.prompts.negative_points {
                overlays.push(AnnotationOverlay {
                    shape: OverlayShape::Point { x: *x, y: *y },
                    color: [0.9, 0.2, 0.2, 1.0], // Red
                    line_width: 3.0,
                    selected: false,
                });
            }

            // Render mask contour if available
            if let Some(mask) = &session.mask {
                if !mask.contour.is_empty() {
                    log::debug!(
                        "SAM2: Rendering mask contour with {} vertices, score={:.2}",
                        mask.contour.len(),
                        mask.score
                    );
                    // Render the mask contour as a semi-transparent filled polygon
                    overlays.push(AnnotationOverlay {
                        shape: OverlayShape::Polygon {
                            vertices: mask.contour.clone(),
                            closed: true,
                        },
                        color: [0.2, 0.6, 1.0, 0.4], // Blue semi-transparent
                        line_width: 2.0,
                        selected: true, // Highlight the mask
                    });
                }
            }

            // Log the number of SAM2 points for debugging
            let total =
                session.prompts.positive_points.len() + session.prompts.negative_points.len();
            if total > 0 {
                log::trace!(
                    "SAM2: Rendering {} points ({} positive, {} negative)",
                    total,
                    session.prompts.positive_points.len(),
                    session.prompts.negative_points.len()
                );
            }
        }
    }

    /// Get preview overlay for current drawing state.
    fn drawing_preview(&self, drawing_state: &DrawingState) -> Option<OverlayShape> {
        match drawing_state {
            DrawingState::Idle => None,
            DrawingState::BoundingBox {
                start_x,
                start_y,
                current_x,
                current_y,
            } => Some(OverlayShape::BoundingBox {
                x: start_x.min(*current_x),
                y: start_y.min(*current_y),
                width: (current_x - start_x).abs(),
                height: (current_y - start_y).abs(),
            }),
            DrawingState::Polygon { vertices } if !vertices.is_empty() => {
                Some(OverlayShape::Polygon {
                    vertices: vertices.clone(),
                    closed: false,
                })
            }
            DrawingState::Polygon { .. } => None,
        }
    }

    /// Get the color for a category as RGBA floats.
    fn get_category_color(&self, category_id: u32) -> [f32; 4] {
        self.categories
            .iter()
            .find(|c| c.id == category_id)
            .map(|c| {
                [
                    c.color[0] as f32 / 255.0,
                    c.color[1] as f32 / 255.0,
                    c.color[2] as f32 / 255.0,
                    1.0,
                ]
            })
            .unwrap_or([1.0, 0.0, 0.0, 1.0])
    }
}
