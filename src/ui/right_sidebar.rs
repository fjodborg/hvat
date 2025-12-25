//! Right sidebar UI component.

use std::rc::Rc;

use hvat_ui::Color;
use hvat_ui::constants::FONT_SIZE_TINY;
use hvat_ui::prelude::*;
use hvat_ui::{
    Alignment, BorderSides, Collapsible, Column, Context, Element, Padding, Panel, ScrollDirection,
    Scrollable, ScrollbarVisibility,
};

use crate::app::HvatApp;
use crate::constants::{
    ANNOTATIONS_MAX_HEIGHT, BRIGHTNESS_MAX, BRIGHTNESS_MIN, BRIGHTNESS_STEP, CONTRAST_MAX,
    CONTRAST_MIN, CONTRAST_STEP, GAMMA_MAX, GAMMA_MIN, GAMMA_STEP, HUE_MAX, HUE_MIN, HUE_STEP,
    SIDEBAR_CONTENT_WIDTH, SIDEBAR_WIDTH, THUMBNAIL_SIZE, THUMBNAIL_SPACING, THUMBNAILS_MAX_HEIGHT,
};
use crate::message::Message;
use crate::model::AnnotationShape;

impl HvatApp {
    /// Build the right sidebar with band selection and image adjustments.
    pub(crate) fn build_right_sidebar(&self) -> Element<Message> {
        // TODO(perf): These clones happen on every view rebuild. Consider using Rc<RefCell<>>
        // for widget states to avoid cloning cost. This is acceptable for now since the states
        // are small, but could become a bottleneck with many widgets.
        let band_state = self.band_selection_collapsed.clone();
        let adj_state = self.adjustments_collapsed.clone();
        let scroll_state = self.right_scroll_state.clone();

        let red_slider = self.red_band_slider.clone();
        let green_slider = self.green_band_slider.clone();
        let blue_slider = self.blue_band_slider.clone();

        let brightness_slider = self.brightness_slider.clone();
        let contrast_slider = self.contrast_slider.clone();
        let gamma_slider = self.gamma_slider.clone();
        let hue_slider = self.hue_slider.clone();

        let max_band = (self.num_bands - 1) as f32;

        // Create UndoContext for slider undo points
        let slider_undo_snapshot = self.snapshot();
        let undo_stack = Rc::clone(&self.undo_stack);
        let undo_ctx = UndoContext::new(undo_stack, slider_undo_snapshot);

        let mut sidebar_ctx = Context::new();

        // Annotations Panel Collapsible (at top for easy access)
        let annotations_state = self.annotations_collapsed.clone();
        let annotations_scroll = self.annotations_scroll_state.clone();
        let current_image_data = self.image_data_store.get(&self.current_image_path());
        let annotations = current_image_data.annotations.clone();
        let categories_for_annotations = self.categories.clone();
        let hidden_categories = self.hidden_categories.clone();

        // Count annotations by category
        let mut category_counts: std::collections::HashMap<u32, usize> =
            std::collections::HashMap::new();
        for ann in &annotations {
            *category_counts.entry(ann.category_id).or_insert(0) += 1;
        }

        // Filter annotations based on hidden categories
        let visible_annotations: Vec<_> = annotations
            .iter()
            .filter(|ann| !hidden_categories.contains(&ann.category_id))
            .collect();

        let total_count = annotations.len();

        // Minimal padding for compact buttons
        let chip_padding = Padding::new(2.0, 6.0, 2.0, 6.0);

        let collapsible_annotations = Collapsible::new("Annotations")
            .state(&annotations_state)
            .scroll_state(&annotations_scroll)
            .width(Length::Fill(1.0))
            .max_height(ANNOTATIONS_MAX_HEIGHT)
            .on_toggle(Message::AnnotationsToggled)
            .on_scroll(Message::AnnotationsScrolled)
            .content(|c| {
                if total_count == 0 {
                    c.text("No annotations").size(FONT_SIZE_SMALL);
                } else {
                    // Category filters - colored buttons
                    for cat in &categories_for_annotations {
                        let cat_id = cat.id;
                        let cat_color = cat.color;
                        let cat_name = cat.name.clone();
                        let count = category_counts.get(&cat_id).copied().unwrap_or(0);
                        let is_hidden = hidden_categories.contains(&cat_id);

                        // Format: [checkbox] Name (count)
                        // Using ASCII-compatible checkbox symbols for better font support
                        // Use two spaces in empty checkbox to better match width of [x]
                        let vis = if is_hidden { "[  ]" } else { "[x]" };
                        let label = format!("{} {} ({})", vis, cat_name, count);

                        // Convert RGB bytes to Color
                        let bg_color =
                            Color::from_rgb_bytes(cat_color[0], cat_color[1], cat_color[2]);

                        c.button(label)
                            .width(Length::Fill(1.0))
                            .padding(chip_padding)
                            .text_align(Alignment::Left)
                            .background_color(bg_color)
                            .on_click(Message::ToggleCategoryFilter(cat_id));
                    }

                    // Divider
                    c.text("---").size(FONT_SIZE_TINY).align(Alignment::Center);

                    // Annotation list - colored buttons
                    if visible_annotations.is_empty() {
                        c.text("All hidden").size(FONT_SIZE_SMALL);
                    } else {
                        for ann in &visible_annotations {
                            let ann_id = ann.id;
                            let is_selected = ann.selected;

                            // Find category
                            let cat = categories_for_annotations
                                .iter()
                                .find(|cat| cat.id == ann.category_id);
                            let cat_color = cat.map(|c| c.color).unwrap_or([128, 128, 128]);
                            let cat_name = cat.map(|c| c.name.as_str()).unwrap_or("?");

                            // Size info
                            let size_info = match &ann.shape {
                                AnnotationShape::BoundingBox { width, height, .. } => {
                                    format_area(width * height)
                                }
                                AnnotationShape::Point { .. } => "pt".to_string(),
                                AnnotationShape::Polygon { vertices } => {
                                    format!(
                                        "{}v {}",
                                        vertices.len(),
                                        format_area(polygon_area(vertices))
                                    )
                                }
                            };

                            // Format: [sel] ID Category Size
                            let sel = if is_selected { "▸" } else { " " };
                            let label = format!("{}{} {} {}", sel, ann_id, cat_name, size_info);

                            // Convert RGB bytes to Color
                            let bg_color =
                                Color::from_rgb_bytes(cat_color[0], cat_color[1], cat_color[2]);

                            c.button(label)
                                .width(Length::Fill(1.0))
                                .padding(chip_padding)
                                .text_align(Alignment::Left)
                                .background_color(bg_color)
                                .on_click(Message::SelectAnnotation(ann_id));
                        }
                    }
                }
            });
        sidebar_ctx.add(Element::new(collapsible_annotations));

        // Band Selection Collapsible
        let band_s = band_state.clone();
        let collapsible_bands = Collapsible::new("Band Selection")
            .state(&band_s)
            .width(Length::Fill(1.0))
            .on_toggle(Message::BandSelectionToggled)
            .content(|c| {
                c.text("Red Channel").size(FONT_SIZE_BODY);
                c.slider(0.0, max_band)
                    .state(&red_slider)
                    .step(1.0)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_CONTENT_WIDTH))
                    .on_change(Message::RedBandChanged)
                    .on_undo_point(undo_ctx.callback_with_label("red_band"))
                    .build();

                c.text("Green Channel").size(FONT_SIZE_BODY);
                c.slider(0.0, max_band)
                    .state(&green_slider)
                    .step(1.0)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_CONTENT_WIDTH))
                    .on_change(Message::GreenBandChanged)
                    .on_undo_point(undo_ctx.callback_with_label("green_band"))
                    .build();

                c.text("Blue Channel").size(FONT_SIZE_BODY);
                c.slider(0.0, max_band)
                    .state(&blue_slider)
                    .step(1.0)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_CONTENT_WIDTH))
                    .on_change(Message::BlueBandChanged)
                    .on_undo_point(undo_ctx.callback_with_label("blue_band"))
                    .build();
            });
        sidebar_ctx.add(Element::new(collapsible_bands));

        // Image Adjustments Collapsible
        let adj_s = adj_state.clone();
        let collapsible_adj = Collapsible::new("Image Adjustments")
            .state(&adj_s)
            .width(Length::Fill(1.0))
            .on_toggle(Message::AdjustmentsToggled)
            .content(|c| {
                c.text(format!("Brightness: {:.2}", brightness_slider.value))
                    .size(FONT_SIZE_BODY);
                c.slider(BRIGHTNESS_MIN, BRIGHTNESS_MAX)
                    .state(&brightness_slider)
                    .step(BRIGHTNESS_STEP)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_CONTENT_WIDTH))
                    .on_change(Message::BrightnessChanged)
                    .on_undo_point(undo_ctx.callback_with_label("brightness"))
                    .build();

                c.text(format!("Contrast: {:.2}", contrast_slider.value))
                    .size(FONT_SIZE_BODY);
                c.slider(CONTRAST_MIN, CONTRAST_MAX)
                    .state(&contrast_slider)
                    .step(CONTRAST_STEP)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_CONTENT_WIDTH))
                    .on_change(Message::ContrastChanged)
                    .on_undo_point(undo_ctx.callback_with_label("contrast"))
                    .build();

                c.text(format!("Gamma: {:.2}", gamma_slider.value))
                    .size(FONT_SIZE_BODY);
                c.slider(GAMMA_MIN, GAMMA_MAX)
                    .state(&gamma_slider)
                    .step(GAMMA_STEP)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_CONTENT_WIDTH))
                    .on_change(Message::GammaChanged)
                    .on_undo_point(undo_ctx.callback_with_label("gamma"))
                    .build();

                c.text(format!("Hue: {:.0}°", hue_slider.value))
                    .size(FONT_SIZE_BODY);
                c.slider(HUE_MIN, HUE_MAX)
                    .state(&hue_slider)
                    .step(HUE_STEP)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_CONTENT_WIDTH))
                    .on_change(Message::HueChanged)
                    .on_undo_point(undo_ctx.callback_with_label("hue"))
                    .build();

                c.button("Reset Adjustments")
                    .width(Length::Fixed(SIDEBAR_CONTENT_WIDTH))
                    .on_click(Message::ResetAdjustments);
            });
        sidebar_ctx.add(Element::new(collapsible_adj));

        // Thumbnails Collapsible (placeholder - actual thumbnails need texture loading)
        let thumbnails_state = self.thumbnails_collapsed.clone();
        let thumbnails_scroll = self.thumbnails_scroll_state.clone();
        let project_images = self
            .project
            .as_ref()
            .map(|p| p.images.clone())
            .unwrap_or_default();
        let current_index = self.project.as_ref().map(|p| p.current_index).unwrap_or(0);

        let collapsible_thumbs = Collapsible::new("Thumbnails")
            .state(&thumbnails_state)
            .scroll_state(&thumbnails_scroll)
            .width(Length::Fill(1.0))
            .max_height(THUMBNAILS_MAX_HEIGHT)
            .on_toggle(Message::ThumbnailsToggled)
            .on_scroll(Message::ThumbnailsScrolled)
            .content(|c| {
                if project_images.is_empty() {
                    c.text("No thumbnails available").size(FONT_SIZE_SECONDARY);
                } else {
                    c.text(format!(
                        "{} images ({}x{} thumbnails)",
                        project_images.len(),
                        THUMBNAIL_SIZE as u32,
                        THUMBNAIL_SIZE as u32
                    ))
                    .size(FONT_SIZE_SECONDARY);

                    // Display as a grid of placeholder buttons
                    // Calculate how many fit per row (account for padding/margins)
                    let thumbs_per_row = ((SIDEBAR_CONTENT_WIDTH - 20.0)
                        / (THUMBNAIL_SIZE + THUMBNAIL_SPACING))
                        as usize;
                    let thumbs_per_row = thumbs_per_row.max(1);

                    // Group images into rows
                    for chunk_start in (0..project_images.len()).step_by(thumbs_per_row) {
                        let chunk_end = (chunk_start + thumbs_per_row).min(project_images.len());

                        c.row(|r| {
                            for idx in chunk_start..chunk_end {
                                let is_current = idx == current_index;

                                // Placeholder: show index as text in a small square button
                                // TODO: Replace with actual image thumbnails when GPU texture loading is implemented
                                // Using ASCII for cross-platform compatibility
                                let label = if is_current {
                                    format!(">{}", idx + 1)
                                } else {
                                    format!("{}", idx + 1)
                                };

                                r.button(label)
                                    .width(Length::Fixed(THUMBNAIL_SIZE))
                                    .height(Length::Fixed(THUMBNAIL_SIZE))
                                    .style(ButtonStyle::Text)
                                    .on_click(Message::ThumbnailSelect(idx));
                            }
                        });
                    }
                }
            });
        sidebar_ctx.add(Element::new(collapsible_thumbs));

        // Quick Reference - "Tips" title left-aligned, content centered
        sidebar_ctx.text("Tips").size(FONT_SIZE_TITLE);
        sidebar_ctx
            .text("Quick Reference")
            .size(FONT_SIZE_SUBSECTION)
            .align(Alignment::Center);

        sidebar_ctx
            .text("Keyboard Shortcuts")
            .size(FONT_SIZE_SECONDARY)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Ctrl+Z/Y - Undo/Redo")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("0 - Zoom 100%, F - Fit")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("+/- - Zoom in/out")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Left/Right - Prev/Next image")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Del - Delete annotation")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);

        sidebar_ctx.text("").size(FONT_SIZE_SMALL);
        sidebar_ctx
            .text("Navigation")
            .size(FONT_SIZE_SECONDARY)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Pan: Click+drag on image")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Zoom: Scroll wheel or +/-")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Images: Arrow keys or topbar")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);

        sidebar_ctx.text("").size(FONT_SIZE_SMALL);
        sidebar_ctx
            .text("Categories & Colors")
            .size(FONT_SIZE_SECONDARY)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Click swatch to pick color")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("1-0 keys select categories")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);

        sidebar_ctx.text("").size(FONT_SIZE_SMALL);
        sidebar_ctx
            .text("Band Selection")
            .size(FONT_SIZE_SECONDARY)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Drag sliders to change bands")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Type values in input fields")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);

        sidebar_ctx.text("").size(FONT_SIZE_SMALL);
        sidebar_ctx
            .text("Polygon Editing")
            .size(FONT_SIZE_SECONDARY)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Select polygon, then:")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Click edge: add & drag vertex")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Right-click vertex: remove")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);

        sidebar_ctx.text("").size(FONT_SIZE_SMALL);
        sidebar_ctx
            .text("Settings (gear icon)")
            .size(FONT_SIZE_SECONDARY)
            .align(Alignment::Center);
        sidebar_ctx
            .text("GPU preload: cache images")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Export/Import config files")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);
        sidebar_ctx
            .text("Customize keybindings")
            .size(FONT_SIZE_SMALL)
            .align(Alignment::Center);

        // Wrap in scrollable
        let content = Element::new(Column::new(sidebar_ctx.take()));
        let scrollable = Scrollable::new(content)
            .state(&scroll_state)
            .direction(ScrollDirection::Vertical)
            .scrollbar_visibility(ScrollbarVisibility::Auto)
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill(1.0))
            .on_scroll(Message::RightScrolled);

        // Wrap in panel with left and top borders
        let panel = Panel::new(Element::new(scrollable))
            .borders(BorderSides::left_top())
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill(1.0));

        Element::new(panel)
    }
}

/// Calculate the area of a polygon using the shoelace formula.
fn polygon_area(vertices: &[(f32, f32)]) -> f32 {
    if vertices.len() < 3 {
        return 0.0;
    }

    let mut area = 0.0;
    let n = vertices.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += vertices[i].0 * vertices[j].1;
        area -= vertices[j].0 * vertices[i].1;
    }
    (area / 2.0).abs()
}

/// Format area in a compact, human-readable format.
/// Uses K for thousands, M for millions.
fn format_area(area: f32) -> String {
    if area >= 1_000_000.0 {
        format!("{:.1}M", area / 1_000_000.0)
    } else if area >= 1_000.0 {
        format!("{:.1}K", area / 1_000.0)
    } else {
        format!("{:.0}", area)
    }
}
