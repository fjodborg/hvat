//! Collapsible widget demo

use crate::element::Element;
use crate::prelude::*;
use crate::renderer::Color;
use crate::state::CollapsibleState;

/// Collapsible demo state
pub struct CollapsibleDemo {
    pub section_states: Vec<CollapsibleState>,
    pub accordion_states: Vec<CollapsibleState>,
    pub nested_outer: CollapsibleState,
    pub nested_inner: CollapsibleState,
    pub scrollable_section: CollapsibleState,
    pub click_count: u32,
}

/// Collapsible demo messages
#[derive(Clone)]
pub enum CollapsibleMessage {
    SectionToggled(usize, CollapsibleState),
    AccordionToggled(usize, CollapsibleState),
    NestedOuterToggled(CollapsibleState),
    NestedInnerToggled(CollapsibleState),
    ScrollableSectionToggled(CollapsibleState),
    ButtonClicked,
}

impl Default for CollapsibleDemo {
    fn default() -> Self {
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

impl CollapsibleDemo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view<M: Clone + 'static>(
        &self,
        wrap: impl Fn(CollapsibleMessage) -> M + Clone + 'static,
    ) -> Element<M> {
        let section_states = self.section_states.clone();
        let accordion_states = self.accordion_states.clone();
        let nested_outer = self.nested_outer.clone();
        let nested_inner = self.nested_inner.clone();
        let scrollable_section = self.scrollable_section.clone();
        let click_count = self.click_count;

        col(move |c| {
            c.text("Collapsible Widget Demo");
            c.text("");
            c.text("-- Basic Collapsible Sections --");

            // Basic collapsible sections
            for (i, state) in section_states.iter().enumerate() {
                let section_title = format!(
                    "Section {} - Click to {}",
                    i + 1,
                    if state.is_expanded {
                        "collapse"
                    } else {
                        "expand"
                    }
                );
                let wrap_section = wrap.clone();
                let state_clone = state.clone();
                c.add(Element::new(
                    collapsible(section_title)
                        .state(&state_clone)
                        .on_toggle(move |s| wrap_section(CollapsibleMessage::SectionToggled(i, s)))
                        .content(|content| {
                            content.text(format!("This is the content of section {}.", i + 1));
                            content.text("You can put any widgets here.");
                        }),
                ));
            }

            c.text("");
            c.text("-- Accordion Mode (one at a time) --");

            // Accordion sections
            let accordion_titles = ["First Panel", "Second Panel", "Third Panel"];
            for (i, (state, title)) in accordion_states
                .iter()
                .zip(accordion_titles.iter())
                .enumerate()
            {
                let wrap_accordion = wrap.clone();
                let state_clone = state.clone();
                c.add(Element::new(
                    collapsible(*title)
                        .state(&state_clone)
                        .header_color(Color::rgba(0.2, 0.15, 0.25, 1.0))
                        .on_toggle(move |s| {
                            wrap_accordion(CollapsibleMessage::AccordionToggled(i, s))
                        })
                        .content(|content| {
                            content.text(format!("Content for: {}", title));
                            content.text("In accordion mode, opening one panel closes the others.");
                        }),
                ));
            }

            c.text("");
            c.text("-- Scrollable Content (max_height = 120px) --");

            // Scrollable collapsible section
            let wrap_scrollable = wrap.clone();
            c.add(Element::new(
                collapsible("Scrollable Section - Use mouse wheel to scroll")
                    .state(&scrollable_section)
                    .header_color(Color::rgba(0.15, 0.25, 0.2, 1.0))
                    .max_height(120.0)
                    .on_toggle(move |s| {
                        wrap_scrollable(CollapsibleMessage::ScrollableSectionToggled(s))
                    })
                    .content(|content| {
                        for i in 1..=20 {
                            content.text(format!(
                                "Line {} - This content scrolls when it exceeds max_height",
                                i
                            ));
                        }
                    }),
            ));

            c.text("");
            c.text("-- Nested Collapsibles --");

            // Nested collapsibles
            let wrap_outer = wrap.clone();
            let wrap_inner = wrap.clone();
            let wrap_button = wrap.clone();
            c.add(Element::new(
                collapsible("Outer Section")
                    .state(&nested_outer)
                    .on_toggle(move |s| wrap_outer(CollapsibleMessage::NestedOuterToggled(s)))
                    .content(move |outer| {
                        outer.text("This is the outer content.");
                        outer.text("");
                        let wrap_inner_clone = wrap_inner.clone();
                        let wrap_button_clone = wrap_button.clone();
                        outer.add(Element::new(
                            collapsible("Inner Section")
                                .state(&nested_inner)
                                .header_color(Color::rgba(0.15, 0.2, 0.25, 1.0))
                                .on_toggle(move |s| {
                                    wrap_inner_clone(CollapsibleMessage::NestedInnerToggled(s))
                                })
                                .content(move |inner| {
                                    inner.text("This is nested inside the outer section.");
                                    inner.text(format!("Button clicks: {}", click_count));
                                    inner.button("Click Me!").on_click(wrap_button_clone(
                                        CollapsibleMessage::ButtonClicked,
                                    ));
                                }),
                        ));
                    }),
            ));

            c.text("");
            c.text("Controls: Click header to toggle | Mouse wheel to scroll");
        })
    }

    pub fn update(&mut self, message: CollapsibleMessage) {
        match message {
            CollapsibleMessage::SectionToggled(index, state) => {
                log::info!("Section {} toggled: expanded={}", index, state.is_expanded);
                if let Some(s) = self.section_states.get_mut(index) {
                    *s = state;
                }
            }
            CollapsibleMessage::AccordionToggled(index, state) => {
                log::info!(
                    "Accordion {} toggled: expanded={}",
                    index,
                    state.is_expanded
                );
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
            CollapsibleMessage::NestedOuterToggled(state) => {
                log::info!("Nested outer toggled: expanded={}", state.is_expanded);
                self.nested_outer = state;
            }
            CollapsibleMessage::NestedInnerToggled(state) => {
                log::info!("Nested inner toggled: expanded={}", state.is_expanded);
                self.nested_inner = state;
            }
            CollapsibleMessage::ScrollableSectionToggled(state) => {
                log::info!("Scrollable section toggled: expanded={}", state.is_expanded);
                self.scrollable_section = state;
            }
            CollapsibleMessage::ButtonClicked => {
                self.click_count += 1;
                log::info!("Button clicked! Count: {}", self.click_count);
            }
        }
    }
}
