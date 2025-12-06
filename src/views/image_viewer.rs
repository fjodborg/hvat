//! Image viewer view - main image viewing and annotation interface.

use crate::annotation::{AnnotationStore, AnnotationTool, Category, DrawingState, Shape};
use crate::hvat_app::Tag;
use crate::hyperspectral::BandSelection;
use crate::message::{ExportFormat, Message, PersistenceMode};
use crate::theme::Theme;
use std::collections::HashSet;
use crate::ui_constants::{
    annotation as ann_const, colors, image_adjust, image_viewer as img_const, padding,
    sidebar as sidebar_const, spacing, text as text_const, title_bar as title_const,
};
use crate::views::helpers::{simple_icon_button, small_icon_button, tool_button};
use crate::widget_state::WidgetState;
use hvat_ui::icon::icons;
use hvat_ui::widgets::{
    button, collapsible, column, container, dropdown, hyperspectral_image, row, scrollable, slider,
    text, text_input, titled_container, Column, Dropdown, Element, Row, ScrollDirection, SliderId,
    TitleStyle,
};
use hvat_ui::{
    BandSelectionUniform, Color, HyperspectralImageHandle, ImageAdjustments, Length, Overlay,
    OverlayItem, OverlayShape,
};

/// Build an overlay from annotations and drawing state.
pub fn build_overlay(
    annotations: &AnnotationStore,
    drawing_state: &DrawingState,
    categories: &std::collections::HashMap<u32, Category>,
) -> Overlay {
    let mut overlay = Overlay::new();

    // Add all annotations
    for ann in annotations.iter() {
        // Get category color from global categories
        let cat_color = categories
            .get(&ann.category_id)
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

            for (_handle_type, pos) in ann.shape.get_handles() {
                overlay.push(OverlayItem::new(
                    OverlayShape::Point {
                        x: pos.x,
                        y: pos.y,
                        radius: ann_const::HANDLE_RADIUS,
                    },
                    handle_color,
                ));
            }
        }
    }

    // Add preview for in-progress drawing
    if let Some(preview_shape) = drawing_state.preview() {
        let cat_color = categories
            .get(&drawing_state.current_category)
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
    tooltip_state: &crate::widget_state::TooltipState,
) -> Row<'static, Message> {
    // Single row with wrap for tool buttons
    row()
        .push(tool_button("Sel(s)", AnnotationTool::Select, tool, tooltip_state))
        .push(tool_button("Box(b)", AnnotationTool::BoundingBox, tool, tooltip_state))
        .push(tool_button("Mask(m)", AnnotationTool::Polygon, tool, tooltip_state))
        .push(tool_button("Pt(p)", AnnotationTool::Point, tool, tooltip_state))
        .spacing(spacing::TIGHT)
        .wrap()
}

/// Annotation action buttons (delete, escape, export, clear)
fn view_annotation_actions(
    tooltip_state: &crate::widget_state::TooltipState,
) -> Row<'static, Message> {
    row()
        .push(simple_icon_button(
            "trash",
            icons::TRASH,
            "Delete selected (Del)",
            Message::delete_selected_annotation(),
            tooltip_state,
        ))
        .push(simple_icon_button(
            "escape",
            icons::ESCAPE,
            "Cancel/Deselect (Esc)",
            Message::tool_shortcut('\x1b'),
            tooltip_state,
        ))
        .push(simple_icon_button(
            "download",
            icons::DOWNLOAD,
            "Export annotations",
            Message::export_annotations(),
            tooltip_state,
        ))
        .push(simple_icon_button(
            "x-circle",
            icons::X_CIRCLE,
            "Clear all annotations",
            Message::clear_annotations(),
            tooltip_state,
        ))
        .spacing(spacing::TIGHT)
        .wrap()
}

