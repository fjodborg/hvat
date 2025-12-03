//! Interactive hyperspectral image widget with pan and zoom support.
//!
//! Similar to PanZoomImage, but uses GPU-based band compositing for instant
//! band selection changes.

use crate::{
    BandSelectionUniform, Color, Event, HyperspectralImageHandle, ImageAdjustments, Key, Layout,
    Length, Limits, MouseButton, Overlay, OverlayShape, Rectangle, Renderer, Widget,
};

/// An interactive hyperspectral image widget that supports panning, zooming, and band selection.
///
/// Band compositing happens on the GPU, so changing band selection only requires
/// updating a uniform buffer - no CPU-side image regeneration needed.
pub struct HyperspectralImage<Message> {
    handle: HyperspectralImageHandle,
    band_selection: BandSelectionUniform,
    width: Length,
    height: Length,
    /// Current pan offset in pixels
    pan: (f32, f32),
    /// Current zoom level (1.0 = 100%)
    zoom: f32,
    /// Whether the user is currently dragging for pan (external state)
    is_dragging: bool,
    /// Whether the user is currently drawing an annotation (external state)
    is_drawing: bool,
    /// Image adjustments (brightness, contrast, gamma, hue)
    adjustments: ImageAdjustments,
    /// Overlay shapes to draw on top of the image
    overlay: Overlay,
    /// Callback when pan drag starts (middle mouse)
    on_drag_start: Option<Box<dyn Fn((f32, f32)) -> Message>>,
    /// Callback when pan drag moves
    on_drag_move: Option<Box<dyn Fn((f32, f32)) -> Message>>,
    /// Callback when pan drag ends
    on_drag_end: Option<Box<dyn Fn() -> Message>>,
    /// Callback when zoom changes (new_zoom, cursor_x, cursor_y, widget_center_x, widget_center_y)
    on_zoom: Option<Box<dyn Fn(f32, f32, f32, f32, f32) -> Message>>,
    /// Callback when left click (annotation) - receives image-space coordinates
    on_click: Option<Box<dyn Fn((f32, f32)) -> Message>>,
    /// Callback when left mouse drag during drawing
    on_draw_move: Option<Box<dyn Fn((f32, f32)) -> Message>>,
    /// Callback when left mouse released (finish drawing)
    on_draw_end: Option<Box<dyn Fn() -> Message>>,
    /// Callback when Space key pressed (finish polygon)
    on_space: Option<Box<dyn Fn() -> Message>>,
}

impl<Message> HyperspectralImage<Message> {
    /// Create a new hyperspectral image widget.
    pub fn new(handle: HyperspectralImageHandle, band_selection: BandSelectionUniform) -> Self {
        Self {
            handle,
            band_selection,
            width: Length::Fill,
            height: Length::Fill,
            pan: (0.0, 0.0),
            zoom: 1.0,
            is_dragging: false,
            is_drawing: false,
            adjustments: ImageAdjustments::new(),
            overlay: Overlay::new(),
            on_drag_start: None,
            on_drag_move: None,
            on_drag_end: None,
            on_zoom: None,
            on_click: None,
            on_draw_move: None,
            on_draw_end: None,
            on_space: None,
        }
    }

    /// Set the band selection for RGB composite.
    pub fn band_selection(mut self, band_selection: BandSelectionUniform) -> Self {
        self.band_selection = band_selection;
        self
    }

    /// Set the overlay shapes to draw on top of the image.
    pub fn overlay(mut self, overlay: Overlay) -> Self {
        self.overlay = overlay;
        self
    }

    /// Set the pan offset (from external state).
    pub fn pan(mut self, pan: (f32, f32)) -> Self {
        self.pan = pan;
        self
    }

    /// Set the zoom level (from external state).
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// Set whether dragging is in progress (from external state).
    pub fn dragging(mut self, is_dragging: bool) -> Self {
        self.is_dragging = is_dragging;
        self
    }

    /// Set whether drawing is in progress (from external state).
    pub fn drawing(mut self, is_drawing: bool) -> Self {
        self.is_drawing = is_drawing;
        self
    }

    /// Set image adjustments (brightness, contrast, gamma, hue).
    pub fn adjustments(mut self, adjustments: ImageAdjustments) -> Self {
        self.adjustments = adjustments;
        self
    }

    /// Set the widget width.
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Set the widget height.
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Set the callback when drag starts.
    pub fn on_drag_start<F>(mut self, f: F) -> Self
    where
        F: Fn((f32, f32)) -> Message + 'static,
    {
        self.on_drag_start = Some(Box::new(f));
        self
    }

    /// Set the callback when drag moves.
    pub fn on_drag_move<F>(mut self, f: F) -> Self
    where
        F: Fn((f32, f32)) -> Message + 'static,
    {
        self.on_drag_move = Some(Box::new(f));
        self
    }

