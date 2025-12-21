//! Export dialog UI component.
//!
//! A modal dialog for selecting export format.

use hvat_ui::constants::BUTTON_PADDING_COMPACT;
use hvat_ui::prelude::*;
use hvat_ui::{Column, Context, Element};

use crate::app::HvatApp;
use crate::message::Message;

impl HvatApp {
    /// Build the export dialog (modal overlay).
    pub(crate) fn build_export_dialog(&self) -> Element<Message> {
        let mut ctx = Context::new();

        // Dialog title
        ctx.text("Export Annotations").size(FONT_SIZE_TITLE);
        ctx.text("");
        ctx.text("Select export format:");
        ctx.text("");

        // Format buttons - iterate over registered formats
        for format in self.format_registry.all() {
            let format_id = format.id().to_string();
            let display_name = format.display_name();

            // Build description based on capabilities
            let mut capabilities = Vec::new();
            if format.supports_polygon() {
                capabilities.push("polygon");
            }
            if format.supports_point() {
                capabilities.push("point");
            }
            capabilities.push("bbox"); // All formats support bbox

            let capability_str = capabilities.join(", ");
            let output_mode = if format.supports_per_image() {
                "per-image files"
            } else {
                "single file"
            };

            ctx.row(|r| {
                r.button(display_name)
                    .padding(BUTTON_PADDING_COMPACT)
                    .width(Length::Fixed(180.0))
                    .on_click(Message::ExportAnnotations(format_id));
                r.text(format!("({}, {})", capability_str, output_mode))
                    .size(FONT_SIZE_SMALL);
            });
            ctx.text("");
        }

        ctx.text("");

        // Cancel button
        ctx.row(|r| {
            r.button("Cancel")
                .padding(BUTTON_PADDING_COMPACT)
                .on_click(Message::CloseExportDialog);
        });

        Element::new(Column::new(ctx.take()).padding(24.0))
    }
}
