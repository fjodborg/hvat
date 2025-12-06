use crate::{Color, ConcreteSize, ConcreteSizeXY, Element, Event, Layout, Limits, Rectangle, Renderer, Widget};

/// A container widget that wraps a single child with optional padding and background color.
pub struct Container<'a, Message> {
    child: Element<'a, Message>,
    padding: f32,
    background: Option<Color>,
    border_color: Option<Color>,
    border_width: f32,
    /// Whether to fill available space (true) or size to content (false)
    fill: bool,
}

impl<'a, Message> Container<'a, Message> {
    /// Create a new container with a child element.
    pub fn new(child: Element<'a, Message>) -> Self {
        Self {
            child,
            padding: 0.0,
            background: None,
            border_color: None,
            border_width: 1.0,
            fill: false,
        }
    }

    /// Set the padding.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set the background color.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Set the border color (enables border).
    pub fn border(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Set the border width (default: 1.0).
    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width;
        self
    }

    /// Make the container fill all available space.
    /// By default, containers size to their content.
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }
}

impl<'a, Message> Widget<Message> for Container<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Calculate child limits (accounting for padding)
        let child_max_width = if limits.max_width.is_finite() {
            (limits.max_width - self.padding * 2.0).max(0.0)
        } else {
            f32::INFINITY
        };
        let child_max_height = if limits.max_height.is_finite() {
            (limits.max_height - self.padding * 2.0).max(0.0)
        } else {
            f32::INFINITY
        };

        let child_limits = Limits::with_range(0.0, child_max_width, 0.0, child_max_height);
        let child_layout = self.child.widget().layout(&child_limits);
        let child_size = child_layout.size();

        // Container size depends on fill mode
        // Note: When fill=true and we have a border, we need to leave room for the border
        // so it doesn't get clipped by parent containers
        let border_inset = if self.border_color.is_some() { self.border_width } else { 0.0 };

        let (width, height) = if self.fill {
            // Fill mode: use all available space (up to limits), minus border inset
            let w = if limits.max_width.is_finite() {
                (limits.max_width - border_inset).max(0.0)
            } else {
                child_size.width + self.padding * 2.0
            };
            let h = if limits.max_height.is_finite() {
                (limits.max_height - border_inset).max(0.0)
            } else {
                child_size.height + self.padding * 2.0
            };
            (w, h)
        } else {
            // Content mode: size to child + padding, capped by limits
            let w = (child_size.width + self.padding * 2.0).min(limits.max_width);
            let h = (child_size.height + self.padding * 2.0).min(limits.max_height);
            (w, h)
        };

        let bounds = Rectangle::new(0.0, 0.0, width, height);

        // Report fill intent
        if self.fill {
            Layout::fill_both(bounds)
        } else {
            Layout::new(bounds)
        }
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        log::trace!(
            "📦 Container draw: bounds={{x:{:.1}, y:{:.1}, w:{:.1}, h:{:.1}}}, padding={}, fill={}",
            bounds.x, bounds.y, bounds.width, bounds.height, self.padding, self.fill
        );

        // Draw background if specified
        if let Some(color) = self.background {
            renderer.fill_rect(bounds, color);
        }

        // Draw border if specified
        if let Some(color) = self.border_color {
            renderer.stroke_rect(bounds, color, self.border_width);
        }

        // Draw child with offset for padding (ensure non-negative dimensions)
        let child_width = (bounds.width - self.padding * 2.0).max(0.0);
        let child_height = (bounds.height - self.padding * 2.0).max(0.0);
        let child_bounds = Rectangle::new(
            bounds.x + self.padding,
            bounds.y + self.padding,
            child_width,
            child_height,
        );
        let child_layout = Layout::new(child_bounds);
        self.child.widget().draw(renderer, &child_layout);
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let child_width = (bounds.width - self.padding * 2.0).max(0.0);
        let child_height = (bounds.height - self.padding * 2.0).max(0.0);
        let child_bounds = Rectangle::new(
            bounds.x + self.padding,
            bounds.y + self.padding,
            child_width,
            child_height,
        );
        let child_layout = Layout::new(child_bounds);
        self.child.widget_mut().on_event(event, &child_layout)
    }

    fn natural_size(&self, max_width: ConcreteSize) -> ConcreteSizeXY {
        // For fill containers, return minimum_size (parent will distribute space)
        if self.fill {
            return self.minimum_size();
        }

        // Calculate child max width (accounting for padding)
        let padding = self.padding * 2.0;
        let child_max = ConcreteSize::new_unchecked((max_width.get() - padding).max(0.0));

        let child_size = self.child.widget().natural_size(child_max);

        ConcreteSizeXY::new(
            child_size.width + ConcreteSize::new_unchecked(padding),
            child_size.height + ConcreteSize::new_unchecked(padding),
        )
    }

    fn minimum_size(&self) -> ConcreteSizeXY {
        let padding = ConcreteSize::new_unchecked(self.padding * 2.0);
        let child_min = self.child.widget().minimum_size();

        ConcreteSizeXY::new(
            child_min.width + padding,
            child_min.height + padding,
        )
    }
}

/// Helper function to create a container.
pub fn container<'a, Message>(child: Element<'a, Message>) -> Container<'a, Message> {
    Container::new(child)
}
