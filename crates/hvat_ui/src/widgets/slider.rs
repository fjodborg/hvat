//! A slider widget for selecting values within a range.

use crate::{Color, Event, Layout, Length, Limits, MouseButton, Rectangle, Renderer, Widget};

/// Identifies which slider is being interacted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SliderId {
    Brightness,
    Contrast,
    Gamma,
    HueShift,
    Custom(u32),
}

/// A slider widget for selecting values within a range.
pub struct Slider<Message> {
    /// Slider identifier
    id: SliderId,
    /// Current value
    value: f32,
    /// Minimum value
    min: f32,
    /// Maximum value
    max: f32,
    /// Step size (0 for continuous)
    step: f32,
    /// Widget width
    width: Length,
    /// Whether this slider is currently being dragged (from external state)
    is_dragging: bool,
    /// Callback when drag starts
    on_drag_start: Option<Box<dyn Fn(SliderId) -> Message>>,
    /// Callback when value changes during drag
    on_change: Option<Box<dyn Fn(f32) -> Message>>,
    /// Callback when drag ends
    on_drag_end: Option<Box<dyn Fn() -> Message>>,
    /// Track color
    track_color: Color,
    /// Fill color
    fill_color: Color,
    /// Thumb color
    thumb_color: Color,
}

impl<Message> Slider<Message> {
    /// Height of the slider track
    const TRACK_HEIGHT: f32 = 6.0;
    /// Diameter of the thumb
    const THUMB_SIZE: f32 = 16.0;
    /// Total widget height
    const HEIGHT: f32 = 24.0;

    /// Create a new slider.
    pub fn new(min: f32, max: f32, value: f32) -> Self {
        Self {
            id: SliderId::Custom(0),
            value: value.clamp(min, max),
            min,
            max,
            step: 0.0,
            width: Length::Units(200.0),
            is_dragging: false,
            on_drag_start: None,
            on_change: None,
            on_drag_end: None,
            track_color: Color::rgb(0.3, 0.3, 0.3),
            fill_color: Color::rgb(0.3, 0.6, 0.9),
            thumb_color: Color::WHITE,
        }
    }

    /// Set the slider ID.
    pub fn id(mut self, id: SliderId) -> Self {
        self.id = id;
        self
    }

    /// Set whether this slider is being dragged (from external state).
    pub fn dragging(mut self, is_dragging: bool) -> Self {
        self.is_dragging = is_dragging;
        self
    }

    /// Set the slider width.
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Set the step size (0 for continuous).
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Set the callback when drag starts.
    pub fn on_drag_start<F>(mut self, f: F) -> Self
    where
        F: Fn(SliderId) -> Message + 'static,
    {
        self.on_drag_start = Some(Box::new(f));
        self
    }

    /// Set the callback when value changes.
    pub fn on_change<F>(mut self, f: F) -> Self
    where
        F: Fn(f32) -> Message + 'static,
    {
        self.on_change = Some(Box::new(f));
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

    /// Set the track color.
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self
    }

    /// Set the fill color.
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = color;
        self
    }

    /// Set the thumb color.
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = color;
        self
    }

    /// Convert x position to value.
    fn x_to_value(&self, x: f32, bounds: &Rectangle) -> f32 {
        let track_start = bounds.x + Self::THUMB_SIZE / 2.0;
        let track_width = bounds.width - Self::THUMB_SIZE;
        let ratio = ((x - track_start) / track_width).clamp(0.0, 1.0);
        let value = self.min + ratio * (self.max - self.min);

        if self.step > 0.0 {
            let steps = ((value - self.min) / self.step).round();
            (self.min + steps * self.step).clamp(self.min, self.max)
        } else {
            value
        }
    }

    /// Get the normalized position (0-1) of the current value.
    fn value_ratio(&self) -> f32 {
        if (self.max - self.min).abs() < f32::EPSILON {
            0.0
        } else {
            (self.value - self.min) / (self.max - self.min)
        }
    }
}

impl<Message: Clone> Widget<Message> for Slider<Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let width = self.width.resolve(limits.max_width, 200.0);
        let size = limits.resolve(width, Self::HEIGHT);
        Layout::new(Rectangle::new(0.0, 0.0, size.width, size.height))
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();
        let ratio = self.value_ratio();

        // Track background
        let track_y = bounds.y + (bounds.height - Self::TRACK_HEIGHT) / 2.0;
        let track_rect = Rectangle::new(
            bounds.x + Self::THUMB_SIZE / 2.0,
            track_y,
            bounds.width - Self::THUMB_SIZE,
            Self::TRACK_HEIGHT,
        );
        renderer.fill_rect(track_rect, self.track_color);

        // Fill (progress)
        let fill_width = (bounds.width - Self::THUMB_SIZE) * ratio;
        if fill_width > 0.0 {
            let fill_rect = Rectangle::new(
                bounds.x + Self::THUMB_SIZE / 2.0,
                track_y,
                fill_width,
                Self::TRACK_HEIGHT,
            );
            renderer.fill_rect(fill_rect, self.fill_color);
        }

        // Thumb
        let thumb_x = bounds.x + ratio * (bounds.width - Self::THUMB_SIZE);
        let thumb_y = bounds.y + (bounds.height - Self::THUMB_SIZE) / 2.0;
        let thumb_rect = Rectangle::new(thumb_x, thumb_y, Self::THUMB_SIZE, Self::THUMB_SIZE);
        renderer.fill_rect(thumb_rect, self.thumb_color);
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();

        match event {
            Event::MousePressed { button: MouseButton::Left, position } => {
                if bounds.contains(*position) {
                    // Calculate value from click position and emit change
                    let new_value = self.x_to_value(position.x, &bounds);

                    // First emit drag start
                    if let Some(ref on_drag_start) = self.on_drag_start {
                        // We'll emit the value change separately
                        let start_msg = on_drag_start(self.id);

                        // Also emit the value change for the click
                        if let Some(ref on_change) = self.on_change {
                            // Return the start message, the app will handle updating the value
                            // based on position in the drag move handler
                            return Some(start_msg);
                        }
                        return Some(start_msg);
                    }

                    // Fallback: just emit value change
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(new_value));
                    }
                }
                None
            }
            Event::MouseReleased { button: MouseButton::Left, .. } => {
                if self.is_dragging {
                    if let Some(ref on_drag_end) = self.on_drag_end {
                        return Some(on_drag_end());
                    }
                }
                None
            }
            Event::MouseMoved { position } => {
                if self.is_dragging {
                    let new_value = self.x_to_value(position.x, &bounds);
                    if let Some(ref on_change) = self.on_change {
                        return Some(on_change(new_value));
                    }
                }
                None
            }
            _ => None,
        }
    }
}

/// Helper function to create a slider.
pub fn slider<Message>(min: f32, max: f32, value: f32) -> Slider<Message> {
    Slider::new(min, max, value)
}
