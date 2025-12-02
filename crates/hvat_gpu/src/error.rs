use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("Failed to find suitable GPU adapter")]
    AdapterNotFound,

    #[error("Failed to request adapter: {0}")]
    AdapterRequest(#[from] wgpu::RequestAdapterError),

    #[error("Failed to request device: {0}")]
    DeviceRequest(#[from] wgpu::RequestDeviceError),

    #[error("Failed to create surface: {0}")]
    SurfaceCreation(#[from] wgpu::CreateSurfaceError),

    #[error("Surface configuration error: incompatible surface")]
    SurfaceConfigError,

    #[error("Texture error: {0}")]
    Texture(String),

    #[error("Shader compilation error: {0}")]
    ShaderCompilation(String),
}

pub type Result<T> = std::result::Result<T, GpuError>;