/// Build the left sidebar: FILES, TOOLS, ADJUSTMENTS, and BANDS sections
#[allow(clippy::too_many_arguments)]
fn view_left_sidebar<'a>(
    status_message: Option<&'a str>,
    tooltip_state: &'a crate::widget_state::TooltipState,
    tool: AnnotationTool,
    text_color: Color,
    theme: &Theme,
    brightness: f32,
    contrast: f32,
    gamma: f32,
    hue_shift: f32,
    widget_state: &'a WidgetState,
    band_selection: &BandSelection,
    num_bands: usize,
    band_persistence: PersistenceMode,
    image_settings_persistence: PersistenceMode,
) -> Column<'a, Message> {
    // FILES section - folder, prev/next, status
    let files_content = column()
        .push(Element::new(
            row()
                .push(simple_icon_button(
                    "folder-open",
                    icons::FOLDER_OPEN,
                    "Load folder",
                    Message::load_folder(),
                    tooltip_state,
                ))
                .push(simple_icon_button(
                    "arrow-left",
                    icons::ARROW_LEFT,
                    "Previous image (←)",
                    Message::previous_image(),
                    tooltip_state,
                ))
                .push(simple_icon_button(
                    "arrow-right",
                    icons::ARROW_RIGHT,
                    "Next image (→)",
                    Message::next_image(),
                    tooltip_state,
                ))
                .spacing(spacing::TIGHT),
        ))
        .push(Element::new(
            text(status_message.unwrap_or("No images loaded"))
                .size(text_const::SMALL)
                .color(text_color),
        ))
        .spacing(spacing::TIGHT);

    let files_section = titled_container("FILES", Element::new(files_content))
        .title_style(TitleStyle::Above)
        .padding(padding::SMALL)
        .title_bg_color(title_const::BG_COLOR)
        .title_text_color(title_const::TEXT_COLOR);

    // TOOLS section - annotation tool buttons and action buttons
    let tools_content = column()
        .push(Element::new(view_annotation_toolbar_compact(
            tool,
            text_color,
            tooltip_state,
        )))
        .push(Element::new(view_annotation_actions(tooltip_state)))
        .spacing(spacing::TIGHT);

    let tools_section = titled_container("TOOLS", Element::new(tools_content))
        .title_style(TitleStyle::Above)
        .padding(padding::SMALL)
        .title_bg_color(title_const::BG_COLOR)
        .title_text_color(title_const::TEXT_COLOR);

    // ADJUSTMENTS section - brightness, contrast, gamma, hue
    let adjustments_content = view_image_settings_compact(
        text_color,
        brightness,
        contrast,
        gamma,
        hue_shift,
        widget_state,
        image_settings_persistence,
    );

    // Reset button for ADJUST section header (small to fit in header)
    let adjust_reset_button = small_icon_button(
        "reload-adjust",
        icons::RELOAD,
        "Reset adjustments",
        Message::reset_image_settings(),
        tooltip_state,
    );

    let adjustments_section = collapsible("ADJUST", Element::new(adjustments_content))
        .collapsed(widget_state.collapsible.image_settings_collapsed)
        .on_toggle(|_| Message::toggle_image_settings_collapsed())
        .text_color(text_color)
        .header_color(theme.accent_color().with_alpha(0.3))
        .header_action(adjust_reset_button);

    // BANDS section - R, G, B band selection
    let bands_content = view_band_selector_compact(
        text_color,
        widget_state,
        band_selection,
        num_bands,
        band_persistence,
    );

    // Reset button for BANDS section header (small to fit in header)
    let bands_reset_button = small_icon_button(
        "reload-bands",
        icons::RELOAD,
        "Reset bands",
        Message::reset_bands(),
        tooltip_state,
    );

    let bands_section = collapsible("BANDS", Element::new(bands_content))
        .collapsed(widget_state.collapsible.band_settings_collapsed)
        .on_toggle(|_| Message::toggle_band_settings_collapsed())
        .text_color(text_color)
        .header_color(theme.accent_color().with_alpha(0.3))
        .header_action(bands_reset_button);

    column()
        .push(Element::new(files_section))
        .push(Element::new(tools_section))
        .push(Element::new(adjustments_section))
        .push(Element::new(bands_section))
        .spacing(spacing::TIGHT)
}

