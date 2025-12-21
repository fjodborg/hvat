//! Left sidebar UI component.

use hvat_ui::constants::{BUTTON_PADDING_COMPACT, COLOR_PICKER_SWATCH_OFFSET, ROW_ITEM_HEIGHT};
use hvat_ui::prelude::*;
use hvat_ui::{
    BorderSides, Collapsible, ColorPicker, ColorSwatch, Column, Context, Element, Panel, Scrollable,
    ScrollDirection, ScrollbarVisibility,
};

use crate::app::HvatApp;
use crate::constants::SIDEBAR_WIDTH;
use crate::message::Message;
use crate::model::AnnotationTool;

impl HvatApp {
    /// Build the left sidebar with tools, categories, and tags.
    pub(crate) fn build_left_sidebar(&self) -> Element<Message> {
        // TODO(perf): These clones happen on every view rebuild. Consider using Rc<RefCell<>>
        // for widget states and Vec types to avoid cloning cost. The `categories` clone is
        // particularly expensive as it clones the entire category list.
        let tools_state = self.tools_collapsed.clone();
        let categories_state = self.categories_collapsed.clone();
        let tags_state = self.tags_collapsed.clone();
        let scroll_state = self.left_scroll_state.clone();
        let selected_tool = self.selected_tool;
        let categories = self.categories.clone();
        let selected_category = self.selected_category;
        let editing_category = self.editing_category;
        let category_name_input = self.category_name_input.clone();
        let category_name_input_state = self.category_name_input_state;
        let color_picker_category = self.color_picker_category;
        let color_picker_state = self.color_picker_state;
        // Global tags (persist across all images)
        let global_tags = self.global_tags.clone();
        // Per-image: which global tags are selected for the current image
        let current_image_data = self.image_data_store.get(&self.current_image_path());
        let selected_tags = current_image_data.selected_tags;
        let tag_input_text = self.tag_input_text.clone();
        let tag_input_state = self.tag_input_state.clone();

        let mut sidebar_ctx = Context::new();

        // Tools Collapsible
        let tools_s = tools_state.clone();
        let collapsible_tools = Collapsible::new("Annotation Tools")
            .state(&tools_s)
            .width(Length::Fill(1.0))
            .on_toggle(Message::ToolsToggled)
            .content(|c| {
                c.text(format!("Current: {}", selected_tool.name())).size(FONT_SIZE_SECONDARY);
                c.text("");
                for tool in AnnotationTool::all() {
                    let is_selected = *tool == selected_tool;
                    let tool_copy = *tool;
                    let label = if is_selected {
                        format!("> {} <", tool.name())
                    } else {
                        tool.name().to_string()
                    };
                    c.button(label)
                        .width(Length::Fill(1.0))
                        .on_click(Message::ToolSelected(tool_copy));
                }
            });
        sidebar_ctx.add(Element::new(collapsible_tools));

        // Categories Collapsible
        let cats_s = categories_state.clone();
        let collapsible_cats = Collapsible::new("Categories")
            .state(&cats_s)
            .width(Length::Fill(1.0))
            .on_toggle(Message::CategoriesToggled)
            .content(|c| {
                for cat in &categories {
                    let is_selected = cat.id == selected_category;
                    let is_editing = editing_category == Some(cat.id);
                    let cat_id = cat.id;
                    let cat_color = cat.color;
                    let cat_name = cat.name.clone();

                    c.row(|r| {
                        // Color swatch (clickable to toggle color picker)
                        // Use consistent height with other row items
                        let swatch = ColorSwatch::new(cat_color)
                            .width(Length::Fixed(20.0))
                            .height(Length::Fixed(ROW_ITEM_HEIGHT))
                            .on_click(Message::ToggleCategoryColorPicker(cat_id));
                        r.add(Element::new(swatch));

                        if is_editing {
                            // Show text input for editing (use Fill to match button width)
                            r.text_input()
                                .value(&category_name_input)
                                .state(&category_name_input_state)
                                .placeholder("Category name...")
                                .width(Length::Fill(1.0))
                                .on_change(Message::CategoryNameChanged)
                                .on_submit(|_| Message::FinishEditingCategory)
                                .build();
                            // Show checkmark to confirm (Enter also works)
                            r.button("✓")
                                .padding(BUTTON_PADDING_COMPACT)
                                .on_click(Message::FinishEditingCategory);
                        } else {
                            // Show category name as button with compact padding for consistent height
                            let label = if is_selected {
                                format!("● {}", cat_name)
                            } else {
                                format!("○ {}", cat_name)
                            };
                            r.button(label)
                                .width(Length::Fill(1.0))
                                .padding(BUTTON_PADDING_COMPACT)
                                .on_click(Message::CategorySelected(cat_id));
                            // Edit button (pen icon)
                            r.button("✎")
                                .padding(BUTTON_PADDING_COMPACT)
                                .on_click(Message::StartEditingCategory(cat_id));
                        }
                    });

                    // Show color picker if open for this category (opens below the swatch)
                    if color_picker_category == Some(cat_id) {
                        // Position picker below the color swatch, aligned with its left edge
                        let picker = ColorPicker::new()
                            .selected(cat_color)
                            .open(true)
                            .x_offset(COLOR_PICKER_SWATCH_OFFSET)
                            .state(&color_picker_state)
                            .on_change(Message::CategoryColorLiveUpdate)  // Live updates from sliders
                            .on_select(Message::CategoryColorApply)       // Palette click applies and closes
                            .on_close(Message::CloseCategoryColorPicker)
                            .on_state_change(Message::ColorPickerStateChanged);  // For drag state tracking
                        c.add(Element::new(picker));
                    }
                }
                c.text("");
                c.button("+ Add Category")
                    .width(Length::Fill(1.0))
                    .on_click(Message::AddCategory);
            });
        sidebar_ctx.add(Element::new(collapsible_cats));

        // Image Tags Collapsible
        let tags_s = tags_state.clone();
        let collapsible_tags = Collapsible::new("Image Tags")
            .state(&tags_s)
            .width(Length::Fill(1.0))
            .on_toggle(Message::TagsToggled)
            .content(|c| {
                for tag in &global_tags {
                    let tag_clone = tag.clone();
                    let tag_for_toggle = tag.clone();
                    let tag_for_remove = tag.clone();
                    let is_selected = selected_tags.contains(tag);

                    c.row(|r| {
                        // Tag name as selectable button (like categories)
                        let label = if is_selected {
                            format!("● {}", tag_clone)
                        } else {
                            format!("○ {}", tag_clone)
                        };
                        r.button(label)
                            .width(Length::Fill(1.0))
                            .padding(BUTTON_PADDING_COMPACT)
                            .on_click(Message::ToggleTag(tag_for_toggle));
                        // Remove button
                        r.button("×")
                            .padding(BUTTON_PADDING_COMPACT)
                            .on_click(Message::RemoveTag(tag_for_remove));
                    });
                }
                c.text("");
                c.row(|r| {
                    r.text_input()
                        .value(&tag_input_text)
                        .state(&tag_input_state)
                        .placeholder("Add tag...")
                        .width(Length::Fill(1.0))
                        .on_change(Message::TagInputChanged)
                        .on_submit(|_| Message::AddTag)
                        .build();
                    r.button("✓")
                        .padding(BUTTON_PADDING_COMPACT)
                        .on_click(Message::AddTag);
                });
                // Add padding at the bottom so text input isn't cut off when scrolled
                c.text("");
            });
        sidebar_ctx.add(Element::new(collapsible_tags));

        // Add extra padding at the bottom of sidebar for scroll space
        sidebar_ctx.text("");
        sidebar_ctx.text("");

        // Wrap in scrollable
        let content = Element::new(Column::new(sidebar_ctx.take()));
        let scrollable = Scrollable::new(content)
            .state(&scroll_state)
            .direction(ScrollDirection::Vertical)
            .scrollbar_visibility(ScrollbarVisibility::Auto)
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill(1.0))
            .on_scroll(Message::LeftScrolled);

        // Wrap in panel with right and top borders
        let panel = Panel::new(Element::new(scrollable))
            .borders(BorderSides::right_top())
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill(1.0));

        Element::new(panel)
    }
}
