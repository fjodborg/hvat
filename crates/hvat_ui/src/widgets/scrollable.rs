//! Scrollable container widget that allows vertical scrolling via scrollbar.

use crate::{Element, Event, Layout, Limits, MouseButton, Point, Rectangle, Renderer, Widget, Color};

/// Width of the scrollbar track
const SCROLLBAR_WIDTH: f32 = 12.0;
/// Padding around scrollbar
const SCROLLBAR_PADDING: f32 = 2.0;
/// Total space reserved for scrollbar area
const SCROLLBAR_AREA: f32 = SCROLLBAR_WIDTH + SCROLLBAR_PADDING * 2.0;

/// A scrollable container that wraps a single child and allows vertical scrolling via scrollbar.
/// Mouse wheel events pass through to children (for zoom support).
pub struct Scrollable<'a, Message> {
    child: Element<'a, Message>,
    /// Current scroll offset (positive = scrolled down)
    scroll_offset: f32,
    /// Height of the viewport (set via builder)
    height: Option<f32>,
    /// Width of the viewport (set via builder)
    width: Option<f32>,
    /// Whether the scrollbar is currently being dragged
    is_dragging: bool,
    /// Callback when scroll offset changes
    on_scroll: Option<Box<dyn Fn(f32) -> Message>>,
    /// Callback when scrollbar drag starts
    on_drag_start: Option<Box<dyn Fn() -> Message>>,
    /// Callback when scrollbar drag ends
    on_drag_end: Option<Box<dyn Fn() -> Message>>,
}

impl<'a, Message> Scrollable<'a, Message> {
    /// Create a new scrollable container with a child element.
    pub fn new(child: Element<'a, Message>) -> Self {
        Self {
            child,
            scroll_offset: 0.0,
            height: None,
            width: None,
            is_dragging: false,
            on_scroll: None,
            on_drag_start: None,
            on_drag_end: None,
        }
    }

    /// Set the scroll offset (from external state).
    pub fn scroll_offset(mut self, offset: f32) -> Self {
        self.scroll_offset = offset;
        self
    }

    /// Set the viewport height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set the viewport width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set whether scrollbar is being dragged (from external state).
    pub fn dragging(mut self, is_dragging: bool) -> Self {
        self.is_dragging = is_dragging;
        self
    }

    /// Set the callback when scroll offset changes.
    pub fn on_scroll<F>(mut self, f: F) -> Self
    where
        F: Fn(f32) -> Message + 'static,
    {
        self.on_scroll = Some(Box::new(f));
        self
    }

    /// Set the callback when scrollbar drag starts.
    pub fn on_drag_start<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'static,
    {
        self.on_drag_start = Some(Box::new(f));
        self
    }

    /// Set the callback when scrollbar drag ends.
    pub fn on_drag_end<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'static,
    {
        self.on_drag_end = Some(Box::new(f));
        self
    }
}

