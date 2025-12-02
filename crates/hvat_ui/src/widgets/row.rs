use crate::{Element, Event, Layout, Limits, Rectangle, Renderer, Widget};

/// A row layout that arranges children horizontally.
pub struct Row<'a, Message> {
    children: Vec<Element<'a, Message>>,
    spacing: f32,
}

impl<'a, Message> Row<'a, Message> {
    /// Create a new row.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            spacing: 0.0,
        }
    }

    /// Create a row with children.
    pub fn with_children(children: Vec<Element<'a, Message>>) -> Self {
        Self {
            children,
            spacing: 0.0,
        }
    }

    /// Add a child element.
    pub fn push(mut self, child: Element<'a, Message>) -> Self {
        self.children.push(child);
        self
    }

    /// Set the spacing between children.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

impl<'a, Message> Widget<Message> for Row<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let mut x = 0.0;
        let mut max_height: f32 = 0.0;
        let mut total_width: f32 = 0.0;

        // Layout all children horizontally
        for (i, child) in self.children.iter().enumerate() {
            let child_layout = child.widget().layout(limits);
            let child_size = child_layout.size();

            total_width += child_size.width;
            max_height = max_height.max(child_size.height);

            if i > 0 {
                total_width += self.spacing;
            }
        }

        let bounds = Rectangle::new(0.0, 0.0, total_width, max_height);
        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();
        let mut x = bounds.x;

        for child in &self.children {
            let child_limits = Limits::fill();
            let child_layout = child.widget().layout(&child_limits);
            let child_size = child_layout.size();

            // Position child at current x
            let child_bounds = Rectangle::new(x, bounds.y, child_size.width, child_size.height);
            let positioned_layout = Layout::new(child_bounds);

            child.widget().draw(renderer, &positioned_layout);

            x += child_size.width + self.spacing;
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let mut x = bounds.x;

        for child in &mut self.children {
            let child_limits = Limits::fill();
            let child_layout = child.widget().layout(&child_limits);
            let child_size = child_layout.size();

            let child_bounds = Rectangle::new(x, bounds.y, child_size.width, child_size.height);
            let positioned_layout = Layout::new(child_bounds);

            if let Some(message) = child.widget_mut().on_event(event, &positioned_layout) {
                return Some(message);
            }

            x += child_size.width + self.spacing;
        }

        None
    }
}

/// Helper function to create a row.
pub fn row<'a, Message>() -> Row<'a, Message> {
    Row::new()
}
