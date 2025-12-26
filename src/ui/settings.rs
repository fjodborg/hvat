//! Settings view UI component.
//!
//! A full-page settings view with Settings and About sections.

use hvat_ui::constants::BUTTON_PADDING_COMPACT;
use hvat_ui::prelude::*;
use hvat_ui::{
    Collapsible, Column, Context, Element, ScrollDirection, Scrollable, ScrollbarVisibility, Text,
};

use crate::app::HvatApp;
use crate::config::LogLevel;
use crate::constants::MAX_GPU_PRELOAD_COUNT;
use crate::keybindings::{KeybindTarget, key_to_string};
use crate::licenses::{DEPENDENCIES, DependencyInfo};
use crate::message::Message;
use crate::model::AnnotationTool;

/// Application version
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Git commit hash (short)
pub const GIT_HASH: &str = env!("GIT_HASH");

/// Application name
const APP_NAME: &str = "HVAT - Hyperspectral Vision Annotation Tool";

/// Fixed keyboard shortcuts (not customizable)
const FIXED_KEYBINDINGS: &[(&str, &str)] = &[
    ("Ctrl+Z", "Undo"),
    ("Ctrl+Shift+Z / Ctrl+Y", "Redo"),
    ("Escape", "Cancel annotation"),
    ("Delete / Backspace", "Delete selected annotation"),
    ("Enter", "Finish polygon"),
    ("0", "Zoom to 100%"),
    ("F", "Fit image to view"),
    ("+", "Zoom in"),
    ("-", "Zoom out"),
];

