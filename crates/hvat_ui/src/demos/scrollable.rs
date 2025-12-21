//! Scrollable widget demo

use crate::element::Element;
use crate::prelude::*;
use crate::widgets::{Column, ScrollDirection, Scrollable, ScrollbarVisibility};
use crate::Context;

/// Scrollable demo state
pub struct ScrollableDemo {
    pub scroll_state: ScrollState,
    pub item_count: usize,
}

/// Scrollable demo messages
#[derive(Clone)]
pub enum ScrollableMessage {
    Scrolled(ScrollState),
    AddItems,
    RemoveItems,
}

impl Default for ScrollableDemo {
    fn default() -> Self {
        Self {
            scroll_state: ScrollState::default(),
            item_count: 30,
        }
    }
}

impl ScrollableDemo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view<M: Clone + 'static>(
        &self,
        wrap: impl Fn(ScrollableMessage) -> M + Clone + 'static,
    ) -> Element<M> {
        let wrap_add = wrap.clone();
        let wrap_remove = wrap.clone();
        let wrap_scroll = wrap.clone();

        let item_count = self.item_count;
        let scroll_offset = self.scroll_state.offset.1;
        let scroll_state = self.scroll_state.clone();

        col(move |c| {
            c.text("Scrollable Widget Demo");

            let add_msg = wrap_add(ScrollableMessage::AddItems);
            let remove_msg = wrap_remove(ScrollableMessage::RemoveItems);

            c.row(|r| {
                r.button("Add Items").on_click(add_msg);
                r.button("Remove Items").on_click(remove_msg);
                r.text(format!(
                    "Items: {} | Offset: {:.0}",
                    item_count, scroll_offset
                ));
            });

            // Build content for scrollable
            let mut content_ctx = Context::new();
            for i in 0..item_count {
                content_ctx.text(format!("Item {} - Scrollable content here", i + 1));
            }
            let content = Element::new(Column::new(content_ctx.take()));

            let wrap_scroll_inner = wrap_scroll.clone();
            let scrollable = Scrollable::new(content)
                .state(&scroll_state)
                .direction(ScrollDirection::Vertical)
                .scrollbar_visibility(ScrollbarVisibility::Always)
                .height(350.0)
                .on_scroll(move |s| wrap_scroll_inner(ScrollableMessage::Scrolled(s)));

            c.add(Element::new(scrollable));

            c.text("Scroll: Mouse wheel | Drag scrollbar");
        })
    }

    pub fn update(&mut self, message: ScrollableMessage) {
        match message {
            ScrollableMessage::Scrolled(state) => {
                log::info!("Scroll updated: offset={:?}", state.offset);
                self.scroll_state = state;
            }
            ScrollableMessage::AddItems => {
                self.item_count = (self.item_count + 10).min(100);
                log::info!("Added items, now: {}", self.item_count);
            }
            ScrollableMessage::RemoveItems => {
                self.item_count = self.item_count.saturating_sub(10).max(5);
                log::info!("Removed items, now: {}", self.item_count);
            }
        }
    }
}
