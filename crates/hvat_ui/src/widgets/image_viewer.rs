//! Image viewer widget with pan and zoom

use crate::callback::Callback;
use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer, TextureId};
use crate::state::{FitMode, ImageViewerState, InteractionMode, PanDragData, PanDragExt, PointerState};
use crate::widget::Widget;
use hvat_gpu::{ImageAdjustments, TransformUniform};
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

/// The kind of pointer event on the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerEventKind {
    /// Single click (press without drag)
    Click,
    /// Start of a drag operation
    DragStart,
    /// Continuation of a drag operation
    DragMove,
    /// End of a drag operation
    DragEnd,
}

/// Pointer event with image coordinates.
///
/// Emitted when the user interacts with the image in annotation mode.
/// Includes the updated viewer state so the app can persist pointer_state changes.
#[derive(Debug, Clone)]
pub struct ImagePointerEvent {
    /// X coordinate in image space (0 to image_width)
    pub image_x: f32,
    /// Y coordinate in image space (0 to image_height)
    pub image_y: f32,
    /// X coordinate in screen space
    pub screen_x: f32,
    /// Y coordinate in screen space
    pub screen_y: f32,
    /// The kind of pointer event
    pub kind: PointerEventKind,
    /// Updated viewer state (app should use this to persist pointer_state)
    pub viewer_state: ImageViewerState,
}

/// Annotation overlay to draw on the image
#[derive(Debug, Clone)]
pub struct AnnotationOverlay {
    /// Type of shape
    pub shape: OverlayShape,
    /// Color (RGBA)
    pub color: [f32; 4],
    /// Line width
    pub line_width: f32,
    /// Whether this annotation is selected
    pub selected: bool,
}

