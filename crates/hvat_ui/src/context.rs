//! Builder context for constructing widget trees

use crate::element::Element;
use crate::layout::{Alignment, Length, Padding};
use crate::renderer::TextureId;
use crate::state::{InteractionMode, NumberInputState, SliderState, TextInputState};
use crate::widgets::{
    AnnotationOverlay, Button, Column, ImagePointerEvent, ImageViewer, NumberInput, Row, Slider,
    Text, TextInput,
};
use hvat_gpu::ImageAdjustments;

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

    /// Add a text widget with builder pattern
    ///
    /// # Examples
    /// ```ignore
    /// ctx.text("Simple text");
    /// ctx.text("Centered").align(Alignment::Center);
    /// ctx.text("Large centered").size(18.0).align(Alignment::Center);
    /// ```
    pub fn text(&mut self, content: impl Into<String>) -> TextDisplayBuilder<'_, M> {
        TextDisplayBuilder {
            ctx: self,
            text: Some(Text::new(content)),
        }
    }

    /// Add a button widget
    pub fn button(&mut self, label: impl Into<String>) -> ButtonBuilder<'_, M> {
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
    ) -> ImageViewerBuilder<'_, M> {
        ImageViewerBuilder {
            ctx: self,
            viewer: ImageViewer::new(texture_id, width, height),
        }
    }

    /// Add an empty image viewer (no texture yet)
    pub fn image_viewer_empty(&mut self) -> ImageViewerBuilder<'_, M> {
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
    pub fn slider(&mut self, min: f32, max: f32) -> SliderBuilder<'_, M> {
        SliderBuilder {
            ctx: self,
            slider: Slider::new(min, max),
        }
    }

    /// Add a text input widget
    pub fn text_input(&mut self) -> TextInputBuilder<'_, M> {
        TextInputBuilder {
            ctx: self,
            input: TextInput::new(),
        }
    }

    /// Add a number input widget
    pub fn number_input(&mut self) -> NumberInputBuilder<'_, M> {
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

/// Builder for text display widgets
///
/// Text is automatically added to the context when this builder is dropped,
/// so you don't need to call `.build()` explicitly (though you can).
///
/// # Examples
/// ```ignore
/// ctx.text("Simple text");  // Added on drop
/// ctx.text("Centered").align(Alignment::Center);  // Fluent API
/// ctx.text("Custom").size(18.0).align(Alignment::Center);
/// ```
// Note: TextDisplayBuilder uses Drop to automatically add text, so #[must_use] is not appropriate
pub struct TextDisplayBuilder<'a, M: 'static> {
    ctx: &'a mut Context<M>,
    text: Option<Text>,
}

impl<'a, M: 'static> TextDisplayBuilder<'a, M> {
    /// Set the font size
    pub fn size(mut self, size: f32) -> Self {
        if let Some(text) = self.text.take() {
            self.text = Some(text.size(size));
        }
        self
    }

    /// Set the text alignment
    ///
    /// Note: For `Alignment::Center` to work visibly, the text widget needs
    /// width to center within. This method automatically sets `Fill` width
    /// when centering.
    pub fn align(mut self, alignment: Alignment) -> Self {
        if let Some(mut text) = self.text.take() {
            text = text.text_align(alignment);
            // For centering to work, we need the text to fill available width
            if alignment == Alignment::Center {
                text = text.width(Length::Fill(1.0));
            }
            self.text = Some(text);
        }
        self
    }

    /// Set the width explicitly
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        if let Some(text) = self.text.take() {
            self.text = Some(text.width(width));
        }
        self
    }

    /// Enable word wrapping
    ///
    /// When enabled, text will wrap to the next line if any word would be clipped.
    /// For proper wrapping, also set a width constraint (e.g., `Length::Fill`).
    pub fn wrap(mut self, wrap: bool) -> Self {
        if let Some(text) = self.text.take() {
            self.text = Some(text.wrap(wrap));
        }
        self
    }

    // No explicit build() needed - Drop handles adding to context
    // The builder is consumed when it goes out of scope
}

impl<'a, M: 'static> Drop for TextDisplayBuilder<'a, M> {
    fn drop(&mut self) {
        // Take the text out and add it to the context
        if let Some(text) = self.text.take() {
            self.ctx.children.push(Element::new(text));
        }
    }
}

