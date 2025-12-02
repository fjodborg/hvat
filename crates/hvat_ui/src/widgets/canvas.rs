use crate::{Event, Layout, Limits, Rectangle, Renderer, Widget};

/// A trait for custom canvas drawing programs.
pub trait Program<Message> {
    /// Handle an event and optionally produce a message.
    fn update(&mut self, event: &Event, bounds: Rectangle) -> Option<Message>;

    /// Draw the canvas content.
    fn draw(&self, renderer: &mut Renderer, bounds: Rectangle);
}

/// A canvas widget for custom drawing.
pub struct Canvas<'a, Message, P: Program<Message>> {
    program: &'a mut P,
    width: Option<f32>,
    height: Option<f32>,
    _phantom: std::marker::PhantomData<Message>,
}

impl<'a, Message, P: Program<Message>> Canvas<'a, Message, P> {
    /// Create a new canvas with a drawing program.
    pub fn new(program: &'a mut P) -> Self {
        Self {
            program,
            width: None,
            height: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the canvas width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the canvas height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
}

impl<'a, Message, P: Program<Message>> Widget<Message> for Canvas<'a, Message, P> {
    fn layout(&self, limits: &Limits) -> Layout {
        let width = self.width.unwrap_or(limits.max_width);
        let height = self.height.unwrap_or(limits.max_height);

        let size = limits.resolve(width, height);
        let bounds = Rectangle::new(0.0, 0.0, size.width, size.height);

        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();
        self.program.draw(renderer, bounds);
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        self.program.update(event, bounds)
    }
}

/// Helper function to create a canvas.
pub fn canvas<'a, Message, P: Program<Message>>(program: &'a mut P) -> Canvas<'a, Message, P> {
    Canvas::new(program)
}
