//! Builder context for constructing widget trees

use crate::element::Element;
use crate::layout::Length;
use crate::renderer::TextureId;
use crate::state::{NumberInputState, SliderState, TextInputState};
use crate::widgets::{
    AnnotationOverlay, Button, Column, ImageClick, ImageViewer, NumberInput, Row, Slider, Text,
    TextInput,
};

/// Context for building widget trees using a closure-based API
///
/// This is passed to view functions to build the UI declaratively.
pub struct Context<M> {
    children: Vec<Element<M>>,
}

impl<M: 'static> Context<M> {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Add a text widget
    pub fn text(&mut self, content: impl Into<String>) -> &mut Self {
        self.children.push(Element::new(Text::new(content)));
        self
    }

    /// Add a text widget with custom size
    pub fn text_sized(&mut self, content: impl Into<String>, size: f32) -> &mut Self {
        self.children.push(Element::new(Text::new(content).size(size)));
        self
    }

    /// Add a button widget
    pub fn button(&mut self, label: impl Into<String>) -> ButtonBuilder<M> {
        ButtonBuilder {
            ctx: self,
            button: Button::new(label),
        }
    }

    /// Add a row of widgets
    pub fn row(&mut self, builder: impl FnOnce(&mut Context<M>)) -> &mut Self {
        let mut ctx = Context::new();
        builder(&mut ctx);
        self.children.push(Element::new(Row::new(ctx.children)));
        self
    }

    /// Add a column of widgets
    pub fn col(&mut self, builder: impl FnOnce(&mut Context<M>)) -> &mut Self {
        let mut ctx = Context::new();
        builder(&mut ctx);
        self.children.push(Element::new(Column::new(ctx.children)));
        self
    }

    /// Add an image viewer widget with a texture
    pub fn image_viewer(
        &mut self,
        texture_id: TextureId,
        width: u32,
        height: u32,
    ) -> ImageViewerBuilder<M> {
        ImageViewerBuilder {
            ctx: self,
            viewer: ImageViewer::new(texture_id, width, height),
        }
    }

    /// Add an empty image viewer (no texture yet)
    pub fn image_viewer_empty(&mut self) -> ImageViewerBuilder<M> {
        ImageViewerBuilder {
            ctx: self,
            viewer: ImageViewer::empty(),
        }
    }

    /// Add a custom element directly
    pub fn add(&mut self, element: Element<M>) -> &mut Self {
        self.children.push(element);
        self
    }

    /// Add a slider widget
    pub fn slider(&mut self, min: f32, max: f32) -> SliderBuilder<M> {
        SliderBuilder {
            ctx: self,
            slider: Slider::new(min, max),
        }
    }

    /// Add a text input widget
    pub fn text_input(&mut self) -> TextInputBuilder<M> {
        TextInputBuilder {
            ctx: self,
            input: TextInput::new(),
        }
    }

    /// Add a number input widget
    pub fn number_input(&mut self) -> NumberInputBuilder<M> {
        NumberInputBuilder {
            ctx: self,
            input: NumberInput::new(),
        }
    }

    /// Take the built children
    pub fn take(self) -> Vec<Element<M>> {
        self.children
    }
}

impl<M: 'static> Default for Context<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for button widgets
pub struct ButtonBuilder<'a, M> {
    ctx: &'a mut Context<M>,
    button: Button<M>,
}

impl<'a, M: Clone + 'static> ButtonBuilder<'a, M> {
    /// Set the click handler
    pub fn on_click(mut self, message: M) -> &'a mut Context<M> {
        self.button = self.button.on_click(message);
        self.ctx.children.push(Element::new(self.button));
        self.ctx
    }

    /// Set button width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.button = self.button.width(width);
        self
    }

    /// Set button height
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.button = self.button.height(height);
        self
    }

}

impl<'a, M: 'static> ButtonBuilder<'a, M> {
    /// Finish without a click handler (button won't do anything)
    pub fn build(self) -> &'a mut Context<M> {
        // Note: Without Clone bound, we can't make Button a Widget
        // So this is essentially a no-op for non-Clone M
        self.ctx
    }
}

/// Builder for image viewer widgets
pub struct ImageViewerBuilder<'a, M> {
    ctx: &'a mut Context<M>,
    viewer: ImageViewer<M>,
}

