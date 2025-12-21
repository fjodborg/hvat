//! hvat_ui unified demo
//!
//! A single demo that showcases all UI widgets with tab switching.
//! Works as both native and WASM target.
//!
//! Run natively: `cargo run --bin hvat_ui_example`
//! Run in browser: `trunk serve --release`
//!
//! Uses reusable demo components from hvat_ui::demos.

use hvat_ui::demos::{
    BasicDemo, BasicMessage, CollapsibleDemo, CollapsibleMessage, DropdownDemo, DropdownMessage,
    ImageViewerDemo, ImageViewerMessage, InputWidgetsDemo, InputWidgetsMessage, ScrollableDemo,
    ScrollableMessage, UndoDemo, UndoMessage,
};
use hvat_ui::prelude::*;

/// Which demo tab is active
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum DemoTab {
    #[default]
    Basic,
    InputWidgets,
    Undo,
    Scrollable,
    Dropdown,
    Collapsible,
    ImageViewer,
}

/// Unified demo application state
struct UnifiedDemo {
    active_tab: DemoTab,
    basic: BasicDemo,
    input_widgets: InputWidgetsDemo,
    undo: UndoDemo,
    scrollable: ScrollableDemo,
    dropdown: DropdownDemo,
    collapsible: CollapsibleDemo,
    image_viewer: ImageViewerDemo,
}

/// Application messages
#[derive(Clone)]
enum Message {
    SwitchTab(DemoTab),
    Basic(BasicMessage),
    InputWidgets(InputWidgetsMessage),
    Undo(UndoMessage),
    Scrollable(ScrollableMessage),
    Dropdown(DropdownMessage),
    Collapsible(CollapsibleMessage),
    ImageViewer(ImageViewerMessage),
}

impl Default for UnifiedDemo {
    fn default() -> Self {
        Self {
            active_tab: DemoTab::Basic,
            basic: BasicDemo::new(),
            input_widgets: InputWidgetsDemo::new(),
            undo: UndoDemo::new(),
            scrollable: ScrollableDemo::new(),
            dropdown: DropdownDemo::new(),
            collapsible: CollapsibleDemo::new(),
            image_viewer: ImageViewerDemo::new(),
        }
    }
}

impl Application for UnifiedDemo {
    type Message = Message;

    fn setup(&mut self, resources: &mut Resources) {
        // Setup image viewer demo (creates test texture)
        self.image_viewer.setup(resources);
    }

    fn on_event(&mut self, event: &hvat_ui::Event) -> Option<Message> {
        // Handle global keyboard shortcuts when Undo tab is active
        if self.active_tab == DemoTab::Undo {
            if let Some(msg) = UndoDemo::handle_key_event(event) {
                return Some(Message::Undo(msg));
            }
        }
        None
    }

    fn view(&self) -> Element<Message> {
        col(|c| {
            c.text("hvat_ui Demo Gallery");

            // Tab buttons - first row
            c.row(|r| {
                r.button("Basic")
                    .on_click(Message::SwitchTab(DemoTab::Basic));
                r.button("Input")
                    .on_click(Message::SwitchTab(DemoTab::InputWidgets));
                r.button("Undo").on_click(Message::SwitchTab(DemoTab::Undo));
                r.button("Scrollable")
                    .on_click(Message::SwitchTab(DemoTab::Scrollable));
            });
            // Tab buttons - second row
            c.row(|r| {
                r.button("Dropdown")
                    .on_click(Message::SwitchTab(DemoTab::Dropdown));
                r.button("Collapsible")
                    .on_click(Message::SwitchTab(DemoTab::Collapsible));
                r.button("ImageViewer")
                    .on_click(Message::SwitchTab(DemoTab::ImageViewer));
            });

            // Show current tab
            c.row(|r| {
                let tab_name = match self.active_tab {
                    DemoTab::Basic => "Basic",
                    DemoTab::InputWidgets => "Input Widgets",
                    DemoTab::Undo => "Undo/Redo",
                    DemoTab::Scrollable => "Scrollable",
                    DemoTab::Dropdown => "Dropdown",
                    DemoTab::Collapsible => "Collapsible",
                    DemoTab::ImageViewer => "ImageViewer",
                };
                r.text(format!("Current: {}", tab_name));
            });

            c.text("────────────────────────────────────────");

            // Active demo content
            let demo_content = match self.active_tab {
                DemoTab::Basic => self.basic.view(Message::Basic),
                DemoTab::InputWidgets => self.input_widgets.view(Message::InputWidgets),
                DemoTab::Undo => self.undo.view(Message::Undo),
                DemoTab::Scrollable => self.scrollable.view(Message::Scrollable),
                DemoTab::Dropdown => self.dropdown.view(Message::Dropdown),
                DemoTab::Collapsible => self.collapsible.view(Message::Collapsible),
                DemoTab::ImageViewer => self.image_viewer.view(Message::ImageViewer),
            };
            c.add(demo_content);
        })
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::SwitchTab(tab) => {
                log::info!("Switching to tab: {:?}", tab as u8);
                self.active_tab = tab;
            }
            Message::Basic(msg) => self.basic.update(msg),
            Message::InputWidgets(msg) => self.input_widgets.update(msg),
            Message::Undo(msg) => self.undo.update(msg),
            Message::Scrollable(msg) => self.scrollable.update(msg),
            Message::Dropdown(msg) => self.dropdown.update(msg),
            Message::Collapsible(msg) => self.collapsible.update(msg),
            Message::ImageViewer(msg) => self.image_viewer.update(msg),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default()
        .title("hvat_ui Demo Gallery")
        .size(900, 700);

    hvat_ui::run_with_settings(UnifiedDemo::default(), settings)
}
