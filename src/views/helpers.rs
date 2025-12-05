//! Helper functions for building common UI patterns.
//!
//! These helpers reduce code duplication in view construction.

use crate::annotation::AnnotationTool;
use crate::message::Message;
use crate::ui_constants::{button as btn_const, slider as slider_const, spacing, text};
use crate::widget_state::TooltipState;
use hvat_ui::widgets::{button, row, slider, text as text_widget, tooltip, Element, Row, SliderId, TooltipPosition};
use hvat_ui::Color;

/// Default tooltip delay in milliseconds.
const TOOLTIP_DELAY_MS: u64 = 400;

/// Build a labeled slider row.
///
/// Creates a row with a label and slider, using standard spacing and dimensions.
///
/// # Arguments
/// * `label` - Text to display (e.g., "Brightness: 0.50")
/// * `text_color` - Color for the label text
/// * `min` - Slider minimum value
/// * `max` - Slider maximum value
/// * `value` - Current slider value
/// * `slider_id` - Unique identifier for the slider
/// * `is_dragging` - Whether the slider is currently being dragged
/// * `on_change` - Message constructor for value changes
/// * `compact` - Use compact width (for band sliders) vs standard width
#[allow(dead_code)]
pub fn labeled_slider<'a, F, E>(
    label: &str,
    text_color: Color,
    min: f32,
    max: f32,
    value: f32,
    slider_id: SliderId,
    is_dragging: bool,
    on_drag_start: fn(SliderId, f32) -> Message,
    on_change: F,
    on_drag_end: E,
    compact: bool,
) -> Row<'a, Message>
where
    F: Fn(f32) -> Message + 'static,
    E: Fn() -> Message + 'static,
{
    let width = if compact {
        slider_const::compact_length()
    } else {
        slider_const::standard_length()
    };

    row()
        .push(Element::new(
            text_widget(label).size(text::SMALL).color(text_color),
        ))
        .push(Element::new(
            slider(min, max, value)
                .id(slider_id)
                .dragging(is_dragging)
                .width(width)
                .on_drag_start(on_drag_start)
                .on_change(on_change)
                .on_drag_end(on_drag_end),
        ))
        .spacing(spacing::STANDARD)
}

/// Build a labeled slider row with step increments (for integer-like values).
#[allow(dead_code)]
pub fn labeled_slider_stepped<'a, F, E>(
    label: &str,
    text_color: Color,
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    slider_id: SliderId,
    is_dragging: bool,
    on_drag_start: fn(SliderId, f32) -> Message,
    on_change: F,
    on_drag_end: E,
    compact: bool,
) -> Row<'a, Message>
where
    F: Fn(f32) -> Message + 'static,
    E: Fn() -> Message + 'static,
{
    let width = if compact {
        slider_const::compact_length()
    } else {
        slider_const::standard_length()
    };

    row()
        .push(Element::new(
            text_widget(label).size(text::SMALL).color(text_color),
        ))
        .push(Element::new(
            slider(min, max, value)
                .id(slider_id)
                .step(step)
                .dragging(is_dragging)
                .width(width)
                .on_drag_start(on_drag_start)
                .on_change(on_change)
                .on_drag_end(on_drag_end),
        ))
        .spacing(spacing::STANDARD)
}

/// Build an annotation tool button with tooltip.
///
/// Creates a button that shows active state with "*" suffix and a tooltip on hover.
///
/// # Arguments
/// * `label` - Button label (e.g., "Select", "BBox")
/// * `tool` - The annotation tool this button represents
/// * `active_tool` - Currently selected tool
/// * `tooltip_state` - External tooltip state for hover tracking
pub fn tool_button(
    label: &str,
    tool: AnnotationTool,
    active_tool: AnnotationTool,
    tooltip_state: &TooltipState,
) -> Element<'static, Message> {
    let display_label = if tool == active_tool {
        format!("{} *", label)
    } else {
        label.to_string()
    };

    let tooltip_text = match tool {
        AnnotationTool::Select => "Select and edit annotations",
        AnnotationTool::BoundingBox => "Draw bounding boxes",
        AnnotationTool::Polygon => "Draw polygon masks",
        AnnotationTool::Point => "Place point markers",
    };

    // Create unique ID for this tooltip
    let tooltip_id = format!("tool_{:?}", tool);

    // Check if tooltip should be shown based on external state
    let show = tooltip_state.should_show(&tooltip_id, TOOLTIP_DELAY_MS);

    // Check if this tooltip is the currently active one
    let is_active = tooltip_state.current_hover() == Some(tooltip_id.as_str());

    Element::new(
        tooltip(
            Element::new(
                button(display_label)
                    .on_press(Message::set_annotation_tool(tool))
                    .width(btn_const::TOOL_WIDTH),
            ),
            tooltip_text,
        )
        .position(TooltipPosition::Bottom)
        .show(show)
        .active(is_active)
        .on_hover_change(move |is_hovered| Message::tooltip_hover(tooltip_id.clone(), is_hovered)),
    )
}

/// Build a row of navigation buttons.
#[allow(dead_code)]
pub fn button_row<'a>(buttons: Vec<Element<'a, Message>>) -> Row<'a, Message> {
    let mut r = row();
    for btn in buttons {
        r = r.push(btn);
    }
    r.spacing(spacing::STANDARD)
}

/// Create a standard width button.
#[allow(dead_code)]
pub fn action_button(label: &str, message: Message) -> Element<'static, Message> {
    Element::new(
        button(label)
            .on_press(message)
            .width(btn_const::STANDARD_WIDTH),
    )
}

/// Create a compact action button.
#[allow(dead_code)]
pub fn compact_button(label: &str, message: Message) -> Element<'static, Message> {
    Element::new(
        button(label)
            .on_press(message)
            .width(btn_const::COMPACT_WIDTH),
    )
}

/// Create an extra compact button.
#[allow(dead_code)]
pub fn xcompact_button(label: &str, message: Message) -> Element<'static, Message> {
    Element::new(
        button(label)
            .on_press(message)
            .width(btn_const::XCOMPACT_WIDTH),
    )
}

/// Create a wide action button.
#[allow(dead_code)]
pub fn wide_button(label: &str, message: Message) -> Element<'static, Message> {
    Element::new(
        button(label)
            .on_press(message)
            .width(btn_const::WIDE_WIDTH),
    )
}

/// Create a zoom control button.
#[allow(dead_code)]
pub fn zoom_button(label: &str, message: Message) -> Element<'static, Message> {
    Element::new(
        button(label)
            .on_press(message)
            .width(btn_const::ZOOM_WIDTH),
    )
}
