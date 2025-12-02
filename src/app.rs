use std::sync::Arc;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use hvat_gpu::{context::GpuContext, pipeline::*, texture::Texture};

pub struct ViewTransform {
    offset_x: f32,
    offset_y: f32,
    zoom: f32,
}

impl ViewTransform {
    pub fn new() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
        }
    }

    pub fn zoom_at_point(&mut self, cursor_x: f32, cursor_y: f32, zoom_factor: f32) {
        let new_zoom = (self.zoom * zoom_factor).clamp(0.1, 10.0);
        let zoom_ratio = new_zoom / self.zoom;

        let cursor_rel_x = cursor_x - self.offset_x;
        let cursor_rel_y = cursor_y - self.offset_y;
        self.offset_x -= cursor_rel_x * (zoom_ratio - 1.0);
        self.offset_y -= cursor_rel_y * (zoom_ratio - 1.0);

        self.zoom = new_zoom;
    }

    pub fn reset(&mut self) {
        self.offset_x = 0.0;
        self.offset_y = 0.0;
        self.zoom = 1.0;
    }
}

pub struct AppState {
    pub gpu_ctx: GpuContext,
    pub pipeline: TexturePipeline,
    pub texture_bind_group: wgpu::BindGroup,
    pub transform: ViewTransform,
    pub is_dragging: bool,
    pub last_cursor_pos: Option<PhysicalPosition<f64>>,
}

impl AppState {
    pub fn new(
        gpu_ctx: GpuContext,
        pipeline: TexturePipeline,
        texture_bind_group: wgpu::BindGroup,
    ) -> Self {
        Self {
            gpu_ctx,
            pipeline,
            texture_bind_group,
            transform: ViewTransform::new(),
            is_dragging: false,
            last_cursor_pos: None,
        }
    }

    pub fn render(&mut self) {
        // Get the next frame
        let frame = match self.gpu_ctx.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("Failed to get frame: {:?}", e).into());
                #[cfg(not(target_arch = "wasm32"))]
                eprintln!("Failed to get frame: {:?}", e);
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Update transform uniform
        let transform_uniform = TransformUniform::from_transform(
            self.transform.offset_x,
            self.transform.offset_y,
            self.transform.zoom,
        );
        self.pipeline.update_transform(&self.gpu_ctx, transform_uniform);

        // Create command encoder
        let mut encoder = self
            .gpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Render
        self.pipeline.render(&mut encoder, &view, &self.texture_bind_group);

        // Submit commands
        self.gpu_ctx.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    pub fn handle_window_event(&mut self, event: WindowEvent, window: &Window) {
        match event {
            WindowEvent::Resized(new_size) => {
                self.gpu_ctx.resize(new_size.width, new_size.height);
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::MouseInput {
                state: button_state,
                button: MouseButton::Left,
                ..
            } => {
                self.is_dragging = button_state == ElementState::Pressed;
                if !self.is_dragging {
                    self.last_cursor_pos = None;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.is_dragging {
                    if let Some(last_pos) = self.last_cursor_pos {
                        let delta_x = position.x - last_pos.x;
                        let delta_y = position.y - last_pos.y;

                        let norm_dx = (delta_x as f32 / self.gpu_ctx.width() as f32) * 2.0;
                        let norm_dy = (delta_y as f32 / self.gpu_ctx.height() as f32) * 2.0;

                        self.transform.offset_x += norm_dx;
                        self.transform.offset_y -= norm_dy;

                        window.request_redraw();
                    }
                    self.last_cursor_pos = Some(position);
                } else {
                    self.last_cursor_pos = Some(position);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_delta = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as f32,
                };

                let zoom_factor = if scroll_delta > 0.0 { 1.25 } else { 0.8 };

                if let Some(cursor_pos) = self.last_cursor_pos {
                    let norm_x = (cursor_pos.x as f32 / self.gpu_ctx.width() as f32) * 2.0 - 1.0;
                    let norm_y = -((cursor_pos.y as f32 / self.gpu_ctx.height() as f32) * 2.0 - 1.0);

                    self.transform.zoom_at_point(norm_x, norm_y, zoom_factor);
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::{KeyCode, PhysicalKey};
                if event.state == ElementState::Pressed {
                    if let PhysicalKey::Code(KeyCode::KeyR) = event.physical_key {
                        self.transform.reset();
                        window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }
}

pub async fn initialize_app(window: Arc<Window>) -> Result<AppState, String> {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&"Creating GPU context...".into());
    #[cfg(not(target_arch = "wasm32"))]
    println!("Creating GPU context...");

    // Initialize GPU context
    let gpu_ctx = match GpuContext::new(window.clone()).await {
        Ok(ctx) => {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&"✓ GPU context created".into());
            #[cfg(not(target_arch = "wasm32"))]
            println!("✓ GPU context created");
            ctx
        }
        Err(e) => {
            let error_msg = format!("GPU initialization failed: {:?}", e);
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&format!("✗ {}", error_msg).into());
            #[cfg(not(target_arch = "wasm32"))]
            eprintln!("✗ {}", error_msg);
            return Err(error_msg);
        }
    };

    // Create pipeline
    let pipeline = TexturePipeline::new(&gpu_ctx);
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&"✓ Pipeline created".into());
    #[cfg(not(target_arch = "wasm32"))]
    println!("✓ Pipeline created");

    // Generate test image
    let test_data = crate::generate_test_image(512, 512);
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&"✓ Test image generated".into());
    #[cfg(not(target_arch = "wasm32"))]
    println!("✓ Test image generated");

    // Create texture
    let texture = match Texture::from_rgba8(&gpu_ctx, &test_data, 512, 512) {
        Ok(tex) => {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&"✓ Texture uploaded to GPU".into());
            #[cfg(not(target_arch = "wasm32"))]
            println!("✓ Texture uploaded to GPU");
            tex
        }
        Err(e) => {
            let error_msg = format!("Texture creation failed: {:?}", e);
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&format!("✗ {}", error_msg).into());
            #[cfg(not(target_arch = "wasm32"))]
            eprintln!("✗ {}", error_msg);
            return Err(error_msg);
        }
    };

    let texture_bind_group = pipeline.create_texture_bind_group(&gpu_ctx, &texture);

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&"✓ All resources initialized!".into());
    #[cfg(not(target_arch = "wasm32"))]
    println!("✓ All resources initialized!");

    Ok(AppState::new(gpu_ctx, pipeline, texture_bind_group))
}

pub fn run_event_loop(event_loop: EventLoop<()>, window: Arc<Window>, mut state: AppState) {
    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { event, .. } => {
                if let WindowEvent::CloseRequested = event {
                    elwt.exit();
                } else {
                    state.handle_window_event(event, &window);
                }
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
