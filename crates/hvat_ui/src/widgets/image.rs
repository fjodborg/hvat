use crate::{ConcreteSize, ConcreteSizeXY, Event, ImageHandle, Layout, Length, Limits, Rectangle, Renderer, Widget};

/// An image widget that displays a texture.
pub struct Image {
    handle: ImageHandle,
    width: Length,
    height: Length,
}

impl Image {
    /// Create a new image widget.
    pub fn new(handle: ImageHandle) -> Self {
        Self {
            handle,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    /// Set the image width.
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Set the image height.
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }
}

impl<Message> Widget<Message> for Image {
    fn layout(&self, limits: &Limits) -> Layout {
        // Intrinsic size is the actual image dimensions
        let intrinsic_width = self.handle.width() as f32;
        let intrinsic_height = self.handle.height() as f32;

        // Resolve width and height based on length specifications
        let width = self.width.resolve(limits.max_width, intrinsic_width);
        let height = self.height.resolve(limits.max_height, intrinsic_height);

        // If only one dimension is specified, maintain aspect ratio
        let (final_width, final_height) = match (self.width, self.height) {
            (Length::Shrink, Length::Shrink) => {
                // Both shrink: use intrinsic size, clamped to limits
                (
                    intrinsic_width.min(limits.max_width),
                    intrinsic_height.min(limits.max_height),
                )
            }
            (Length::Shrink, _) => {
                // Height specified, width shrinks: maintain aspect ratio
                let calculated_width = height * self.handle.aspect_ratio();
                (calculated_width.min(limits.max_width), height)
            }
            (_, Length::Shrink) => {
                // Width specified, height shrinks: maintain aspect ratio
                let calculated_height = width / self.handle.aspect_ratio();
                (width, calculated_height.min(limits.max_height))
            }
            _ => {
                // Both specified: use as-is
                (width, height)
            }
        };

        let size = limits.resolve(final_width, final_height);
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

        // Upload texture and draw it
        renderer.draw_image(&self.handle, bounds);
    }

    fn on_event(&mut self, _event: &Event, _layout: &Layout) -> Option<Message> {
        None // Basic image doesn't handle events
    }

    fn natural_size(&self, max_width: ConcreteSize) -> ConcreteSizeXY {
        let intrinsic_width = self.handle.width() as f32;
        let intrinsic_height = self.handle.height() as f32;

        // For Fill dimensions, return minimum size (32x32)
        match (self.width, self.height) {
            (Length::Fill, _) | (_, Length::Fill) | (Length::FillPortion(_), _) | (_, Length::FillPortion(_)) => {
                return ConcreteSizeXY::from_f32(32.0, 32.0);
            }
            _ => {}
        }

        // Calculate size based on Length specifications
        let (width, height) = match (self.width, self.height) {
            (Length::Shrink, Length::Shrink) => {
                (intrinsic_width.min(max_width.get()), intrinsic_height)
            }
            (Length::Units(w), Length::Units(h)) => (w, h),
            (Length::Units(w), Length::Shrink) => {
                let h = w / self.handle.aspect_ratio();
                (w, h)
            }
            (Length::Shrink, Length::Units(h)) => {
                let w = h * self.handle.aspect_ratio();
                (w.min(max_width.get()), h)
            }
            _ => (intrinsic_width, intrinsic_height),
        };

        ConcreteSizeXY::from_f32(width, height)
    }

    fn minimum_size(&self) -> ConcreteSizeXY {
        // Image can shrink to a small thumbnail
        ConcreteSizeXY::from_f32(32.0, 32.0)
    }
}

/// Helper function to create an image.
pub fn image(handle: ImageHandle) -> Image {
    Image::new(handle)
}
