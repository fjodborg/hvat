//! Demo showcasing input widgets: Slider, TextInput, NumberInput

use crate::element::Element;
use crate::layout::Length;
use crate::prelude::*;
use crate::state::{NumberInputState, SliderState, TextInputState};

/// Input widgets demo state
pub struct InputWidgetsDemo {
    /// Basic slider state
    pub slider_value: SliderState,
    /// Slider with input state
    pub slider_input_value: SliderState,
    /// Stepped slider state
    pub stepped_slider_value: SliderState,
    /// Text input state
    pub text_input_state: TextInputState,
    /// Text input value
    pub text_value: String,
    /// Number input state
    pub number_input_state: NumberInputState,
    /// Second number input state (no buttons)
    pub number_input_state2: NumberInputState,
}

/// Input widgets demo messages
#[derive(Clone)]
pub enum InputWidgetsMessage {
    SliderChanged(SliderState),
    SliderInputChanged(SliderState),
    SteppedSliderChanged(SliderState),
    TextInputChanged(String, TextInputState),
    TextInputSubmitted(String),
    NumberInputChanged(f32, NumberInputState),
    NumberInput2Changed(f32, NumberInputState),
}

impl Default for InputWidgetsDemo {
    fn default() -> Self {
        Self::new()
    }
}

impl InputWidgetsDemo {
    pub fn new() -> Self {
        Self {
            slider_value: SliderState::new(50.0),
            slider_input_value: SliderState::new(25.0),
            stepped_slider_value: SliderState::new(5.0),
            text_input_state: TextInputState::new(),
            text_value: String::new(),
            number_input_state: NumberInputState::new(42.0),
            number_input_state2: NumberInputState::new(3.14),
        }
    }

