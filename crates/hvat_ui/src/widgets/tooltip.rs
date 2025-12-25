//! Tooltip widget for displaying contextual information on hover
//!
//! Tooltips are rendered as overlays at the highest z-order, appearing near the mouse cursor.
//! Use the [`TooltipManager`] to control tooltip timing and visibility.
//!
//! # Example Usage
//!
//! ```ignore
//! // In your application state:
//! tooltip_manager: TooltipManager,
//!
//! // In tick():
//! self.tooltip_manager.tick(delta_time);
//!
//! // In your widget code (e.g., on MouseMove):
//! if bounds.contains(pos.0, pos.1) {
//!     let content = TooltipContent::rich("Polygon Tool", "Hotkey: R\nDraw polygon annotations");
//!     let request = TooltipRequest::new("tool_polygon", content, bounds);
//!     tooltip_manager.request(request, pos);
//! } else {
//!     tooltip_manager.clear_if_id("tool_polygon");
//! }
//!
//! // In your view(), render the tooltip:
//! if let Some((content, pos)) = self.tooltip_manager.visible_tooltip() {
//!     // Add TooltipOverlay to your view tree (at the end for highest z-order)
//! }
//! ```

use crate::constants::{FONT_SIZE_BODY, FONT_SIZE_SMALL};
use crate::element::Element;
use crate::event::Event;
use crate::layout::{Bounds, Size};
use crate::renderer::{Color, Renderer};
use crate::state::TooltipContent;
use crate::theme::Theme;
use crate::widget::{EventResult, Widget};

/// Small spacing constant for tooltip layout
const SPACING_SM: f32 = 8.0;
/// Extra small spacing for tooltip layout
const SPACING_XS: f32 = 4.0;

/// Configuration for tooltip appearance
#[derive(Debug, Clone)]
pub struct TooltipConfig {
    /// Maximum width for the tooltip (text will wrap)
    pub max_width: f32,
    /// Padding inside the tooltip
    pub padding: f32,
    /// Background color
    pub background: Color,
    /// Border color
    pub border_color: Color,
    /// Border width
    pub border_width: f32,
    /// Title text color (for rich tooltips)
    pub title_color: Color,
    /// Body text color
    pub body_color: Color,
    /// Font size for title
    pub title_size: f32,
    /// Font size for body
    pub body_size: f32,
    /// Offset from cursor position
    pub cursor_offset: (f32, f32),
}

impl Default for TooltipConfig {
    fn default() -> Self {
        Self {
            max_width: 250.0,
            padding: SPACING_SM,
            background: Color::rgba(0.15, 0.15, 0.18, 0.95),
            border_color: Color::rgba(0.4, 0.4, 0.45, 0.8),
            border_width: 1.0,
            title_color: Color::rgb(0.95, 0.95, 0.97),
            body_color: Color::rgb(0.8, 0.8, 0.85),
            title_size: FONT_SIZE_BODY,
            body_size: FONT_SIZE_SMALL,
            cursor_offset: (12.0, 12.0),
        }
    }
}

impl TooltipConfig {
    /// Create a tooltip config with theme colors
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            background: theme.popup_bg.darken(0.1),
            border_color: theme.border,
            title_color: theme.text_primary,
            body_color: theme.text_secondary,
            ..Default::default()
        }
    }
}

/// Tooltip overlay widget
///
/// Renders a tooltip at a specific screen position. This should be added
/// to your view tree when [`TooltipManager::is_visible()`] returns true.
///
/// The tooltip renders as an overlay (above all other content) and does not
/// capture mouse events.
pub struct TooltipOverlay {
    /// The content to display
    content: TooltipContent,
    /// Screen position (typically mouse cursor position)
    position: (f32, f32),
    /// Configuration for appearance
    config: TooltipConfig,
    /// Window size for boundary checking
    window_size: (f32, f32),
}

impl TooltipOverlay {
    /// Create a new tooltip overlay
    pub fn new(content: TooltipContent, position: (f32, f32)) -> Self {
        Self {
            content,
            position,
            config: TooltipConfig::default(),
            window_size: (1024.0, 768.0), // Default, will be updated
        }
    }

