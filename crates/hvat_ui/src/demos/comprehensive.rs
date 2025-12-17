//! Comprehensive demo showcasing the full hvat_ui framework
//!
//! This demo demonstrates:
//! - Central image viewer with pan/zoom
//! - Left sidebar with scrollable controls (collapsible sections)
//! - Right sidebar with sliders for image adjustments
//! - Proper layout with Row/Column containers

use crate::element::Element;
use crate::event::{Event, KeyCode};
use crate::layout::Length;
use crate::prelude::*;
use crate::state::{
    CollapsibleState, DropdownState, NumberInputState, SliderState, TextInputState, UndoContext,
    UndoStack,
};
use crate::widgets::{Column, Row, Scrollable, ScrollDirection, ScrollbarVisibility};
use crate::Context;
use std::cell::RefCell;
use std::rc::Rc;

/// Sidebar width constant
const SIDEBAR_WIDTH: f32 = 250.0;

/// Snapshot for undo/redo in comprehensive demo
#[derive(Debug, Clone)]
struct DemoSnapshot {
    notes_text: String,
    brightness: f32,
    contrast: f32,
    gamma: f32,
    saturation: f32,
}

/// Comprehensive demo state
pub struct ComprehensiveDemo {
    // Image viewer state
    pub viewer_state: ImageViewerState,
    pub texture_id: Option<TextureId>,
    pub texture_size: (u32, u32),

    // Left sidebar - collapsible sections
    pub view_settings_collapsed: CollapsibleState,
    pub tools_collapsed: CollapsibleState,
    pub info_collapsed: CollapsibleState,
    pub widgets_collapsed: CollapsibleState,
    pub left_scroll_state: ScrollState,

    // Right sidebar - sliders for adjustments
    pub brightness_slider: SliderState,
    pub contrast_slider: SliderState,
    pub gamma_slider: SliderState,
    pub saturation_slider: SliderState,
    pub zoom_slider: SliderState,
    pub right_scroll_state: ScrollState,

    // Tool selection
    pub selected_tool: Tool,

    // Number inputs
    pub x_offset_input: NumberInputState,
    pub y_offset_input: NumberInputState,

    // Dropdown demo
    pub blend_mode_dropdown: DropdownState,
    pub selected_blend_mode: Option<usize>,

    // Text input
    pub notes_input_text: String,
    pub notes_input_state: TextInputState,

    // Global undo stack for the demo (Rc<RefCell> for interior mutability)
    undo_stack: Rc<RefCell<UndoStack<DemoSnapshot>>>,

    // Window size for dropdown positioning
    pub window_height: f32,
}

/// Available tools
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pan,
    Zoom,
    Select,
    Measure,
}

impl Default for Tool {
    fn default() -> Self {
        Tool::Pan
    }
}

impl Tool {
    fn name(&self) -> &'static str {
        match self {
            Tool::Pan => "Pan",
            Tool::Zoom => "Zoom",
            Tool::Select => "Select",
            Tool::Measure => "Measure",
        }
    }

    fn all() -> &'static [Tool] {
        &[Tool::Pan, Tool::Zoom, Tool::Select, Tool::Measure]
    }
}

/// Blend mode options
const BLEND_MODES: &[&str] = &[
    "Normal",
    "Multiply",
    "Screen",
    "Overlay",
    "Soft Light",
    "Hard Light",
    "Color Dodge",
    "Color Burn",
    "Darken",
    "Lighten",
    "Difference",
    "Exclusion",
];

/// Comprehensive demo messages
#[derive(Clone)]
pub enum ComprehensiveMessage {
    // Image viewer
    ViewerChanged(ImageViewerState),
    ResetView,
    FitToWindow,
    ZoomToActual,

    // Left sidebar collapsibles
    ViewSettingsToggled(CollapsibleState),
    ToolsToggled(CollapsibleState),
    InfoToggled(CollapsibleState),
    WidgetsToggled(CollapsibleState),
    LeftScrolled(ScrollState),

    // Tool selection
    ToolSelected(Tool),

    // Right sidebar sliders
    BrightnessChanged(SliderState),
    ContrastChanged(SliderState),
    GammaChanged(SliderState),
    SaturationChanged(SliderState),
    ZoomSliderChanged(SliderState),
    RightScrolled(ScrollState),
    ResetAdjustments,

