//! Builder context for constructing widget trees

use crate::element::Element;
use crate::layout::Length;
use crate::widgets::{Button, Column, ImageViewer, Row, Text};
use hvat_gpu::Texture;

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

    /// Add an image viewer widget
    pub fn image_viewer(&mut self, texture: &Texture) -> ImageViewerBuilder<M> {
        ImageViewerBuilder {
            ctx: self,
            viewer: ImageViewer::new(texture),
        }
    }

    /// Add a custom element directly
    pub fn add(&mut self, element: Element<M>) -> &mut Self {
        self.children.push(element);
        self
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

    /// Finish building
    pub fn build(self) -> &'a mut Context<M> {
        self.ctx.children.push(Element::new(self.viewer));
        self.ctx
    }
}
