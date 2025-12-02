use std::collections::HashMap;
use std::sync::Arc;
use winit::window::Window;

use hvat_gpu::{GpuContext, ImageAdjustments, TexturePipeline, Texture, TransformUniform};
use wgpu_text::{BrushBuilder, TextBrush, glyph_brush::{Section, Text as GlyphText, ab_glyph::FontArc}};

use crate::{Element, Limits, Rectangle};
use crate::color_pipeline::ColorPipeline;

/// A draw command to be executed during rendering
#[derive(Debug, Clone)]
pub enum DrawCommand {
    FillRect {
        rect: Rectangle,
        color: Color,
    },
    StrokeRect {
        rect: Rectangle,
        color: Color,
        width: f32,
    },
    DrawText {
        text: String,
        position: crate::Point,
        color: Color,
        size: f32,
    },
    DrawImage {
        handle: crate::ImageHandle,
        rect: Rectangle,
        /// Pan offset in pixels
        pan: (f32, f32),
        /// Zoom level (1.0 = 100%)
        zoom: f32,
        /// Image adjustments (brightness, contrast, gamma, hue)
        adjustments: ImageAdjustments,
    },
}

/// The renderer abstracts away GPU rendering details from widgets.
///
/// It provides high-level drawing primitives that widgets can use.
pub struct Renderer {
    gpu_ctx: GpuContext,
    texture_pipeline: TexturePipeline,
    color_pipeline: ColorPipeline,
    text_brush: TextBrush<FontArc>,
    width: u32,
    height: u32,
    draw_commands: Vec<DrawCommand>,
    /// Cache for GPU textures, keyed by ImageHandle data pointer
    texture_cache: HashMap<usize, (Texture, wgpu::BindGroup)>,
}

impl Renderer {
    /// Create a new renderer for the given window.
    pub async fn new(window: Arc<Window>) -> Result<Self, String> {
        let gpu_ctx = GpuContext::new(window)
            .await
            .map_err(|e| format!("Failed to create GPU context: {:?}", e))?;

        let texture_pipeline = TexturePipeline::new(&gpu_ctx);

        let format = gpu_ctx.surface_config.format;
        let color_pipeline = ColorPipeline::new(&gpu_ctx.device, format);

        let width = gpu_ctx.width();
        let height = gpu_ctx.height();

        // Create text brush with default font
        use wgpu_text::glyph_brush::ab_glyph::FontArc;

        // On WASM, use embedded font. On native, use system fonts.
        let font = {
            #[cfg(target_arch = "wasm32")]
            {
                // For WASM, use embedded DejaVuSans font
                const FONT_DATA: &[u8] = include_bytes!("../../../assets/DejaVuSans.ttf");
                FontArc::try_from_slice(FONT_DATA)
                    .map_err(|e| format!("Failed to load embedded font: {:?}", e))?
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                // Try to load a system font
                if let Ok(font_data) = std::fs::read("/usr/share/fonts/truetype/DejaVuSans.ttf") {
                    FontArc::try_from_vec(font_data)
                        .map_err(|e| format!("Failed to parse system font: {:?}", e))?
                } else if let Ok(font_data) = std::fs::read("/usr/share/fonts/truetype/Carlito-Regular.ttf") {
                    // Alternative Linux font
                    FontArc::try_from_vec(font_data)
                        .map_err(|e| format!("Failed to parse system font: {:?}", e))?
                } else if let Ok(font_data) = std::fs::read("/System/Library/Fonts/Helvetica.ttc") {
                    // macOS fallback
                    FontArc::try_from_vec(font_data)
                        .map_err(|e| format!("Failed to parse system font: {:?}", e))?
                } else if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\arial.ttf") {
                    // Windows fallback
                    FontArc::try_from_vec(font_data)
                        .map_err(|e| format!("Failed to parse system font: {:?}", e))?
                } else {
                    return Err("No system fonts found. Please install DejaVu fonts or specify a font path.".to_string());
                }
            }
        };

        let text_brush = BrushBuilder::using_font(font)
            .build(&gpu_ctx.device, width, height, format);

