//! Color rendering pipeline for solid color shapes.

use wgpu::util::DeviceExt;

use super::Pipeline;
use crate::vertex::ColorVertex;

/// Pipeline for rendering solid color rectangles and shapes.
pub struct ColorPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
}

impl ColorPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        // Load shader from file
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Color Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/color.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Color Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Color Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[ColorVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // No culling for 2D
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
        });

        Self { render_pipeline }
    }

    /// Create vertex and index buffers for a filled rectangle.
    pub fn create_rect_vertices(
        device: &wgpu::Device,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        window_width: f32,
        window_height: f32,
    ) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        // Convert from screen coordinates to NDC (-1 to 1)
        let x1 = (x / window_width) * 2.0 - 1.0;
        let y1 = 1.0 - (y / window_height) * 2.0;
        let x2 = ((x + width) / window_width) * 2.0 - 1.0;
        let y2 = 1.0 - ((y + height) / window_height) * 2.0;

        let vertices = [
            ColorVertex {
                position: [x1, y1],
                color,
            }, // Top-left
            ColorVertex {
                position: [x2, y1],
                color,
            }, // Top-right
            ColorVertex {
                position: [x2, y2],
                color,
            }, // Bottom-right
            ColorVertex {
                position: [x1, y2],
                color,
            }, // Bottom-left
        ];

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rect Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rect Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer, indices.len() as u32)
    }

    /// Create vertex and index buffers for a stroked rectangle (outline).
    pub fn create_stroke_rect_vertices(
        device: &wgpu::Device,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        thickness: f32,
        window_width: f32,
        window_height: f32,
    ) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        // Convert from screen coordinates to NDC
        let x1 = (x / window_width) * 2.0 - 1.0;
        let y1 = 1.0 - (y / window_height) * 2.0;
        let x2 = ((x + width) / window_width) * 2.0 - 1.0;
        let y2 = 1.0 - ((y + height) / window_height) * 2.0;

        // Convert thickness to NDC
        let t_x = (thickness / window_width) * 2.0;
        let t_y = (thickness / window_height) * 2.0;

        // Create 4 rectangles for the stroke (top, right, bottom, left)
        let vertices = vec![
            // Top edge
            ColorVertex { position: [x1, y1], color },
            ColorVertex { position: [x2, y1], color },
            ColorVertex { position: [x2, y1 - t_y], color },
            ColorVertex { position: [x1, y1 - t_y], color },
            // Right edge
            ColorVertex { position: [x2 - t_x, y1], color },
            ColorVertex { position: [x2, y1], color },
            ColorVertex { position: [x2, y2], color },
            ColorVertex { position: [x2 - t_x, y2], color },
            // Bottom edge
            ColorVertex { position: [x1, y2 + t_y], color },
            ColorVertex { position: [x2, y2 + t_y], color },
            ColorVertex { position: [x2, y2], color },
            ColorVertex { position: [x1, y2], color },
            // Left edge
            ColorVertex { position: [x1, y1], color },
            ColorVertex { position: [x1 + t_x, y1], color },
            ColorVertex { position: [x1 + t_x, y2], color },
            ColorVertex { position: [x1, y2], color },
        ];

        let indices: Vec<u16> = vec![
            // Top
            0, 1, 2, 0, 2, 3,
            // Right
            4, 5, 6, 4, 6, 7,
            // Bottom
            8, 9, 10, 8, 10, 11,
            // Left
            12, 13, 14, 12, 14, 15,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stroke Rect Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stroke Rect Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer, indices.len() as u32)
    }
}

impl Pipeline for ColorPipeline {
    fn render_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.render_pipeline
    }
}
