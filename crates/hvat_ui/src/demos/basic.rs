//! Basic button counter demo

use crate::element::Element;
use crate::prelude::*;

/// Basic demo state
#[derive(Default)]
pub struct BasicDemo {
    pub click_count: u32,
}

/// Basic demo messages
#[derive(Clone)]
pub enum BasicMessage {
    Clicked,
}

impl BasicDemo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view<M: Clone + 'static>(&self, wrap: impl Fn(BasicMessage) -> M + Clone + 'static) -> Element<M> {
        let click_count = self.click_count;
        col(move |c| {
            c.text("Basic Demo");
            c.text("A simple button counter example").size(12.0);

            let msg = wrap(BasicMessage::Clicked);
            c.row(|r| {
                r.button("Click me!").on_click(msg);
                r.text(format!("Clicked {} times", click_count));
            });

            c.text("Click the button to increment the counter.");
        })
    }

    pub fn update(&mut self, message: BasicMessage) {
        match message {
            BasicMessage::Clicked => {
                self.click_count += 1;
                log::info!("Button clicked! Count: {}", self.click_count);
            }
        }
    }
}
