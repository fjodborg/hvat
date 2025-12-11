//! Application trait and runtime

use crate::element::Element;
use crate::event::{Event, KeyCode, KeyModifiers, MouseButton};
use crate::layout::{Bounds, Size};
use crate::renderer::{Renderer, TextureId};
use hvat_gpu::{ClearColor, GpuContext, Texture};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowAttributes, WindowId};

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

/// Application settings
#[derive(Debug, Clone)]
pub struct Settings {
    /// Window title
    pub title: String,
    /// Initial window size
    pub size: (u32, u32),
    /// Background clear color
    pub background: ClearColor,
    /// Whether to enable VSync
    pub vsync: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            title: "hvat_ui Application".to_string(),
            size: (1024, 768),
            background: ClearColor::DARK_GRAY,
            vsync: true,
        }
    }
}

impl Settings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    pub fn background(mut self, color: ClearColor) -> Self {
        self.background = color;
        self
    }

    pub fn vsync(mut self, enabled: bool) -> Self {
        self.vsync = enabled;
        self
    }
}

/// Resources provided to the application for GPU operations
pub struct Resources<'a> {
    gpu_ctx: &'a GpuContext,
    renderer: &'a mut Renderer,
}

impl<'a> Resources<'a> {
    /// Register a texture and return its ID for use in ImageViewer
    pub fn register_texture(&mut self, texture: &Texture) -> TextureId {
        self.renderer.register_texture(self.gpu_ctx, texture)
    }

    /// Unregister a texture
    pub fn unregister_texture(&mut self, id: TextureId) {
        self.renderer.unregister_texture(id);
    }

    /// Get the GPU context for creating textures
    pub fn gpu_context(&self) -> &GpuContext {
        self.gpu_ctx
    }
}

/// The main application trait that users implement
pub trait Application: Sized {
    /// The message type for this application
    type Message: 'static;

    /// Build the view for this application
    fn view(&self) -> Element<Self::Message>;

    /// Handle a message and update state
    fn update(&mut self, message: Self::Message);

    /// Called on application startup with access to GPU resources.
    /// Use this to load textures and other GPU assets.
    fn setup(&mut self, _resources: &mut Resources) {}

    /// Called on application startup (legacy, prefer setup())
    fn init(&mut self) {}

    /// Called each frame before rendering
    fn tick(&mut self) {}
}

/// Internal application state
struct AppState<A: Application> {
    app: A,
    gpu_ctx: GpuContext,
    renderer: Renderer,
    root: Option<Element<A::Message>>,
    window_size: (u32, u32),
    cursor_position: (f32, f32),
    modifiers: KeyModifiers,
    settings: Settings,
}

impl<A: Application> AppState<A> {
    async fn new(window: Arc<Window>, mut app: A, settings: Settings) -> Self {
        let gpu_ctx = GpuContext::new(window.clone())
            .await
            .expect("Failed to create GPU context");

        let mut renderer = Renderer::new(&gpu_ctx);
        let window_size = (gpu_ctx.width(), gpu_ctx.height());

        // Call legacy init first
        app.init();

        // Call setup with GPU resources
        {
            let mut resources = Resources {
                gpu_ctx: &gpu_ctx,
                renderer: &mut renderer,
            };
            app.setup(&mut resources);
        }

        Self {
            app,
            gpu_ctx,
            renderer,
            root: None,
            window_size,
            cursor_position: (0.0, 0.0),
            modifiers: KeyModifiers::default(),
            settings,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.gpu_ctx.resize(width, height);
            self.renderer.resize(width, height);
            self.window_size = (width, height);
            log::debug!("Resized to {}x{}", width, height);
            // Rebuild layout on resize
            self.rebuild_view();
            self.layout();
        }
    }

    fn rebuild_view(&mut self) {
        self.root = Some(self.app.view());
    }