/// Build the right sidebar: TAGS and CATEGORIES sections
fn view_right_sidebar<'a>(
    categories: Vec<&'a Category>,
    current_category: u32,
    text_color: Color,
    new_category_text: &'a str,
    is_category_input_focused: bool,
    available_tags: &'a [Tag],
    current_image_tags: &'a HashSet<u32>,
    new_tag_text: &'a str,
    is_tag_input_focused: bool,
) -> Column<'a, Message> {
    // TAGS section
    let tags_content = view_tag_selector(
        available_tags,
        current_image_tags,
        text_color,
        new_tag_text,
        is_tag_input_focused,
    );

    let tags_section = titled_container("TAGS", Element::new(tags_content))
        .title_style(TitleStyle::Above)
        .padding(padding::SMALL)
        .title_bg_color(title_const::BG_COLOR)
        .title_text_color(title_const::TEXT_COLOR);

    // CATEGORIES section
    let categories_content = view_category_selector(
        categories,
        current_category,
        text_color,
        new_category_text,
        is_category_input_focused,
    );

    let categories_section = titled_container("CATEGORIES", Element::new(categories_content))
        .title_style(TitleStyle::Above)
        .padding(padding::SMALL)
        .title_bg_color(title_const::BG_COLOR)
        .title_text_color(title_const::TEXT_COLOR);

    column()
        .push(Element::new(tags_section))
        .push(Element::new(categories_section))
        .spacing(spacing::TIGHT)
}

/// Build the zoom toolbar row (sits above the image)
fn view_zoom_toolbar<'a>(
    tooltip_state: &'a crate::widget_state::TooltipState,
) -> Row<'a, Message> {
    row()
        .push(simple_icon_button(
            "zoom-in",
            icons::ZOOM_IN,
            "Zoom in (+)",
            Message::zoom_in(),
            tooltip_state,
        ))
        .push(simple_icon_button(
            "zoom-out",
            icons::ZOOM_OUT,
            "Zoom out (-)",
            Message::zoom_out(),
            tooltip_state,
        ))
        .push(simple_icon_button(
            "rulers",
            icons::RULERS,
            "1:1 pixel ratio",
            Message::reset_to_one_to_one(),
            tooltip_state,
        ))
        .push(simple_icon_button(
            "fit",
            icons::ASPECT_RATIO,
            "Fit to view",
            Message::reset_view(),
            tooltip_state,
        ))
        .spacing(spacing::TIGHT)
}

/// Build the bottom status bar (simple status info row)
fn view_status_bar(
    text_color: Color,
    widget_state: &WidgetState,
    band_selection: &BandSelection,
    num_bands: usize,
    annotations_count: usize,
    is_drawing: bool,
    zoom: f32,
    image_dimensions: Option<(u32, u32)>,
) -> Row<'static, Message> {
    // Calculate pixel ratio for status bar
    let (widget_w, widget_h) = widget_state
        .image
        .widget_bounds
        .unwrap_or((img_const::WIDTH, img_const::HEIGHT));

    let pixel_ratio_str = if let Some((img_w, img_h)) = image_dimensions {
        let img_aspect = img_w as f32 / img_h as f32;
        let widget_aspect = widget_w / widget_h;
        let fit_scale = if img_aspect > widget_aspect {
            widget_w / img_w as f32
        } else {
            widget_h / img_h as f32
        };
        let actual_scale = fit_scale * zoom;
        let ratio = 1.0 / actual_scale;
        if ratio >= 1.0 {
            format!("1:{:.1}", ratio)
        } else {
            format!("{:.1}:1", 1.0 / ratio)
        }
    } else {
        "1:?".to_string()
    };

    // Band info for status bar
    let band_info = format!(
        "R:{} G:{} B:{}/{}",
        band_selection.red, band_selection.green, band_selection.blue, num_bands
    );

    row()
        .push(Element::new(
            text(if is_drawing { "●Drawing..." } else { "●Ready" })
                .size(text_const::SMALL)
                .color(if is_drawing { colors::ACCENT } else { colors::FPS_TEXT }),
        ))
        .push(Element::new(
            text(format!("Zoom: {:.0}%", zoom * 100.0))
                .size(text_const::SMALL)
                .color(text_color),
        ))
        .push(Element::new(
            text(pixel_ratio_str)
                .size(text_const::SMALL)
                .color(colors::MUTED_TEXT),
        ))
        .push(Element::new(
            text(band_info)
                .size(text_const::SMALL)
                .color(colors::MUTED_TEXT),
        ))
        .push(Element::new(
            text(format!("{} annotations", annotations_count))
                .size(text_const::SMALL)
                .color(colors::MUTED_TEXT),
        ))
        .spacing(spacing::MEDIUM)
}

