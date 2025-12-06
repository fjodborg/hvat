//! Scrollable container widget that allows vertical and/or horizontal scrolling via scrollbars.
//!
//! This widget consists of:
//! - A content viewport with clipping
//! - Vertical and/or horizontal scrollbars with track and thumb
//! - Coordinate transformation for events

use crate::{builder_field, callback_setter, builder_option, ConcreteSize, ConcreteSizeXY, Element, Event, Layout, Limits, MouseButton, Point, Rectangle, Renderer, Widget};
use super::config::{ScrollbarConfig, ScrollDirection};

/// Scrollbar axis for unified drawing.
#[derive(Clone, Copy)]
enum ScrollAxis {
    Vertical,
    Horizontal,
}

/// A scrollable container that wraps a single child and allows scrolling via scrollbars.
/// Supports vertical, horizontal, or both scroll directions.
/// Mouse wheel events pass through to children (for zoom support).
pub struct Scrollable<'a, Message> {
    child: Element<'a, Message>,
    direction: ScrollDirection,
    scroll_offset_y: f32,
    scroll_offset_x: f32,
    height: Option<f32>,
    width: Option<f32>,
    is_dragging_y: bool,
    is_dragging_x: bool,
    drag_start_mouse_y: Option<f32>,
    drag_start_scroll_y: Option<f32>,
    drag_start_mouse_x: Option<f32>,
    drag_start_scroll_x: Option<f32>,
    scrollbar_config: ScrollbarConfig,
    fill_viewport: bool,
    on_scroll_y: Option<Box<dyn Fn(f32) -> Message>>,
    on_scroll_x: Option<Box<dyn Fn(f32) -> Message>>,
    on_drag_start_y: Option<Box<dyn Fn(f32) -> Message>>,
    on_drag_end_y: Option<Box<dyn Fn() -> Message>>,
    on_drag_start_x: Option<Box<dyn Fn(f32) -> Message>>,
    on_drag_end_x: Option<Box<dyn Fn() -> Message>>,
}

impl<'a, Message> Scrollable<'a, Message> {
    /// Create a new scrollable container with a child element.
    pub fn new(child: Element<'a, Message>) -> Self {
        Self {
            child,
            direction: ScrollDirection::Vertical,
            scroll_offset_y: 0.0,
            scroll_offset_x: 0.0,
            height: None,
            width: None,
            is_dragging_y: false,
            is_dragging_x: false,
            drag_start_mouse_y: None,
            drag_start_scroll_y: None,
            drag_start_mouse_x: None,
            drag_start_scroll_x: None,
            scrollbar_config: ScrollbarConfig::default(),
            fill_viewport: false,
            on_scroll_y: None,
            on_scroll_x: None,
            on_drag_start_y: None,
            on_drag_end_y: None,
            on_drag_start_x: None,
            on_drag_end_x: None,
        }
    }

    /// Enable fill_viewport mode - children with Length::Fill will expand to fill the viewport.
    pub fn fill_viewport(mut self) -> Self {
        self.fill_viewport = true;
        self
    }

    // Builder methods using macros
    builder_field!(direction, ScrollDirection);
    builder_field!(scroll_offset_y, f32);
    builder_field!(scroll_offset_x, f32);
    builder_option!(height, f32);
    builder_option!(width, f32);
    builder_field!(scrollbar_config, ScrollbarConfig);

    /// Set whether vertical scrollbar is being dragged (from external state).
    pub fn dragging_y(mut self, is_dragging: bool) -> Self {
        self.is_dragging_y = is_dragging;
        self
    }

    /// Set whether horizontal scrollbar is being dragged (from external state).
    pub fn dragging_x(mut self, is_dragging: bool) -> Self {
        self.is_dragging_x = is_dragging;
        self
    }

    /// Set the vertical drag start position (mouse Y and scroll offset when drag started).
    pub fn drag_start_y(mut self, mouse_y: f32, scroll_offset: f32) -> Self {
        self.drag_start_mouse_y = Some(mouse_y);
        self.drag_start_scroll_y = Some(scroll_offset);
        self
    }