    fn layout(&mut self) {
        if let Some(root) = &mut self.root {
            let available = Size::new(self.window_size.0 as f32, self.window_size.1 as f32);
            root.layout(available);
        }
    }

    fn handle_event(&mut self, event: Event) {
        if let Some(root) = &mut self.root {
            let bounds = Bounds::new(
                0.0,
                0.0,
                self.window_size.0 as f32,
                self.window_size.1 as f32,
            );

            if let Some(message) = root.on_event(&event, bounds) {
                self.app.update(message);
                self.rebuild_view();
                self.layout();
            }
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.app.tick();

        // Only build view if not yet built (view is rebuilt on message handling)
        if self.root.is_none() {
            self.rebuild_view();
            self.layout();
        }

        let output = self.gpu_ctx.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .gpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("UI Render Encoder"),
            });

        // Clear background
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.settings.background.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        // Draw UI
        if let Some(root) = &self.root {
            let bounds = Bounds::new(
                0.0,
                0.0,
                self.window_size.0 as f32,
                self.window_size.1 as f32,
            );
            root.draw(&mut self.renderer, bounds);
        }

        self.renderer.render(&self.gpu_ctx, &mut encoder, &view);

        self.gpu_ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

/// Winit application handler wrapper
#[cfg(not(target_arch = "wasm32"))]
struct WinitApp<A: Application> {
    window: Option<Arc<Window>>,
    state: Option<AppState<A>>,
    app: Option<A>,
    settings: Settings,
}

#[cfg(not(target_arch = "wasm32"))]
impl<A: Application> WinitApp<A> {
    fn new(app: A, settings: Settings) -> Self {
        Self {
            window: None,
            state: None,
            app: Some(app),
            settings,
        }
    }
}

/// WASM version uses Rc<RefCell<>> for async state initialization
#[cfg(target_arch = "wasm32")]
struct WinitApp<A: Application> {
    window: Option<Arc<Window>>,
    state: Rc<RefCell<Option<AppState<A>>>>,
    app: Option<A>,
    settings: Settings,
}

#[cfg(target_arch = "wasm32")]
impl<A: Application> WinitApp<A> {
    fn new(app: A, settings: Settings) -> Self {
        Self {
            window: None,
            state: Rc::new(RefCell::new(None)),
            app: Some(app),
            settings,
        }
    }
}

// Native implementation
#[cfg(not(target_arch = "wasm32"))]
impl<A: Application + 'static> ApplicationHandler for WinitApp<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_title(&self.settings.title)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    self.settings.size.0,
                    self.settings.size.1,
                ));

            let window = Arc::new(
                event_loop
                    .create_window(window_attrs)
                    .expect("Failed to create window"),
            );
            self.window = Some(window.clone());

            if let Some(app) = self.app.take() {
                let settings = self.settings.clone();
                let state = pollster::block_on(AppState::new(window, app, settings));
                self.state = Some(state);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &mut self.state else {
            return;
        };

        handle_window_event(state, event_loop, event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

// WASM implementation
#[cfg(target_arch = "wasm32")]
impl<A: Application + 'static> ApplicationHandler for WinitApp<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            use winit::platform::web::WindowAttributesExtWebSys;
            use wasm_bindgen::JsCast;

            let mut window_attrs = WindowAttributes::default()
                .with_title(&self.settings.title)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    self.settings.size.0,
                    self.settings.size.1,
                ));

            // Attach to canvas element
            let canvas = web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    // Try to find existing canvas, or create one
                    doc.get_element_by_id("canvas")
                        .or_else(|| {
                            let canvas = doc.create_element("canvas").ok()?;
                            canvas.set_id("canvas");
                            doc.body()?.append_child(&canvas).ok()?;
                            Some(canvas)
                        })
                })
                .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok());

            if let Some(canvas) = canvas {
                log::info!("Attaching to canvas element");
                window_attrs = window_attrs.with_canvas(Some(canvas));
            } else {
                log::error!("Failed to get or create canvas element");
            }

            let window = Arc::new(
                event_loop
                    .create_window(window_attrs)
                    .expect("Failed to create window"),
            );
            self.window = Some(window.clone());

            if let Some(app) = self.app.take() {
                let settings = self.settings.clone();
                let state_cell = self.state.clone();
                let window_for_redraw = window.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    log::info!("Starting async WASM initialization");
                    let state = AppState::new(window, app, settings).await;
                    *state_cell.borrow_mut() = Some(state);
                    log::info!("WASM initialization complete");
                    // Request initial redraw now that state is ready
                    window_for_redraw.request_redraw();
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
        let mut state_ref = self.state.borrow_mut();
        let Some(state) = state_ref.as_mut() else {
            return;
        };

        handle_window_event(state, event_loop, event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// Shared window event handling logic
fn handle_window_event<A: Application>(
    state: &mut AppState<A>,
    event_loop: &ActiveEventLoop,
    event: WindowEvent,
) {
    match event {
        WindowEvent::CloseRequested => {
            log::info!("Close requested");
            event_loop.exit();
        }

        WindowEvent::Resized(size) => {
            state.resize(size.width, size.height);
        }

        WindowEvent::RedrawRequested => {
            match state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => {
                    state.resize(state.window_size.0, state.window_size.1);
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    log::error!("Out of memory");
                    event_loop.exit();
                }
                Err(e) => {
                    log::error!("Render error: {:?}", e);
                }
            }
        }

        WindowEvent::CursorMoved { position, .. } => {
            state.cursor_position = (position.x as f32, position.y as f32);
            state.handle_event(Event::MouseMove {
                position: state.cursor_position,
                modifiers: state.modifiers,
            });
        }

        WindowEvent::MouseInput { state: btn_state, button, .. } => {
            let button = MouseButton::from_winit(button);
            let event = if btn_state == ElementState::Pressed {
                Event::MousePress {
                    button,
                    position: state.cursor_position,
                    modifiers: state.modifiers,
                }
            } else {
                Event::MouseRelease {
                    button,
                    position: state.cursor_position,
                    modifiers: state.modifiers,
                }
            };
            state.handle_event(event);
        }

        WindowEvent::MouseWheel { delta, .. } => {
            let delta = match delta {
                MouseScrollDelta::LineDelta(x, y) => (x * 20.0, y * 20.0),
                MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
            };
            state.handle_event(Event::MouseScroll {
                delta,
                position: state.cursor_position,
                modifiers: state.modifiers,
            });
        }

        WindowEvent::KeyboardInput { event, .. } => {
            if let PhysicalKey::Code(keycode) = event.physical_key {
                let key = KeyCode::from_winit(keycode);
                let ui_event = if event.state == ElementState::Pressed {
                    Event::KeyPress {
                        key,
                        modifiers: state.modifiers,
                    }
                } else {
                    Event::KeyRelease {
                        key,
                        modifiers: state.modifiers,
                    }
                };
                state.handle_event(ui_event);
            }

            // Generate TextInput event for text input (on key press only)
            if event.state == ElementState::Pressed {
                if let Some(text) = &event.text {
                    let text_str = text.as_str();
                    // Filter out control characters (but allow space)
                    if !text_str.is_empty() && text_str.chars().all(|c| !c.is_control() || c == ' ') {
                        state.handle_event(Event::TextInput {
                            text: text_str.to_string(),
                        });
                    }
                }
            }
        }

        WindowEvent::ModifiersChanged(modifiers) => {
            state.modifiers = KeyModifiers::from_winit(modifiers);
        }

        _ => {}
    }
}

/// Run an application
pub fn run<A: Application + 'static>(
    app: A,
    settings: Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }

    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Debug).expect("Failed to init logger");
    }

    log::info!("Starting hvat_ui application: {}", settings.title);

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut winit_app = WinitApp::new(app, settings);
    event_loop.run_app(&mut winit_app)?;

    Ok(())
}
