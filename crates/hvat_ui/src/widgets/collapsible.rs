//! Collapsible/expandable section widget

use crate::callback::Callback;
use crate::constants::{
    COLLAPSIBLE_CONTENT_PADDING, COLLAPSIBLE_HEADER_HEIGHT, COLLAPSIBLE_HEADER_PADDING_X,
    COLLAPSIBLE_ICON_MARGIN, COLLAPSIBLE_ICON_SIZE, SCROLLBAR_MIN_THUMB, SCROLLBAR_PADDING,
    SCROLLBAR_WIDTH_COMPACT, SCROLL_SPEED,
};
use crate::element::Element;
use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Length, Size};
use crate::renderer::{Color, Renderer};
use crate::state::{CollapsibleState, ScrollState};
use crate::widget::Widget;
use crate::widgets::scrollbar::{self, draw_simple_vertical_scrollbar, ScrollbarParams};
use crate::Context;

/// Configuration for collapsible widget appearance
#[derive(Debug, Clone)]
pub struct CollapsibleConfig {
    /// Header background color
    pub header_bg: Color,
    /// Header hover color
    pub header_hover: Color,
    /// Header text color
    pub header_text_color: Color,
    /// Content background color
    pub content_bg: Color,
    /// Border color
    pub border_color: Color,
    /// Header font size
    pub header_font_size: f32,
    /// Header height
    pub header_height: f32,
    /// Maximum content height before scrolling (None = no limit)
    pub max_content_height: Option<f32>,
    /// Content padding (ensures child borders are visible)
    pub content_padding: f32,
}

impl Default for CollapsibleConfig {
    fn default() -> Self {
        Self {
            header_bg: Color::rgba(0.15, 0.15, 0.18, 1.0),
            header_hover: Color::rgba(0.2, 0.2, 0.24, 1.0),
            header_text_color: Color::TEXT_PRIMARY,
            content_bg: Color::rgba(0.12, 0.12, 0.14, 1.0),
            border_color: Color::BORDER,
            header_font_size: 14.0,
            header_height: COLLAPSIBLE_HEADER_HEIGHT,
            max_content_height: None,
            content_padding: COLLAPSIBLE_CONTENT_PADDING,
        }
    }
}

/// A collapsible/expandable section widget
///
/// Features:
/// - Click header to toggle expanded/collapsed state
/// - Chevron icon that rotates based on state
/// - Custom header content support
/// - Optional scrollable content with max_height
pub struct Collapsible<M> {
    /// Internal state (cloned from external)
    state: CollapsibleState,
    /// Header title text
    header_text: String,
    /// Content element (built via closure)
    content: Option<Element<M>>,
    /// Scroll state for scrollable content
    scroll_state: ScrollState,
    /// Width constraint
    width: Length,
    /// Configuration
    config: CollapsibleConfig,
    /// Callback when toggled
    on_toggle: Callback<CollapsibleState, M>,
    /// Callback when scroll offset changes (for nested scroll consumption)
    on_scroll: Callback<ScrollState, M>,
    /// Internal: cached header bounds
    header_bounds: Bounds,
    /// Internal: cached content size (full size before clamping)
    content_size: Size,
    /// Internal: visible content height (after max_height clamping)
    visible_content_height: f32,
    /// Internal: is hovering over header
    hover_header: bool,
    /// Internal: is dragging the scrollbar
    scrollbar_dragging: bool,
    /// Internal: offset within thumb where drag started
    scrollbar_drag_offset: f32,
}

impl<M> Default for Collapsible<M> {
    fn default() -> Self {
        Self {
            state: CollapsibleState::default(),
            header_text: "Section".to_string(),
            content: None,
            scroll_state: ScrollState::default(),
            width: Length::Fill(1.0),
            config: CollapsibleConfig::default(),
            on_toggle: Callback::none(),
            on_scroll: Callback::none(),
            header_bounds: Bounds::ZERO,
            content_size: Size::ZERO,
            visible_content_height: 0.0,
            hover_header: false,
            scrollbar_dragging: false,
            scrollbar_drag_offset: 0.0,
        }
    }
}

