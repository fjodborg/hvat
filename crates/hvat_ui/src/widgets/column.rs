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

        log::debug!(
            "📦 Column draw: bounds={{x:{:.1}, y:{:.1}, w:{:.1}, h:{:.1}}}",
            bounds.x, bounds.y, bounds.width, bounds.height
        );

        // First pass: calculate sizes and find "fill" children (height = 0)
        let mut child_sizes: Vec<(f32, bool)> = Vec::new(); // (height, is_fill)
        let mut total_fixed_height = 0.0;
        let mut fill_count = 0;

        for (i, child) in self.children.iter().enumerate() {
            let child_limits = Limits::with_range(0.0, bounds.width, 0.0, f32::INFINITY);
            let child_layout = child.widget().layout(&child_limits);
            let child_height = child_layout.size().height;

            if child_height == 0.0 {
                // This child wants to fill remaining space
                child_sizes.push((0.0, true));
                fill_count += 1;
            } else {
                child_sizes.push((child_height, false));
                total_fixed_height += child_height;
            }

            if i > 0 {
                total_fixed_height += self.spacing;
            }
        }

        // Calculate fill height
        // If bounds.height is infinite, fill children get 0 height (can't fill infinite space)
        let fill_height = if fill_count > 0 && bounds.height.is_finite() {
            let remaining_for_fill = (bounds.height - total_fixed_height).max(0.0);
            remaining_for_fill / fill_count as f32
        } else {
            0.0
        };

        // Second pass: position children
        let mut child_layouts: Vec<(Rectangle, &Element<'a, Message>)> = Vec::new();
        let mut y = bounds.y;

        for (i, child) in self.children.iter().enumerate() {
            if i > 0 {
                y += self.spacing;
            }

            let (height, is_fill) = child_sizes[i];
            let actual_height = if is_fill { fill_height } else { height };

            let child_bounds = Rectangle::new(bounds.x, y, bounds.width, actual_height);
            child_layouts.push((child_bounds, child));

            y += actual_height;
        }

        // Draw in reverse order so first child (e.g., header) draws on top
        for (child_bounds, child) in child_layouts.into_iter().rev() {
            let positioned_layout = Layout::new(child_bounds);
            child.widget().draw(renderer, &positioned_layout);
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();

        // First pass: calculate sizes and find "fill" children (height = 0)
        let mut child_sizes: Vec<(f32, bool)> = Vec::new();
        let mut total_fixed_height = 0.0;
        let mut fill_count = 0;

        for (i, child) in self.children.iter().enumerate() {
            let child_limits = Limits::with_range(0.0, bounds.width, 0.0, f32::INFINITY);
            let child_layout = child.widget().layout(&child_limits);
            let child_height = child_layout.size().height;

            if child_height == 0.0 {
                child_sizes.push((0.0, true));
                fill_count += 1;
            } else {
                child_sizes.push((child_height, false));
                total_fixed_height += child_height;
            }

            if i > 0 {
                total_fixed_height += self.spacing;
            }
        }

        // Calculate fill height
        // If bounds.height is infinite, fill children get 0 height (can't fill infinite space)
        let fill_height = if fill_count > 0 && bounds.height.is_finite() {
            let remaining_for_fill = (bounds.height - total_fixed_height).max(0.0);
            remaining_for_fill / fill_count as f32
        } else {
            0.0
        };

        // Second pass: handle events
        let mut y = bounds.y;

        for (i, child) in self.children.iter_mut().enumerate() {
            if i > 0 {
                y += self.spacing;
            }

            let (height, is_fill) = child_sizes[i];
            let actual_height = if is_fill { fill_height } else { height };

            let child_bounds = Rectangle::new(bounds.x, y, bounds.width, actual_height);
            let positioned_layout = Layout::new(child_bounds);

            if let Some(message) = child.widget_mut().on_event(event, &positioned_layout) {
                return Some(message);
            }

            y += actual_height;
        }

        None
    }
}

/// Helper function to create a column.
pub fn column<'a, Message>() -> Column<'a, Message> {
    Column::new()
}
