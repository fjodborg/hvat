//! Counter view - demo counter widget.

use crate::theme::Theme;
use crate::message::{Message, CounterMessage};
use hvat_ui::widgets::{column, row, button, container, text, Element, Column};
use hvat_ui::Color;

/// Build the counter demo view.
pub fn view_counter(theme: &Theme, text_color: Color, counter: i32) -> Column<'static, Message> {
    column()
        .push(Element::new(
            text("Counter Demo")
                .size(24.0)
                .color(text_color),
        ))
        .push(Element::new(
            container(Element::new(
                text(format!("{}", counter))
                    .size(48.0)
                    .color(theme.accent_color()),
            ))
            .padding(20.0),
        ))
        .push(Element::new(
            row()
                .push(Element::new(
                    button("Increment")
                        .on_press(Message::increment())
                        .width(150.0),
                ))
                .push(Element::new(
                    button("Decrement")
                        .on_press(Message::decrement())
                        .width(150.0),
                ))
                .push(Element::new(
                    button("Reset")
                        .on_press(Message::Counter(CounterMessage::Reset))
                        .width(150.0),
                ))
                .spacing(15.0),
        ))
        .spacing(30.0)
}