/// Builder for button widgets
#[must_use = "ButtonBuilder does nothing unless .on_click() or .build() is called"]
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

    /// Set button padding
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.button = self.button.padding(padding);
        self
    }

    /// Set button margin (space around the button)
    pub fn margin(mut self, margin: impl Into<Padding>) -> Self {
        self.button = self.button.margin(margin);
        self
    }

    /// Set button style (Normal or Text)
    pub fn style(mut self, style: crate::widgets::ButtonStyle) -> Self {
        self.button = self.button.style(style);
        self
    }

    /// Set horizontal text alignment within the button
    pub fn text_align(mut self, align: Alignment) -> Self {
        self.button = self.button.text_align(align);
        self
    }

    /// Set a custom background color
    ///
    /// When set, this overrides the style-based background colors.
    /// The color will be slightly lightened on hover and darkened on press.
    pub fn background_color(mut self, color: crate::renderer::Color) -> Self {
        self.button = self.button.background_color(color);
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
#[must_use = "ImageViewerBuilder does nothing unless .build() is called"]
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

    /// Set the interaction mode (View or Annotate)
    pub fn interaction_mode(mut self, mode: InteractionMode) -> Self {
        self.viewer = self.viewer.interaction_mode(mode);
        self
    }

    /// Set the pointer event handler for annotation tools
    pub fn on_pointer<F>(mut self, handler: F) -> Self
    where
        F: Fn(ImagePointerEvent) -> M + 'static,
    {
        self.viewer = self.viewer.on_pointer(handler);
        self
    }

    /// Set annotation overlays to draw
    pub fn overlays(mut self, overlays: Vec<AnnotationOverlay>) -> Self {
        self.viewer = self.viewer.overlays(overlays);
        self
    }

    /// Set image adjustments (brightness, contrast, gamma, hue shift)
    ///
    /// These adjustments are applied on the GPU for real-time performance.
    pub fn adjustments(mut self, adjustments: ImageAdjustments) -> Self {
        self.viewer = self.viewer.adjustments(adjustments);
        self
    }

    /// Finish building
    pub fn build(self) -> &'a mut Context<M> {
        self.ctx.children.push(Element::new(self.viewer));
        self.ctx
    }
}

/// Builder for slider widgets
#[must_use = "SliderBuilder does nothing unless .build() is called"]
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

    /// Set the change handler
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(SliderState) -> M + 'static,
    {
        self.slider = self.slider.on_change(handler);
        self
    }

    /// Set the undo point handler (called when drag starts or input field gains focus)
    ///
    /// This is a side-effect callback invoked at the start of an edit operation.
    /// Use it to save an undo snapshot before the edit begins.
    pub fn on_undo_point<F>(mut self, handler: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.slider = self.slider.on_undo_point(handler);
        self
    }

    /// Finish building without handler
    pub fn build(self) -> &'a mut Context<M> {
        self.ctx.children.push(Element::new(self.slider));
        self.ctx
    }
}

/// Builder for text input widgets
#[must_use = "TextInputBuilder does nothing unless .build() is called"]
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

    /// Set the change handler
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(String, TextInputState) -> M + 'static,
    {
        self.input = self.input.on_change(handler);
        self
    }

    /// Set the submit handler
    pub fn on_submit<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.input = self.input.on_submit(handler);
        self
    }

    /// Set the undo point handler (called when input gains focus)
    ///
    /// This is a side-effect callback invoked at the start of an edit operation.
    /// Use it to save an undo snapshot before the edit begins.
    pub fn on_undo_point<F>(mut self, handler: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.input = self.input.on_undo_point(handler);
        self
    }

    /// Finish building without handler
    pub fn build(self) -> &'a mut Context<M> {
        self.ctx.children.push(Element::new(self.input));
        self.ctx
    }
}

/// Builder for number input widgets
#[must_use = "NumberInputBuilder does nothing unless .build() is called"]
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

    /// Set the change handler
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(f32, NumberInputState) -> M + 'static,
    {
        self.input = self.input.on_change(handler);
        self
    }

    /// Set the undo point handler (called when input gains focus)
    ///
    /// This is a side-effect callback invoked at the start of an edit operation.
    /// Use it to save an undo snapshot before the edit begins.
    pub fn on_undo_point<F>(mut self, handler: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.input = self.input.on_undo_point(handler);
        self
    }

    /// Finish building without handler
    pub fn build(self) -> &'a mut Context<M> {
        self.ctx.children.push(Element::new(self.input));
        self.ctx
    }
}
