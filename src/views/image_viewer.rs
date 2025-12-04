//! Image viewer view - main image viewing and annotation interface.

use crate::annotation::{AnnotationStore, AnnotationTool, Category, DrawingState, Shape};
use crate::hvat_app::Tag;
use crate::hyperspectral::BandSelection;
use crate::message::{ExportFormat, Message, PersistenceMode};
use crate::theme::Theme;
use std::collections::HashSet;
use crate::ui_constants::{
    annotation as ann_const, colors, image_viewer as img_const, padding,
    sidebar as sidebar_const, spacing, text as text_const, title_bar as title_const,
};
use crate::views::helpers::tool_button;
use crate::widget_state::WidgetState;
use hvat_ui::widgets::{
    button, collapsible, column, container, dropdown, hyperspectral_image, row, scrollable, slider,
    text, text_input, titled_container, Column, Element, Row, ScrollDirection, SliderId,
};
use hvat_ui::{
    BandSelectionUniform, Color, HyperspectralImageHandle, ImageAdjustments, Length, Overlay,
    OverlayItem, OverlayShape,
};

/// Build an overlay from annotations and drawing state.
pub fn build_overlay(annotations: &AnnotationStore, drawing_state: &DrawingState) -> Overlay {
    let mut overlay = Overlay::new();

    // Add all annotations
    for ann in annotations.iter() {
        // Get category color
        let cat_color = annotations
            .get_category(ann.category_id)
            .map(|c| Color::new(c.color[0], c.color[1], c.color[2], c.color[3]))
            .unwrap_or(colors::DEFAULT_GRAY);

        let shape = match &ann.shape {
            Shape::Point(p) => OverlayShape::Point {
                x: p.x,
                y: p.y,
                radius: ann_const::POINT_RADIUS,
            },
            Shape::BoundingBox(b) => OverlayShape::Rect {
                x: b.x,
                y: b.y,
                width: b.width,
                height: b.height,
            },
            Shape::Polygon(poly) => OverlayShape::Polygon {
                vertices: poly.vertices.iter().map(|p| (p.x, p.y)).collect(),
                closed: poly.closed,
            },
        };

        let selected = annotations.selected() == Some(ann.id);
        overlay.push(OverlayItem::new(shape, cat_color).selected(selected));

        // Add drag handles for selected annotation (when in Select mode)
        if selected && drawing_state.tool == AnnotationTool::Select {
            let handle_color = Color::WHITE;
            let handle_radius = 4.0; // Small handle circles

            for (_handle_type, pos) in ann.shape.get_handles() {
                overlay.push(OverlayItem::new(
                    OverlayShape::Point {
                        x: pos.x,
                        y: pos.y,
                        radius: handle_radius,
                    },
                    handle_color,
                ));
            }
        }
    }

    // Add preview for in-progress drawing
    if let Some(preview_shape) = drawing_state.preview() {
        let cat_color = annotations
            .get_category(drawing_state.current_category)
            .map(|c| Color::new(c.color[0], c.color[1], c.color[2], ann_const::PREVIEW_ALPHA))
            .unwrap_or(Color::new(0.7, 0.7, 0.7, ann_const::PREVIEW_ALPHA));

        let shape = match preview_shape {
            Shape::Point(p) => OverlayShape::Point {
                x: p.x,
                y: p.y,
                radius: ann_const::POINT_RADIUS,
            },
            Shape::BoundingBox(b) => OverlayShape::Rect {
                x: b.x,
                y: b.y,
                width: b.width,
                height: b.height,
            },
            Shape::Polygon(poly) => OverlayShape::Polygon {
                vertices: poly.vertices.iter().map(|p| (p.x, p.y)).collect(),
                closed: false, // Preview is always open
            },
        };

        overlay.set_preview(Some(OverlayItem::new(shape, cat_color)));
    }

    overlay
}

