//! Image viewer widget with pan and zoom

use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer, TextureId};
use crate::state::{FitMode, ImageViewerState};
use crate::widget::Widget;
use hvat_gpu::TransformUniform;
use std::marker::PhantomData;

/// Zoom factor per scroll notch
const ZOOM_FACTOR: f32 = 1.25;

/// Pan speed for keyboard navigation (in clip space units)
const PAN_SPEED: f32 = 0.1;

/// Control button size
const CONTROL_BUTTON_SIZE: f32 = 28.0;

/// Control panel padding
const CONTROL_PADDING: f32 = 8.0;

/// Control button spacing
const CONTROL_SPACING: f32 = 4.0;

/// An image viewer widget with pan and zoom capabilities
pub struct ImageViewer<M> {
    /// Texture ID (registered with renderer)
    texture_id: Option<TextureId>,
    /// Texture width
    texture_width: u32,
    /// Texture height
    texture_height: u32,
    /// Current state
    state: ImageViewerState,
    /// Change handler
    on_change: Option<Box<dyn Fn(ImageViewerState) -> M>>,
    /// Enable panning
    pannable: bool,
    /// Enable zooming
    zoomable: bool,
    /// Show built-in controls
    show_controls: bool,
    /// Width
    width: Length,
    /// Height
    height: Length,
    /// Phantom data for message type
    _phantom: PhantomData<M>,
}

impl<M> ImageViewer<M> {
    /// Create a new image viewer with the given texture ID and dimensions
    pub fn new(texture_id: TextureId, width: u32, height: u32) -> Self {
        Self {
            texture_id: Some(texture_id),
            texture_width: width,
            texture_height: height,
            state: ImageViewerState::default(),
            on_change: None,
            pannable: true,
            zoomable: true,
            show_controls: true,
            width: Length::fill(),
            height: Length::fill(),
            _phantom: PhantomData,
        }
    }

    /// Create an empty image viewer (no texture)
    pub fn empty() -> Self {
        Self {
            texture_id: None,
            texture_width: 0,
            texture_height: 0,
            state: ImageViewerState::default(),
            on_change: None,
            pannable: true,
            zoomable: true,
            show_controls: true,
            width: Length::fill(),
            height: Length::fill(),
            _phantom: PhantomData,
        }
    }

    /// Set the texture
    pub fn texture(mut self, texture_id: TextureId, width: u32, height: u32) -> Self {
        self.texture_id = Some(texture_id);
        self.texture_width = width;
        self.texture_height = height;
        self
    }

    /// Set the viewer state
    pub fn state(mut self, state: &ImageViewerState) -> Self {
        self.state = state.clone();
        self
    }

