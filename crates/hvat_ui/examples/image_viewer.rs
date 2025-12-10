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
    /// Texture ID (set in setup)
    texture_id: Option<TextureId>,
    /// Texture dimensions
    texture_size: (u32, u32),
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
            texture_id: None,
            texture_size: (0, 0),
        }
    }
}

/// Create a simple test pattern texture (checkerboard with colors)
fn create_test_pattern(width: u32, height: u32) -> Vec<u8> {
    let mut data = vec![0u8; (width * height * 4) as usize];
    let tile_size = 32;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let tile_x = (x / tile_size) as usize;
            let tile_y = (y / tile_size) as usize;

            // Create a colored checkerboard pattern
            let (r, g, b) = if (tile_x + tile_y) % 2 == 0 {
                // Gradient based on position
                (
                    ((x as f32 / width as f32) * 255.0) as u8,
                    ((y as f32 / height as f32) * 255.0) as u8,
                    128u8,
                )
            } else {
                // Alternate color
                (40, 40, 50)
            };

            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255; // Alpha
        }
    }

    data
}

impl Application for ImageViewerDemo {
    type Message = Message;

    fn setup(&mut self, resources: &mut Resources) {
        // Create a test texture
        let width = 512;
        let height = 512;
        let pattern = create_test_pattern(width, height);

        // Create GPU texture
        let gpu_ctx = resources.gpu_context();
        match hvat_ui::Texture::from_rgba8(gpu_ctx, &pattern, width, height) {
            Ok(texture) => {
                // Register with renderer
                let id = resources.register_texture(&texture);
                self.texture_id = Some(id);
                self.texture_size = (width, height);
                log::info!("Created test texture {}x{}", width, height);
            }
            Err(e) => {
                log::error!("Failed to create texture: {:?}", e);
            }
        }
    }

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

            // Image viewer
            if let Some(texture_id) = self.texture_id {
                c.image_viewer(texture_id, self.texture_size.0, self.texture_size.1)
                    .state(&self.viewer_state)
                    .show_controls(self.show_controls)
                    .on_change(Message::ViewerChanged)
                    .build();
            } else {
                c.image_viewer_empty()
                    .state(&self.viewer_state)
                    .show_controls(self.show_controls)
                    .build();
            }

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