    /// Set the horizontal drag start position (mouse X and scroll offset when drag started).
    pub fn drag_start_x(mut self, mouse_x: f32, scroll_offset: f32) -> Self {
        self.drag_start_mouse_x = Some(mouse_x);
        self.drag_start_scroll_x = Some(scroll_offset);
        self
    }

    // Callback setters using macros
    callback_setter!(on_scroll_y, f32);
    callback_setter!(on_scroll_x, f32);
    callback_setter!(on_drag_start_y, f32);
    callback_setter!(on_drag_start_x, f32);
    callback_setter!(on_drag_end_y);
    callback_setter!(on_drag_end_x);

    // === Helper methods for scrollbar calculations ===

    fn scrollbar_area(&self) -> f32 {
        self.scrollbar_config.total_area()
    }

    /// Calculate thumb size for given viewport and content sizes.
    fn thumb_size(&self, viewport_size: f32, content_size: f32) -> f32 {
        (viewport_size / content_size * viewport_size).max(self.scrollbar_config.min_thumb_height)
    }

    /// Calculate thumb position for given scroll offset.
    fn thumb_position(&self, scroll_offset: f32, viewport_start: f32, viewport_size: f32, thumb_size: f32, max_scroll: f32) -> f32 {
        let clamped_offset = scroll_offset.clamp(0.0, max_scroll);
        let scroll_ratio = if max_scroll > 0.0 { clamped_offset / max_scroll } else { 0.0 };
        viewport_start + scroll_ratio * (viewport_size - thumb_size)
    }

    /// Get scrollbar position on the cross axis.
    fn scrollbar_cross_pos(&self, viewport_start: f32, viewport_size: f32) -> f32 {
        viewport_start + viewport_size - self.scrollbar_config.width - self.scrollbar_config.padding
    }

    /// Create scrollbar hit bounds (slightly larger for easier clicking).
    fn scrollbar_hit_bounds(&self, viewport: &Rectangle, has_cross_scrollbar: bool, axis: ScrollAxis) -> Rectangle {
        let scrollbar_area = self.scrollbar_area();
        match axis {
            ScrollAxis::Vertical => {
                let x = self.scrollbar_cross_pos(viewport.x, viewport.width);
                let height = if has_cross_scrollbar { viewport.height - scrollbar_area } else { viewport.height };
                Rectangle::new(x - 4.0, viewport.y, self.scrollbar_config.width + 8.0, height)
            }
            ScrollAxis::Horizontal => {
                let y = self.scrollbar_cross_pos(viewport.y, viewport.height);
                let width = if has_cross_scrollbar { viewport.width - scrollbar_area } else { viewport.width };
                Rectangle::new(viewport.x, y - 4.0, width, self.scrollbar_config.width + 8.0)
            }
        }
    }

    /// Draw a scrollbar (unified for both axes).
    fn draw_scrollbar(&self, renderer: &mut Renderer, viewport: &Rectangle, content_size: f32, has_cross_scrollbar: bool, axis: ScrollAxis) {
        let config = &self.scrollbar_config;
        let scrollbar_area = self.scrollbar_area();

        let (track_bounds, thumb_bounds, is_dragging) = match axis {
            ScrollAxis::Vertical => {
                let scrollbar_x = self.scrollbar_cross_pos(viewport.x, viewport.width);
                let track_height = if has_cross_scrollbar { viewport.height - scrollbar_area } else { viewport.height };
                let track = Rectangle::new(scrollbar_x, viewport.y, config.width, track_height);

                let thumb_height = self.thumb_size(track_height, content_size);
                let max_scroll = (content_size - track_height).max(0.0);
                let thumb_y = self.thumb_position(self.scroll_offset_y, viewport.y, track_height, thumb_height, max_scroll);
                let thumb = Rectangle::new(scrollbar_x, thumb_y, config.width, thumb_height);

                (track, thumb, self.is_dragging_y)
            }
            ScrollAxis::Horizontal => {
                let scrollbar_y = self.scrollbar_cross_pos(viewport.y, viewport.height);
                let track_width = if has_cross_scrollbar { viewport.width - scrollbar_area } else { viewport.width };
                let track = Rectangle::new(viewport.x, scrollbar_y, track_width, config.width);

                let thumb_width = self.thumb_size(track_width, content_size);
                let max_scroll = (content_size - track_width).max(0.0);
                let thumb_x = self.thumb_position(self.scroll_offset_x, viewport.x, track_width, thumb_width, max_scroll);
                let thumb = Rectangle::new(thumb_x, scrollbar_y, thumb_width, config.width);

                (track, thumb, self.is_dragging_x)
            }
        };

        renderer.fill_rect(track_bounds, config.track_color);
        let thumb_color = if is_dragging { config.thumb_active_color } else { config.thumb_color };
        renderer.fill_rect(thumb_bounds, thumb_color);
    }