/// Build the annotation toolbar (compact version for sidebar).
/// Tool shortcuts: s=Select, b=Box, m=Mask (polygon), p=Point
fn view_annotation_toolbar_compact(
    tool: AnnotationTool,
    _text_color: Color,
) -> Column<'static, Message> {
    column()
        .push(Element::new(
            row()
                .push(tool_button("Sel(s)", AnnotationTool::Select, tool))
                .push(tool_button("Box(b)", AnnotationTool::BoundingBox, tool))
                .push(tool_button("Mask(m)", AnnotationTool::Polygon, tool))
                .push(tool_button("Pt(p)", AnnotationTool::Point, tool))
                .spacing(spacing::TIGHT)
                .wrap(), // Wrap to next line if not enough space
        ))
        .push(Element::new(
            row()
                .push(Element::new(
                    button("Del(⌫)")
                        .on_press(Message::delete_selected_annotation())
                        .width(60.0),
                ))
                .push(Element::new(
                    button("Esc")
                        .on_press(Message::tool_shortcut('\x1b'))
                        .width(45.0),
                ))
                .push(Element::new(
                    button("Exp")
                        .on_press(Message::export_annotations())
                        .width(45.0),
                ))
                .push(Element::new(
                    button("Clr")
                        .on_press(Message::clear_annotations())
                        .width(45.0),
                ))
                .spacing(spacing::TIGHT)
                .wrap(), // Wrap to next line if not enough space
        ))
        .spacing(spacing::TIGHT)
}

/// Build the export modal content (centered dialog with format selection).
pub fn view_export_modal_content(current_format: ExportFormat) -> Column<'static, Message> {
    column()
        .push(Element::new(
            text("Export Annotations")
                .size(20.0)
                .color(colors::ACCENT),
        ))
        .push(Element::new(
            text("Select export format:")
                .size(text_const::NORMAL)
                .color(colors::MUTED_TEXT),
        ))
        // Format selection buttons (radio-button style)
        .push(Element::new(view_format_buttons(current_format)))
        .push(Element::new(
            row()
                .push(Element::new(
                    button("Export")
                        .on_press(Message::perform_export())
                        .width(120.0),
                ))
                .push(Element::new(
                    button("Cancel")
                        .on_press(Message::close_export_dialog())
                        .width(100.0),
                ))
                .spacing(spacing::NORMAL),
        ))
        .spacing(spacing::NORMAL)
}

/// Build format selection buttons.
fn view_format_buttons(current_format: ExportFormat) -> Column<'static, Message> {
    let mut col = column().spacing(spacing::TIGHT);

    for format in ExportFormat::all() {
        let is_selected = *format == current_format;
        let label = if is_selected {
            format!("[{}]", format.name())
        } else {
            format.name().to_string()
        };

        col = col.push(Element::new(
            button(label)
                .on_press(Message::set_export_format(*format))
                .width(200.0),
        ));
    }

    col
}

/// Build the category selector (compact list with hotkeys).
/// Shows categories as: "[color] 1. CategoryName [id]" with current selection highlighted.
/// Includes text input for adding new categories.
fn view_category_selector(
    categories: Vec<&Category>,
    current_category: u32,
    _text_color: Color,
    new_category_text: &str,
    is_input_focused: bool,
) -> Column<'static, Message> {
    let mut col = column().spacing(2.0);

    // Sort categories by ID for consistent display
    let mut sorted_cats: Vec<_> = categories.into_iter().collect();
    sorted_cats.sort_by_key(|c| c.id);

    // Show up to 9 categories (hotkeys 1-9)
    for (idx, cat) in sorted_cats.iter().take(9).enumerate() {
        let hotkey = idx + 1; // 1-9
        let is_selected = cat.id == current_category;

        // Format: "1. Name [0]" where 1 is hotkey and [0] is category ID
        let label = format!("{}. {} [{}]", hotkey, cat.name, cat.id);

        // Get category color
        let cat_color = Color::new(cat.color[0], cat.color[1], cat.color[2], cat.color[3]);

        // Show ">" inside the color button when selected
        let color_label = if is_selected { ">" } else { "" };

        // Row with color square (with arrow if selected) and label button
        let category_id = cat.id;
        col = col.push(Element::new(
            row()
                .push(Element::new(
                    button(color_label)
                        .width(18.0)
                        .height(18.0)
                        .bg_color(cat_color),
                ))
                .push(Element::new(
                    button(label)
                        .on_press(Message::set_annotation_category(category_id))
                        .width(sidebar_const::WIDTH - 32.0)
                        .height(22.0),
                ))
                .spacing(4.0),
        ));
    }

    // Add new category input row
    col = col.push(Element::new(
        row()
            .push(Element::new(
                text_input(new_category_text)
                    .placeholder("New category...")
                    .width(sidebar_const::WIDTH - 55.0)
                    .height(24.0)
                    .focused(is_input_focused)
                    .on_change(Message::set_new_category_text)
                    .on_focus(Message::set_category_input_focused)
                    .on_submit(|_| Message::submit_new_category()),
            ))
            .push(Element::new(
                button("+")
                    .on_press(Message::submit_new_category())
                    .width(35.0)
                    .height(24.0),
            ))
            .spacing(spacing::TIGHT),
    ));

    col
}

