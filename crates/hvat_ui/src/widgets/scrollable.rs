//! Scrollable container widget

use crate::callback::Callback;
use crate::constants::{SCROLLBAR_MIN_THUMB, SCROLLBAR_WIDTH, SCROLL_SPEED};
use crate::element::Element;
use crate::event::{Event, MouseButton};
use crate::layout::{Bounds, Length, Padding, Size};
use crate::renderer::{Color, Renderer};
use crate::state::{ScrollDragExt, ScrollState};
use crate::widget::Widget;
use crate::widgets::scrollbar::{self, ScrollbarParams};

/// Scroll direction for scrollable containers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollDirection {
    /// Vertical scrolling only
    #[default]
    Vertical,
    /// Horizontal scrolling only
    Horizontal,
    /// Both vertical and horizontal scrolling
    Both,
}

/// Scrollbar visibility options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarVisibility {
    /// Show scrollbar only when content overflows
    #[default]
    Auto,
    /// Always show scrollbar
    Always,
    /// Never show scrollbar
    Never,
}

/// Configuration for scrollbar appearance
#[derive(Debug, Clone)]
pub struct ScrollbarConfig {
    /// Width of the scrollbar track
    pub width: f32,
    /// Minimum thumb size
    pub min_thumb_size: f32,
    /// Track color
    pub track_color: Color,
    /// Thumb color
    pub thumb_color: Color,
    /// Thumb hover color
    pub thumb_hover_color: Color,
    /// Thumb drag color
    pub thumb_drag_color: Color,
}

impl Default for ScrollbarConfig {
    fn default() -> Self {
        Self {
            width: SCROLLBAR_WIDTH,
            min_thumb_size: SCROLLBAR_MIN_THUMB,
            track_color: Color::rgba(0.15, 0.15, 0.18, 0.5),
            thumb_color: Color::rgba(0.4, 0.4, 0.45, 0.8),
            thumb_hover_color: Color::rgba(0.5, 0.5, 0.55, 0.9),
            thumb_drag_color: Color::rgba(0.6, 0.6, 0.65, 1.0),
        }
    }
}

/// A scrollable container widget
///
/// This widget owns a clone of the scroll state and emits changes via on_scroll callback.
/// This allows it to work with immutable borrows in view() methods.
pub struct Scrollable<M> {
    /// Child content element
    content: Element<M>,
    /// Internal scroll state (cloned from external)
    state: ScrollState,
    /// Scroll direction
    direction: ScrollDirection,
    /// Scrollbar visibility
    scrollbar_visibility: ScrollbarVisibility,
    /// Scrollbar configuration
    scrollbar_config: ScrollbarConfig,
    /// Width constraint
    width: Length,
    /// Height constraint
    height: Length,
    /// Padding around content
    padding: Padding,
    /// Callback when scroll offset changes
    on_scroll: Callback<ScrollState, M>,
    /// Internal: cached content size from layout
    content_size: Size,
    /// Internal: cached viewport size from layout
    viewport_size: Size,
    /// Internal: whether hovering over vertical scrollbar
    hover_v_scrollbar: bool,
    /// Internal: whether hovering over horizontal scrollbar
    hover_h_scrollbar: bool,
}

