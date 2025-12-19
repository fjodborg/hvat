//! Left sidebar UI component.

use hvat_ui::prelude::*;
use hvat_ui::{Collapsible, Column, Context, Element, Scrollable, ScrollDirection, ScrollbarVisibility};

use crate::app::HvatApp;
use crate::constants::SIDEBAR_WIDTH;
use crate::message::Message;
use crate::model::AnnotationTool;

impl HvatApp {
    /// Build the left sidebar with tools, categories, and tags.
    pub(crate) fn build_left_sidebar(&self) -> Element<Message> {
        let tools_state = self.tools_collapsed.clone();
        let categories_state = self.categories_collapsed.clone();
        let tags_state = self.tags_collapsed.clone();
        let scroll_state = self.left_scroll_state.clone();
        let selected_tool = self.selected_tool;
        let categories = self.categories.clone();
        let selected_category = self.selected_category;
        let image_tags = self.image_tags.clone();
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
                c.text_sized(format!("Current: {}", selected_tool.name()), 11.0);
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
                    let cat_id = cat.id;
                    let label = if is_selected {
                        format!("● {}", cat.name)
                    } else {
                        format!("○ {}", cat.name)
                    };
                    c.button(label)
                        .width(Length::Fill(1.0))
                        .on_click(Message::CategorySelected(cat_id));
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
                for tag in &image_tags {
                    let tag_clone = tag.clone();
                    c.row(|r| {
                        r.text_sized(format!("[{}]", tag_clone), 11.0);
                        r.button("×")
                            .width(Length::Fixed(25.0))
                            .on_click(Message::RemoveTag(tag_clone));
                    });
                }
                c.text("");
                c.text_input()
                    .value(&tag_input_text)
                    .state(&tag_input_state)
                    .placeholder("Add tag...")
                    .width(Length::Fixed(SIDEBAR_WIDTH - 30.0))
                    .on_change(Message::TagInputChanged)
                    .on_submit(|_| Message::AddTag)
                    .build();
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

        Element::new(scrollable)
    }
}
