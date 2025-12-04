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
    /// Current vertical scroll offset (positive = scrolled down)
    pub offset_y: f32,
    /// Current horizontal scroll offset (positive = scrolled right)
    pub offset_x: f32,
    /// Whether the vertical scrollbar is being dragged
    pub is_dragging_y: bool,
    /// Whether the horizontal scrollbar is being dragged
    pub is_dragging_x: bool,
    /// Mouse Y position when vertical drag started (for relative dragging)
    pub drag_start_mouse_y: Option<f32>,
    /// Scroll offset when vertical drag started
    pub drag_start_scroll_y: Option<f32>,
    /// Mouse X position when horizontal drag started (for relative dragging)
    pub drag_start_mouse_x: Option<f32>,
    /// Scroll offset when horizontal drag started
    pub drag_start_scroll_x: Option<f32>,
}

impl ScrollState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the vertical scroll offset.
    pub fn set_offset_y(&mut self, offset: f32) {
        self.offset_y = offset;
    }

    /// Set the horizontal scroll offset.
    pub fn set_offset_x(&mut self, offset: f32) {
        self.offset_x = offset;
    }

    /// Start vertical scrollbar drag with mouse position for relative dragging.
    pub fn start_drag_y(&mut self, mouse_y: f32) {
        self.is_dragging_y = true;
        self.drag_start_mouse_y = Some(mouse_y);
        self.drag_start_scroll_y = Some(self.offset_y);
    }

    /// Start horizontal scrollbar drag with mouse position for relative dragging.
    pub fn start_drag_x(&mut self, mouse_x: f32) {
        self.is_dragging_x = true;
        self.drag_start_mouse_x = Some(mouse_x);
        self.drag_start_scroll_x = Some(self.offset_x);
    }

    /// End vertical scrollbar drag.
    pub fn end_drag_y(&mut self) {
        self.is_dragging_y = false;
        self.drag_start_mouse_y = None;
        self.drag_start_scroll_y = None;
    }

    /// End horizontal scrollbar drag.
    pub fn end_drag_x(&mut self) {
        self.is_dragging_x = false;
        self.drag_start_mouse_x = None;
        self.drag_start_scroll_x = None;
    }
}

/// Transient state for dropdown widgets.
#[derive(Debug, Clone, Default)]
pub struct DropdownState {
    /// Whether band persistence dropdown is open
    pub band_persistence_open: bool,
    /// Whether image settings persistence dropdown is open
    pub image_settings_persistence_open: bool,
    /// Whether export format dropdown is open
    pub export_format_open: bool,
}

impl DropdownState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open band persistence dropdown and close others.
    pub fn open_band_persistence(&mut self) {
        self.band_persistence_open = true;
        self.image_settings_persistence_open = false;
        self.export_format_open = false;
    }

    /// Open image settings persistence dropdown and close others.
    pub fn open_image_settings_persistence(&mut self) {
        self.image_settings_persistence_open = true;
        self.band_persistence_open = false;
        self.export_format_open = false;
    }

    /// Open export format dropdown and close others.
    pub fn open_export_format(&mut self) {
        self.export_format_open = true;
        self.band_persistence_open = false;
        self.image_settings_persistence_open = false;
    }

    /// Close band persistence dropdown.
    pub fn close_band_persistence(&mut self) {
        self.band_persistence_open = false;
    }

    /// Close image settings persistence dropdown.
    pub fn close_image_settings_persistence(&mut self) {
        self.image_settings_persistence_open = false;
    }

    /// Close export format dropdown.
    pub fn close_export_format(&mut self) {
        self.export_format_open = false;
    }

    /// Toggle export format dropdown.
    pub fn toggle_export_format(&mut self) {
        if self.export_format_open {
            self.close_export_format();
        } else {
            self.open_export_format();
        }
    }

    /// Close all dropdowns.
    pub fn close_all(&mut self) {
        self.band_persistence_open = false;
        self.image_settings_persistence_open = false;
        self.export_format_open = false;
    }
}

/// Transient state for collapsible containers.
#[derive(Debug, Clone)]
pub struct CollapsibleState {
    /// Whether image settings section is collapsed
    pub image_settings_collapsed: bool,
    /// Whether band settings section is collapsed
    pub band_settings_collapsed: bool,
}

impl Default for CollapsibleState {
    fn default() -> Self {
        Self {
            image_settings_collapsed: true,  // Closed by default
            band_settings_collapsed: false,  // Open by default
        }
    }
}

impl CollapsibleState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle_image_settings(&mut self) {
        self.image_settings_collapsed = !self.image_settings_collapsed;
    }

    pub fn toggle_band_settings(&mut self) {
        self.band_settings_collapsed = !self.band_settings_collapsed;
    }
}

/// State for category input field.
#[derive(Debug, Clone, Default)]
pub struct CategoryInputState {
    /// Current text in the new category input field
    pub new_category_name: String,
    /// Whether the input field is focused
    pub is_focused: bool,
}

impl CategoryInputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the input text.
    pub fn set_text(&mut self, text: String) {
        self.new_category_name = text;
    }

    /// Clear the input field.
    pub fn clear(&mut self) {
        self.new_category_name.clear();
        self.is_focused = false;
    }

    /// Set focus state.
    pub fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }
}

/// State for tag input field.
#[derive(Debug, Clone, Default)]
pub struct TagInputState {
    /// Current text in the new tag input field
    pub new_tag_name: String,
    /// Whether the input field is focused
    pub is_focused: bool,
}

impl TagInputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the input text.
    pub fn set_text(&mut self, text: String) {
        self.new_tag_name = text;
    }

    /// Clear the input field.
    pub fn clear(&mut self) {
        self.new_tag_name.clear();
        self.is_focused = false;
    }

    /// Set focus state.
    pub fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
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
    /// Sidebar scroll state (independent from main content)
    pub sidebar_scroll: ScrollState,
    /// Dropdown states
    pub dropdown: DropdownState,
    /// Collapsible container states
    pub collapsible: CollapsibleState,
    /// Category input state
    pub category_input: CategoryInputState,
    /// Tag input state
    pub tag_input: TagInputState,
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
        assert_eq!(state.offset_y, 0.0);
        assert_eq!(state.offset_x, 0.0);
        assert!(!state.is_dragging_y);
        assert!(!state.is_dragging_x);

        state.set_offset_y(50.0);
        assert_eq!(state.offset_y, 50.0);

        state.set_offset_x(30.0);
        assert_eq!(state.offset_x, 30.0);

        state.start_drag_y(100.0);
        assert!(state.is_dragging_y);

        state.start_drag_x(50.0);
        assert!(state.is_dragging_x);

        state.end_drag_y();
        assert!(!state.is_dragging_y);

        state.end_drag_x();
        assert!(!state.is_dragging_x);
    }
}
