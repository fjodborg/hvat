//! Comprehensive example demonstrating the full hvat_ui framework
//!
//! Run with: cargo run --example comprehensive
//!
//! This example showcases:
//! - Image viewer with pan/zoom in the center
//! - Left sidebar with scrollable collapsible sections
//! - Right sidebar with sliders for image adjustments
//! - Proper three-column layout

use hvat_ui::demos::{ComprehensiveDemo, ComprehensiveMessage};
use hvat_ui::{Application, Element, Event, Resources, Settings};

/// Application wrapper for the comprehensive demo
struct ComprehensiveApp {
    demo: ComprehensiveDemo,
}

impl Application for ComprehensiveApp {
    type Message = ComprehensiveMessage;

    fn setup(&mut self, resources: &mut Resources) {
        self.demo.setup(resources);
    }

    fn view(&self) -> Element<Self::Message> {
        // No wrapping needed - messages go directly through
        self.demo.view(|msg| msg)
    }

    fn update(&mut self, message: Self::Message) {
        self.demo.update(message);
    }

    fn on_event(&mut self, event: &Event) -> Option<Self::Message> {
        // Handle global keyboard shortcuts (Ctrl+Z/Y for undo/redo)
        ComprehensiveDemo::handle_key_event(event)
    }

    fn on_resize(&mut self, width: f32, height: f32) {
        self.demo.set_window_size(width, height);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = ComprehensiveApp {
        demo: ComprehensiveDemo::new(),
    };

    let settings = Settings::new()
        .title("hvat_ui Comprehensive Demo")
        .size(1400, 700)
        .target_fps(60);

    hvat_ui::run_with_settings(app, settings)
}