/// Build the tag selector (compact list with Ctrl+hotkeys).
/// Shows tags as: "[color] Ctrl+1. TagName" with checkmarks for active tags.
/// Includes text input for adding new tags.
fn view_tag_selector(
    tags: &[Tag],
    active_tags: &HashSet<u32>,
    _text_color: Color,
    new_tag_text: &str,
    is_input_focused: bool,
) -> Column<'static, Message> {
    let mut col = column().spacing(2.0);

    // Header
    col = col.push(Element::new(
        text("Image Tags (Ctrl+1-9)").size(text_const::SMALL),
    ));

    // Sort tags by ID for consistent display
    let mut sorted_tags: Vec<_> = tags.iter().collect();
    sorted_tags.sort_by_key(|t| t.id);

    // Show up to 9 tags (hotkeys Ctrl+1-9)
    for (idx, tag) in sorted_tags.iter().take(9).enumerate() {
        let hotkey = idx + 1; // 1-9
        let is_active = active_tags.contains(&tag.id);

        // Format: "1. Name" where 1 is hotkey
        let label = format!("{}. {}", hotkey, tag.name);

        // Get tag color
        let tag_color = Color::new(tag.color[0], tag.color[1], tag.color[2], tag.color[3]);

        // Show checkmark inside the color button when active
        let color_label = if is_active { "✓" } else { "" };

        // Row with color square (with checkmark if active) and label button
        let tag_id = tag.id;
        col = col.push(Element::new(
            row()
                .push(Element::new(
                    button(color_label)
                        .on_press(Message::toggle_tag(tag_id))
                        .width(18.0)
                        .height(18.0)
                        .bg_color(tag_color),
                ))
                .push(Element::new(
                    button(label)
                        .on_press(Message::toggle_tag(tag_id))
                        .width(sidebar_const::WIDTH - 32.0)
                        .height(22.0),
                ))
                .spacing(4.0),
        ));
    }

    // Add new tag input row
    col = col.push(Element::new(
        row()
            .push(Element::new(
                text_input(new_tag_text)
                    .placeholder("New tag...")
                    .width(sidebar_const::WIDTH - 55.0)
                    .height(24.0)
                    .focused(is_input_focused)
                    .on_change(Message::set_new_tag_text)
                    .on_focus(Message::set_tag_input_focused)
                    .on_submit(|_| Message::submit_new_tag()),
            ))
            .push(Element::new(
                button("+")
                    .on_press(Message::submit_new_tag())
                    .width(35.0)
                    .height(24.0),
            ))
            .spacing(spacing::TIGHT),
    ));

    col
}

