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
                    _ => {}
                },
                _ => {}
            }
        })
        .map_err(|e| format!("Event loop error: {:?}", e))
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

    // Create event loop
    let event_loop = EventLoop::new().map_err(|e| format!("Failed to create event loop: {:?}", e))?;

    // Create window with canvas
    let window = {
        let mut builder = WindowBuilder::new()
            .with_inner_size(winit::dpi::LogicalSize::new(
                settings.window_size.0,
                settings.window_size.1,
            ))
            .with_resizable(settings.resizable);

        if let Some(ref title) = settings.window_title {
            builder = builder.with_title(title.clone());
        }

        let web_window = web_sys::window().ok_or("No global window exists")?;
        let document = web_window.document().ok_or("No document in window")?;
        let body = document.body().ok_or("No body in document")?;

        let canvas = document
            .create_element("canvas")
            .map_err(|_| "Failed to create canvas")?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| "Failed to convert to canvas")?;

        canvas.set_width(settings.window_size.0);
        canvas.set_height(settings.window_size.1);

        body.append_child(&canvas)
            .map_err(|_| "Failed to append canvas to body")?;

        builder = builder.with_canvas(Some(canvas));

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
    let window_size = settings.window_size;

    // Spawn async initialization
    wasm_bindgen_futures::spawn_local(async move {
        match ApplicationState::new_async(app, window_clone).await {
            Ok(mut state) => {
                // Force resize to configured size
                state.renderer.resize(window_size.0, window_size.1);
                *app_state_clone.borrow_mut() = Some(state);
                log::info!("WASM renderer initialized successfully");
            }
            Err(e) => {
                log::error!("Failed to initialize renderer: {}", e);
            }
        }
    });

    // Track mouse state for event conversion
    let mut mouse_position = crate::Point::zero();

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
                        app_state.renderer.resize(size.width, size.height);
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
}

impl<A: Application> ApplicationState<A> {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(app: A, window: Arc<Window>) -> Result<Self, String> {
        let renderer = pollster::block_on(async {
            Renderer::new(window).await
        })?;

        Ok(Self { app, renderer })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_async(app: A, window: Arc<Window>) -> Result<Self, String> {
        let renderer = Renderer::new(window).await?;
        Ok(Self { app, renderer })
    }

    pub fn update(&mut self, message: A::Message) {
        self.app.update(message);
    }

    pub fn view(&self) -> Element<A::Message> {
        self.app.view()
    }

    pub fn render(&mut self) {
        let element = self.app.view();
        self.renderer.render(element);
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
}
