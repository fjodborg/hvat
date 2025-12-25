//! Context menu UI for HVAT application.
//!
//! Provides right-click context menu for annotation editing.

use hvat_ui::{ContextMenu, Element, MenuItem};

use crate::app::HvatApp;
use crate::message::Message;

impl HvatApp {
    /// Build the context menu widget.
    pub(crate) fn build_context_menu(&self) -> Element<Message> {
        // Build menu items
        let mut items = Vec::new();

        // If we right-clicked on an annotation, show category options
        if self.context_menu_annotation_id.is_some() {
            // Add header text (as a disabled item)
            items.push(MenuItem::new("header", "Assign Category:").disabled());

            // Add separator
            items.push(MenuItem::separator());

            // Add category options
            for category in &self.categories {
                let item_id = format!("category_{}", category.id);
                let item = MenuItem::new(item_id, &category.name).with_color(category.color);
                items.push(item);
            }
        } else {
            // No annotation - show category selection for new annotations
            items.push(MenuItem::new("header", "Select Category:").disabled());
            items.push(MenuItem::separator());

            for category in &self.categories {
                let item_id = format!("category_{}", category.id);
                // Using ASCII symbols for cross-platform compatibility
                let label = if category.id == self.selected_category {
                    format!("* {}", category.name)
                } else {
                    format!("  {}", category.name)
                };
                let item = MenuItem::new(item_id, &label).with_color(category.color);
                items.push(item);
            }
        }

        // Add WASM-specific hint for browser context menu
        #[cfg(target_arch = "wasm32")]
        {
            items.push(MenuItem::separator());
            items.push(
                MenuItem::new("browser_hint", "Shift+Right-click for browser menu").disabled(),
            );
        }

        Element::new(
            ContextMenu::new()
                .open(self.context_menu_open)
                .position(self.context_menu_position.0, self.context_menu_position.1)
                .viewport_size(self.window_size.0, self.window_size.1)
                .items(items)
                .on_select(Message::ContextMenuSelect)
                .on_close(|_| Message::CloseContextMenu),
        )
    }
}
