//! Collapsible widget example
//!
//! Demonstrates:
//! - Basic collapsible sections
//! - Nested collapsibles
//! - Accordion mode (only one open at a time)
//! - Scrollable content with max_height
//! - Custom styling

use hvat_ui::prelude::*;
use hvat_ui::{collapsible, Color, CollapsibleState, Element};

/// Demo application state
#[derive(Default)]
struct CollapsibleDemo {
    /// States for individual collapsible sections
    section_states: Vec<CollapsibleState>,
    /// States for accordion sections
    accordion_states: Vec<CollapsibleState>,
    /// Nested section state
    nested_outer: CollapsibleState,
    nested_inner: CollapsibleState,
    /// Scrollable section state
    scrollable_section: CollapsibleState,
    /// Click counter (for nested content demo)
    click_count: u32,
}

impl CollapsibleDemo {
    fn new() -> Self {
        Self {
            section_states: vec![
                CollapsibleState::expanded(),
                CollapsibleState::collapsed(),
                CollapsibleState::collapsed(),
            ],
            accordion_states: vec![
                CollapsibleState::expanded(),
                CollapsibleState::collapsed(),
                CollapsibleState::collapsed(),
            ],
            nested_outer: CollapsibleState::expanded(),
            nested_inner: CollapsibleState::collapsed(),
            scrollable_section: CollapsibleState::expanded(),
            click_count: 0,
        }
    }
}

/// Message types
#[derive(Clone)]
enum Message {
    /// A regular section was toggled
    SectionToggled(usize, CollapsibleState),
    /// An accordion section was toggled (closes others)
    AccordionToggled(usize, CollapsibleState),
    /// Outer nested section toggled
    NestedOuterToggled(CollapsibleState),
    /// Inner nested section toggled
    NestedInnerToggled(CollapsibleState),
    /// Scrollable section toggled
    ScrollableSectionToggled(CollapsibleState),
    /// Button clicked inside content
    ButtonClicked,
}

impl Application for CollapsibleDemo {
    type Message = Message;

    fn view(&self) -> Element<Self::Message> {
        col(|c| {
            c.text("Collapsible Widget Demo");
            c.text("");
            c.text("-- Basic Collapsible Sections --");

            // Basic collapsible sections
            for (i, state) in self.section_states.iter().enumerate() {
                let section_title = format!(
                    "Section {} - Click to {}",
                    i + 1,
                    if state.is_expanded {
                        "collapse"
                    } else {
                        "expand"
                    }
                );
                c.add(Element::new(
                    collapsible(section_title)
                        .state(state)
                        .on_toggle(move |s| Message::SectionToggled(i, s))
                        .content(|content| {
                            content.text(format!("This is the content of section {}.", i + 1));
                            content.text("You can put any widgets here.");
                            content.text(format!("State: expanded={}", state.is_expanded));
                        }),
                ));
            }

            c.text("");
            c.text("-- Accordion Mode (one at a time) --");

            // Accordion sections
            let accordion_titles = ["First Panel", "Second Panel", "Third Panel"];
            for (i, (state, title)) in self
                .accordion_states
                .iter()
                .zip(accordion_titles.iter())
                .enumerate()
            {
                c.add(Element::new(
                    collapsible(*title)
                        .state(state)
                        .header_color(Color::rgba(0.2, 0.15, 0.25, 1.0))
                        .on_toggle(move |s| Message::AccordionToggled(i, s))
                        .content(|content| {
                            content.text(format!("Content for: {}", title));
                            content
                                .text("In accordion mode, opening one panel closes the others.");
                        }),
                ));
            }

            c.text("");
            c.text("-- Scrollable Content (max_height = 120px) --");

            // Scrollable collapsible section
            c.add(Element::new(
                collapsible("Scrollable Section - Use mouse wheel to scroll")
                    .state(&self.scrollable_section)
                    .header_color(Color::rgba(0.15, 0.25, 0.2, 1.0))
                    .max_height(120.0)
                    .on_toggle(Message::ScrollableSectionToggled)
                    .content(|content| {
                        for i in 1..=20 {
                            content.text(format!("Line {} - This content scrolls when it exceeds max_height", i));
                        }
                    }),
            ));

            c.text("");
            c.text("-- Nested Collapsibles --");

            // Nested collapsibles
            c.add(Element::new(
                collapsible("Outer Section")
                    .state(&self.nested_outer)
                    .on_toggle(Message::NestedOuterToggled)
                    .content(|outer| {
                        outer.text("This is the outer content.");
                        outer.text("");
                        outer.add(Element::new(
                            collapsible("Inner Section")
                                .state(&self.nested_inner)
                                .header_color(Color::rgba(0.15, 0.2, 0.25, 1.0))
                                .on_toggle(Message::NestedInnerToggled)
                                .content(|inner| {
                                    inner.text("This is nested inside the outer section.");
                                    inner.text(format!("Button clicks: {}", self.click_count));
                                    inner.button("Click Me!").on_click(Message::ButtonClicked);
                                }),
                        ));
                    }),
            ));

            c.text("");
            c.text("Controls: Click header to toggle | Enter/Space when hovering | Mouse wheel to scroll");
        })
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::SectionToggled(index, state) => {
                log::info!("Section {} toggled: expanded={}", index, state.is_expanded);
                if let Some(s) = self.section_states.get_mut(index) {
                    *s = state;
                }
            }
            Message::AccordionToggled(index, state) => {
                log::info!(
                    "Accordion {} toggled: expanded={}",
                    index,
                    state.is_expanded
                );
                // In accordion mode, close all others when one is opened
                if state.is_expanded {
                    for (i, s) in self.accordion_states.iter_mut().enumerate() {
                        if i == index {
                            *s = state.clone();
                        } else {
                            s.is_expanded = false;
                        }
                    }
                } else if let Some(s) = self.accordion_states.get_mut(index) {
                    *s = state;
                }
            }
            Message::NestedOuterToggled(state) => {
                log::info!("Nested outer toggled: expanded={}", state.is_expanded);
                self.nested_outer = state;
            }
            Message::NestedInnerToggled(state) => {
                log::info!("Nested inner toggled: expanded={}", state.is_expanded);
                self.nested_inner = state;
            }
            Message::ScrollableSectionToggled(state) => {
                log::info!("Scrollable section toggled: expanded={}", state.is_expanded);
                self.scrollable_section = state;
            }
            Message::ButtonClicked => {
                self.click_count += 1;
                log::info!("Button clicked! Count: {}", self.click_count);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default()
        .title("Collapsible Widget Demo")
        .size(800, 700);

    hvat_ui::run_with_settings(CollapsibleDemo::new(), settings)
}
