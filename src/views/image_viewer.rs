//! Image viewer view - main image viewing and annotation interface.

use crate::annotation::{AnnotationStore, AnnotationTool, DrawingState, Shape};
use crate::hyperspectral::BandSelection;
use crate::message::{Message, PersistenceMode};
use crate::theme::Theme;
use crate::ui_constants::{
    annotation as ann_const, button as btn_const, colors, image_viewer as img_const, padding,
    slider as slider_const, spacing, text as text_const,
};
use crate::views::helpers::tool_button;
use crate::widget_state::WidgetState;
use hvat_ui::widgets::{
    button, collapsible, column, container, dropdown, hyperspectral_image, row, slider, text,
    Column, Element, Row, SliderId,
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

/// Build the annotation toolbar.
pub fn view_annotation_toolbar(tool: AnnotationTool, _text_color: Color) -> Row<'static, Message> {
    row()
        .push(tool_button("Select", AnnotationTool::Select, tool))
        .push(tool_button("BBox", AnnotationTool::BoundingBox, tool))
        .push(tool_button("Polygon", AnnotationTool::Polygon, tool))
        .push(tool_button("Point", AnnotationTool::Point, tool))
        .push(Element::new(
            button("Delete")
                .on_press(Message::delete_selected_annotation())
                .width(btn_const::COMPACT_WIDTH),
        ))
        .push(Element::new(
            button("Export")
                .on_press(Message::export_annotations())
                .width(btn_const::COMPACT_WIDTH),
        ))
        .push(Element::new(
            button("Clear")
                .on_press(Message::clear_annotations())
                .width(btn_const::XCOMPACT_WIDTH),
        ))
        .spacing(spacing::TIGHT)
}