/// Compact image settings for sidebar (vertical layout)
fn view_image_settings_compact(
    text_color: Color,
    brightness: f32,
    contrast: f32,
    gamma: f32,
    hue_shift: f32,
    widget_state: &WidgetState,
    persistence: PersistenceMode,
) -> Column<'static, Message> {
    column()
        .push(Element::new(view_compact_slider(
            "Bri",
            text_color,
            image_adjust::BRIGHTNESS_MIN,
            image_adjust::BRIGHTNESS_MAX,
            brightness,
            SliderId::Brightness,
            widget_state.slider.is_dragging(SliderId::Brightness),
            Message::set_brightness,
        )))
        .push(Element::new(view_compact_slider(
            "Con",
            text_color,
            image_adjust::CONTRAST_MIN,
            image_adjust::CONTRAST_MAX,
            contrast,
            SliderId::Contrast,
            widget_state.slider.is_dragging(SliderId::Contrast),
            Message::set_contrast,
        )))
        .push(Element::new(view_compact_slider(
            "Gam",
            text_color,
            image_adjust::GAMMA_MIN,
            image_adjust::GAMMA_MAX,
            gamma,
            SliderId::Gamma,
            widget_state.slider.is_dragging(SliderId::Gamma),
            Message::set_gamma,
        )))
        .push(Element::new(view_compact_slider(
            "Hue",
            text_color,
            image_adjust::HUE_MIN,
            image_adjust::HUE_MAX,
            hue_shift,
            SliderId::HueShift,
            widget_state.slider.is_dragging(SliderId::HueShift),
            Message::set_hue_shift,
        )))
        .push(Element::new(
            row()
                .push(Element::new(
                    text("State:").size(text_const::SMALL).color(text_color),
                ))
                .push(Element::new(view_persistence_dropdown_compact(
                    persistence,
                    widget_state.dropdown.image_settings_persistence_open,
                    false,
                )))
                .spacing(spacing::TIGHT),
        ))
        .spacing(spacing::TIGHT)
}

/// Compact slider for sidebar (label: value on same line as slider)
fn view_compact_slider<F>(
    label: &str,
    text_color: Color,
    min: f32,
    max: f32,
    value: f32,
    slider_id: SliderId,
    is_dragging: bool,
    on_change: F,
) -> Row<'static, Message>
where
    F: Fn(f32) -> Message + 'static,
{
    let value_str = if max > 100.0 {
        format!("{}: {:.0}°", label, value)
    } else {
        format!("{}: {:.2}", label, value)
    };

    row()
        .push(Element::new(
            text(value_str)
                .size(text_const::SMALL)
                .color(text_color)
                .width(sidebar_const::SLIDER_LABEL_WIDTH),
        ))
        .push(Element::new(
            slider(min, max, value)
                .id(slider_id)
                .dragging(is_dragging)
                .width(Length::Units(80.0))
                .on_drag_start(Message::slider_drag_start)
                .on_change(on_change)
                .on_drag_end(Message::slider_drag_end),
        ))
        .spacing(spacing::TIGHT)
}

