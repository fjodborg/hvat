//! Dropdown Widget Example
//!
//! Demonstrates the dropdown/select widget with various configurations.

use hvat_ui::prelude::*;
use hvat_ui::{Dropdown, Element};

/// Application state
struct DropdownDemo {
    /// Simple dropdown state
    simple_dropdown: DropdownState,
    /// Searchable dropdown state
    searchable_dropdown: DropdownState,
    /// Selected index for simple dropdown
    selected_simple: Option<usize>,
    /// Selected index for searchable dropdown
    selected_search: Option<usize>,
}

/// Application messages
#[derive(Clone)]
enum Message {
    /// Simple dropdown state changed
    SimpleStateChanged(DropdownState),
    /// Simple dropdown selection changed
    SimpleSelected(usize),
    /// Searchable dropdown state changed
    SearchStateChanged(DropdownState),
    /// Searchable dropdown selection changed
    SearchSelected(usize),
    /// Reset selections
    Reset,
}

impl Default for DropdownDemo {
    fn default() -> Self {
        Self {
            simple_dropdown: DropdownState::new(),
            searchable_dropdown: DropdownState::new(),
            selected_simple: None,
            selected_search: None,
        }
    }
}

const SIMPLE_OPTIONS: &[&str] = &[
    "Option 1",
    "Option 2",
    "Option 3",
    "Option 4",
    "Option 5",
];

const COUNTRY_OPTIONS: &[&str] = &[
    "Argentina",
    "Australia",
    "Brazil",
    "Canada",
    "China",
    "France",
    "Germany",
    "India",
    "Italy",
    "Japan",
    "Mexico",
    "Netherlands",
    "Spain",
    "United Kingdom",
    "United States",
];

impl Application for DropdownDemo {
    type Message = Message;

    fn view(&self) -> Element<Message> {
        col(|c| {
            // Header
            c.text("Dropdown Widget Demo");

            // Control bar
            c.row(|r| {
                r.button("Reset").on_click(Message::Reset);
            });

            // Dropdown examples
            c.row(|r| {
                // Left column - Simple dropdown
                r.col(|panel| {
                    panel.text("Simple Dropdown");
                    panel.text(format!(
                        "Selected: {}",
                        self.selected_simple
                            .and_then(|i| SIMPLE_OPTIONS.get(i))
                            .unwrap_or(&"None")
                    ));

                    // Create the actual dropdown widget
                    let dropdown = Dropdown::new()
                        .state(&self.simple_dropdown)
                        .options(SIMPLE_OPTIONS.iter().copied())
                        .selected(self.selected_simple)
                        .placeholder("Choose an option...")
                        .width(200.0)
                        .on_select(Message::SimpleSelected)
                        .on_change(Message::SimpleStateChanged);

                    panel.add(Element::new(dropdown));
                });

                // Right column - Searchable dropdown
                r.col(|panel| {
                    panel.text("Searchable Dropdown");
                    panel.text(format!(
                        "Selected: {}",
                        self.selected_search
                            .and_then(|i| COUNTRY_OPTIONS.get(i))
                            .unwrap_or(&"None")
                    ));

                    // Create searchable dropdown
                    let dropdown = Dropdown::new()
                        .state(&self.searchable_dropdown)
                        .options(COUNTRY_OPTIONS.iter().copied())
                        .selected(self.selected_search)
                        .placeholder("Select a country...")
                        .searchable(true)
                        .width(250.0)
                        .on_select(Message::SearchSelected)
                        .on_change(Message::SearchStateChanged);

                    panel.add(Element::new(dropdown));
                });
            });

            // Instructions
            c.text("Click to open | Arrow keys to navigate | Enter to select | Escape to close");
            c.text("Searchable dropdown: Type to filter options");
        })
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::SimpleStateChanged(state) => {
                log::info!("SimpleStateChanged: is_open={}", state.is_open);
                self.simple_dropdown = state;
            }
            Message::SimpleSelected(index) => {
                self.selected_simple = Some(index);
                self.simple_dropdown.close();
                log::info!("Simple dropdown selected: {}", SIMPLE_OPTIONS.get(index).unwrap_or(&"?"));
            }
            Message::SearchStateChanged(state) => {
                self.searchable_dropdown = state;
            }
            Message::SearchSelected(index) => {
                self.selected_search = Some(index);
                self.searchable_dropdown.close();
                log::info!("Search dropdown selected: {}", COUNTRY_OPTIONS.get(index).unwrap_or(&"?"));
            }
            Message::Reset => {
                self.selected_simple = None;
                self.selected_search = None;
                self.simple_dropdown = DropdownState::new();
                self.searchable_dropdown = DropdownState::new();
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default()
        .title("Dropdown Widget Demo")
        .size(800, 600);

    hvat_ui::run_with_settings(DropdownDemo::default(), settings)
}