/// Build the image viewer view.
///
/// Layout: Main row with image panel (left) and sidebar (right).
/// - Image panel: titled container with the hyperspectral image
/// - Sidebar: all settings (file controls, zoom, adjustments, bands, annotations)
#[allow(clippy::too_many_arguments)]
pub fn view_image_viewer<'a>(
    theme: &Theme,
    text_color: Color,
    hyperspectral_handle: &'a HyperspectralImageHandle,
    zoom: f32,
    pan_x: f32,
    pan_y: f32,
    brightness: f32,
    contrast: f32,
    gamma: f32,
    hue_shift: f32,
    widget_state: &'a WidgetState,
    drawing_state: &'a DrawingState,
    annotations: &'a AnnotationStore,
    status_message: Option<&'a str>,
    band_selection: &BandSelection,
    num_bands: usize,
    overlay: Overlay,
    band_persistence: PersistenceMode,
    image_settings_persistence: PersistenceMode,
    available_tags: &'a [Tag],
    current_image_tags: &'a HashSet<u32>,
) -> Row<'a, Message> {
    // Create image adjustments from current settings
    let adjustments = ImageAdjustments {
        brightness,
        contrast,
        gamma,
        hue_shift,
    };

    // Convert BandSelection to GPU uniform format
    let band_uniform = BandSelectionUniform::new(
        band_selection.red,
        band_selection.green,
        band_selection.blue,
        num_bands,
    );

    // Create the GPU-accelerated hyperspectral image widget
    // Uses Fill to expand to available space
    let image_widget = hyperspectral_image(hyperspectral_handle.clone(), band_uniform)
        .pan((pan_x, pan_y))
        .zoom(zoom)
        .dragging(widget_state.image.is_dragging)
        .drawing(drawing_state.is_drawing || drawing_state.editing.is_dragging)
        .adjustments(adjustments)
        .overlay(overlay)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_drag_start(Message::image_drag_start)
        .on_drag_move(Message::image_drag_move)
        .on_drag_end(Message::image_drag_end)
        .on_zoom(Message::image_zoom_at_point)
        .on_click(|(x, y)| Message::start_drawing(x, y))
        .on_draw_move(|(x, y)| Message::continue_drawing(x, y))
        .on_draw_end(Message::finish_drawing)
        .on_space(Message::force_finish_polygon)
        .on_number_key(|num| {
            // Map hotkey (1-9) to category ID based on sorted order
            // Hotkey 1 = first category (usually ID 0), etc.
            Message::select_category_by_hotkey(num)
        })
        .on_ctrl_number_key(|num| {
            // Ctrl+1-9 toggles tags
            Message::toggle_tag_by_hotkey(num)
        })
        .on_tool_key(Message::tool_shortcut)
        // Disable keyboard shortcuts when any text input is focused
        .keyboard_disabled(widget_state.category_input.is_focused || widget_state.tag_input.is_focused);

    // === LEFT PANEL: Image with titled container (fills available space) ===
    let image_panel = titled_container("Image", Element::new(image_widget))
        .fill()
        .padding(padding::MINIMAL)
        .border(colors::BORDER)
        .border_width(img_const::BORDER_WIDTH)
        .title_bg_color(title_const::BG_COLOR)
        .title_text_color(title_const::TEXT_COLOR);

    // Status text
    let status_text = status_message.unwrap_or("No images loaded");

    // === RIGHT PANEL: Sidebar with all controls ===
    let sidebar = column()
        // File controls
        .push(Element::new(
            row()
                .push(Element::new(
                    button("Load")
                        .on_press(Message::load_folder())
                        .width(60.0),
                ))
                .push(Element::new(
                    button("<")
                        .on_press(Message::previous_image())
                        .width(35.0),
                ))
                .push(Element::new(
                    button(">")
                        .on_press(Message::next_image())
                        .width(35.0),
                ))
                .spacing(spacing::TIGHT),
        ))
        .push(Element::new(
            text(status_text)
                .size(text_const::SMALL)
                .color(text_color),
        ))
        // View controls
        .push(Element::new(
            row()
                .push(Element::new(
                    text(format!("Zoom: {:.1}x", zoom))
                        .size(text_const::SMALL)
                        .color(text_color),
                ))
                .push(Element::new(
                    button("+")
                        .on_press(Message::zoom_in())
                        .width(30.0),
                ))
                .push(Element::new(
                    button("-")
                        .on_press(Message::zoom_out())
                        .width(30.0),
                ))
                .push(Element::new(
                    button("Reset")
                        .on_press(Message::reset_view())
                        .width(55.0),
                ))
                .spacing(spacing::TIGHT),
        ))
        // Image Settings section (collapsible)
        .push(Element::new(
            collapsible(
                "Adjustments",
                Element::new(view_image_settings_sidebar(
                    text_color,
                    brightness,
                    contrast,
                    gamma,
                    hue_shift,
                    widget_state,
                )),
            )
            .collapsed(widget_state.collapsible.image_settings_collapsed)
            .on_toggle(|_| Message::toggle_image_settings_collapsed())
            .text_color(text_color)
            .header_color(theme.accent_color().with_alpha(0.3)),
        ))
        // Band Selection section (collapsible)
        .push(Element::new(
            collapsible(
                "Bands",
                Element::new(view_band_selector_sidebar(
                    text_color,
                    widget_state,
                    band_selection,
                    num_bands,
                )),
            )
            .collapsed(widget_state.collapsible.band_settings_collapsed)
            .on_toggle(|_| Message::toggle_band_settings_collapsed())
            .text_color(text_color)
            .header_color(theme.accent_color().with_alpha(0.3)),
        ))
        // Annotation tools
        .push(Element::new(view_annotation_toolbar_compact(
            drawing_state.tool,
            text_color,
        )))
        // Category selector (compact list with hotkeys 1-9)
        .push(Element::new(view_category_selector(
            annotations.categories().collect(),
            drawing_state.current_category,
            text_color,
            &widget_state.category_input.new_category_name,
            widget_state.category_input.is_focused,
        )))
        // Tag selector (compact list with Ctrl+hotkeys 1-9)
        .push(Element::new(view_tag_selector(
            available_tags,
            current_image_tags,
            text_color,
            &widget_state.tag_input.new_tag_name,
            widget_state.tag_input.is_focused,
        )))
        .push(Element::new(
            text(format!(
                "{} annotations | {}",
                annotations.len(),
                if drawing_state.is_drawing {
                    "Drawing..."
                } else {
                    "Ready"
                }
            ))
            .size(text_const::SMALL)
            .color(colors::MUTED_TEXT),
        ))
        // Persistence settings
        .push(Element::new(view_persistence_row(
            "Bands:",
            text_color,
            band_persistence,
            widget_state.dropdown.band_persistence_open,
            true,
        )))
        .push(Element::new(view_persistence_row(
            "Image:",
            text_color,
            image_settings_persistence,
            widget_state.dropdown.image_settings_persistence_open,
            false,
        )))
        .spacing(spacing::TIGHT);

    // Main layout: image panel (fills) + sidebar (fixed width, scrollable, fills height)
    // Sidebar scrollable fills available height (no fixed height specified)
    let mut sidebar_scrollable = scrollable(Element::new(sidebar))
        .direction(ScrollDirection::Vertical)
        .width(sidebar_const::WIDTH + 20.0) // Content width + scrollbar
        // No height specified - fills available space
        .scroll_offset_y(widget_state.sidebar_scroll.offset_y)
        .dragging_y(widget_state.sidebar_scroll.is_dragging_y)
        .on_scroll_y(Message::sidebar_scroll_y)
        .on_drag_start_y(Message::sidebar_scrollbar_drag_start_y)
        .on_drag_end_y(Message::sidebar_scrollbar_drag_end_y);

    // Pass drag start position for relative scrollbar dragging
    if let (Some(mouse_y), Some(scroll_y)) = (
        widget_state.sidebar_scroll.drag_start_mouse_y,
        widget_state.sidebar_scroll.drag_start_scroll_y,
    ) {
        sidebar_scrollable = sidebar_scrollable.drag_start_y(mouse_y, scroll_y);
    }

    // Wrap in container with border for debugging
    let sidebar_with_border = container(Element::new(sidebar_scrollable))
        .border(colors::BORDER);

    // Help text at the bottom
    let help_text = text("Middle-click drag to pan, scroll to zoom")
        .size(text_const::SMALL)
        .color(colors::MUTED_TEXT);

    // Sidebar column: scrollable content fills, help text at bottom
    let sidebar_column = column()
        .push(Element::new(sidebar_with_border))
        .push(Element::new(help_text))
        .spacing(spacing::TIGHT);

    row()
        .push(Element::new(image_panel))
        .push(Element::new(sidebar_column))
        .spacing(sidebar_const::GAP)
}

