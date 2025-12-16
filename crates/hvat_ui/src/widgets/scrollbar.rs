//! Reusable scrollbar calculation and rendering utilities
//!
//! This module provides shared functions for scrollbar thumb calculation,
//! scroll offset mapping, and scrollbar rendering used by Scrollable, Dropdown,
//! and Collapsible widgets.

use crate::constants::{SCROLLBAR_MIN_THUMB, SCROLLBAR_WIDTH};
use crate::layout::Bounds;
use crate::renderer::{Color, Renderer};

/// Scrollbar orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarOrientation {
    Vertical,
    Horizontal,
}

/// Parameters for scrollbar thumb calculation
#[derive(Debug, Clone, Copy)]
pub struct ScrollbarParams {
    /// Total content size (height for vertical, width for horizontal)
    pub content_size: f32,
    /// Visible viewport size
    pub viewport_size: f32,
    /// Current scroll offset
    pub scroll_offset: f32,
    /// Track bounds (where the scrollbar is drawn)
    pub track_bounds: Bounds,
    /// Scrollbar width/height (perpendicular to scroll direction)
    pub bar_size: f32,
    /// Minimum thumb size
    pub min_thumb_size: f32,
}

impl ScrollbarParams {
    /// Create params with default scrollbar size
    pub fn new(
        content_size: f32,
        viewport_size: f32,
        scroll_offset: f32,
        track_bounds: Bounds,
    ) -> Self {
        Self {
            content_size,
            viewport_size,
            scroll_offset,
            track_bounds,
            bar_size: SCROLLBAR_WIDTH,
            min_thumb_size: SCROLLBAR_MIN_THUMB,
        }
    }

    /// Set custom bar size
    pub fn with_bar_size(mut self, size: f32) -> Self {
        self.bar_size = size;
        self
    }

    /// Set custom minimum thumb size
    pub fn with_min_thumb(mut self, size: f32) -> Self {
        self.min_thumb_size = size;
        self
    }
}

/// Result of scrollbar thumb calculation
#[derive(Debug, Clone, Copy)]
pub struct ScrollbarThumb {
    /// Thumb bounds
    pub bounds: Bounds,
    /// Ratio of scroll position (0.0 to 1.0)
    pub scroll_ratio: f32,
    /// Whether scrolling is needed (content > viewport)
    pub needs_scroll: bool,
}

// =============================================================================
// Thumb Calculation
// =============================================================================