/// Compact band selector for sidebar (vertical layout)
fn view_band_selector_compact(
    text_color: Color,
    widget_state: &WidgetState,
    band_selection: &BandSelection,
    num_bands: usize,
    persistence: PersistenceMode,
) -> Column<'static, Message> {
    let max_band = num_bands.saturating_sub(1) as f32;

    column()
        .push(Element::new(view_compact_band_slider(
            "R",
            colors::CHANNEL_RED,
            max_band,
            band_selection.red as f32,
            SliderId::BandRed,
            widget_state.slider.is_dragging(SliderId::BandRed),
            |_id, value| Message::start_red_band(value as usize),
            |v| Message::set_red_band(v as usize),
        )))
        .push(Element::new(view_compact_band_slider(
            "G",
            colors::CHANNEL_GREEN,
            max_band,
            band_selection.green as f32,
            SliderId::BandGreen,
            widget_state.slider.is_dragging(SliderId::BandGreen),
            |_id, value| Message::start_green_band(value as usize),
            |v| Message::set_green_band(v as usize),
        )))
        .push(Element::new(view_compact_band_slider(
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
            row()
                .push(Element::new(
                    text("State:").size(text_const::SMALL).color(text_color),
                ))
                .push(Element::new(view_persistence_dropdown_compact(
                    persistence,
                    widget_state.dropdown.band_persistence_open,
                    true,
                )))
                .spacing(spacing::TIGHT),
        ))
        .spacing(spacing::TIGHT)
}

/// Compact band slider for sidebar (label on same line as slider)
fn view_compact_band_slider<S, C>(
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
                .color(label_color)
                .width(sidebar_const::BAND_LABEL_WIDTH),
        ))
        .push(Element::new(
            slider(0.0, max_band, value)
                .id(slider_id)
                .step(1.0)
                .dragging(is_dragging)
                .width(Length::Units(100.0))
                .on_drag_start(on_drag_start)
                .on_change(on_change)
                .on_drag_end(Message::apply_bands),
        ))
        .spacing(spacing::TIGHT)
}

