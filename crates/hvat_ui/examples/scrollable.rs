//! Scrollable Widget Example
//!
//! Demonstrates the scrollable container widget.

use hvat_ui::prelude::*;
use hvat_ui::{Column, Context, Element, Scrollable, ScrollDirection, ScrollbarVisibility};

/// Application state
struct ScrollableDemo {
    /// Vertical scroll state
    scroll_state: ScrollState,
    /// Number of items
    item_count: usize,
}

/// Application messages
#[derive(Clone)]
enum Message {
    /// Scroll changed
    Scrolled(ScrollState),
    /// Add more items
    AddItems,
    /// Remove items
    RemoveItems,
}

impl Default for ScrollableDemo {
    fn default() -> Self {
        Self {
            scroll_state: ScrollState::new(),
            item_count: 30,
        }
    }
}

impl Application for ScrollableDemo {
    type Message = Message;

    fn view(&self) -> Element<Message> {
        col(|c| {
            // Header
            c.text("Scrollable Widget Demo");

            // Control bar - simplified
            c.row(|r| {
                r.button("Add Items").on_click(Message::AddItems);
                r.button("Remove Items").on_click(Message::RemoveItems);
                r.text(format!("Items: {} | Offset: {:.0}", self.item_count, self.scroll_state.offset.1));
            });

            // Build content for scrollable
            let mut content_ctx = Context::new();
            for i in 0..self.item_count {
                content_ctx.text(format!("Item {} - Scrollable content here", i + 1));
            }
            let content = Element::new(Column::new(content_ctx.take()));

            // Create scrollable widget with always-visible scrollbar
            let scrollable = Scrollable::new(content)
                .state(&self.scroll_state)
                .direction(ScrollDirection::Vertical)
                .scrollbar_visibility(ScrollbarVisibility::Always)
                .height(400.0)
                .on_scroll(Message::Scrolled);

            c.add(Element::new(scrollable));

            // Instructions
            c.text("Scroll: Mouse wheel | Drag scrollbar");
        })
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Scrolled(state) => {
                log::info!("Scroll updated: offset={:?}", state.offset);
                self.scroll_state = state;
            }
            Message::AddItems => {
                self.item_count = (self.item_count + 10).min(100);
                log::info!("Added items, now: {}", self.item_count);
            }
            Message::RemoveItems => {
                self.item_count = self.item_count.saturating_sub(10).max(5);
                log::info!("Removed items, now: {}", self.item_count);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default()
        .title("Scrollable Widget Demo")
        .size(800, 600);

    hvat_ui::run_with_settings(ScrollableDemo::default(), settings)
}