    /// Set the tooltip configuration
    pub fn config(mut self, config: TooltipConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the window size for boundary checking
    pub fn window_size(mut self, width: f32, height: f32) -> Self {
        self.window_size = (width, height);
        self
    }

    /// Calculate the tooltip position, adjusting for screen boundaries
    fn calculate_position(&self, size: (f32, f32)) -> (f32, f32) {
        let (mouse_x, mouse_y) = self.position;
        let (offset_x, offset_y) = self.config.cursor_offset;
        let (tooltip_w, tooltip_h) = size;
        let (window_w, window_h) = self.window_size;

        // Start with offset from cursor
        let mut x = mouse_x + offset_x;
        let mut y = mouse_y + offset_y;

        // Adjust if tooltip would go off right edge
        if x + tooltip_w > window_w - SPACING_SM {
            x = mouse_x - tooltip_w - offset_x;
        }

        // Adjust if tooltip would go off bottom edge
        if y + tooltip_h > window_h - SPACING_SM {
            y = mouse_y - tooltip_h - offset_y;
        }

        // Ensure we don't go off left/top edges
        x = x.max(SPACING_SM);
        y = y.max(SPACING_SM);

        (x, y)
    }
}

impl<M: 'static> Widget<M> for TooltipOverlay {
    fn layout(&mut self, _available: Size) -> Size {
        // Return zero size - tooltip renders as an absolute-positioned overlay
        // and should not affect the layout of other elements.
        // Actual size is calculated in draw() using the renderer.
        Size::ZERO
    }

    fn draw(&self, renderer: &mut Renderer, _bounds: Bounds) {
        // Measure content to get actual size
        // Note: We need to cast away const here since measure needs &mut renderer
        // This is safe because measure doesn't actually mutate visible state
        let renderer_ptr = renderer as *const Renderer as *mut Renderer;
        let size = unsafe { (*renderer_ptr).measure_content_helper(&self.content, &self.config) };

        let (x, y) = self.calculate_position_internal(size);

        let tooltip_bounds = Bounds::new(x, y, size.0, size.1);

        // Register as overlay (highest z-order)
        renderer.register_overlay_with_z_order(tooltip_bounds, u32::MAX);

        // Begin overlay rendering
        renderer.begin_overlay();

        // Draw background
        renderer.fill_rect(tooltip_bounds, self.config.background);

        // Draw border
        renderer.stroke_rect(
            tooltip_bounds,
            self.config.border_color,
            self.config.border_width,
        );

        // Draw content
        let padding = self.config.padding;
        let content_x = x + padding;
        let content_y = y + padding;
        let max_content_width = self.config.max_width - padding * 2.0;

        match &self.content {
            TooltipContent::Text(text) => {
                renderer.text_wrapped(
                    text,
                    content_x,
                    content_y,
                    self.config.body_size,
                    self.config.body_color,
                    max_content_width,
                    crate::renderer::TextAlign::Left,
                );
            }
            TooltipContent::Rich { title, body } => {
                // Draw title
                renderer.text_wrapped(
                    title,
                    content_x,
                    content_y,
                    self.config.title_size,
                    self.config.title_color,
                    max_content_width,
                    crate::renderer::TextAlign::Left,
                );

                // Calculate title height for body positioning
                let (_, title_h) = unsafe {
                    (*renderer_ptr).measure_text_wrapped(
                        title,
                        self.config.title_size,
                        max_content_width,
                    )
                };

                // Draw body
                renderer.text_wrapped(
                    body,
                    content_x,
                    content_y + title_h + SPACING_XS,
                    self.config.body_size,
                    self.config.body_color,
                    max_content_width,
                    crate::renderer::TextAlign::Left,
                );
            }
            TooltipContent::Custom(_hint) => {
                renderer.text_wrapped(
                    "[Custom content]",
                    content_x,
                    content_y,
                    self.config.body_size,
                    self.config.body_color,
                    max_content_width,
                    crate::renderer::TextAlign::Left,
                );
            }
        }

        // End overlay rendering
        renderer.end_overlay();
    }

    fn on_event(&mut self, _event: &Event, _bounds: Bounds) -> EventResult<M> {
        // Tooltips don't capture events - they're display only
        EventResult::None
    }

