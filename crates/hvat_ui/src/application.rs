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
#[cfg(target_arch = "wasm32")]
use std::sync::Mutex;

// ============================================================================
// WASM Drag-Drop State (global state for receiving drag-drop events from JS)
// ============================================================================

/// Represents a dropped file from WASM drag-drop
#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct WasmDroppedFile {
    /// Filename of the dropped file
    pub name: String,
    /// Raw file data bytes
    pub data: Vec<u8>,
}

/// Pending drag-drop events from WASM
#[cfg(target_arch = "wasm32")]
pub enum WasmDragDropEvent {
    /// Files dropped on the canvas
    FilesDropped(Vec<WasmDroppedFile>),
    /// Drag hover started
    HoverStarted,
    /// Drag hover ended
    HoverEnded,
}

#[cfg(target_arch = "wasm32")]
static PENDING_DRAG_DROP: std::sync::OnceLock<Mutex<Vec<WasmDragDropEvent>>> =
    std::sync::OnceLock::new();

#[cfg(target_arch = "wasm32")]
fn pending_drag_drop_state() -> &'static Mutex<Vec<WasmDragDropEvent>> {
    PENDING_DRAG_DROP.get_or_init(|| Mutex::new(Vec::new()))
}

/// Push a drag-drop event to the pending queue (called from JS callbacks)
#[cfg(target_arch = "wasm32")]
pub fn push_wasm_drag_drop_event(event: WasmDragDropEvent) {
    if let Ok(mut pending) = pending_drag_drop_state().lock() {
        pending.push(event);
    }
}

