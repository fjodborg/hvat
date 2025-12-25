//! Reusable scrollbar calculation and rendering utilities
//!
//! This module provides shared functions for scrollbar thumb calculation,
//! scroll offset mapping, and scrollbar rendering used by Scrollable, Dropdown,
//! and Collapsible widgets.
//!
//! ## Coordinate Space Conventions
//!
//! Scrolling involves two coordinate spaces:
//!
//! - **Viewport space**: The visible region on screen (what the user sees)
//! - **Content space**: The full content area (may be larger than viewport)
//!
//! ### Converting between spaces:
//!
//! - **Event positions** (from mouse events) are in viewport/screen space
//! - To convert to content space: `content_pos = viewport_pos + scroll_offset`
//! - **Drawing content** at the correct position:
//!   - Content is drawn at: `viewport_pos - scroll_offset`
//!
//! ### When to add vs subtract scroll offset:
//!
//! | Operation | Formula | Reason |
//! |-----------|---------|--------|
//! | Event pos → content pos | `+offset` | Scroll down = content moved up, so add to get content position |
//! | Content pos → draw pos | `-offset` | Drawing higher content means drawing further up (negative Y) |
//! | capture_bounds | No adjustment | Bounds are already in screen space |
//!
//! ## Usage Examples
//!
//! ```rust,ignore
//! use hvat_ui::widgets::scrollbar::{ScrollbarParams, calculate_vertical_thumb};
//!
//! // Create params for a scrollable area
//! let params = ScrollbarParams::new(
//!     content_height,     // Total content size
//!     viewport_height,    // Visible area
//!     scroll_offset,      // Current scroll position
//!     track_bounds,       // Where to draw scrollbar
//! );
//!
//! // Calculate thumb geometry
//! if let Some(thumb) = calculate_vertical_thumb(&params) {
//!     renderer.fill_rect(thumb.bounds, thumb_color);
//! }
//!
//! // Convert mouse drag to scroll offset
//! let new_offset = thumb_y_to_scroll_offset(
//!     mouse_y - drag_offset,
//!     track_bounds,
//!     thumb_height,
//!     content_height,
//!     viewport_height,
//! );
//! ```

use crate::constants::{SCROLLBAR_MIN_THUMB, SCROLLBAR_WIDTH};
use crate::layout::Bounds;
use crate::renderer::{Color, Renderer};

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

/// Draw a simple vertical scrollbar with default colors (no hover/drag states)
///
/// This is a convenience function for widgets that don't need interactive scrollbar states
/// (like dropdown and collapsible which handle scrolling via other means).
pub fn draw_simple_vertical_scrollbar(
    renderer: &mut Renderer,
    track_bounds: Bounds,
    content_size: f32,
    viewport_size: f32,
    scroll_offset: f32,
    bar_width: f32,
) {
    // Draw track
    renderer.fill_rect(track_bounds, Color::SCROLLBAR_TRACK);

    // Calculate and draw thumb if scrolling is needed
    let max_scroll = (content_size - viewport_size).max(0.0);
    if max_scroll > 0.0 && content_size > 0.0 {
        let visible_ratio = (viewport_size / content_size).min(1.0);
        let thumb_height = (track_bounds.height * visible_ratio).max(SCROLLBAR_MIN_THUMB);

        let scroll_ratio = (scroll_offset / max_scroll).clamp(0.0, 1.0);
        let available_travel = track_bounds.height - thumb_height;
        let thumb_y = track_bounds.y + scroll_ratio * available_travel;

        let thumb_bounds = Bounds::new(track_bounds.x, thumb_y, bar_width, thumb_height);
        // Use pill-shaped thumb (radius = half width) for modern look
        let thumb_radius = bar_width / 2.0;
        renderer.fill_rounded_rect(thumb_bounds, Color::SCROLLBAR_THUMB, thumb_radius);
    }
}

/// Draw a simple horizontal scrollbar with default colors (no hover/drag states)
///
/// This is a convenience function for widgets that don't need interactive scrollbar states
/// (like dropdown and collapsible which handle scrolling via other means).
pub fn draw_simple_horizontal_scrollbar(
    renderer: &mut Renderer,
    track_bounds: Bounds,
    content_size: f32,
    viewport_size: f32,
    scroll_offset: f32,
    bar_height: f32,
) {
    // Draw track
    renderer.fill_rect(track_bounds, Color::SCROLLBAR_TRACK);

    // Calculate and draw thumb if scrolling is needed
    let max_scroll = (content_size - viewport_size).max(0.0);
    if max_scroll > 0.0 && content_size > 0.0 {
        let visible_ratio = (viewport_size / content_size).min(1.0);
        let thumb_width = (track_bounds.width * visible_ratio).max(SCROLLBAR_MIN_THUMB);

        let scroll_ratio = (scroll_offset / max_scroll).clamp(0.0, 1.0);
        let available_travel = track_bounds.width - thumb_width;
        let thumb_x = track_bounds.x + scroll_ratio * available_travel;

        let thumb_bounds = Bounds::new(thumb_x, track_bounds.y, thumb_width, bar_height);
        // Use pill-shaped thumb (radius = half height) for modern look
        let thumb_radius = bar_height / 2.0;
        renderer.fill_rounded_rect(thumb_bounds, Color::SCROLLBAR_THUMB, thumb_radius);
    }
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
        let scroll =
            thumb_y_to_scroll_offset(0.0, track, thumb_height, content_size, viewport_size);
        assert!((scroll - 0.0).abs() < 0.001);

        // At bottom (thumb_y = track_height - thumb_height = 150)
        let scroll =
            thumb_y_to_scroll_offset(150.0, track, thumb_height, content_size, viewport_size);
        assert!((scroll - 200.0).abs() < 0.001);
    }
}