impl<M: 'static> Scrollable<M> {
    /// Create a new scrollable container
    pub fn new(content: Element<M>) -> Self {
        Self {
            content,
            state: ScrollState::default(),
            direction: ScrollDirection::default(),
            scrollbar_visibility: ScrollbarVisibility::default(),
            scrollbar_config: ScrollbarConfig::default(),
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            padding: Padding::ZERO,
            on_scroll: Callback::none(),
            content_size: Size::ZERO,
            viewport_size: Size::ZERO,
            hover_v_scrollbar: false,
            hover_h_scrollbar: false,
        }
    }

    /// Set the scroll state (copies the state)
    pub fn state(mut self, state: &ScrollState) -> Self {
        self.state = *state;
        self
    }

    /// Set the scroll direction
    pub fn direction(mut self, direction: ScrollDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set scrollbar visibility
    pub fn scrollbar_visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        self.scrollbar_visibility = visibility;
        self
    }

    /// Set scrollbar width
    pub fn scrollbar_width(mut self, width: f32) -> Self {
        self.scrollbar_config.width = width;
        self
    }

    /// Set the width constraint
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Set the height constraint
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Set padding around the content
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Set callback for scroll changes
    pub fn on_scroll<F>(mut self, callback: F) -> Self
    where
        F: Fn(ScrollState) -> M + 'static,
    {
        self.on_scroll = Callback::new(callback);
        self
    }

    /// Check if vertical scrolling is needed
    fn needs_vertical_scroll(&self) -> bool {
        matches!(
            self.direction,
            ScrollDirection::Vertical | ScrollDirection::Both
        ) && self.content_size.height > self.viewport_size.height
    }

    /// Check if horizontal scrolling is needed
    fn needs_horizontal_scroll(&self) -> bool {
        matches!(
            self.direction,
            ScrollDirection::Horizontal | ScrollDirection::Both
        ) && self.content_size.width > self.viewport_size.width
    }

    /// Check if vertical scrollbar should be shown
    fn show_vertical_scrollbar(&self) -> bool {
        match self.scrollbar_visibility {
            ScrollbarVisibility::Always => {
                matches!(
                    self.direction,
                    ScrollDirection::Vertical | ScrollDirection::Both
                )
            }
            ScrollbarVisibility::Auto => self.needs_vertical_scroll(),
            ScrollbarVisibility::Never => false,
        }
    }

    /// Check if horizontal scrollbar should be shown
    fn show_horizontal_scrollbar(&self) -> bool {
        match self.scrollbar_visibility {
            ScrollbarVisibility::Always => {
                matches!(
                    self.direction,
                    ScrollDirection::Horizontal | ScrollDirection::Both
                )
            }
            ScrollbarVisibility::Auto => self.needs_horizontal_scroll(),
            ScrollbarVisibility::Never => false,
        }
    }

    /// Calculate vertical scrollbar thumb geometry using scrollbar utilities
    fn v_scrollbar_thumb(&self, bounds: Bounds) -> Option<Bounds> {
        if !self.show_vertical_scrollbar() || self.content_size.height <= 0.0 {
            return None;
        }

        let params = self.v_scrollbar_params(bounds);
        let thumb = scrollbar::calculate_vertical_thumb(&params)?;

        log::trace!(
            "v_scrollbar_thumb: bounds.y={:.1}, track_height={:.1}, thumb_height={:.1}, scroll_ratio={:.3}, thumb_y={:.1}",
            bounds.y, params.track_bounds.height, thumb.bounds.height, thumb.scroll_ratio, thumb.bounds.y
        );

        Some(thumb.bounds)
    }

    /// Calculate horizontal scrollbar thumb geometry using scrollbar utilities
    fn h_scrollbar_thumb(&self, bounds: Bounds) -> Option<Bounds> {
        if !self.show_horizontal_scrollbar() || self.content_size.width <= 0.0 {
            return None;
        }

        let params = self.h_scrollbar_params(bounds);
        let thumb = scrollbar::calculate_horizontal_thumb(&params)?;

        Some(thumb.bounds)
    }

    /// Clamp scroll offset to valid range using scrollbar utility
    fn clamp_scroll(&mut self) {
        let max_scroll_y = (self.content_size.height - self.viewport_size.height).max(0.0);
        log::debug!(
            "clamp_scroll: content_size={:?}, viewport_size={:?}, current_offset={:?}, max_scroll_y={:.1}",
            self.content_size,
            self.viewport_size,
            self.state.offset,
            max_scroll_y
        );

        let new_offset = scrollbar::clamp_scroll_offsets(
            self.state.offset,
            (self.content_size.width, self.content_size.height),
            (self.viewport_size.width, self.viewport_size.height),
        );

        if (new_offset.1 - self.state.offset.1).abs() > 0.1 {
            log::debug!(
                "  -> Clamped offset from {:.1} to {:.1}",
                self.state.offset.1,
                new_offset.1
            );
        }

        self.state.offset = new_offset;
    }

    /// Emit a state change if handler is set
    fn emit_change(&self) -> Option<M> {
        self.on_scroll.call(self.state)
    }

    /// Calculate viewport bounds from current layout bounds
    /// This is the visible content area excluding padding and scrollbars
    #[inline]
    fn calc_viewport_bounds(&self, bounds: Bounds) -> Bounds {
        Bounds::new(
            bounds.x + self.padding.left,
            bounds.y + self.padding.top,
            self.viewport_size.width,
            self.viewport_size.height,
        )
    }

    /// Calculate viewport size from current layout bounds
    /// This recalculates the viewport dimensions accounting for padding and scrollbars
    #[inline]
    fn calc_viewport_size(&self, bounds: Bounds) -> Size {
        let scrollbar_width = self.scrollbar_config.width;
        let base_width = bounds.width - self.padding.horizontal();
        let base_height = bounds.height - self.padding.vertical();

        // Check if scrollbars will be shown (using cached content_size)
        let needs_v = self.needs_vertical_scroll();
        let needs_h = self.needs_horizontal_scroll();

        let width = base_width - if needs_v { scrollbar_width } else { 0.0 };
        let height = base_height - if needs_h { scrollbar_width } else { 0.0 };

        Size::new(width, height)
    }

    /// Calculate content bounds for drawing (applies scroll offset)
    #[inline]
    fn calc_content_bounds_for_draw(&self, viewport_bounds: Bounds) -> Bounds {
        Bounds::new(
            viewport_bounds.x - self.state.offset.0,
            viewport_bounds.y - self.state.offset.1,
            self.content_size.width,
            self.content_size.height,
        )
    }

    /// Calculate content bounds for event dispatch (no scroll offset applied)
    #[inline]
    fn calc_content_bounds_for_events(&self, viewport_bounds: Bounds) -> Bounds {
        Bounds::new(
            viewport_bounds.x,
            viewport_bounds.y,
            self.content_size.width,
            self.content_size.height,
        )
    }

    /// Build ScrollbarParams for vertical scrollbar
    fn v_scrollbar_params(&self, bounds: Bounds) -> ScrollbarParams {
        // CRITICAL FIX: Calculate viewport height from current bounds, not cached viewport_size
        // This ensures track bounds are always consistent with the actual draw bounds
        let viewport_size = self.calc_viewport_size(bounds);

        let track_height = viewport_size.height
            - if self.show_horizontal_scrollbar() {
                self.scrollbar_config.width
            } else {
                0.0
            };
        let track_bounds = Bounds::new(
            bounds.right() - self.scrollbar_config.width,
            bounds.y + self.padding.top,
            self.scrollbar_config.width,
            track_height,
        );

        let max_scroll = (self.content_size.height - viewport_size.height).max(0.0);
        log::trace!(
            "v_scrollbar_params: content={:.1}, viewport={:.1}, offset={:.1}, max_scroll={:.1}, track_height={:.1}",
            self.content_size.height,
            viewport_size.height,
            self.state.offset.1,
            max_scroll,
            track_height
        );

        ScrollbarParams::new(
            self.content_size.height,
            viewport_size.height,
            self.state.offset.1,
            track_bounds,
        )
        .with_bar_size(self.scrollbar_config.width)
        .with_min_thumb(self.scrollbar_config.min_thumb_size)
    }

    /// Build ScrollbarParams for horizontal scrollbar
    fn h_scrollbar_params(&self, bounds: Bounds) -> ScrollbarParams {
        // CRITICAL FIX: Calculate viewport width from current bounds, not cached viewport_size
        let viewport_size = self.calc_viewport_size(bounds);

        let track_width = viewport_size.width
            - if self.show_vertical_scrollbar() {
                self.scrollbar_config.width
            } else {
                0.0
            };
        let track_bounds = Bounds::new(
            bounds.x + self.padding.left,
            bounds.bottom() - self.scrollbar_config.width,
            track_width,
            self.scrollbar_config.width,
        );
        ScrollbarParams::new(
            self.content_size.width,
            viewport_size.width,
            self.state.offset.0,
            track_bounds,
        )
        .with_bar_size(self.scrollbar_config.width)
        .with_min_thumb(self.scrollbar_config.min_thumb_size)
    }
}

