//! Color rendering pipeline for solid color shapes.

use wgpu::util::DeviceExt;

use super::{Pipeline, PipelineBuilder};
use crate::vertex::ColorVertex;

/// Pipeline for rendering solid color rectangles and shapes.
pub struct ColorPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
}

impl ColorPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Color Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/color.wgsl").into()),
        });

        let render_pipeline = PipelineBuilder::new(device, format)
            .with_label("Color Render Pipeline")
            .with_shader(&shader, "vs_main", "fs_main")
            .with_vertex_buffer(ColorVertex::desc())
            .with_blend_state(wgpu::BlendState::ALPHA_BLENDING)
            .build();

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

impl ColorPipeline {
    /// Create vertex and index buffers for a line segment.
    pub fn create_line_vertices(
        device: &wgpu::Device,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [f32; 4],
        thickness: f32,
        window_width: f32,
        window_height: f32,
    ) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        // Convert to NDC
        let x1_ndc = (x1 / window_width) * 2.0 - 1.0;
        let y1_ndc = 1.0 - (y1 / window_height) * 2.0;
        let x2_ndc = (x2 / window_width) * 2.0 - 1.0;
        let y2_ndc = 1.0 - (y2 / window_height) * 2.0;

        // Calculate perpendicular direction for thickness
        let dx = x2_ndc - x1_ndc;
        let dy = y2_ndc - y1_ndc;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.0001 {
            // Zero-length line
            return Self::create_rect_vertices(device, x1, y1, 1.0, 1.0, color, window_width, window_height);
        }

        // Normalize and scale by half thickness (in NDC)
        let t_x = (thickness / window_width) * 2.0 / 2.0;
        let t_y = (thickness / window_height) * 2.0 / 2.0;
        let nx = -dy / len * t_y;
        let ny = dx / len * t_x;

