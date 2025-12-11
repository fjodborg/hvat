//! Dropdown Widget Example (Standalone)
//!
//! Run with: cargo run --example dropdown

use hvat_ui::demos::{DropdownDemo, DropdownMessage};
use hvat_ui::prelude::*;

struct App {
    demo: DropdownDemo,
}

impl Default for App {
    fn default() -> Self {
        Self {
            demo: DropdownDemo::new(),
        }
    }
}

impl Application for App {
    type Message = DropdownMessage;

    fn view(&self) -> Element<Self::Message> {
        self.demo.view(|msg| msg)
    }

    fn update(&mut self, message: Self::Message) {
        self.demo.update(message);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default()
        .title("Dropdown Widget Demo")
        .size(800, 600);

    hvat_ui::run_with_settings(App::default(), settings)
}