/// Build sidebar version of image settings (compact sliders).
fn view_image_settings_sidebar(
    text_color: Color,
    brightness: f32,
    contrast: f32,
    gamma: f32,
    hue_shift: f32,
    widget_state: &WidgetState,
) -> Column<'static, Message> {
    column()
        .push(Element::new(view_compact_slider(
            "Brightness",
            text_color,
            -1.0,
            1.0,
            brightness,
            SliderId::Brightness,
            widget_state.slider.is_dragging(SliderId::Brightness),
            Message::set_brightness,
        )))
        .push(Element::new(view_compact_slider(
            "Contrast",
            text_color,
            0.1,
            3.0,
            contrast,
            SliderId::Contrast,
            widget_state.slider.is_dragging(SliderId::Contrast),
            Message::set_contrast,
        )))
        .push(Element::new(view_compact_slider(
            "Gamma",
            text_color,
            0.1,
            3.0,
            gamma,
            SliderId::Gamma,
            widget_state.slider.is_dragging(SliderId::Gamma),
            Message::set_gamma,
        )))
        .push(Element::new(view_compact_slider(
            "Hue",
            text_color,
            -180.0,
            180.0,
            hue_shift,
            SliderId::HueShift,
            widget_state.slider.is_dragging(SliderId::HueShift),
            Message::set_hue_shift,
        )))
        .push(Element::new(
            button("Reset")
                .on_press(Message::reset_image_settings())
                .width(sidebar_const::WIDTH - 20.0),
        ))
        .spacing(spacing::TIGHT)
}

