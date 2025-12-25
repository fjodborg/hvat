//! Left sidebar UI component.

use hvat_ui::constants::{BUTTON_PADDING_COMPACT, COLOR_PICKER_SWATCH_OFFSET, ROW_ITEM_HEIGHT};
use hvat_ui::prelude::*;
use hvat_ui::theme::current_theme;
use hvat_ui::{
    BorderSides, Collapsible, ColorPicker, ColorSwatch, Column, Context, Element, FileTree, Panel,
    ScrollDirection, Scrollable, ScrollbarVisibility, TooltipContent,
};

use crate::app::HvatApp;
use crate::constants::{FILE_LIST_MAX_HEIGHT, SIDEBAR_WIDTH};
use crate::keybindings::{key_to_string, optional_key_to_string};
use crate::message::Message;
use crate::model::AnnotationTool;

/// Fixed width for Edit/OK button to prevent layout flicker
const ACTION_BUTTON_WIDTH: f32 = 40.0;

/// Build a labeled item row with color swatch, name button, edit, and delete buttons.
/// Used for both categories and tags to ensure consistent layout.
///
/// Parameters:
/// - `r`: The row builder
/// - `hotkey_str`: Hotkey indicator text (e.g., "1", "2")
/// - `show_hotkey`: Whether to show the hotkey indicator
/// - `color`: RGB color for the swatch
/// - `filled`: Whether the swatch should be filled (true) or outline only (false)
/// - `is_selected`: Whether this item is selected (adds * prefix to name)
/// - `is_editing`: Whether we're in edit mode for this item
/// - `name`: The display name
/// - `name_input`: Current text in the name input field (when editing)
/// - `name_input_state`: State of the name input field
/// - `on_swatch_click`: Message to send when swatch is clicked (None for no action)
/// - `on_name_change`: Message constructor for name input changes
/// - `on_name_submit`: Message to send when name editing is submitted
/// - `on_name_click`: Message to send when name button is clicked
/// - `on_edit_click`: Message to send when edit button is clicked
/// - `on_delete_click`: Message to send when delete button is clicked
#[allow(clippy::too_many_arguments)]
fn build_item_row(
    r: &mut Context<Message>,
    hotkey_str: &str,
    show_hotkey: bool,
    color: [u8; 3],
    filled: bool,
    is_selected: bool,
    is_editing: bool,
    name: &str,
    name_input: &str,
    name_input_state: &TextInputState,
    on_swatch_click: Option<Message>,
    on_name_change: fn(String, TextInputState) -> Message,
    on_name_submit: Message,
    on_name_click: Message,
    on_edit_click: Message,
    on_delete_click: Message,
) {
    // Hotkey indicator (small, left of swatch)
    if show_hotkey {
        r.text(hotkey_str).size(FONT_SIZE_SMALL);
    }

    // Color swatch - same code path for both categories and tags
    let mut swatch = ColorSwatch::new(color)
        .width(Length::Fixed(20.0))
        .height(Length::Fixed(ROW_ITEM_HEIGHT))
        .filled(filled);
    if let Some(msg) = on_swatch_click {
        swatch = swatch.on_click(msg);
    }
    r.add(Element::new(swatch));

    if is_editing {
        // Show text input for editing
        let submit_msg = on_name_submit.clone();
        r.text_input()
            .value(name_input)
            .state(name_input_state)
            .placeholder("Name...")
            .width(Length::Fill(1.0))
            .on_change(on_name_change)
            .on_submit(move |_| submit_msg.clone())
            .build();
        // OK button to confirm
        r.button("OK")
            .width(Length::Fixed(ACTION_BUTTON_WIDTH))
            .padding(BUTTON_PADDING_COMPACT)
            .on_click(on_name_submit);
    } else {
        // Show name as button with selection indicator
        let label = if is_selected {
            format!("* {}", name)
        } else {
            name.to_string()
        };
        r.button(label)
            .width(Length::Fill(1.0))
            .padding(BUTTON_PADDING_COMPACT)
            .text_align(Alignment::Left)
            .on_click(on_name_click);
        // Edit button
        r.button("Edit")
            .width(Length::Fixed(ACTION_BUTTON_WIDTH))
            .padding(BUTTON_PADDING_COMPACT)
            .on_click(on_edit_click);
        // Delete button
        r.button("x")
            .width(Length::Fixed(20.0))
            .padding(BUTTON_PADDING_COMPACT)
            .on_click(on_delete_click);
    }
}

