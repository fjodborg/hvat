use crate::{ConcreteSize, ConcreteSizeXY, Event, Layout, Limits, Renderer};

/// The Widget trait defines the behavior of all UI widgets.
///
/// Widgets are the building blocks of the UI. They:
/// - Calculate their layout based on size constraints
/// - Draw themselves using the renderer
/// - Handle events and optionally produce messages
/// - Report their natural and minimum sizes for layout calculations
pub trait Widget<Message> {
    /// Calculate the layout for this widget given size constraints.
    fn layout(&self, limits: &Limits) -> Layout;

    /// Draw the widget using the renderer at the given layout.
    fn draw(&self, renderer: &mut Renderer, layout: &Layout);

    /// Handle an event and optionally produce a message.
    /// Returns Some(message) if the event should trigger an update.
    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let _ = (event, layout);
        None
    }

    /// What size does this widget prefer given a maximum width constraint?
    ///
    /// This method MUST return finite values (enforced by ConcreteSize type).
    /// Fill widgets should return their `minimum_size()` from this method,
    /// as the parent will distribute remaining space proportionally.
    ///
    /// # Arguments
    /// * `max_width` - The maximum width available for this widget
    ///
    /// # Returns
    /// The natural (preferred) size of this widget. For text, this is the
    /// measured text size. For images, the intrinsic dimensions. For Fill
    /// widgets, the minimum size.
    fn natural_size(&self, max_width: ConcreteSize) -> ConcreteSizeXY {
        // Default implementation: derive from layout() for backward compatibility
        let limits = Limits::with_range(0.0, max_width.get(), 0.0, f32::INFINITY);
        let layout = self.layout(&limits);
        let size = layout.size();
        ConcreteSizeXY::from_f32(
            if size.width.is_finite() { size.width } else { 0.0 },
            if size.height.is_finite() { size.height } else { 0.0 },
        )
    }

    /// Absolute minimum size before the widget breaks.
    ///
    /// This represents the smallest size at which the widget can still
    /// function and display correctly. For example:
    /// - A slider needs at least thumb width + some track
    /// - A button needs enough space for its icon/text
    /// - A scrollable can shrink to zero (it just shows scrollbars)
    ///
    /// Default: zero (widget can shrink to nothing).
    fn minimum_size(&self) -> ConcreteSizeXY {
        ConcreteSizeXY::ZERO
    }

    /// Can this widget shrink below its natural_size?
    ///
    /// Default: true (widget is flexible).
    /// Override to return false for widgets with fixed dimensions.
    fn is_shrinkable(&self) -> bool {
        true
    }
}