    fn has_active_overlay(&self) -> bool {
        // Tooltips are display-only overlays that don't capture events.
        // Return false so clicks pass through to underlying widgets.
        // This prevents the click-consumption logic from blocking clicks
        // when a tooltip happens to be visible.
        false
    }
}

impl TooltipOverlay {
    /// Internal helper for calculating position (avoids borrow issues)
    fn calculate_position_internal(&self, size: (f32, f32)) -> (f32, f32) {
        self.calculate_position(size)
    }
}

// Helper trait for renderer to measure tooltip content
impl Renderer {
    /// Measure tooltip content size (helper for TooltipOverlay)
    pub fn measure_content_helper(
        &mut self,
        content: &TooltipContent,
        config: &TooltipConfig,
    ) -> (f32, f32) {
        let padding = config.padding;
        let max_content_width = config.max_width - padding * 2.0;

        match content {
            TooltipContent::Text(text) => {
                let (text_w, text_h) =
                    self.measure_text_wrapped(text, config.body_size, max_content_width);
                (text_w + padding * 2.0, text_h + padding * 2.0)
            }
            TooltipContent::Rich { title, body } => {
                let (title_w, title_h) =
                    self.measure_text_wrapped(title, config.title_size, max_content_width);
                let (body_w, body_h) =
                    self.measure_text_wrapped(body, config.body_size, max_content_width);

                let content_w = title_w.max(body_w);
                let content_h = title_h + SPACING_XS + body_h;

                (content_w + padding * 2.0, content_h + padding * 2.0)
            }
            TooltipContent::Custom(_hint) => {
                let text = "[Custom content]";
                let (text_w, text_h) =
                    self.measure_text_wrapped(text, config.body_size, max_content_width);
                (text_w + padding * 2.0, text_h + padding * 2.0)
            }
        }
    }
}

/// Create a tooltip overlay element
pub fn tooltip_overlay<M: 'static>(content: TooltipContent, position: (f32, f32)) -> Element<M> {
    Element::new(TooltipOverlay::new(content, position))
}

/// Create a tooltip overlay with window size for proper boundary detection
pub fn tooltip_overlay_with_size<M: 'static>(
    content: TooltipContent,
    position: (f32, f32),
    window_size: (f32, f32),
) -> Element<M> {
    Element::new(TooltipOverlay::new(content, position).window_size(window_size.0, window_size.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tooltip_content_text() {
        let content = TooltipContent::text("Hello");
        match content {
            TooltipContent::Text(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_tooltip_content_rich() {
        let content = TooltipContent::rich("Title", "Body text");
        match content {
            TooltipContent::Rich { title, body } => {
                assert_eq!(title, "Title");
                assert_eq!(body, "Body text");
            }
            _ => panic!("Expected Rich variant"),
        }
    }

    #[test]
    fn test_tooltip_config_default() {
        let config = TooltipConfig::default();
        assert!(config.max_width > 0.0);
        assert!(config.padding > 0.0);
    }

    #[test]
    fn test_tooltip_position_basic() {
        let tooltip = TooltipOverlay::new(TooltipContent::text("Test"), (100.0, 100.0))
            .window_size(800.0, 600.0);

        let size = (100.0, 50.0);
        let (x, y) = tooltip.calculate_position(size);

        // Should be offset from cursor
        assert!(x > 100.0);
        assert!(y > 100.0);
    }

    #[test]
    fn test_tooltip_position_right_edge() {
        let tooltip = TooltipOverlay::new(TooltipContent::text("Test"), (750.0, 100.0))
            .window_size(800.0, 600.0);

        let size = (100.0, 50.0);
        let (x, _y) = tooltip.calculate_position(size);

        // Should flip to left of cursor
        assert!(x < 750.0);
    }

    #[test]
    fn test_tooltip_position_bottom_edge() {
        let tooltip = TooltipOverlay::new(TooltipContent::text("Test"), (100.0, 550.0))
            .window_size(800.0, 600.0);

        let size = (100.0, 50.0);
        let (_x, y) = tooltip.calculate_position(size);

        // Should flip to above cursor
        assert!(y < 550.0);
    }
}
