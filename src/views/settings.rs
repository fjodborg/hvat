//! Settings view - application settings.

use crate::theme::Theme;
use crate::message::Message;
use hvat_ui::widgets::{column, row, button, container, text, Element, Column};
use hvat_ui::Color;

/// Build the settings view.
pub fn view_settings(theme: &Theme, text_color: Color, show_debug_info: bool) -> Column<'static, Message> {
    column()
        .push(Element::new(
            text("Settings")
                .size(24.0)
                .color(text_color),
        ))
        .push(Element::new(
            container(Element::new(
                column()
                    .push(Element::new(
                        text("Theme")
                            .size(16.0)
                            .color(theme.accent_color()),
                    ))
                    .push(Element::new(
                        row()
                            .push(Element::new(
                                button("Dark Theme")
                                    .on_press(Message::set_theme(Theme::dark()))
                                    .width(120.0),
                            ))
                            .push(Element::new(
                                button("Light Theme")
                                    .on_press(Message::set_theme(Theme::light()))
                                    .width(120.0),
                            ))
                            .spacing(10.0),
                    ))
                    .spacing(15.0),
            ))
            .padding(20.0),
        ))
        .push(Element::new(
            container(Element::new(
                column()
                    .push(Element::new(
                        text("Debug")
                            .size(16.0)
                            .color(theme.accent_color()),
                    ))
                    .push(Element::new(
                        button(if show_debug_info {
                            "Hide Debug Info"
                        } else {
                            "Show Debug Info"
                        })
                        .on_press(Message::toggle_debug_info())
                        .width(150.0),
                    ))
                    .spacing(15.0),
            ))
            .padding(20.0),
        ))
        .spacing(20.0)
}
