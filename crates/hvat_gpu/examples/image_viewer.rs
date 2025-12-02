use hvat_gpu::{GpuContext, Texture, TexturePipeline, TransformUniform};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

struct ViewTransform {
    offset_x: f32,
    offset_y: f32,
    zoom: f32,
}

impl ViewTransform {
    fn new() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
        }
    }

    fn to_uniform(&self) -> TransformUniform {
        TransformUniform::from_transform(self.offset_x, self.offset_y, self.zoom)
    }

    /// Pan by delta in screen space
    fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.offset_x += delta_x;
        self.offset_y += delta_y;
    }

    /// Zoom around a point (in clip space -1 to 1)
    fn zoom_at_point(&mut self, cursor_x: f32, cursor_y: f32, zoom_factor: f32) {
        let new_zoom = (self.zoom * zoom_factor).clamp(0.1, 10.0);
        let zoom_ratio = new_zoom / self.zoom;

        // Calculate cursor position relative to current transform
        let cursor_rel_x = cursor_x - self.offset_x;
        let cursor_rel_y = cursor_y - self.offset_y;

        // Adjust offset so point under cursor stays fixed
        self.offset_x -= cursor_rel_x * (zoom_ratio - 1.0);
        self.offset_y -= cursor_rel_y * (zoom_ratio - 1.0);

        self.zoom = new_zoom;
    }

    fn reset(&mut self) {
        self.offset_x = 0.0;
        self.offset_y = 0.0;
        self.zoom = 1.0;
    }
}

struct InputState {
    mouse_pressed: bool,
    last_mouse_pos: Option<(f64, f64)>,
}

impl InputState {
    fn new() -> Self {
        Self {
            mouse_pressed: false,
            last_mouse_pos: None,
        }
    }
}

struct AppState {
    gpu_ctx: GpuContext,
    pipeline: TexturePipeline,
    texture: Texture,
    texture_bind_group: wgpu::BindGroup,
    transform: ViewTransform,
    input: InputState,
}

impl AppState {
    async fn new(window: Arc<Window>) -> Self {
        // Initialize GPU
        let gpu_ctx = GpuContext::new(window.clone())
            .await
            .expect("Failed to create GPU context");

        // Create pipeline
        let pipeline = TexturePipeline::new(&gpu_ctx);

        // Create a test image (colorful gradient)
        let width = 800;
        let height = 600;
        let mut image_data = vec![0u8; (width * height * 4) as usize];

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                image_data[idx] = (x * 255 / width) as u8; // R
                image_data[idx + 1] = (y * 255 / height) as u8; // G
                image_data[idx + 2] = 128; // B
                image_data[idx + 3] = 255; // A
            }
        }

        let texture = Texture::from_rgba8(&gpu_ctx, &image_data, width, height)
            .expect("Failed to create texture");

        let texture_bind_group = pipeline.create_texture_bind_group(&gpu_ctx, &texture);

        Self {
            gpu_ctx,
            pipeline,
            texture,
            texture_bind_group,
            transform: ViewTransform::new(),
            input: InputState::new(),
        }
    }

    fn resize(&mut self, new_width: u32, new_height: u32) {
        self.gpu_ctx.resize(new_width, new_height);
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    self.input.mouse_pressed = *state == ElementState::Pressed;
                    if !self.input.mouse_pressed {
                        self.input.last_mouse_pos = None;
                    }
                    return true;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.input.mouse_pressed {
                    if let Some((last_x, last_y)) = self.input.last_mouse_pos {
                        let delta_x = (position.x - last_x) as f32;
                        let delta_y = (position.y - last_y) as f32;

                        // Convert screen space delta to clip space
                        let clip_delta_x = delta_x / (self.gpu_ctx.width() as f32 / 2.0);
                        let clip_delta_y = -delta_y / (self.gpu_ctx.height() as f32 / 2.0);

                        self.transform.pan(clip_delta_x, clip_delta_y);
                    }
                    self.input.last_mouse_pos = Some((position.x, position.y));
                    return true;
                }
                self.input.last_mouse_pos = Some((position.x, position.y));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 20.0,
                };

                // Get cursor position in clip space
                let (cursor_clip_x, cursor_clip_y) = if let Some((x, y)) = self.input.last_mouse_pos
                {
                    let clip_x = (x as f32 / self.gpu_ctx.width() as f32) * 2.0 - 1.0;
                    let clip_y = -((y as f32 / self.gpu_ctx.height() as f32) * 2.0 - 1.0);
                    (clip_x, clip_y)
                } else {
                    (0.0, 0.0) // Center if no cursor position
                };

                // Multiplicative zoom: 25% per scroll notch
                let zoom_factor = if scroll_delta > 0.0 { 1.25 } else { 1.0 / 1.25 };
                self.transform
                    .zoom_at_point(cursor_clip_x, cursor_clip_y, zoom_factor);

                return true;
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                match keycode {
                    KeyCode::KeyR => {
                        self.transform.reset();
                        return true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        false
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Get surface texture
        let output = self.gpu_ctx.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Update transform uniform
        self.pipeline
            .update_transform(&self.gpu_ctx, self.transform.to_uniform());

        // Create command encoder
        let mut encoder = self
            .gpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Render
        self.pipeline
            .render(&mut encoder, &view, &self.texture_bind_group);

        // Submit commands
        self.gpu_ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

struct ImageViewerApp {
    window: Option<Arc<Window>>,
    state: Option<AppState>,
}

impl ImageViewerApp {
    fn new() -> Self {
        Self {
            window: None,
            state: None,
        }
    }
}

impl ApplicationHandler for ImageViewerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_title("Image Viewer - Pan with mouse, zoom with scroll wheel, R to reset")
                .with_inner_size(winit::dpi::LogicalSize::new(1024, 768));

            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            self.window = Some(window.clone());

            // Initialize app state asynchronously
            #[cfg(not(target_arch = "wasm32"))]
            {
                let state = pollster::block_on(AppState::new(window));
                self.state = Some(state);
            }

            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let state = AppState::new(window).await;
                    self.state = Some(state);
                });
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(state) = &mut self.state {
            if !state.input(&event) {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                        ..
                    } => {
                        event_loop.exit();
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(physical_size.width, physical_size.height);
                    }
                    WindowEvent::RedrawRequested => {
                        match state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => {
                                state.resize(state.gpu_ctx.width(), state.gpu_ctx.height())
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                            Err(e) => eprintln!("Render error: {:?}", e),
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = ImageViewerApp::new();
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = ImageViewerApp::new();
    event_loop.run_app(&mut app).unwrap();
}