/// Take all pending drag-drop events
#[cfg(target_arch = "wasm32")]
fn take_pending_drag_drop_events() -> Vec<WasmDragDropEvent> {
    if let Ok(mut pending) = pending_drag_drop_state().lock() {
        std::mem::take(&mut *pending)
    } else {
        Vec::new()
    }
}

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
    /// Path to window icon (PNG format, native only)
    pub icon_path: Option<String>,
    /// Embedded icon bytes (PNG format, native only) - takes precedence over icon_path
    pub icon_bytes: Option<&'static [u8]>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            title: "hvat_ui Application".to_string(),
            size: (1024, 768),
            background: ClearColor::DARK_GRAY,
            vsync: true,
            target_fps: 60,
            icon_path: None,
            icon_bytes: None,
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

    pub fn icon(mut self, path: impl Into<String>) -> Self {
        self.icon_path = Some(path.into());
        self
    }

    pub fn icon_bytes(mut self, bytes: &'static [u8]) -> Self {
        self.icon_bytes = Some(bytes);
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

    /// Returns true if any text input field is currently focused.
    /// Used to suppress keyboard shortcuts when typing in text fields.
    /// Override this to track your application's text input focus state.
    fn is_text_input_focused(&self) -> bool {
        false
    }

    /// Returns true if the view needs to be rebuilt immediately, even during a mouse drag.
    /// Override this when your application needs to show real-time updates during drag
    /// operations (e.g., drawing annotation previews, dragging objects).
    ///
    /// Note: When this returns true, the rebuild happens immediately which can reset
    /// widget state. This is intentional for drawing operations but would break button
    /// click handling if used incorrectly.
    fn needs_immediate_rebuild(&self) -> bool {
        false
    }
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
    /// Whether the current click was consumed by GlobalMousePress (e.g., closing an overlay).
    /// When true, the subsequent MousePress/MouseRelease from the same physical click are skipped.
    click_consumed: bool,
    /// Whether a mouse button is currently pressed.
    /// While true, view rebuilds are deferred to preserve widget state (e.g., button's Pressed state)
    /// between MousePress and MouseRelease events.
    mouse_button_pressed: bool,
    /// Accumulated dropped file paths (native only).
    /// Files are accumulated here and then sent as a batch when the drop ends.
    #[cfg(not(target_arch = "wasm32"))]
    pending_dropped_files: Vec<std::path::PathBuf>,
    /// Whether we need to flush dropped files on the next frame (native only).
    /// Set to true after DroppedFile events, checked on RedrawRequested.
    #[cfg(not(target_arch = "wasm32"))]
    flush_dropped_files_pending: bool,
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
            click_consumed: false,
            mouse_button_pressed: false,
            #[cfg(not(target_arch = "wasm32"))]
            pending_dropped_files: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            flush_dropped_files_pending: false,
        }
    }

    /// Flush any pending dropped files as a single batch event.
    #[cfg(not(target_arch = "wasm32"))]
    fn flush_pending_dropped_files(&mut self) {
        if !self.pending_dropped_files.is_empty() {
            let paths = std::mem::take(&mut self.pending_dropped_files);
            log::info!("Flushing {} dropped files as batch", paths.len());
            self.handle_event(Event::FilesDropped { paths });
            self.request_redraw();
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
        // Process pending WASM drag-drop events
        #[cfg(target_arch = "wasm32")]
        {
            use std::path::PathBuf;
            for event in take_pending_drag_drop_events() {
                match event {
                    WasmDragDropEvent::FilesDropped(files) => {
                        log::info!("Processing {} dropped files from WASM", files.len());
                        // Send each file as a DroppedFileData event first (for reading data)
                        for file in &files {
                            self.handle_event(Event::DroppedFileData {
                                name: file.name.clone(),
                                data: file.data.clone(),
                            });
                        }
                        // Then send the FilesDropped event with virtual paths
                        let paths: Vec<PathBuf> =
                            files.iter().map(|f| PathBuf::from(&f.name)).collect();
                        self.handle_event(Event::FilesDropped { paths });
                    }
                    WasmDragDropEvent::HoverStarted => {
                        self.handle_event(Event::FileHoverStarted { paths: vec![] });
                    }
                    WasmDragDropEvent::HoverEnded => {
                        self.handle_event(Event::FileHoverEnded);
                    }
                }
            }
        }

        self.app.tick();

        // Call tick_with_resources for GPU updates (e.g., texture updates)
        // If it returns true, there's more work to do (e.g., preloading images)
        let needs_more_ticks = {
            let mut resources = Resources {
                gpu_ctx: &self.gpu_ctx,
                renderer: &mut self.renderer,
            };
            let result = self.app.tick_with_resources(&mut resources);
            if result {
                self.needs_rebuild = true;
            }
            result
        };

        // Update frame timing
        self.last_frame_time = Instant::now();
        // Keep requesting redraws if tick_with_resources indicated more work
        self.redraw_requested = needs_more_ticks;

        // Only rebuild view if needed (dirty flag).
        // IMPORTANT: Defer rebuilds while a mouse button is pressed to preserve widget state
        // (e.g., button's Pressed state) between MousePress and MouseRelease events.
        // This ensures clicks work correctly even when blur events trigger state changes.
        // EXCEPTION: If the app explicitly requests immediate rebuild (e.g., for drawing previews),
        // bypass the deferral to show real-time updates during drag operations.
        let immediate_rebuild_requested = self.app.needs_immediate_rebuild();
        if (self.needs_rebuild && (!self.mouse_button_pressed || immediate_rebuild_requested))
            || self.root.is_none()
        {
            self.rebuild_view();
            self.layout();
        }

        let output = self.gpu_ctx.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.gpu_ctx
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
            // Load window icon - try embedded bytes first, then file path
            log::info!(
                "Loading window icon: icon_bytes={}, icon_path={:?}",
                self.settings.icon_bytes.is_some(),
                self.settings.icon_path
            );
            let window_icon = self
                .settings
                .icon_bytes
                .and_then(|bytes| match image::load_from_memory(bytes) {
                    Ok(img) => {
                        let rgba = img.into_rgba8();
                        let (width, height) = rgba.dimensions();
                        match winit::window::Icon::from_rgba(rgba.into_raw(), width, height) {
                            Ok(icon) => {
                                log::info!(
                                    "Loaded window icon from embedded bytes ({}x{})",
                                    width,
                                    height
                                );
                                Some(icon)
                            }
                            Err(e) => {
                                log::warn!("Failed to create window icon from bytes: {}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to decode embedded icon bytes: {}", e);
                        None
                    }
                })
                .or_else(|| {
                    self.settings
                        .icon_path
                        .as_ref()
                        .and_then(|path| match image::open(path) {
                            Ok(img) => {
                                let rgba = img.into_rgba8();
                                let (width, height) = rgba.dimensions();
                                match winit::window::Icon::from_rgba(rgba.into_raw(), width, height)
                                {
                                    Ok(icon) => {
                                        log::info!("Loaded window icon from file: {}", path);
                                        Some(icon)
                                    }
                                    Err(e) => {
                                        log::warn!("Failed to create window icon: {}", e);
                                        None
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!("Failed to load window icon from {}: {}", path, e);
                                None
                            }
                        })
                });

            // Use LogicalSize - this is what worked before and respects DPI scaling properly
            let mut window_attrs = WindowAttributes::default()
                .with_title(&self.settings.title)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    self.settings.size.0,
                    self.settings.size.1,
                ));

            if let Some(icon) = window_icon {
                window_attrs = window_attrs.with_window_icon(Some(icon));
            }

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
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

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
                    doc.get_element_by_id("canvas").or_else(|| {
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

            // Set up ResizeObserver and drag-drop for the canvas
            if let Some(ref canvas) = canvas {
                let window_for_resize = window.clone();
                setup_canvas_resize_observer(canvas.clone(), window_for_resize);
                let window_for_dragdrop = window.clone();
                setup_canvas_drag_drop(canvas.clone(), window_for_dragdrop);
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

        WindowEvent::ScaleFactorChanged {
            scale_factor,
            inner_size_writer: _,
        } => {
            log::info!("Scale factor changed to {}", scale_factor);
            // The window will send a Resized event after this, so we just log here
        }

        WindowEvent::RedrawRequested => {
            // Flush any pending dropped files before rendering (native only)
            // This ensures all DroppedFile events are batched together
            #[cfg(not(target_arch = "wasm32"))]
            if state.flush_dropped_files_pending {
                state.flush_dropped_files_pending = false;
                state.flush_pending_dropped_files();
            }

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

        WindowEvent::MouseInput {
            state: btn_state,
            button,
            ..
        } => {
            let button = MouseButton::from_winit(button);

            // Track mouse button state to defer rebuilds between press and release.
            // This preserves widget state (e.g., button's Pressed state) during a click.
            if btn_state == ElementState::Pressed {
                state.mouse_button_pressed = true;
            } else {
                state.mouse_button_pressed = false;
            }

            // On press, first send a global event to allow focused widgets to blur
            if btn_state == ElementState::Pressed {
                // Reset consumption flag at the start of a new click
                state.click_consumed = false;

                // Check if an overlay is open BEFORE sending GlobalMousePress
                // This is crucial: we only consume clicks that close overlays,
                // NOT clicks that merely blur focused text inputs
                let had_overlay = state
                    .root
                    .as_ref()
                    .map(|r| r.has_active_overlay())
                    .unwrap_or(false);

                // Send GlobalMousePress - rebuilds are deferred by mouse_button_pressed flag
                let handled = state.handle_event(Event::GlobalMousePress {
                    button,
                    position: state.cursor_position,
                });

                // Only consume the click if:
                // 1. An overlay was open before the event, AND
                // 2. The event was handled (overlay closed)
                // This allows text input blur to NOT consume the click,
                // so the button underneath still receives the click
                if handled && had_overlay {
                    state.renderer.clear_overlay_registry();
                    state.click_consumed = true;
                    log::debug!("Click consumed by overlay close");
                }
            }

            // Skip regular MousePress/MouseRelease if the click was consumed by GlobalMousePress
            // This prevents the trigger widget from receiving events that would reopen the overlay
            if state.click_consumed {
                log::trace!("Skipping MousePress/MouseRelease - click was consumed");
                return;
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
                // Allow browser dev tools shortcuts to pass through (WASM only)
                // F12, Ctrl+Shift+I, Ctrl+Shift+J, Ctrl+Shift+C are common dev tools shortcuts
                #[cfg(target_arch = "wasm32")]
                {
                    use winit::keyboard::KeyCode as WK;
                    let dominated = matches!(
                        keycode,
                        WK::F12
                            | WK::F5  // Refresh
                            | WK::F11 // Fullscreen
                    ) || (state.modifiers.ctrl
                        && state.modifiers.shift
                        && matches!(keycode, WK::KeyI | WK::KeyJ | WK::KeyC));

                    if dominated {
                        // Don't handle this key - let the browser handle it
                        return;
                    }
                }

                let key = KeyCode::from_winit(keycode);
                // Check application's text input focus state
                let text_input_focused = state.app.is_text_input_focused();
                let ui_event = if event.state == ElementState::Pressed {
                    Event::KeyPress {
                        key,
                        modifiers: state.modifiers,
                        text_input_focused,
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
            if event.state == ElementState::Pressed && !state.modifiers.ctrl && !state.modifiers.alt
            {
                if let Some(text) = &event.text {
                    let text_str = text.as_str();
                    // Filter out control characters (but allow space)
                    if !text_str.is_empty() && text_str.chars().all(|c| !c.is_control() || c == ' ')
                    {
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

        WindowEvent::DroppedFile(path) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                // Accumulate dropped files - winit sends one event per file
                // We'll flush them as a batch on the next redraw
                log::info!("File dropped (accumulating): {:?}", path);
                state.pending_dropped_files.push(path);
                state.flush_dropped_files_pending = true;
            }
            #[cfg(target_arch = "wasm32")]
            {
                // On WASM, drag-drop is handled separately via HTML5 events
                log::info!("File dropped: {:?}", path);
                state.handle_event(Event::FilesDropped { paths: vec![path] });
            }
            state.request_redraw();
            if let Some(w) = window {
                w.request_redraw();
            }
        }

        WindowEvent::HoveredFile(path) => {
            log::debug!("File hover started: {:?}", path);
            state.handle_event(Event::FileHoverStarted { paths: vec![path] });
            state.request_redraw();
            if let Some(w) = window {
                w.request_redraw();
            }
        }

        WindowEvent::HoveredFileCancelled => {
            log::debug!("File hover cancelled");
            // Flush any accumulated dropped files first (native only)
            #[cfg(not(target_arch = "wasm32"))]
            state.flush_pending_dropped_files();
            state.handle_event(Event::FileHoverEnded);
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
    use std::cell::Cell;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

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
                let width = bw
                    .inner_width()
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(800.0) as u32;
                let height = bw
                    .inner_height()
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(600.0) as u32;

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
    if let Err(e) = browser_window
        .add_event_listener_with_callback("resize", resize_callback.as_ref().unchecked_ref())
    {
        log::error!("Failed to add resize event listener: {:?}", e);
        return;
    }

    // Keep the closure alive
    resize_callback.forget();
    log::info!("Window resize event listener set up with debouncing");

    // Also trigger an initial resize to set the correct size
    if let Some(bw) = web_sys::window() {
        let width = bw
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(800.0) as u32;
        let height = bw
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(600.0) as u32;
        if width > 0 && height > 0 {
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(width, height));
        }
    }
}

/// Set up drag-drop event listeners for WASM.
/// This enables file/folder drop support on the canvas element.
#[cfg(target_arch = "wasm32")]
fn setup_canvas_drag_drop(canvas: web_sys::HtmlCanvasElement, window: Arc<Window>) {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    // Prevent default behavior for dragover (required for drop to work)
    let dragover_callback =
        Closure::<dyn Fn(web_sys::DragEvent)>::new(|event: web_sys::DragEvent| {
            event.prevent_default();
            event.stop_propagation();
            // Set drop effect to copy
            if let Some(dt) = event.data_transfer() {
                dt.set_drop_effect("copy");
            }
        });

    if let Err(e) = canvas
        .add_event_listener_with_callback("dragover", dragover_callback.as_ref().unchecked_ref())
    {
        log::error!("Failed to add dragover listener: {:?}", e);
    }
    dragover_callback.forget();

    // Prevent default for dragenter
    let dragenter_callback =
        Closure::<dyn Fn(web_sys::DragEvent)>::new(|event: web_sys::DragEvent| {
            event.prevent_default();
            event.stop_propagation();
            log::debug!("Drag enter on canvas");
            push_wasm_drag_drop_event(WasmDragDropEvent::HoverStarted);
        });

    if let Err(e) = canvas
        .add_event_listener_with_callback("dragenter", dragenter_callback.as_ref().unchecked_ref())
    {
        log::error!("Failed to add dragenter listener: {:?}", e);
    }
    dragenter_callback.forget();

    // Handle dragleave
    let dragleave_callback =
        Closure::<dyn Fn(web_sys::DragEvent)>::new(|event: web_sys::DragEvent| {
            event.prevent_default();
            event.stop_propagation();
            log::debug!("Drag leave from canvas");
            push_wasm_drag_drop_event(WasmDragDropEvent::HoverEnded);
        });

    if let Err(e) = canvas
        .add_event_listener_with_callback("dragleave", dragleave_callback.as_ref().unchecked_ref())
    {
        log::error!("Failed to add dragleave listener: {:?}", e);
    }
    dragleave_callback.forget();

    // Handle drop event - read files and folders asynchronously
    let window_for_drop = window.clone();
    let drop_callback =
        Closure::<dyn Fn(web_sys::DragEvent)>::new(move |event: web_sys::DragEvent| {
            event.prevent_default();
            event.stop_propagation();
            log::info!("Drop event on canvas");

            // End hover state
            push_wasm_drag_drop_event(WasmDragDropEvent::HoverEnded);

            let Some(data_transfer) = event.data_transfer() else {
                log::warn!("No data transfer in drop event");
                return;
            };

            // Try to use DataTransferItemList for folder support
            let items = data_transfer.items();
            let item_count = items.length();
            log::info!("Dropped {} items (using DataTransferItemList)", item_count);

            if item_count == 0 {
                log::warn!("No items in drop event");
                return;
            }

            // Collect entries from all items (files and folders)
            // We need to get all entries synchronously during the drop event,
            // then process them asynchronously
            let mut entries: Vec<web_sys::FileSystemEntry> = Vec::new();
            for i in 0..item_count {
                if let Some(item) = items.get(i) {
                    // Use webkitGetAsEntry to detect files vs folders
                    // Returns Result<Option<FileSystemEntry>, JsValue>
                    match item.webkit_get_as_entry() {
                        Ok(Some(entry)) => {
                            log::info!(
                                "Entry {}: name='{}', isFile={}, isDirectory={}",
                                i,
                                entry.name(),
                                entry.is_file(),
                                entry.is_directory()
                            );
                            entries.push(entry);
                        }
                        Ok(None) => {
                            log::debug!("Item {} has no entry (might be non-file data)", i);
                        }
                        Err(e) => {
                            log::error!("Failed to get entry for item {}: {:?}", i, e);
                        }
                    }
                }
            }

            if entries.is_empty() {
                log::warn!("No file/folder entries found in drop");
                return;
            }

            // Process entries asynchronously (handles both files and folders)
            let window_for_redraw = window_for_drop.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let dropped_files = process_entries(entries).await;

                if !dropped_files.is_empty() {
                    log::info!(
                        "Pushing {} dropped files to event queue",
                        dropped_files.len()
                    );
                    push_wasm_drag_drop_event(WasmDragDropEvent::FilesDropped(dropped_files));
                    window_for_redraw.request_redraw();
                } else {
                    log::warn!("No valid image files found in drop");
                }
            });
        });

    if let Err(e) =
        canvas.add_event_listener_with_callback("drop", drop_callback.as_ref().unchecked_ref())
    {
        log::error!("Failed to add drop listener: {:?}", e);
    }
    drop_callback.forget();

    log::info!("Canvas drag-drop event listeners set up");
}

/// Read a File asynchronously using FileReader (WASM only).
/// This is a utility function for reading web_sys::File objects.
#[cfg(target_arch = "wasm32")]
pub async fn read_file_async(file: &web_sys::File) -> Result<Vec<u8>, String> {
    use js_sys::{ArrayBuffer, Uint8Array};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    // Use the blob's arrayBuffer() method which returns a Promise
    let array_buffer_promise = file.array_buffer();
    let array_buffer = JsFuture::from(array_buffer_promise)
        .await
        .map_err(|e| format!("Failed to read file: {:?}", e))?;

    let array_buffer: ArrayBuffer = array_buffer
        .dyn_into()
        .map_err(|_| "Failed to convert to ArrayBuffer")?;

    let uint8_array = Uint8Array::new(&array_buffer);
    Ok(uint8_array.to_vec())
}

/// Check if a filename has a supported image extension
#[cfg(target_arch = "wasm32")]
fn is_image_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".bmp")
        || lower.ends_with(".tiff")
        || lower.ends_with(".tif")
        || lower.ends_with(".webp")
}

/// Process a list of FileSystemEntry objects (files and folders) and return all image files
#[cfg(target_arch = "wasm32")]
async fn process_entries(entries: Vec<web_sys::FileSystemEntry>) -> Vec<WasmDroppedFile> {
    use wasm_bindgen::JsCast;

    let mut all_files: Vec<WasmDroppedFile> = Vec::new();

    for entry in entries {
        if entry.is_file() {
            // It's a file - read it directly
            if let Some(file_entry) = entry.dyn_ref::<web_sys::FileSystemFileEntry>() {
                match read_file_entry(file_entry).await {
                    Ok(dropped_file) => {
                        if is_image_file(&dropped_file.name) {
                            log::info!("Read file: {}", dropped_file.name);
                            all_files.push(dropped_file);
                        } else {
                            log::debug!("Skipping non-image file: {}", dropped_file.name);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read file entry: {}", e);
                    }
                }
            }
        } else if entry.is_directory() {
            // It's a directory - read all files recursively
            if let Some(dir_entry) = entry.dyn_ref::<web_sys::FileSystemDirectoryEntry>() {
                log::info!("Processing directory: {}", dir_entry.name());
                match read_directory_recursive(dir_entry, "").await {
                    Ok(files) => {
                        for dropped_file in files {
                            if is_image_file(&dropped_file.name) {
                                log::info!("Read file from folder: {}", dropped_file.name);
                                all_files.push(dropped_file);
                            } else {
                                log::debug!("Skipping non-image file: {}", dropped_file.name);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read directory: {}", e);
                    }
                }
            }
        }
    }

    // Sort files by name for consistent ordering
    all_files.sort_by(|a, b| a.name.cmp(&b.name));
    log::info!("Total image files found: {}", all_files.len());

    all_files
}

/// Read a FileSystemFileEntry and return its contents
#[cfg(target_arch = "wasm32")]
async fn read_file_entry(entry: &web_sys::FileSystemFileEntry) -> Result<WasmDroppedFile, String> {
    use js_sys::Promise;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    // FileSystemFileEntry.file() takes a callback, so we wrap it in a Promise
    let promise = Promise::new(&mut |resolve, reject| {
        // The file() method calls success callback with the File object
        let success_cb = Closure::once(move |file: web_sys::File| {
            resolve.call1(&wasm_bindgen::JsValue::NULL, &file).unwrap();
        });
        let error_cb = Closure::once(move |err: wasm_bindgen::JsValue| {
            reject.call1(&wasm_bindgen::JsValue::NULL, &err).unwrap();
        });

        entry.file_with_callback_and_callback(
            success_cb.as_ref().unchecked_ref(),
            error_cb.as_ref().unchecked_ref(),
        );

        // Prevent closures from being dropped
        success_cb.forget();
        error_cb.forget();
    });

    let file: web_sys::File = JsFuture::from(promise)
        .await
        .map_err(|e| format!("Failed to get file: {:?}", e))?
        .dyn_into()
        .map_err(|_| "Failed to convert to File")?;

    let name = file.name();
    let data = read_file_async(&file).await?;

    Ok(WasmDroppedFile { name, data })
}

/// Read all files from a directory recursively
/// Uses Box::pin to handle recursive async calls
#[cfg(target_arch = "wasm32")]
fn read_directory_recursive<'a>(
    dir_entry: &'a web_sys::FileSystemDirectoryEntry,
    prefix: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<WasmDroppedFile>, String>> + 'a>>
{
    use wasm_bindgen::JsCast;

    let prefix = prefix.to_string();

    Box::pin(async move {
        let mut all_files: Vec<WasmDroppedFile> = Vec::new();
        let reader = dir_entry.create_reader();

        // Read entries in batches (browsers limit to ~100 entries per call)
        loop {
            let entries = read_entries_batch(&reader).await?;
            if entries.is_empty() {
                break;
            }

            for entry in entries {
                let entry_name = entry.name();
                let full_path = if prefix.is_empty() {
                    entry_name.clone()
                } else {
                    format!("{}/{}", prefix, entry_name)
                };

                if entry.is_file() {
                    if let Some(file_entry) = entry.dyn_ref::<web_sys::FileSystemFileEntry>() {
                        match read_file_entry(file_entry).await {
                            Ok(mut dropped_file) => {
                                // Use full path as name for nested files
                                dropped_file.name = full_path;
                                all_files.push(dropped_file);
                            }
                            Err(e) => {
                                log::error!("Failed to read file {}: {}", full_path, e);
                            }
                        }
                    }
                } else if entry.is_directory() {
                    if let Some(sub_dir) = entry.dyn_ref::<web_sys::FileSystemDirectoryEntry>() {
                        match read_directory_recursive(sub_dir, &full_path).await {
                            Ok(sub_files) => {
                                all_files.extend(sub_files);
                            }
                            Err(e) => {
                                log::error!("Failed to read subdirectory {}: {}", full_path, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(all_files)
    })
}

/// Read a batch of entries from a directory reader
/// Note: readEntries may need to be called multiple times to get all entries
#[cfg(target_arch = "wasm32")]
async fn read_entries_batch(
    reader: &web_sys::FileSystemDirectoryReader,
) -> Result<Vec<web_sys::FileSystemEntry>, String> {
    use js_sys::Promise;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let promise = Promise::new(&mut |resolve, reject| {
        let success_cb = Closure::once(move |entries: js_sys::Array| {
            resolve
                .call1(&wasm_bindgen::JsValue::NULL, &entries)
                .unwrap();
        });
        let error_cb = Closure::once(move |err: wasm_bindgen::JsValue| {
            reject.call1(&wasm_bindgen::JsValue::NULL, &err).unwrap();
        });

        if let Err(e) = reader.read_entries_with_callback_and_callback(
            success_cb.as_ref().unchecked_ref(),
            error_cb.as_ref().unchecked_ref(),
        ) {
            log::error!("Failed to call readEntries: {:?}", e);
        }

        success_cb.forget();
        error_cb.forget();
    });

    let result = JsFuture::from(promise)
        .await
        .map_err(|e| format!("readEntries failed: {:?}", e))?;

    let array: js_sys::Array = result
        .dyn_into()
        .map_err(|_| "Failed to convert to Array")?;

    let mut entries = Vec::new();
    for i in 0..array.length() {
        if let Ok(entry) = array.get(i).dyn_into::<web_sys::FileSystemEntry>() {
            entries.push(entry);
        }
    }

    log::debug!("Read {} entries from directory", entries.len());
    Ok(entries)
}
