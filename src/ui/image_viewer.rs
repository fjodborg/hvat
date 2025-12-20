//! Image viewer UI component.

use hvat_ui::prelude::*;
use hvat_ui::{AnnotationOverlay, Column, Context, Element, OverlayShape};

use crate::app::HvatApp;
use crate::message::Message;
use crate::model::{AnnotationShape, AnnotationTool, DrawingState};

impl From<&AnnotationShape> for OverlayShape {
    fn from(shape: &AnnotationShape) -> Self {
        match shape {
            AnnotationShape::BoundingBox { x, y, width, height } => {
                OverlayShape::BoundingBox { x: *x, y: *y, width: *width, height: *height }
            }
            AnnotationShape::Point { x, y } => OverlayShape::Point { x: *x, y: *y },
            AnnotationShape::Polygon { vertices } => {
                OverlayShape::Polygon { vertices: vertices.clone(), closed: true }
            }
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
    fn build_overlays(&self) -> Vec<AnnotationOverlay> {
        let path = self.current_image_path();
        let image_data = self.image_data_store.get(&path);

        let mut overlays: Vec<_> = image_data
            .annotations
            .iter()
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

        overlays
    }

    /// Get preview overlay for current drawing state.
    fn drawing_preview(&self, drawing_state: &DrawingState) -> Option<OverlayShape> {
        match drawing_state {
            DrawingState::Idle => None,
            DrawingState::BoundingBox { start_x, start_y, current_x, current_y } => {
                Some(OverlayShape::BoundingBox {
                    x: start_x.min(*current_x),
                    y: start_y.min(*current_y),
                    width: (current_x - start_x).abs(),
                    height: (current_y - start_y).abs(),
                })
            }
            DrawingState::Polygon { vertices } if !vertices.is_empty() => {
                Some(OverlayShape::Polygon { vertices: vertices.clone(), closed: false })
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