    /// Measure child content using natural_size().
    fn measure_content(&self, viewport_width: f32, _viewport_height: f32) -> crate::Size {
        let max_width = ConcreteSize::new_unchecked(viewport_width);
        let content_size = self.child.widget().natural_size(max_width);
        crate::Size::new(content_size.width.get(), content_size.height.get())
    }
}

impl<'a, Message: Clone> Widget<Message> for Scrollable<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        let viewport_width = self.width.unwrap_or_else(|| {
            if limits.max_width.is_finite() { limits.max_width } else { 300.0 }
        });
        let viewport_height = self.height.unwrap_or(0.0);
        let bounds = Rectangle::new(0.0, 0.0, viewport_width, viewport_height);

        if self.height.is_none() {
            Layout::fill_height(bounds)
        } else {
            Layout::new(bounds)
        }
    }

    fn draw(&self, renderer: &mut Renderer, layout: &Layout) {
        let bounds = layout.bounds();
        let scrollbar_area = self.scrollbar_area();

        log::trace!(
            "📜 Scrollable draw: bounds={{x:{:.1}, y:{:.1}, w:{:.1}, h:{:.1}}}, scroll=({:.1}, {:.1}), dir={:?}",
            bounds.x, bounds.y, bounds.width, bounds.height, self.scroll_offset_x, self.scroll_offset_y, self.direction
        );

        let content_size = self.measure_content(bounds.width, bounds.height);

        log::trace!(
            "📜 Scrollable content_size: w={:.1}, h={:.1}, viewport_h={:.1}, needs_scroll={}",
            content_size.width, content_size.height, bounds.height, content_size.height > bounds.height
        );

        let needs_v_scrollbar = self.direction.has_vertical() && content_size.height > bounds.height;
        let needs_h_scrollbar = self.direction.has_horizontal() && content_size.width > bounds.width;

        let content_width = if needs_v_scrollbar { bounds.width - scrollbar_area } else { bounds.width };
        let content_height = if needs_h_scrollbar { bounds.height - scrollbar_area } else { bounds.height };

        let final_content_size = if needs_v_scrollbar && content_width != bounds.width {
            self.measure_content(content_width, bounds.height)
        } else {
            content_size
        };

        let child_width = if self.fill_viewport { final_content_size.width.max(content_width) } else { final_content_size.width };
        let child_height = if self.fill_viewport { final_content_size.height.max(content_height) } else { final_content_size.height };

        let max_scroll_y = (child_height - content_height).max(0.0);
        let max_scroll_x = (child_width - content_width).max(0.0);
        let clamped_scroll_y = self.scroll_offset_y.clamp(0.0, max_scroll_y);
        let clamped_scroll_x = self.scroll_offset_x.clamp(0.0, max_scroll_x);

        log::trace!(
            "📜 Scrollable scroll: child_h={:.1}, content_h={:.1}, max_scroll_y={:.1}, scroll_offset={:.1}, clamped={:.1}",
            child_height, content_height, max_scroll_y, self.scroll_offset_y, clamped_scroll_y
        );

        let clip_bounds = Rectangle::new(bounds.x, bounds.y, content_width, content_height);
        renderer.push_clip(clip_bounds);
        renderer.push_scroll_offset_y(clamped_scroll_y);
        renderer.push_scroll_offset_x(clamped_scroll_x);

        let child_bounds = Rectangle::new(bounds.x, bounds.y, child_width, child_height);
        let child_layout = Layout::new(child_bounds);
        self.child.widget().draw(renderer, &child_layout);

        renderer.pop_scroll_offset_x();
        renderer.pop_scroll_offset_y();
        renderer.pop_clip();

        if needs_v_scrollbar {
            self.draw_scrollbar(renderer, &bounds, child_height, needs_h_scrollbar, ScrollAxis::Vertical);
        }
        if needs_h_scrollbar {
            self.draw_scrollbar(renderer, &bounds, child_width, needs_v_scrollbar, ScrollAxis::Horizontal);
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let scrollbar_area = self.scrollbar_area();

        let content_size = self.measure_content(bounds.width, bounds.height);

        let needs_v_scrollbar = self.direction.has_vertical() && content_size.height > bounds.height;
        let needs_h_scrollbar = self.direction.has_horizontal() && content_size.width > bounds.width;

        let content_width = if needs_v_scrollbar { bounds.width - scrollbar_area } else { bounds.width };
        let content_height = if needs_h_scrollbar { bounds.height - scrollbar_area } else { bounds.height };

        let final_content_size = if needs_v_scrollbar && content_width != bounds.width {
            self.measure_content(content_width, bounds.height)
        } else {
            content_size
        };

        let child_width = if self.fill_viewport { final_content_size.width.max(content_width) } else { final_content_size.width };
        let child_height = if self.fill_viewport { final_content_size.height.max(content_height) } else { final_content_size.height };

        let max_scroll_y = (child_height - content_height).max(0.0);
        let max_scroll_x = (child_width - content_width).max(0.0);

        if needs_v_scrollbar {
            log::trace!(
                "📜 Scrollable on_event: child_h={:.1}, content_h={:.1}, max_scroll_y={:.1}, current_offset={:.1}",
                child_height, content_height, max_scroll_y, self.scroll_offset_y
            );
        }

        // Check if scroll offset needs clamping
        if self.scroll_offset_y > max_scroll_y {
            if let Some(ref on_scroll_y) = self.on_scroll_y {
                return Some(on_scroll_y(max_scroll_y));
            }
        }
        if self.scroll_offset_x > max_scroll_x {
            if let Some(ref on_scroll_x) = self.on_scroll_x {
                return Some(on_scroll_x(max_scroll_x));
            }
        }

        let scrollbar_hit_y = self.scrollbar_hit_bounds(&bounds, needs_h_scrollbar, ScrollAxis::Vertical);
        let scrollbar_hit_x = self.scrollbar_hit_bounds(&bounds, needs_v_scrollbar, ScrollAxis::Horizontal);

        let make_child_layout = || Layout::new(Rectangle::new(bounds.x, bounds.y, child_width, child_height));

        match event {
            Event::MousePressed { button: MouseButton::Left, position } => {
                if needs_v_scrollbar && scrollbar_hit_y.contains(*position) {
                    log::debug!("📜 Vertical scrollbar click - start drag at y={:.1}", position.y);
                    if let Some(ref on_drag_start_y) = self.on_drag_start_y {
                        return Some(on_drag_start_y(position.y));
                    }
                    return None;
                }

                if needs_h_scrollbar && scrollbar_hit_x.contains(*position) {
                    log::debug!("📜 Horizontal scrollbar click - start drag at x={:.1}", position.x);
                    if let Some(ref on_drag_start_x) = self.on_drag_start_x {
                        return Some(on_drag_start_x(position.x));
                    }
                    return None;
                }

                let scrollbar_x = self.scrollbar_cross_pos(bounds.x, bounds.width);
                let scrollbar_y = self.scrollbar_cross_pos(bounds.y, bounds.height);
                let in_content = bounds.contains(*position)
                    && (!needs_v_scrollbar || position.x < scrollbar_x - 4.0)
                    && (!needs_h_scrollbar || position.y < scrollbar_y - 4.0);

                if in_content {
                    let content_pos = Point::new(position.x + self.scroll_offset_x, position.y + self.scroll_offset_y);
                    let transformed = Event::MousePressed { button: MouseButton::Left, position: content_pos };
                    return self.child.widget_mut().on_event(&transformed, &make_child_layout());
                }
                None
            }
            Event::MouseReleased { button: MouseButton::Left, position } => {
                if self.is_dragging_y {
                    log::debug!("📜 Vertical scrollbar drag ended");
                    if let Some(ref on_drag_end_y) = self.on_drag_end_y {
                        return Some(on_drag_end_y());
                    }
                }
                if self.is_dragging_x {
                    log::debug!("📜 Horizontal scrollbar drag ended");
                    if let Some(ref on_drag_end_x) = self.on_drag_end_x {
                        return Some(on_drag_end_x());
                    }
                }

                let content_pos = Point::new(position.x + self.scroll_offset_x, position.y + self.scroll_offset_y);
                let transformed = Event::MouseReleased { button: MouseButton::Left, position: content_pos };
                self.child.widget_mut().on_event(&transformed, &make_child_layout())
            }
            Event::MouseMoved { position } => {
                if self.is_dragging_y && needs_v_scrollbar {
                    if let (Some(start_mouse_y), Some(start_scroll_y)) = (self.drag_start_mouse_y, self.drag_start_scroll_y) {
                        let track_height = if needs_h_scrollbar { bounds.height - scrollbar_area } else { bounds.height };
                        let thumb_height = self.thumb_size(track_height, final_content_size.height);
                        let track_range = track_height - thumb_height;

                        let mouse_delta = position.y - start_mouse_y;
                        let scroll_per_pixel = if track_range > 0.0 { max_scroll_y / track_range } else { 0.0 };
                        let new_offset = (start_scroll_y + mouse_delta * scroll_per_pixel).clamp(0.0, max_scroll_y);

                        log::debug!(
                            "📜 Drag: delta={:.1}, new_offset={:.1}, max={:.1}, track_h={:.1}, thumb_h={:.1}, child_h={:.1}, content_h={:.1}",
                            mouse_delta, new_offset, max_scroll_y, track_height, thumb_height, child_height, content_height
                        );
                        if let Some(ref on_scroll_y) = self.on_scroll_y {
                            return Some(on_scroll_y(new_offset));
                        }
                    }
                }

                if self.is_dragging_x && needs_h_scrollbar {
                    if let (Some(start_mouse_x), Some(start_scroll_x)) = (self.drag_start_mouse_x, self.drag_start_scroll_x) {
                        let track_width = if needs_v_scrollbar { bounds.width - scrollbar_area } else { bounds.width };
                        let thumb_width = self.thumb_size(track_width, final_content_size.width);
                        let track_range = track_width - thumb_width;

                        let mouse_delta = position.x - start_mouse_x;
                        let scroll_per_pixel = if track_range > 0.0 { max_scroll_x / track_range } else { 0.0 };
                        let new_offset = (start_scroll_x + mouse_delta * scroll_per_pixel).clamp(0.0, max_scroll_x);

                        log::trace!("📜 Horizontal drag: delta={:.1}, offset={:.1}/{:.1}", mouse_delta, new_offset, max_scroll_x);
                        if let Some(ref on_scroll_x) = self.on_scroll_x {
                            return Some(on_scroll_x(new_offset));
                        }
                    }
                }

                let content_pos = Point::new(position.x + self.scroll_offset_x, position.y + self.scroll_offset_y);
                let transformed = Event::MouseMoved { position: content_pos };
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

    fn natural_size(&self, max_width: ConcreteSize) -> ConcreteSizeXY {
        let content_size = self.child.widget().natural_size(max_width);

        let width = self.width
            .map(|w| ConcreteSize::new_unchecked(w))
            .unwrap_or(content_size.width);

        let height = self.height
            .map(|h| ConcreteSize::new_unchecked(h))
            .unwrap_or(content_size.height);

        ConcreteSizeXY::new(width, height)
    }

    fn minimum_size(&self) -> ConcreteSizeXY {
        ConcreteSizeXY::ZERO
    }

    fn is_shrinkable(&self) -> bool {
        true
    }
}

/// Helper function to create a scrollable container.
pub fn scrollable<'a, Message>(child: Element<'a, Message>) -> Scrollable<'a, Message> {
    Scrollable::new(child)
}
