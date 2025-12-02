use crate::{Element, Event, Layout, Limits, Rectangle, Renderer, Widget};

/// A column layout that arranges children vertically.
pub struct Column<'a, Message> {
    children: Vec<Element<'a, Message>>,
    spacing: f32,
}

impl<'a, Message> Column<'a, Message> {
    /// Create a new column.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            spacing: 0.0,
        }
    }

    /// Create a column with children.
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

impl<'a, Message> Widget<Message> for Column<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let mut max_width: f32 = 0.0;
        let mut total_height: f32 = 0.0;

        // Layout all children vertically
        for (i, child) in self.children.iter().enumerate() {
            let child_layout = child.widget().layout(limits);
            let child_size = child_layout.size();

            total_height += child_size.height;
            max_width = max_width.max(child_size.width);

            if i > 0 {
                total_height += self.spacing;
            }
        }

        let bounds = Rectangle::new(0.0, 0.0, max_width, total_height);
        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();
        let mut y = bounds.y;

        for child in &self.children {
            let child_limits = Limits::fill();
            let child_layout = child.widget().layout(&child_limits);
            let child_size = child_layout.size();

            // Position child at current y
            let child_bounds = Rectangle::new(bounds.x, y, child_size.width, child_size.height);
            let positioned_layout = Layout::new(child_bounds);

            child.widget().draw(renderer, &positioned_layout);

            y += child_size.height + self.spacing;
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let mut y = bounds.y;

        for child in &mut self.children {
            let child_limits = Limits::fill();
            let child_layout = child.widget().layout(&child_limits);
            let child_size = child_layout.size();

            let child_bounds = Rectangle::new(bounds.x, y, child_size.width, child_size.height);
            let positioned_layout = Layout::new(child_bounds);

            if let Some(message) = child.widget_mut().on_event(event, &positioned_layout) {
                return Some(message);
            }

            y += child_size.height + self.spacing;
        }

        None
    }
}

/// Helper function to create a column.
pub fn column<'a, Message>() -> Column<'a, Message> {
    Column::new()
}