impl<'a, Message: Clone> Widget<Message> for Scrollable<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Use specified dimensions or limits
        let viewport_height = self.height.unwrap_or(limits.max_height);
        let viewport_width = self.width.unwrap_or(limits.max_width);

        // Include scrollbar area in our width
        let bounds = Rectangle::new(0.0, 0.0, viewport_width, viewport_height);
        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();

        // Calculate content area width (excluding scrollbar)
        let content_width = bounds.width - SCROLLBAR_AREA;

        // Get the content size - use unconstrained height so content can be taller than viewport
        let content_limits = Limits::with_range(0.0, content_width, 0.0, 100000.0);
        let content_layout = self.child.widget().layout(&content_limits);
        let content_height = content_layout.size().height;

        // Calculate visible area with scroll offset
        let visible_bounds = Rectangle::new(
            bounds.x,
            bounds.y - self.scroll_offset,
            content_width,
            content_height,
        );
        let visible_layout = Layout::new(visible_bounds);

        // Draw the child
        self.child.widget().draw(renderer, &visible_layout);

        // Scrollbar position - at the right edge of our bounds
        let scrollbar_x = bounds.x + bounds.width - SCROLLBAR_WIDTH - SCROLLBAR_PADDING;

        // Track background - always visible
        let track_bounds = Rectangle::new(
            scrollbar_x,
            bounds.y,
            SCROLLBAR_WIDTH,
            bounds.height,
        );
        renderer.fill_rect(track_bounds, Color::rgb(0.25, 0.25, 0.25));

        // Draw scrollbar thumb if content is larger than viewport
        if content_height > bounds.height {
            // Thumb size proportional to visible content
            let thumb_height = (bounds.height / content_height * bounds.height).max(30.0);
            let max_scroll = content_height - bounds.height;
            let scroll_ratio = if max_scroll > 0.0 {
                self.scroll_offset / max_scroll
            } else {
                0.0
            };
            let thumb_y = bounds.y + scroll_ratio * (bounds.height - thumb_height);

            // Thumb color changes when dragging
            let thumb_color = if self.is_dragging {
                Color::rgb(0.7, 0.7, 0.7)
            } else {
                Color::rgb(0.45, 0.45, 0.45)
            };

            let thumb_bounds = Rectangle::new(
                scrollbar_x,
                thumb_y,
                SCROLLBAR_WIDTH,
                thumb_height,
            );
            renderer.fill_rect(thumb_bounds, thumb_color);
        } else {
            // Content fits - draw full-size thumb to indicate no scrolling needed
            let thumb_bounds = Rectangle::new(
                scrollbar_x,
                bounds.y,
                SCROLLBAR_WIDTH,
                bounds.height,
            );
            renderer.fill_rect(thumb_bounds, Color::rgb(0.35, 0.35, 0.35));
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();

        // Calculate content area width (excluding scrollbar)
        let content_width = bounds.width - SCROLLBAR_AREA;

        // Get content height - use unconstrained height so content can be taller than viewport
        let content_limits = Limits::with_range(0.0, content_width, 0.0, 100000.0);
        let content_layout = self.child.widget().layout(&content_limits);
        let content_height = content_layout.size().height;

        // Scrollbar hit area
        let scrollbar_x = bounds.x + bounds.width - SCROLLBAR_WIDTH - SCROLLBAR_PADDING;
        let scrollbar_hit_bounds = Rectangle::new(
            scrollbar_x - 4.0, // Slightly larger hit area
            bounds.y,
            SCROLLBAR_WIDTH + 8.0,
            bounds.height,
        );

        let max_scroll = (content_height - bounds.height).max(0.0);

        // Helper to create child layout
        let make_child_layout = |scroll_offset: f32| {
            let visible_bounds = Rectangle::new(
                bounds.x,
                bounds.y - scroll_offset,
                content_width,
                content_height,
            );
            Layout::new(visible_bounds)
        };

        match event {
            Event::MousePressed { button: MouseButton::Left, position } => {
                // Check if click is on scrollbar track
                if scrollbar_hit_bounds.contains(*position) && content_height > bounds.height {
                    // Calculate thumb position to determine if click is on thumb or track
                    let thumb_height = (bounds.height / content_height * bounds.height).max(30.0);
                    let scroll_ratio = if max_scroll > 0.0 {
                        self.scroll_offset / max_scroll
                    } else {
                        0.0
                    };
                    let thumb_y = bounds.y + scroll_ratio * (bounds.height - thumb_height);
                    let thumb_end_y = thumb_y + thumb_height;

                    // Check if click is on the thumb itself
                    let on_thumb = position.y >= thumb_y && position.y <= thumb_end_y;

                    if on_thumb {
                        // Clicking on thumb: just start dragging, don't jump
                        log::debug!("📜 Scrollbar thumb grab at ({:.1}, {:.1})", position.x, position.y);
                        if let Some(ref on_drag_start) = self.on_drag_start {
                            return Some(on_drag_start());
                        }
                    } else {
                        // Clicking on track: jump to that position and start dragging
                        // Calculate where the center of the thumb should be at click position
                        let click_in_track = position.y - bounds.y - thumb_height / 2.0;
                        let track_range = bounds.height - thumb_height;
                        let click_ratio = (click_in_track / track_range).clamp(0.0, 1.0);
                        let new_offset = (click_ratio * max_scroll).clamp(0.0, max_scroll);

                        log::debug!("📜 Scrollbar track click at ({:.1}, {:.1}), new_offset={:.1}, max_scroll={:.1}",
                            position.x, position.y, new_offset, max_scroll);

                        // Emit scroll position change
                        if let Some(ref on_scroll) = self.on_scroll {
                            return Some(on_scroll(new_offset));
                        }
                    }
                    return None;
                }

                // Pass to child (click is in content area)
                if bounds.contains(*position) && position.x < scrollbar_x - 4.0 {
                    let transformed = Event::MousePressed {
                        button: MouseButton::Left,
                        position: Point::new(position.x, position.y + self.scroll_offset),
                    };
                    let child_layout = make_child_layout(self.scroll_offset);
                    return self.child.widget_mut().on_event(&transformed, &child_layout);
                }
                None
            }
            Event::MouseReleased { button: MouseButton::Left, position } => {
                if self.is_dragging {
                    log::debug!("📜 Scrollbar drag ended");
                    if let Some(ref on_drag_end) = self.on_drag_end {
                        return Some(on_drag_end());
                    }
                }

                // Pass to child
                let transformed = Event::MouseReleased {
                    button: MouseButton::Left,
                    position: Point::new(position.x, position.y + self.scroll_offset),
                };
                let child_layout = make_child_layout(self.scroll_offset);
                return self.child.widget_mut().on_event(&transformed, &child_layout);
            }
            Event::MouseMoved { position } => {
                if self.is_dragging && content_height > bounds.height {
                    // Calculate scroll from mouse Y position
                    let thumb_height = (bounds.height / content_height * bounds.height).max(30.0);
                    let track_range = bounds.height - thumb_height;
                    let mouse_ratio = ((position.y - bounds.y - thumb_height / 2.0) / track_range).clamp(0.0, 1.0);
                    let new_offset = (mouse_ratio * max_scroll).clamp(0.0, max_scroll);

                    log::trace!("📜 Scrollbar drag: mouse_y={:.1}, ratio={:.2}, offset={:.1}/{:.1}",
                        position.y, mouse_ratio, new_offset, max_scroll);

                    if let Some(ref on_scroll) = self.on_scroll {
                        return Some(on_scroll(new_offset));
                    }
                }

                // Pass to child
                let transformed = Event::MouseMoved {
                    position: Point::new(position.x, position.y + self.scroll_offset),
                };
                let child_layout = make_child_layout(self.scroll_offset);
                return self.child.widget_mut().on_event(&transformed, &child_layout);
            }
            Event::MouseWheel { .. } => {
                // Pass mouse wheel to children (for zoom support)
                let child_layout = make_child_layout(self.scroll_offset);
                return self.child.widget_mut().on_event(event, &child_layout);
            }
            _ => {
                // Pass other events to child
                let child_layout = make_child_layout(self.scroll_offset);
                return self.child.widget_mut().on_event(event, &child_layout);
            }
        }
    }
}

/// Helper function to create a scrollable container.
pub fn scrollable<'a, Message>(child: Element<'a, Message>) -> Scrollable<'a, Message> {
    Scrollable::new(child)
}
