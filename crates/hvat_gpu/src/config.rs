//! Configuration structs for GPU settings.
//!
//! Provides configurable options for GPU context, textures, and rendering
//! with sensible defaults for 2D image viewing applications.

/// Configuration for GPU context initialization.
#[derive(Debug, Clone)]
pub struct GpuConfig {
    /// Power preference for adapter selection.
    pub power_preference: wgpu::PowerPreference,
    /// Present mode (VSync behavior).
    pub present_mode: wgpu::PresentMode,
    /// Maximum frames in flight.
    pub max_frame_latency: u32,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::default(),
            present_mode: wgpu::PresentMode::Fifo, // VSync on
            max_frame_latency: 2,
        }
    }
}

impl GpuConfig {
    /// Create config optimized for low latency (gaming/interactive).
    pub fn low_latency() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            present_mode: wgpu::PresentMode::Mailbox, // Low latency, may tear
            max_frame_latency: 1,
        }
    }

    /// Create config optimized for power efficiency.
    pub fn power_saving() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::LowPower,
            present_mode: wgpu::PresentMode::Fifo,
            max_frame_latency: 2,
        }
    }

    /// Set power preference.
    pub fn with_power_preference(mut self, pref: wgpu::PowerPreference) -> Self {
        self.power_preference = pref;
        self
    }

    /// Set present mode.
    pub fn with_present_mode(mut self, mode: wgpu::PresentMode) -> Self {
        self.present_mode = mode;
        self
    }

    /// Set maximum frame latency.
    pub fn with_max_frame_latency(mut self, latency: u32) -> Self {
        self.max_frame_latency = latency;
        self
    }
}

/// Configuration for texture creation and sampling.
#[derive(Debug, Clone)]
pub struct TextureConfig {
    /// Magnification filter mode.
    pub mag_filter: wgpu::FilterMode,
    /// Minification filter mode.
    pub min_filter: wgpu::FilterMode,
    /// Mipmap filter mode.
    pub mipmap_filter: wgpu::FilterMode,
    /// Address mode for U coordinate.
    pub address_mode_u: wgpu::AddressMode,
    /// Address mode for V coordinate.
    pub address_mode_v: wgpu::AddressMode,
}

impl Default for TextureConfig {
    fn default() -> Self {
        Self {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
        }
    }
}

impl TextureConfig {
    /// Create config for pixel-perfect rendering (no interpolation).
    pub fn nearest() -> Self {
        Self {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
        }
    }

    /// Create config for smooth interpolation.
    pub fn linear() -> Self {
        Self::default()
    }

    /// Set magnification filter.
    pub fn with_mag_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.mag_filter = filter;
        self
    }

    /// Set minification filter.
    pub fn with_min_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.min_filter = filter;
        self
    }

    /// Set address mode for both U and V.
    pub fn with_address_mode(mut self, mode: wgpu::AddressMode) -> Self {
        self.address_mode_u = mode;
        self.address_mode_v = mode;
        self
    }
}

/// Configuration for render passes.
#[derive(Debug, Clone, Copy)]
pub struct RenderConfig {
    /// Clear color for the background.
    pub clear_color: ClearColor,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            clear_color: ClearColor::DARK_GRAY,
        }
    }
}

impl RenderConfig {
    /// Create config with a specific clear color.
    pub fn with_clear_color(mut self, color: ClearColor) -> Self {
        self.clear_color = color;
        self
    }
}

/// Clear color for render passes.
#[derive(Debug, Clone, Copy)]
pub struct ClearColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl ClearColor {
    /// Dark gray (default for image viewers).
    pub const DARK_GRAY: ClearColor = ClearColor {
        r: 0.1,
        g: 0.1,
        b: 0.1,
        a: 1.0,
    };
    /// Black.
    pub const BLACK: ClearColor = ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// White.
    pub const WHITE: ClearColor = ClearColor {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    /// Transparent.
    pub const TRANSPARENT: ClearColor = ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    /// Create a custom clear color.
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    /// Create from RGB (alpha = 1.0).
    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Convert to wgpu::Color.
    pub fn to_wgpu(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

impl From<ClearColor> for wgpu::Color {
    fn from(c: ClearColor) -> Self {
        c.to_wgpu()
    }
}
