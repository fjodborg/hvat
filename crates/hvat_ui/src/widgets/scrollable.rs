//! Scrollable container widget that allows vertical scrolling via scrollbar.
//!
//! This widget consists of:
//! - A content viewport with clipping
//! - A scrollbar with track and thumb
//! - Coordinate transformation for events

use crate::{Element, Event, Layout, Limits, MouseButton, Point, Rectangle, Renderer, Widget};
use super::config::ScrollbarConfig;

/// Default scrollbar configuration (used if not overridden).
fn default_scrollbar_config() -> ScrollbarConfig {
    ScrollbarConfig::default()
}

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
    /// Scrollbar appearance configuration
    scrollbar_config: ScrollbarConfig,
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
            scrollbar_config: default_scrollbar_config(),
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

    /// Set the scrollbar configuration.
    pub fn scrollbar_config(mut self, config: ScrollbarConfig) -> Self {
        self.scrollbar_config = config;
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

    // === Helper methods for scrollbar calculations ===

    /// Get the total area reserved for the scrollbar.
    fn scrollbar_area(&self) -> f32 {
        self.scrollbar_config.total_area()
    }

    /// Calculate thumb height based on viewport and content sizes.
    fn thumb_height(&self, viewport_height: f32, content_height: f32) -> f32 {
        (viewport_height / content_height * viewport_height).max(self.scrollbar_config.min_thumb_height)
    }

    /// Calculate thumb Y position based on scroll offset.
    fn thumb_y(&self, viewport_y: f32, viewport_height: f32, thumb_height: f32, max_scroll: f32) -> f32 {
        let scroll_ratio = if max_scroll > 0.0 {
            self.scroll_offset / max_scroll
        } else {
            0.0
        };
        viewport_y + scroll_ratio * (viewport_height - thumb_height)
    }

    /// Calculate scroll offset from mouse Y position during drag.
    fn scroll_from_mouse_y(&self, mouse_y: f32, viewport_y: f32, viewport_height: f32, thumb_height: f32, max_scroll: f32) -> f32 {
        let track_range = viewport_height - thumb_height;
        let mouse_ratio = ((mouse_y - viewport_y - thumb_height / 2.0) / track_range).clamp(0.0, 1.0);
        (mouse_ratio * max_scroll).clamp(0.0, max_scroll)
    }

    /// Get scrollbar X position.
    fn scrollbar_x(&self, viewport_x: f32, viewport_width: f32) -> f32 {
        viewport_x + viewport_width - self.scrollbar_config.width - self.scrollbar_config.padding
    }

    /// Create scrollbar hit bounds (slightly larger for easier clicking).
    fn scrollbar_hit_bounds(&self, viewport: &Rectangle) -> Rectangle {
        let x = self.scrollbar_x(viewport.x, viewport.width);
        Rectangle::new(
            x - 4.0,
            viewport.y,
            self.scrollbar_config.width + 8.0,
            viewport.height,
        )
    }

    /// Draw the scrollbar (track and thumb).
    fn draw_scrollbar(&self, renderer: &mut Renderer, viewport: &Rectangle, content_height: f32) {
        let scrollbar_x = self.scrollbar_x(viewport.x, viewport.width);
        let config = &self.scrollbar_config;

        // Track background
        let track_bounds = Rectangle::new(
            scrollbar_x,
            viewport.y,
            config.width,
            viewport.height,
        );
        renderer.fill_rect(track_bounds, config.track_color);

        // Thumb
        let thumb_height = self.thumb_height(viewport.height, content_height);
        let max_scroll = content_height - viewport.height;
        let thumb_y = self.thumb_y(viewport.y, viewport.height, thumb_height, max_scroll);

        let thumb_color = if self.is_dragging {
            config.thumb_active_color
        } else {
            config.thumb_color
        };

        let thumb_bounds = Rectangle::new(scrollbar_x, thumb_y, config.width, thumb_height);
        renderer.fill_rect(thumb_bounds, thumb_color);
    }
}

