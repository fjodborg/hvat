//! Helper functions for building common UI patterns.
//!
//! These helpers reduce code duplication in view construction.

use crate::annotation::{AnnotationTool, AnnotationToolBehavior};
use crate::message::Message;
use crate::ui_constants::{button as btn_const, icons as icon_const, slider as slider_const, spacing, text, tooltip as tooltip_const};
use crate::widget_state::TooltipState;
use hvat_ui::icon::{get_icon, icons};
use hvat_ui::widgets::{button, icon_button, row, slider, text as text_widget, tooltip, Element, Row, SliderId, TooltipPosition};
use hvat_ui::Color;

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
/// Creates an icon button that shows active state and a tooltip on hover.
///
/// # Arguments
/// * `label` - Button label (fallback for icon failures)
/// * `tool` - The annotation tool this button represents
/// * `active_tool` - Currently selected tool
/// * `tooltip_state` - External tooltip state for hover tracking
pub fn tool_button(
    label: &str,
    tool: AnnotationTool,
    active_tool: AnnotationTool,
    tooltip_state: &TooltipState,
) -> Element<'static, Message> {
    // Use trait methods for tool metadata
    let tooltip_text = tool.tooltip();
    let icon_name = tool.icon_name();

    // Get the icon data for this tool
    let icon_data = match tool {
        AnnotationTool::Select => icons::CURSOR,
        AnnotationTool::BoundingBox => icons::BOUNDING_BOX,
        AnnotationTool::Polygon => icons::HEXAGON,
        AnnotationTool::Point => icons::GEO_ALT,
    };

    // Try to get the icon; fall back to text button if it fails
    let Some(icon) = get_icon(icon_name, icon_data, icon_const::SIZE, icon_const::COLOR) else {
        // Fallback to text button if icon rasterization fails
        let display_label = if tool == active_tool {
            format!("{} *", label)
        } else {
            label.to_string()
        };

        return Element::new(
            button(display_label)
                .on_press(Message::set_annotation_tool(tool))
                .width(btn_const::TOOL_WIDTH),
        );
    };

    // Create unique ID for this tooltip
    let tooltip_id = format!("tool_{:?}", tool);

    // Check if tooltip should be shown based on external state
    let show = tooltip_state.should_show(&tooltip_id, tooltip_const::DELAY_MS);

    // Check if this tooltip is the currently active one
    let is_hover_active = tooltip_state.current_hover() == Some(tooltip_id.as_str());

    // Is this the currently selected tool?
    let is_tool_active = tool == active_tool;

    Element::new(
        tooltip(
            Element::new(
                icon_button(icon)
                    .on_press(Message::set_annotation_tool(tool))
                    .size(icon_const::BUTTON_SIZE)
                    .active(is_tool_active),
            ),
            tooltip_text,
        )
        .position(TooltipPosition::Bottom)
        .show(show)
        .active(is_hover_active)
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

/// Build a simple icon button with tooltip.
///
/// Creates an icon button that shows a tooltip on hover.
///
/// # Arguments
/// * `icon_name` - Unique name for caching
/// * `icon_data` - SVG icon data bytes
/// * `tooltip_text` - Text shown in tooltip
/// * `message` - Message to send on click
/// * `tooltip_state` - External tooltip state for hover tracking
pub fn simple_icon_button(
    icon_name: &str,
    icon_data: &'static [u8],
    tooltip_text: &str,
    message: Message,
    tooltip_state: &TooltipState,
) -> Element<'static, Message> {
    // Try to get the icon; fall back to text button if it fails
    let Some(icon) = get_icon(icon_name, icon_data, icon_const::SIZE, icon_const::COLOR) else {
        // Fallback to text button if icon rasterization fails
        return Element::new(
            button(icon_name)
                .on_press(message)
                .width(btn_const::ZOOM_WIDTH),
        );
    };

    // Create unique ID for this tooltip
    let tooltip_id = format!("icon_{}", icon_name);
    let tooltip_id_clone = tooltip_id.clone();

    // Check if tooltip should be shown based on external state
    let show = tooltip_state.should_show(&tooltip_id, tooltip_const::DELAY_MS);

    // Check if this tooltip is the currently active one
    let is_hover_active = tooltip_state.current_hover() == Some(tooltip_id.as_str());

    let tooltip_text = tooltip_text.to_string();

    Element::new(
        tooltip(
            Element::new(
                icon_button(icon)
                    .on_press(message)
                    .size(icon_const::BUTTON_SIZE),
            ),
            tooltip_text,
        )
        .position(TooltipPosition::Bottom)
        .show(show)
        .active(is_hover_active)
        .on_hover_change(move |is_hovered| Message::tooltip_hover(tooltip_id_clone.clone(), is_hovered)),
    )
}