impl HvatApp {
    /// Build the settings view (full page).
    pub(crate) fn build_settings_view(&self) -> Element<Message> {
        let scroll_state = self.settings_scroll_state.clone();
        let settings_section_collapsed = self.settings_section_collapsed;
        let appearance_section_collapsed = self.appearance_section_collapsed;
        let keybindings_section_collapsed = self.keybindings_section_collapsed;
        let folders_section_collapsed = self.folders_section_collapsed;
        let performance_section_collapsed = self.performance_section_collapsed;
        let dependencies_collapsed = self.dependencies_collapsed;
        let license_collapsed = self.license_collapsed.clone();

        // Clone values for closures
        let dark_theme = self.dark_theme;
        let log_level = self.log_level;
        let export_folder = self.export_folder.clone();
        let export_folder_state = self.export_folder_state.clone();
        let import_folder = self.import_folder.clone();
        let import_folder_state = self.import_folder_state.clone();
        let gpu_preload_count = self.gpu_preload_count;
        let gpu_preload_slider = self.gpu_preload_slider.clone();

        let mut ctx = Context::new();

        // Header with back button and export/import
        ctx.row(|r| {
            r.button("< Back")
                .padding(BUTTON_PADDING_COMPACT)
                .on_click(Message::CloseSettings);
            r.button("Export")
                .padding(BUTTON_PADDING_COMPACT)
                .on_click(Message::ExportConfig);
            r.button("Import")
                .padding(BUTTON_PADDING_COMPACT)
                .on_click(Message::ImportConfig);
            r.text("Settings").size(FONT_SIZE_TITLE);
            // Spacer to push version to the right
            r.add(Element::new(Text::new("").width(Length::Fill(1.0))));
            r.text(format!("v{} ({})", APP_VERSION, GIT_HASH))
                .size(FONT_SIZE_SMALL);
        });

        ctx.text("");

        // ========================================
        // Settings Section (collapsible)
        // ========================================
        let settings_collapsible = Collapsible::new("Settings")
            .state(&settings_section_collapsed)
            .on_toggle(Message::SettingsSectionToggled)
            .content(|c| {
                // --------------------------------
                // Appearance subsection (collapsible)
                // --------------------------------
                let appearance_collapsible = Collapsible::new("Appearance")
                    .state(&appearance_section_collapsed)
                    .on_toggle(Message::AppearanceSectionToggled)
                    .content(|ac| {
                        ac.row(|r| {
                            r.text("Theme:");
                            r.button(if dark_theme { "Dark" } else { "Light" })
                                .padding(BUTTON_PADDING_COMPACT)
                                .on_click(Message::ThemeChanged(!dark_theme));
                        });
                        ac.text("(Theme switching not yet implemented)")
                            .size(FONT_SIZE_SMALL);

                        ac.text("");

                        // Log Level setting
                        ac.row(|r| {
                            r.text("Log Level:");
                            for level in LogLevel::all() {
                                let is_selected = *level == log_level;
                                let label = if is_selected {
                                    format!("[{}]", level.name())
                                } else {
                                    level.name().to_string()
                                };
                                r.button(label)
                                    .padding(BUTTON_PADDING_COMPACT)
                                    .on_click(Message::LogLevelChanged(*level));
                            }
                        });
                        ac.text("Controls verbosity of log output (Error = least, Trace = most)")
                            .size(FONT_SIZE_SMALL);
                    });
                c.add(Element::new(appearance_collapsible));

                c.text("");

                // --------------------------------
                // Default Folders subsection (collapsible)
                // --------------------------------
                let folders_collapsible = Collapsible::new("Default Folders")
                    .state(&folders_section_collapsed)
                    .on_toggle(Message::FoldersSectionToggled)
                    .content(|fc| {
                        fc.row(|r| {
                            r.text("Export:");
                            r.text_input()
                                .placeholder("./exports")
                                .value(&export_folder)
                                .state(&export_folder_state)
                                .width(Length::Fixed(200.0))
                                .on_change(Message::ExportFolderChanged)
                                .build();
                        });

                        fc.row(|r| {
                            r.text("Import:");
                            r.text_input()
                                .placeholder("./imports")
                                .value(&import_folder)
                                .state(&import_folder_state)
                                .width(Length::Fixed(200.0))
                                .on_change(Message::ImportFolderChanged)
                                .build();
                        });
                    });
                c.add(Element::new(folders_collapsible));

                c.text("");

                // --------------------------------
                // Performance subsection (collapsible)
                // --------------------------------
                let performance_collapsible = Collapsible::new("Performance")
                    .state(&performance_section_collapsed)
                    .on_toggle(Message::PerformanceSectionToggled)
                    .content(|pc| {
                        pc.row(|r| {
                            r.text("GPU Preload:");
                            r.slider(0.0, MAX_GPU_PRELOAD_COUNT as f32)
                                .step(1.0)
                                .state(&gpu_preload_slider)
                                .width(Length::Fixed(120.0))
                                .on_change(Message::GpuPreloadCountChanged)
                                .build();
                            r.text(format!("{}", gpu_preload_count));
                        });
                        pc.text("Number of images to preload before/after current (0 = disabled)")
                            .size(FONT_SIZE_SMALL);
                    });
                c.add(Element::new(performance_collapsible));

                c.text("");

                // --------------------------------
                // Keybindings subsection (collapsible)
                // --------------------------------
                let keybindings = self.keybindings.clone();
                let categories = self.categories.clone();
                let capturing_keybind = self.capturing_keybind;

                let keybindings_collapsible = Collapsible::new("Keyboard Shortcuts")
                    .state(&keybindings_section_collapsed)
                    .on_toggle(Message::KeybindingsSectionToggled)
                    .content(move |kc| {
                        // Customizable Tool Hotkeys section
                        kc.text("Annotation Tools").size(FONT_SIZE_SECONDARY);
                        kc.text("Click a key to change the hotkey")
                            .size(FONT_SIZE_SMALL);
                        kc.text("");

                        for tool in AnnotationTool::all() {
                            let tool_copy = *tool;
                            let current_key = keybindings.key_for_tool(*tool);
                            let is_capturing =
                                capturing_keybind == Some(KeybindTarget::Tool(*tool));

                            let key_label = if is_capturing {
                                "Press key...".to_string()
                            } else {
                                key_to_string(current_key).to_string()
                            };

                            kc.row(|r| {
                                r.add(Element::new(
                                    Text::new(tool.name()).width(Length::Fixed(120.0)),
                                ));
                                r.button(key_label)
                                    .width(Length::Fixed(80.0))
                                    .padding(BUTTON_PADDING_COMPACT)
                                    .on_click(Message::StartCapturingKeybind(KeybindTarget::Tool(
                                        tool_copy,
                                    )));
                            });
                        }

                        kc.text("");

                        // Customizable Category Hotkeys section
                        kc.text("Category Selection").size(FONT_SIZE_SECONDARY);
                        kc.text("Keys 1-9, 0 select categories 1-10")
                            .size(FONT_SIZE_SMALL);
                        kc.text("");

                        for (index, cat) in categories.iter().take(10).enumerate() {
                            let current_key = keybindings.key_for_category_index(index);
                            let is_capturing =
                                capturing_keybind == Some(KeybindTarget::Category(index));

                            let key_label = if is_capturing {
                                "Press key...".to_string()
                            } else {
                                current_key
                                    .map(|k| key_to_string(k))
                                    .unwrap_or("-")
                                    .to_string()
                            };

                            kc.row(|r| {
                                r.add(Element::new(
                                    Text::new(format!("{}. {}", index + 1, cat.name))
                                        .width(Length::Fixed(120.0)),
                                ));
                                r.button(key_label)
                                    .width(Length::Fixed(80.0))
                                    .padding(BUTTON_PADDING_COMPACT)
                                    .on_click(Message::StartCapturingKeybind(
                                        KeybindTarget::Category(index),
                                    ));
                            });
                        }

                        kc.text("");
                        kc.button("Reset to Defaults")
                            .padding(BUTTON_PADDING_COMPACT)
                            .on_click(Message::ResetKeybindings);

                        kc.text("");
                        kc.text("");

                        // Fixed shortcuts
                        kc.text("Fixed Shortcuts").size(FONT_SIZE_SECONDARY);
                        kc.text("");
                        for (key, action) in FIXED_KEYBINDINGS {
                            kc.row(|r| {
                                r.add(Element::new(Text::new(*key).width(Length::Fixed(180.0))));
                                r.text(*action);
                            });
                        }
                    });
                c.add(Element::new(keybindings_collapsible));
            });

        ctx.add(Element::new(settings_collapsible));

        ctx.text("");
        ctx.text("");

        // ========================================
        // About Section
        // ========================================
        ctx.text("About")
            .size(FONT_SIZE_SECTION)
            .align(Alignment::Center);
        ctx.text("");
        ctx.text(APP_NAME).align(Alignment::Center);
        ctx.text("");
        ctx.text("A GPU-accelerated desktop and web application")
            .align(Alignment::Center);
        ctx.text("for hyperspectral image annotation.")
            .align(Alignment::Center);
        ctx.text("");
        ctx.text("License: AGPL-3.0").align(Alignment::Center);
        ctx.text("Source: https://github.com/fjodborg/hvat")
            .align(Alignment::Center);
        ctx.text("My intention is to make it so you own the data and the output of the program, but in this stage i'm keeping it AGPL-3.0, but i might change it to MIT in the future")
            .align(Alignment::Center)
            .wrap(true);

        ctx.text("");
        ctx.text("");

        // Group dependencies by license
        let mut by_license: std::collections::HashMap<&str, Vec<&DependencyInfo>> =
            std::collections::HashMap::new();
        for dep in DEPENDENCIES.iter() {
            by_license.entry(dep.license).or_default().push(dep);
        }

        // Sort licenses alphabetically
        let mut licenses: Vec<_> = by_license.keys().collect();
        licenses.sort();

        // Dependencies Section (collapsible) with nested license collapsibles
        let collapsible =
            Collapsible::new(format!("Third-Party Dependencies ({})", DEPENDENCIES.len()))
                .state(&dependencies_collapsed)
                .on_toggle(Message::DependenciesToggled)
                .content(|c| {
                    // Create a nested collapsible for each license type
                    for license in &licenses {
                        let deps = by_license.get(*license).unwrap();
                        let license_string = license.to_string();
                        let license_state = license_collapsed
                            .get(&license_string)
                            .copied()
                            .unwrap_or_else(CollapsibleState::collapsed);

                        // Clone for the closure
                        let license_for_closure = license_string.clone();

                        let license_collapsible =
                            Collapsible::new(format!("{} ({})", license, deps.len()))
                                .state(&license_state)
                                .on_toggle(move |state| {
                                    Message::LicenseToggled(license_for_closure.clone(), state)
                                })
                                .content(|lc| {
                                    for dep in deps {
                                        let repo = dep.repository.unwrap_or("crates.io");
                                        lc.text(format!("{} v{}", dep.name, dep.version));
                                        lc.text(format!("  {}", repo)).size(FONT_SIZE_SMALL);
                                    }
                                });

                        c.add(Element::new(license_collapsible));
                    }
                });

        ctx.add(Element::new(collapsible));

        // Extra padding at bottom
        ctx.text("");
        ctx.text("");

        // Wrap in scrollable
        // Column must fill width so centered text can center properly
        let content = Element::new(
            Column::new(ctx.take())
                .padding(16.0)
                .width(Length::Fill(1.0)),
        );
        let scrollable = Scrollable::new(content)
            .state(&scroll_state)
            .direction(ScrollDirection::Vertical)
            .scrollbar_visibility(ScrollbarVisibility::Auto)
            .width(Length::Fill(1.0))
            .height(Length::Fill(1.0))
            .on_scroll(Message::SettingsScrolled);

        Element::new(scrollable)
    }
}
