//! Pipeline builder for reducing boilerplate in render pipeline creation.
//!
//! Provides a fluent API for creating wgpu render pipelines with sensible defaults
//! for 2D rendering.

/// Builder for creating wgpu render pipelines with common defaults.
///
/// # Example
/// ```ignore
/// let pipeline = PipelineBuilder::new(&device, format)
///     .with_shader(shader, "vs_main", "fs_main")
///     .with_vertex_layout::<Vertex>()
///     .with_blend_state(wgpu::BlendState::ALPHA_BLENDING)
///     .with_bind_group_layouts(&[&uniform_layout, &texture_layout])
///     .build();
/// ```
pub struct PipelineBuilder<'a> {
    device: &'a wgpu::Device,
    format: wgpu::TextureFormat,
    label: Option<&'a str>,
    shader: Option<&'a wgpu::ShaderModule>,
    vs_entry: &'a str,
    fs_entry: &'a str,
    vertex_buffers: Vec<wgpu::VertexBufferLayout<'a>>,
    bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
    blend_state: Option<wgpu::BlendState>,
    cull_mode: Option<wgpu::Face>,
    topology: wgpu::PrimitiveTopology,
}

impl<'a> PipelineBuilder<'a> {
    /// Create a new pipeline builder with default 2D settings.
    pub fn new(device: &'a wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self {
            device,
            format,
            label: None,
            shader: None,
            vs_entry: "vs_main",
            fs_entry: "fs_main",
            vertex_buffers: Vec::new(),
            bind_group_layouts: Vec::new(),
            blend_state: None,
            cull_mode: None, // No culling for 2D by default
            topology: wgpu::PrimitiveTopology::TriangleList,
        }
    }

    /// Set the pipeline label for debugging.
    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Set the shader module and entry points.
    pub fn with_shader(
        mut self,
        shader: &'a wgpu::ShaderModule,
        vs_entry: &'a str,
        fs_entry: &'a str,
    ) -> Self {
        self.shader = Some(shader);
        self.vs_entry = vs_entry;
        self.fs_entry = fs_entry;
        self
    }

    /// Add a vertex buffer layout.
    pub fn with_vertex_buffer(mut self, layout: wgpu::VertexBufferLayout<'a>) -> Self {
        self.vertex_buffers.push(layout);
        self
    }

    /// Set all bind group layouts.
    pub fn with_bind_group_layouts(mut self, layouts: &[&'a wgpu::BindGroupLayout]) -> Self {
        self.bind_group_layouts = layouts.to_vec();
        self
    }

    /// Set the blend state (default: REPLACE).
    pub fn with_blend_state(mut self, blend: wgpu::BlendState) -> Self {
        self.blend_state = Some(blend);
        self
    }

    /// Set the cull mode (default: None for 2D).
    pub fn with_cull_mode(mut self, cull: Option<wgpu::Face>) -> Self {
        self.cull_mode = cull;
        self
    }

    /// Set the primitive topology (default: TriangleList).
    pub fn with_topology(mut self, topology: wgpu::PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    /// Build the render pipeline.
    ///
    /// # Panics
    /// Panics if no shader module was provided.
    pub fn build(self) -> wgpu::RenderPipeline {
        let shader = self.shader.expect("PipelineBuilder requires a shader module");

        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: self.label.map(|l| format!("{} Layout", l)).as_deref(),
            bind_group_layouts: &self.bind_group_layouts,
            push_constant_ranges: &[],
        });

        self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: self.label,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some(self.vs_entry),
                buffers: &self.vertex_buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(self.fs_entry),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.format,
                    blend: self.blend_state,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: self.topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: self.cull_mode,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }
}

/// Helper for creating common bind group layout entries.
pub struct BindGroupLayoutBuilder<'a> {
    device: &'a wgpu::Device,
    label: Option<&'a str>,
    entries: Vec<wgpu::BindGroupLayoutEntry>,
}

impl<'a> BindGroupLayoutBuilder<'a> {
    /// Create a new bind group layout builder.
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self {
            device,
            label: None,
            entries: Vec::new(),
        }
    }

    /// Set the layout label.
    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Add a uniform buffer entry.
    pub fn add_uniform_buffer(mut self, binding: u32, visibility: wgpu::ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self
    }

    /// Add a 2D texture entry.
    pub fn add_texture_2d(mut self, binding: u32, visibility: wgpu::ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            },
            count: None,
        });
        self
    }

    /// Add a filtering sampler entry.
    pub fn add_sampler(mut self, binding: u32, visibility: wgpu::ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        self
    }

    /// Build the bind group layout.
    pub fn build(self) -> wgpu::BindGroupLayout {
        self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: self.label,
            entries: &self.entries,
        })
    }
}
