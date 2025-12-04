//! Home view - welcome screen.

use crate::theme::Theme;
use crate::message::Message;
use hvat_ui::widgets::{button, column, row, text, Column, Element};
use hvat_ui::Color;

/// Build the home/welcome view.
pub fn view_home(theme: &Theme, text_color: Color) -> Column<'static, Message> {
    // Project management buttons
    let project_buttons = row()
        .push(Element::new(
            button("Save Project")
                .on_press(Message::save_project())
                .width(150.0),
        ))
        .push(Element::new(
            button("Load Project")
                .on_press(Message::load_project())
                .width(150.0),
        ))
        .spacing(10.0);

    column()
        .push(Element::new(
            text("Welcome to HVAT")
                .size(28.0)
                .color(text_color),
        ))
        .push(Element::new(
            text("A GPU-accelerated hyperspectral image annotation tool")
                .size(14.0)
                .color(text_color),
        ))
        .push(Element::new(
            text("Project Management:")
                .size(16.0)
                .color(theme.accent_color()),
        ))
        .push(Element::new(project_buttons))
        .push(Element::new(
            text("Features:")
                .size(16.0)
                .color(theme.accent_color()),
        ))
        .push(Element::new(
            text("• Fast GPU rendering with wgpu")
                .size(14.0)
                .color(text_color),
        ))
        .push(Element::new(
            text("• Cross-platform (native + WASM)")
                .size(14.0)
                .color(text_color),
        ))
        .push(Element::new(
            text("• Pan and zoom")
                .size(14.0)
                .color(text_color),
        ))
        .push(Element::new(
            text("• Custom UI framework")
                .size(14.0)
                .color(text_color),
        ))
        .push(Element::new(
            text("Navigate using the tabs above to explore features")
                .size(14.0)
                .color(theme.accent_color()),
        ))
        .spacing(20.0)
}
