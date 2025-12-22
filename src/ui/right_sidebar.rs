//! Right sidebar UI component.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use hvat_ui::constants::BUTTON_PADDING_COMPACT;
use hvat_ui::prelude::*;
use hvat_ui::{
    BorderSides, Collapsible, Column, Context, Element, Panel, ScrollDirection, Scrollable,
    ScrollbarVisibility,
};

use crate::app::HvatApp;
use crate::constants::{
    BRIGHTNESS_MAX, BRIGHTNESS_MIN, BRIGHTNESS_STEP, CONTRAST_MAX, CONTRAST_MIN, CONTRAST_STEP,
    FILE_LIST_MAX_HEIGHT, GAMMA_MAX, GAMMA_MIN, GAMMA_STEP, HUE_MAX, HUE_MIN, HUE_STEP,
    SIDEBAR_CONTENT_WIDTH, SIDEBAR_WIDTH, THUMBNAIL_SIZE, THUMBNAIL_SPACING, THUMBNAILS_MAX_HEIGHT,
};
use crate::message::Message;

/// Represents a file entry with its original index in the project
struct FileEntry {
    filename: String,
    index: usize,
}

/// Groups files by their parent folder relative to the project root
fn group_files_by_folder(
    images: &[PathBuf],
    project_folder: &Path,
) -> BTreeMap<String, Vec<FileEntry>> {
    let mut groups: BTreeMap<String, Vec<FileEntry>> = BTreeMap::new();

    for (idx, path) in images.iter().enumerate() {
        // Get relative path from project folder
        let relative = if !project_folder.as_os_str().is_empty() {
            path.strip_prefix(project_folder)
                .ok()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| path.clone())
        } else {
            path.clone()
        };

        // Extract parent folder and filename
        let (folder, filename) = if let Some(parent) = relative.parent() {
            let folder_str = parent.to_str().unwrap_or("").to_string();
            let filename = relative
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
            (folder_str, filename)
        } else {
            // No parent - file is at root
            let filename = relative
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
            (String::new(), filename)
        };

        groups.entry(folder).or_default().push(FileEntry {
            filename,
            index: idx,
        });
    }

    groups
}

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

        sidebar_ctx
            .text("Image Controls")
            .size(FONT_SIZE_SUBSECTION);

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

        // File List Collapsible
        let file_list_state = self.file_list_collapsed.clone();
        let project_images = self
            .project
            .as_ref()
            .map(|p| p.images.clone())
            .unwrap_or_default();
        let current_index = self.project.as_ref().map(|p| p.current_index).unwrap_or(0);
        let project_folder = self
            .project
            .as_ref()
            .map(|p| p.folder.clone())
            .unwrap_or_default();

        // Group files by folder for hierarchical display
        let file_groups = group_files_by_folder(&project_images, &project_folder);

        let file_list_scroll = self.file_list_scroll_state.clone();
        let collapsible_files = Collapsible::new("File List")
            .state(&file_list_state)
            .scroll_state(&file_list_scroll)
            .scroll_direction(ScrollDirection::Both)
            .width(Length::Fill(1.0))
            .max_height(FILE_LIST_MAX_HEIGHT)
            .on_toggle(Message::FileListToggled)
            .on_scroll(Message::FileListScrolled)
            .content(|c| {
                if project_images.is_empty() {
                    c.text("No files loaded").size(FONT_SIZE_SECONDARY);
                    c.text("Use 'Open Folder' to load images")
                        .size(FONT_SIZE_SMALL);
                } else {
                    c.text(format!("{} files loaded", project_images.len()))
                        .size(FONT_SIZE_SECONDARY);
                    c.text("");

                    // Display files grouped by folder
                    for (folder, files) in &file_groups {
                        // Show folder header (if not root)
                        if !folder.is_empty() {
                            c.text(format!("📁 {}/", folder)).size(FONT_SIZE_SECONDARY);
                        }

                        // Show files in this folder
                        for entry in files {
                            let is_current = entry.index == current_index;

                            // Indent files under folders
                            let prefix = if folder.is_empty() { "" } else { "  " };

                            // Format the label with selection indicator
                            let label = if is_current {
                                format!("{}▸ {}", prefix, entry.filename)
                            } else {
                                format!("{}  {}", prefix, entry.filename)
                            };

                            c.button(label)
                                .width(Length::Fill(1.0))
                                .padding(BUTTON_PADDING_COMPACT)
                                .style(ButtonStyle::Text)
                                .on_click(Message::FileListSelect(entry.index));
                        }
                    }
                }
            });
        sidebar_ctx.add(Element::new(collapsible_files));

        // Thumbnails Collapsible (placeholder - actual thumbnails need texture loading)
        let thumbnails_state = self.thumbnails_collapsed.clone();
        let thumbnails_scroll = self.thumbnails_scroll_state.clone();
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
                    c.text("");

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
                                let label = if is_current {
                                    format!("▸{}", idx + 1)
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

        // Keyboard shortcuts info
        sidebar_ctx.text("");
        sidebar_ctx.text("────────────────────");
        sidebar_ctx
            .text("Keyboard shortcuts:")
            .size(FONT_SIZE_SECONDARY);
        sidebar_ctx.text("Ctrl+Z - Undo").size(FONT_SIZE_SMALL);
        sidebar_ctx.text("Ctrl+Y - Redo").size(FONT_SIZE_SMALL);
        sidebar_ctx.text("0 - Zoom to 100%").size(FONT_SIZE_SMALL);
        sidebar_ctx.text("F - Fit to window").size(FONT_SIZE_SMALL);
        sidebar_ctx.text("+/- - Zoom in/out").size(FONT_SIZE_SMALL);

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
