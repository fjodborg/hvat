use std::sync::Arc;
use wgpu;
use winit::window::Window;

use crate::config::GpuConfig;
use crate::error::Result;

/// Check if WebGPU is supported in the browser
#[cfg(target_arch = "wasm32")]
fn is_webgpu_supported() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };

    let navigator = window.navigator();

    // Check if navigator.gpu exists
    let gpu = js_sys::Reflect::get(&navigator, &wasm_bindgen::JsValue::from_str("gpu"));

    match gpu {
        Ok(val) => !val.is_undefined() && !val.is_null(),
        Err(_) => false,
    }
}

/// Main GPU context managing wgpu device, queue, and surface
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub config: GpuConfig,
}

impl GpuContext {
    /// Initialize GPU context for a window with default configuration.
    ///
    /// This is async to support both native and WASM backends.
    /// On native, you can use `pollster::block_on()` to call this.
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        Self::with_config(window, GpuConfig::default()).await
    }

    /// Initialize GPU context for a window with custom configuration.
    pub async fn with_config(window: Arc<Window>, config: GpuConfig) -> Result<Self> {
        #[cfg(target_arch = "wasm32")]
        {
            // Check if WebGPU is supported via navigator.gpu
            if is_webgpu_supported() {
                web_sys::console::log_1(&"🔍 WebGPU detected via navigator.gpu".into());
                web_sys::console::log_1(&"⚠️  Attempting WebGPU initialization (experimental in wgpu 27)...".into());

                // Try WebGPU first
                match Self::new_with_backend(window.clone(), wgpu::Backends::BROWSER_WEBGPU, config.clone()).await {
                    Ok(ctx) => {
                        web_sys::console::log_1(&"✅ WebGPU initialization successful!".into());
                        return Ok(ctx);
                    }
                    Err(e) => {
                        web_sys::console::log_1(&format!("❌ WebGPU initialization failed: {:?}", e).into());
                        web_sys::console::log_1(&"🔄 Falling back to WebGL...".into());
                    }
                }
            } else {
                web_sys::console::log_1(&"ℹ️  WebGPU not available (navigator.gpu not found)".into());
                web_sys::console::log_1(&"🔄 Using WebGL backend...".into());
            }

            // Fall back to WebGL
            Self::new_with_backend(window, wgpu::Backends::GL, config).await
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::new_with_backend(window, wgpu::Backends::PRIMARY, config).await
        }
    }

    /// Initialize GPU context with a specific backend
    #[cfg(target_arch = "wasm32")]
    async fn new_with_backend(window: Arc<Window>, backends: wgpu::Backends, config: GpuConfig) -> Result<Self> {
        web_sys::console::log_1(&format!("Initializing GPU with backend: {:?}", backends).into());

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: config.power_preference,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let info = adapter.get_info();
        web_sys::console::log_1(&format!("✓ GPU initialized with backend: {:?}", info.backend).into());

        Self::finish_init(adapter, surface, window, config).await
    }

    /// Initialize GPU context with a specific backend (native)
    #[cfg(not(target_arch = "wasm32"))]
    async fn new_with_backend(window: Arc<Window>, backends: wgpu::Backends, config: GpuConfig) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: config.power_preference,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        Self::finish_init(adapter, surface, window, config).await
    }

    /// Complete GPU context initialization with adapter and surface
    async fn finish_init(
        adapter: wgpu::Adapter,
        surface: wgpu::Surface<'static>,
        window: Arc<Window>,
        config: GpuConfig,
    ) -> Result<Self> {

        // Use adapter limits to ensure compatibility with WebGL
        // WebGL doesn't support compute shaders, so we can't use Limits::default()
        let limits = wgpu::Limits::downlevel_webgl2_defaults()
            .using_resolution(adapter.limits());

        // Request device and queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Main Device"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        // Configure surface
        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        // Use present mode from config, falling back to Fifo if not supported
        let present_mode = if surface_caps.present_modes.contains(&config.present_mode) {
            config.present_mode
        } else {
            wgpu::PresentMode::Fifo // Always supported
        };

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: config.max_frame_latency,
        };

        surface.configure(&device, &surface_config);

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            config,
        })
    }

    /// Handle window resize
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        self.surface_config.width = new_width.max(1);
        self.surface_config.height = new_height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Get current surface width
    pub fn width(&self) -> u32 {
        self.surface_config.width
    }

    /// Get current surface height
    pub fn height(&self) -> u32 {
        self.surface_config.height
    }

    /// Get aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.surface_config.width as f32 / self.surface_config.height as f32
    }
}