impl<M: 'static> Widget<M> for Scrollable<M> {
    fn has_active_overlay(&self) -> bool {
        self.content.has_active_overlay()
    }

    fn has_active_drag(&self) -> bool {
        // Either the scrollbar itself is being dragged, or a child widget is being dragged
        self.state.drag.is_dragging() || self.content.has_active_drag()
    }

    fn capture_bounds(&self, layout_bounds: Bounds) -> Option<Bounds> {
        // If content has an overlay, expand our capture bounds to include it
        if self.content.has_active_overlay() {
            let viewport_bounds = self.calc_viewport_bounds(layout_bounds);
            // Content bounds are in the same coordinate space used for event dispatch
            // The child's capture_bounds should return screen-space coordinates
            let content_bounds = self.calc_content_bounds_for_events(viewport_bounds);
            // Get the content's capture bounds - they're already in screen space
            // (same space as event positions before scroll adjustment)
            if let Some(content_capture) = self.content.capture_bounds(content_bounds) {
                // Return union of layout bounds and the overlay's capture bounds
                // No offset adjustment needed - capture bounds are in screen space
                return Some(layout_bounds.union(&content_capture));
            }
        }
        None
    }

    fn layout(&mut self, available: Size) -> Size {
        log::debug!("Scrollable layout: available={:?}", available);

        // Resolve our own size (use content size as fallback for Shrink mode)
        // Note: We don't know content size yet, so we use available as initial fallback
        // and re-resolve later if needed. For most cases, scrollables use Fill anyway.
        let own_width = self.width.resolve(available.width, available.width);
        let own_height = self.height.resolve(available.height, available.height);

        // Calculate viewport size (accounting for scrollbars)
        let scrollbar_width = self.scrollbar_config.width;
        let mut viewport_width = own_width - self.padding.horizontal();
        let mut viewport_height = own_height - self.padding.vertical();

        // First pass: layout content to determine if scrollbars are needed
        // Give content unlimited size in scroll direction
        let content_available = match self.direction {
            ScrollDirection::Vertical => Size::new(viewport_width, f32::MAX),
            ScrollDirection::Horizontal => Size::new(f32::MAX, viewport_height),
            ScrollDirection::Both => Size::new(f32::MAX, f32::MAX),
        };

        self.content_size = self.content.layout(content_available);
        log::debug!("Scrollable content size: {:?}", self.content_size);

        // Adjust viewport for scrollbars if needed
        if self.scrollbar_visibility != ScrollbarVisibility::Never {
            let needs_v = matches!(
                self.direction,
                ScrollDirection::Vertical | ScrollDirection::Both
            ) && self.content_size.height > viewport_height;
            let needs_h = matches!(
                self.direction,
                ScrollDirection::Horizontal | ScrollDirection::Both
            ) && self.content_size.width > viewport_width;

            if needs_v {
                viewport_width -= scrollbar_width;
            }
            if needs_h {
                viewport_height -= scrollbar_width;
            }

            // Re-layout if viewport changed due to scrollbars
            if needs_v || needs_h {
                let content_available = match self.direction {
                    ScrollDirection::Vertical => Size::new(viewport_width, f32::MAX),
                    ScrollDirection::Horizontal => Size::new(f32::MAX, viewport_height),
                    ScrollDirection::Both => Size::new(f32::MAX, f32::MAX),
                };
                self.content_size = self.content.layout(content_available);
            }
        }

        // Store the previous viewport size for comparison
        let prev_viewport_height = self.viewport_size.height;

        // Always update viewport size - we need accurate dimensions
        self.viewport_size = Size::new(viewport_width, viewport_height);

        // Only clamp scroll offset if:
        // 1. The viewport got SMALLER (need to constrain scroll)
        // 2. The viewport stayed the same size AND we're not in a FlexLayout probe pass
        //
        // We skip clamping when viewport gets LARGER because this often happens
        // during FlexLayout's two-pass layout: first pass gives full available height,
        // second pass gives correct allocated height. If we clamp on the first (larger)
        // pass, we incorrectly reset the scroll position.
        //
        // CRITICAL FIX: Don't clamp when the available height equals our own_height,
        // indicating we're in the FlexLayout first pass where it's probing our requirements.
        // In this case, viewport_height doesn't reflect the final allocated size yet.
        let is_flexlayout_first_pass = (available.height - own_height).abs() < 1.0;

        let should_clamp = prev_viewport_height > 0.0  // Not the very first layout
            && viewport_height <= prev_viewport_height  // Viewport same or smaller
            && !is_flexlayout_first_pass; // Not in FlexLayout probe pass

        if should_clamp {
            self.clamp_scroll();
        } else {
            log::debug!(
                "Scrollable: skipping clamp (viewport changed {:.1} -> {:.1}, is_first_pass={})",
                prev_viewport_height,
                viewport_height,
                is_flexlayout_first_pass
            );
        }

        Size::new(own_width, own_height)
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        log::debug!(
            "Scrollable draw: bounds={:?}, viewport_size={:?}, content_size={:?}, scroll_offset={:?}",
            bounds,
            self.viewport_size,
            self.content_size,
            self.state.offset
        );

        // Calculate viewport bounds (excluding scrollbars)
        let viewport_bounds = self.calc_viewport_bounds(bounds);

        log::debug!(
            "Scrollable: viewport_bounds={:?} (clip region), content will be drawn at y={:.1}",
            viewport_bounds,
            viewport_bounds.y - self.state.offset.1
        );

        // Push clip for viewport
        renderer.push_clip(viewport_bounds);

        // Draw content with scroll offset applied
        let content_bounds = self.calc_content_bounds_for_draw(viewport_bounds);

        log::debug!(
            "Scrollable: content_bounds={:?} (where content is drawn)",
            content_bounds
        );

        self.content.draw(renderer, content_bounds);

        renderer.pop_clip();

        // Draw vertical scrollbar
        if self.show_vertical_scrollbar() {
            let params = self.v_scrollbar_params(bounds);
            renderer.fill_rect(params.track_bounds, self.scrollbar_config.track_color);

            // Draw thumb
            if let Some(thumb) = self.v_scrollbar_thumb(bounds) {
                let color = if self.state.drag.is_dragging() {
                    self.scrollbar_config.thumb_drag_color
                } else if self.hover_v_scrollbar {
                    self.scrollbar_config.thumb_hover_color
                } else {
                    self.scrollbar_config.thumb_color
                };
                log::trace!(
                    "Drawing V scrollbar thumb at y={:.1}, height={:.1}, track_bottom={:.1}, thumb_bottom={:.1}",
                    thumb.y, thumb.height, params.track_bounds.bottom(), thumb.y + thumb.height
                );
                renderer.fill_rect(thumb, color);
            }
        }

        // Draw horizontal scrollbar
        if self.show_horizontal_scrollbar() {
            let params = self.h_scrollbar_params(bounds);
            renderer.fill_rect(params.track_bounds, self.scrollbar_config.track_color);

            // Draw thumb
            if let Some(thumb) = self.h_scrollbar_thumb(bounds) {
                let color = if self.state.drag.is_dragging() {
                    self.scrollbar_config.thumb_drag_color
                } else if self.hover_h_scrollbar {
                    self.scrollbar_config.thumb_hover_color
                } else {
                    self.scrollbar_config.thumb_color
                };
                renderer.fill_rect(thumb, color);
            }
        }

        // Draw corner if both scrollbars visible
        if self.show_vertical_scrollbar() && self.show_horizontal_scrollbar() {
            let corner_bounds = Bounds::new(
                bounds.right() - self.scrollbar_config.width,
                bounds.bottom() - self.scrollbar_config.width,
                self.scrollbar_config.width,
                self.scrollbar_config.width,
            );
            renderer.fill_rect(corner_bounds, self.scrollbar_config.track_color);
        }
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> Option<M> {
        // Calculate viewport bounds
        let viewport_bounds = self.calc_viewport_bounds(bounds);

        // Check if position is within bounds
        let pos = event.position();
        let in_bounds = pos.map_or(false, |(x, y)| bounds.contains(x, y));

        // Handle scrollbar interactions first
        match event {
            Event::MousePress {
                button: MouseButton::Left,
                position,
                ..
            } => {
                // Check vertical scrollbar
                if let Some(thumb) = self.v_scrollbar_thumb(bounds) {
                    if thumb.contains(position.0, position.1) {
                        self.state
                            .drag
                            .start_drag_with(crate::state::ScrollDragData {
                                thumb_offset: position.1 - thumb.y,
                            });
                        return self.emit_change();
                    }

                    // Click on track (not thumb) - jump to position
                    let params = self.v_scrollbar_params(bounds);
                    if params.track_bounds.contains(position.0, position.1) {
                        // Jump scroll position - center thumb on click position
                        let thumb_y = position.1 - thumb.height / 2.0;
                        self.state.offset.1 = scrollbar::thumb_y_to_scroll_offset(
                            thumb_y,
                            params.track_bounds,
                            thumb.height,
                            self.content_size.height,
                            self.viewport_size.height,
                        );
                        self.clamp_scroll();
                        return self.emit_change();
                    }
                }

                // Check horizontal scrollbar
                if let Some(thumb) = self.h_scrollbar_thumb(bounds) {
                    if thumb.contains(position.0, position.1) {
                        self.state
                            .drag
                            .start_drag_with(crate::state::ScrollDragData {
                                thumb_offset: position.0 - thumb.x,
                            });
                        return self.emit_change();
                    }

                    // Click on track (not thumb) - jump to position
                    let params = self.h_scrollbar_params(bounds);
                    if params.track_bounds.contains(position.0, position.1) {
                        // Jump scroll position - center thumb on click position
                        let thumb_x = position.0 - thumb.width / 2.0;
                        self.state.offset.0 = scrollbar::thumb_x_to_scroll_offset(
                            thumb_x,
                            params.track_bounds,
                            thumb.width,
                            self.content_size.width,
                            self.viewport_size.width,
                        );
                        self.clamp_scroll();
                        return self.emit_change();
                    }
                }
            }

            Event::MouseRelease {
                button: MouseButton::Left,
                ..
            } => {
                if self.state.drag.is_dragging() {
                    self.state.drag.stop_drag();
                    return self.emit_change();
                }
            }

            Event::CursorLeft => {
                // Cursor left window - release any drag state
                if self.state.drag.is_dragging() {
                    self.state.drag.stop_drag();
                    log::debug!("Scrollable: stopped dragging (cursor left window)");
                    return self.emit_change();
                }
            }

            Event::MouseMove { position, .. } => {
                // Update hover state
                self.hover_v_scrollbar = self
                    .v_scrollbar_thumb(bounds)
                    .map_or(false, |t| t.contains(position.0, position.1));
                self.hover_h_scrollbar = self
                    .h_scrollbar_thumb(bounds)
                    .map_or(false, |t| t.contains(position.0, position.1));

                // Handle scrollbar drag
                if let Some(drag_offset) = self.state.drag.thumb_offset() {
                    // Vertical scrollbar drag
                    if self.show_vertical_scrollbar() && !self.hover_h_scrollbar {
                        if let Some(thumb) = self.v_scrollbar_thumb(bounds) {
                            let params = self.v_scrollbar_params(bounds);
                            let thumb_y = position.1 - drag_offset;
                            let available_travel = params.track_bounds.height - thumb.height;
                            let max_thumb_y = params.track_bounds.y + available_travel;
                            log::debug!(
                                "Scrollbar drag: position.1={:.1}, drag_offset={:.1}, thumb_y={:.1}, track.y={:.1}, track.h={:.1}, thumb.h={:.1}, avail_travel={:.1}, max_thumb_y={:.1}",
                                position.1, drag_offset, thumb_y, params.track_bounds.y, params.track_bounds.height, thumb.height, available_travel, max_thumb_y
                            );
                            self.state.offset.1 = scrollbar::thumb_y_to_scroll_offset(
                                thumb_y,
                                params.track_bounds,
                                thumb.height,
                                self.content_size.height,
                                self.viewport_size.height,
                            );
                            self.clamp_scroll();
                            return self.emit_change();
                        }
                    }

                    // Horizontal scrollbar drag
                    if self.show_horizontal_scrollbar() && !self.hover_v_scrollbar {
                        if let Some(thumb) = self.h_scrollbar_thumb(bounds) {
                            let params = self.h_scrollbar_params(bounds);
                            let thumb_x = position.0 - drag_offset;
                            self.state.offset.0 = scrollbar::thumb_x_to_scroll_offset(
                                thumb_x,
                                params.track_bounds,
                                thumb.width,
                                self.content_size.width,
                                self.viewport_size.width,
                            );
                            self.clamp_scroll();
                            return self.emit_change();
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
                let content_bounds = self.calc_content_bounds_for_events(viewport_bounds);

                // If event is for an overlay, forward to children so they can handle it
                if *overlay_hint {
                    let adjusted_event = Event::MouseScroll {
                        delta: *delta,
                        position: (
                            position.0 + self.state.offset.0,
                            position.1 + self.state.offset.1,
                        ),
                        modifiers: *modifiers,
                        overlay_hint: *overlay_hint,
                    };
                    if let Some(msg) = self.content.on_event(&adjusted_event, content_bounds) {
                        return Some(msg);
                    }
                    // Overlay handled it (even without message)
                    return None;
                }

                // Only handle scroll within bounds for normal scrolling behavior
                if in_bounds {
                    // Check if position is within viewport (for forwarding to children)
                    let in_viewport = viewport_bounds.contains(position.0, position.1);

                    // Forward to children if within viewport
                    if in_viewport {
                        let adjusted_event = Event::MouseScroll {
                            delta: *delta,
                            position: (
                                position.0 + self.state.offset.0,
                                position.1 + self.state.offset.1,
                            ),
                            modifiers: *modifiers,
                            overlay_hint: *overlay_hint,
                        };
                        if let Some(msg) = self.content.on_event(&adjusted_event, content_bounds) {
                            // Child consumed the scroll event
                            return Some(msg);
                        }
                    }

                    // Apply scroll based on direction (only if child didn't consume it)
                    let (scroll_x, scroll_y) = match self.direction {
                        ScrollDirection::Vertical => (0.0, -delta.1 * SCROLL_SPEED),
                        ScrollDirection::Horizontal => (-delta.0 * SCROLL_SPEED, 0.0),
                        ScrollDirection::Both => (-delta.0 * SCROLL_SPEED, -delta.1 * SCROLL_SPEED),
                    };

                    if scroll_x != 0.0 || scroll_y != 0.0 {
                        self.state.offset.0 += scroll_x;
                        self.state.offset.1 += scroll_y;
                        self.clamp_scroll();

                        return self.emit_change();
                    }
                }
            }

            _ => {}
        }

        // Forward events to content if within viewport
        // Note: MouseRelease must always be forwarded to children (e.g., for buttons)
        // regardless of position, to handle cases where mouse moves slightly during click
        if !self.state.drag.is_dragging() {
            // Check if content has an active overlay that might extend outside viewport
            let content_bounds = self.calc_content_bounds_for_events(viewport_bounds);
            let _overlay_capture = if self.content.has_active_overlay() {
                self.content.capture_bounds(content_bounds)
            } else {
                None
            };

            // Adjust event position for scroll offset
            let adjusted_event = match event {
                Event::MousePress {
                    button,
                    position,
                    modifiers,
                    screen_position,
                    overlay_hint,
                } => {
                    // Allow if within viewport OR if event is for an overlay
                    let in_viewport = viewport_bounds.contains(position.0, position.1);

                    if in_viewport || *overlay_hint {
                        Some(Event::MousePress {
                            button: *button,
                            position: (
                                position.0 + self.state.offset.0,
                                position.1 + self.state.offset.1,
                            ),
                            modifiers: *modifiers,
                            // Preserve or set the original screen position
                            screen_position: screen_position.or(Some(*position)),
                            overlay_hint: *overlay_hint,
                        })
                    } else {
                        None
                    }
                }
                Event::MouseRelease {
                    button,
                    position,
                    modifiers,
                    overlay_hint,
                } => {
                    // Always forward MouseRelease to children - they may need it
                    // even if mouse moved outside (e.g., button clicks)
                    Some(Event::MouseRelease {
                        button: *button,
                        position: (
                            position.0 + self.state.offset.0,
                            position.1 + self.state.offset.1,
                        ),
                        modifiers: *modifiers,
                        overlay_hint: *overlay_hint,
                    })
                }
                Event::MouseMove {
                    position,
                    modifiers,
                    overlay_hint,
                } => {
                    // Allow if within viewport, if event is for an overlay, or if content has active drag
                    let in_viewport = viewport_bounds.contains(position.0, position.1);
                    let content_has_drag = self.content.has_active_drag();

                    if in_viewport || *overlay_hint || content_has_drag {
                        Some(Event::MouseMove {
                            position: (
                                position.0 + self.state.offset.0,
                                position.1 + self.state.offset.1,
                            ),
                            modifiers: *modifiers,
                            overlay_hint: *overlay_hint,
                        })
                    } else {
                        None
                    }
                }
                Event::GlobalMousePress { button, position } => {
                    // Adjust GlobalMousePress position for scroll offset so that
                    // child widgets (like dropdowns) can correctly determine if
                    // the click is inside/outside their bounds
                    Some(Event::GlobalMousePress {
                        button: *button,
                        position: (
                            position.0 + self.state.offset.0,
                            position.1 + self.state.offset.1,
                        ),
                    })
                }
                _ => Some(event.clone()),
            };

            if let Some(evt) = adjusted_event {
                let content_bounds = self.calc_content_bounds_for_events(viewport_bounds);
                return self.content.on_event(&evt, content_bounds);
            }
        }

        None
    }
}
