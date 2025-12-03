use crate::{Layout, Widget};

/// A unique identifier for a widget in the UI tree.
///
/// Widget IDs are used for:
/// - Tracking which widget is being dragged
/// - Identifying widgets for focus management
/// - Correlating widgets with external state
/// - Supporting drag-and-drop panel rearrangement
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WidgetId(String);

impl WidgetId {
    /// Create a new widget ID from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<S: Into<String>> From<S> for WidgetId {
    fn from(s: S) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An Element is a type-erased widget that can contain any widget type.
///
/// This is the main building block of the UI tree. Elements wrap widgets
/// and handle message type conversion through the Widget trait.
///
/// Elements can optionally have an ID for tracking and identification.
pub struct Element<'a, Message> {
    widget: Box<dyn Widget<Message> + 'a>,
    id: Option<WidgetId>,
}

impl<'a, Message> Element<'a, Message> {
    /// Create a new element from a widget.
    pub fn new(widget: impl Widget<Message> + 'a) -> Self {
        Self {
            widget: Box::new(widget),
            id: None,
        }
    }

    /// Create a new element with an ID.
    pub fn with_id(widget: impl Widget<Message> + 'a, id: impl Into<WidgetId>) -> Self {
        Self {
            widget: Box::new(widget),
            id: Some(id.into()),
        }
    }

    /// Set the ID of this element.
    pub fn id(mut self, id: impl Into<WidgetId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Get the ID of this element, if any.
    pub fn get_id(&self) -> Option<&WidgetId> {
        self.id.as_ref()
    }

    /// Check if this element has a specific ID.
    pub fn has_id(&self, id: &str) -> bool {
        self.id.as_ref().map(|i| i.as_str() == id).unwrap_or(false)
    }

    /// Map the message type of this element to a different type.
    /// Preserves the element's ID if set.
    pub fn map<B>(self, f: impl Fn(Message) -> B + 'static) -> Element<'a, B>
    where
        Message: 'static,
        B: 'static,
    {
        let id = self.id.clone();
        let mut mapped = Element::new(Map {
            element: self,
            mapper: Box::new(f),
        });
        mapped.id = id;
        mapped
    }

    /// Get the widget for layout calculation.
    pub fn widget(&self) -> &dyn Widget<Message> {
        &*self.widget
    }

    /// Get the mutable widget for event handling.
    pub fn widget_mut(&mut self) -> &mut dyn Widget<Message> {
        &mut *self.widget
    }
}

/// A widget that maps messages from one type to another.
struct Map<'a, A, B> {
    element: Element<'a, A>,
    mapper: Box<dyn Fn(A) -> B>,
}

impl<'a, A, B> Widget<B> for Map<'a, A, B>
where
    A: 'static,
    B: 'static,
{
    fn layout(&self, limits: &crate::Limits) -> crate::Layout {
        self.element.widget().layout(limits)
    }

    fn draw(&self, renderer: &mut crate::Renderer, layout: &Layout) {
        self.element.widget().draw(renderer, layout);
    }

    fn on_event(
        &mut self,
        event: &crate::Event,
        layout: &Layout,
    ) -> Option<B> {
        self.element
            .widget_mut()
            .on_event(event, layout)
            .map(&self.mapper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_id_creation() {
        let id1 = WidgetId::new("test-button");
        let id2 = WidgetId::from("test-button");
        let id3: WidgetId = "test-button".into();

        assert_eq!(id1, id2);
        assert_eq!(id2, id3);
        assert_eq!(id1.as_str(), "test-button");
    }

    #[test]
    fn test_widget_id_display() {
        let id = WidgetId::new("my-widget");
        assert_eq!(format!("{}", id), "my-widget");
    }

    #[test]
    fn test_widget_id_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(WidgetId::new("a"));
        set.insert(WidgetId::new("b"));
        set.insert(WidgetId::new("a")); // duplicate

        assert_eq!(set.len(), 2);
    }
}