/// Get tooltip content for an annotation tool
fn tool_tooltip(tool: AnnotationTool, hotkey: &str) -> TooltipContent {
    match tool {
        AnnotationTool::Select => TooltipContent::rich(
            "Select Tool",
            format!(
                "Hotkey: {}\n\nSelect and modify existing annotations.\n\
                Click to select, drag to move.\n\
                Click corners/edges to resize.",
                hotkey
            ),
        ),
        AnnotationTool::BoundingBox => TooltipContent::rich(
            "Bounding Box Tool",
            format!(
                "Hotkey: {}\n\nDraw rectangular annotations.\n\
                Click and drag to create a box.",
                hotkey
            ),
        ),
        AnnotationTool::Polygon => TooltipContent::rich(
            "Polygon Tool",
            format!(
                "Hotkey: {}\n\nDraw polygon annotations.\n\
                Left click to add points, click on first point to close or press Enter.\n\
                If annotation is selected, left click on edge to add point and \n\
                right-click on point to remove.",
                hotkey
            ),
        ),
        AnnotationTool::Point => TooltipContent::rich(
            "Point Tool",
            format!(
                "Hotkey: {}\n\nPlace point annotations.\n\
                Click to place a point marker.",
                hotkey
            ),
        ),
    }
}

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
        let category_input_text = self.category_input_text.clone();
        let category_input_state = self.category_input_state.clone();
        let category_name_input = self.category_name_input.clone();
        let category_name_input_state = self.category_name_input_state;
        let color_picker_category = self.color_picker_category;
        let color_picker_state = self.color_picker_state;
        // Global tags (persist across all images, like categories)
        let tags = self.tags.clone();
        let selected_tag = self.selected_tag;
        // Per-image: which tag IDs are selected for the current image
        let current_image_data = self.image_data_store.get(&self.current_image_path());
        let selected_tag_ids = current_image_data.selected_tag_ids;
        let tag_input_text = self.tag_input_text.clone();
        let tag_input_state = self.tag_input_state.clone();
        let editing_tag = self.editing_tag;
        let tag_name_input = self.tag_name_input.clone();
        let tag_name_input_state = self.tag_name_input_state;
        let color_picker_tag = self.color_picker_tag;
        let keybindings = self.keybindings.clone();

        let mut sidebar_ctx = Context::new();

        // File Explorer Collapsible (VSCode-style tree view)
        let file_explorer_state = self.file_explorer_collapsed.clone();
        let file_explorer_scroll = self.file_explorer_scroll_state.clone();
        let file_tree_state = self.file_explorer_state.clone();

        // Build the file tree from project
        let file_tree_nodes = self
            .project
            .as_ref()
            .map(|p| p.build_file_tree())
            .unwrap_or_default();
        let current_path = self
            .project
            .as_ref()
            .and_then(|p| p.current_relative_path());
        let num_files = self.project.as_ref().map(|p| p.images.len()).unwrap_or(0);

        let collapsible_files = Collapsible::new("File Explorer")
            .state(&file_explorer_state)
            .scroll_state(&file_explorer_scroll)
            .scroll_direction(ScrollDirection::Both)
            .width(Length::Fill(1.0))
            .max_height(FILE_LIST_MAX_HEIGHT)
            .on_toggle(Message::FileExplorerToggled)
            .on_scroll(Message::FileExplorerScrolled)
            .content(|c| {
                if file_tree_nodes.is_empty() {
                    c.text("No files loaded").size(FONT_SIZE_SECONDARY);
                    c.text("Use 'Open Folder' to load images")
                        .size(FONT_SIZE_SMALL);
                } else {
                    c.text(format!("{} files", num_files))
                        .size(FONT_SIZE_SECONDARY);

                    // Create the file tree widget
                    let file_tree = FileTree::new()
                        .nodes(file_tree_nodes)
                        .state(&file_tree_state)
                        .selected(current_path)
                        .width(Length::Fill(1.0))
                        .on_select(Message::FileExplorerSelect)
                        .on_state_change(Message::FileExplorerStateChanged);

                    c.add(Element::new(file_tree));
                }
            });
        sidebar_ctx.add(Element::new(collapsible_files));

        // Tools Collapsible
        let tools_s = tools_state.clone();
        let keybindings_for_tools = keybindings.clone();
        let collapsible_tools = Collapsible::new("Annotation Tools")
            .state(&tools_s)
            .width(Length::Fill(1.0))
            .on_toggle(Message::ToolsToggled)
            .content(move |c| {
                for tool in AnnotationTool::all() {
                    let is_selected = *tool == selected_tool;
                    let tool_copy = *tool;
                    let hotkey = keybindings_for_tools.key_for_tool(*tool);
                    let hotkey_str = key_to_string(hotkey);
                    // Using parentheses instead of brackets for cross-platform compatibility
                    let label = if is_selected {
                        format!("> {} < ({})", tool.name(), hotkey_str)
                    } else {
                        format!("{} ({})", tool.name(), hotkey_str)
                    };

                    // Create tooltip ID and content for this tool
                    let tooltip_id = format!("tool_{}", tool.name().to_lowercase());
                    let tooltip_content = tool_tooltip(*tool, &hotkey_str);

                    c.button(label)
                        .width(Length::Fill(1.0))
                        .tooltip(
                            tooltip_id,
                            tooltip_content,
                            |id, content, bounds, pos| {
                                Message::TooltipRequest(id, content, bounds, pos)
                            },
                            |id| Message::TooltipClear(id),
                        )
                        .on_click(Message::ToolSelected(tool_copy));
                }
            });
        sidebar_ctx.add(Element::new(collapsible_tools));

        // Categories Collapsible
        let cats_s = categories_state.clone();
        let keybindings_for_cats = keybindings.clone();
        let collapsible_cats = Collapsible::new("Categories")
            .state(&cats_s)
            .width(Length::Fill(1.0))
            .on_toggle(Message::CategoriesToggled)
            .content(move |c| {
                for (cat_index, cat) in categories.iter().enumerate() {
                    let is_selected = cat.id == selected_category;
                    let is_editing = editing_category == Some(cat.id);
                    let cat_id = cat.id;
                    let cat_color = cat.color;
                    let cat_name = cat.name.clone();
                    // Get hotkey for this category index (only first 10 categories have hotkeys)
                    let hotkey = keybindings_for_cats.key_for_category_index(cat_index);
                    let hotkey_str = optional_key_to_string(hotkey);

                    c.row(|r| {
                        build_item_row(
                            r,
                            &hotkey_str,
                            cat_index < 10,
                            cat_color,
                            true, // Categories always filled
                            is_selected,
                            is_editing,
                            &cat_name,
                            &category_name_input,
                            &category_name_input_state,
                            Some(Message::ToggleCategoryColorPicker(cat_id)),
                            Message::CategoryNameChanged,
                            Message::FinishEditingCategory,
                            Message::CategorySelected(cat_id),
                            Message::StartEditingCategory(cat_id),
                            Message::DeleteCategory(cat_id),
                        );
                    });

                    // Show color picker if open for this category (opens below the swatch)
                    if color_picker_category == Some(cat.id) {
                        // Position picker below the color swatch, aligned with its left edge
                        let picker = ColorPicker::new()
                            .selected(cat_color)
                            .open(true)
                            .x_offset(COLOR_PICKER_SWATCH_OFFSET)
                            .state(&color_picker_state)
                            .on_change(Message::CategoryColorLiveUpdate) // Live updates from sliders
                            .on_select(Message::CategoryColorApply) // Palette click applies and closes
                            .on_close(Message::CloseCategoryColorPicker)
                            .on_state_change(Message::ColorPickerStateChanged); // For drag state tracking
                        c.add(Element::new(picker));
                    }
                }
                // Text input + button to add new categories
                c.row(|r| {
                    r.text_input()
                        .value(&category_input_text)
                        .state(&category_input_state)
                        .placeholder("Add category...")
                        .width(Length::Fill(1.0))
                        .on_change(Message::CategoryInputChanged)
                        .on_submit(|_| Message::AddCategory)
                        .build();
                    r.button("+")
                        .padding(BUTTON_PADDING_COMPACT)
                        .on_click(Message::AddCategory);
                });
            });
        sidebar_ctx.add(Element::new(collapsible_cats));

        // Image Tags Collapsible (similar structure to Categories)
        let tags_s = tags_state.clone();
        let collapsible_tags = Collapsible::new("Image Tags")
            .state(&tags_s)
            .width(Length::Fill(1.0))
            .on_toggle(Message::TagsToggled)
            .content(|c| {
                for (tag_index, tag) in tags.iter().enumerate() {
                    let is_selected = tag.id == selected_tag;
                    let is_on_image = selected_tag_ids.contains(&tag.id);
                    let is_editing = editing_tag == Some(tag.id);
                    let tag_id = tag.id;
                    let tag_color = tag.color;
                    let tag_name = tag.name.clone();
                    let hotkey_str = format!("{}", (tag_index + 1) % 10);

                    c.row(|r| {
                        build_item_row(
                            r,
                            &hotkey_str,
                            tag_index < 10,
                            tag_color,
                            is_on_image, // Tags: filled when applied to current image
                            is_selected,
                            is_editing,
                            &tag_name,
                            &tag_name_input,
                            &tag_name_input_state,
                            Some(Message::ToggleTagColorPicker(tag_id)),
                            Message::TagNameChanged,
                            Message::FinishEditingTag,
                            Message::ToggleImageTag(tag_id), // Toggle tag on current image
                            Message::StartEditingTag(tag_id),
                            Message::DeleteTag(tag_id),
                        );
                    });

                    // Show color picker if open for this tag
                    if color_picker_tag == Some(tag.id) {
                        let picker = ColorPicker::new()
                            .selected(tag_color)
                            .open(true)
                            .x_offset(COLOR_PICKER_SWATCH_OFFSET)
                            .state(&color_picker_state)
                            .on_change(Message::TagColorLiveUpdate)
                            .on_select(Message::TagColorApply)
                            .on_close(Message::CloseTagColorPicker)
                            .on_state_change(Message::ColorPickerStateChanged);
                        c.add(Element::new(picker));
                    }
                }
                // Text input + button to add new tags
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
                    r.button("+")
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

        let theme = current_theme();

        // Wrap in panel with right and top borders + distinct background
        let panel = Panel::new(Element::new(scrollable))
            .borders(BorderSides::right_top())
            .border_color(theme.divider)
            .background(theme.surface)
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill(1.0));

        Element::new(panel)
    }
}
