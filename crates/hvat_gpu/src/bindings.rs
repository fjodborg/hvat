//! Shader binding constants.
//!
//! This module defines constants for shader binding locations that are shared
//! between Rust code and WGSL shaders. This ensures that binding numbers stay
//! synchronized and prevents runtime failures due to mismatches.
//!
//! # Usage
//!
//! In Rust code:
//! ```ignore
//! use hvat_gpu::bindings::texture::*;
//! layout_builder.add_uniform_buffer(UNIFORM_TRANSFORM_BINDING, ...);
//! ```
//!
//! In WGSL shaders, these same numbers must be used:
//! ```wgsl
//! @group(0) @binding(0)  // UNIFORM_GROUP, UNIFORM_TRANSFORM_BINDING
//! var<uniform> transform: Transform;
//! ```

/// Binding constants for the texture pipeline.
pub mod texture {
    /// Group 0: Uniforms
    pub const UNIFORM_GROUP: u32 = 0;
    /// Binding 0 in group 0: Transform matrix uniform
    pub const UNIFORM_TRANSFORM_BINDING: u32 = 0;
    /// Binding 1 in group 0: Image adjustments uniform
    pub const UNIFORM_ADJUSTMENTS_BINDING: u32 = 1;

    /// Group 1: Texture resources
    pub const TEXTURE_GROUP: u32 = 1;
    /// Binding 0 in group 1: Texture 2D
    pub const TEXTURE_BINDING: u32 = 0;
    /// Binding 1 in group 1: Sampler
    pub const SAMPLER_BINDING: u32 = 1;
}

/// Binding constants for the color pipeline.
pub mod color {
    /// Group 0: Uniforms
    pub const UNIFORM_GROUP: u32 = 0;
    /// Binding 0 in group 0: Transform matrix uniform
    pub const UNIFORM_TRANSFORM_BINDING: u32 = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_bindings_are_sequential() {
        // Verify uniforms are in group 0 with sequential bindings
        assert_eq!(texture::UNIFORM_GROUP, 0);
        assert_eq!(texture::UNIFORM_TRANSFORM_BINDING, 0);
        assert_eq!(texture::UNIFORM_ADJUSTMENTS_BINDING, 1);

        // Verify textures are in group 1 with sequential bindings
        assert_eq!(texture::TEXTURE_GROUP, 1);
        assert_eq!(texture::TEXTURE_BINDING, 0);
        assert_eq!(texture::SAMPLER_BINDING, 1);
    }

    #[test]
    fn test_color_bindings() {
        assert_eq!(color::UNIFORM_GROUP, 0);
        assert_eq!(color::UNIFORM_TRANSFORM_BINDING, 0);
    }
}