/// Compact slider with label above.
fn view_compact_slider<F>(
    label: &str,
    text_color: Color,
    min: f32,
    max: f32,
    value: f32,
    slider_id: SliderId,
    is_dragging: bool,
    on_change: F,
) -> Column<'static, Message>
where
    F: Fn(f32) -> Message + 'static,
{
    let value_str = if max > 100.0 {
        format!("{}: {:.0}", label, value)
    } else {
        format!("{}: {:.2}", label, value)
    };

    column()
        .push(Element::new(
            text(value_str).size(text_const::SMALL).color(text_color),
        ))
        .push(Element::new(
            slider(min, max, value)
                .id(slider_id)
                .dragging(is_dragging)
                .width(Length::Units(sidebar_const::WIDTH - 20.0))
                .on_drag_start(Message::slider_drag_start)
                .on_change(on_change)
                .on_drag_end(Message::slider_drag_end),
        ))
        .spacing(2.0)
}

/// Build sidebar version of band selector.
fn view_band_selector_sidebar(
    _text_color: Color,
    widget_state: &WidgetState,
    band_selection: &BandSelection,
    num_bands: usize,
) -> Column<'static, Message> {
    let max_band = num_bands.saturating_sub(1) as f32;

    column()
        .push(Element::new(
            text(format!("{} bands", num_bands))
                .size(text_const::SMALL)
                .color(colors::MUTED_TEXT),
        ))
        .push(Element::new(view_band_slider_compact(
            "R",
            colors::CHANNEL_RED,
            max_band,
            band_selection.red as f32,
            SliderId::BandRed,
            widget_state.slider.is_dragging(SliderId::BandRed),
            |_id, value| Message::start_red_band(value as usize),
            |v| Message::set_red_band(v as usize),
        )))
        .push(Element::new(view_band_slider_compact(
            "G",
            colors::CHANNEL_GREEN,
            max_band,
            band_selection.green as f32,
            SliderId::BandGreen,
            widget_state.slider.is_dragging(SliderId::BandGreen),
            |_id, value| Message::start_green_band(value as usize),
            |v| Message::set_green_band(v as usize),
        )))
        .push(Element::new(view_band_slider_compact(
            "B",
            colors::CHANNEL_BLUE,
            max_band,
            band_selection.blue as f32,
            SliderId::BandBlue,
            widget_state.slider.is_dragging(SliderId::BandBlue),
            |_id, value| Message::start_blue_band(value as usize),
            |v| Message::set_blue_band(v as usize),
        )))
        .push(Element::new(
            button("Reset (0,1,2)")
                .on_press(Message::reset_bands())
                .width(sidebar_const::WIDTH - 20.0),
        ))
        .spacing(spacing::TIGHT)
}

