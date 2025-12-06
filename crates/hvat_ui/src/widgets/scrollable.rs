//! Scrollable container widget that allows vertical and/or horizontal scrolling via scrollbars.
//!
//! This widget consists of:
//! - A content viewport with clipping
//! - Vertical and/or horizontal scrollbars with track and thumb
//! - Coordinate transformation for events

use crate::{Element, Event, Layout, Limits, MouseButton, Point, Rectangle, Renderer, Widget};
use super::config::{ScrollbarConfig, ScrollDirection};

/// Default scrollbar configuration (used if not overridden).
fn default_scrollbar_config() -> ScrollbarConfig {
    ScrollbarConfig::default()
}

/// A scrollable container that wraps a single child and allows scrolling via scrollbars.
/// Supports vertical, horizontal, or both scroll directions.
/// Mouse wheel events pass through to children (for zoom support).
pub struct Scrollable<'a, Message> {
    child: Element<'a, Message>,
    /// Scroll direction (vertical, horizontal, or both)
    direction: ScrollDirection,
    /// Current vertical scroll offset (positive = scrolled down)
    scroll_offset_y: f32,
    /// Current horizontal scroll offset (positive = scrolled right)
    scroll_offset_x: f32,
    /// Height of the viewport (set via builder)
    height: Option<f32>,
    /// Width of the viewport (set via builder)
    width: Option<f32>,
    /// Whether the vertical scrollbar is currently being dragged
    is_dragging_y: bool,
    /// Whether the horizontal scrollbar is currently being dragged
    is_dragging_x: bool,
    /// Mouse Y position when vertical drag started (for relative dragging)
    drag_start_mouse_y: Option<f32>,
    /// Scroll offset when vertical drag started
    drag_start_scroll_y: Option<f32>,
    /// Mouse X position when horizontal drag started (for relative dragging)
    drag_start_mouse_x: Option<f32>,
    /// Scroll offset when horizontal drag started
    drag_start_scroll_x: Option<f32>,
    /// Scrollbar appearance configuration
    scrollbar_config: ScrollbarConfig,
    /// Whether children should fill the viewport (for Length::Fill support)
    fill_viewport: bool,
    /// Callback when vertical scroll offset changes
    on_scroll_y: Option<Box<dyn Fn(f32) -> Message>>,
    /// Callback when horizontal scroll offset changes
    on_scroll_x: Option<Box<dyn Fn(f32) -> Message>>,
    /// Callback when vertical scrollbar drag starts (receives mouse_y position)
    on_drag_start_y: Option<Box<dyn Fn(f32) -> Message>>,
    /// Callback when vertical scrollbar drag ends
    on_drag_end_y: Option<Box<dyn Fn() -> Message>>,
    /// Callback when horizontal scrollbar drag starts (receives mouse_x position)
    on_drag_start_x: Option<Box<dyn Fn(f32) -> Message>>,
    /// Callback when horizontal scrollbar drag ends
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
            scrollbar_config: default_scrollbar_config(),
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

    /// Set the scroll direction.
    pub fn direction(mut self, direction: ScrollDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set the vertical scroll offset (from external state).
    pub fn scroll_offset_y(mut self, offset: f32) -> Self {
        self.scroll_offset_y = offset;
        self
    }

    /// Set the horizontal scroll offset (from external state).
    pub fn scroll_offset_x(mut self, offset: f32) -> Self {
        self.scroll_offset_x = offset;
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

    /// Set the scrollbar configuration.
    pub fn scrollbar_config(mut self, config: ScrollbarConfig) -> Self {
        self.scrollbar_config = config;
        self
    }

    /// Set the callback when vertical scroll offset changes.
    pub fn on_scroll_y<F>(mut self, f: F) -> Self
    where
        F: Fn(f32) -> Message + 'static,
    {
        self.on_scroll_y = Some(Box::new(f));
        self
    }

    /// Set the callback when horizontal scroll offset changes.
    pub fn on_scroll_x<F>(mut self, f: F) -> Self
    where
        F: Fn(f32) -> Message + 'static,
    {
        self.on_scroll_x = Some(Box::new(f));
        self
    }

    /// Set the callback when vertical scrollbar drag starts.
    /// The callback receives the mouse Y position at drag start.
    pub fn on_drag_start_y<F>(mut self, f: F) -> Self
    where
        F: Fn(f32) -> Message + 'static,
    {
        self.on_drag_start_y = Some(Box::new(f));
        self
    }

    /// Set the callback when vertical scrollbar drag ends.
    pub fn on_drag_end_y<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'static,
    {
        self.on_drag_end_y = Some(Box::new(f));
        self
    }

    /// Set the callback when horizontal scrollbar drag starts.
    /// The callback receives the mouse X position at drag start.
    pub fn on_drag_start_x<F>(mut self, f: F) -> Self
    where
        F: Fn(f32) -> Message + 'static,
    {
        self.on_drag_start_x = Some(Box::new(f));
        self
    }

    /// Set the callback when horizontal scrollbar drag ends.
    pub fn on_drag_end_x<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'static,
    {
        self.on_drag_end_x = Some(Box::new(f));
        self
    }

    // === Helper methods for scrollbar calculations ===

    /// Get the total area reserved for scrollbars.
    fn scrollbar_area(&self) -> f32 {
        self.scrollbar_config.total_area()
    }

    /// Calculate vertical thumb height based on viewport and content sizes.
    fn thumb_height(&self, viewport_height: f32, content_height: f32) -> f32 {
        (viewport_height / content_height * viewport_height).max(self.scrollbar_config.min_thumb_height)
    }

    /// Calculate horizontal thumb width based on viewport and content sizes.
    fn thumb_width(&self, viewport_width: f32, content_width: f32) -> f32 {
        (viewport_width / content_width * viewport_width).max(self.scrollbar_config.min_thumb_height)
    }

    /// Calculate vertical thumb Y position based on scroll offset.
    fn thumb_y_for_offset(&self, scroll_offset: f32, viewport_y: f32, viewport_height: f32, thumb_height: f32, max_scroll: f32) -> f32 {
        let clamped_offset = scroll_offset.clamp(0.0, max_scroll);
        let scroll_ratio = if max_scroll > 0.0 {
            clamped_offset / max_scroll
        } else {
            0.0
        };
        viewport_y + scroll_ratio * (viewport_height - thumb_height)
    }

    /// Calculate horizontal thumb X position based on scroll offset.
    fn thumb_x_for_offset(&self, scroll_offset: f32, viewport_x: f32, viewport_width: f32, thumb_width: f32, max_scroll: f32) -> f32 {
        let clamped_offset = scroll_offset.clamp(0.0, max_scroll);
        let scroll_ratio = if max_scroll > 0.0 {
            clamped_offset / max_scroll
        } else {
            0.0
        };
        viewport_x + scroll_ratio * (viewport_width - thumb_width)
    }

    /// Get vertical scrollbar X position (right side of viewport).
    fn scrollbar_x(&self, viewport_x: f32, viewport_width: f32) -> f32 {
        viewport_x + viewport_width - self.scrollbar_config.width - self.scrollbar_config.padding
    }

    /// Get horizontal scrollbar Y position (bottom of viewport).
    fn scrollbar_y(&self, viewport_y: f32, viewport_height: f32) -> f32 {
        viewport_y + viewport_height - self.scrollbar_config.width - self.scrollbar_config.padding
    }

    /// Create vertical scrollbar hit bounds (slightly larger for easier clicking).
    fn scrollbar_hit_bounds_y(&self, viewport: &Rectangle, has_h_scrollbar: bool) -> Rectangle {
        let x = self.scrollbar_x(viewport.x, viewport.width);
        let height = if has_h_scrollbar {
            viewport.height - self.scrollbar_area()
        } else {
            viewport.height
        };
        Rectangle::new(
            x - 4.0,
            viewport.y,
            self.scrollbar_config.width + 8.0,
            height,
        )
    }

    /// Create horizontal scrollbar hit bounds (slightly larger for easier clicking).
    fn scrollbar_hit_bounds_x(&self, viewport: &Rectangle, has_v_scrollbar: bool) -> Rectangle {
        let y = self.scrollbar_y(viewport.y, viewport.height);
        let width = if has_v_scrollbar {
            viewport.width - self.scrollbar_area()
        } else {
            viewport.width
        };
        Rectangle::new(
            viewport.x,
            y - 4.0,
            width,
            self.scrollbar_config.width + 8.0,
        )
    }

    /// Draw the vertical scrollbar (track and thumb).
    fn draw_scrollbar_y(&self, renderer: &mut Renderer, viewport: &Rectangle, content_height: f32, has_h_scrollbar: bool) {
        let scrollbar_x = self.scrollbar_x(viewport.x, viewport.width);
        let config = &self.scrollbar_config;

        // Track height excludes horizontal scrollbar area if present
        let track_height = if has_h_scrollbar {
            viewport.height - self.scrollbar_area()
        } else {
            viewport.height
        };

        // Track background
        let track_bounds = Rectangle::new(
            scrollbar_x,
            viewport.y,
            config.width,
            track_height,
        );
        renderer.fill_rect(track_bounds, config.track_color);

        // Thumb
        let thumb_height = self.thumb_height(track_height, content_height);
        let max_scroll = (content_height - track_height).max(0.0);
        let thumb_y = self.thumb_y_for_offset(self.scroll_offset_y, viewport.y, track_height, thumb_height, max_scroll);

        let thumb_color = if self.is_dragging_y {
            config.thumb_active_color
        } else {
            config.thumb_color
        };

        let thumb_bounds = Rectangle::new(scrollbar_x, thumb_y, config.width, thumb_height);
        renderer.fill_rect(thumb_bounds, thumb_color);
    }

    /// Draw the horizontal scrollbar (track and thumb).
    fn draw_scrollbar_x(&self, renderer: &mut Renderer, viewport: &Rectangle, content_width: f32, has_v_scrollbar: bool) {
        let scrollbar_y = self.scrollbar_y(viewport.y, viewport.height);
        let config = &self.scrollbar_config;

        // Track width excludes vertical scrollbar area if present
        let track_width = if has_v_scrollbar {
            viewport.width - self.scrollbar_area()
        } else {
            viewport.width
        };

        // Track background
        let track_bounds = Rectangle::new(
            viewport.x,
            scrollbar_y,
            track_width,
            config.width,
        );
        renderer.fill_rect(track_bounds, config.track_color);

        // Thumb
        let thumb_width = self.thumb_width(track_width, content_width);
        let max_scroll = (content_width - track_width).max(0.0);
        let thumb_x = self.thumb_x_for_offset(self.scroll_offset_x, viewport.x, track_width, thumb_width, max_scroll);

        let thumb_color = if self.is_dragging_x {
            config.thumb_active_color
        } else {
            config.thumb_color
        };

        let thumb_bounds = Rectangle::new(thumb_x, scrollbar_y, thumb_width, config.width);
        renderer.fill_rect(thumb_bounds, thumb_color);
    }

    /// Measure child content using ContentMeasure context.
    /// This tells children to report natural size, not fill.
    fn measure_content(&self, viewport_width: f32, _viewport_height: f32) -> crate::Size {
        // Use ContentMeasure context so children report natural size
        let content_limits = Limits::for_content_measure(viewport_width, f32::INFINITY);
        let content_layout = self.child.widget().layout(&content_limits);
        content_layout.size()
    }
}

impl<'a, Message: Clone> Widget<Message> for Scrollable<'a, Message> {
    fn layout(&self, limits: &Limits) -> Layout {
        // Viewport size: use specified dimensions or available space
        let viewport_width = self.width.unwrap_or_else(|| {
            if limits.max_width.is_finite() {
                limits.max_width
            } else {
                300.0 // reasonable default
            }
        });

        // For height: if not specified, we want to fill available space
        let viewport_height = self.height.unwrap_or(0.0);

        let bounds = Rectangle::new(0.0, 0.0, viewport_width, viewport_height);

        // If height is not specified, we want to fill vertically
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

        // Measure content to determine scrollbar needs
        let content_size = self.measure_content(bounds.width, bounds.height);

        log::trace!(
            "📜 Scrollable content_size: w={:.1}, h={:.1}, viewport_h={:.1}, needs_scroll={}",
            content_size.width, content_size.height, bounds.height, content_size.height > bounds.height
        );

        let needs_v_scrollbar = self.direction.has_vertical() && content_size.height > bounds.height;
        let needs_h_scrollbar = self.direction.has_horizontal() && content_size.width > bounds.width;

        // Calculate content area (exclude scrollbar areas)
        let content_width = if needs_v_scrollbar {
            bounds.width - scrollbar_area
        } else {
            bounds.width
        };
        let content_height = if needs_h_scrollbar {
            bounds.height - scrollbar_area
        } else {
            bounds.height
        };

        // Re-measure with adjusted width if vertical scrollbar present
        let final_content_size = if needs_v_scrollbar && content_width != bounds.width {
            self.measure_content(content_width, bounds.height)
        } else {
            content_size
        };

        // Child size - if fill_viewport, ensure at least viewport size
        let child_width = if self.fill_viewport {
            final_content_size.width.max(content_width)
        } else {
            final_content_size.width
        };
        let child_height = if self.fill_viewport {
            final_content_size.height.max(content_height)
        } else {
            final_content_size.height
        };

        // Calculate max scroll and clamp current offsets
        let max_scroll_y = (child_height - content_height).max(0.0);
        let max_scroll_x = (child_width - content_width).max(0.0);
        let clamped_scroll_y = self.scroll_offset_y.clamp(0.0, max_scroll_y);
        let clamped_scroll_x = self.scroll_offset_x.clamp(0.0, max_scroll_x);

        log::trace!(
            "📜 Scrollable scroll: child_h={:.1}, content_h={:.1}, max_scroll_y={:.1}, scroll_offset={:.1}, clamped={:.1}",
            child_height, content_height, max_scroll_y, self.scroll_offset_y, clamped_scroll_y
        );

        // Push clip and scroll offsets (using clamped values)
        let clip_bounds = Rectangle::new(bounds.x, bounds.y, content_width, content_height);
        renderer.push_clip(clip_bounds);
        renderer.push_scroll_offset_y(clamped_scroll_y);
        renderer.push_scroll_offset_x(clamped_scroll_x);

        // Draw child with its natural size (capped to viewport for non-scrolling dimensions)
        let child_bounds = Rectangle::new(bounds.x, bounds.y, child_width, child_height);
        let child_layout = Layout::new(child_bounds);
        self.child.widget().draw(renderer, &child_layout);

        // Pop scroll offsets and clip
        renderer.pop_scroll_offset_x();
        renderer.pop_scroll_offset_y();
        renderer.pop_clip();

        // Draw scrollbars if needed
        if needs_v_scrollbar {
            self.draw_scrollbar_y(renderer, &bounds, child_height, needs_h_scrollbar);
        }
        if needs_h_scrollbar {
            self.draw_scrollbar_x(renderer, &bounds, child_width, needs_v_scrollbar);
        }
    }

    fn on_event(&mut self, event: &Event, layout: &Layout) -> Option<Message> {
        let bounds = layout.bounds();
        let scrollbar_area = self.scrollbar_area();

        // Measure content
        let content_size = self.measure_content(bounds.width, bounds.height);

        let needs_v_scrollbar = self.direction.has_vertical() && content_size.height > bounds.height;
        let needs_h_scrollbar = self.direction.has_horizontal() && content_size.width > bounds.width;

        let content_width = if needs_v_scrollbar { bounds.width - scrollbar_area } else { bounds.width };
        let content_height = if needs_h_scrollbar { bounds.height - scrollbar_area } else { bounds.height };

        // Re-measure with adjusted width if vertical scrollbar present
        let final_content_size = if needs_v_scrollbar && content_width != bounds.width {
            self.measure_content(content_width, bounds.height)
        } else {
            content_size
        };

        // Child size - if fill_viewport, ensure at least viewport size
        let child_width = if self.fill_viewport {
            final_content_size.width.max(content_width)
        } else {
            final_content_size.width
        };
        let child_height = if self.fill_viewport {
            final_content_size.height.max(content_height)
        } else {
            final_content_size.height
        };

        let max_scroll_y = (child_height - content_height).max(0.0);
        let max_scroll_x = (child_width - content_width).max(0.0);

        // Log for debugging
        if needs_v_scrollbar {
            log::trace!(
                "📜 Scrollable on_event: child_h={:.1}, content_h={:.1}, max_scroll_y={:.1}, current_offset={:.1}",
                child_height, content_height, max_scroll_y, self.scroll_offset_y
            );
        }

        // Check if scroll offset needs clamping (e.g., after window resize)
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

        let scrollbar_hit_y = self.scrollbar_hit_bounds_y(&bounds, needs_h_scrollbar);
        let scrollbar_hit_x = self.scrollbar_hit_bounds_x(&bounds, needs_v_scrollbar);

        // Helper to create child layout for event handling
        let make_child_layout = || {
            Layout::new(Rectangle::new(bounds.x, bounds.y, child_width, child_height))
        };

        match event {
            Event::MousePressed { button: MouseButton::Left, position } => {
                // Check vertical scrollbar
                if needs_v_scrollbar && scrollbar_hit_y.contains(*position) {
                    log::debug!("📜 Vertical scrollbar click - start drag at y={:.1}", position.y);
                    if let Some(ref on_drag_start_y) = self.on_drag_start_y {
                        return Some(on_drag_start_y(position.y));
                    }
                    return None;
                }

                // Check horizontal scrollbar
                if needs_h_scrollbar && scrollbar_hit_x.contains(*position) {
                    log::debug!("📜 Horizontal scrollbar click - start drag at x={:.1}", position.x);
                    if let Some(ref on_drag_start_x) = self.on_drag_start_x {
                        return Some(on_drag_start_x(position.x));
                    }
                    return None;
                }

                // Pass to child (click is in content area)
                let scrollbar_x = self.scrollbar_x(bounds.x, bounds.width);
                let scrollbar_y = self.scrollbar_y(bounds.y, bounds.height);
                let in_content = bounds.contains(*position)
                    && (!needs_v_scrollbar || position.x < scrollbar_x - 4.0)
                    && (!needs_h_scrollbar || position.y < scrollbar_y - 4.0);

                if in_content {
                    let content_pos = Point::new(
                        position.x + self.scroll_offset_x,
                        position.y + self.scroll_offset_y,
                    );
                    let transformed = Event::MousePressed {
                        button: MouseButton::Left,
                        position: content_pos,
                    };
                    return self.child.widget_mut().on_event(&transformed, &make_child_layout());
                }
                None
            }
            Event::MouseReleased { button: MouseButton::Left, position } => {
                // Handle drag end for vertical scrollbar
                if self.is_dragging_y {
                    log::debug!("📜 Vertical scrollbar drag ended");
                    if let Some(ref on_drag_end_y) = self.on_drag_end_y {
                        return Some(on_drag_end_y());
                    }
                }
                // Handle drag end for horizontal scrollbar
                if self.is_dragging_x {
                    log::debug!("📜 Horizontal scrollbar drag ended");
                    if let Some(ref on_drag_end_x) = self.on_drag_end_x {
                        return Some(on_drag_end_x());
                    }
                }

                let content_pos = Point::new(
                    position.x + self.scroll_offset_x,
                    position.y + self.scroll_offset_y,
                );
                let transformed = Event::MouseReleased {
                    button: MouseButton::Left,
                    position: content_pos,
                };
                self.child.widget_mut().on_event(&transformed, &make_child_layout())
            }
            Event::MouseMoved { position } => {
                // Handle vertical scrollbar drag
                if self.is_dragging_y && needs_v_scrollbar {
                    if let (Some(start_mouse_y), Some(start_scroll_y)) = (self.drag_start_mouse_y, self.drag_start_scroll_y) {
                        let track_height = if needs_h_scrollbar { bounds.height - scrollbar_area } else { bounds.height };
                        let thumb_height = self.thumb_height(track_height, final_content_size.height);
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

                // Handle horizontal scrollbar drag
                if self.is_dragging_x && needs_h_scrollbar {
                    if let (Some(start_mouse_x), Some(start_scroll_x)) = (self.drag_start_mouse_x, self.drag_start_scroll_x) {
                        let track_width = if needs_v_scrollbar { bounds.width - scrollbar_area } else { bounds.width };
                        let thumb_width = self.thumb_width(track_width, final_content_size.width);
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

                let content_pos = Point::new(
                    position.x + self.scroll_offset_x,
                    position.y + self.scroll_offset_y,
                );
                let transformed = Event::MouseMoved {
                    position: content_pos,
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
