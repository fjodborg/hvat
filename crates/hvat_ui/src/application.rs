use std::sync::Arc;
use winit::window::Window;

use crate::{Element, Renderer};

/// Core application trait that defines the lifecycle of an hvat_ui application.
///
/// This trait is inspired by the Elm Architecture and similar to iced's Application trait.
/// Applications maintain state, respond to messages, and produce a view.
pub trait Application: Sized {
    /// The message type that this application handles.
    /// Messages represent events that can update the application state.
    type Message: Send + 'static;

    /// Initialize the application state.
    /// This is called once at startup.
    fn new() -> Self;

    /// Return the window title for the application.
    fn title(&self) -> String;

    /// Update the application state in response to a message.
    /// Returns a command to execute (for async operations), or None.
    fn update(&mut self, message: Self::Message);

    /// Produce the view tree for the current application state.
    /// The view is a tree of Elements that describe the UI.
    fn view(&self) -> Element<Self::Message>;

    /// Called every frame for timing/animation purposes.
    /// Override to return a message for frame-by-frame updates.
    /// Default implementation returns None.
    fn tick(&self) -> Option<Self::Message> {
        None
    }
}

/// Settings for running an application.
pub struct Settings {
    /// Window title (can be overridden by Application::title)
    pub window_title: Option<String>,

    /// Initial window size
    pub window_size: (u32, u32),

    /// Minimum window size (prevents resizing below this)
    pub min_window_size: Option<(u32, u32)>,

    /// Whether the window should be resizable
    pub resizable: bool,

    /// Log level (default: Debug)
    pub log_level: log::LevelFilter,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window_title: None,
            window_size: (800, 600),
            min_window_size: Some((400, 300)),
            resizable: true,
            log_level: log::LevelFilter::Debug,
        }
    }
}

