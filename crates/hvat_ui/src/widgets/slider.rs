//! A slider widget for selecting values within a range.

use crate::{builder_field, callback_setter, Color, ConcreteSize, ConcreteSizeXY, Event, Layout, Length, Limits, MouseButton, Rectangle, Renderer, Widget};
use crate::theme::colors;

/// Identifies which slider is being interacted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SliderId {
    Brightness,
    Contrast,
    Gamma,
    HueShift,
    BandRed,
    BandGreen,
    BandBlue,
    Custom(u32),
}

/// A slider widget for selecting values within a range.
pub struct Slider<Message> {
    id: SliderId,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    width: Length,
    is_dragging: bool,
    on_drag_start: Option<Box<dyn Fn(SliderId, f32) -> Message>>,
    on_change: Option<Box<dyn Fn(f32) -> Message>>,
    on_drag_end: Option<Box<dyn Fn() -> Message>>,
    track_color: Color,
    fill_color: Color,
    thumb_color: Color,
}

impl<Message> Slider<Message> {
    const TRACK_HEIGHT: f32 = 6.0;
    const THUMB_SIZE: f32 = 16.0;
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
            track_color: colors::SLIDER_TRACK,
            fill_color: colors::SLIDER_FILL,
            thumb_color: colors::SLIDER_THUMB,
        }
    }

    // Builder methods using macros
    builder_field!(id, SliderId);
    builder_field!(width, Length);
    builder_field!(step, f32);
    builder_field!(track_color, Color);
    builder_field!(fill_color, Color);
    builder_field!(thumb_color, Color);

    /// Set whether this slider is being dragged (from external state).
    pub fn dragging(mut self, is_dragging: bool) -> Self {
        self.is_dragging = is_dragging;
        self
    }

    /// Set the callback when drag starts.
    pub fn on_drag_start<F>(mut self, f: F) -> Self
    where
        F: Fn(SliderId, f32) -> Message + 'static,
    {
        self.on_drag_start = Some(Box::new(f));
        self
    }

    // Callback setters using macros
    callback_setter!(on_change, f32);
    callback_setter!(on_drag_end);

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
        let bounds = Rectangle::new(0.0, 0.0, size.width, size.height);

        if matches!(self.width, Length::Fill) {
            Layout::fill_width(bounds)
        } else {
            Layout::new(bounds)
        }
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
                    let new_value = self.x_to_value(position.x, &bounds);
                    log::debug!("🎚️ Slider {:?} MousePressed at value {:.2}", self.id, new_value);

                    if let Some(ref on_drag_start) = self.on_drag_start {
                        return Some(on_drag_start(self.id, new_value));
                    }
                }
                None
            }
            Event::MouseReleased { button: MouseButton::Left, .. } => {
                if self.is_dragging {
                    log::info!("🎚️ Slider {:?} MouseReleased while dragging, firing on_drag_end", self.id);
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

    fn natural_size(&self, max_width: ConcreteSize) -> ConcreteSizeXY {
        let width = match self.width {
            Length::Fill | Length::FillPortion(_) => return self.minimum_size(),
            Length::Units(px) => px,
            Length::Shrink => 200.0,
        };
        ConcreteSizeXY::from_f32(width.min(max_width.get()), Self::HEIGHT)
    }

    fn minimum_size(&self) -> ConcreteSizeXY {
        ConcreteSizeXY::from_f32(Self::THUMB_SIZE * 3.0, Self::HEIGHT)
    }

    fn is_shrinkable(&self) -> bool {
        true
    }
}

/// Helper function to create a slider.
pub fn slider<Message>(min: f32, max: f32, value: f32) -> Slider<Message> {
    Slider::new(min, max, value)
}