impl<'a, M: 'static + Clone> ImageViewerBuilder<'a, M> {
    /// Set the viewer state
    pub fn state(mut self, state: &crate::state::ImageViewerState) -> Self {
        self.viewer = self.viewer.state(state);
        self
    }

    /// Set the change handler
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(crate::state::ImageViewerState) -> M + 'static,
    {
        self.viewer = self.viewer.on_change(handler);
        self
    }

    /// Enable/disable pan
    pub fn pannable(mut self, enabled: bool) -> Self {
        self.viewer = self.viewer.pannable(enabled);
        self
    }

    /// Enable/disable zoom
    pub fn zoomable(mut self, enabled: bool) -> Self {
        self.viewer = self.viewer.zoomable(enabled);
        self
    }

    /// Show/hide built-in controls
    pub fn show_controls(mut self, show: bool) -> Self {
        self.viewer = self.viewer.show_controls(show);
        self
    }

    /// Set width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.viewer = self.viewer.width(width);
        self
    }

    /// Set height
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.viewer = self.viewer.height(height);
        self
    }

    /// Enable annotation mode (left click for drawing instead of panning)
    pub fn annotation_mode(mut self, enabled: bool) -> Self {
        self.viewer = self.viewer.annotation_mode(enabled);
        self
    }

    /// Set the click handler for annotation tools
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(ImageClick) -> M + 'static,
    {
        self.viewer = self.viewer.on_click(handler);
        self
    }

    /// Set annotation overlays to draw
    pub fn overlays(mut self, overlays: Vec<AnnotationOverlay>) -> Self {
        self.viewer = self.viewer.overlays(overlays);
        self
    }

    /// Finish building
    pub fn build(self) -> &'a mut Context<M> {
        self.ctx.children.push(Element::new(self.viewer));
        self.ctx
    }
}

/// Builder for slider widgets
pub struct SliderBuilder<'a, M> {
    ctx: &'a mut Context<M>,
    slider: Slider<M>,
}

impl<'a, M: Clone + 'static> SliderBuilder<'a, M> {
    /// Set the state
    pub fn state(mut self, state: &SliderState) -> Self {
        self.slider = self.slider.state(state);
        self
    }

    /// Set the step size
    pub fn step(mut self, step: f32) -> Self {
        self.slider = self.slider.step(step);
        self
    }

    /// Show value label above thumb
    pub fn show_value(mut self, show: bool) -> Self {
        self.slider = self.slider.show_value(show);
        self
    }

    /// Show editable input field next to slider
    pub fn show_input(mut self, show: bool) -> Self {
        self.slider = self.slider.show_input(show);
        self
    }

    /// Set width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.slider = self.slider.width(width);
        self
    }

    /// Set the change handler and finish building
    pub fn on_change<F>(mut self, handler: F) -> &'a mut Context<M>
    where
        F: Fn(SliderState) -> M + 'static,
    {
        self.slider = self.slider.on_change(handler);
        self.ctx.children.push(Element::new(self.slider));
        self.ctx
    }

    /// Finish building without handler
    pub fn build(self) -> &'a mut Context<M> {
        self.ctx.children.push(Element::new(self.slider));
        self.ctx
    }
}

/// Builder for text input widgets
pub struct TextInputBuilder<'a, M> {
    ctx: &'a mut Context<M>,
    input: TextInput<M>,
}

impl<'a, M: Clone + 'static> TextInputBuilder<'a, M> {
    /// Set the current value
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.input = self.input.value(value);
        self
    }

    /// Set the placeholder text
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.input = self.input.placeholder(placeholder);
        self
    }

    /// Set the state
    pub fn state(mut self, state: &TextInputState) -> Self {
        self.input = self.input.state(state);
        self
    }

    /// Set width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.input = self.input.width(width);
        self
    }

    /// Set the change handler and finish building
    pub fn on_change<F>(mut self, handler: F) -> &'a mut Context<M>
    where
        F: Fn(String, TextInputState) -> M + 'static,
    {
        self.input = self.input.on_change(handler);
        self.ctx.children.push(Element::new(self.input));
        self.ctx
    }

    /// Set the submit handler
    pub fn on_submit<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.input = self.input.on_submit(handler);
        self
    }

    /// Finish building without handler
    pub fn build(self) -> &'a mut Context<M> {
        self.ctx.children.push(Element::new(self.input));
        self.ctx
    }
}

/// Builder for number input widgets
pub struct NumberInputBuilder<'a, M> {
    ctx: &'a mut Context<M>,
    input: NumberInput<M>,
}

impl<'a, M: Clone + 'static> NumberInputBuilder<'a, M> {
    /// Set the state
    pub fn state(mut self, state: &NumberInputState) -> Self {
        self.input = self.input.state(state);
        self
    }

    /// Set minimum value
    pub fn min(mut self, min: f32) -> Self {
        self.input = self.input.min(min);
        self
    }

    /// Set maximum value
    pub fn max(mut self, max: f32) -> Self {
        self.input = self.input.max(max);
        self
    }

    /// Set range (min and max)
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.input = self.input.range(min, max);
        self
    }

    /// Set step size
    pub fn step(mut self, step: f32) -> Self {
        self.input = self.input.step(step);
        self
    }

    /// Show/hide increment/decrement buttons
    pub fn show_buttons(mut self, show: bool) -> Self {
        self.input = self.input.show_buttons(show);
        self
    }

    /// Set width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.input = self.input.width(width);
        self
    }

    /// Set the change handler and finish building
    pub fn on_change<F>(mut self, handler: F) -> &'a mut Context<M>
    where
        F: Fn(f32, NumberInputState) -> M + 'static,
    {
        self.input = self.input.on_change(handler);
        self.ctx.children.push(Element::new(self.input));
        self.ctx
    }

    /// Finish building without handler
    pub fn build(self) -> &'a mut Context<M> {
        self.ctx.children.push(Element::new(self.input));
        self.ctx
    }
}