    /// Set the change handler
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(ImageViewerState) -> M + 'static,
    {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Enable/disable panning
    pub fn pannable(mut self, enabled: bool) -> Self {
        self.pannable = enabled;
        self
    }

    /// Enable/disable zooming
    pub fn zoomable(mut self, enabled: bool) -> Self {
        self.zoomable = enabled;
        self
    }

    /// Show/hide built-in controls
    pub fn show_controls(mut self, show: bool) -> Self {
        self.show_controls = show;
        self
    }

    /// Set width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Set height
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Calculate the transform uniform based on current state and bounds
    fn calculate_transform(&self, bounds: &Bounds) -> TransformUniform {
        if self.texture_width == 0 || self.texture_height == 0 {
            return TransformUniform::new();
        }

        let image_aspect = self.texture_width as f32 / self.texture_height as f32;
        let view_aspect = bounds.width / bounds.height;

        // Calculate aspect ratio correction
        // The quad is a square (-1 to 1), we need to scale it to match image aspect
        // relative to the view aspect
        let (aspect_scale_x, aspect_scale_y) = if image_aspect > view_aspect {
            // Image is wider than view - fit to width, scale down height
            (1.0, view_aspect / image_aspect)
        } else {
            // Image is taller than view - fit to height, scale down width
            (image_aspect / view_aspect, 1.0)
        };

        // Calculate zoom factor based on fit mode
        let zoom = match self.state.fit_mode {
            FitMode::FitToView => 1.0,
            FitMode::OneToOne => {
                // 1:1 pixel mapping - calculate zoom to show actual pixels
                // At zoom=1, the image fills the view. For 1:1, we need to
                // scale so that 1 image pixel = 1 screen pixel
                if image_aspect > view_aspect {
                    bounds.width / self.texture_width as f32
                } else {
                    bounds.height / self.texture_height as f32
                }
            }
            FitMode::Manual => self.state.zoom,
        };

        // Apply zoom to aspect-corrected scale
        let scale_x = aspect_scale_x * zoom;
        let scale_y = aspect_scale_y * zoom;

        TransformUniform::from_transform_xy(self.state.pan.0, self.state.pan.1, scale_x, scale_y)
    }

    /// Convert screen position to clip space relative to widget bounds
    fn screen_to_clip(&self, x: f32, y: f32, bounds: &Bounds) -> (f32, f32) {
        let rel_x = (x - bounds.x) / bounds.width;
        let rel_y = (y - bounds.y) / bounds.height;
        let clip_x = rel_x * 2.0 - 1.0;
        let clip_y = -(rel_y * 2.0 - 1.0); // Flip Y
        (clip_x, clip_y)
    }

    /// Get the bounds for a control button
    fn control_button_bounds(&self, index: usize, widget_bounds: &Bounds) -> Bounds {
        let x = widget_bounds.x
            + CONTROL_PADDING
            + (CONTROL_BUTTON_SIZE + CONTROL_SPACING) * index as f32;
        let y = widget_bounds.y + CONTROL_PADDING;
        Bounds::new(x, y, CONTROL_BUTTON_SIZE, CONTROL_BUTTON_SIZE)
    }

    /// Check which control button is at a position (if any)
    fn control_button_at(&self, x: f32, y: f32, bounds: &Bounds) -> Option<ControlButton> {
        if !self.show_controls {
            return None;
        }

        for (i, button) in [
            ControlButton::ZoomIn,
            ControlButton::ZoomOut,
            ControlButton::OneToOne,
            ControlButton::FitToView,
        ]
        .iter()
        .enumerate()
        {
            let btn_bounds = self.control_button_bounds(i, bounds);
            if btn_bounds.contains(x, y) {
                return Some(*button);
            }
        }
        None
    }

    /// Emit a state change if handler is set
    fn emit_change(&self) -> Option<M> {
        self.on_change.as_ref().map(|f| f(self.state.clone()))
    }
}

#[derive(Debug, Clone, Copy)]
enum ControlButton {
    ZoomIn,
    ZoomOut,
    OneToOne,
    FitToView,
}

impl<M: 'static> Widget<M> for ImageViewer<M> {
    fn has_active_drag(&self) -> bool {
        self.state.dragging
    }

