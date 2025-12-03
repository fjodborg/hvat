//! Zoom-to-cursor mathematics.
//!
//! This module contains the mathematical functions for zoom operations,
//! extracted for testability and reusability.

/// Represents pan/zoom transform state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
}

impl Transform {
    /// Create a new transform with the given zoom and pan.
    pub fn new(zoom: f32, pan_x: f32, pan_y: f32) -> Self {
        Self { zoom, pan_x, pan_y }
    }

    /// Create an identity transform (zoom=1, no pan).
    pub fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0)
    }

    /// Calculate zoom-to-cursor transformation.
    ///
    /// This keeps the point under the cursor fixed while zooming.
    /// The algorithm:
    /// 1. Find the image-space point under the cursor
    /// 2. After zooming, adjust pan so that same point stays under cursor
    ///
    /// # Arguments
    /// * `new_zoom` - The new zoom level
    /// * `cursor_x`, `cursor_y` - Cursor position in screen space
    /// * `widget_center_x`, `widget_center_y` - Center of the widget in screen space
    ///
    /// # Returns
    /// A new Transform with adjusted zoom and pan values.
    pub fn zoom_to_cursor(
        &self,
        new_zoom: f32,
        cursor_x: f32,
        cursor_y: f32,
        widget_center_x: f32,
        widget_center_y: f32,
    ) -> Transform {
        // Cursor position relative to widget center
        let cursor_rel_x = cursor_x - widget_center_x;
        let cursor_rel_y = cursor_y - widget_center_y;

        // Image-space point under cursor (before zoom)
        let img_x = (cursor_rel_x - self.pan_x) / self.zoom;
        let img_y = (cursor_rel_y - self.pan_y) / self.zoom;

        // Calculate new pan to keep the image point under cursor
        let new_pan_x = cursor_rel_x - img_x * new_zoom;
        let new_pan_y = cursor_rel_y - img_y * new_zoom;

        Transform {
            zoom: new_zoom,
            pan_x: new_pan_x,
            pan_y: new_pan_y,
        }
    }

    /// Apply a pan delta to the transform.
    pub fn pan_by(&self, dx: f32, dy: f32) -> Transform {
        Transform {
            zoom: self.zoom,
            pan_x: self.pan_x + dx,
            pan_y: self.pan_y + dy,
        }
    }

    /// Zoom in by a factor (e.g., 1.2 for 20% zoom in).
    pub fn zoom_in(&self, factor: f32, max_zoom: f32) -> Transform {
        Transform {
            zoom: (self.zoom * factor).min(max_zoom),
            pan_x: self.pan_x,
            pan_y: self.pan_y,
        }
    }

    /// Zoom out by a factor (e.g., 1.2 for 20% zoom out).
    pub fn zoom_out(&self, factor: f32, min_zoom: f32) -> Transform {
        Transform {
            zoom: (self.zoom / factor).max(min_zoom),
            pan_x: self.pan_x,
            pan_y: self.pan_y,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.0001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_identity_transform() {
        let t = Transform::identity();
        assert_eq!(t.zoom, 1.0);
        assert_eq!(t.pan_x, 0.0);
        assert_eq!(t.pan_y, 0.0);
    }

    #[test]
    fn test_zoom_to_cursor_at_center() {
        // Zooming at the center should not change pan
        let t = Transform::identity();
        let new_t = t.zoom_to_cursor(2.0, 100.0, 100.0, 100.0, 100.0);

        assert_eq!(new_t.zoom, 2.0);
        assert!(approx_eq(new_t.pan_x, 0.0));
        assert!(approx_eq(new_t.pan_y, 0.0));
    }

    #[test]
    fn test_zoom_to_cursor_preserves_cursor_point() {
        // After zooming, the same image point should be under the cursor
        let t = Transform::new(1.0, 50.0, 30.0);
        let cursor_x = 150.0;
        let cursor_y = 120.0;
        let widget_cx = 100.0;
        let widget_cy = 100.0;

        // Calculate image point under cursor before zoom
        let cursor_rel_x = cursor_x - widget_cx;
        let cursor_rel_y = cursor_y - widget_cy;
        let img_x_before = (cursor_rel_x - t.pan_x) / t.zoom;
        let img_y_before = (cursor_rel_y - t.pan_y) / t.zoom;

        // Zoom to 2x
        let new_t = t.zoom_to_cursor(2.0, cursor_x, cursor_y, widget_cx, widget_cy);

        // Calculate image point under cursor after zoom
        let img_x_after = (cursor_rel_x - new_t.pan_x) / new_t.zoom;
        let img_y_after = (cursor_rel_y - new_t.pan_y) / new_t.zoom;

        // The image point under cursor should be the same
        assert!(approx_eq(img_x_before, img_x_after));
        assert!(approx_eq(img_y_before, img_y_after));
    }

    #[test]
    fn test_zoom_to_cursor_zoom_out() {
        // Test zooming out
        let t = Transform::new(2.0, 100.0, 100.0);
        let new_t = t.zoom_to_cursor(1.0, 150.0, 150.0, 100.0, 100.0);

        assert_eq!(new_t.zoom, 1.0);
        // Pan should adjust to keep the point under cursor
    }

    #[test]
    fn test_pan_by() {
        let t = Transform::new(1.0, 10.0, 20.0);
        let new_t = t.pan_by(5.0, -10.0);

        assert_eq!(new_t.zoom, 1.0);
        assert_eq!(new_t.pan_x, 15.0);
        assert_eq!(new_t.pan_y, 10.0);
    }

    #[test]
    fn test_zoom_in_with_max() {
        let t = Transform::new(4.0, 0.0, 0.0);
        let new_t = t.zoom_in(1.5, 5.0);

        // 4.0 * 1.5 = 6.0, but max is 5.0
        assert_eq!(new_t.zoom, 5.0);
    }

    #[test]
    fn test_zoom_out_with_min() {
        let t = Transform::new(0.3, 0.0, 0.0);
        let new_t = t.zoom_out(1.5, 0.2);

        // 0.3 / 1.5 = 0.2, exactly at min
        assert!(approx_eq(new_t.zoom, 0.2));
    }

    #[test]
    fn test_zoom_in_normal() {
        let t = Transform::new(1.0, 0.0, 0.0);
        let new_t = t.zoom_in(1.2, 5.0);

        assert!(approx_eq(new_t.zoom, 1.2));
    }

    #[test]
    fn test_zoom_out_normal() {
        let t = Transform::new(1.0, 0.0, 0.0);
        let new_t = t.zoom_out(1.2, 0.2);

        assert!(approx_eq(new_t.zoom, 1.0 / 1.2));
    }

    #[test]
    fn test_multiple_zoom_operations() {
        // Zoom in then out should approximately return to original
        let t = Transform::identity();
        let zoomed_in = t.zoom_in(1.5, 10.0);
        let zoomed_out = zoomed_in.zoom_out(1.5, 0.1);

        assert!(approx_eq(zoomed_out.zoom, 1.0));
    }

    #[test]
    fn test_pan_preserves_zoom() {
        let t = Transform::new(2.5, 0.0, 0.0);
        let panned = t.pan_by(100.0, 200.0);

        assert_eq!(panned.zoom, 2.5);
    }
}