/// Build the image viewer view.
///
/// Takes individual parameters rather than a struct to avoid lifetime issues
/// when the returned Element outlives a local struct.
///
/// Uses GPU-based band compositing - band selection changes only update a
/// uniform buffer, no CPU-side image regeneration needed.
///
/// The overlay is passed in pre-built to avoid rebuilding every frame.
/// It should be rebuilt only when annotations or drawing state changes.
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
    // Band selection (always shown) - passed to GPU for compositing
    band_selection: &BandSelection,
    num_bands: usize,
    // Pre-built overlay (cached, rebuilt only when dirty)
    overlay: Overlay,
    // Persistence modes for settings across image navigation
    band_persistence: PersistenceMode,
    image_settings_persistence: PersistenceMode,
) -> Column<'a, Message> {
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
    // Band compositing happens on the GPU - instant band selection changes!
    let image_widget = hyperspectral_image(hyperspectral_handle.clone(), band_uniform)
        .pan((pan_x, pan_y))
        .zoom(zoom)
        .dragging(widget_state.image.is_dragging)
        .drawing(drawing_state.is_drawing)
        .adjustments(adjustments)
        .overlay(overlay)
        .width(Length::Units(img_const::WIDTH))
        .height(Length::Units(img_const::HEIGHT))
        .on_drag_start(Message::image_drag_start)
        .on_drag_move(Message::image_drag_move)
        .on_drag_end(Message::image_drag_end)
        .on_zoom(Message::image_zoom_at_point)
        // Annotation callbacks
        .on_click(|(x, y)| Message::start_drawing(x, y))
        .on_draw_move(|(x, y)| Message::continue_drawing(x, y))
        .on_draw_end(Message::finish_drawing)
        .on_space(Message::force_finish_polygon);

    // Status text
    let status_text = status_message.unwrap_or("No images loaded");

    column()
        .push(Element::new(
            text("Image Viewer").size(text_const::TITLE).color(text_color),
        ))
        // File loading controls
        .push(Element::new(
            row()
                .push(Element::new(
                    button("Load Folder")
                        .on_press(Message::load_folder())
                        .width(btn_const::STANDARD_WIDTH),
                ))
                .push(Element::new(
                    button("< Prev")
                        .on_press(Message::previous_image())
                        .width(btn_const::TOOL_WIDTH),
                ))
                .push(Element::new(
                    button("Next >")
                        .on_press(Message::next_image())
                        .width(btn_const::TOOL_WIDTH),
                ))
                .push(Element::new(
                    text(status_text).size(text_const::SMALL).color(text_color),
                ))
                .spacing(spacing::STANDARD),
        ))
        .push(Element::new(
            text(format!(
                "Zoom: {:.2}x | Pan: ({:.0}, {:.0})",
                zoom, pan_x, pan_y
            ))
            .size(text_const::BODY)
            .color(text_color),
        ))
        // Image display area with border
        .push(Element::new(
            container(Element::new(image_widget))
                .padding(padding::MINIMAL)
                .border(colors::BORDER)
                .border_width(img_const::BORDER_WIDTH),
        ))
        .push(Element::new(
            text("Middle-click drag to pan, scroll to zoom")
                .size(text_const::SMALL)
                .color(colors::MUTED_TEXT),
        ))
        // Zoom/pan button controls
        .push(Element::new(
            row()
                .push(Element::new(
                    button("Zoom In")
                        .on_press(Message::zoom_in())
                        .width(btn_const::ZOOM_WIDTH),
                ))
                .push(Element::new(
                    button("Zoom Out")
                        .on_press(Message::zoom_out())
                        .width(btn_const::ZOOM_WIDTH),
                ))
                .push(Element::new(
                    button("Reset View")
                        .on_press(Message::reset_view())
                        .width(btn_const::ZOOM_WIDTH),
                ))
                .spacing(spacing::STANDARD),
        ))
        // Image Settings - collapsible (closed by default)
        .push(Element::new(
            collapsible(
                "Image Settings",
                Element::new(view_image_settings_content(
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
        // Annotation toolbar
        .push(Element::new(
            text("Annotation Tools:")
                .size(text_const::BODY)
                .color(theme.accent_color()),
        ))
        .push(Element::new(view_annotation_toolbar(
            drawing_state.tool,
            text_color,
        )))
        // Annotation info
        .push(Element::new(
            text(format!(
                "Annotations: {} | Tool: {:?} | {}",
                annotations.len(),
                drawing_state.tool,
                if drawing_state.is_drawing {
                    "Drawing..."
                } else {
                    "Ready"
                }
            ))
            .size(text_const::SMALL)
            .color(text_color),
        ))
        // Band Selection - collapsible (open by default)
        .push(Element::new(
            collapsible(
                "Band Selection (RGB Mapping)",
                Element::new(view_band_selector(
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
        // Persistence mode selectors
        .push(Element::new(
            text("Settings Persistence:")
                .size(text_const::BODY)
                .color(theme.accent_color()),
        ))
        .push(Element::new(view_band_persistence_selector(
            "Bands:",
            text_color,
            band_persistence,
            widget_state.dropdown.band_persistence_open,
        )))
        .push(Element::new(view_image_settings_persistence_selector(
            "Image:",
            text_color,
            image_settings_persistence,
            widget_state.dropdown.image_settings_persistence_open,
        )))
        .spacing(spacing::TIGHT + 3.0) // 8.0 = TIGHT(5) + 3
}

/// Build the image settings content (for inside collapsible).
fn view_image_settings_content(
    text_color: Color,
    brightness: f32,
    contrast: f32,
    gamma: f32,
    hue_shift: f32,
    widget_state: &WidgetState,
) -> Column<'static, Message> {
    column()
        // Brightness slider
        .push(Element::new(view_slider_row(
            &format!("Brightness: {:.2}", brightness),
            text_color,
            -1.0,
            1.0,
            brightness,
            SliderId::Brightness,
            widget_state.slider.is_dragging(SliderId::Brightness),
            Message::set_brightness,
        )))
        // Contrast slider
        .push(Element::new(view_slider_row(
            &format!("Contrast:   {:.2}", contrast),
            text_color,
            0.1,
            3.0,
            contrast,
            SliderId::Contrast,
            widget_state.slider.is_dragging(SliderId::Contrast),
            Message::set_contrast,
        )))
        // Gamma slider
        .push(Element::new(view_slider_row(
            &format!("Gamma:      {:.2}", gamma),
            text_color,
            0.1,
            3.0,
            gamma,
            SliderId::Gamma,
            widget_state.slider.is_dragging(SliderId::Gamma),
            Message::set_gamma,
        )))
        // Hue shift slider
        .push(Element::new(view_slider_row(
            &format!("Hue Shift:  {:.0}", hue_shift),
            text_color,
            -180.0,
            180.0,
            hue_shift,
            SliderId::HueShift,
            widget_state.slider.is_dragging(SliderId::HueShift),
            Message::set_hue_shift,
        )))
        // Reset button
        .push(Element::new(
            button("Reset Image Settings")
                .on_press(Message::reset_image_settings())
                .width(btn_const::WIDE_WIDTH),
        ))
        .spacing(spacing::TIGHT)
}

/// Helper to build an image settings slider row.
fn view_slider_row<F>(
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
    row()
        .push(Element::new(
            text(label).size(text_const::SMALL).color(text_color),
        ))
        .push(Element::new(
            slider(min, max, value)
                .id(slider_id)
                .dragging(is_dragging)
                .width(slider_const::standard_length())
                .on_drag_start(Message::slider_drag_start)
                .on_change(on_change)
                .on_drag_end(Message::slider_drag_end),
        ))
        .spacing(spacing::STANDARD)
}

/// Build the band selector UI.
/// Always visible - allows mapping any band to R/G/B channels.
fn view_band_selector(
    text_color: Color,
    widget_state: &WidgetState,
    band_selection: &BandSelection,
    num_bands: usize,
) -> Column<'static, Message> {
    let max_band = num_bands.saturating_sub(1) as f32;

    // Band sliders - always active
    // Note: on_drag_end triggers apply_bands() to regenerate the composite
    let red_slider = view_band_slider_row(
        &format!("R <- Band {}", band_selection.red),
        colors::CHANNEL_RED,
        max_band,
        band_selection.red as f32,
        SliderId::BandRed,
        widget_state.slider.is_dragging(SliderId::BandRed),
        |_id, value| Message::start_red_band(value as usize),
        |v| Message::set_red_band(v as usize),
    );

    let green_slider = view_band_slider_row(
        &format!("G <- Band {}", band_selection.green),
        colors::CHANNEL_GREEN,
        max_band,
        band_selection.green as f32,
        SliderId::BandGreen,
        widget_state.slider.is_dragging(SliderId::BandGreen),
        |_id, value| Message::start_green_band(value as usize),
        |v| Message::set_green_band(v as usize),
    );

    let blue_slider = view_band_slider_row(
        &format!("B <- Band {}", band_selection.blue),
        colors::CHANNEL_BLUE,
        max_band,
        band_selection.blue as f32,
        SliderId::BandBlue,
        widget_state.slider.is_dragging(SliderId::BandBlue),
        |_id, value| Message::start_blue_band(value as usize),
        |v| Message::set_blue_band(v as usize),
    );

    let reset_btn = button("Reset (0,1,2)")
        .on_press(Message::reset_bands())
        .width(btn_const::BAND_RESET_WIDTH);

    column()
        .push(Element::new(
            text(format!("{} channels available", num_bands))
                .size(text_const::SMALL)
                .color(text_color),
        ))
        .push(Element::new(red_slider))
        .push(Element::new(green_slider))
        .push(Element::new(blue_slider))
        .push(Element::new(reset_btn))
        .spacing(spacing::TIGHT)
}

/// Helper to build a band slider row.
fn view_band_slider_row<S, C>(
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
            text(label).size(text_const::SMALL).color(label_color),
        ))
        .push(Element::new(
            slider(0.0, max_band, value)
                .id(slider_id)
                .step(1.0)
                .dragging(is_dragging)
                .width(slider_const::compact_length())
                .on_drag_start(on_drag_start)
                .on_change(on_change)
                .on_drag_end(Message::apply_bands),
        ))
        .spacing(spacing::STANDARD)
}

/// Helper to build a persistence mode selector row with dropdown.
/// Shows a label and dropdown for selecting the mode.
fn view_band_persistence_selector(
    label: &str,
    text_color: Color,
    current_mode: PersistenceMode,
    is_open: bool,
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

    row()
        .push(Element::new(
            text(label).size(text_const::SMALL).color(text_color),
        ))
        .push(Element::new(
            dropdown(options, selected_index)
                .width(90.0)
                .open(is_open)
                .on_open(|| Message::open_band_persistence_dropdown())
                .on_close(|| Message::close_band_persistence_dropdown())
                .on_select(|idx| {
                    let mode = match idx {
                        0 => PersistenceMode::Reset,
                        1 => PersistenceMode::PerImage,
                        _ => PersistenceMode::Constant,
                    };
                    Message::set_band_persistence(mode)
                })
                .text_color(text_color)
                .overlay(true), // Render above other content
        ))
        .spacing(spacing::TIGHT)
}

/// Helper to build an image settings persistence mode selector row with dropdown.
fn view_image_settings_persistence_selector(
    label: &str,
    text_color: Color,
    current_mode: PersistenceMode,
    is_open: bool,
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

    row()
        .push(Element::new(
            text(label).size(text_const::SMALL).color(text_color),
        ))
        .push(Element::new(
            dropdown(options, selected_index)
                .width(90.0)
                .open(is_open)
                .on_open(|| Message::open_image_settings_persistence_dropdown())
                .on_close(|| Message::close_image_settings_persistence_dropdown())
                .on_select(|idx| {
                    let mode = match idx {
                        0 => PersistenceMode::Reset,
                        1 => PersistenceMode::PerImage,
                        _ => PersistenceMode::Constant,
                    };
                    Message::set_image_settings_persistence(mode)
                })
                .text_color(text_color)
                .overlay(true), // Render above other content
        ))
        .spacing(spacing::TIGHT)
}