impl<M: 'static> Collapsible<M> {
    /// Create a new collapsible section
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            header_text: header.into(),
            ..Default::default()
        }
    }

    /// Set the collapsible state (copies the state)
    pub fn state(mut self, state: &CollapsibleState) -> Self {
        self.state = *state;
        self
    }

    /// Set the content using a builder function
    pub fn content<F>(mut self, builder: F) -> Self
    where
        F: FnOnce(&mut Context<M>),
    {
        use crate::widgets::Column;
        let mut ctx = Context::new();
        builder(&mut ctx);
        self.content = Some(Element::new(Column::new(ctx.take())));
        self
    }

    /// Set the width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Set configuration
    pub fn config(mut self, config: CollapsibleConfig) -> Self {
        self.config = config;
        self
    }

    /// Set header background color
    pub fn header_color(mut self, color: Color) -> Self {
        self.config.header_bg = color;
        self
    }

    /// Set maximum content height (enables scrolling when content exceeds this)
    pub fn max_height(mut self, height: f32) -> Self {
        self.config.max_content_height = Some(height);
        self
    }

    /// Set callback for toggle events
    pub fn on_toggle<F>(mut self, callback: F) -> Self
    where
        F: Fn(CollapsibleState) -> M + 'static,
    {
        self.on_toggle = Callback::new(callback);
        self
    }

    /// Set callback for scroll changes
    ///
    /// This callback is invoked when the internal scroll position changes.
    /// Setting this callback allows nested scrolling to work correctly by
    /// indicating that the scroll event was consumed by this widget.
    pub fn on_scroll<F>(mut self, callback: F) -> Self
    where
        F: Fn(ScrollState) -> M + 'static,
    {
        self.on_scroll = Callback::new(callback);
        self
    }

    /// Set the scroll state (copies the state)
    pub fn scroll_state(mut self, state: &ScrollState) -> Self {
        log::trace!(
            "Collapsible '{}': scroll_state set to offset={:?}",
            self.header_text,
            state.offset
        );
        self.scroll_state = *state;
        self
    }

    /// Emit a toggle state change if handler is set
    fn emit_change(&self) -> Option<M> {
        self.on_toggle.call(self.state)
    }

    /// Emit a scroll state change if handler is set
    fn emit_scroll_change(&self) -> Option<M> {
        log::trace!(
            "Collapsible '{}': emitting scroll change offset={:?}",
            self.header_text,
            self.scroll_state.offset
        );
        self.on_scroll.call(self.scroll_state)
    }

    /// Check if content needs scrolling
    fn needs_scrolling(&self) -> bool {
        if let Some(max_height) = self.config.max_content_height {
            self.content_size.height > max_height
        } else {
            false
        }
    }

    /// Get the actual visible height (clamped by max_height if set)
    fn get_visible_height(&self) -> f32 {
        if !self.state.is_expanded {
            return 0.0;
        }

        if let Some(max_height) = self.config.max_content_height {
            self.content_size.height.min(max_height)
        } else {
            self.content_size.height
        }
    }

    /// Calculate header bounds from layout bounds
    #[inline]
    fn calc_header_bounds(&self, layout_bounds: Bounds) -> Bounds {
        Bounds::new(
            layout_bounds.x,
            layout_bounds.y,
            self.header_bounds.width,
            self.config.header_height,
        )
    }

    /// Calculate viewport bounds (content area) from layout bounds
    #[inline]
    fn calc_viewport_bounds(&self, layout_bounds: Bounds) -> Bounds {
        let header_bounds = self.calc_header_bounds(layout_bounds);
        Bounds::new(
            layout_bounds.x,
            header_bounds.bottom(),
            layout_bounds.width,
            self.visible_content_height,
        )
    }

    /// Calculate content bounds for drawing (applies scroll offset and padding)
    #[inline]
    fn calc_content_bounds_for_draw(&self, layout_bounds: Bounds) -> Bounds {
        let header_bounds = self.calc_header_bounds(layout_bounds);
        let padding = self.config.content_padding;
        Bounds::new(
            layout_bounds.x + padding,
            header_bounds.bottom() + padding - self.scroll_state.offset.1,
            self.content_size.width,
            self.content_size.height,
        )
    }

    /// Calculate content bounds for events (no scroll offset applied, includes padding)
    #[inline]
    fn calc_content_bounds_for_events(&self, layout_bounds: Bounds) -> Bounds {
        let header_bounds = self.calc_header_bounds(layout_bounds);
        let padding = self.config.content_padding;
        Bounds::new(
            layout_bounds.x + padding,
            header_bounds.bottom() + padding,
            self.content_size.width,
            self.content_size.height,
        )
    }

    /// Get scrollbar track bounds relative to viewport
    fn scrollbar_track_bounds(&self, viewport_bounds: Bounds) -> Bounds {
        let scrollbar_x = viewport_bounds.right() - SCROLLBAR_WIDTH_COMPACT - SCROLLBAR_PADDING;
        let scrollbar_height = viewport_bounds.height - SCROLLBAR_PADDING * 2.0;
        Bounds::new(
            scrollbar_x,
            viewport_bounds.y + SCROLLBAR_PADDING,
            SCROLLBAR_WIDTH_COMPACT,
            scrollbar_height,
        )
    }

    /// Build ScrollbarParams for the collapsible's scrollbar
    fn scrollbar_params(&self, viewport_bounds: Bounds) -> ScrollbarParams {
        let track_bounds = self.scrollbar_track_bounds(viewport_bounds);
        ScrollbarParams::new(
            self.content_size.height,
            self.visible_content_height,
            self.scroll_state.offset.1,
            track_bounds,
        )
        .with_bar_size(SCROLLBAR_WIDTH_COMPACT)
        .with_min_thumb(SCROLLBAR_MIN_THUMB)
    }
}

