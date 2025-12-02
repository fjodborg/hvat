use crate::{Layout, Widget};

/// An Element is a type-erased widget that can contain any widget type.
///
/// This is the main building block of the UI tree. Elements wrap widgets
/// and handle message type conversion through the Widget trait.
pub struct Element<'a, Message> {
    widget: Box<dyn Widget<Message> + 'a>,
}

impl<'a, Message> Element<'a, Message> {
    /// Create a new element from a widget.
    pub fn new(widget: impl Widget<Message> + 'a) -> Self {
        Self {
            widget: Box::new(widget),
        }
    }

    /// Map the message type of this element to a different type.
    pub fn map<B>(self, f: impl Fn(Message) -> B + 'static) -> Element<'a, B>
    where
        Message: 'static,
        B: 'static,
    {
        Element::new(Map {
            element: self,
            mapper: Box::new(f),
        })
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
