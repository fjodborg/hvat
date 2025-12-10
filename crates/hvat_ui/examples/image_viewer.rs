//! Image Viewer Example
//!
//! Demonstrates the image viewer widget with pan/zoom capabilities.

use hvat_ui::prelude::*;

/// Application state
struct ImageViewerDemo {
    /// Viewer state (pan, zoom, etc.)
    viewer_state: ImageViewerState,
    /// Show controls toggle
    show_controls: bool,
}

/// Application messages
#[derive(Clone)]
enum Message {
    /// Viewer state changed
    ViewerChanged(ImageViewerState),
    /// Toggle controls visibility
    ToggleControls,
    /// Reset view
    ResetView,
    /// Zoom in
    ZoomIn,
    /// Zoom out
    ZoomOut,
}

impl Default for ImageViewerDemo {
    fn default() -> Self {
        Self {
            viewer_state: ImageViewerState::new(),
            show_controls: true,
        }
    }
}

impl Application for ImageViewerDemo {
    type Message = Message;

    fn view(&self) -> Element<Message> {
        hvat_ui::col(|c| {
            // Info bar at top
            c.row(|r| {
                r.text(format!(
                    "Zoom: {:.0}% | Pan: ({:.2}, {:.2})",
                    self.viewer_state.zoom * 100.0,
                    self.viewer_state.pan.0,
                    self.viewer_state.pan.1
                ));
            });

            // Control bar
            c.row(|r| {
                r.button("Reset View").on_click(Message::ResetView);
                r.button("Zoom In").on_click(Message::ZoomIn);
                r.button("Zoom Out").on_click(Message::ZoomOut);
                r.button(if self.show_controls {
                    "Hide Controls"
                } else {
                    "Show Controls"
                })
                .on_click(Message::ToggleControls);
            });

            // Instructions
            c.text("Drag to pan | Scroll to zoom | +/- keys | 0 for 1:1 | F for fit");
        })
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::ViewerChanged(state) => {
                self.viewer_state = state;
            }
            Message::ToggleControls => {
                self.show_controls = !self.show_controls;
            }
            Message::ResetView => {
                self.viewer_state.reset();
            }
            Message::ZoomIn => {
                self.viewer_state.zoom_in();
            }
            Message::ZoomOut => {
                self.viewer_state.zoom_out();
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = hvat_ui::Settings::default()
        .title("Image Viewer Demo")
        .size(1024, 768);

    hvat_ui::run_with_settings(ImageViewerDemo::default(), settings)
}
