//! Application trait and runtime

use crate::element::Element;
use crate::event::{Event, KeyCode, KeyModifiers, MouseButton};
use crate::layout::{Bounds, Size};
use crate::renderer::{Renderer, TextureId};
use hvat_gpu::{ClearColor, GpuConfig, GpuContext, Texture};
use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;
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
    /// Target frames per second (0 = unlimited, respects vsync)
    pub target_fps: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            title: "hvat_ui Application".to_string(),
            size: (1024, 768),
            background: ClearColor::DARK_GRAY,
            vsync: true,
            target_fps: 60,
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

    pub fn target_fps(mut self, fps: u32) -> Self {
        self.target_fps = fps;
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

    /// Called each frame before rendering (without GPU resource access)
    fn tick(&mut self) {}

    /// Called each frame before rendering with GPU resource access.
    /// Use this to update textures when data changes.
    /// Returns true if the view needs to be rebuilt.
    fn tick_with_resources(&mut self, _resources: &mut Resources) -> bool {
        false
    }

    /// Handle raw events before they're sent to widgets.
    /// Return Some(message) to handle the event and prevent widget processing.
    /// Return None to let widgets process the event normally.
    /// This is useful for global keyboard shortcuts like Ctrl+Z for undo.
    fn on_event(&mut self, _event: &Event) -> Option<Self::Message> {
        None
    }

    /// Called when the window is resized.
    /// Override this to respond to window size changes.
    fn on_resize(&mut self, _width: f32, _height: f32) {}
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
    /// Whether view needs to be rebuilt (dirty flag)
    needs_rebuild: bool,
    /// Last frame time for FPS limiting
    last_frame_time: Instant,
    /// Minimum frame duration based on target FPS
    frame_duration: Duration,
    /// Whether a redraw was requested (by user interaction)
    redraw_requested: bool,
}

impl<A: Application> AppState<A> {
    async fn new(window: Arc<Window>, mut app: A, settings: Settings) -> Self {
        // Configure GPU with vsync setting
        let gpu_config = if settings.vsync {
            GpuConfig::default() // Uses Fifo (vsync on)
        } else {
            GpuConfig::low_latency() // Uses Mailbox (vsync off)
        };

        let gpu_ctx = GpuContext::with_config(window.clone(), gpu_config)
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

        // Calculate frame duration from target FPS
        let frame_duration = if settings.target_fps > 0 {
            Duration::from_secs_f64(1.0 / settings.target_fps as f64)
        } else {
            Duration::ZERO
        };

        Self {
            app,
            gpu_ctx,
            renderer,
            root: None,
            window_size,
            cursor_position: (0.0, 0.0),
            modifiers: KeyModifiers::default(),
            settings,
            needs_rebuild: true, // Initial build needed
            last_frame_time: Instant::now(),
            frame_duration,
            redraw_requested: true, // Initial redraw needed
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.gpu_ctx.resize(width, height);
            self.renderer.resize(width, height);
            self.window_size = (width, height);
            log::debug!("Resized to {}x{}", width, height);
            // Notify the application
            self.app.on_resize(width as f32, height as f32);
            // Mark for rebuild on resize
            self.needs_rebuild = true;
            self.redraw_requested = true;
        }
    }

    fn rebuild_view(&mut self) {
        self.root = Some(self.app.view());
        self.needs_rebuild = false;
    }

    fn layout(&mut self) {
        if let Some(root) = &mut self.root {
            let available = Size::new(self.window_size.0 as f32, self.window_size.1 as f32);
            root.layout(available);
        }
    }

    fn handle_event(&mut self, event: Event) -> bool {
        // Set overlay hint if the event position is within a registered overlay
        // This allows widgets to know if an event is meant for an overlay
        let event = if let Some((x, y)) = event.position() {
            if self.renderer.has_overlay_at(x, y) {
                log::trace!("Event at ({}, {}) is in overlay", x, y);
                event.with_overlay_hint(true)
            } else {
                event
            }
        } else {
            event
        };

        // First, let the application handle the event (for global shortcuts)
        if let Some(message) = self.app.on_event(&event) {
            self.app.update(message);
            self.needs_rebuild = true;
            self.redraw_requested = true;
            return true;
        }

        // Then, let widgets handle the event
        if let Some(root) = &mut self.root {
            let bounds = Bounds::new(
                0.0,
                0.0,
                self.window_size.0 as f32,
                self.window_size.1 as f32,
            );

            if let Some(message) = root.on_event(&event, bounds) {
                self.app.update(message);
                self.needs_rebuild = true;
                self.redraw_requested = true;
                return true;
            }
        }
        false
    }