    /// Set the callback when drag ends.
    pub fn on_drag_end<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'static,
    {
        self.on_drag_end = Some(Box::new(f));
        self
    }

    /// Set the callback when zoom changes.
    pub fn on_zoom<F>(mut self, f: F) -> Self
    where
        F: Fn(f32, f32, f32, f32, f32) -> Message + 'static,
    {
        self.on_zoom = Some(Box::new(f));
        self
    }

    /// Set the callback when left mouse clicked (for annotation).
    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: Fn((f32, f32)) -> Message + 'static,
    {
        self.on_click = Some(Box::new(f));
        self
    }

    /// Set the callback when left mouse drag during drawing.
    pub fn on_draw_move<F>(mut self, f: F) -> Self
    where
        F: Fn((f32, f32)) -> Message + 'static,
    {
        self.on_draw_move = Some(Box::new(f));
        self
    }

    /// Set the callback when left mouse released.
    pub fn on_draw_end<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'static,
    {
        self.on_draw_end = Some(Box::new(f));
        self
    }

    /// Set the callback when Space key pressed.
    pub fn on_space<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'static,
    {
        self.on_space = Some(Box::new(f));
        self
    }

    /// Convert screen coordinates to image coordinates.
    fn screen_to_image(&self, screen_x: f32, screen_y: f32, bounds: &Rectangle) -> (f32, f32) {
        let center_x = bounds.x + bounds.width / 2.0;
        let center_y = bounds.y + bounds.height / 2.0;
        let rel_x = screen_x - center_x;
        let rel_y = screen_y - center_y;
        let img_x = (rel_x - self.pan.0) / self.zoom;
        let img_y = (rel_y - self.pan.1) / self.zoom;
        (img_x, img_y)
    }

    /// Convert image coordinates to screen coordinates.
    fn image_to_screen(&self, img_x: f32, img_y: f32, bounds: &Rectangle) -> (f32, f32) {
        let center_x = bounds.x + bounds.width / 2.0;
        let center_y = bounds.y + bounds.height / 2.0;
        let screen_x = center_x + img_x * self.zoom + self.pan.0;
        let screen_y = center_y + img_y * self.zoom + self.pan.1;
        (screen_x, screen_y)
    }

    /// Draw overlay shapes on top of the image.
    fn draw_overlay(&self, renderer: &mut Renderer, bounds: &Rectangle) {
        const STROKE_WIDTH: f32 = 2.0;
        const POINT_RADIUS: f32 = 6.0;
        const SELECTION_STROKE_WIDTH: f32 = 3.0;

        let draw_item = |renderer: &mut Renderer, item: &crate::OverlayItem| {
            let color = item.color;
            let stroke_width = if item.selected {
                SELECTION_STROKE_WIDTH
            } else {
                STROKE_WIDTH
            };

            match &item.shape {
                OverlayShape::Point { x, y, radius } => {
                    let (sx, sy) = self.image_to_screen(*x, *y, bounds);
                    let r = radius.max(POINT_RADIUS);
                    renderer.fill_circle(sx, sy, r, color);
                    renderer.stroke_circle(sx, sy, r, Color::BLACK, 1.0);
                    if item.selected {
                        renderer.stroke_circle(sx, sy, r + 2.0, Color::WHITE, 2.0);
                    }
                }
                OverlayShape::Rect { x, y, width, height } => {
                    let (sx, sy) = self.image_to_screen(*x, *y, bounds);
                    let sw = width * self.zoom;
                    let sh = height * self.zoom;
                    let rect = Rectangle::new(sx, sy, sw, sh);
                    let fill_color = Color::new(color.r, color.g, color.b, 0.2);
                    renderer.fill_rect(rect, fill_color);
                    renderer.stroke_rect(rect, color, stroke_width);
                    if item.selected {
                        renderer.stroke_rect(
                            Rectangle::new(sx - 2.0, sy - 2.0, sw + 4.0, sh + 4.0),
                            Color::WHITE,
                            2.0,
                        );
                    }
                }
                OverlayShape::Polygon { vertices, closed } => {
                    if vertices.is_empty() {
                        return;
                    }
                    if vertices.len() == 1 {
                        let (x, y) = vertices[0];
                        let (sx, sy) = self.image_to_screen(x, y, bounds);
                        renderer.fill_circle(sx, sy, 6.0, color);
                        renderer.stroke_circle(sx, sy, 6.0, Color::BLACK, 1.0);
                        return;
                    }
                    for i in 0..vertices.len() {
                        let (x1, y1) = vertices[i];
                        let (x2, y2) = if *closed {
                            vertices[(i + 1) % vertices.len()]
                        } else if i + 1 < vertices.len() {
                            vertices[i + 1]
                        } else {
                            break;
                        };
                        let (sx1, sy1) = self.image_to_screen(x1, y1, bounds);
                        let (sx2, sy2) = self.image_to_screen(x2, y2, bounds);
                        renderer.draw_line(sx1, sy1, sx2, sy2, color, stroke_width);
                    }
                    for (x, y) in vertices {
                        let (sx, sy) = self.image_to_screen(*x, *y, bounds);
                        renderer.fill_circle(sx, sy, 4.0, color);
                    }
                }
                OverlayShape::Line { x1, y1, x2, y2 } => {
                    let (sx1, sy1) = self.image_to_screen(*x1, *y1, bounds);
                    let (sx2, sy2) = self.image_to_screen(*x2, *y2, bounds);
                    renderer.draw_line(sx1, sy1, sx2, sy2, color, stroke_width);
                }
            }
        };

        for item in &self.overlay.items {
            draw_item(renderer, item);
        }

        if let Some(ref preview) = self.overlay.preview {
            draw_item(renderer, preview);
        }
    }
}

