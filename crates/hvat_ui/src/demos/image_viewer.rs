//! Image viewer widget demo

use crate::prelude::*;
use crate::Element;

/// Image viewer demo state
pub struct ImageViewerDemo {
    pub viewer_state: ImageViewerState,
    pub show_controls: bool,
    pub texture_id: Option<TextureId>,
    pub texture_size: (u32, u32),
}

/// Image viewer demo messages
#[derive(Clone)]
pub enum ImageViewerMessage {
    ViewerChanged(ImageViewerState),
    ToggleControls,
    ResetView,
    ZoomIn,
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
pub fn create_test_pattern(width: u32, height: u32) -> Vec<u8> {
    let mut data = vec![0u8; (width * height * 4) as usize];
    let tile_size = 32;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let tile_x = (x / tile_size) as usize;
            let tile_y = (y / tile_size) as usize;

            let (r, g, b) = if (tile_x + tile_y) % 2 == 0 {
                (
                    ((x as f32 / width as f32) * 255.0) as u8,
                    ((y as f32 / height as f32) * 255.0) as u8,
                    128u8,
                )
            } else {
                (40, 40, 50)
            };

            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    data
}

impl ImageViewerDemo {
    pub fn new() -> Self {
        Self::default()
    }

    /// Setup the demo with a test texture. Call this in Application::setup()
    pub fn setup(&mut self, resources: &mut Resources) {
        let width = 4096;
        let height = 4096;
        let pattern = create_test_pattern(width, height);

        let gpu_ctx = resources.gpu_context();
        match crate::Texture::from_rgba8(gpu_ctx, &pattern, width, height) {
            Ok(texture) => {
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

    pub fn view<M: Clone + 'static>(&self, wrap: impl Fn(ImageViewerMessage) -> M + Clone + 'static) -> Element<M> {
        let viewer_state = self.viewer_state.clone();
        let show_controls = self.show_controls;
        let texture_id = self.texture_id;
        let texture_size = self.texture_size;

        let wrap_reset = wrap.clone();
        let wrap_zoom_in = wrap.clone();
        let wrap_zoom_out = wrap.clone();
        let wrap_toggle = wrap.clone();
        let wrap_viewer = wrap.clone();

        crate::col(move |c| {
            // Info bar
            c.row(|r| {
                r.text(format!(
                    "Zoom: {:.0}% | Pan: ({:.2}, {:.2})",
                    viewer_state.zoom * 100.0,
                    viewer_state.pan.0,
                    viewer_state.pan.1
                ));
            });

            // Control bar
            let reset_msg = wrap_reset(ImageViewerMessage::ResetView);
            let zoom_in_msg = wrap_zoom_in(ImageViewerMessage::ZoomIn);
            let zoom_out_msg = wrap_zoom_out(ImageViewerMessage::ZoomOut);
            let toggle_msg = wrap_toggle(ImageViewerMessage::ToggleControls);

            c.row(|r| {
                r.button("Reset View").on_click(reset_msg);
                r.button("Zoom In").on_click(zoom_in_msg);
                r.button("Zoom Out").on_click(zoom_out_msg);
                r.button(if show_controls { "Hide Controls" } else { "Show Controls" })
                    .on_click(toggle_msg);
            });

            // Image viewer
            if let Some(tex_id) = texture_id {
                let wrap_change = wrap_viewer.clone();
                c.image_viewer(tex_id, texture_size.0, texture_size.1)
                    .state(&viewer_state)
                    .show_controls(show_controls)
                    .on_change(move |s| wrap_change(ImageViewerMessage::ViewerChanged(s)))
                    .build();
            } else {
                c.image_viewer_empty()
                    .state(&viewer_state)
                    .show_controls(show_controls)
                    .build();
            }

            c.text("Drag to pan | Scroll to zoom | +/- keys | 0 for 1:1 | F for fit");
        })
    }

    pub fn update(&mut self, message: ImageViewerMessage) {
        match message {
            ImageViewerMessage::ViewerChanged(state) => {
                self.viewer_state = state;
            }
            ImageViewerMessage::ToggleControls => {
                self.show_controls = !self.show_controls;
            }
            ImageViewerMessage::ResetView => {
                self.viewer_state.reset();
            }
            ImageViewerMessage::ZoomIn => {
                self.viewer_state.zoom_in();
            }
            ImageViewerMessage::ZoomOut => {
                self.viewer_state.zoom_out();
            }
        }
    }
}
