//! Interactive image widget with pan and zoom support.

use crate::{Color, ConcreteSize, ConcreteSizeXY, Event, ImageAdjustments, ImageHandle, Key, Layout, Length, Limits, MouseButton, Overlay, OverlayShape, Rectangle, Renderer, Widget};

/// An interactive image widget that supports panning and zooming.
pub struct PanZoomImage<Message> {
    handle: ImageHandle,
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

impl<Message> PanZoomImage<Message> {
    /// Create a new pan/zoom image widget.
    pub fn new(handle: ImageHandle) -> Self {
        Self {
            handle,
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
    /// Callback receives: (new_zoom, cursor_x, cursor_y, widget_center_x, widget_center_y)
    pub fn on_zoom<F>(mut self, f: F) -> Self
    where
        F: Fn(f32, f32, f32, f32, f32) -> Message + 'static,
    {
        self.on_zoom = Some(Box::new(f));
        self
    }

    /// Set the callback when left mouse clicked (for annotation).
    /// Callback receives image-space coordinates (x, y).
    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: Fn((f32, f32)) -> Message + 'static,
    {
        self.on_click = Some(Box::new(f));
        self
    }

    /// Set the callback when left mouse moves during drawing.
    /// Callback receives image-space coordinates (x, y).
    pub fn on_draw_move<F>(mut self, f: F) -> Self
    where
        F: Fn((f32, f32)) -> Message + 'static,
    {
        self.on_draw_move = Some(Box::new(f));
        self
    }

    /// Set the callback when left mouse released (finish drawing).
    pub fn on_draw_end<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'static,
    {
        self.on_draw_end = Some(Box::new(f));
        self
    }

    /// Set the callback when Space key is pressed (finish polygon).
    pub fn on_space<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'static,
    {
        self.on_space = Some(Box::new(f));
        self
    }

    /// Convert screen coordinates to image coordinates.
    /// Takes into account pan and zoom.
    /// Returns coordinates with origin at top-left corner of image (standard image coords).
    fn screen_to_image(&self, screen_x: f32, screen_y: f32, bounds: &Rectangle) -> (f32, f32) {
        // Widget center
        let center_x = bounds.x + bounds.width / 2.0;
        let center_y = bounds.y + bounds.height / 2.0;

        // Position relative to widget center
        let rel_x = screen_x - center_x;
        let rel_y = screen_y - center_y;

        // Remove pan offset and divide by zoom to get center-relative coordinates
        let center_rel_x = (rel_x - self.pan.0) / self.zoom;
        let center_rel_y = (rel_y - self.pan.1) / self.zoom;

        // Convert to top-left origin coordinates by adding half the image dimensions
        let img_x = center_rel_x + self.handle.width() as f32 / 2.0;
        let img_y = center_rel_y + self.handle.height() as f32 / 2.0;

        (img_x, img_y)
    }

    /// Convert image coordinates to screen coordinates.
    /// Takes into account pan and zoom.
    /// Expects coordinates with origin at top-left corner of image (standard image coords).
    fn image_to_screen(&self, img_x: f32, img_y: f32, bounds: &Rectangle) -> (f32, f32) {
        // Widget center
        let center_x = bounds.x + bounds.width / 2.0;
        let center_y = bounds.y + bounds.height / 2.0;

        // Convert from top-left origin to center-relative coordinates
        let center_rel_x = img_x - self.handle.width() as f32 / 2.0;
        let center_rel_y = img_y - self.handle.height() as f32 / 2.0;

        // Apply zoom and pan to get screen coordinates
        let screen_x = center_x + center_rel_x * self.zoom + self.pan.0;
        let screen_y = center_y + center_rel_y * self.zoom + self.pan.1;

        (screen_x, screen_y)
    }

    /// Draw overlay shapes on top of the image.
    fn draw_overlay(&self, renderer: &mut Renderer, bounds: &Rectangle) {
        // Style constants
        const STROKE_WIDTH: f32 = 2.0;
        const POINT_RADIUS: f32 = 6.0;
        const SELECTION_STROKE_WIDTH: f32 = 3.0;

        // Helper to draw a single overlay item
        let draw_item = |renderer: &mut Renderer, item: &crate::OverlayItem| {
            let color = item.color;
            let stroke_width = if item.selected { SELECTION_STROKE_WIDTH } else { STROKE_WIDTH };

            // Selection highlight color (bright yellow/white)
            let _selection_color = Color::new(1.0, 1.0, 0.5, 0.9);

            match &item.shape {
                OverlayShape::Point { x, y, radius } => {
                    let (sx, sy) = self.image_to_screen(*x, *y, bounds);
                    let r = radius.max(POINT_RADIUS);
                    // Draw filled circle
                    renderer.fill_circle(sx, sy, r, color);
                    // Draw outline for visibility
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
                    // Draw semi-transparent fill
                    let fill_color = Color::new(color.r, color.g, color.b, 0.2);
                    renderer.fill_rect(rect, fill_color);
                    // Draw outline
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
                    // If only one point, draw it as a marker
                    if vertices.len() == 1 {
                        let (x, y) = vertices[0];
                        let (sx, sy) = self.image_to_screen(x, y, bounds);
                        renderer.fill_circle(sx, sy, 6.0, color);
                        renderer.stroke_circle(sx, sy, 6.0, Color::BLACK, 1.0);
                        return;
                    }
                    // Draw polygon edges
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
                    // Draw vertex markers
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

        // Draw all overlay items
        for item in &self.overlay.items {
            draw_item(renderer, item);
        }

        // Draw preview last (on top)
        if let Some(ref preview) = self.overlay.preview {
            draw_item(renderer, preview);
        }
    }
}

impl<Message: Clone> Widget<Message> for PanZoomImage<Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Use available space, but fall back to reasonable defaults if unbounded
        let available_width = if limits.max_width.is_finite() {
            limits.max_width
        } else {
            600.0 // Default width when unbounded
        };
        let available_height = if limits.max_height.is_finite() {
            limits.max_height
        } else {
            400.0 // Default height when unbounded
        };

        let width = self.width.resolve(available_width, available_width);
        let height = self.height.resolve(available_height, available_height);

        let size = limits.resolve(width, height);
        let bounds = Rectangle::new(0.0, 0.0, size.width, size.height);

        // Report fill intent based on Length
        let fills_width = matches!(self.width, Length::Fill);
        let fills_height = matches!(self.height, Length::Fill);

        match (fills_width, fills_height) {
            (true, true) => Layout::fill_both(bounds),
            (true, false) => Layout::fill_width(bounds),
            (false, true) => Layout::fill_height(bounds),
            (false, false) => Layout::new(bounds),
        }
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        // Clip image to widget bounds - prevents image from extending beyond container
        renderer.push_clip(bounds);

        // Draw the image with current pan/zoom transform and adjustments
        renderer.draw_image_with_adjustments(&self.handle, bounds, self.pan, self.zoom, self.adjustments);

        // Draw overlay shapes on top of the image
        self.draw_overlay(renderer, &bounds);

        renderer.pop_clip();
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();

        match event {
            // Left mouse - annotation drawing
            Event::MousePressed { button: MouseButton::Left, position } => {
                if bounds.contains(*position) {
                    let img_coords = self.screen_to_image(position.x, position.y, &bounds);
                    if let Some(ref on_click) = self.on_click {
                        return Some(on_click(img_coords));
                    }
                }
                None
            }
            Event::MouseReleased { button: MouseButton::Left, .. } => {
                if self.is_drawing {
                    if let Some(ref on_draw_end) = self.on_draw_end {
                        return Some(on_draw_end());
                    }
                }
                None
            }
            // Space key - finish polygon
            Event::KeyPressed { key: Key::Space, .. } => {
                if let Some(ref on_space) = self.on_space {
                    return Some(on_space());
                }
                None
            }
            // Middle mouse - panning
            Event::MousePressed { button: MouseButton::Middle, position } => {
                // Check if click is within bounds (middle mouse for panning)
                if bounds.contains(*position) {
                    // Emit drag start message
                    if let Some(ref on_drag_start) = self.on_drag_start {
                        return Some(on_drag_start((position.x, position.y)));
                    }
                }
                None
            }
            Event::MouseReleased { button: MouseButton::Middle, .. } => {
                if self.is_dragging {
                    // Emit drag end message
                    if let Some(ref on_drag_end) = self.on_drag_end {
                        return Some(on_drag_end());
                    }
                }
                None
            }
            Event::MouseMoved { position } => {
                // Handle pan dragging (middle mouse)
                if self.is_dragging {
                    if let Some(ref on_drag_move) = self.on_drag_move {
                        return Some(on_drag_move((position.x, position.y)));
                    }
                }
                // Handle annotation drawing (left mouse)
                if self.is_drawing && bounds.contains(*position) {
                    let img_coords = self.screen_to_image(position.x, position.y, &bounds);
                    if let Some(ref on_draw_move) = self.on_draw_move {
                        return Some(on_draw_move(img_coords));
                    }
                }
                None
            }
            Event::MouseWheel { delta, position } => {
                // Check if mouse is within bounds
                if bounds.contains(*position) {
                    // Zoom factor: positive delta = zoom in, negative = zoom out
                    let zoom_factor = if *delta > 0.0 { 1.1 } else { 0.9 };
                    let new_zoom = (self.zoom * zoom_factor).clamp(0.1, 10.0);

                    // Calculate widget center
                    let widget_center_x = bounds.x + bounds.width / 2.0;
                    let widget_center_y = bounds.y + bounds.height / 2.0;

                    log::debug!("Zoom: {:.2}x at ({:.1}, {:.1}), widget center: ({:.1}, {:.1})",
                        new_zoom, position.x, position.y, widget_center_x, widget_center_y);

                    // Emit message if callback is set
                    if let Some(ref on_zoom) = self.on_zoom {
                        return Some(on_zoom(new_zoom, position.x, position.y, widget_center_x, widget_center_y));
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn natural_size(&self, _max_width: ConcreteSize) -> ConcreteSizeXY {
        // PanZoomImage typically fills available space
        // Return minimum size (200x200) since it's usually Fill
        ConcreteSizeXY::from_f32(200.0, 200.0)
    }

    fn minimum_size(&self) -> ConcreteSizeXY {
        // PanZoomImage needs minimum space for display and interaction
        ConcreteSizeXY::from_f32(200.0, 200.0)
    }
}

/// Helper function to create a pan/zoom image.
pub fn pan_zoom_image<Message>(handle: ImageHandle) -> PanZoomImage<Message> {
    PanZoomImage::new(handle)
}