/// Compact persistence dropdown for sidebar.
/// Automatically drops up if not enough space below (using viewport detection).
fn view_persistence_dropdown_compact(
    current_mode: PersistenceMode,
    is_open: bool,
    is_band: bool,
) -> Dropdown<Message> {
    let options = vec![
        "Non-Persistent".to_string(),
        "Per Image".to_string(),
        "Persistent".to_string(),
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

    dropdown(options, selected_index)
        .width(95.0)
        .open(is_open)
        .on_open(on_open)
        .on_close(on_close)
        .on_select(on_select)
        .overlay(true)
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
                        .on_press(Message::set_annotation_category(category_id))
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
/// Layout: Three-column layout:
/// - Left sidebar: FILES (folder, prev/next), TOOLS (annotation tools)
/// - Center: Image with bottom adjustments panel and status bar
/// - Right sidebar: TAGS, CATEGORIES
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
    categories: Vec<&'a Category>,
    status_message: Option<&'a str>,
    band_selection: &BandSelection,
    num_bands: usize,
    overlay: Overlay,
    band_persistence: PersistenceMode,
    image_settings_persistence: PersistenceMode,
    available_tags: &'a [Tag],
    current_image_tags: &'a HashSet<u32>,
    image_dimensions: Option<(u32, u32)>,
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
        .drawing(drawing_state.is_drawing() || drawing_state.editing.is_dragging())
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
        .keyboard_disabled(widget_state.category_input.is_focused || widget_state.tag_input.is_focused)
        // Report widget bounds for pixel ratio calculation
        .on_layout(Message::report_widget_bounds);

    // === LEFT SIDEBAR ===
    let left_sidebar_content = view_left_sidebar(
        status_message,
        &widget_state.tooltip,
        drawing_state.tool,
        text_color,
        theme,
        brightness,
        contrast,
        gamma,
        hue_shift,
        widget_state,
        band_selection,
        num_bands,
        band_persistence,
        image_settings_persistence,
    );

    // Wrap left sidebar in scrollable container with border for consistent styling
    let mut left_sidebar_scrollable = scrollable(Element::new(left_sidebar_content))
        .direction(ScrollDirection::Vertical)
        .width(sidebar_const::LEFT_WIDTH + 20.0)
        .scroll_offset_y(widget_state.left_sidebar_scroll.offset_y)
        .dragging_y(widget_state.left_sidebar_scroll.is_dragging_y)
        .on_scroll_y(Message::left_sidebar_scroll_y)
        .on_drag_start_y(Message::left_sidebar_scrollbar_drag_start_y)
        .on_drag_end_y(Message::left_sidebar_scrollbar_drag_end_y);

    if let (Some(mouse_y), Some(scroll_y)) = (
        widget_state.left_sidebar_scroll.drag_start_mouse_y,
        widget_state.left_sidebar_scroll.drag_start_scroll_y,
    ) {
        left_sidebar_scrollable = left_sidebar_scrollable.drag_start_y(mouse_y, scroll_y);
    }

    let left_sidebar = container(Element::new(left_sidebar_scrollable))
        .border(colors::BORDER);

    // === CENTER PANEL: Image with header (zoom toolbar) and footer (status bar) ===
    // Build the zoom toolbar (header)
    let zoom_toolbar = view_zoom_toolbar(&widget_state.tooltip);

    // Build the status bar (footer)
    let status_bar = view_status_bar(
        text_color,
        widget_state,
        band_selection,
        num_bands,
        annotations.len(),
        drawing_state.is_drawing(),
        zoom,
        image_dimensions,
    );

    // Image panel with header (toolbar) and footer (status bar), no title
    let center_panel = titled_container("", Element::new(image_widget))
        .fill()
        .title_style(TitleStyle::None)
        .header(Element::new(zoom_toolbar))
        .footer(Element::new(status_bar))
        .padding(padding::MINIMAL)
        .border(colors::BORDER)
        .border_width(img_const::BORDER_WIDTH)
        .title_bg_color(title_const::BG_COLOR)
        .title_text_color(title_const::TEXT_COLOR);

    // === RIGHT SIDEBAR ===
    let right_sidebar = view_right_sidebar(
        categories,
        drawing_state.current_category,
        text_color,
        &widget_state.category_input.new_category_name,
        widget_state.category_input.is_focused,
        available_tags,
        current_image_tags,
        &widget_state.tag_input.new_tag_name,
        widget_state.tag_input.is_focused,
    );

    // Wrap right sidebar in scrollable container
    let mut right_sidebar_scrollable = scrollable(Element::new(right_sidebar))
        .direction(ScrollDirection::Vertical)
        .width(sidebar_const::RIGHT_WIDTH + 20.0)
        .scroll_offset_y(widget_state.sidebar_scroll.offset_y)
        .dragging_y(widget_state.sidebar_scroll.is_dragging_y)
        .on_scroll_y(Message::sidebar_scroll_y)
        .on_drag_start_y(Message::sidebar_scrollbar_drag_start_y)
        .on_drag_end_y(Message::sidebar_scrollbar_drag_end_y);

    if let (Some(mouse_y), Some(scroll_y)) = (
        widget_state.sidebar_scroll.drag_start_mouse_y,
        widget_state.sidebar_scroll.drag_start_scroll_y,
    ) {
        right_sidebar_scrollable = right_sidebar_scrollable.drag_start_y(mouse_y, scroll_y);
    }

    let right_sidebar_container = container(Element::new(right_sidebar_scrollable))
        .border(colors::BORDER);

    // === MAIN LAYOUT: left sidebar | center (image+footer) | right sidebar ===
    row()
        .push(Element::new(left_sidebar))
        .push(Element::new(center_panel))
        .push(Element::new(right_sidebar_container))
        .spacing(sidebar_const::GAP)
}