        let vertices = [
            ColorVertex { position: [x1_ndc - nx, y1_ndc - ny], color },
            ColorVertex { position: [x1_ndc + nx, y1_ndc + ny], color },
            ColorVertex { position: [x2_ndc + nx, y2_ndc + ny], color },
            ColorVertex { position: [x2_ndc - nx, y2_ndc - ny], color },
        ];

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer, indices.len() as u32)
    }

    /// Create vertex and index buffers for a filled circle.
    pub fn create_circle_vertices(
        device: &wgpu::Device,
        cx: f32,
        cy: f32,
        radius: f32,
        color: [f32; 4],
        window_width: f32,
        window_height: f32,
    ) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        const SEGMENTS: usize = 16;

        let cx_ndc = (cx / window_width) * 2.0 - 1.0;
        let cy_ndc = 1.0 - (cy / window_height) * 2.0;
        let rx = (radius / window_width) * 2.0;
        let ry = (radius / window_height) * 2.0;

        let mut vertices = Vec::with_capacity(SEGMENTS + 1);
        // Center vertex
        vertices.push(ColorVertex { position: [cx_ndc, cy_ndc], color });

        // Circle vertices
        for i in 0..SEGMENTS {
            let angle = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
            let x = cx_ndc + rx * angle.cos();
            let y = cy_ndc + ry * angle.sin();
            vertices.push(ColorVertex { position: [x, y], color });
        }

        // Indices for triangle fan
        let mut indices = Vec::with_capacity(SEGMENTS * 3);
        for i in 0..SEGMENTS {
            indices.push(0u16);
            indices.push((i + 1) as u16);
            indices.push(((i + 1) % SEGMENTS + 1) as u16);
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Circle Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Circle Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer, indices.len() as u32)
    }

    /// Create vertex and index buffers for a stroked circle (outline).
    pub fn create_stroke_circle_vertices(
        device: &wgpu::Device,
        cx: f32,
        cy: f32,
        radius: f32,
        color: [f32; 4],
        thickness: f32,
        window_width: f32,
        window_height: f32,
    ) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        const SEGMENTS: usize = 24;

        let cx_ndc = (cx / window_width) * 2.0 - 1.0;
        let cy_ndc = 1.0 - (cy / window_height) * 2.0;
        let rx_inner = ((radius - thickness / 2.0) / window_width) * 2.0;
        let ry_inner = ((radius - thickness / 2.0) / window_height) * 2.0;
        let rx_outer = ((radius + thickness / 2.0) / window_width) * 2.0;
        let ry_outer = ((radius + thickness / 2.0) / window_height) * 2.0;

        let mut vertices = Vec::with_capacity(SEGMENTS * 2);

        // Generate inner and outer ring vertices
        for i in 0..SEGMENTS {
            let angle = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            // Inner vertex
            vertices.push(ColorVertex {
                position: [cx_ndc + rx_inner * cos_a, cy_ndc + ry_inner * sin_a],
                color,
            });
            // Outer vertex
            vertices.push(ColorVertex {
                position: [cx_ndc + rx_outer * cos_a, cy_ndc + ry_outer * sin_a],
                color,
            });
        }

        // Indices forming quads between inner and outer ring
        let mut indices = Vec::with_capacity(SEGMENTS * 6);
        for i in 0..SEGMENTS {
            let i0 = (i * 2) as u16;
            let i1 = (i * 2 + 1) as u16;
            let i2 = ((i + 1) % SEGMENTS * 2) as u16;
            let i3 = ((i + 1) % SEGMENTS * 2 + 1) as u16;
            indices.extend_from_slice(&[i0, i1, i3, i0, i3, i2]);
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stroke Circle Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stroke Circle Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer, indices.len() as u32)
    }

    // =========================================================================
    // Batched vertex generation (no GPU buffer allocation)
    // =========================================================================

    /// Append vertices/indices for a filled rectangle to existing vectors.
    /// Returns the number of indices added.
    pub fn append_rect(
        vertices: &mut Vec<ColorVertex>,
        indices: &mut Vec<u16>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        window_width: f32,
        window_height: f32,
    ) -> u32 {
        let base_idx = vertices.len() as u16;

        // Convert from screen coordinates to NDC (-1 to 1)
        let x1 = (x / window_width) * 2.0 - 1.0;
        let y1 = 1.0 - (y / window_height) * 2.0;
        let x2 = ((x + width) / window_width) * 2.0 - 1.0;
        let y2 = 1.0 - ((y + height) / window_height) * 2.0;

        vertices.extend_from_slice(&[
            ColorVertex { position: [x1, y1], color }, // Top-left
            ColorVertex { position: [x2, y1], color }, // Top-right
            ColorVertex { position: [x2, y2], color }, // Bottom-right
            ColorVertex { position: [x1, y2], color }, // Bottom-left
        ]);

        indices.extend_from_slice(&[
            base_idx, base_idx + 1, base_idx + 2,
            base_idx, base_idx + 2, base_idx + 3,
        ]);

        6
    }

    /// Append vertices/indices for a stroked rectangle to existing vectors.
    pub fn append_stroke_rect(
        vertices: &mut Vec<ColorVertex>,
        indices: &mut Vec<u16>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        thickness: f32,
        window_width: f32,
        window_height: f32,
    ) -> u32 {
        let base_idx = vertices.len() as u16;

        let x1 = (x / window_width) * 2.0 - 1.0;
        let y1 = 1.0 - (y / window_height) * 2.0;
        let x2 = ((x + width) / window_width) * 2.0 - 1.0;
        let y2 = 1.0 - ((y + height) / window_height) * 2.0;

        let t_x = (thickness / window_width) * 2.0;
        let t_y = (thickness / window_height) * 2.0;

        // 4 rectangles for the stroke (top, right, bottom, left)
        vertices.extend_from_slice(&[
            // Top edge (0-3)
            ColorVertex { position: [x1, y1], color },
            ColorVertex { position: [x2, y1], color },
            ColorVertex { position: [x2, y1 - t_y], color },
            ColorVertex { position: [x1, y1 - t_y], color },
            // Right edge (4-7)
            ColorVertex { position: [x2 - t_x, y1], color },
            ColorVertex { position: [x2, y1], color },
            ColorVertex { position: [x2, y2], color },
            ColorVertex { position: [x2 - t_x, y2], color },
            // Bottom edge (8-11)
            ColorVertex { position: [x1, y2 + t_y], color },
            ColorVertex { position: [x2, y2 + t_y], color },
            ColorVertex { position: [x2, y2], color },
            ColorVertex { position: [x1, y2], color },
            // Left edge (12-15)
            ColorVertex { position: [x1, y1], color },
            ColorVertex { position: [x1 + t_x, y1], color },
            ColorVertex { position: [x1 + t_x, y2], color },
            ColorVertex { position: [x1, y2], color },
        ]);

        indices.extend_from_slice(&[
            // Top
            base_idx, base_idx + 1, base_idx + 2, base_idx, base_idx + 2, base_idx + 3,
            // Right
            base_idx + 4, base_idx + 5, base_idx + 6, base_idx + 4, base_idx + 6, base_idx + 7,
            // Bottom
            base_idx + 8, base_idx + 9, base_idx + 10, base_idx + 8, base_idx + 10, base_idx + 11,
            // Left
            base_idx + 12, base_idx + 13, base_idx + 14, base_idx + 12, base_idx + 14, base_idx + 15,
        ]);

        24
    }

    /// Append vertices/indices for a line to existing vectors.
    pub fn append_line(
        vertices: &mut Vec<ColorVertex>,
        indices: &mut Vec<u16>,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [f32; 4],
        thickness: f32,
        window_width: f32,
        window_height: f32,
    ) -> u32 {
        let base_idx = vertices.len() as u16;

        // Calculate direction in pixel space first
        let dx_px = x2 - x1;
        let dy_px = y2 - y1;
        let len_px = (dx_px * dx_px + dy_px * dy_px).sqrt();
        if len_px < 0.0001 {
            // Zero-length line - draw a small rect
            return Self::append_rect(vertices, indices, x1, y1, 1.0, 1.0, color, window_width, window_height);
        }

        // Calculate perpendicular unit vector in pixel space
        let half_thickness = thickness / 2.0;
        let nx_px = -dy_px / len_px * half_thickness;
        let ny_px = dx_px / len_px * half_thickness;

        // Convert the four corners to NDC
        let to_ndc_x = |px: f32| (px / window_width) * 2.0 - 1.0;
        let to_ndc_y = |py: f32| 1.0 - (py / window_height) * 2.0;

        vertices.extend_from_slice(&[
            ColorVertex { position: [to_ndc_x(x1 + nx_px), to_ndc_y(y1 + ny_px)], color },
            ColorVertex { position: [to_ndc_x(x1 - nx_px), to_ndc_y(y1 - ny_px)], color },
            ColorVertex { position: [to_ndc_x(x2 - nx_px), to_ndc_y(y2 - ny_px)], color },
            ColorVertex { position: [to_ndc_x(x2 + nx_px), to_ndc_y(y2 + ny_px)], color },
        ]);

        indices.extend_from_slice(&[
            base_idx, base_idx + 1, base_idx + 2,
            base_idx, base_idx + 2, base_idx + 3,
        ]);

        6
    }

    /// Append vertices/indices for a filled circle to existing vectors.
    pub fn append_circle(
        vertices: &mut Vec<ColorVertex>,
        indices: &mut Vec<u16>,
        cx: f32,
        cy: f32,
        radius: f32,
        color: [f32; 4],
        window_width: f32,
        window_height: f32,
    ) -> u32 {
        const SEGMENTS: usize = 16;
        let base_idx = vertices.len() as u16;

        let cx_ndc = (cx / window_width) * 2.0 - 1.0;
        let cy_ndc = 1.0 - (cy / window_height) * 2.0;
        let rx = (radius / window_width) * 2.0;
        let ry = (radius / window_height) * 2.0;

        // Center vertex
        vertices.push(ColorVertex { position: [cx_ndc, cy_ndc], color });

        // Circle vertices
        for i in 0..SEGMENTS {
            let angle = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
            vertices.push(ColorVertex {
                position: [cx_ndc + rx * angle.cos(), cy_ndc + ry * angle.sin()],
                color,
            });
        }

        // Triangle fan indices
        for i in 0..SEGMENTS {
            indices.push(base_idx);
            indices.push(base_idx + (i as u16) + 1);
            indices.push(base_idx + ((i + 1) % SEGMENTS) as u16 + 1);
        }

        (SEGMENTS * 3) as u32
    }

    /// Append vertices/indices for a stroked circle to existing vectors.
    pub fn append_stroke_circle(
        vertices: &mut Vec<ColorVertex>,
        indices: &mut Vec<u16>,
        cx: f32,
        cy: f32,
        radius: f32,
        color: [f32; 4],
        thickness: f32,
        window_width: f32,
        window_height: f32,
    ) -> u32 {
        const SEGMENTS: usize = 24;
        let base_idx = vertices.len() as u16;

        let cx_ndc = (cx / window_width) * 2.0 - 1.0;
        let cy_ndc = 1.0 - (cy / window_height) * 2.0;
        let rx_inner = ((radius - thickness / 2.0) / window_width) * 2.0;
        let ry_inner = ((radius - thickness / 2.0) / window_height) * 2.0;
        let rx_outer = ((radius + thickness / 2.0) / window_width) * 2.0;
        let ry_outer = ((radius + thickness / 2.0) / window_height) * 2.0;

        // Inner and outer ring vertices
        for i in 0..SEGMENTS {
            let angle = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            vertices.push(ColorVertex {
                position: [cx_ndc + rx_inner * cos_a, cy_ndc + ry_inner * sin_a],
                color,
            });
            vertices.push(ColorVertex {
                position: [cx_ndc + rx_outer * cos_a, cy_ndc + ry_outer * sin_a],
                color,
            });
        }

        // Quads between inner and outer ring
        for i in 0..SEGMENTS {
            let i0 = base_idx + (i * 2) as u16;
            let i1 = base_idx + (i * 2 + 1) as u16;
            let i2 = base_idx + ((i + 1) % SEGMENTS * 2) as u16;
            let i3 = base_idx + ((i + 1) % SEGMENTS * 2 + 1) as u16;
            indices.extend_from_slice(&[i0, i1, i3, i0, i3, i2]);
        }

        (SEGMENTS * 6) as u32
    }
}
