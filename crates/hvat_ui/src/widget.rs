use crate::{Event, Layout, Limits, Renderer};

/// The Widget trait defines the behavior of all UI widgets.
///
/// Widgets are the building blocks of the UI. They:
/// - Calculate their layout based on size constraints
/// - Draw themselves using the renderer
/// - Handle events and optionally produce messages
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
}