/// Calculate vertical scrollbar thumb bounds and position
///
/// Returns None if scrolling is not needed (content fits in viewport)
pub fn calculate_vertical_thumb(params: &ScrollbarParams) -> Option<ScrollbarThumb> {
    if params.content_size <= params.viewport_size || params.content_size <= 0.0 {
        return None;
    }

    let track = &params.track_bounds;
    let track_height = track.height;

    // Calculate thumb size based on visible ratio
    let visible_ratio = (params.viewport_size / params.content_size).min(1.0);
    let thumb_height = (track_height * visible_ratio).max(params.min_thumb_size);

    // Calculate scroll ratio
    let max_scroll = (params.content_size - params.viewport_size).max(0.0);
    let scroll_ratio = if max_scroll > 0.0 {
        (params.scroll_offset / max_scroll).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Calculate thumb position
    let available_travel = track_height - thumb_height;
    let thumb_y = track.y + scroll_ratio * available_travel;

    Some(ScrollbarThumb {
        bounds: Bounds::new(
            track.right() - params.bar_size,
            thumb_y,
            params.bar_size,
            thumb_height,
        ),
        scroll_ratio,
        needs_scroll: true,
    })
}

/// Calculate horizontal scrollbar thumb bounds and position
///
/// Returns None if scrolling is not needed (content fits in viewport)
pub fn calculate_horizontal_thumb(params: &ScrollbarParams) -> Option<ScrollbarThumb> {
    if params.content_size <= params.viewport_size || params.content_size <= 0.0 {
        return None;
    }

    let track = &params.track_bounds;
    let track_width = track.width;

    // Calculate thumb size based on visible ratio
    let visible_ratio = (params.viewport_size / params.content_size).min(1.0);
    let thumb_width = (track_width * visible_ratio).max(params.min_thumb_size);

    // Calculate scroll ratio
    let max_scroll = (params.content_size - params.viewport_size).max(0.0);
    let scroll_ratio = if max_scroll > 0.0 {
        (params.scroll_offset / max_scroll).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Calculate thumb position
    let available_travel = track_width - thumb_width;
    let thumb_x = track.x + scroll_ratio * available_travel;

    Some(ScrollbarThumb {
        bounds: Bounds::new(
            thumb_x,
            track.bottom() - params.bar_size,
            thumb_width,
            params.bar_size,
        ),
        scroll_ratio,
        needs_scroll: true,
    })
}

// =============================================================================
// Scroll Offset from Thumb Position
// =============================================================================

/// Convert a thumb Y position to a scroll offset (for vertical scrollbar)
pub fn thumb_y_to_scroll_offset(
    thumb_y: f32,
    track_bounds: Bounds,
    thumb_height: f32,
    content_size: f32,
    viewport_size: f32,
) -> f32 {
    let available_travel = track_bounds.height - thumb_height;
    if available_travel <= 0.0 {
        return 0.0;
    }

    let ratio = ((thumb_y - track_bounds.y) / available_travel).clamp(0.0, 1.0);
    let max_scroll = (content_size - viewport_size).max(0.0);
    ratio * max_scroll
}

/// Convert a thumb X position to a scroll offset (for horizontal scrollbar)
pub fn thumb_x_to_scroll_offset(
    thumb_x: f32,
    track_bounds: Bounds,
    thumb_width: f32,
    content_size: f32,
    viewport_size: f32,
) -> f32 {
    let available_travel = track_bounds.width - thumb_width;
    if available_travel <= 0.0 {
        return 0.0;
    }

    let ratio = ((thumb_x - track_bounds.x) / available_travel).clamp(0.0, 1.0);
    let max_scroll = (content_size - viewport_size).max(0.0);
    ratio * max_scroll
}

// =============================================================================
// Scroll Clamping
// =============================================================================

/// Clamp a scroll offset to valid range
pub fn clamp_scroll_offset(offset: f32, content_size: f32, viewport_size: f32) -> f32 {
    let max_scroll = (content_size - viewport_size).max(0.0);
    offset.clamp(0.0, max_scroll)
}

/// Clamp both horizontal and vertical scroll offsets
pub fn clamp_scroll_offsets(
    offset: (f32, f32),
    content_size: (f32, f32),
    viewport_size: (f32, f32),
) -> (f32, f32) {
    (
        clamp_scroll_offset(offset.0, content_size.0, viewport_size.0),
        clamp_scroll_offset(offset.1, content_size.1, viewport_size.1),
    )
}

// =============================================================================
// Rendering
// =============================================================================

/// Colors for scrollbar rendering
#[derive(Debug, Clone, Copy)]
pub struct ScrollbarColors {
    pub track: Color,
    pub thumb: Color,
    pub thumb_hover: Color,
    pub thumb_drag: Color,
}

impl Default for ScrollbarColors {
    fn default() -> Self {
        Self {
            track: Color::rgba(0.15, 0.15, 0.18, 0.5),
            thumb: Color::rgba(0.4, 0.4, 0.45, 0.8),
            thumb_hover: Color::rgba(0.5, 0.5, 0.55, 0.9),
            thumb_drag: Color::rgba(0.6, 0.6, 0.65, 1.0),
        }
    }
}

/// Draw a vertical scrollbar track
pub fn draw_vertical_track(renderer: &mut Renderer, bounds: Bounds, color: Color) {
    renderer.fill_rect(bounds, color);
}

/// Draw a scrollbar thumb with state-based color
pub fn draw_thumb(
    renderer: &mut Renderer,
    bounds: Bounds,
    colors: &ScrollbarColors,
    is_hovered: bool,
    is_dragging: bool,
) {
    let color = if is_dragging {
        colors.thumb_drag
    } else if is_hovered {
        colors.thumb_hover
    } else {
        colors.thumb
    };
    renderer.fill_rect(bounds, color);
}

/// Draw a complete vertical scrollbar (track + thumb)
pub fn draw_vertical_scrollbar(
    renderer: &mut Renderer,
    track_bounds: Bounds,
    thumb: &ScrollbarThumb,
    colors: &ScrollbarColors,
    is_hovered: bool,
    is_dragging: bool,
) {
    draw_vertical_track(renderer, track_bounds, colors.track);
    draw_thumb(renderer, thumb.bounds, colors, is_hovered, is_dragging);
}

/// Draw a complete horizontal scrollbar (track + thumb)
pub fn draw_horizontal_scrollbar(
    renderer: &mut Renderer,
    track_bounds: Bounds,
    thumb: &ScrollbarThumb,
    colors: &ScrollbarColors,
    is_hovered: bool,
    is_dragging: bool,
) {
    // Track is horizontal, at the bottom
    renderer.fill_rect(track_bounds, colors.track);
    draw_thumb(renderer, thumb.bounds, colors, is_hovered, is_dragging);
}

// =============================================================================
// Hit Testing
// =============================================================================

/// Check if a point is inside the vertical scrollbar track area
pub fn point_in_vertical_track(x: f32, y: f32, track_bounds: Bounds, bar_size: f32) -> bool {
    let track_x = track_bounds.right() - bar_size;
    x >= track_x && x <= track_bounds.right() && y >= track_bounds.y && y <= track_bounds.bottom()
}

/// Check if a point is inside the horizontal scrollbar track area
pub fn point_in_horizontal_track(x: f32, y: f32, track_bounds: Bounds, bar_size: f32) -> bool {
    let track_y = track_bounds.bottom() - bar_size;
    x >= track_bounds.x && x <= track_bounds.right() && y >= track_y && y <= track_bounds.bottom()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertical_thumb_not_needed() {
        let params = ScrollbarParams::new(100.0, 200.0, 0.0, Bounds::new(0.0, 0.0, 100.0, 200.0));
        assert!(calculate_vertical_thumb(&params).is_none());
    }

    #[test]
    fn test_vertical_thumb_at_start() {
        let params = ScrollbarParams::new(400.0, 200.0, 0.0, Bounds::new(0.0, 0.0, 100.0, 200.0));
        let thumb = calculate_vertical_thumb(&params).unwrap();
        assert!((thumb.scroll_ratio - 0.0).abs() < 0.001);
        assert!(thumb.needs_scroll);
    }

    #[test]
    fn test_vertical_thumb_at_middle() {
        let params = ScrollbarParams::new(400.0, 200.0, 100.0, Bounds::new(0.0, 0.0, 100.0, 200.0));
        let thumb = calculate_vertical_thumb(&params).unwrap();
        assert!((thumb.scroll_ratio - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_vertical_thumb_at_end() {
        let params = ScrollbarParams::new(400.0, 200.0, 200.0, Bounds::new(0.0, 0.0, 100.0, 200.0));
        let thumb = calculate_vertical_thumb(&params).unwrap();
        assert!((thumb.scroll_ratio - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_clamp_scroll_offset() {
        assert!((clamp_scroll_offset(-10.0, 400.0, 200.0) - 0.0).abs() < 0.001);
        assert!((clamp_scroll_offset(100.0, 400.0, 200.0) - 100.0).abs() < 0.001);
        assert!((clamp_scroll_offset(300.0, 400.0, 200.0) - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_thumb_y_to_scroll() {
        let track = Bounds::new(0.0, 0.0, 100.0, 200.0);
        let thumb_height = 50.0;
        let content_size = 400.0;
        let viewport_size = 200.0;

        // At top
        let scroll = thumb_y_to_scroll_offset(0.0, track, thumb_height, content_size, viewport_size);
        assert!((scroll - 0.0).abs() < 0.001);

        // At bottom (thumb_y = track_height - thumb_height = 150)
        let scroll =
            thumb_y_to_scroll_offset(150.0, track, thumb_height, content_size, viewport_size);
        assert!((scroll - 200.0).abs() < 0.001);
    }
}
