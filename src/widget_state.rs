//! Widget state management layer.
//!
//! This module provides an intermediate layer for managing transient widget state
//! that would otherwise be lost when widgets are rebuilt every frame.
//!
//! The goal is to keep the top-level application struct simple while properly
//! handling drag states, hover states, and other ephemeral UI state.

use hvat_ui::widgets::SliderId;

/// Transient state for the image viewer/pan-zoom widget.
#[derive(Debug, Clone, Default)]
pub struct ImageViewState {
    /// Whether the image is currently being dragged
    pub is_dragging: bool,
    /// Last drag position for calculating delta
    pub last_drag_pos: Option<(f32, f32)>,
}

impl ImageViewState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a drag operation.
    pub fn start_drag(&mut self, pos: (f32, f32)) {
        self.is_dragging = true;
        self.last_drag_pos = Some(pos);
    }

    /// Update drag position and return the delta.
    pub fn update_drag(&mut self, pos: (f32, f32)) -> Option<(f32, f32)> {
        if self.is_dragging {
            if let Some(last_pos) = self.last_drag_pos {
                let delta = (pos.0 - last_pos.0, pos.1 - last_pos.1);
                self.last_drag_pos = Some(pos);
                return Some(delta);
            }
        }
        None
    }

    /// End the drag operation.
    pub fn end_drag(&mut self) {
        self.is_dragging = false;
        self.last_drag_pos = None;
    }
}

/// Transient state for sliders.
#[derive(Debug, Clone, Default)]
pub struct SliderState {
    /// Which slider is currently being dragged (if any)
    pub active_slider: Option<SliderId>,
}

impl SliderState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start dragging a slider.
    pub fn start_drag(&mut self, id: SliderId) {
        self.active_slider = Some(id);
    }

    /// End slider drag.
    pub fn end_drag(&mut self) {
        self.active_slider = None;
    }

    /// Check if a specific slider is being dragged.
    pub fn is_dragging(&self, id: SliderId) -> bool {
        self.active_slider == Some(id)
    }
}

/// Transient state for scrollable containers.
#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    /// Current scroll offset
    pub offset: f32,
    /// Whether the scrollbar is being dragged
    pub is_dragging: bool,
}

impl ScrollState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the scroll offset.
    pub fn set_offset(&mut self, offset: f32) {
        self.offset = offset;
    }

    /// Start scrollbar drag.
    pub fn start_drag(&mut self) {
        self.is_dragging = true;
    }

    /// End scrollbar drag.
    pub fn end_drag(&mut self) {
        self.is_dragging = false;
    }
}

/// Combined widget state manager.
///
/// This struct aggregates all transient UI state in one place,
/// making it easy to pass to the view function and keeping the
/// main application struct focused on domain state.
#[derive(Debug, Clone, Default)]
pub struct WidgetState {
    /// Image viewer state (pan/zoom dragging)
    pub image: ImageViewState,
    /// Slider state (which slider is active)
    pub slider: SliderState,
    /// Main content scroll state
    pub scroll: ScrollState,
}

impl WidgetState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all transient state (useful when switching views/tabs).
    pub fn reset(&mut self) {
        self.image = ImageViewState::default();
        self.slider = SliderState::default();
        // Note: scroll state is intentionally preserved across resets
    }

    /// Reset scroll state (useful when loading new content).
    pub fn reset_scroll(&mut self) {
        self.scroll = ScrollState::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_view_state_drag() {
        let mut state = ImageViewState::new();
        assert!(!state.is_dragging);

        state.start_drag((100.0, 100.0));
        assert!(state.is_dragging);
        assert_eq!(state.last_drag_pos, Some((100.0, 100.0)));

        let delta = state.update_drag((110.0, 105.0));
        assert_eq!(delta, Some((10.0, 5.0)));
        assert_eq!(state.last_drag_pos, Some((110.0, 105.0)));

        state.end_drag();
        assert!(!state.is_dragging);
        assert_eq!(state.last_drag_pos, None);
    }

    #[test]
    fn test_slider_state() {
        let mut state = SliderState::new();
        assert!(state.active_slider.is_none());

        state.start_drag(SliderId::Brightness);
        assert!(state.is_dragging(SliderId::Brightness));
        assert!(!state.is_dragging(SliderId::Contrast));

        state.end_drag();
        assert!(!state.is_dragging(SliderId::Brightness));
    }

    #[test]
    fn test_scroll_state() {
        let mut state = ScrollState::new();
        assert_eq!(state.offset, 0.0);
        assert!(!state.is_dragging);

        state.set_offset(50.0);
        assert_eq!(state.offset, 50.0);

        state.start_drag();
        assert!(state.is_dragging);

        state.end_drag();
        assert!(!state.is_dragging);
    }
}