    pub fn view<M: Clone + 'static>(
        &self,
        wrap: impl Fn(InputWidgetsMessage) -> M + Clone + 'static,
    ) -> Element<M> {
        // Clone state values needed inside closures
        let slider_state = self.slider_value.clone();
        let slider_value = self.slider_value.value;
        let slider_input_state = self.slider_input_value.clone();
        let slider_input_value = self.slider_input_value.value;
        let stepped_slider_state = self.stepped_slider_value.clone();
        let stepped_slider_value = self.stepped_slider_value.value;
        let text_input_state = self.text_input_state.clone();
        let text_value = self.text_value.clone();
        let number_input_state = self.number_input_state.clone();
        let number_input_state2 = self.number_input_state2.clone();

        // Clone wrap for each widget that needs it
        let wrap_slider = wrap.clone();
        let wrap_slider_input = wrap.clone();
        let wrap_stepped = wrap.clone();
        let wrap_text_submit = wrap.clone();
        let wrap_text_change = wrap.clone();
        let wrap_number1 = wrap.clone();
        let wrap_number2 = wrap.clone();

        col(move |c| {
            c.text("Input Widgets Demo");
            c.text_sized("Demonstrates slider, text input, and number input widgets", 12.0);
            c.text("");

            // Basic Slider
            c.text("Basic Slider (0-100):");
            c.row(|r| {
                r.slider(0.0, 100.0)
                    .state(&slider_state)
                    .width(Length::Fixed(300.0))
                    .on_change({
                        let w = wrap_slider.clone();
                        move |s| w(InputWidgetsMessage::SliderChanged(s))
                    })
                    .build();
                r.text(format!("Value: {:.1}", slider_value));
            });
            c.text("");

            // Slider with editable input
            c.text("Slider with Editable Input (0-100):");
            c.row(|r| {
                r.slider(0.0, 100.0)
                    .state(&slider_input_state)
                    .show_input(true)
                    .width(Length::Fixed(300.0))
                    .on_change({
                        let w = wrap_slider_input.clone();
                        move |s| w(InputWidgetsMessage::SliderInputChanged(s))
                    })
                    .build();
                r.text(format!("Value: {:.1}", slider_input_value));
            });
            c.text("");

            // Stepped slider with value label
            c.text("Stepped Slider (0-10, step=1) with Value Label:");
            c.row(|r| {
                r.slider(0.0, 10.0)
                    .state(&stepped_slider_state)
                    .step(1.0)
                    .show_value(true)
                    .show_input(true)
                    .width(Length::Fixed(300.0))
                    .on_change({
                        let w = wrap_stepped.clone();
                        move |s| w(InputWidgetsMessage::SteppedSliderChanged(s))
                    })
                    .build();
                r.text(format!("Value: {}", stepped_slider_value as i32));
            });
            c.text("");

            // Text Input
            c.text("Text Input:");
            c.row(|r| {
                r.text_input()
                    .value(&text_value)
                    .placeholder("Enter some text...")
                    .state(&text_input_state)
                    .width(Length::Fixed(300.0))
                    .on_submit({
                        let w = wrap_text_submit.clone();
                        move |s| w(InputWidgetsMessage::TextInputSubmitted(s))
                    })
                    .on_change({
                        let w = wrap_text_change.clone();
                        move |s, state| w(InputWidgetsMessage::TextInputChanged(s, state))
                    })
                    .build();
            });
            c.text(format!("Entered: \"{}\"", text_value));
            c.text("");

            // Number Input with buttons
            c.text("Number Input with Buttons (0-100, step=1):");
            c.row(|r| {
                r.number_input()
                    .state(&number_input_state)
                    .range(0.0, 100.0)
                    .step(1.0)
                    .width(Length::Fixed(120.0))
                    .on_change({
                        let w = wrap_number1.clone();
                        move |v, s| w(InputWidgetsMessage::NumberInputChanged(v, s))
                    })
                    .build();
                r.text("Use +/- buttons, arrow keys, or scroll");
            });
            c.text("");

            // Number Input without buttons
            c.text("Number Input without Buttons (decimal, step=0.1):");
            c.row(|r| {
                r.number_input()
                    .state(&number_input_state2)
                    .step(0.1)
                    .show_buttons(false)
                    .width(Length::Fixed(100.0))
                    .on_change({
                        let w = wrap_number2.clone();
                        move |v, s| w(InputWidgetsMessage::NumberInput2Changed(v, s))
                    })
                    .build();
                r.text("Type directly or use arrow keys");
            });
            c.text("");

            // Instructions
            c.text_sized("Controls:", 14.0);
            c.text_sized("• Slider: Click/drag track or thumb, scroll wheel, arrow keys when hovered", 11.0);
            c.text_sized("• Input field: Click to focus, type value, Enter/Escape to confirm", 11.0);
            c.text_sized("• Number input: Click +/- buttons, Up/Down arrows, scroll wheel", 11.0);
            c.text("");
            c.text_sized("Focus behavior:", 14.0);
            c.text_sized("• Click outside any input to blur (deselect) it", 11.0);
            c.text_sized("• Clicking +/- buttons blurs the number input field", 11.0);
        })
    }

    pub fn update(&mut self, message: InputWidgetsMessage) {
        match message {
            InputWidgetsMessage::SliderChanged(state) => {
                self.slider_value = state;
                log::info!("Basic slider: {:.1}", self.slider_value.value);
            }
            InputWidgetsMessage::SliderInputChanged(state) => {
                self.slider_input_value = state;
                log::info!("Slider with input: {:.1}", self.slider_input_value.value);
            }
            InputWidgetsMessage::SteppedSliderChanged(state) => {
                self.stepped_slider_value = state;
                log::info!("Stepped slider: {}", self.stepped_slider_value.value as i32);
            }
            InputWidgetsMessage::TextInputChanged(value, state) => {
                self.text_value = value;
                self.text_input_state = state;
                log::debug!("Text input changed: '{}'", self.text_value);
            }
            InputWidgetsMessage::TextInputSubmitted(value) => {
                log::info!("Text input submitted: '{}'", value);
            }
            InputWidgetsMessage::NumberInputChanged(value, state) => {
                self.number_input_state = state;
                log::info!("Number input 1: {}", value);
            }
            InputWidgetsMessage::NumberInput2Changed(value, state) => {
                self.number_input_state2 = state;
                log::info!("Number input 2: {}", value);
            }
        }
    }
}