    // Number inputs
    XOffsetChanged(f32, NumberInputState),
    YOffsetChanged(f32, NumberInputState),

    // Dropdown
    BlendModeSelected(usize),
    BlendModeDropdownChanged(DropdownState),

    // Text input
    NotesChanged(String, TextInputState),
    NotesSubmitted(String),

    // Global undo/redo
    Undo,
    Redo,
}

impl Default for ComprehensiveDemo {
    fn default() -> Self {
        Self::new()
    }
}

impl ComprehensiveDemo {
    pub fn new() -> Self {
        Self {
            viewer_state: ImageViewerState::new(),
            texture_id: None,
            texture_size: (0, 0),

            view_settings_collapsed: CollapsibleState::expanded(),
            tools_collapsed: CollapsibleState::expanded(),
            info_collapsed: CollapsibleState::collapsed(),
            widgets_collapsed: CollapsibleState::expanded(),
            left_scroll_state: ScrollState::new(),

            brightness_slider: SliderState::new(0.0),
            contrast_slider: SliderState::new(1.0),
            gamma_slider: SliderState::new(1.0),
            saturation_slider: SliderState::new(1.0),
            zoom_slider: SliderState::new(100.0),
            right_scroll_state: ScrollState::new(),

            selected_tool: Tool::default(),

            x_offset_input: NumberInputState::new(0.0),
            y_offset_input: NumberInputState::new(0.0),

            blend_mode_dropdown: DropdownState::new(),
            selected_blend_mode: Some(0), // Normal

            notes_input_text: String::new(),
            notes_input_state: TextInputState::new(),

            undo_stack: Rc::new(RefCell::new(UndoStack::new(50))),

            window_height: 900.0, // Default, updated by resize
        }
    }

    /// Create a snapshot of current state for undo
    fn snapshot(&self) -> DemoSnapshot {
        DemoSnapshot {
            notes_text: self.notes_input_text.clone(),
            brightness: self.brightness_slider.value,
            contrast: self.contrast_slider.value,
            gamma: self.gamma_slider.value,
            saturation: self.saturation_slider.value,
        }
    }

    /// Restore state from a snapshot
    fn restore(&mut self, snapshot: &DemoSnapshot) {
        self.notes_input_text = snapshot.notes_text.clone();
        self.notes_input_state.cursor = self.notes_input_text.len();
        self.notes_input_state.selection = None;
        self.brightness_slider.set_value(snapshot.brightness);
        self.contrast_slider.set_value(snapshot.contrast);
        self.gamma_slider.set_value(snapshot.gamma);
        self.saturation_slider.set_value(snapshot.saturation);
    }