/// Compact band slider with colored label.
fn view_band_slider_compact<S, C>(
    label: &str,
    label_color: Color,
    max_band: f32,
    value: f32,
    slider_id: SliderId,
    is_dragging: bool,
    on_drag_start: S,
    on_change: C,
) -> Row<'static, Message>
where
    S: Fn(SliderId, f32) -> Message + 'static,
    C: Fn(f32) -> Message + 'static,
{
    row()
        .push(Element::new(
            text(format!("{}: {}", label, value as usize))
                .size(text_const::SMALL)
                .color(label_color),
        ))
        .push(Element::new(
            slider(0.0, max_band, value)
                .id(slider_id)
                .step(1.0)
                .dragging(is_dragging)
                .width(Length::Units(sidebar_const::WIDTH - 70.0))
                .on_drag_start(on_drag_start)
                .on_change(on_change)
                .on_drag_end(Message::apply_bands),
        ))
        .spacing(spacing::TIGHT)
}

/// Persistence mode row with dropdown.
fn view_persistence_row(
    label: &str,
    text_color: Color,
    current_mode: PersistenceMode,
    is_open: bool,
    is_band: bool,
) -> Row<'static, Message> {
    let options = vec![
        "Reset".to_string(),
        "Per Image".to_string(),
        "Keep".to_string(),
    ];
    let selected_index = match current_mode {
        PersistenceMode::Reset => 0,
        PersistenceMode::PerImage => 1,
        PersistenceMode::Constant => 2,
    };

    let on_open: Box<dyn Fn() -> Message> = if is_band {
        Box::new(|| Message::open_band_persistence_dropdown())
    } else {
        Box::new(|| Message::open_image_settings_persistence_dropdown())
    };

    let on_close: Box<dyn Fn() -> Message> = if is_band {
        Box::new(|| Message::close_band_persistence_dropdown())
    } else {
        Box::new(|| Message::close_image_settings_persistence_dropdown())
    };

    let on_select: Box<dyn Fn(usize) -> Message> = if is_band {
        Box::new(|idx| {
            let mode = match idx {
                0 => PersistenceMode::Reset,
                1 => PersistenceMode::PerImage,
                _ => PersistenceMode::Constant,
            };
            Message::set_band_persistence(mode)
        })
    } else {
        Box::new(|idx| {
            let mode = match idx {
                0 => PersistenceMode::Reset,
                1 => PersistenceMode::PerImage,
                _ => PersistenceMode::Constant,
            };
            Message::set_image_settings_persistence(mode)
        })
    };

    row()
        .push(Element::new(
            text(label).size(text_const::SMALL).color(text_color),
        ))
        .push(Element::new(
            dropdown(options, selected_index)
                .width(80.0)
                .open(is_open)
                .on_open(on_open)
                .on_close(on_close)
                .on_select(on_select)
                .text_color(text_color)
                .overlay(true),
        ))
        .spacing(spacing::TIGHT)
}