        Ok(Self {
            gpu_ctx,
            texture_pipeline,
            color_pipeline,
            text_brush,
            width,
            height,
            draw_commands: Vec::new(),
            texture_cache: HashMap::new(),
        })
    }

    /// Resize the renderer.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.gpu_ctx.resize(width, height);
        // Resize text brush viewport
        self.text_brush.resize_view(width as f32, height as f32, &self.gpu_ctx.queue);
    }

    /// Get the current surface size.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Render an element tree.
    pub fn render<Message>(&mut self, element: Element<Message>) {
        // Clear draw commands from last frame
        self.draw_commands.clear();

        // Layout the element
        let limits = Limits::new(self.width as f32, self.height as f32);
        let layout = element.widget().layout(&limits);

        // Collect draw commands from widgets
        element.widget().draw(self, &layout);

        // Get the next frame
        let frame = match self.gpu_ctx.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                log::error!("Failed to get frame: {:?}", e);
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Queue text sections before rendering
        let text_sections: Vec<Section> = self.draw_commands
            .iter()
            .filter_map(|cmd| {
                if let DrawCommand::DrawText { text, position, color, size } = cmd {
                    Some(Section::default()
                        .add_text(GlyphText::new(text)
                            .with_scale(*size)
                            .with_color([color.r, color.g, color.b, color.a]))
                        .with_screen_position((position.x, position.y)))
                } else {
                    None
                }
            })
            .collect();

        // Queue text for rendering
        let text_section_refs: Vec<&Section> = text_sections.iter().collect();
        if !text_section_refs.is_empty() {
            if let Err(e) = self.text_brush.queue(&self.gpu_ctx.device, &self.gpu_ctx.queue, text_section_refs) {
                log::error!("Failed to queue text: {:?}", e);
            }
        }

        // Pre-process image commands: create/cache textures
        let image_commands: Vec<_> = self.draw_commands
            .iter()
            .filter_map(|cmd| {
                if let DrawCommand::DrawImage { handle, rect, pan, zoom, adjustments } = cmd {
                    Some((handle.clone(), *rect, *pan, *zoom, *adjustments))
                } else {
                    None
                }
            })
            .collect();

        // Create/cache GPU textures for images
        for (handle, _, _, _, _) in &image_commands {
            let cache_key = handle.data().as_ptr() as usize;
            if !self.texture_cache.contains_key(&cache_key) {
                // Create GPU texture from ImageHandle
                match Texture::from_rgba8(
                    &self.gpu_ctx,
                    handle.data(),
                    handle.width(),
                    handle.height(),
                ) {
                    Ok(texture) => {
                        let bind_group = self.texture_pipeline.create_texture_bind_group(&self.gpu_ctx, &texture);
                        self.texture_cache.insert(cache_key, (texture, bind_group));
                        log::debug!("Created GPU texture for image ({}x{})", handle.width(), handle.height());
                    }
                    Err(e) => {
                        log::error!("Failed to create texture: {:?}", e);
                    }
                }
            }
        }

        // Create command encoder
        let mut encoder = self
            .gpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // First pass: Clear screen
        {
            let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        // Second pass: Render images using texture pipeline
        for (handle, rect, pan, zoom, adjustments) in &image_commands {
            let cache_key = handle.data().as_ptr() as usize;
            if let Some((_texture, bind_group)) = self.texture_cache.get(&cache_key) {
                // Calculate transform for pan/zoom within the widget bounds
                // Convert widget rect to NDC coordinates
                let ndc_x = (rect.x / self.width as f32) * 2.0 - 1.0;
                let ndc_y = 1.0 - (rect.y / self.height as f32) * 2.0;
                let ndc_w = (rect.width / self.width as f32) * 2.0;
                let ndc_h = (rect.height / self.height as f32) * 2.0;

                // Calculate image aspect ratio and fit it within the rect
                let img_aspect = handle.width() as f32 / handle.height() as f32;
                let rect_aspect = rect.width / rect.height;

                let (scale_x, scale_y) = if img_aspect > rect_aspect {
                    // Image is wider - fit to width
                    (ndc_w / 2.0, (ndc_w / 2.0) / img_aspect)
                } else {
                    // Image is taller - fit to height
                    (ndc_h / 2.0 * img_aspect, ndc_h / 2.0)
                };

                // Apply zoom
                let scale_x = scale_x * zoom;
                let scale_y = scale_y * zoom;

                // Apply pan (convert pixel pan to NDC)
                let pan_ndc_x = (pan.0 / self.width as f32) * 2.0;
                let pan_ndc_y = -(pan.1 / self.height as f32) * 2.0;

                // Center of the widget in NDC
                let center_x = ndc_x + ndc_w / 2.0 + pan_ndc_x;
                let center_y = ndc_y - ndc_h / 2.0 + pan_ndc_y;

                // Create transform matrix
                let transform = TransformUniform {
                    matrix: [
                        [scale_x, 0.0, 0.0, 0.0],
                        [0.0, scale_y, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [center_x, center_y, 0.0, 1.0],
                    ],
                };
                self.texture_pipeline.update_transform(&self.gpu_ctx, transform);
                self.texture_pipeline.update_adjustments(&self.gpu_ctx, *adjustments);

                // Render image
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Image Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load, // Don't clear, preserve previous content
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    render_pass.set_pipeline(&self.texture_pipeline.render_pipeline);
                    render_pass.set_bind_group(0, &self.texture_pipeline.uniform_bind_group, &[]);
                    render_pass.set_bind_group(1, bind_group, &[]);
                    render_pass.set_vertex_buffer(0, self.texture_pipeline.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(self.texture_pipeline.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..self.texture_pipeline.num_indices, 0, 0..1);
                }
            }
        }

        // Third pass: Render UI elements (rects, text) on top
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve image layer
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.color_pipeline.render_pipeline);

            // Execute UI draw commands (not images)
            for command in &self.draw_commands {
                match command {
                    DrawCommand::FillRect { rect, color } => {
                        let (vertex_buffer, index_buffer, num_indices) =
                            ColorPipeline::create_rect_vertices(
                                &self.gpu_ctx.device,
                                rect.x,
                                rect.y,
                                rect.width,
                                rect.height,
                                [color.r, color.g, color.b, color.a],
                                self.width as f32,
                                self.height as f32,
                            );

                        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        render_pass.draw_indexed(0..num_indices, 0, 0..1);
                    }
                    DrawCommand::StrokeRect { rect, color, width } => {
                        let (vertex_buffer, index_buffer, num_indices) =
                            ColorPipeline::create_stroke_rect_vertices(
                                &self.gpu_ctx.device,
                                rect.x,
                                rect.y,
                                rect.width,
                                rect.height,
                                [color.r, color.g, color.b, color.a],
                                *width,
                                self.width as f32,
                                self.height as f32,
                            );

                        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        render_pass.draw_indexed(0..num_indices, 0, 0..1);
                    }
                    DrawCommand::DrawText { .. } | DrawCommand::DrawImage { .. } => {
                        // Text is handled separately, images already rendered
                    }
                }
            }

            // Draw text on top of everything
            self.text_brush.draw(&mut render_pass);
        }

        // Submit commands
        self.gpu_ctx.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    /// Get a reference to the GPU context.
    pub fn gpu_context(&self) -> &GpuContext {
        &self.gpu_ctx
    }

    /// Get a reference to the texture pipeline.
    pub fn texture_pipeline(&self) -> &TexturePipeline {
        &self.texture_pipeline
    }

    // Drawing primitives that widgets can use

    /// Draw a filled rectangle.
    pub fn fill_rect(&mut self, rect: Rectangle, color: Color) {
        self.draw_commands.push(DrawCommand::FillRect { rect, color });
    }

    /// Draw a rectangle outline.
    pub fn stroke_rect(&mut self, rect: Rectangle, color: Color, width: f32) {
        self.draw_commands.push(DrawCommand::StrokeRect { rect, color, width });
    }

    /// Draw text.
    pub fn draw_text(
        &mut self,
        text: &str,
        position: crate::Point,
        color: Color,
        size: f32,
    ) {
        self.draw_commands.push(DrawCommand::DrawText {
            text: text.to_string(),
            position,
            color,
            size,
        });
    }

    /// Draw a texture.
    pub fn draw_texture(
        &mut self,
        texture: &Texture,
        rect: Rectangle,
    ) {
        // TODO: Use the texture pipeline
        let _ = (texture, rect);
    }

    /// Draw an image from an ImageHandle.
    pub fn draw_image(
        &mut self,
        handle: &crate::ImageHandle,
        rect: Rectangle,
    ) {
        self.draw_commands.push(DrawCommand::DrawImage {
            handle: handle.clone(),
            rect,
            pan: (0.0, 0.0),
            zoom: 1.0,
            adjustments: ImageAdjustments::new(),
        });
    }

    /// Draw an image with pan and zoom transform.
    pub fn draw_image_transformed(
        &mut self,
        handle: &crate::ImageHandle,
        rect: Rectangle,
        pan: (f32, f32),
        zoom: f32,
    ) {
        self.draw_commands.push(DrawCommand::DrawImage {
            handle: handle.clone(),
            rect,
            pan,
            zoom,
            adjustments: ImageAdjustments::new(),
        });
    }

    /// Draw an image with pan, zoom, and image adjustments.
    pub fn draw_image_with_adjustments(
        &mut self,
        handle: &crate::ImageHandle,
        rect: Rectangle,
        pan: (f32, f32),
        zoom: f32,
        adjustments: ImageAdjustments,
    ) {
        self.draw_commands.push(DrawCommand::DrawImage {
            handle: handle.clone(),
            rect,
            pan,
            zoom,
            adjustments,
        });
    }
}

/// RGBA color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}