    /// Handle keyboard events for undo/redo shortcuts
    /// Returns Some(message) if a shortcut was triggered
    pub fn handle_key_event(event: &Event) -> Option<ComprehensiveMessage> {
        if let Event::KeyPress { key, modifiers, .. } = event {
            if modifiers.ctrl {
                match key {
                    KeyCode::Z if modifiers.shift => {
                        return Some(ComprehensiveMessage::Redo);
                    }
                    KeyCode::Z => {
                        return Some(ComprehensiveMessage::Undo);
                    }
                    KeyCode::Y => {
                        return Some(ComprehensiveMessage::Redo);
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Update window dimensions (call this on resize)
    pub fn set_window_size(&mut self, _width: f32, height: f32) {
        self.window_height = height;
    }

    /// Setup the demo with a test texture. Call this in Application::setup()
    pub fn setup(&mut self, resources: &mut Resources) {
        let width = 2048;
        let height = 2048;
        let pattern = create_gradient_pattern(width, height);

        let gpu_ctx = resources.gpu_context();
        match crate::Texture::from_rgba8(gpu_ctx, &pattern, width, height) {
            Ok(texture) => {
                let id = resources.register_texture(&texture);
                self.texture_id = Some(id);
                self.texture_size = (width, height);
                log::info!("ComprehensiveDemo: Created test texture {}x{}", width, height);
            }
            Err(e) => {
                log::error!("ComprehensiveDemo: Failed to create texture: {:?}", e);
            }
        }
    }

    pub fn view<M: Clone + 'static>(
        &self,
        wrap: impl Fn(ComprehensiveMessage) -> M + Clone + 'static,
    ) -> Element<M> {
        // Clone what we need for closures
        let viewer_state = self.viewer_state.clone();
        let view_settings_state = self.view_settings_collapsed.clone();
        let tools_state = self.tools_collapsed.clone();
        let info_state = self.info_collapsed.clone();
        let widgets_state = self.widgets_collapsed.clone();
        let left_scroll_state = self.left_scroll_state.clone();

        let brightness_state = self.brightness_slider.clone();
        let contrast_state = self.contrast_slider.clone();
        let gamma_state = self.gamma_slider.clone();
        let saturation_state = self.saturation_slider.clone();
        let zoom_state = self.zoom_slider.clone();
        let right_scroll_state = self.right_scroll_state.clone();

        let selected_tool = self.selected_tool;
        let texture_id = self.texture_id;
        let texture_size = self.texture_size;

        let x_offset_state = self.x_offset_input.clone();
        let y_offset_state = self.y_offset_input.clone();

        let blend_mode_state = self.blend_mode_dropdown.clone();
        let selected_blend_mode = self.selected_blend_mode;

        let notes_text = self.notes_input_text.clone();
        let notes_state = self.notes_input_state.clone();

        // Create wrapper closures for each section
        let wrap_viewer = wrap.clone();
        let wrap_left = wrap.clone();
        let wrap_right = wrap.clone();

        // Create snapshot for undo closures
        let current_snapshot = self.snapshot();

        // Build the left sidebar
        let left_sidebar = self.build_left_sidebar(
            wrap_left,
            view_settings_state,
            tools_state,
            info_state,
            widgets_state,
            left_scroll_state,
            selected_tool,
            viewer_state.clone(),
            x_offset_state,
            y_offset_state,
            blend_mode_state,
            selected_blend_mode,
            notes_text,
            notes_state,
            self.window_height,
            current_snapshot.clone(),
            Rc::clone(&self.undo_stack),
        );

        // Build the center image viewer
        let center_viewer = self.build_image_viewer(wrap_viewer, viewer_state, texture_id, texture_size);

        // Build the right sidebar
        let right_sidebar = self.build_right_sidebar(
            wrap_right,
            brightness_state,
            contrast_state,
            gamma_state,
            saturation_state,
            zoom_state,
            right_scroll_state,
            current_snapshot,
            Rc::clone(&self.undo_stack),
        );

        log::debug!("ComprehensiveDemo: Building main layout with 3 children");

        // Build the main layout: Row with [LeftSidebar | ImageViewer | RightSidebar]
        // Set the Row to fill the available space
        let mut row = Row::new(vec![left_sidebar, center_viewer, right_sidebar]);
        row = row.width(Length::Fill(1.0)).height(Length::Fill(1.0));
        Element::new(row)
    }

    fn build_left_sidebar<M: Clone + 'static>(
        &self,
        wrap: impl Fn(ComprehensiveMessage) -> M + Clone + 'static,
        view_settings_state: CollapsibleState,
        tools_state: CollapsibleState,
        info_state: CollapsibleState,
        widgets_state: CollapsibleState,
        scroll_state: ScrollState,
        selected_tool: Tool,
        viewer_state: ImageViewerState,
        x_offset_state: NumberInputState,
        y_offset_state: NumberInputState,
        blend_mode_state: DropdownState,
        selected_blend_mode: Option<usize>,
        notes_text: String,
        notes_state: TextInputState,
        window_height: f32,
        notes_undo_snapshot: DemoSnapshot,
        undo_stack: Rc<RefCell<UndoStack<DemoSnapshot>>>,
    ) -> Element<M> {
        let wrap_vs = wrap.clone();
        let wrap_tools = wrap.clone();
        let wrap_info = wrap.clone();
        let wrap_widgets = wrap.clone();
        let wrap_scroll = wrap.clone();
        let wrap_tool_select = wrap.clone();
        let wrap_reset = wrap.clone();
        let wrap_fit = wrap.clone();
        let wrap_actual = wrap.clone();
        let wrap_x = wrap.clone();
        let wrap_y = wrap.clone();
        let wrap_dropdown_select = wrap.clone();
        let wrap_dropdown_change = wrap.clone();
        let wrap_notes = wrap.clone();
        let wrap_notes_submit = wrap.clone();

        // Create UndoContext for clean on_undo_point callbacks
        let undo_ctx = UndoContext::new(undo_stack, notes_undo_snapshot);

        // Build sidebar content
        let mut sidebar_ctx = Context::new();

        sidebar_ctx.text_sized("Controls", 16.0);
        sidebar_ctx.text("");

        // View Settings Collapsible
        let vs_state = view_settings_state.clone();
        let collapsible_vs = crate::Collapsible::new("View Settings")
            .state(&vs_state)
            .width(Length::Fill(1.0))
            .on_toggle(move |s| wrap_vs(ComprehensiveMessage::ViewSettingsToggled(s)))
            .content(|c| {
                let reset_msg = wrap_reset(ComprehensiveMessage::ResetView);
                let fit_msg = wrap_fit(ComprehensiveMessage::FitToWindow);
                let actual_msg = wrap_actual(ComprehensiveMessage::ZoomToActual);

                c.text(format!("Zoom: {:.0}%", viewer_state.zoom * 100.0));
                c.text(format!(
                    "Pan: ({:.1}, {:.1})",
                    viewer_state.pan.0, viewer_state.pan.1
                ));
                c.text("");

                // All view control buttons in one row
                c.row(|r| {
                    r.button("Reset").width(Length::Fixed(60.0)).on_click(reset_msg);
                    r.button("Fit").width(Length::Fixed(50.0)).on_click(fit_msg);
                    r.button("1:1").width(Length::Fixed(50.0)).on_click(actual_msg);
                });

                c.text("");
                c.text_sized("Pan Offset:", 12.0);

                c.row(|r| {
                    r.text_sized("X:", 12.0);
                    r.number_input()
                        .state(&x_offset_state)
                        .range(-2.0, 2.0)
                        .step(0.01)
                        .width(Length::Fixed(80.0))
                        .on_change(move |v, s| wrap_x(ComprehensiveMessage::XOffsetChanged(v, s)))
                        .build();
                });

                c.row(|r| {
                    r.text_sized("Y:", 12.0);
                    r.number_input()
                        .state(&y_offset_state)
                        .range(-2.0, 2.0)
                        .step(0.01)
                        .width(Length::Fixed(80.0))
                        .on_change(move |v, s| wrap_y(ComprehensiveMessage::YOffsetChanged(v, s)))
                        .build();
                });
            });
        sidebar_ctx.add(Element::new(collapsible_vs));

        // Tools Collapsible
        let tools_s = tools_state.clone();
        let collapsible_tools = crate::Collapsible::new("Tools (demo selection)")
            .state(&tools_s)
            .width(Length::Fill(1.0))
            .max_height(250.0)
            .on_toggle(move |s| wrap_tools(ComprehensiveMessage::ToolsToggled(s)))
            .content(|c| {
                c.text_sized(format!("Current: {}", selected_tool.name()), 11.0);
                c.text("");
                for tool in Tool::all() {
                    let is_selected = *tool == selected_tool;
                    let tool_copy = *tool;
                    let wrap_t = wrap_tool_select.clone();
                    let label = if is_selected {
                        format!("> {} <", tool.name())
                    } else {
                        tool.name().to_string()
                    };
                    c.button(label)
                        .width(Length::Fill(1.0))
                        .on_click(wrap_t(ComprehensiveMessage::ToolSelected(tool_copy)));
                }
            });
        sidebar_ctx.add(Element::new(collapsible_tools));

        // Widgets Demo Collapsible (dropdown and text input)
        let widgets_s = widgets_state.clone();
        let collapsible_widgets = crate::Collapsible::new("More Widgets")
            .state(&widgets_s)
            .width(Length::Fill(1.0))
            .on_toggle(move |s| wrap_widgets(ComprehensiveMessage::WidgetsToggled(s)))
            .content(|c| {
                // Dropdown demo
                c.text_sized("Dropdown (searchable):", 12.0);
                c.text_sized(
                    format!(
                        "Selected: {}",
                        selected_blend_mode
                            .map(|i| BLEND_MODES[i])
                            .unwrap_or("None")
                    ),
                    10.0,
                );
                c.add(
                    Element::new(
                        crate::Dropdown::new()
                            .state(&blend_mode_state)
                            .options(BLEND_MODES.iter().map(|s| s.to_string()))
                            .selected(selected_blend_mode)
                            .placeholder("Select blend mode...")
                            .searchable(true)
                            .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
                            .viewport_height(window_height)
                            .on_select(move |idx| {
                                wrap_dropdown_select(ComprehensiveMessage::BlendModeSelected(idx))
                            })
                            .on_change(move |state| {
                                wrap_dropdown_change(ComprehensiveMessage::BlendModeDropdownChanged(
                                    state,
                                ))
                            }),
                    )
                );

                c.text("");
                c.text_sized("Text Input:", 12.0);
                c.text_input()
                    .value(&notes_text)
                    .state(&notes_state)
                    .placeholder("Type here...")
                    .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
                    .on_change(move |text, state| {
                        wrap_notes(ComprehensiveMessage::NotesChanged(text, state))
                    })
                    .on_submit(move |text| {
                        wrap_notes_submit(ComprehensiveMessage::NotesSubmitted(text))
                    })
                    .on_undo_point(undo_ctx.callback_with_label("notes"))
                    .build();
                c.text_sized(format!("Text: \"{}\"", notes_text), 9.0);
            });
        sidebar_ctx.add(Element::new(collapsible_widgets));

        // Info Collapsible
        let info_s = info_state.clone();
        let collapsible_info = crate::Collapsible::new("Info")
            .state(&info_s)
            .width(Length::Fill(1.0))
            .on_toggle(move |s| wrap_info(ComprehensiveMessage::InfoToggled(s)))
            .content(|c| {
                c.text_sized("hvat_ui Framework Demo", 12.0);
                c.text_sized("", 8.0);
                c.text_sized("Features demonstrated:", 11.0);
                c.text_sized("- Image viewer with pan/zoom", 10.0);
                c.text_sized("- Collapsible sections", 10.0);
                c.text_sized("- Scrollable containers", 10.0);
                c.text_sized("- Sliders with inputs", 10.0);
                c.text_sized("- Number inputs", 10.0);
                c.text_sized("- Dropdown (searchable)", 10.0);
                c.text_sized("- Text input", 10.0);
                c.text_sized("- Buttons", 10.0);
                c.text_sized("- Row/Column layout", 10.0);
            });
        sidebar_ctx.add(Element::new(collapsible_info));

        // Wrap in scrollable
        let content = Element::new(Column::new(sidebar_ctx.take()));
        let scrollable = Scrollable::new(content)
            .state(&scroll_state)
            .direction(ScrollDirection::Vertical)
            .scrollbar_visibility(ScrollbarVisibility::Auto)
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill(1.0))
            .on_scroll(move |s| wrap_scroll(ComprehensiveMessage::LeftScrolled(s)));

        Element::new(scrollable)
    }

    fn build_image_viewer<M: Clone + 'static>(
        &self,
        wrap: impl Fn(ComprehensiveMessage) -> M + Clone + 'static,
        viewer_state: ImageViewerState,
        texture_id: Option<TextureId>,
        texture_size: (u32, u32),
    ) -> Element<M> {
        let wrap_change = wrap.clone();

        let mut ctx = Context::new();

        if let Some(tex_id) = texture_id {
            ctx.image_viewer(tex_id, texture_size.0, texture_size.1)
                .state(&viewer_state)
                .show_controls(true)
                .width(Length::Fill(1.0))
                .height(Length::Fill(1.0))
                .on_change(move |s| wrap_change(ComprehensiveMessage::ViewerChanged(s)))
                .build();
        } else {
            ctx.image_viewer_empty()
                .state(&viewer_state)
                .show_controls(true)
                .width(Length::Fill(1.0))
                .height(Length::Fill(1.0))
                .build();
        }

        Element::new(Column::new(ctx.take()))
    }

    fn build_right_sidebar<M: Clone + 'static>(
        &self,
        wrap: impl Fn(ComprehensiveMessage) -> M + Clone + 'static,
        brightness_state: SliderState,
        contrast_state: SliderState,
        gamma_state: SliderState,
        saturation_state: SliderState,
        zoom_state: SliderState,
        scroll_state: ScrollState,
        slider_undo_snapshot: DemoSnapshot,
        undo_stack: Rc<RefCell<UndoStack<DemoSnapshot>>>,
    ) -> Element<M> {
        let wrap_brightness = wrap.clone();
        let wrap_contrast = wrap.clone();
        let wrap_gamma = wrap.clone();
        let wrap_saturation = wrap.clone();
        let wrap_zoom = wrap.clone();
        let wrap_scroll = wrap.clone();
        let wrap_reset = wrap.clone();

        // Create UndoContext for clean on_undo_point callbacks
        let undo_ctx = UndoContext::new(undo_stack, slider_undo_snapshot);

        let mut sidebar_ctx = Context::new();

        sidebar_ctx.text_sized("View Controls", 16.0);
        sidebar_ctx.text("");

        // Zoom slider (percentage) - this one actually works!
        sidebar_ctx.text_sized("Zoom % (controls viewer)", 12.0);
        sidebar_ctx.slider(10.0, 500.0)
            .state(&zoom_state)
            .step(1.0)
            .show_input(true)
            .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
            .on_change(move |s| wrap_zoom(ComprehensiveMessage::ZoomSliderChanged(s)))
            .build();
        sidebar_ctx.text("");

        // Separator
        sidebar_ctx.text("────────────────────");
        sidebar_ctx.text_sized("Demo Sliders", 14.0);
        sidebar_ctx.text_sized("(values stored, no image effect)", 10.0);
        sidebar_ctx.text("");

        // Brightness slider
        sidebar_ctx.text_sized(format!("Brightness: {:.2}", brightness_state.value), 12.0);
        sidebar_ctx.slider(-1.0, 1.0)
            .state(&brightness_state)
            .step(0.01)
            .show_input(true)
            .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
            .on_change(move |s| wrap_brightness(ComprehensiveMessage::BrightnessChanged(s)))
            .on_undo_point(undo_ctx.callback_with_label("brightness"))
            .build();
        sidebar_ctx.text("");

        // Contrast slider
        sidebar_ctx.text_sized(format!("Contrast: {:.2}", contrast_state.value), 12.0);
        sidebar_ctx.slider(0.0, 3.0)
            .state(&contrast_state)
            .step(0.01)
            .show_input(true)
            .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
            .on_change(move |s| wrap_contrast(ComprehensiveMessage::ContrastChanged(s)))
            .on_undo_point(undo_ctx.callback_with_label("contrast"))
            .build();
        sidebar_ctx.text("");

        // Gamma slider
        sidebar_ctx.text_sized(format!("Gamma: {:.2}", gamma_state.value), 12.0);
        sidebar_ctx.slider(0.1, 3.0)
            .state(&gamma_state)
            .step(0.01)
            .show_input(true)
            .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
            .on_change(move |s| wrap_gamma(ComprehensiveMessage::GammaChanged(s)))
            .on_undo_point(undo_ctx.callback_with_label("gamma"))
            .build();
        sidebar_ctx.text("");

        // Saturation slider
        sidebar_ctx.text_sized(format!("Saturation: {:.2}", saturation_state.value), 12.0);
        sidebar_ctx.slider(0.0, 2.0)
            .state(&saturation_state)
            .step(0.01)
            .show_input(true)
            .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
            .on_change(move |s| wrap_saturation(ComprehensiveMessage::SaturationChanged(s)))
            .on_undo_point(undo_ctx.callback_with_label("saturation"))
            .build();
        sidebar_ctx.text("");

        // Reset button
        sidebar_ctx.button("Reset Sliders")
            .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
            .on_click(wrap_reset(ComprehensiveMessage::ResetAdjustments));

        sidebar_ctx.text("");
        sidebar_ctx.text("────────────────────");
        sidebar_ctx.text_sized("Keyboard shortcuts:", 11.0);
        sidebar_ctx.text_sized("0 - Zoom to 100%", 10.0);
        sidebar_ctx.text_sized("F - Fit to window", 10.0);
        sidebar_ctx.text_sized("+/- - Zoom in/out", 10.0);
        sidebar_ctx.text_sized("Drag - Pan image", 10.0);
        sidebar_ctx.text_sized("Scroll - Zoom", 10.0);

        // Wrap in scrollable
        let content = Element::new(Column::new(sidebar_ctx.take()));
        let scrollable = Scrollable::new(content)
            .state(&scroll_state)
            .direction(ScrollDirection::Vertical)
            .scrollbar_visibility(ScrollbarVisibility::Auto)
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill(1.0))
            .on_scroll(move |s| wrap_scroll(ComprehensiveMessage::RightScrolled(s)));