    /// Check if enough time has passed for next frame (FPS limiting)
    fn should_render(&self) -> bool {
        if self.frame_duration.is_zero() {
            return true;
        }
        self.last_frame_time.elapsed() >= self.frame_duration
    }

    /// Request a redraw for next frame
    fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.app.tick();

        // Call tick_with_resources for GPU updates (e.g., texture updates)
        {
            let mut resources = Resources {
                gpu_ctx: &self.gpu_ctx,
                renderer: &mut self.renderer,
            };
            if self.app.tick_with_resources(&mut resources) {
                self.needs_rebuild = true;
            }
        }

        // Update frame timing
        self.last_frame_time = Instant::now();
        self.redraw_requested = false;

        // Only rebuild view if needed (dirty flag)
        if self.needs_rebuild || self.root.is_none() {
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
            // Clear overlay registry before drawing so widgets can re-register
            self.renderer.clear_overlay_registry();

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
            // Use LogicalSize - this is what worked before and respects DPI scaling properly
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

            // Log window size info for debugging
            let inner = window.inner_size();
            let outer = window.outer_size();
            let scale = window.scale_factor();
            log::info!(
                "Window created: requested_logical={}x{}, inner_physical={}x{}, outer_physical={}x{}, scale_factor={}",
                self.settings.size.0, self.settings.size.1,
                inner.width, inner.height,
                outer.width, outer.height,
                scale
            );

            self.window = Some(window.clone());

            if let Some(app) = self.app.take() {
                let settings = self.settings.clone();
                let state = pollster::block_on(AppState::new(window.clone(), app, settings));
                self.state = Some(state);

                // Request initial redraw
                window.request_redraw();
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

        let window = self.window.clone();
        handle_window_event(state, event_loop, event, window.as_ref());
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Always request redraw if there's pending work
        // FPS limiting is handled in RedrawRequested, not here
        if let Some(state) = &self.state {
            if state.redraw_requested {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
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

            if let Some(ref canvas) = canvas {
                log::info!("Attaching to canvas element");
                window_attrs = window_attrs.with_canvas(Some(canvas.clone()));
            } else {
                log::error!("Failed to get or create canvas element");
            }

            let window = Arc::new(
                event_loop
                    .create_window(window_attrs)
                    .expect("Failed to create window"),
            );
            self.window = Some(window.clone());

            // Set up ResizeObserver for the canvas to handle browser resize
            if let Some(canvas) = canvas {
                let window_for_resize = window.clone();
                setup_canvas_resize_observer(canvas, window_for_resize);
            }

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

        let window = self.window.clone();
        handle_window_event(state, event_loop, event, window.as_ref());
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Always request redraw if there's pending work
        // FPS limiting is handled in RedrawRequested, not here
        let state_ref = self.state.borrow();
        if let Some(state) = state_ref.as_ref() {
            if state.redraw_requested {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }
}

/// Shared window event handling logic
fn handle_window_event<A: Application>(
    state: &mut AppState<A>,
    event_loop: &ActiveEventLoop,
    event: WindowEvent,
    window: Option<&Arc<Window>>,
) {
    match event {
        WindowEvent::CloseRequested => {
            log::info!("Close requested");
            event_loop.exit();
        }

        WindowEvent::Resized(size) => {
            log::info!("Window resized to {}x{}", size.width, size.height);
            state.resize(size.width, size.height);
            // Force layout recalculation on resize
            state.needs_rebuild = true;
            // Request redraw after resize
            if let Some(w) = window {
                w.request_redraw();
            }
        }

        WindowEvent::ScaleFactorChanged { scale_factor, inner_size_writer: _ } => {
            log::info!("Scale factor changed to {}", scale_factor);
            // The window will send a Resized event after this, so we just log here
        }

        WindowEvent::RedrawRequested => {
            // Always render when requested - this is critical for Wayland where
            // the window won't appear until it has rendered its first frame
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
                overlay_hint: false, // Set by handle_event if needed
            });
            // Always request redraw on mouse move for hover effects and drag operations
            // Widgets may update internal state (e.g., scrollbar dragging) without producing messages
            state.request_redraw();
            if let Some(w) = window {
                w.request_redraw();
            }
        }

        WindowEvent::MouseInput { state: btn_state, button, .. } => {
            let button = MouseButton::from_winit(button);

            // On press, first send a global event to allow focused widgets to blur
            if btn_state == ElementState::Pressed {
                let handled = state.handle_event(Event::GlobalMousePress {
                    button,
                    position: state.cursor_position,
                });
                // If GlobalMousePress was handled (e.g., closing an overlay), clear the
                // overlay registry so subsequent MousePress doesn't get stale overlay_hint
                if handled {
                    state.renderer.clear_overlay_registry();
                }
            }

            let event = if btn_state == ElementState::Pressed {
                Event::MousePress {
                    button,
                    position: state.cursor_position,
                    modifiers: state.modifiers,
                    screen_position: Some(state.cursor_position),
                    overlay_hint: false, // Set by handle_event if needed
                }
            } else {
                Event::MouseRelease {
                    button,
                    position: state.cursor_position,
                    modifiers: state.modifiers,
                    overlay_hint: false, // Set by handle_event if needed
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
                overlay_hint: false, // Set by handle_event if needed
            });
            // Always request redraw for scroll events - widgets may update internal scroll state
            // without producing a message (e.g., Scrollable, Collapsible with scroll)
            state.request_redraw();
            if let Some(w) = window {
                w.request_redraw();
            }
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
            // Don't generate TextInput when Ctrl or Alt is held (those are shortcuts, not text input)
            if event.state == ElementState::Pressed && !state.modifiers.ctrl && !state.modifiers.alt {
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

        WindowEvent::Focused(focused) => {
            if !focused {
                // Window lost focus - notify widgets to blur any focused inputs
                log::debug!("Window lost focus");
                state.handle_event(Event::FocusLost);
                state.request_redraw();
                if let Some(w) = window {
                    w.request_redraw();
                }
            }
        }

        WindowEvent::CursorLeft { .. } => {
            // Cursor left the window - release any drag states
            log::debug!("Cursor left window");
            state.handle_event(Event::CursorLeft);
            state.request_redraw();
            if let Some(w) = window {
                w.request_redraw();
            }
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
    // Use Wait for event-driven rendering (saves CPU/battery)
    // The about_to_wait handler will request redraws when needed
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut winit_app = WinitApp::new(app, settings);
    event_loop.run_app(&mut winit_app)?;

    Ok(())
}

/// Set up window resize handling for WASM.
/// This ensures the canvas resizes properly when the browser window is resized.
/// Uses debouncing to reduce flashing during resize.
#[cfg(target_arch = "wasm32")]
fn setup_canvas_resize_observer(_canvas: web_sys::HtmlCanvasElement, window: Arc<Window>) {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use std::cell::Cell;

    // Get the browser window object
    let Some(browser_window) = web_sys::window() else {
        log::error!("Failed to get browser window for resize handling");
        return;
    };

    // Shared state for debounce timeout ID (leaked to live forever)
    let timeout_id: &'static Cell<Option<i32>> = Box::leak(Box::new(Cell::new(None)));

    // Clone window for the closure
    let window_for_callback = window.clone();

    // The actual resize function that will be called after debounce
    let do_resize: &'static Closure<dyn Fn()> = Box::leak(Box::new(Closure::<dyn Fn()>::new({
        let window = window_for_callback.clone();
        move || {
            if let Some(bw) = web_sys::window() {
                let width = bw.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(800.0) as u32;
                let height = bw.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(600.0) as u32;

                if width > 0 && height > 0 {
                    log::debug!("Applying resize to {}x{}", width, height);
                    let _ = window.request_inner_size(winit::dpi::LogicalSize::new(width, height));
                    window.request_redraw();
                }
            }
        }
    })));

    // Create the debounced resize callback
    let resize_callback = Closure::<dyn Fn()>::new(move || {
        // Clear any pending timeout
        if let Some(id) = timeout_id.get() {
            if let Some(bw) = web_sys::window() {
                bw.clear_timeout_with_handle(id);
            }
        }

        // Set a new timeout (50ms debounce)
        if let Some(bw) = web_sys::window() {
            match bw.set_timeout_with_callback_and_timeout_and_arguments_0(
                do_resize.as_ref().unchecked_ref(),
                50, // 50ms debounce delay
            ) {
                Ok(id) => {
                    timeout_id.set(Some(id));
                }
                Err(e) => {
                    log::error!("Failed to set resize timeout: {:?}", e);
                }
            }
        }
    });

    // Add the resize event listener to the browser window
    if let Err(e) = browser_window.add_event_listener_with_callback(
        "resize",
        resize_callback.as_ref().unchecked_ref(),
    ) {
        log::error!("Failed to add resize event listener: {:?}", e);
        return;
    }

    // Keep the closure alive
    resize_callback.forget();
    log::info!("Window resize event listener set up with debouncing");

    // Also trigger an initial resize to set the correct size
    if let Some(bw) = web_sys::window() {
        let width = bw.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(800.0) as u32;
        let height = bw.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(600.0) as u32;
        if width > 0 && height > 0 {
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(width, height));
        }
    }
}