/// Run an application with the given settings (native version).
///
/// This function creates a window, initializes the GPU context, and runs the event loop.
/// It returns when the window is closed.
#[cfg(not(target_arch = "wasm32"))]
pub fn run<A: Application + 'static>(settings: Settings) -> Result<(), String> {
    use winit::event::{Event as WinitEvent, WindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::WindowBuilder;

    env_logger::Builder::from_default_env()
        .filter_level(settings.log_level)
        // Mute noisy dependency logs
        .filter_module("cosmic_text", log::LevelFilter::Warn)
        .filter_module("wgpu", log::LevelFilter::Warn)
        .filter_module("naga", log::LevelFilter::Warn)
        .init();

    log::info!("Starting hvat_ui application");
    log::debug!("Window size: {}x{}", settings.window_size.0, settings.window_size.1);

    // Create event loop
    let event_loop = EventLoop::new().map_err(|e| format!("Failed to create event loop: {:?}", e))?;

    // Create window
    let window = {
        let mut builder = WindowBuilder::new()
            .with_inner_size(winit::dpi::LogicalSize::new(
                settings.window_size.0,
                settings.window_size.1,
            ))
            .with_resizable(settings.resizable);

        if let Some((min_w, min_h)) = settings.min_window_size {
            builder = builder.with_min_inner_size(winit::dpi::LogicalSize::new(min_w, min_h));
            log::debug!("Minimum window size: {}x{}", min_w, min_h);
        }

        if let Some(title) = settings.window_title {
            builder = builder.with_title(title);
        }

        Arc::new(
            builder
                .build(&event_loop)
                .map_err(|e| format!("Failed to create window: {:?}", e))?,
        )
    };

    // Create application
    let app = A::new();
    let title = app.title();
    window.set_title(&title);

    // Create application state
    let mut app_state = ApplicationState::new(app, Arc::clone(&window))?;

    // Track mouse state for event conversion
    let mut mouse_position = crate::Point::zero();
    // Track keyboard modifier state
    let mut current_modifiers = crate::Modifiers::default();

    // Run event loop
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);

            match event {
                WinitEvent::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::Resized(size) => {
                        app_state.renderer.resize(size.width, size.height);
                        // Send resize event to widgets so they can update their bounds
                        let ui_event = crate::Event::WindowResized {
                            width: size.width as f32,
                            height: size.height as f32,
                        };
                        app_state.handle_event(ui_event);
                        window.request_redraw();
                    }
                    WindowEvent::RedrawRequested => {
                        // Call tick for frame timing
                        if let Some(tick_msg) = app_state.app.tick() {
                            app_state.app.update(tick_msg);
                        }
                        app_state.render();
                        // Request next frame for continuous rendering
                        window.request_redraw();
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        mouse_position = crate::Point::new(position.x as f32, position.y as f32);
                        let ui_event = crate::Event::MouseMoved {
                            position: mouse_position,
                        };
                        app_state.handle_event(ui_event);
                        window.request_redraw();
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        let mouse_button = match button {
                            winit::event::MouseButton::Left => crate::MouseButton::Left,
                            winit::event::MouseButton::Right => crate::MouseButton::Right,
                            winit::event::MouseButton::Middle => crate::MouseButton::Middle,
                            winit::event::MouseButton::Other(n) => crate::MouseButton::Other(n),
                            _ => return,
                        };

                        let ui_event = match state {
                            winit::event::ElementState::Pressed => crate::Event::MousePressed {
                                button: mouse_button,
                                position: mouse_position,
                            },
                            winit::event::ElementState::Released => crate::Event::MouseReleased {
                                button: mouse_button,
                                position: mouse_position,
                            },
                        };

                        // Send event to widgets
                        app_state.handle_event(ui_event);
                        window.request_redraw();
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let delta_y = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_x, y) => y * 20.0,
                            winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                        };

                        let ui_event = crate::Event::MouseWheel {
                            delta: delta_y,
                            position: mouse_position,
                        };

                        // Send event to widgets
                        app_state.handle_event(ui_event);
                        window.request_redraw();
                    }
                    WindowEvent::ModifiersChanged(mods) => {
                        let state = mods.state();
                        current_modifiers = crate::Modifiers {
                            shift: state.shift_key(),
                            ctrl: state.control_key(),
                            alt: state.alt_key(),
                            meta: state.super_key(),
                        };
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if let Some(key) = convert_key(&event.logical_key) {
                            let ui_event = match event.state {
                                winit::event::ElementState::Pressed => crate::Event::KeyPressed {
                                    key,
                                    modifiers: current_modifiers,
                                },
                                winit::event::ElementState::Released => crate::Event::KeyReleased {
                                    key,
                                    modifiers: current_modifiers,
                                },
                            };
                            app_state.handle_event(ui_event);
                            window.request_redraw();
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .map_err(|e| format!("Event loop error: {:?}", e))
}

/// Convert winit key to hvat_ui key.
fn convert_key(key: &winit::keyboard::Key) -> Option<crate::Key> {
    use winit::keyboard::{Key as WinitKey, NamedKey};
    match key {
        WinitKey::Named(named) => match named {
            NamedKey::Enter => Some(crate::Key::Enter),
            NamedKey::Escape => Some(crate::Key::Escape),
            NamedKey::Backspace => Some(crate::Key::Backspace),
            NamedKey::Delete => Some(crate::Key::Delete),
            NamedKey::Tab => Some(crate::Key::Tab),
            NamedKey::Space => Some(crate::Key::Space),
            NamedKey::ArrowUp => Some(crate::Key::Up),
            NamedKey::ArrowDown => Some(crate::Key::Down),
            NamedKey::ArrowLeft => Some(crate::Key::Left),
            NamedKey::ArrowRight => Some(crate::Key::Right),
            NamedKey::Home => Some(crate::Key::Home),
            NamedKey::End => Some(crate::Key::End),
            NamedKey::PageUp => Some(crate::Key::PageUp),
            NamedKey::PageDown => Some(crate::Key::PageDown),
            _ => None,
        },
        WinitKey::Character(c) => {
            let chars: Vec<char> = c.chars().collect();
            if chars.len() == 1 {
                Some(crate::Key::Char(chars[0]))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Run an application with the given settings (WASM version).
///
/// This function creates a window, initializes the GPU context asynchronously,
/// and runs the event loop. For WASM, initialization must be async to avoid
/// blocking the browser's main thread.
#[cfg(target_arch = "wasm32")]
pub fn run<A: Application + 'static>(settings: Settings) -> Result<(), String> {
    use winit::event::{Event as WinitEvent, WindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::WindowBuilder;
    use winit::platform::web::WindowBuilderExtWebSys;
    use web_sys::wasm_bindgen::JsCast;
    use std::cell::RefCell;
    use std::rc::Rc;

    // Initialize logger
    if let Some(level) = settings.log_level.to_level() {
        console_log::init_with_level(level)
            .map_err(|e| format!("Failed to initialize logger: {:?}", e))?;
    }

    log::info!("Starting hvat_ui application (WASM)");
    log::debug!("Window size: {}x{}", settings.window_size.0, settings.window_size.1);

    // Store minimum size for clamping during resize
    let min_size = settings.min_window_size;

    // Create event loop
    let event_loop = EventLoop::new().map_err(|e| format!("Failed to create event loop: {:?}", e))?;

    // Create window with canvas that fills the browser window
    let window = {
        let web_window = web_sys::window().ok_or("No global window exists")?;
        let document = web_window.document().ok_or("No document in window")?;
        let body = document.body().ok_or("No body in document")?;

        // Get the actual browser window dimensions
        let browser_width = web_window.inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(settings.window_size.0 as f64) as u32;
        let browser_height = web_window.inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(settings.window_size.1 as f64) as u32;

        // Clamp to minimum size for initial render dimensions
        let (render_width, render_height) = clamp_to_min_size(browser_width, browser_height, min_size);

        log::info!("Browser window size: {}x{}, render size: {}x{}", browser_width, browser_height, render_width, render_height);

        // Remove any existing canvas with our ID to avoid "context already in use" errors
        if let Some(existing) = document.get_element_by_id("hvat-canvas") {
            existing.remove();
            log::info!("Removed existing canvas element");
        }

        let canvas = document
            .create_element("canvas")
            .map_err(|_| "Failed to create canvas")?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| "Failed to convert to canvas")?;

        // Set canvas pixel dimensions to the clamped render size
        canvas.set_width(render_width);
        canvas.set_height(render_height);

        // Apply CSS to ensure the canvas fills the viewport
        canvas.style().set_property("position", "fixed").ok();
        canvas.style().set_property("top", "0").ok();
        canvas.style().set_property("left", "0").ok();
        canvas.style().set_property("width", "100%").ok();
        canvas.style().set_property("height", "100%").ok();

        // Set canvas ID so we can find it later for resize
        canvas.set_id("hvat-canvas");

        body.append_child(&canvas)
            .map_err(|_| "Failed to append canvas to body")?;

        let mut builder = WindowBuilder::new()
            .with_inner_size(winit::dpi::LogicalSize::new(render_width, render_height))
            .with_resizable(settings.resizable)
            .with_canvas(Some(canvas));

        if let Some((min_w, min_h)) = settings.min_window_size {
            builder = builder.with_min_inner_size(winit::dpi::LogicalSize::new(min_w, min_h));
        }

        if let Some(ref title) = settings.window_title {
            builder = builder.with_title(title.clone());
        }

        Arc::new(
            builder
                .build(&event_loop)
                .map_err(|e| format!("Failed to create window: {:?}", e))?,
        )
    };

    // Create application
    let app = A::new();
    let title = app.title();
    window.set_title(&title);

    // Store app state in RefCell for async initialization
    // We'll initialize the renderer asynchronously and then start the event loop
    let app_state: Rc<RefCell<Option<ApplicationState<A>>>> = Rc::new(RefCell::new(None));
    let app_state_clone = Rc::clone(&app_state);
    let window_clone = Arc::clone(&window);

    // Spawn async initialization
    wasm_bindgen_futures::spawn_local(async move {
        match ApplicationState::new_async(app, window_clone.clone()).await {
            Ok(mut state) => {
                // Get actual window size for initial resize
                let size = window_clone.inner_size();
                state.renderer.resize(size.width, size.height);
                *app_state_clone.borrow_mut() = Some(state);
                log::info!("WASM renderer initialized successfully ({}x{})", size.width, size.height);
            }
            Err(e) => {
                log::error!("Failed to initialize renderer: {}", e);
            }
        }
    });

    // Track mouse state for event conversion
    let mut mouse_position = crate::Point::zero();
    // Track keyboard modifier state
    let mut current_modifiers = crate::Modifiers::default();

    // Run event loop
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);

            // Skip events until renderer is initialized
            let mut state_guard = app_state.borrow_mut();
            let Some(ref mut app_state) = *state_guard else {
                // Renderer not ready yet, request redraw to check again
                if matches!(event, WinitEvent::WindowEvent { event: WindowEvent::RedrawRequested, .. }) {
                    window.request_redraw();
                }
                return;
            };

            match event {
                WinitEvent::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::Resized(size) => {
                        // Clamp to minimum size for render dimensions
                        let (render_width, render_height) = clamp_to_min_size(size.width, size.height, min_size);

                        // Update the canvas pixel dimensions (not just CSS size)
                        if let Some(web_window) = web_sys::window() {
                            if let Some(document) = web_window.document() {
                                if let Some(canvas) = document.get_element_by_id("hvat-canvas") {
                                    if let Ok(canvas) = canvas.dyn_into::<web_sys::HtmlCanvasElement>() {
                                        canvas.set_width(render_width);
                                        canvas.set_height(render_height);
                                        log::debug!("Canvas resized to {}x{} (browser: {}x{})", render_width, render_height, size.width, size.height);
                                    }
                                }
                            }
                        }
                        app_state.renderer.resize(render_width, render_height);
                        // Send resize event to widgets so they can update their bounds
                        let ui_event = crate::Event::WindowResized {
                            width: render_width as f32,
                            height: render_height as f32,
                        };
                        app_state.handle_event(ui_event);
                        window.request_redraw();
                    }
                    WindowEvent::RedrawRequested => {
                        // Check for browser window resize on each frame
                        if let Some(web_window) = web_sys::window() {
                            let browser_width = web_window.inner_width()
                                .ok()
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0) as u32;
                            let browser_height = web_window.inner_height()
                                .ok()
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0) as u32;

                            // Clamp to minimum size for render dimensions
                            let (render_width, render_height) = clamp_to_min_size(browser_width, browser_height, min_size);

                            let current_size = window.inner_size();
                            if render_width > 0 && render_height > 0 &&
                               (render_width != current_size.width || render_height != current_size.height) {
                                // Browser window was resized, update canvas with clamped dimensions
                                if let Some(document) = web_window.document() {
                                    if let Some(canvas) = document.get_element_by_id("hvat-canvas") {
                                        if let Ok(canvas) = canvas.dyn_into::<web_sys::HtmlCanvasElement>() {
                                            canvas.set_width(render_width);
                                            canvas.set_height(render_height);
                                            log::debug!("Browser resize detected: {}x{}, render: {}x{}", browser_width, browser_height, render_width, render_height);
                                        }
                                    }
                                }
                                app_state.renderer.resize(render_width, render_height);
                                let _ = window.request_inner_size(winit::dpi::LogicalSize::new(render_width, render_height));
                            }
                        }

                        // Call tick for frame timing
                        if let Some(tick_msg) = app_state.app.tick() {
                            app_state.app.update(tick_msg);
                        }
                        app_state.render();
                        // Request next frame for continuous rendering
                        window.request_redraw();
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        mouse_position = crate::Point::new(position.x as f32, position.y as f32);
                        let ui_event = crate::Event::MouseMoved {
                            position: mouse_position,
                        };
                        app_state.handle_event(ui_event);
                        window.request_redraw();
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        let mouse_button = match button {
                            winit::event::MouseButton::Left => crate::MouseButton::Left,
                            winit::event::MouseButton::Right => crate::MouseButton::Right,
                            winit::event::MouseButton::Middle => crate::MouseButton::Middle,
                            winit::event::MouseButton::Other(n) => crate::MouseButton::Other(n),
                            _ => return,
                        };

                        let ui_event = match state {
                            winit::event::ElementState::Pressed => crate::Event::MousePressed {
                                button: mouse_button,
                                position: mouse_position,
                            },
                            winit::event::ElementState::Released => crate::Event::MouseReleased {
                                button: mouse_button,
                                position: mouse_position,
                            },
                        };

                        // Send event to widgets
                        app_state.handle_event(ui_event);
                        window.request_redraw();
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let delta_y = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_x, y) => y * 20.0,
                            winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                        };

                        let ui_event = crate::Event::MouseWheel {
                            delta: delta_y,
                            position: mouse_position,
                        };

                        // Send event to widgets
                        app_state.handle_event(ui_event);
                        window.request_redraw();
                    }
                    WindowEvent::ModifiersChanged(mods) => {
                        let state = mods.state();
                        current_modifiers = crate::Modifiers {
                            shift: state.shift_key(),
                            ctrl: state.control_key(),
                            alt: state.alt_key(),
                            meta: state.super_key(),
                        };
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if let Some(key) = convert_key(&event.logical_key) {
                            let ui_event = match event.state {
                                winit::event::ElementState::Pressed => crate::Event::KeyPressed {
                                    key,
                                    modifiers: current_modifiers,
                                },
                                winit::event::ElementState::Released => crate::Event::KeyReleased {
                                    key,
                                    modifiers: current_modifiers,
                                },
                            };
                            app_state.handle_event(ui_event);
                            window.request_redraw();
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .map_err(|e| format!("Event loop error: {:?}", e))
}

/// The application runtime state.
/// This is internal and managed by the framework.
pub(crate) struct ApplicationState<A: Application> {
    pub app: A,
    pub renderer: Renderer,
    pub layout_cache: crate::LayoutCache,
}

impl<A: Application> ApplicationState<A> {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(app: A, window: Arc<Window>) -> Result<Self, String> {
        let renderer = pollster::block_on(async {
            Renderer::new(window).await
        })?;

        Ok(Self { app, renderer, layout_cache: crate::LayoutCache::new() })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_async(app: A, window: Arc<Window>) -> Result<Self, String> {
        let renderer = Renderer::new(window).await?;
        Ok(Self { app, renderer, layout_cache: crate::LayoutCache::new() })
    }

    pub fn update(&mut self, message: A::Message) {
        self.app.update(message);
    }

    pub fn view(&self) -> Element<A::Message> {
        self.app.view()
    }

    pub fn render(&mut self) {
        // Begin frame for layout cache
        self.layout_cache.begin_frame();

        let element = self.app.view();
        self.renderer.render(element);

        // End frame and cleanup stale cache entries
        self.layout_cache.end_frame();
    }

    pub fn handle_event(&mut self, event: crate::Event) {
        // Get the current view and process the event
        let message = {
            let mut element = self.app.view();

            // Layout the element to get bounds
            let limits = crate::Limits::new(
                self.renderer.size().0 as f32,
                self.renderer.size().1 as f32,
            );
            let layout = element.widget().layout(&limits);

            // Send event to widgets and collect messages
            element.widget_mut().on_event(&event, &layout)
        }; // element is dropped here, releasing the borrow

        // Now we can update the application state
        if let Some(message) = message {
            self.app.update(message);
        }
    }

    /// Invalidate the layout cache. Call when the widget tree structure changes.
    pub fn invalidate_layout(&mut self) {
        self.layout_cache.invalidate();
    }

    /// Get layout cache statistics for debugging.
    pub fn layout_cache_stats(&self) -> crate::CacheStats {
        self.layout_cache.stats()
    }
}

/// Helper function to clamp dimensions to minimum size (WASM only).
/// Returns (width, height) clamped to at least min_size if provided.
#[cfg(target_arch = "wasm32")]
fn clamp_to_min_size(width: u32, height: u32, min_size: Option<(u32, u32)>) -> (u32, u32) {
    match min_size {
        Some((min_w, min_h)) => (width.max(min_w), height.max(min_h)),
        None => (width, height),
    }
}
