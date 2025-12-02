use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::web::WindowBuilderExtWebSys,
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
    gpu_ctx: Option<GpuContext>,
    pipeline: Option<TexturePipeline>,
    texture_bind_group: Option<wgpu::BindGroup>,
    transform: ViewTransform,
    is_dragging: bool,
    last_cursor_pos: Option<PhysicalPosition<f64>>,
}

impl AppState {
    fn render(&mut self) {
        let Some(ref gpu_ctx) = self.gpu_ctx else {
            return;
        };
        let Some(ref pipeline) = self.pipeline else {
            return;
        };
        let Some(ref texture_bind_group) = self.texture_bind_group else {
            return;
        };

        // Get the next frame
        let frame = match gpu_ctx.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                web_sys::console::log_1(&"Failed to get frame".into());
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
        pipeline.update_transform(gpu_ctx, transform_uniform);

        // Create command encoder
        let mut encoder = gpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Render
        pipeline.render(&mut encoder, &view, texture_bind_group);

        // Submit commands
        gpu_ctx.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(async {
        run().await;
    });
}

async fn run() {
    web_sys::console::log_1(&"HVAT WASM starting...".into());

    let event_loop = EventLoop::new().expect("couldn't create event loop");

    // Get the window from the document
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    let canvas = document
        .create_element("canvas")
        .expect("couldn't create canvas")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("couldn't convert to canvas");

    canvas.set_width(800);
    canvas.set_height(600);

    body.append_child(&canvas)
        .expect("couldn't append canvas to body");

    let window = WindowBuilder::new()
        .with_title("HVAT - GPU Test (WASM)")
        .with_canvas(Some(canvas))
        .build(&event_loop)
        .expect("couldn't create window");

    let window = Arc::new(window);

    web_sys::console::log_1(&"Creating GPU context...".into());

    // Initialize GPU context
    let gpu_ctx = match GpuContext::new(window.clone()).await {
        Ok(ctx) => {
            web_sys::console::log_1(&"✓ GPU context created".into());
            ctx
        }
        Err(e) => {
            web_sys::console::log_1(&format!("✗ GPU initialization failed: {:?}", e).into());
            return;
        }
    };

    // Create pipeline
    let pipeline = TexturePipeline::new(&gpu_ctx);
    web_sys::console::log_1(&"✓ Pipeline created".into());

    // Generate test image
    let test_data = crate::generate_test_image(512, 512);
    web_sys::console::log_1(&"✓ Test image generated".into());

    // Create texture
    let texture = match Texture::from_rgba8(&gpu_ctx, &test_data, 512, 512) {
        Ok(tex) => {
            web_sys::console::log_1(&"✓ Texture uploaded to GPU".into());
            tex
        }
        Err(e) => {
            web_sys::console::log_1(&format!("✗ Texture creation failed: {:?}", e).into());
            return;
        }
    };

    let texture_bind_group = pipeline.create_texture_bind_group(&gpu_ctx, &texture);

    let state = Rc::new(RefCell::new(AppState {
        gpu_ctx: Some(gpu_ctx),
        pipeline: Some(pipeline),
        texture_bind_group: Some(texture_bind_group),
        transform: ViewTransform::new(),
        is_dragging: false,
        last_cursor_pos: None,
    }));

    web_sys::console::log_1(&"✓ All resources initialized!".into());
    web_sys::console::log_1(&"Controls: Drag to pan, scroll to zoom, R to reset".into());
    web_sys::console::log_1(&"ℹ️  Note: Winit uses exceptions for control flow - any exception errors below are expected and can be ignored.".into());

    let state_clone = state.clone();
    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { event, .. } => {
                let mut state = state_clone.borrow_mut();

                match event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::Resized(new_size) => {
                        if let Some(ref mut gpu_ctx) = state.gpu_ctx {
                            gpu_ctx.resize(new_size.width, new_size.height);
                        }
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

                                if let Some(ref gpu_ctx) = state.gpu_ctx {
                                    let norm_dx = (delta_x as f32 / gpu_ctx.width() as f32) * 2.0;
                                    let norm_dy = (delta_y as f32 / gpu_ctx.height() as f32) * 2.0;

                                    state.transform.offset_x += norm_dx;
                                    state.transform.offset_y -= norm_dy;

                                    window.request_redraw();
                                }
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

                        if let (Some(cursor_pos), Some(gpu_ctx)) =
                            (state.last_cursor_pos, &state.gpu_ctx)
                        {
                            let norm_x = (cursor_pos.x as f32 / gpu_ctx.width() as f32) * 2.0 - 1.0;
                            let norm_y = -((cursor_pos.y as f32 / gpu_ctx.height() as f32) * 2.0 - 1.0);

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
                }
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
