//! Top bar UI component.

use hvat_ui::prelude::*;
use hvat_ui::{Context, Element, Row};

use crate::app::HvatApp;
use crate::message::Message;

impl HvatApp {
    /// Build the top bar with navigation and control buttons.
    pub(crate) fn build_topbar(&self) -> Element<Message> {
        let viewer_state = self.viewer_state.clone();

        // Get image name and progress from project state
        let (image_name, progress) = match &self.project {
            Some(project) => (project.current_name(), project.progress()),
            None => ("(no folder loaded)".to_string(), "0/0".to_string()),
        };

        let mut ctx = Context::new();
        ctx.row(|r| {
            r.button("Open Folder")
                .width(Length::Fixed(90.0))
                .on_click(Message::OpenFolder);
            r.button("◄ Prev")
                .width(Length::Fixed(60.0))
                .on_click(Message::PrevImage);
            r.button("Next ►")
                .width(Length::Fixed(60.0))
                .on_click(Message::NextImage);
            r.text(" │ ");
            r.text(format!("{} [{}]", image_name, progress));
            r.text(" │ ");
            r.text(format!("Zoom: {:.0}%", viewer_state.zoom * 100.0));
            r.text(" │ ");
            r.button("Undo")
                .width(Length::Fixed(50.0))
                .on_click(Message::Undo);
            r.button("Redo")
                .width(Length::Fixed(50.0))
                .on_click(Message::Redo);
            r.button("⚙")
                .width(Length::Fixed(30.0))
                .on_click(Message::ToggleSettings);
        });

        let row = Row::new(ctx.take())
            .width(Length::Fill(1.0))
            .height(Length::Fixed(40.0))
            .spacing(8.0);

        Element::new(row)
    }
}
