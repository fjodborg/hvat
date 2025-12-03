//! Image viewer view - main image viewing and annotation interface.

use crate::annotation::{AnnotationStore, AnnotationTool, DrawingState, Shape};
use crate::hyperspectral::BandSelection;
use crate::message::Message;
use crate::theme::Theme;
use crate::widget_state::WidgetState;
use hvat_ui::widgets::{
    button, column, container, pan_zoom_image, row, slider, text, Column, Element, Row, SliderId,
};
use hvat_ui::{Color, ImageAdjustments, ImageHandle, Length, Overlay, OverlayItem, OverlayShape};

/// Build an overlay from annotations and drawing state.
pub fn build_overlay(annotations: &AnnotationStore, drawing_state: &DrawingState) -> Overlay {
    let mut overlay = Overlay::new();

    // Add all annotations
    for ann in annotations.iter() {
        // Get category color
        let cat_color = annotations
            .get_category(ann.category_id)
            .map(|c| Color::new(c.color[0], c.color[1], c.color[2], c.color[3]))
            .unwrap_or(Color::rgb(0.7, 0.7, 0.7));

        let shape = match &ann.shape {
            Shape::Point(p) => OverlayShape::Point {
                x: p.x,
                y: p.y,
                radius: 6.0,
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
            .map(|c| Color::new(c.color[0], c.color[1], c.color[2], 0.5))
            .unwrap_or(Color::new(0.7, 0.7, 0.7, 0.5));

        let shape = match preview_shape {
            Shape::Point(p) => OverlayShape::Point {
                x: p.x,
                y: p.y,
                radius: 6.0,
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
    // Tool selection buttons with visual indication of active tool
    let select_btn = if tool == AnnotationTool::Select {
        button("Select *")
            .on_press(Message::set_annotation_tool(AnnotationTool::Select))
            .width(80.0)
    } else {
        button("Select")
            .on_press(Message::set_annotation_tool(AnnotationTool::Select))
            .width(80.0)
    };

    let bbox_btn = if tool == AnnotationTool::BoundingBox {
        button("BBox *")
            .on_press(Message::set_annotation_tool(AnnotationTool::BoundingBox))
            .width(80.0)
    } else {
        button("BBox")
            .on_press(Message::set_annotation_tool(AnnotationTool::BoundingBox))
            .width(80.0)
    };

    let poly_btn = if tool == AnnotationTool::Polygon {
        button("Polygon *")
            .on_press(Message::set_annotation_tool(AnnotationTool::Polygon))
            .width(80.0)
    } else {
        button("Polygon")
            .on_press(Message::set_annotation_tool(AnnotationTool::Polygon))
            .width(80.0)
    };

    let point_btn = if tool == AnnotationTool::Point {
        button("Point *")
            .on_press(Message::set_annotation_tool(AnnotationTool::Point))
            .width(80.0)
    } else {
        button("Point")
            .on_press(Message::set_annotation_tool(AnnotationTool::Point))
            .width(80.0)
    };

    row()
        .push(Element::new(select_btn))
        .push(Element::new(bbox_btn))
        .push(Element::new(poly_btn))
        .push(Element::new(point_btn))
        .push(Element::new(
            button("Delete")
                .on_press(Message::delete_selected_annotation())
                .width(70.0),
        ))
        .push(Element::new(
            button("Export")
                .on_press(Message::export_annotations())
                .width(70.0),
        ))
        .push(Element::new(
            button("Clear")
                .on_press(Message::clear_annotations())
                .width(60.0),
        ))
        .spacing(5.0)
}

/// Build the image viewer view.
///
/// Takes individual parameters rather than a struct to avoid lifetime issues
/// when the returned Element outlives a local struct.
#[allow(clippy::too_many_arguments)]
pub fn view_image_viewer<'a>(
    theme: &Theme,
    text_color: Color,
    current_image: &'a ImageHandle,
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
    // Band selection (always shown)
    band_selection: &BandSelection,
    num_bands: usize,
) -> Column<'a, Message> {
    // Create image adjustments from current settings
    let adjustments = ImageAdjustments {
        brightness,
        contrast,
        gamma,
        hue_shift,
    };

    // Build the annotation overlay
    let overlay = build_overlay(annotations, drawing_state);

    // Create the pan/zoom image widget
    let image_widget = pan_zoom_image(current_image.clone())
        .pan((pan_x, pan_y))
        .zoom(zoom)
        .dragging(widget_state.image.is_dragging)
        .drawing(drawing_state.is_drawing)
        .adjustments(adjustments)
        .overlay(overlay)
        .width(Length::Units(600.0))
        .height(Length::Units(400.0))
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
            text("Image Viewer").size(24.0).color(text_color),
        ))
        // File loading controls
        .push(Element::new(
            row()
                .push(Element::new(
                    button("Load Folder")
                        .on_press(Message::load_folder())
                        .width(120.0),
                ))
                .push(Element::new(
                    button("< Prev")
                        .on_press(Message::previous_image())
                        .width(80.0),
                ))
                .push(Element::new(
                    button("Next >")
                        .on_press(Message::next_image())
                        .width(80.0),
                ))
                .push(Element::new(
                    text(status_text).size(12.0).color(text_color),
                ))
                .spacing(10.0),
        ))
        .push(Element::new(
            text(format!(
                "Zoom: {:.2}x | Pan: ({:.0}, {:.0})",
                zoom, pan_x, pan_y
            ))
            .size(14.0)
            .color(text_color),
        ))
        // Image display area with border
        .push(Element::new(
            container(Element::new(image_widget))
                .padding(4.0)
                .border(Color::rgb(0.4, 0.4, 0.4))
                .border_width(2.0),
        ))
        .push(Element::new(
            text("Middle-click drag to pan, scroll to zoom")
                .size(12.0)
                .color(Color::rgb(0.6, 0.6, 0.6)),
        ))
        // Zoom/pan button controls
        .push(Element::new(
            row()
                .push(Element::new(
                    button("Zoom In").on_press(Message::zoom_in()).width(90.0),
                ))
                .push(Element::new(
                    button("Zoom Out").on_press(Message::zoom_out()).width(90.0),
                ))
                .push(Element::new(
                    button("Reset View")
                        .on_press(Message::reset_view())
                        .width(90.0),
                ))
                .spacing(10.0),
        ))
        // Image manipulation controls with sliders
        .push(Element::new(
            text("Image Settings:")
                .size(14.0)
                .color(theme.accent_color()),
        ))
        // Brightness slider
        .push(Element::new(
            row()
                .push(Element::new(
                    text(format!("Brightness: {:.2}", brightness))
                        .size(12.0)
                        .color(text_color),
                ))
                .push(Element::new(
                    slider(-1.0, 1.0, brightness)
                        .id(SliderId::Brightness)
                        .dragging(widget_state.slider.is_dragging(SliderId::Brightness))
                        .width(Length::Units(200.0))
                        .on_drag_start(Message::slider_drag_start)
                        .on_change(Message::set_brightness)
                        .on_drag_end(Message::slider_drag_end),
                ))
                .spacing(10.0),
        ))
        // Contrast slider
        .push(Element::new(
            row()
                .push(Element::new(
                    text(format!("Contrast:   {:.2}", contrast))
                        .size(12.0)
                        .color(text_color),
                ))
                .push(Element::new(
                    slider(0.1, 3.0, contrast)
                        .id(SliderId::Contrast)
                        .dragging(widget_state.slider.is_dragging(SliderId::Contrast))
                        .width(Length::Units(200.0))
                        .on_drag_start(Message::slider_drag_start)
                        .on_change(Message::set_contrast)
                        .on_drag_end(Message::slider_drag_end),
                ))
                .spacing(10.0),
        ))
        // Gamma slider
        .push(Element::new(
            row()
                .push(Element::new(
                    text(format!("Gamma:      {:.2}", gamma))
                        .size(12.0)
                        .color(text_color),
                ))
                .push(Element::new(
                    slider(0.1, 3.0, gamma)
                        .id(SliderId::Gamma)
                        .dragging(widget_state.slider.is_dragging(SliderId::Gamma))
                        .width(Length::Units(200.0))
                        .on_drag_start(Message::slider_drag_start)
                        .on_change(Message::set_gamma)
                        .on_drag_end(Message::slider_drag_end),
                ))
                .spacing(10.0),
        ))
        // Hue shift slider
        .push(Element::new(
            row()
                .push(Element::new(
                    text(format!("Hue Shift:  {:.0}", hue_shift))
                        .size(12.0)
                        .color(text_color),
                ))
                .push(Element::new(
                    slider(-180.0, 180.0, hue_shift)
                        .id(SliderId::HueShift)
                        .dragging(widget_state.slider.is_dragging(SliderId::HueShift))
                        .width(Length::Units(200.0))
                        .on_drag_start(Message::slider_drag_start)
                        .on_change(Message::set_hue_shift)
                        .on_drag_end(Message::slider_drag_end),
                ))
                .spacing(10.0),
        ))
        // Reset button
        .push(Element::new(
            button("Reset Image Settings")
                .on_press(Message::reset_image_settings())
                .width(180.0),
        ))
        // Annotation toolbar
        .push(Element::new(
            text("Annotation Tools:")
                .size(14.0)
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
            .size(12.0)
            .color(text_color),
        ))
        // Band selection (always visible)
        .push(Element::new(
            text("Band Selection (RGB Mapping):")
                .size(14.0)
                .color(theme.accent_color()),
        ))
        .push(Element::new(view_band_selector(
            text_color,
            widget_state,
            band_selection,
            num_bands,
        )))
        .spacing(8.0)
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
    let red_slider = row()
        .push(Element::new(
            text(format!("R <- Band {}", band_selection.red))
                .size(12.0)
                .color(Color::rgb(1.0, 0.3, 0.3)),
        ))
        .push(Element::new(
            slider(0.0, max_band, band_selection.red as f32)
                .id(SliderId::BandRed)
                .step(1.0)
                .dragging(widget_state.slider.is_dragging(SliderId::BandRed))
                .width(Length::Units(150.0))
                .on_drag_start(|_id, value| Message::start_red_band(value as usize))
                .on_change(|v| Message::set_red_band(v as usize))
                .on_drag_end(Message::apply_bands),
        ))
        .spacing(10.0);

    let green_slider = row()
        .push(Element::new(
            text(format!("G <- Band {}", band_selection.green))
                .size(12.0)
                .color(Color::rgb(0.3, 1.0, 0.3)),
        ))
        .push(Element::new(
            slider(0.0, max_band, band_selection.green as f32)
                .id(SliderId::BandGreen)
                .step(1.0)
                .dragging(widget_state.slider.is_dragging(SliderId::BandGreen))
                .width(Length::Units(150.0))
                .on_drag_start(|_id, value| Message::start_green_band(value as usize))
                .on_change(|v| Message::set_green_band(v as usize))
                .on_drag_end(Message::apply_bands),
        ))
        .spacing(10.0);

    let blue_slider = row()
        .push(Element::new(
            text(format!("B <- Band {}", band_selection.blue))
                .size(12.0)
                .color(Color::rgb(0.3, 0.3, 1.0)),
        ))
        .push(Element::new(
            slider(0.0, max_band, band_selection.blue as f32)
                .id(SliderId::BandBlue)
                .step(1.0)
                .dragging(widget_state.slider.is_dragging(SliderId::BandBlue))
                .width(Length::Units(150.0))
                .on_drag_start(|_id, value| Message::start_blue_band(value as usize))
                .on_change(|v| Message::set_blue_band(v as usize))
                .on_drag_end(Message::apply_bands),
        ))
        .spacing(10.0);

    let reset_btn = button("Reset (0,1,2)")
        .on_press(Message::reset_bands())
        .width(110.0);

    column()
        .push(Element::new(
            text(format!("{} channels available", num_bands))
                .size(12.0)
                .color(text_color),
        ))
        .push(Element::new(red_slider))
        .push(Element::new(green_slider))
        .push(Element::new(blue_slider))
        .push(Element::new(reset_btn))
        .spacing(5.0)
}
