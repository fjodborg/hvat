//! Scrollable container widget

use crate::constants::{SCROLLBAR_MIN_THUMB, SCROLLBAR_WIDTH, SCROLL_SPEED};
use crate::element::Element;
use crate::event::{Event, MouseButton};
use crate::layout::{Bounds, Length, Padding, Size};
use crate::renderer::{Color, Renderer};
use crate::state::ScrollState;
use crate::widget::Widget;

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
    on_scroll: Option<Box<dyn Fn(ScrollState) -> M>>,
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
            state: ScrollState::new(),
            direction: ScrollDirection::default(),
            scrollbar_visibility: ScrollbarVisibility::default(),
            scrollbar_config: ScrollbarConfig::default(),
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            padding: Padding::ZERO,
            on_scroll: None,
            content_size: Size::ZERO,
            viewport_size: Size::ZERO,
            hover_v_scrollbar: false,
            hover_h_scrollbar: false,
        }
    }

    /// Set the scroll state (clones from external state)
    pub fn state(mut self, state: &ScrollState) -> Self {
        self.state = state.clone();
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
        self.on_scroll = Some(Box::new(callback));
        self
    }

    /// Check if vertical scrolling is needed
    fn needs_vertical_scroll(&self) -> bool {
        matches!(self.direction, ScrollDirection::Vertical | ScrollDirection::Both)
            && self.content_size.height > self.viewport_size.height
    }

    /// Check if horizontal scrolling is needed
    fn needs_horizontal_scroll(&self) -> bool {
        matches!(self.direction, ScrollDirection::Horizontal | ScrollDirection::Both)
            && self.content_size.width > self.viewport_size.width
    }

    /// Check if vertical scrollbar should be shown
    fn show_vertical_scrollbar(&self) -> bool {
        match self.scrollbar_visibility {
            ScrollbarVisibility::Always => {
                matches!(self.direction, ScrollDirection::Vertical | ScrollDirection::Both)
            }
            ScrollbarVisibility::Auto => self.needs_vertical_scroll(),
            ScrollbarVisibility::Never => false,
        }
    }

    /// Check if horizontal scrollbar should be shown
    fn show_horizontal_scrollbar(&self) -> bool {
        match self.scrollbar_visibility {
            ScrollbarVisibility::Always => {
                matches!(self.direction, ScrollDirection::Horizontal | ScrollDirection::Both)
            }
            ScrollbarVisibility::Auto => self.needs_horizontal_scroll(),
            ScrollbarVisibility::Never => false,
        }
    }

    /// Calculate vertical scrollbar thumb geometry
    fn v_scrollbar_thumb(&self, bounds: Bounds) -> Option<Bounds> {
        if !self.show_vertical_scrollbar() || self.content_size.height <= 0.0 {
            return None;
        }

        let track_height = bounds.height
            - if self.show_horizontal_scrollbar() {
                self.scrollbar_config.width
            } else {
                0.0
            };

        let visible_ratio = (self.viewport_size.height / self.content_size.height).min(1.0);
        let thumb_height = (track_height * visible_ratio).max(self.scrollbar_config.min_thumb_size);

        let max_scroll = (self.content_size.height - self.viewport_size.height).max(0.0);
        let scroll_ratio = if max_scroll > 0.0 {
            (self.state.offset.1 / max_scroll).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let available_travel = track_height - thumb_height;
        let thumb_y = bounds.y + scroll_ratio * available_travel;

        Some(Bounds::new(
            bounds.right() - self.scrollbar_config.width,
            thumb_y,
            self.scrollbar_config.width,
            thumb_height,
        ))
    }

    /// Calculate horizontal scrollbar thumb geometry
    fn h_scrollbar_thumb(&self, bounds: Bounds) -> Option<Bounds> {
        if !self.show_horizontal_scrollbar() || self.content_size.width <= 0.0 {
            return None;
        }

        let track_width = bounds.width
            - if self.show_vertical_scrollbar() {
                self.scrollbar_config.width
            } else {
                0.0
            };

        let visible_ratio = (self.viewport_size.width / self.content_size.width).min(1.0);
        let thumb_width = (track_width * visible_ratio).max(self.scrollbar_config.min_thumb_size);

        let max_scroll = (self.content_size.width - self.viewport_size.width).max(0.0);
        let scroll_ratio = if max_scroll > 0.0 {
            (self.state.offset.0 / max_scroll).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let available_travel = track_width - thumb_width;
        let thumb_x = bounds.x + scroll_ratio * available_travel;

        Some(Bounds::new(
            thumb_x,
            bounds.bottom() - self.scrollbar_config.width,
            thumb_width,
            self.scrollbar_config.width,
        ))
    }

    /// Clamp scroll offset to valid range
    fn clamp_scroll(&mut self) {
        let max_x = (self.content_size.width - self.viewport_size.width).max(0.0);
        let max_y = (self.content_size.height - self.viewport_size.height).max(0.0);

        self.state.offset.0 = self.state.offset.0.clamp(0.0, max_x);
        self.state.offset.1 = self.state.offset.1.clamp(0.0, max_y);
    }

    /// Emit a state change if handler is set
    fn emit_change(&self) -> Option<M> {
        self.on_scroll.as_ref().map(|f| f(self.state.clone()))
    }
}

impl<M: 'static> Widget<M> for Scrollable<M> {
    fn has_active_overlay(&self) -> bool {
        self.content.has_active_overlay()
    }

    fn has_active_drag(&self) -> bool {
        // Either the scrollbar itself is being dragged, or a child widget is being dragged
        self.state.dragging || self.content.has_active_drag()
    }

    fn capture_bounds(&self, layout_bounds: Bounds) -> Option<Bounds> {
        // If content has an overlay, expand our capture bounds to include it
        if self.content.has_active_overlay() {
            let viewport_bounds = Bounds::new(
                layout_bounds.x + self.padding.left,
                layout_bounds.y + self.padding.top,
                self.viewport_size.width,
                self.viewport_size.height,
            );
            let content_bounds = Bounds::new(
                viewport_bounds.x,
                viewport_bounds.y,
                self.content_size.width,
                self.content_size.height,
            );
            // Get the content's capture bounds and translate them
            if let Some(content_capture) = self.content.capture_bounds(content_bounds) {
                // The content's capture bounds are in scrolled content space,
                // we need to translate them back to screen space
                let screen_capture = Bounds::new(
                    content_capture.x - self.state.offset.0,
                    content_capture.y - self.state.offset.1,
                    content_capture.width,
                    content_capture.height,
                );
                // Return union of layout bounds and the overlay's screen bounds
                return Some(layout_bounds.union(&screen_capture));
            }
        }
        None
    }

    fn layout(&mut self, available: Size) -> Size {
        log::debug!("Scrollable layout: available={:?}", available);

        // Resolve our own size
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
            let needs_v = matches!(self.direction, ScrollDirection::Vertical | ScrollDirection::Both)
                && self.content_size.height > viewport_height;
            let needs_h = matches!(self.direction, ScrollDirection::Horizontal | ScrollDirection::Both)
                && self.content_size.width > viewport_width;

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

        self.viewport_size = Size::new(viewport_width, viewport_height);

        // Clamp scroll offset to valid range
        self.clamp_scroll();

        Size::new(own_width, own_height)
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Bounds) {
        log::debug!("Scrollable draw: bounds={:?}", bounds);

        // Calculate viewport bounds (excluding scrollbars)
        let viewport_bounds = Bounds::new(
            bounds.x + self.padding.left,
            bounds.y + self.padding.top,
            self.viewport_size.width,
            self.viewport_size.height,
        );

        // Push clip for viewport
        renderer.push_clip(viewport_bounds);

        // Draw content with scroll offset applied
        let content_bounds = Bounds::new(
            viewport_bounds.x - self.state.offset.0,
            viewport_bounds.y - self.state.offset.1,
            self.content_size.width,
            self.content_size.height,
        );
        self.content.draw(renderer, content_bounds);

        renderer.pop_clip();

        // Draw vertical scrollbar
        if self.show_vertical_scrollbar() {
            let track_bounds = Bounds::new(
                bounds.right() - self.scrollbar_config.width,
                bounds.y,
                self.scrollbar_config.width,
                bounds.height
                    - if self.show_horizontal_scrollbar() {
                        self.scrollbar_config.width
                    } else {
                        0.0
                    },
            );

            // Draw track
            renderer.fill_rect(track_bounds, self.scrollbar_config.track_color);

            // Draw thumb
            if let Some(thumb) = self.v_scrollbar_thumb(bounds) {
                let color = if self.state.dragging {
                    self.scrollbar_config.thumb_drag_color
                } else if self.hover_v_scrollbar {
                    self.scrollbar_config.thumb_hover_color
                } else {
                    self.scrollbar_config.thumb_color
                };
                renderer.fill_rect(thumb, color);
            }
        }

        // Draw horizontal scrollbar
        if self.show_horizontal_scrollbar() {
            let track_bounds = Bounds::new(
                bounds.x,
                bounds.bottom() - self.scrollbar_config.width,
                bounds.width
                    - if self.show_vertical_scrollbar() {
                        self.scrollbar_config.width
                    } else {
                        0.0
                    },
                self.scrollbar_config.width,
            );

            // Draw track
            renderer.fill_rect(track_bounds, self.scrollbar_config.track_color);

            // Draw thumb
            if let Some(thumb) = self.h_scrollbar_thumb(bounds) {
                let color = if self.state.dragging {
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
        let viewport_bounds = Bounds::new(
            bounds.x + self.padding.left,
            bounds.y + self.padding.top,
            self.viewport_size.width,
            self.viewport_size.height,
        );

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
                        self.state.dragging = true;
                        self.state.drag_start_offset = Some(position.1 - thumb.y);
                        return self.emit_change();
                    }

                    // Click on track (not thumb) - jump to position
                    let track_bounds = Bounds::new(
                        bounds.right() - self.scrollbar_config.width,
                        bounds.y,
                        self.scrollbar_config.width,
                        bounds.height
                            - if self.show_horizontal_scrollbar() {
                                self.scrollbar_config.width
                            } else {
                                0.0
                            },
                    );
                    if track_bounds.contains(position.0, position.1) {
                        // Jump scroll position
                        let track_height = track_bounds.height;
                        let thumb_height = thumb.height;
                        let click_pos = position.1 - bounds.y - thumb_height / 2.0;
                        let available_travel = track_height - thumb_height;
                        let scroll_ratio = (click_pos / available_travel).clamp(0.0, 1.0);
                        let max_scroll = (self.content_size.height - self.viewport_size.height).max(0.0);
                        self.state.offset.1 = scroll_ratio * max_scroll;
                        self.clamp_scroll();
                        return self.emit_change();
                    }
                }

                // Check horizontal scrollbar
                if let Some(thumb) = self.h_scrollbar_thumb(bounds) {
                    if thumb.contains(position.0, position.1) {
                        self.state.dragging = true;
                        self.state.drag_start_offset = Some(position.0 - thumb.x);
                        return self.emit_change();
                    }

                    // Click on track (not thumb) - jump to position
                    let track_bounds = Bounds::new(
                        bounds.x,
                        bounds.bottom() - self.scrollbar_config.width,
                        bounds.width
                            - if self.show_vertical_scrollbar() {
                                self.scrollbar_config.width
                            } else {
                                0.0
                            },
                        self.scrollbar_config.width,
                    );
                    if track_bounds.contains(position.0, position.1) {
                        // Jump scroll position
                        let track_width = track_bounds.width;
                        let thumb_width = thumb.width;
                        let click_pos = position.0 - bounds.x - thumb_width / 2.0;
                        let available_travel = track_width - thumb_width;
                        let scroll_ratio = (click_pos / available_travel).clamp(0.0, 1.0);
                        let max_scroll = (self.content_size.width - self.viewport_size.width).max(0.0);
                        self.state.offset.0 = scroll_ratio * max_scroll;
                        self.clamp_scroll();
                        return self.emit_change();
                    }
                }
            }

            Event::MouseRelease {
                button: MouseButton::Left,
                ..
            } => {
                if self.state.dragging {
                    self.state.dragging = false;
                    self.state.drag_start_offset = None;
                    return self.emit_change();
                }
            }

            Event::CursorLeft => {
                // Cursor left window - release any drag state
                if self.state.dragging {
                    self.state.dragging = false;
                    self.state.drag_start_offset = None;
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
                if self.state.dragging {
                    if let Some(drag_offset) = self.state.drag_start_offset {
                        // Vertical scrollbar drag
                        if self.show_vertical_scrollbar() && !self.hover_h_scrollbar {
                            if let Some(thumb) = self.v_scrollbar_thumb(bounds) {
                                let track_height = bounds.height
                                    - if self.show_horizontal_scrollbar() {
                                        self.scrollbar_config.width
                                    } else {
                                        0.0
                                    };
                                let thumb_height = thumb.height;
                                let available_travel = track_height - thumb_height;

                                if available_travel > 0.0 {
                                    let thumb_y = position.1 - bounds.y - drag_offset;
                                    let scroll_ratio = (thumb_y / available_travel).clamp(0.0, 1.0);
                                    let max_scroll =
                                        (self.content_size.height - self.viewport_size.height).max(0.0);
                                    self.state.offset.1 = scroll_ratio * max_scroll;
                                    self.clamp_scroll();
                                    return self.emit_change();
                                }
                            }
                        }

                        // Horizontal scrollbar drag
                        if self.show_horizontal_scrollbar() && !self.hover_v_scrollbar {
                            if let Some(thumb) = self.h_scrollbar_thumb(bounds) {
                                let track_width = bounds.width
                                    - if self.show_vertical_scrollbar() {
                                        self.scrollbar_config.width
                                    } else {
                                        0.0
                                    };
                                let thumb_width = thumb.width;
                                let available_travel = track_width - thumb_width;

                                if available_travel > 0.0 {
                                    let thumb_x = position.0 - bounds.x - drag_offset;
                                    let scroll_ratio = (thumb_x / available_travel).clamp(0.0, 1.0);
                                    let max_scroll =
                                        (self.content_size.width - self.viewport_size.width).max(0.0);
                                    self.state.offset.0 = scroll_ratio * max_scroll;
                                    self.clamp_scroll();
                                    return self.emit_change();
                                }
                            }
                        }
                    }
                }
            }

            Event::MouseScroll { delta, position, modifiers, overlay_hint } => {
                let content_bounds = Bounds::new(
                    viewport_bounds.x,
                    viewport_bounds.y,
                    self.content_size.width,
                    self.content_size.height,
                );

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
        if !self.state.dragging {
            // Check if content has an active overlay that might extend outside viewport
            let content_bounds = Bounds::new(
                viewport_bounds.x,
                viewport_bounds.y,
                self.content_size.width,
                self.content_size.height,
            );
            let overlay_capture = if self.content.has_active_overlay() {
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
                Event::MouseMove { position, modifiers, overlay_hint } => {
                    // Allow if within viewport OR if event is for an overlay
                    let in_viewport = viewport_bounds.contains(position.0, position.1);

                    if in_viewport || *overlay_hint {
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
                _ => Some(event.clone()),
            };

            if let Some(evt) = adjusted_event {
                let content_bounds = Bounds::new(
                    viewport_bounds.x,
                    viewport_bounds.y,
                    self.content_size.width,
                    self.content_size.height,
                );
                return self.content.on_event(&evt, content_bounds);
            }
        }

        None
    }
}
