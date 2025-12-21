//! Common overlay utilities for popup widgets (dropdowns, color pickers, tooltips, etc.)
//!
//! This module provides helper types and functions for implementing overlay widgets
//! that share common behavior like:
//! - Opening/closing state
//! - Closing when clicking outside
//! - Closing on Escape key or focus loss

use crate::event::{Event, KeyCode};
use crate::layout::Bounds;

/// Helper for checking if an overlay should close based on events.
///
/// This provides common closing logic that overlay widgets can use
/// to handle GlobalMousePress, Escape, and FocusLost events consistently.
pub struct OverlayCloseHelper;

impl OverlayCloseHelper {
    /// Check if a GlobalMousePress event should close the overlay.
    ///
    /// Returns `true` if the click position is outside the overlay bounds.
    ///
    /// # Arguments
    /// * `position` - The click position from GlobalMousePress event
    /// * `overlay_bounds` - The bounds of the overlay popup
    #[inline]
    pub fn should_close_on_global_press(position: (f32, f32), overlay_bounds: Bounds) -> bool {
        !overlay_bounds.contains(position.0, position.1)
    }

    /// Check if an event should close the overlay (Escape key or FocusLost).
    ///
    /// Returns `true` for Escape key press or FocusLost events.
    #[inline]
    pub fn should_close_on_event(event: &Event) -> bool {
        matches!(
            event,
            Event::KeyPress { key: KeyCode::Escape, .. } | Event::FocusLost
        )
    }

    /// Combined check for whether any event should close an open overlay.
    ///
    /// This handles:
    /// - GlobalMousePress outside overlay bounds
    /// - Escape key press
    /// - FocusLost event
    ///
    /// # Arguments
    /// * `event` - The event to check
    /// * `overlay_bounds` - The bounds of the overlay popup (used for GlobalMousePress)
    ///
    /// # Returns
    /// `true` if the overlay should close
    pub fn should_close(event: &Event, overlay_bounds: Bounds) -> bool {
        match event {
            Event::GlobalMousePress { position, .. } => {
                Self::should_close_on_global_press(*position, overlay_bounds)
            }
            Event::KeyPress { key: KeyCode::Escape, .. } | Event::FocusLost => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_close_on_global_press_outside() {
        let bounds = Bounds::new(100.0, 100.0, 50.0, 50.0);
        // Click outside
        assert!(OverlayCloseHelper::should_close_on_global_press((50.0, 50.0), bounds));
        assert!(OverlayCloseHelper::should_close_on_global_press((200.0, 200.0), bounds));
    }

    #[test]
    fn test_should_close_on_global_press_inside() {
        let bounds = Bounds::new(100.0, 100.0, 50.0, 50.0);
        // Click inside
        assert!(!OverlayCloseHelper::should_close_on_global_press((125.0, 125.0), bounds));
    }

    #[test]
    fn test_should_close_on_escape() {
        let event = Event::KeyPress {
            key: KeyCode::Escape,
            modifiers: crate::event::KeyModifiers::default(),
        };
        assert!(OverlayCloseHelper::should_close_on_event(&event));
    }

    #[test]
    fn test_should_close_on_focus_lost() {
        let event = Event::FocusLost;
        assert!(OverlayCloseHelper::should_close_on_event(&event));
    }

    #[test]
    fn test_should_not_close_on_other_events() {
        let event = Event::KeyPress {
            key: KeyCode::Enter,
            modifiers: crate::event::KeyModifiers::default(),
        };
        assert!(!OverlayCloseHelper::should_close_on_event(&event));
    }
}
