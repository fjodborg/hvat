use std::sync::Arc;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use hvat_gpu::{context::GpuContext, pipeline::*, texture::Texture};

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

    fn zoom_at_point(&mut self, cursor_x: f32, cursor_y: f32, zoom_factor: f32) {
        let new_zoom = (self.zoom * zoom_factor).clamp(0.1, 10.0);
        let zoom_ratio = new_zoom / self.zoom;

        let cursor_rel_x = cursor_x - self.offset_x;
        let cursor_rel_y = cursor_y - self.offset_y;
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

struct AppState {
    gpu_ctx: GpuContext,
    pipeline: TexturePipeline,
    texture_bind_group: wgpu::BindGroup,
    transform: ViewTransform,
    is_dragging: bool,
    last_cursor_pos: Option<PhysicalPosition<f64>>,
}

impl AppState {
    fn render(&mut self) {
        // Get the next frame
        let frame = match self.gpu_ctx.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
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
}

pub fn run() {
    println!("HVAT - Native Application");
    println!("Controls: Drag to pan, scroll to zoom, R to reset");
    println!();

    let event_loop = EventLoop::new().expect("couldn't create event loop");

    let window = WindowBuilder::new()
        .with_title("HVAT - GPU Test (Native)")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .expect("couldn't create window");

    let window = Arc::new(window);

    println!("Creating GPU context...");

    // Initialize GPU context (blocking on native)
    let gpu_ctx = match pollster::block_on(GpuContext::new(window.clone())) {
        Ok(ctx) => {
            println!("✓ GPU context created");
            ctx
        }
        Err(e) => {
            eprintln!("✗ GPU initialization failed: {:?}", e);
            return;
        }
    };

    // Create pipeline
    let pipeline = TexturePipeline::new(&gpu_ctx);
    println!("✓ Pipeline created");

    // Generate test image
    let test_data = crate::generate_test_image(512, 512);
    println!("✓ Test image generated");

    // Create texture
    let texture = match Texture::from_rgba8(&gpu_ctx, &test_data, 512, 512) {
        Ok(tex) => {
            println!("✓ Texture uploaded to GPU");
            tex
        }
        Err(e) => {
            eprintln!("✗ Texture creation failed: {:?}", e);
            return;
        }
    };

    let texture_bind_group = pipeline.create_texture_bind_group(&gpu_ctx, &texture);

    let mut state = AppState {
        gpu_ctx,
        pipeline,
        texture_bind_group,
        transform: ViewTransform::new(),
        is_dragging: false,
        last_cursor_pos: None,
    };

    println!("✓ All resources initialized!");
    println!();

    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                }
                WindowEvent::Resized(new_size) => {
                    state.gpu_ctx.resize(new_size.width, new_size.height);
                }
                WindowEvent::RedrawRequested => {
                    state.render();
                }
                WindowEvent::MouseInput {
                    state: button_state,
                    button: MouseButton::Left,
                    ..
                } => {
                    state.is_dragging = button_state == ElementState::Pressed;
                    if !state.is_dragging {
                        state.last_cursor_pos = None;
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if state.is_dragging {
                        if let Some(last_pos) = state.last_cursor_pos {
                            let delta_x = position.x - last_pos.x;
                            let delta_y = position.y - last_pos.y;

                            let norm_dx = (delta_x as f32 / state.gpu_ctx.width() as f32) * 2.0;
                            let norm_dy = (delta_y as f32 / state.gpu_ctx.height() as f32) * 2.0;

                            state.transform.offset_x += norm_dx;
                            state.transform.offset_y -= norm_dy;

                            window.request_redraw();
                        }
                        state.last_cursor_pos = Some(position);
                    } else {
                        state.last_cursor_pos = Some(position);
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let scroll_delta = match delta {
                        MouseScrollDelta::LineDelta(_x, y) => y,
                        MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as f32,
                    };

                    let zoom_factor = if scroll_delta > 0.0 { 1.25 } else { 0.8 };

                    if let Some(cursor_pos) = state.last_cursor_pos {
                        let norm_x = (cursor_pos.x as f32 / state.gpu_ctx.width() as f32) * 2.0 - 1.0;
                        let norm_y = -((cursor_pos.y as f32 / state.gpu_ctx.height() as f32) * 2.0 - 1.0);

                        state.transform.zoom_at_point(norm_x, norm_y, zoom_factor);
                        window.request_redraw();
                    }
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    use winit::keyboard::{KeyCode, PhysicalKey};
                    if event.state == ElementState::Pressed {
                        if let PhysicalKey::Code(KeyCode::KeyR) = event.physical_key {
                            state.transform.reset();
                            window.request_redraw();
                        }
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
