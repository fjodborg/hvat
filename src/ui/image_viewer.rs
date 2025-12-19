//! Image viewer UI component.

use hvat_ui::prelude::*;
use hvat_ui::{Column, Context, Element};

use crate::app::HvatApp;
use crate::message::Message;

impl HvatApp {
    /// Build the central image viewer.
    pub(crate) fn build_image_viewer(&self) -> Element<Message> {
        let viewer_state = self.viewer_state.clone();
        let texture_id = self.texture_id;
        let texture_size = self.image_size;

        // Note: Adjustments are applied by HyperspectralPipeline, not ImageViewer.
        // The render target texture already has band compositing + adjustments baked in.

        let mut ctx = Context::new();

        if let Some(tex_id) = texture_id {
            ctx.image_viewer(tex_id, texture_size.0, texture_size.1)
                .state(&viewer_state)
                .show_controls(true)
                .width(Length::Fill(1.0))
                .height(Length::Fill(1.0))
                .on_change(Message::ViewerChanged)
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
}
