//! Interactive image widget with pan and zoom support.

use crate::{Event, ImageAdjustments, ImageHandle, Layout, Length, Limits, MouseButton, Rectangle, Renderer, Widget};

/// An interactive image widget that supports panning and zooming.
pub struct PanZoomImage<Message> {
    handle: ImageHandle,
    width: Length,
    height: Length,
    /// Current pan offset in pixels
    pan: (f32, f32),
    /// Current zoom level (1.0 = 100%)
    zoom: f32,
    /// Whether the user is currently dragging (external state)
    is_dragging: bool,
    /// Image adjustments (brightness, contrast, gamma, hue)
    adjustments: ImageAdjustments,
    /// Callback when drag starts
    on_drag_start: Option<Box<dyn Fn((f32, f32)) -> Message>>,
    /// Callback when drag moves
    on_drag_move: Option<Box<dyn Fn((f32, f32)) -> Message>>,
    /// Callback when drag ends
    on_drag_end: Option<Box<dyn Fn() -> Message>>,
    /// Callback when zoom changes
    on_zoom: Option<Box<dyn Fn(f32) -> Message>>,
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
            adjustments: ImageAdjustments::new(),
            on_drag_start: None,
            on_drag_move: None,
            on_drag_end: None,
            on_zoom: None,
        }
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
        F: Fn(f32) -> Message + 'static,
    {
        self.on_zoom = Some(Box::new(f));
        self
    }
}

impl<Message: Clone> Widget<Message> for PanZoomImage<Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Use available space
        let width = self.width.resolve(limits.max_width, limits.max_width);
        let height = self.height.resolve(limits.max_height, limits.max_height);

        let size = limits.resolve(width, height);
        let bounds = Rectangle::new(0.0, 0.0, size.width, size.height);

        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        // Draw the image with current pan/zoom transform and adjustments
        renderer.draw_image_with_adjustments(&self.handle, bounds, self.pan, self.zoom, self.adjustments);
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();

        match event {
            Event::MousePressed { button: MouseButton::Left, position } => {
                // Check if click is within bounds
                if bounds.contains(*position) {
                    // Emit drag start message
                    if let Some(ref on_drag_start) = self.on_drag_start {
                        return Some(on_drag_start((position.x, position.y)));
                    }
                }
                None
            }
            Event::MouseReleased { button: MouseButton::Left, .. } => {
                if self.is_dragging {
                    // Emit drag end message
                    if let Some(ref on_drag_end) = self.on_drag_end {
                        return Some(on_drag_end());
                    }
                }
                None
            }
            Event::MouseMoved { position } => {
                if self.is_dragging {
                    // Emit drag move message with current position
                    if let Some(ref on_drag_move) = self.on_drag_move {
                        return Some(on_drag_move((position.x, position.y)));
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

                    log::debug!("Zoom: {:.2}x at {:?}", new_zoom, position);

                    // Emit message if callback is set
                    if let Some(ref on_zoom) = self.on_zoom {
                        return Some(on_zoom(new_zoom));
                    }
                }
                None
            }
            _ => None,
        }
    }
}

/// Helper function to create a pan/zoom image.
pub fn pan_zoom_image<Message>(handle: ImageHandle) -> PanZoomImage<Message> {
    PanZoomImage::new(handle)
}