impl<M: 'static> Widget<M> for Collapsible<M> {
    fn has_active_overlay(&self) -> bool {
        self.content
            .as_ref()
            .map_or(false, |c| c.has_active_overlay())
    }

    fn has_active_drag(&self) -> bool {
        // Forward to content - needed for widgets like ColorPicker that have internal drag state
        self.scrollbar_dragging || self.content.as_ref().map_or(false, |c| c.has_active_drag())
    }

    fn capture_bounds(&self, layout_bounds: Bounds) -> Option<Bounds> {
        if !self.state.is_expanded {
            return None;
        }
        if let Some(content) = &self.content {
            if content.has_active_overlay() {
                let content_bounds = self.calc_content_bounds_for_events(layout_bounds);
                if let Some(content_capture) = content.capture_bounds(content_bounds) {
                    // Return capture bounds in content space (same as event dispatch space)
                    // The scroll offset is applied to event positions before dispatch,
                    // so capture bounds should NOT subtract scroll offset here
                    return Some(layout_bounds.union(&content_capture));
                }
            }
        }
        None
    }

    fn layout(&mut self, available: Size) -> Size {
        let width = self.width.resolve(available.width, available.width);
        let padding = self.config.content_padding;

        // Header is always visible
        self.header_bounds = Bounds::new(0.0, 0.0, width, self.config.header_height);

        // Layout content if present
        if let Some(content) = &mut self.content {
            // Account for content padding when calculating available space
            let content_width = width - padding * 2.0;
            // For scrollable content, give it unlimited height to measure full size
            let content_available = if self.config.max_content_height.is_some() {
                Size::new(content_width, f32::MAX)
            } else {
                Size::new(
                    content_width,
                    available.height - self.config.header_height - padding * 2.0,
                )
            };
            self.content_size = content.layout(content_available);

            // Create/update scrollable wrapper if needed
            if self.needs_scrolling() {
                // We need to create a scrollable wrapper
                // For now, we'll handle scrolling manually in draw/on_event
                log::debug!(
                    "Content needs scrolling: {} > {:?}",
                    self.content_size.height,
                    self.config.max_content_height
                );
            }
        } else {
            self.content_size = Size::ZERO;
        }

        // Calculate visible height (includes padding)
        self.visible_content_height = self.get_visible_height() + padding * 2.0;
        let total_height = self.config.header_height + self.visible_content_height;

        Size::new(width, total_height)
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        log::debug!(
            "Collapsible draw: bounds={:?}, is_expanded={}, needs_scroll={}",
            bounds,
            self.state.is_expanded,
            self.needs_scrolling(),
        );

        // Draw header
        let header_bounds = self.calc_header_bounds(bounds);

        let header_bg = if self.hover_header {
            self.config.header_hover
        } else {
            self.config.header_bg
        };

        renderer.fill_rect(header_bounds, header_bg);
        renderer.stroke_rect(header_bounds, self.config.border_color, 1.0);

        // Draw chevron icon
        let icon = if self.state.is_expanded { "▼" } else { "▶" };
        let icon_x = header_bounds.x + COLLAPSIBLE_HEADER_PADDING_X;
        let icon_y = header_bounds.y + (self.config.header_height - COLLAPSIBLE_ICON_SIZE) / 2.0;
        renderer.text(
            icon,
            icon_x,
            icon_y,
            COLLAPSIBLE_ICON_SIZE,
            self.config.header_text_color,
        );

        // Draw header text
        let text_x = icon_x + COLLAPSIBLE_ICON_SIZE + COLLAPSIBLE_ICON_MARGIN;
        let text_y =
            header_bounds.y + (self.config.header_height - self.config.header_font_size) / 2.0;
        renderer.text(
            &self.header_text,
            text_x,
            text_y,
            self.config.header_font_size,
            self.config.header_text_color,
        );

        // Draw content if expanded
        if self.state.is_expanded {
            if let Some(content) = &self.content {
                let viewport_bounds = self.calc_viewport_bounds(bounds);

                // Draw content background
                renderer.fill_rect(viewport_bounds, self.config.content_bg);

                if self.needs_scrolling() {
                    // Clip content to viewport
                    renderer.push_clip(viewport_bounds);

                    // Draw content offset by scroll position
                    let content_bounds = self.calc_content_bounds_for_draw(bounds);
                    content.draw(renderer, content_bounds);

                    renderer.pop_clip();

                    // Draw scrollbar - note: drawn AFTER pop_clip so it's not clipped
                    let track_bounds = self.scrollbar_track_bounds(viewport_bounds);
                    log::debug!(
                        "Collapsible '{}' scrollbar: viewport={:?}, track={:?}, content_h={}, visible_h={}, offset={}",
                        self.header_text,
                        viewport_bounds,
                        track_bounds,
                        self.content_size.height,
                        self.visible_content_height,
                        self.scroll_state.offset.1
                    );
                    self.draw_scrollbar(renderer, viewport_bounds);
                } else {
                    // No scrolling needed, draw content directly
                    let content_bounds = self.calc_content_bounds_for_events(bounds);
                    content.draw(renderer, content_bounds);
                }

                // Draw bottom border
                renderer.stroke_rect(viewport_bounds, self.config.border_color, 1.0);
            }
        }
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        let header_bounds = self.calc_header_bounds(bounds);
        let viewport_bounds = self.calc_viewport_bounds(bounds);

        match event {
            Event::MousePress {
                button: MouseButton::Left,
                position,
                screen_position,
                overlay_hint,
                ..
            } => {
                // Check header click - but skip if this event is for an overlay (e.g., dropdown popup)
                // The overlay hint is set by the application based on the overlay registry
                if header_bounds.contains(position.0, position.1) && !overlay_hint {
                    log::debug!("Collapsible header clicked - toggling");
                    self.state.toggle();
                    return self.emit_change();
                }

                // Check for scrollbar interaction first
                if self.state.is_expanded && self.needs_scrolling() {
                    let params = self.scrollbar_params(viewport_bounds);

                    // Check if click is on the scrollbar track area (wider hit area for usability)
                    let scrollbar_hit_area = Bounds::new(
                        params.track_bounds.x - SCROLLBAR_PADDING,
                        params.track_bounds.y,
                        params.track_bounds.width + SCROLLBAR_PADDING * 2.0,
                        params.track_bounds.height,
                    );

                    if scrollbar_hit_area.contains(position.0, position.1) {
                        if let Some(thumb) = scrollbar::calculate_vertical_thumb(&params) {
                            if thumb.bounds.contains(position.0, position.1) {
                                // Clicked on thumb - start dragging
                                self.scrollbar_dragging = true;
                                self.scrollbar_drag_offset = position.1 - thumb.bounds.y;
                                log::debug!(
                                    "Collapsible scrollbar drag started, offset={}",
                                    self.scrollbar_drag_offset
                                );
                            } else {
                                // Clicked on track - jump to position (center thumb on click)
                                let thumb_y = position.1 - thumb.bounds.height / 2.0;
                                self.scroll_state.offset.1 = scrollbar::thumb_y_to_scroll_offset(
                                    thumb_y,
                                    params.track_bounds,
                                    thumb.bounds.height,
                                    self.content_size.height,
                                    self.visible_content_height,
                                );
                                log::debug!(
                                    "Collapsible scrollbar track clicked, new offset={}",
                                    self.scroll_state.offset.1
                                );
                            }
                        }
                        return None;
                    }
                }

                // Forward to content if expanded
                if self.state.is_expanded {
                    let content_bounds = self.calc_content_bounds_for_events(bounds);

                    // Forward if within viewport OR if event is for an overlay (e.g., dropdown popup)
                    let in_viewport = viewport_bounds.contains(position.0, position.1);
                    if in_viewport || *overlay_hint {
                        // Extract values before mutable borrow
                        let needs_scroll = self.needs_scrolling();
                        let scroll_offset = self.scroll_state.offset.1;

                        if let Some(content) = &mut self.content {
                            // Adjust position for scroll offset
                            let adjusted_pos = if needs_scroll {
                                (position.0, position.1 + scroll_offset)
                            } else {
                                *position
                            };

                            // Create adjusted event, preserving original screen_position and overlay_hint
                            let adjusted_event = Event::MousePress {
                                button: MouseButton::Left,
                                position: adjusted_pos,
                                modifiers: event.modifiers(),
                                screen_position: *screen_position,
                                overlay_hint: *overlay_hint,
                            };

                            return content.on_event(&adjusted_event, content_bounds);
                        }
                    }
                }
            }

            Event::MouseMove {
                position,
                overlay_hint,
                ..
            } => {
                // Only show header hover if not in an overlay area
                // The overlay hint tells us if the cursor is over an overlay (e.g., dropdown popup)
                self.hover_header = header_bounds.contains(position.0, position.1) && !overlay_hint;

                // Handle scrollbar dragging
                if self.scrollbar_dragging && self.state.is_expanded && self.needs_scrolling() {
                    let params = self.scrollbar_params(viewport_bounds);
                    if let Some(thumb) = scrollbar::calculate_vertical_thumb(&params) {
                        let thumb_y = position.1 - self.scrollbar_drag_offset;
                        self.scroll_state.offset.1 = scrollbar::thumb_y_to_scroll_offset(
                            thumb_y,
                            params.track_bounds,
                            thumb.bounds.height,
                            self.content_size.height,
                            self.visible_content_height,
                        );
                        log::debug!(
                            "Collapsible scrollbar dragging, offset={}",
                            self.scroll_state.offset.1
                        );
                    }
                    return None;
                }

                // Forward to content if expanded
                if self.state.is_expanded {
                    let content_bounds = self.calc_content_bounds_for_events(bounds);

                    // Forward if within viewport, if event is for an overlay, or if content has active drag
                    let in_viewport = viewport_bounds.contains(position.0, position.1);
                    let content_has_drag =
                        self.content.as_ref().map_or(false, |c| c.has_active_drag());
                    if in_viewport || *overlay_hint || content_has_drag {
                        // Extract values before mutable borrow
                        let needs_scroll = self.needs_scrolling();
                        let scroll_offset = self.scroll_state.offset.1;

                        if let Some(content) = &mut self.content {
                            let adjusted_pos = if needs_scroll {
                                (position.0, position.1 + scroll_offset)
                            } else {
                                *position
                            };

                            let adjusted_event = Event::MouseMove {
                                position: adjusted_pos,
                                modifiers: event.modifiers(),
                                overlay_hint: *overlay_hint,
                            };

                            return content.on_event(&adjusted_event, content_bounds);
                        }
                    }
                }
            }

            Event::MouseScroll {
                delta,
                position,
                modifiers,
                overlay_hint,
            } => {
                if self.state.is_expanded {
                    let content_bounds = self.calc_content_bounds_for_events(bounds);

                    // Forward scroll to content if it's for an overlay or within viewport
                    // This allows overlays (dropdowns) to close on scroll
                    let in_viewport = viewport_bounds.contains(position.0, position.1);
                    if *overlay_hint || in_viewport {
                        // Extract values before mutable borrow
                        let needs_scroll = self.needs_scrolling();
                        let scroll_offset = self.scroll_state.offset.1;
                        let content_bounds_val = content_bounds; // Use already calculated value

                        if let Some(content) = &mut self.content {
                            let adjusted_pos = if needs_scroll {
                                (position.0, position.1 + scroll_offset)
                            } else {
                                *position
                            };

                            let adjusted_event = Event::MouseScroll {
                                delta: *delta,
                                position: adjusted_pos,
                                modifiers: *modifiers,
                                overlay_hint: *overlay_hint,
                            };

                            if let Some(msg) = content.on_event(&adjusted_event, content_bounds_val)
                            {
                                return Some(msg);
                            }
                            // If event was for an overlay, it was handled
                            if *overlay_hint {
                                return None;
                            }
                        }
                    }

                    // Handle scrolling in content area (only if not for an overlay)
                    if !overlay_hint
                        && self.needs_scrolling()
                        && viewport_bounds.contains(position.0, position.1)
                    {
                        let max_scroll =
                            (self.content_size.height - self.visible_content_height).max(0.0);
                        // Negative delta.1 means scroll down (content moves up), positive means scroll up
                        let scroll_amount = -delta.1 * SCROLL_SPEED;
                        let old_offset = self.scroll_state.offset.1;
                        self.scroll_state.offset.1 =
                            (self.scroll_state.offset.1 + scroll_amount).clamp(0.0, max_scroll);

                        // Only emit if we actually scrolled (prevents parent from scrolling too)
                        if (self.scroll_state.offset.1 - old_offset).abs() > 0.001 {
                            log::debug!(
                                "Collapsible scroll: delta={}, offset={}, max={}",
                                delta.1,
                                self.scroll_state.offset.1,
                                max_scroll
                            );
                            // Emit scroll change to indicate we consumed the event
                            return self.emit_scroll_change();
                        }
                        // If we're at the boundary and can't scroll further, let parent handle it
                    }
                }
            }

            Event::CursorLeft => {
                // Cursor left window - release any drag state
                if self.scrollbar_dragging {
                    self.scrollbar_dragging = false;
                    log::debug!("Collapsible scrollbar drag ended (cursor left window)");
                }
                // Forward to content to release its drag states
                let content_bounds = self.calc_content_bounds_for_events(bounds);
                if let Some(content) = &mut self.content {
                    return content.on_event(event, content_bounds);
                }
                return None;
            }

            Event::MouseRelease {
                button,
                position,
                overlay_hint,
                ..
            } => {
                // Stop scrollbar dragging
                if self.scrollbar_dragging {
                    self.scrollbar_dragging = false;
                    log::debug!("Collapsible scrollbar drag ended");
                    return None;
                }

                // Forward to content if expanded
                let in_viewport = viewport_bounds.contains(position.0, position.1);
                let content_has_drag = self.content.as_ref().map_or(false, |c| c.has_active_drag());
                if self.state.is_expanded && (in_viewport || *overlay_hint || content_has_drag) {
                    // Extract values before mutable borrow
                    let needs_scroll = self.needs_scrolling();
                    let scroll_offset = self.scroll_state.offset.1;
                    let content_bounds = self.calc_content_bounds_for_events(bounds);

                    if let Some(content) = &mut self.content {
                        let adjusted_pos = if needs_scroll {
                            (position.0, position.1 + scroll_offset)
                        } else {
                            *position
                        };

                        let adjusted_event = Event::MouseRelease {
                            button: *button,
                            position: adjusted_pos,
                            modifiers: event.modifiers(),
                            overlay_hint: *overlay_hint,
                        };

                        return content.on_event(&adjusted_event, content_bounds);
                    }
                }
            }

            Event::KeyPress { key, .. } => {
                log::trace!(
                    "Collapsible: KeyPress {:?}, expanded={}",
                    key,
                    self.state.is_expanded
                );
                // Forward to content FIRST if expanded (so focused inputs can handle Enter)
                if self.state.is_expanded {
                    let content_bounds = self.calc_content_bounds_for_events(bounds);
                    if let Some(content) = &mut self.content {
                        if let Some(msg) = content.on_event(event, content_bounds) {
                            log::trace!("Collapsible: content handled KeyPress");
                            return Some(msg);
                        }
                    }
                }

                // Toggle on Enter/Space when hovering over header (only if content didn't handle it)
                if self.hover_header {
                    match key {
                        KeyCode::Enter | KeyCode::Space => {
                            log::trace!("Collapsible: toggling on Enter/Space (header hover)");
                            self.state.toggle();
                            return self.emit_change();
                        }
                        _ => {}
                    }
                }
            }

            _ => {
                // Forward other events to content if expanded
                if self.state.is_expanded {
                    let content_bounds = self.calc_content_bounds_for_events(bounds);
                    if let Some(content) = &mut self.content {
                        return content.on_event(event, content_bounds);
                    }
                }
            }
        }

        None
    }
}

impl<M: 'static> Collapsible<M> {
    /// Draw the scrollbar for scrollable content
    fn draw_scrollbar(&self, renderer: &mut Renderer, viewport_bounds: Bounds) {
        let track_bounds = self.scrollbar_track_bounds(viewport_bounds);

        draw_simple_vertical_scrollbar(
            renderer,
            track_bounds,
            self.content_size.height,
            self.visible_content_height,
            self.scroll_state.offset.1,
            SCROLLBAR_WIDTH_COMPACT,
        );
    }
}
