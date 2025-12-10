//! hvat_ui example application
//!
//! This is a simple demo showing the UI framework capabilities.

use hvat_ui::prelude::*;

/// Demo application state
struct DemoApp {
    click_count: u32,
}

/// Demo messages
#[derive(Clone)]
enum Message {
    Clicked,
}

impl Default for DemoApp {
    fn default() -> Self {
        Self { click_count: 0 }
    }
}

impl Application for DemoApp {
    type Message = Message;

    fn view(&self) -> Element<Message> {
        hvat_ui::col(|c| {
            c.text("hvat_ui Demo");
            c.text_sized("A simple UI framework", 12.0);

            c.row(|r| {
                r.button("Click me!").on_click(Message::Clicked);
                r.text(format!("Clicked {} times", self.click_count));
            });

            c.text("Use the buttons above to interact with the demo.");
        })
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Clicked => {
                self.click_count += 1;
                log::info!("Button clicked! Count: {}", self.click_count);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = hvat_ui::Settings::default()
        .title("hvat_ui Demo")
        .size(800, 600);

    hvat_ui::run_with_settings(DemoApp::default(), settings)
}
