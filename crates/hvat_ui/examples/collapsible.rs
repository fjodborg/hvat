//! Collapsible Widget Example (Standalone)
//!
//! Run with: cargo run --example collapsible

use hvat_ui::demos::{CollapsibleDemo, CollapsibleMessage};
use hvat_ui::prelude::*;

struct App {
    demo: CollapsibleDemo,
}

impl Default for App {
    fn default() -> Self {
        Self {
            demo: CollapsibleDemo::new(),
        }
    }
}

impl Application for App {
    type Message = CollapsibleMessage;

    fn view(&self) -> Element<Self::Message> {
        self.demo.view(|msg| msg)
    }

    fn update(&mut self, message: Self::Message) {
        self.demo.update(message);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default()
        .title("Collapsible Widget Demo")
        .size(800, 700);

    hvat_ui::run_with_settings(App::default(), settings)
}
