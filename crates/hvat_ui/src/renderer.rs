use std::sync::Arc;
use winit::window::Window;

use hvat_gpu::{GpuContext, TexturePipeline, Texture};
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
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("Failed to get frame: {:?}", e).into());
                #[cfg(not(target_arch = "wasm32"))]
                eprintln!("Failed to get frame: {:?}", e);
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
                eprintln!("Failed to queue text: {:?}", e);
            }
        }

        // Create command encoder
        let mut encoder = self
            .gpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Clear the screen and execute draw commands
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
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

            render_pass.set_pipeline(&self.color_pipeline.render_pipeline);

            // Execute all draw commands
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
                    DrawCommand::DrawText { text, position, color, size } => {
                        // Text is queued separately - skip in this loop
                        let _ = (text, position, color, size);
                    }
                    DrawCommand::DrawImage { .. } => {
                        // TODO: Implement image rendering
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
