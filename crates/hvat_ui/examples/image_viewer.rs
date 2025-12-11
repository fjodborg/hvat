//! Image Viewer Example (Standalone)
//!
//! Run with: cargo run --example image_viewer

use hvat_ui::demos::{ImageViewerDemo, ImageViewerMessage};
use hvat_ui::prelude::*;

struct App {
    demo: ImageViewerDemo,
}

impl Default for App {
    fn default() -> Self {
        Self {
            demo: ImageViewerDemo::new(),
        }
    }
}

impl Application for App {
    type Message = ImageViewerMessage;

    fn setup(&mut self, resources: &mut Resources) {
        self.demo.setup(resources);
    }

    fn view(&self) -> Element<Self::Message> {
        self.demo.view(|msg| msg)
    }

    fn update(&mut self, message: Self::Message) {
        self.demo.update(message);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default()
        .title("Image Viewer Demo")
        .size(1024, 768);

    hvat_ui::run_with_settings(App::default(), settings)
}
