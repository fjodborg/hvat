//! Right sidebar UI component.

use std::rc::Rc;

use hvat_ui::prelude::*;
use hvat_ui::{Collapsible, Column, Context, Element, Scrollable, ScrollDirection, ScrollbarVisibility};

use crate::app::HvatApp;
use crate::constants::SIDEBAR_WIDTH;
use crate::message::Message;

impl HvatApp {
    /// Build the right sidebar with band selection and image adjustments.
    pub(crate) fn build_right_sidebar(&self) -> Element<Message> {
        let band_state = self.band_selection_collapsed.clone();
        let adj_state = self.adjustments_collapsed.clone();
        let scroll_state = self.right_scroll_state.clone();

        let red_slider = self.red_band_slider.clone();
        let green_slider = self.green_band_slider.clone();
        let blue_slider = self.blue_band_slider.clone();

        let brightness_slider = self.brightness_slider.clone();
        let contrast_slider = self.contrast_slider.clone();
        let gamma_slider = self.gamma_slider.clone();
        let hue_slider = self.hue_slider.clone();

        let max_band = (self.num_bands - 1) as f32;

        let slider_undo_snapshot = self.snapshot();
        let undo_stack = Rc::clone(&self.undo_stack);
        let undo_ctx = UndoContext::new(undo_stack, slider_undo_snapshot);

        let mut sidebar_ctx = Context::new();

        sidebar_ctx.text_sized("Image Controls", 14.0);
        sidebar_ctx.text("");

        // Band Selection Collapsible
        let band_s = band_state.clone();
        let collapsible_bands = Collapsible::new("Band Selection")
            .state(&band_s)
            .width(Length::Fill(1.0))
            .on_toggle(Message::BandSelectionToggled)
            .content(|c| {
                c.text_sized("Red Channel", 12.0);
                c.slider(0.0, max_band)
                    .state(&red_slider)
                    .step(1.0)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
                    .on_change(Message::RedBandChanged)
                    .on_undo_point(undo_ctx.callback_with_label("red_band"))
                    .build();

                c.text("");
                c.text_sized("Green Channel", 12.0);
                c.slider(0.0, max_band)
                    .state(&green_slider)
                    .step(1.0)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
                    .on_change(Message::GreenBandChanged)
                    .on_undo_point(undo_ctx.callback_with_label("green_band"))
                    .build();

                c.text("");
                c.text_sized("Blue Channel", 12.0);
                c.slider(0.0, max_band)
                    .state(&blue_slider)
                    .step(1.0)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
                    .on_change(Message::BlueBandChanged)
                    .on_undo_point(undo_ctx.callback_with_label("blue_band"))
                    .build();
            });
        sidebar_ctx.add(Element::new(collapsible_bands));

        // Image Adjustments Collapsible
        let adj_s = adj_state.clone();
        let collapsible_adj = Collapsible::new("Image Adjustments")
            .state(&adj_s)
            .width(Length::Fill(1.0))
            .on_toggle(Message::AdjustmentsToggled)
            .content(|c| {
                c.text_sized(format!("Brightness: {:.2}", brightness_slider.value), 12.0);
                c.slider(-1.0, 1.0)
                    .state(&brightness_slider)
                    .step(0.01)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
                    .on_change(Message::BrightnessChanged)
                    .on_undo_point(undo_ctx.callback_with_label("brightness"))
                    .build();

                c.text("");
                c.text_sized(format!("Contrast: {:.2}", contrast_slider.value), 12.0);
                c.slider(0.1, 3.0)
                    .state(&contrast_slider)
                    .step(0.01)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
                    .on_change(Message::ContrastChanged)
                    .on_undo_point(undo_ctx.callback_with_label("contrast"))
                    .build();

                c.text("");
                c.text_sized(format!("Gamma: {:.2}", gamma_slider.value), 12.0);
                c.slider(0.1, 3.0)
                    .state(&gamma_slider)
                    .step(0.01)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
                    .on_change(Message::GammaChanged)
                    .on_undo_point(undo_ctx.callback_with_label("gamma"))
                    .build();

                c.text("");
                c.text_sized(format!("Hue: {:.0}°", hue_slider.value), 12.0);
                c.slider(0.0, 360.0)
                    .state(&hue_slider)
                    .step(1.0)
                    .show_input(true)
                    .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
                    .on_change(Message::HueChanged)
                    .on_undo_point(undo_ctx.callback_with_label("hue"))
                    .build();

                c.text("");
                c.button("Reset Adjustments")
                    .width(Length::Fixed(SIDEBAR_WIDTH - 20.0))
                    .on_click(Message::ResetAdjustments);
            });
        sidebar_ctx.add(Element::new(collapsible_adj));

        // Keyboard shortcuts info
        sidebar_ctx.text("");
        sidebar_ctx.text("────────────────────");
        sidebar_ctx.text_sized("Keyboard shortcuts:", 11.0);
        sidebar_ctx.text_sized("Ctrl+Z - Undo", 10.0);
        sidebar_ctx.text_sized("Ctrl+Y - Redo", 10.0);
        sidebar_ctx.text_sized("0 - Zoom to 100%", 10.0);
        sidebar_ctx.text_sized("F - Fit to window", 10.0);
        sidebar_ctx.text_sized("+/- - Zoom in/out", 10.0);

        // Wrap in scrollable
        let content = Element::new(Column::new(sidebar_ctx.take()));
        let scrollable = Scrollable::new(content)
            .state(&scroll_state)
            .direction(ScrollDirection::Vertical)
            .scrollbar_visibility(ScrollbarVisibility::Auto)
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill(1.0))
            .on_scroll(Message::RightScrolled);

        Element::new(scrollable)
    }
}
