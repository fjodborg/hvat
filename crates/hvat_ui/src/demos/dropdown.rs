//! Dropdown widget demo

use crate::element::Element;
use crate::prelude::*;
use crate::widgets::Dropdown;

/// Dropdown demo state
pub struct DropdownDemo {
    pub simple_dropdown: DropdownState,
    pub searchable_dropdown: DropdownState,
    pub selected_simple: Option<usize>,
    pub selected_search: Option<usize>,
}

/// Dropdown demo messages
#[derive(Clone)]
pub enum DropdownMessage {
    SimpleStateChanged(DropdownState),
    SimpleSelected(usize),
    SearchStateChanged(DropdownState),
    SearchSelected(usize),
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

pub const SIMPLE_OPTIONS: &[&str] = &[
    "Option 1",
    "Option 2",
    "Option 3",
    "Option 4",
    "Option 5",
];

pub const COUNTRY_OPTIONS: &[&str] = &[
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

impl DropdownDemo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view<M: Clone + 'static>(&self, wrap: impl Fn(DropdownMessage) -> M + Clone + 'static) -> Element<M> {
        let wrap_reset = wrap.clone();
        let wrap_simple_state = wrap.clone();
        let wrap_simple_select = wrap.clone();
        let wrap_search_state = wrap.clone();
        let wrap_search_select = wrap.clone();

        let selected_simple = self.selected_simple;
        let selected_search = self.selected_search;
        let simple_dropdown = self.simple_dropdown.clone();
        let searchable_dropdown = self.searchable_dropdown.clone();

        col(move |c| {
            c.text("Dropdown Widget Demo");

            let reset_msg = wrap_reset(DropdownMessage::Reset);
            c.row(|r| {
                r.button("Reset").on_click(reset_msg);
            });

            let simple_dropdown_clone = simple_dropdown.clone();
            let searchable_dropdown_clone = searchable_dropdown.clone();
            let wrap_simple_state_inner = wrap_simple_state.clone();
            let wrap_simple_select_inner = wrap_simple_select.clone();
            let wrap_search_state_inner = wrap_search_state.clone();
            let wrap_search_select_inner = wrap_search_select.clone();

            c.row(move |r| {
                // Simple dropdown
                let simple_dropdown_inner = simple_dropdown_clone.clone();
                let wrap_ss = wrap_simple_state_inner.clone();
                let wrap_ssel = wrap_simple_select_inner.clone();

                r.col(move |panel| {
                    panel.text("Simple Dropdown");
                    panel.text(format!(
                        "Selected: {}",
                        selected_simple
                            .and_then(|i| SIMPLE_OPTIONS.get(i))
                            .unwrap_or(&"None")
                    ));

                    let dropdown = Dropdown::new()
                        .state(&simple_dropdown_inner)
                        .options(SIMPLE_OPTIONS.iter().copied())
                        .selected(selected_simple)
                        .placeholder("Choose an option...")
                        .width(200.0)
                        .on_select(move |i| wrap_ssel(DropdownMessage::SimpleSelected(i)))
                        .on_change(move |s| wrap_ss(DropdownMessage::SimpleStateChanged(s)));

                    panel.add(Element::new(dropdown));
                });

                // Searchable dropdown
                let searchable_dropdown_inner = searchable_dropdown_clone.clone();
                let wrap_srs = wrap_search_state_inner.clone();
                let wrap_srsel = wrap_search_select_inner.clone();

                r.col(move |panel| {
                    panel.text("Searchable Dropdown");
                    panel.text(format!(
                        "Selected: {}",
                        selected_search
                            .and_then(|i| COUNTRY_OPTIONS.get(i))
                            .unwrap_or(&"None")
                    ));

                    let dropdown = Dropdown::new()
                        .state(&searchable_dropdown_inner)
                        .options(COUNTRY_OPTIONS.iter().copied())
                        .selected(selected_search)
                        .placeholder("Select a country...")
                        .searchable(true)
                        .width(250.0)
                        .on_select(move |i| wrap_srsel(DropdownMessage::SearchSelected(i)))
                        .on_change(move |s| wrap_srs(DropdownMessage::SearchStateChanged(s)));

                    panel.add(Element::new(dropdown));
                });
            });

            c.text("Click to open | Arrow keys | Enter to select | Escape to close");
        })
    }

    pub fn update(&mut self, message: DropdownMessage) {
        match message {
            DropdownMessage::SimpleStateChanged(state) => {
                log::info!("SimpleStateChanged: is_open={}", state.is_open);
                self.simple_dropdown = state;
            }
            DropdownMessage::SimpleSelected(index) => {
                self.selected_simple = Some(index);
                self.simple_dropdown.close();
                log::info!("Simple dropdown selected: {}", SIMPLE_OPTIONS.get(index).unwrap_or(&"?"));
            }
            DropdownMessage::SearchStateChanged(state) => {
                self.searchable_dropdown = state;
            }
            DropdownMessage::SearchSelected(index) => {
                self.selected_search = Some(index);
                self.searchable_dropdown.close();
                log::info!("Search dropdown selected: {}", COUNTRY_OPTIONS.get(index).unwrap_or(&"?"));
            }
            DropdownMessage::Reset => {
                self.selected_simple = None;
                self.selected_search = None;
                self.simple_dropdown = DropdownState::new();
                self.searchable_dropdown = DropdownState::new();
            }
        }
    }
}