        Element::new(scrollable)
    }

    pub fn update(&mut self, message: ComprehensiveMessage) {
        match message {
            // Image viewer
            ComprehensiveMessage::ViewerChanged(state) => {
                self.viewer_state = state;
                // Sync zoom slider and offset inputs with viewer state
                // state.zoom is always the actual zoom value (single source of truth)
                let zoom_percent = (self.viewer_state.zoom * 100.0).clamp(10.0, 500.0);
                // Use set_value to sync both value and input_text
                self.zoom_slider.set_value(zoom_percent);
                self.x_offset_input.set_value(self.viewer_state.pan.0);
                self.y_offset_input.set_value(self.viewer_state.pan.1);
                log::debug!(
                    "Viewer changed: zoom={:.0}%, pan=({:.1}, {:.1}), mode={:?}",
                    zoom_percent,
                    self.viewer_state.pan.0,
                    self.viewer_state.pan.1,
                    self.viewer_state.fit_mode
                );
            }
            ComprehensiveMessage::ResetView => {
                // Reset to fit-to-view mode
                // The actual zoom value (1.0 for fit) will be applied when the viewer processes it
                self.viewer_state.reset();
                // For FitToView, zoom is always 1.0 (100%)
                self.zoom_slider.set_value(100.0);
                self.x_offset_input.set_value(0.0);
                self.y_offset_input.set_value(0.0);
                log::info!("View reset (FitToView mode, zoom=100%)");
            }
            ComprehensiveMessage::FitToWindow => {
                // Fit to window - zoom is 1.0 (100% of view)
                self.viewer_state.set_fit_to_view();
                self.zoom_slider.set_value(100.0);
                self.x_offset_input.set_value(0.0);
                self.y_offset_input.set_value(0.0);
                log::info!("Fit to window (zoom=100%)");
            }
            ComprehensiveMessage::ZoomToActual => {
                // 1:1 pixel mapping - set_one_to_one will calculate zoom if cached sizes available
                self.viewer_state.set_one_to_one();
                // Update slider with calculated zoom (if available from cached sizes)
                let zoom_percent = (self.viewer_state.zoom * 100.0).clamp(10.0, 500.0);
                self.zoom_slider.set_value(zoom_percent);
                self.x_offset_input.set_value(0.0);
                self.y_offset_input.set_value(0.0);
                log::info!("Zoom to 1:1 (zoom={:.0}%)", zoom_percent);
            }

            // Left sidebar collapsibles
            ComprehensiveMessage::ViewSettingsToggled(state) => {
                self.view_settings_collapsed = state;
            }
            ComprehensiveMessage::ToolsToggled(state) => {
                self.tools_collapsed = state;
            }
            ComprehensiveMessage::InfoToggled(state) => {
                self.info_collapsed = state;
            }
            ComprehensiveMessage::WidgetsToggled(state) => {
                self.widgets_collapsed = state;
            }
            ComprehensiveMessage::LeftScrolled(state) => {
                self.left_scroll_state = state;
            }

            // Tool selection
            ComprehensiveMessage::ToolSelected(tool) => {
                self.selected_tool = tool;
                log::info!("Tool selected: {:?}", tool);
            }

            // Right sidebar sliders
            ComprehensiveMessage::BrightnessChanged(state) => {
                self.brightness_slider = state;
            }
            ComprehensiveMessage::ContrastChanged(state) => {
                self.contrast_slider = state;
            }
            ComprehensiveMessage::GammaChanged(state) => {
                self.gamma_slider = state;
            }
            ComprehensiveMessage::SaturationChanged(state) => {
                self.saturation_slider = state;
            }
            ComprehensiveMessage::ZoomSliderChanged(state) => {
                self.zoom_slider = state;
                // Sync to viewer - switch to Manual mode so the zoom value is used
                self.viewer_state.zoom = self.zoom_slider.value / 100.0;
                self.viewer_state.fit_mode = crate::state::FitMode::Manual;
                log::debug!("Zoom slider: {:.0}% (Manual mode)", self.zoom_slider.value);
            }
            ComprehensiveMessage::RightScrolled(state) => {
                self.right_scroll_state = state;
            }
            ComprehensiveMessage::ResetAdjustments => {
                self.brightness_slider.set_value(0.0);
                self.contrast_slider.set_value(1.0);
                self.gamma_slider.set_value(1.0);
                self.saturation_slider.set_value(1.0);
                log::info!("Adjustments reset");
            }

            // Number inputs - update viewer pan position
            ComprehensiveMessage::XOffsetChanged(value, state) => {
                self.x_offset_input = state;
                self.viewer_state.pan.0 = value;
                // Switch to manual mode when panning
                self.viewer_state.fit_mode = crate::state::FitMode::Manual;
                log::debug!("X offset: {} -> pan updated (Manual mode)", value);
            }
            ComprehensiveMessage::YOffsetChanged(value, state) => {
                self.y_offset_input = state;
                self.viewer_state.pan.1 = value;
                // Switch to manual mode when panning
                self.viewer_state.fit_mode = crate::state::FitMode::Manual;
                log::debug!("Y offset: {} -> pan updated (Manual mode)", value);
            }

            // Dropdown
            ComprehensiveMessage::BlendModeSelected(idx) => {
                self.selected_blend_mode = Some(idx);
                self.blend_mode_dropdown.close();
                log::info!("Blend mode selected: {} ({})", BLEND_MODES[idx], idx);
            }
            ComprehensiveMessage::BlendModeDropdownChanged(state) => {
                self.blend_mode_dropdown = state;
            }

            // Text input
            ComprehensiveMessage::NotesChanged(text, state) => {
                self.notes_input_text = text;
                self.notes_input_state = state;
            }

            ComprehensiveMessage::NotesSubmitted(text) => {
                log::info!("Notes submitted: \"{}\"", text);
            }

            // Global undo/redo
            ComprehensiveMessage::Undo => {
                let current = self.snapshot();
                let prev = self.undo_stack.borrow_mut().undo(current);
                if let Some(prev) = prev {
                    self.restore(&prev);
                    log::info!(
                        "Undo: brightness={:.2}, text='{}'",
                        self.brightness_slider.value,
                        self.notes_input_text
                    );
                }
            }
            ComprehensiveMessage::Redo => {
                let current = self.snapshot();
                let next = self.undo_stack.borrow_mut().redo(current);
                if let Some(next) = next {
                    self.restore(&next);
                    log::info!(
                        "Redo: brightness={:.2}, text='{}'",
                        self.brightness_slider.value,
                        self.notes_input_text
                    );
                }
            }
        }
    }
}

/// Create a gradient test pattern for the image viewer
pub fn create_gradient_pattern(width: u32, height: u32) -> Vec<u8> {
    let mut data = vec![0u8; (width * height * 4) as usize];
    let tile_size = 64;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let tile_x = (x / tile_size) as usize;
            let tile_y = (y / tile_size) as usize;

            // Create a visually interesting pattern
            let (r, g, b) = if (tile_x + tile_y) % 2 == 0 {
                // Gradient tile
                let r = ((x as f32 / width as f32) * 200.0 + 55.0) as u8;
                let g = ((y as f32 / height as f32) * 200.0 + 55.0) as u8;
                let b = (((x + y) as f32 / (width + height) as f32) * 150.0 + 50.0) as u8;
                (r, g, b)
            } else {
                // Darker tile with subtle gradient
                let base = 30 + ((x % tile_size + y % tile_size) / 4) as u8;
                (base, base, base + 10)
            };

            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    data
}