    fn layout(&mut self, available: Size) -> Size {
        Size::new(
            self.width.resolve(available.width, available.width),
            self.height.resolve(available.height, available.height),
        )
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        // Draw background
        renderer.fill_rect(bounds, Color::rgb(0.1, 0.1, 0.12));

        // Draw the texture if we have one
        if let Some(texture_id) = self.texture_id {
            let transform = self.calculate_transform(&bounds);
            renderer.texture(texture_id, bounds, transform);
        } else {
            // Draw a placeholder if no texture
            let placeholder_color = Color::rgba(0.2, 0.3, 0.4, 0.5);
            let img_bounds = Bounds::new(
                bounds.x + bounds.width * 0.1,
                bounds.y + bounds.height * 0.1,
                bounds.width * 0.8,
                bounds.height * 0.8,
            );
            renderer.fill_rect(img_bounds, placeholder_color);

            // "No Image" text
            renderer.text(
                "No Image",
                bounds.x + bounds.width / 2.0 - 30.0,
                bounds.y + bounds.height / 2.0 - 7.0,
                14.0,
                Color::TEXT_SECONDARY,
            );
        }

        // Switch to overlay layer for controls (rendered after textures)
        renderer.begin_overlay();

        // Draw zoom info
        let zoom_text = format!("{:.0}%", self.state.zoom * 100.0);
        renderer.text(
            &zoom_text,
            bounds.x + bounds.width - 60.0,
            bounds.y + bounds.height - 24.0,
            12.0,
            Color::TEXT_SECONDARY,
        );

        // Draw controls if enabled
        if self.show_controls {
            let controls = [
                ("+", "Zoom In"),
                ("-", "Zoom Out"),
                ("1:1", "1:1"),
                ("Fit", "Fit"),
            ];

            for (i, (label, _tooltip)) in controls.iter().enumerate() {
                let btn_bounds = self.control_button_bounds(i, &bounds);

                // Button background
                renderer.fill_rect(btn_bounds, Color::BUTTON_BG);
                renderer.stroke_rect(btn_bounds, Color::BORDER, 1.0);

                // Button label
                let label_x = btn_bounds.x + (btn_bounds.width - label.len() as f32 * 7.0) / 2.0;
                let label_y = btn_bounds.y + (btn_bounds.height - 12.0) / 2.0;
                renderer.text(*label, label_x, label_y, 11.0, Color::TEXT_PRIMARY);
            }
        }

        // Draw border around the viewer
        renderer.stroke_rect(bounds, Color::BORDER, 1.0);

        renderer.end_overlay();
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        match event {
            // Handle control button clicks
            Event::MousePress {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if !bounds.contains(position.0, position.1) {
                    return None;
                }

                // Check control buttons first
                if let Some(btn) = self.control_button_at(position.0, position.1, &bounds) {
                    match btn {
                        ControlButton::ZoomIn => {
                            self.state.zoom_in();
                            return self.emit_change();
                        }
                        ControlButton::ZoomOut => {
                            self.state.zoom_out();
                            return self.emit_change();
                        }
                        ControlButton::OneToOne => {
                            self.state.set_one_to_one();
                            return self.emit_change();
                        }
                        ControlButton::FitToView => {
                            self.state.set_fit_to_view();
                            return self.emit_change();
                        }
                    }
                }

                // Start panning
                if self.pannable {
                    self.state.dragging = true;
                    self.state.last_drag_pos = Some(*position);
                    // Emit change to persist dragging state
                    return self.emit_change();
                }
                None
            }

            Event::MouseRelease {
                button: MouseButton::Left,
                ..
            } => {
                if self.state.dragging {
                    self.state.dragging = false;
                    self.state.last_drag_pos = None;
                    // Emit change to persist state
                    return self.emit_change();
                }
                None
            }

            Event::MouseMove { position, .. } => {
                if self.state.dragging && self.pannable {
                    if let Some((last_x, last_y)) = self.state.last_drag_pos {
                        let delta_x = position.0 - last_x;
                        let delta_y = position.1 - last_y;

                        // Convert screen delta to clip space delta
                        let clip_delta_x = delta_x / (bounds.width / 2.0);
                        let clip_delta_y = -delta_y / (bounds.height / 2.0);

                        self.state.pan_by(clip_delta_x, clip_delta_y);
                        self.state.last_drag_pos = Some(*position);

                        return self.emit_change();
                    }
                }
                None
            }

            Event::MouseScroll { delta, position, .. } => {
                if !bounds.contains(position.0, position.1) || !self.zoomable {
                    return None;
                }

                // Skip if on control buttons
                if self.control_button_at(position.0, position.1, &bounds).is_some() {
                    return None;
                }

                let (clip_x, clip_y) = self.screen_to_clip(position.0, position.1, &bounds);

                let zoom_factor = if delta.1 > 0.0 {
                    ZOOM_FACTOR
                } else {
                    1.0 / ZOOM_FACTOR
                };

                self.state.zoom_at(clip_x, clip_y, zoom_factor);
                self.emit_change()
            }

            Event::KeyPress { key, .. } => {
                match key {
                    KeyCode::Plus | KeyCode::Equal => {
                        if self.zoomable {
                            self.state.zoom_in();
                            return self.emit_change();
                        }
                    }
                    KeyCode::Minus => {
                        if self.zoomable {
                            self.state.zoom_out();
                            return self.emit_change();
                        }
                    }
                    KeyCode::Key0 => {
                        self.state.set_one_to_one();
                        return self.emit_change();
                    }
                    KeyCode::F => {
                        self.state.set_fit_to_view();
                        return self.emit_change();
                    }
                    KeyCode::Up => {
                        if self.pannable {
                            self.state.pan_by(0.0, PAN_SPEED);
                            return self.emit_change();
                        }
                    }
                    KeyCode::Down => {
                        if self.pannable {
                            self.state.pan_by(0.0, -PAN_SPEED);
                            return self.emit_change();
                        }
                    }
                    KeyCode::Left => {
                        if self.pannable {
                            self.state.pan_by(PAN_SPEED, 0.0);
                            return self.emit_change();
                        }
                    }
                    KeyCode::Right => {
                        if self.pannable {
                            self.state.pan_by(-PAN_SPEED, 0.0);
                            return self.emit_change();
                        }
                    }
                    _ => {}
                }
                None
            }

            _ => None,
        }
    }
}