/// Shape types for annotation overlays
#[derive(Debug, Clone)]
pub enum OverlayShape {
    /// Bounding box (x, y, width, height) in image coordinates
    BoundingBox { x: f32, y: f32, width: f32, height: f32 },
    /// Point marker (x, y) in image coordinates
    Point { x: f32, y: f32 },
    /// Polygon (vertices) in image coordinates
    Polygon { vertices: Vec<(f32, f32)>, closed: bool },
}

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
    /// Image adjustments (brightness, contrast, gamma, hue) - applied on GPU
    adjustments: ImageAdjustments,
    /// Change handler
    on_change: Callback<ImageViewerState, M>,
    /// Pointer event handler for annotation tools
    on_pointer: Callback<ImagePointerEvent, M>,
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
    /// Annotation overlays to draw
    overlays: Vec<AnnotationOverlay>,
    /// Interaction mode (View or Annotate)
    interaction_mode: InteractionMode,
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
            adjustments: ImageAdjustments::default(),
            on_change: Callback::none(),
            on_pointer: Callback::none(),
            pannable: true,
            zoomable: true,
            show_controls: true,
            width: Length::fill(),
            height: Length::fill(),
            overlays: Vec::new(),
            interaction_mode: InteractionMode::default(),
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
            adjustments: ImageAdjustments::default(),
            on_change: Callback::none(),
            on_pointer: Callback::none(),
            pannable: true,
            zoomable: true,
            show_controls: true,
            width: Length::fill(),
            height: Length::fill(),
            overlays: Vec::new(),
            interaction_mode: InteractionMode::default(),
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
        self.on_change = Callback::new(handler);
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

    /// Set the pointer event handler for annotation tools
    pub fn on_pointer<F>(mut self, handler: F) -> Self
    where
        F: Fn(ImagePointerEvent) -> M + 'static,
    {
        self.on_pointer = Callback::new(handler);
        self
    }

    /// Set the interaction mode (View or Annotate)
    pub fn interaction_mode(mut self, mode: InteractionMode) -> Self {
        self.interaction_mode = mode;
        self
    }

    /// Set annotation overlays to draw
    pub fn overlays(mut self, overlays: Vec<AnnotationOverlay>) -> Self {
        self.overlays = overlays;
        self
    }

    /// Set image adjustments (brightness, contrast, gamma, hue shift)
    ///
    /// These adjustments are applied on the GPU for real-time performance.
    pub fn adjustments(mut self, adjustments: ImageAdjustments) -> Self {
        self.adjustments = adjustments;
        self
    }

    /// Convert screen coordinates to image coordinates
    pub fn screen_to_image(&self, screen_x: f32, screen_y: f32, bounds: &Bounds) -> (f32, f32) {
        if self.texture_width == 0 || self.texture_height == 0 {
            return (0.0, 0.0);
        }

        // Get clip space coordinates
        let (clip_x, clip_y) = self.screen_to_clip(screen_x, screen_y, bounds);

        // Get zoom (1.0 = 1:1 pixel ratio)
        let zoom = self.calculate_zoom_for_mode(bounds);

        // Calculate scale (same as in calculate_transform)
        let scale_x = (self.texture_width as f32 / bounds.width) * zoom;
        let scale_y = (self.texture_height as f32 / bounds.height) * zoom;

        // Reverse the transform: clip_space = (image_uv * 2 - 1) * scale + pan
        // So: image_uv = ((clip_space - pan) / scale + 1) / 2
        let uv_x = ((clip_x - self.state.pan.0) / scale_x + 1.0) / 2.0;
        let uv_y = ((clip_y - self.state.pan.1) / scale_y + 1.0) / 2.0;

        // Convert UV to image coordinates
        // Note: UV y is flipped relative to image coordinates
        let image_x = uv_x * self.texture_width as f32;
        let image_y = (1.0 - uv_y) * self.texture_height as f32;

        (image_x, image_y)
    }

    /// Convert image coordinates to screen coordinates
    pub fn image_to_screen(&self, image_x: f32, image_y: f32, bounds: &Bounds) -> (f32, f32) {
        if self.texture_width == 0 || self.texture_height == 0 {
            return (bounds.x, bounds.y);
        }

        // Convert image coords to UV (0-1)
        let uv_x = image_x / self.texture_width as f32;
        let uv_y = 1.0 - (image_y / self.texture_height as f32); // Flip Y

        // Get zoom (1.0 = 1:1 pixel ratio)
        let zoom = self.calculate_zoom_for_mode(bounds);

        // Calculate scale (same as in calculate_transform)
        let scale_x = (self.texture_width as f32 / bounds.width) * zoom;
        let scale_y = (self.texture_height as f32 / bounds.height) * zoom;

        // Transform UV to clip space
        let clip_x = (uv_x * 2.0 - 1.0) * scale_x + self.state.pan.0;
        let clip_y = (uv_y * 2.0 - 1.0) * scale_y + self.state.pan.1;

        // Convert clip space to screen coordinates
        let rel_x = (clip_x + 1.0) / 2.0;
        let rel_y = (-clip_y + 1.0) / 2.0; // Flip Y back

        let screen_x = bounds.x + rel_x * bounds.width;
        let screen_y = bounds.y + rel_y * bounds.height;

        (screen_x, screen_y)
    }

    /// Calculate the transform uniform based on current state and bounds
    ///
    /// The transform maps the image quad to clip space (-1 to 1).
    /// Zoom semantics: zoom = screen_pixels_per_image_pixel
    /// - zoom = 1.0 (100%) means 1:1 pixel ratio
    /// - zoom = 2.0 (200%) means image appears 2x larger
    fn calculate_transform(&self, bounds: &Bounds) -> TransformUniform {
        if self.texture_width == 0 || self.texture_height == 0 {
            return TransformUniform::new();
        }

        // Get the effective zoom for the current mode
        let zoom = self.calculate_zoom_for_mode(bounds);

        // Calculate the scale that would give 1:1 pixel ratio in clip space
        // At 1:1: image_pixels * scale = clip_space_size (which is 2.0 for -1 to 1)
        // So: scale_1to1 = 2.0 / view_pixels (since we want 1 image pixel = 1 screen pixel)
        // But we also need to account for the image size relative to view
        //
        // The quad goes from -1 to 1 (size 2 in clip space)
        // At 1:1 zoom: tex_width pixels should map to tex_width screen pixels
        // In clip space: tex_width screen pixels = tex_width / view_width * 2.0 clip units
        let base_scale_x = (self.texture_width as f32 / bounds.width) * zoom;
        let base_scale_y = (self.texture_height as f32 / bounds.height) * zoom;

        TransformUniform::from_transform_xy(self.state.pan.0, self.state.pan.1, base_scale_x, base_scale_y)
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

    /// Calculate the appropriate zoom value based on fit mode
    /// Returns zoom where 1.0 = 1:1 pixel ratio (100%)
    fn calculate_zoom_for_mode(&self, bounds: &Bounds) -> f32 {
        match self.state.fit_mode {
            FitMode::FitToView => ImageViewerState::calculate_fit_zoom(
                bounds.width,
                bounds.height,
                self.texture_width,
                self.texture_height,
            ),
            FitMode::OneToOne => 1.0, // 1:1 is zoom = 1.0 (100%)
            FitMode::Manual => self.state.zoom,
        }
    }

    /// Emit a state change with zoom updated based on current fit mode
    fn emit_change_with_bounds(&mut self, bounds: &Bounds) -> Option<M> {
        // Cache the view and texture sizes so external code can calculate fit zoom
        self.state.cached_view_size = Some((bounds.width, bounds.height));
        self.state.cached_texture_size = Some((self.texture_width, self.texture_height));

        // Calculate zoom based on fit_mode
        self.state.zoom = self.calculate_zoom_for_mode(bounds);

        // Convert temporary modes (FitToView, OneToOne) to Manual after applying
        // This makes Fit/1:1 a one-time action rather than a persistent mode
        if self.state.fit_mode != FitMode::Manual {
            self.state.fit_mode = FitMode::Manual;
        }

        self.on_change.call(self.state.clone())
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
        self.state.drag.is_dragging()
    }

    fn layout(&mut self, available: Size) -> Size {
        // Use texture dimensions as content size fallback for Shrink mode
        let content_width = self.texture_width as f32;
        let content_height = self.texture_height as f32;
        let size = Size::new(
            self.width.resolve(available.width, content_width),
            self.height.resolve(available.height, content_height),
        );

        // Sync state with layout bounds - this resolves any pending FitToView mode
        // and ensures the zoom value is correct before draw() and before any events.
        self.state.sync_with_bounds(size.width, size.height, self.texture_width, self.texture_height);

        size
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        // Draw background
        renderer.fill_rect(bounds, Color::rgb(0.1, 0.1, 0.12));

        // Draw the texture if we have one
        if let Some(texture_id) = self.texture_id {
            let transform = self.calculate_transform(&bounds);
            renderer.texture_with_adjustments(texture_id, bounds, transform, self.adjustments);
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

        // Clip all overlay drawing to widget bounds
        renderer.push_clip(bounds);

        // Draw annotation overlays
        for overlay in &self.overlays {
            let color = Color::rgba(overlay.color[0], overlay.color[1], overlay.color[2], overlay.color[3]);
            let selected_color = Color::rgba(1.0, 1.0, 0.0, 1.0); // Yellow for selected

            match &overlay.shape {
                OverlayShape::BoundingBox { x, y, width, height } => {
                    // Convert image coordinates to screen coordinates
                    let (screen_x1, screen_y1) = self.image_to_screen(*x, *y, &bounds);
                    let (screen_x2, screen_y2) = self.image_to_screen(*x + *width, *y + *height, &bounds);

                    let box_bounds = Bounds::new(
                        screen_x1.min(screen_x2),
                        screen_y1.min(screen_y2),
                        (screen_x2 - screen_x1).abs(),
                        (screen_y2 - screen_y1).abs(),
                    );

                    // Draw filled rectangle with transparency
                    let fill_color = Color::rgba(color.r, color.g, color.b, 0.2);
                    renderer.fill_rect(box_bounds, fill_color);

                    // Draw border
                    let border_color = if overlay.selected { selected_color } else { color };
                    renderer.stroke_rect(box_bounds, border_color, overlay.line_width);

                    // Draw corner handles if selected
                    if overlay.selected {
                        let handle_size = 6.0;
                        let corners = [
                            (screen_x1, screen_y1),
                            (screen_x2, screen_y1),
                            (screen_x2, screen_y2),
                            (screen_x1, screen_y2),
                        ];
                        for (cx, cy) in corners {
                            let handle_bounds = Bounds::new(
                                cx - handle_size / 2.0,
                                cy - handle_size / 2.0,
                                handle_size,
                                handle_size,
                            );
                            renderer.fill_rect(handle_bounds, Color::WHITE);
                            renderer.stroke_rect(handle_bounds, selected_color, 1.0);
                        }
                    }
                }
                OverlayShape::Point { x, y } => {
                    let (screen_x, screen_y) = self.image_to_screen(*x, *y, &bounds);
                    let radius = 6.0;

                    // Draw a circle approximation (diamond shape)
                    let point_color = if overlay.selected { selected_color } else { color };
                    let point_bounds = Bounds::new(
                        screen_x - radius,
                        screen_y - radius,
                        radius * 2.0,
                        radius * 2.0,
                    );
                    renderer.fill_rect(point_bounds, point_color);
                    renderer.stroke_rect(point_bounds, Color::WHITE, 1.0);
                }
                OverlayShape::Polygon { vertices, closed } => {
                    if vertices.is_empty() {
                        continue;
                    }

                    let line_color = if overlay.selected { selected_color } else { color };
                    let screen_verts: Vec<(f32, f32)> = vertices
                        .iter()
                        .map(|(x, y)| self.image_to_screen(*x, *y, &bounds))
                        .collect();

                    // Draw edges (need at least 2 vertices)
                    for i in 0..screen_verts.len().saturating_sub(1) {
                        let (x1, y1) = screen_verts[i];
                        let (x2, y2) = screen_verts[i + 1];
                        renderer.line(x1, y1, x2, y2, line_color, overlay.line_width);
                    }
                    // Close the polygon if needed
                    if *closed && screen_verts.len() >= 2 {
                        let (x1, y1) = screen_verts[screen_verts.len() - 1];
                        let (x2, y2) = screen_verts[0];
                        renderer.line(x1, y1, x2, y2, line_color, overlay.line_width);
                    }

                    // Draw vertex handles if selected OR if polygon is not closed (preview mode)
                    if overlay.selected || !*closed {
                        let handle_size = 6.0;
                        for (i, (sx, sy)) in screen_verts.iter().enumerate() {
                            let handle_bounds = Bounds::new(
                                sx - handle_size / 2.0,
                                sy - handle_size / 2.0,
                                handle_size,
                                handle_size,
                            );
                            // First vertex gets special color when not closed (to show where to click to close)
                            let handle_fill = if i == 0 && !*closed && screen_verts.len() >= 3 {
                                Color::rgba(0.0, 1.0, 0.0, 0.8) // Green - click here to close
                            } else {
                                Color::WHITE
                            };
                            renderer.fill_rect(handle_bounds, handle_fill);
                            renderer.stroke_rect(handle_bounds, line_color, 1.0);
                        }
                    }
                }
            }
        }

        // Pop clip before drawing controls (they should be visible even at edges)
        renderer.pop_clip();

        // Draw zoom info - use calculated zoom for current mode/bounds
        let display_zoom = self.calculate_zoom_for_mode(&bounds);
        let zoom_text = format!("{:.0}%", display_zoom * 100.0);
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
                ("+", 14.0),   // Larger font for + symbol
                ("-", 14.0),   // Larger font for - symbol
                ("1:1", 10.0), // Smaller font for text labels
                ("Fit", 10.0),
            ];

            for (i, (label, font_size)) in controls.iter().enumerate() {
                let btn_bounds = self.control_button_bounds(i, &bounds);

                // Button background
                renderer.fill_rect(btn_bounds, Color::BUTTON_BG);
                renderer.stroke_rect(btn_bounds, Color::BORDER, 1.0);

                // Button label - center in button
                // Approximate character width based on font size (roughly 0.6 * font_size for monospace)
                let char_width = font_size * 0.6;
                let text_width = label.len() as f32 * char_width;
                let label_x = btn_bounds.x + (btn_bounds.width - text_width) / 2.0;
                let label_y = btn_bounds.y + (btn_bounds.height - font_size) / 2.0;
                renderer.text(*label, label_x, label_y, *font_size, Color::TEXT_PRIMARY);
            }
        }

        // Draw border around the viewer
        renderer.stroke_rect(bounds, Color::BORDER, 1.0);

        renderer.end_overlay();
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        // Sync state with current bounds - this resolves any pending FitToView mode
        // and ensures zoom operations use the correct base value.
        self.state.sync_with_bounds(bounds.width, bounds.height, self.texture_width, self.texture_height);

        match event {
            // Handle control button clicks (left mouse)
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
                            return self.emit_change_with_bounds(&bounds);
                        }
                        ControlButton::ZoomOut => {
                            self.state.zoom_out();
                            return self.emit_change_with_bounds(&bounds);
                        }
                        ControlButton::OneToOne => {
                            self.state.set_one_to_one();
                            return self.emit_change_with_bounds(&bounds);
                        }
                        ControlButton::FitToView => {
                            self.state.set_fit_to_view();
                            return self.emit_change_with_bounds(&bounds);
                        }
                    }
                }

                // In annotation mode, emit pointer events for drawing
                if self.interaction_mode == InteractionMode::Annotate {
                    self.state.pointer_state = PointerState::AnnotationDrag;
                    self.state.sync_with_bounds(bounds.width, bounds.height, self.texture_width, self.texture_height);
                    let (image_x, image_y) = self.screen_to_image(position.0, position.1, &bounds);
                    return self.on_pointer.call(ImagePointerEvent {
                        image_x,
                        image_y,
                        screen_x: position.0,
                        screen_y: position.1,
                        kind: PointerEventKind::DragStart,
                        viewer_state: self.state.clone(),
                    });
                }
                None
            }

            // Handle left mouse release for annotation mode
            Event::MouseRelease {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if self.state.pointer_state == PointerState::AnnotationDrag {
                    self.state.pointer_state = PointerState::Idle;
                    self.state.sync_with_bounds(bounds.width, bounds.height, self.texture_width, self.texture_height);
                    let (image_x, image_y) = self.screen_to_image(position.0, position.1, &bounds);
                    return self.on_pointer.call(ImagePointerEvent {
                        image_x,
                        image_y,
                        screen_x: position.0,
                        screen_y: position.1,
                        kind: PointerEventKind::DragEnd,
                        viewer_state: self.state.clone(),
                    });
                }
                None
            }

            // Start panning with middle mouse button
            Event::MousePress {
                button: MouseButton::Middle,
                position,
                ..
            } => {
                if !bounds.contains(position.0, position.1) {
                    return None;
                }

                if self.pannable {
                    self.state.drag.start_drag_with(PanDragData {
                        last_pos: *position,
                    });
                    // Emit change to persist dragging state
                    return self.emit_change_with_bounds(&bounds);
                }
                None
            }

            Event::MouseRelease {
                button: MouseButton::Middle,
                ..
            } => {
                if self.state.drag.is_dragging() {
                    self.state.drag.stop_drag();
                    // Emit change to persist state
                    return self.emit_change_with_bounds(&bounds);
                }
                None
            }

            Event::MouseMove { position, .. } => {
                // Handle pan drag move (middle mouse) - check first, works regardless of annotation mode
                if let Some((last_x, last_y)) = self.state.drag.last_pos() {
                    if self.pannable {
                        let delta_x = position.0 - last_x;
                        let delta_y = position.1 - last_y;

                        let clip_delta_x = delta_x / (bounds.width / 2.0);
                        let clip_delta_y = -delta_y / (bounds.height / 2.0);

                        self.state.pan_by(clip_delta_x, clip_delta_y);
                        self.state.drag.update_pos(*position);

                        return self.emit_change_with_bounds(&bounds);
                    }
                }

                // Handle annotation drag move (left mouse)
                if self.state.pointer_state == PointerState::AnnotationDrag {
                    let (image_x, image_y) = self.screen_to_image(position.0, position.1, &bounds);
                    return self.on_pointer.call(ImagePointerEvent {
                        image_x,
                        image_y,
                        screen_x: position.0,
                        screen_y: position.1,
                        kind: PointerEventKind::DragMove,
                        viewer_state: self.state.clone(),
                    });
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
                self.emit_change_with_bounds(&bounds)
            }

            Event::KeyPress { key, .. } => {
                match key {
                    KeyCode::Plus | KeyCode::Equal => {
                        if self.zoomable {
                            self.state.zoom_in();
                            return self.emit_change_with_bounds(&bounds);
                        }
                    }
                    KeyCode::Minus => {
                        if self.zoomable {
                            self.state.zoom_out();
                            return self.emit_change_with_bounds(&bounds);
                        }
                    }
                    KeyCode::Key0 => {
                        self.state.set_one_to_one();
                        return self.emit_change_with_bounds(&bounds);
                    }
                    KeyCode::F => {
                        self.state.set_fit_to_view();
                        return self.emit_change_with_bounds(&bounds);
                    }
                    KeyCode::Up => {
                        if self.pannable {
                            self.state.pan_by(0.0, PAN_SPEED);
                            return self.emit_change_with_bounds(&bounds);
                        }
                    }
                    KeyCode::Down => {
                        if self.pannable {
                            self.state.pan_by(0.0, -PAN_SPEED);
                            return self.emit_change_with_bounds(&bounds);
                        }
                    }
                    KeyCode::Left => {
                        if self.pannable {
                            self.state.pan_by(PAN_SPEED, 0.0);
                            return self.emit_change_with_bounds(&bounds);
                        }
                    }
                    KeyCode::Right => {
                        if self.pannable {
                            self.state.pan_by(-PAN_SPEED, 0.0);
                            return self.emit_change_with_bounds(&bounds);
                        }
                    }
                    _ => {}
                }
                None
            }

            Event::CursorLeft => {
                // Cursor left window - release any drag states
                let mut changed = false;
                if self.state.drag.is_dragging() {
                    self.state.drag.stop_drag();
                    changed = true;
                    log::debug!("ImageViewer: stopped panning (cursor left window)");
                }
                if self.state.pointer_state != PointerState::Idle {
                    self.state.pointer_state = PointerState::Idle;
                    changed = true;
                    log::debug!("ImageViewer: stopped annotation dragging (cursor left window)");
                }
                if changed {
                    return self.emit_change_with_bounds(&bounds);
                }
                None
            }

            _ => None,
        }
    }
}