impl<Message: Clone> Widget<Message> for HyperspectralImage<Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let available_width = if limits.max_width.is_finite() {
            limits.max_width
        } else {
            600.0
        };
        let available_height = if limits.max_height.is_finite() {
            limits.max_height
        } else {
            400.0
        };

        let width = self.width.resolve(available_width, available_width);
        let height = self.height.resolve(available_height, available_height);

        let size = limits.resolve(width, height);
        let bounds = Rectangle::new(0.0, 0.0, size.width, size.height);

        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        // Clip image to widget bounds
        renderer.push_clip(bounds);

        // Draw the hyperspectral image with GPU-based band compositing
        renderer.draw_hyperspectral_image_with_adjustments(
            &self.handle,
            bounds,
            self.pan,
            self.zoom,
            self.band_selection,
            self.adjustments,
        );

        // Draw overlay shapes on top
        self.draw_overlay(renderer, &bounds);

        renderer.pop_clip();
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();

        match event {
            // Left mouse - annotation drawing
            Event::MousePressed {
                button: MouseButton::Left,
                position,
            } => {
                if bounds.contains(*position) {
                    let img_coords = self.screen_to_image(position.x, position.y, &bounds);
                    if let Some(ref on_click) = self.on_click {
                        return Some(on_click(img_coords));
                    }
                }
                None
            }
            Event::MouseReleased {
                button: MouseButton::Left,
                ..
            } => {
                if self.is_drawing {
                    if let Some(ref on_draw_end) = self.on_draw_end {
                        return Some(on_draw_end());
                    }
                }
                None
            }
            // Space key - finish polygon
            Event::KeyPressed {
                key: Key::Space, ..
            } => {
                if let Some(ref on_space) = self.on_space {
                    return Some(on_space());
                }
                None
            }
            // Middle mouse - panning
            Event::MousePressed {
                button: MouseButton::Middle,
                position,
            } => {
                if bounds.contains(*position) {
                    if let Some(ref on_drag_start) = self.on_drag_start {
                        return Some(on_drag_start((position.x, position.y)));
                    }
                }
                None
            }
            Event::MouseReleased {
                button: MouseButton::Middle,
                ..
            } => {
                if self.is_dragging {
                    if let Some(ref on_drag_end) = self.on_drag_end {
                        return Some(on_drag_end());
                    }
                }
                None
            }
            Event::MouseMoved { position } => {
                if self.is_dragging {
                    if let Some(ref on_drag_move) = self.on_drag_move {
                        return Some(on_drag_move((position.x, position.y)));
                    }
                }
                if self.is_drawing && bounds.contains(*position) {
                    let img_coords = self.screen_to_image(position.x, position.y, &bounds);
                    if let Some(ref on_draw_move) = self.on_draw_move {
                        return Some(on_draw_move(img_coords));
                    }
                }
                None
            }
            Event::MouseWheel { delta, position } => {
                if bounds.contains(*position) {
                    let zoom_factor = if *delta > 0.0 { 1.1 } else { 0.9 };
                    let new_zoom = (self.zoom * zoom_factor).clamp(0.1, 10.0);

                    let widget_center_x = bounds.x + bounds.width / 2.0;
                    let widget_center_y = bounds.y + bounds.height / 2.0;

                    if let Some(ref on_zoom) = self.on_zoom {
                        return Some(on_zoom(
                            new_zoom,
                            position.x,
                            position.y,
                            widget_center_x,
                            widget_center_y,
                        ));
                    }
                }
                None
            }
            _ => None,
        }
    }
}

/// Convenience function to create a hyperspectral image widget.
pub fn hyperspectral_image<Message>(
    handle: HyperspectralImageHandle,
    band_selection: BandSelectionUniform,
) -> HyperspectralImage<Message> {
    HyperspectralImage::new(handle, band_selection)
}