impl<'a, Message: Clone> Widget<Message> for Scrollable<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Use specified dimensions or limits
        // For height: if not specified, report 0 to indicate "fill remaining space"
        // The parent container will give us actual height during draw()
        let viewport_height = self.height.unwrap_or(0.0);
        let viewport_width = self.width.unwrap_or(limits.max_width);

        // Include scrollbar area in our width
        let bounds = Rectangle::new(0.0, 0.0, viewport_width, viewport_height);
        Layout::new(bounds)
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();
        let scrollbar_area = self.scrollbar_area();

        log::debug!(
            "📜 Scrollable draw: bounds={{x:{:.1}, y:{:.1}, w:{:.1}, h:{:.1}}}, scroll_offset={:.1}",
            bounds.x, bounds.y, bounds.width, bounds.height, self.scroll_offset
        );

        // Determine if scrollbar is needed
        let needs_scrollbar = {
            let content_width = bounds.width - scrollbar_area;
            let content_limits = Limits::with_range(0.0, content_width, 0.0, 100000.0);
            let content_layout = self.child.widget().layout(&content_limits);
            content_layout.size().height > bounds.height
        };

        // Calculate content area width (exclude scrollbar area only if needed)
        let content_width = if needs_scrollbar {
            bounds.width - scrollbar_area
        } else {
            bounds.width
        };

        // Get the content size
        let content_limits = Limits::with_range(0.0, content_width, 0.0, 100000.0);
        let content_layout = self.child.widget().layout(&content_limits);
        let content_height = content_layout.size().height;

        // Push clip and scroll offset
        let clip_bounds = Rectangle::new(bounds.x, bounds.y, content_width, bounds.height);
        renderer.push_clip(clip_bounds);
        renderer.push_scroll_offset(self.scroll_offset);

        // Draw child
        let child_bounds = Rectangle::new(bounds.x, bounds.y, content_width, content_height);
        let child_layout = Layout::new(child_bounds);
        self.child.widget().draw(renderer, &child_layout);

        // Pop scroll offset and clip
        renderer.pop_scroll_offset();
        renderer.pop_clip();

        // Draw scrollbar if needed
        if needs_scrollbar {
            self.draw_scrollbar(renderer, &bounds, content_height);
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let scrollbar_area = self.scrollbar_area();

        // Calculate content dimensions
        let content_width = bounds.width - scrollbar_area;
        let content_limits = Limits::with_range(0.0, content_width, 0.0, 100000.0);
        let content_layout = self.child.widget().layout(&content_limits);
        let content_height = content_layout.size().height;

        let scrollbar_hit_bounds = self.scrollbar_hit_bounds(&bounds);
        let scrollbar_x = self.scrollbar_x(bounds.x, bounds.width);
        let max_scroll = (content_height - bounds.height).max(0.0);

        // Helper to create child layout for event handling
        let make_child_layout = || {
            Layout::new(Rectangle::new(bounds.x, bounds.y, content_width, content_height))
        };

        match event {
            Event::MousePressed { button: MouseButton::Left, position } => {
                // Check if click is on scrollbar track
                if scrollbar_hit_bounds.contains(*position) && content_height > bounds.height {
                    let thumb_height = self.thumb_height(bounds.height, content_height);
                    let thumb_y = self.thumb_y(bounds.y, bounds.height, thumb_height, max_scroll);
                    let thumb_end_y = thumb_y + thumb_height;
                    let on_thumb = position.y >= thumb_y && position.y <= thumb_end_y;

                    if on_thumb {
                        log::debug!("📜 Scrollbar thumb grab at ({:.1}, {:.1})", position.x, position.y);
                        if let Some(ref on_drag_start) = self.on_drag_start {
                            return Some(on_drag_start());
                        }
                    } else {
                        let new_offset = self.scroll_from_mouse_y(position.y, bounds.y, bounds.height, thumb_height, max_scroll);
                        log::debug!("📜 Scrollbar track click at ({:.1}, {:.1}), new_offset={:.1}", position.x, position.y, new_offset);
                        if let Some(ref on_scroll) = self.on_scroll {
                            return Some(on_scroll(new_offset));
                        }
                    }
                    return None;
                }

                // Pass to child (click is in content area)
                if bounds.contains(*position) && position.x < scrollbar_x - 4.0 {
                    let content_y = position.y + self.scroll_offset;
                    let transformed = Event::MousePressed {
                        button: MouseButton::Left,
                        position: Point::new(position.x, content_y),
                    };
                    return self.child.widget_mut().on_event(&transformed, &make_child_layout());
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

                let content_y = position.y + self.scroll_offset;
                let transformed = Event::MouseReleased {
                    button: MouseButton::Left,
                    position: Point::new(position.x, content_y),
                };
                self.child.widget_mut().on_event(&transformed, &make_child_layout())
            }
            Event::MouseMoved { position } => {
                if self.is_dragging && content_height > bounds.height {
                    let thumb_height = self.thumb_height(bounds.height, content_height);
                    let new_offset = self.scroll_from_mouse_y(position.y, bounds.y, bounds.height, thumb_height, max_scroll);
                    log::trace!("📜 Scrollbar drag: mouse_y={:.1}, offset={:.1}/{:.1}", position.y, new_offset, max_scroll);
                    if let Some(ref on_scroll) = self.on_scroll {
                        return Some(on_scroll(new_offset));
                    }
                }

                let content_y = position.y + self.scroll_offset;
                let transformed = Event::MouseMoved {
                    position: Point::new(position.x, content_y),
                };
                self.child.widget_mut().on_event(&transformed, &make_child_layout())
            }
            Event::MouseWheel { .. } => {
                self.child.widget_mut().on_event(event, &make_child_layout())
            }
            _ => {
                self.child.widget_mut().on_event(event, &make_child_layout())
            }
        }
    }
}

/// Helper function to create a scrollable container.
pub fn scrollable<'a, Message>(child: Element<'a, Message>) -> Scrollable<'a, Message> {
    Scrollable::new(child)
}
