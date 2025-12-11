//! Scrollable Widget Example (Standalone)
//!
//! Run with: cargo run --example scrollable

use hvat_ui::demos::{ScrollableDemo, ScrollableMessage};
use hvat_ui::prelude::*;

struct App {
    demo: ScrollableDemo,
}

impl Default for App {
    fn default() -> Self {
        Self {
            demo: ScrollableDemo::new(),
        }
    }
}

impl Application for App {
    type Message = ScrollableMessage;

    fn view(&self) -> Element<Self::Message> {
        self.demo.view(|msg| msg)
    }

    fn update(&mut self, message: Self::Message) {
        self.demo.update(message);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default()
        .title("Scrollable Widget Demo")
        .size(800, 600);

    hvat_ui::run_with_settings(App::default(), settings)
}
