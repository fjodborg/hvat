//! Confirmation dialog widget for in-app modal confirmations.
//!
//! A modal dialog that displays a message and offers confirm/cancel options.
//! Used instead of OS-native dialogs for consistent cross-platform UX.

use crate::callback::Callback;
use crate::event::{Event, KeyCode, MouseButton};
use crate::layout::{Bounds, Size};
use crate::renderer::{Color, Renderer};
use crate::theme::current_theme;
use crate::widget::{EventResult, Widget};
use crate::widgets::overlay::OverlayCloseHelper;

/// Corner radius for dialog (0 for sharp corners)
const DIALOG_CORNER_RADIUS: f32 = 0.0;

/// Configuration for the confirm dialog
#[derive(Debug, Clone)]
pub struct ConfirmDialogConfig {
    /// Background color
    pub bg_color: Color,
    /// Text color
    pub text_color: Color,
    /// Secondary text color (for description)
    pub secondary_text_color: Color,
    /// Border color
    pub border_color: Color,
    /// Button background color
    pub button_bg: Color,
    /// Button hover color
    pub button_hover: Color,
    /// Confirm button color (destructive actions)
    pub confirm_button_bg: Color,
    /// Confirm button hover color
    pub confirm_button_hover: Color,
    /// Title font size
    pub title_font_size: f32,
    /// Description font size
    pub description_font_size: f32,
    /// Button font size
    pub button_font_size: f32,
    /// Dialog width
    pub width: f32,
    /// Padding inside dialog
    pub padding: f32,
    /// Button height
    pub button_height: f32,
    /// Button width
    pub button_width: f32,
    /// Spacing between buttons
    pub button_spacing: f32,
    /// Shadow offset
    pub shadow_offset: f32,
    /// Backdrop color (semi-transparent overlay behind dialog)
    pub backdrop_color: Color,
}

impl Default for ConfirmDialogConfig {
    fn default() -> Self {
        Self {
            bg_color: Color::rgba(0.12, 0.12, 0.15, 0.98),
            text_color: Color::TEXT_PRIMARY,
            secondary_text_color: Color::TEXT_SECONDARY,
            border_color: Color::BORDER,
            button_bg: Color::rgba(0.2, 0.2, 0.25, 1.0),
            button_hover: Color::rgba(0.3, 0.3, 0.35, 1.0),
            confirm_button_bg: Color::rgba(0.6, 0.2, 0.2, 1.0),
            confirm_button_hover: Color::rgba(0.7, 0.3, 0.3, 1.0),
            title_font_size: 16.0,
            description_font_size: 14.0,
            button_font_size: 14.0,
            width: 320.0,
            padding: 20.0,
            button_height: 32.0,
            button_width: 80.0,
            button_spacing: 12.0,
            shadow_offset: 4.0,
            backdrop_color: Color::rgba(0.0, 0.0, 0.0, 0.5),
        }
    }
}

/// Button being hovered in the dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoveredButton {
    None,
    Confirm,
    Cancel,
}

/// Confirmation dialog widget
///
/// Displays a modal dialog with a title, optional description,
/// and confirm/cancel buttons. Centered in the viewport.
pub struct ConfirmDialog<M> {
    /// Dialog title
    title: String,
    /// Optional description text
    description: Option<String>,
    /// Confirm button label
    confirm_label: String,
    /// Cancel button label
    cancel_label: String,
    /// Whether the dialog is open
    is_open: bool,
    /// Viewport size for centering
    viewport_size: (f32, f32),
    /// Configuration
    config: ConfirmDialogConfig,
    /// Currently hovered button
    hovered: HoveredButton,
    /// Callback when confirmed
    on_confirm: Callback<(), M>,
    /// Callback when cancelled
    on_cancel: Callback<(), M>,
}

impl<M> Default for ConfirmDialog<M> {
    fn default() -> Self {
        Self {
            title: String::new(),
            description: None,
            confirm_label: "Confirm".to_string(),
            cancel_label: "Cancel".to_string(),
            is_open: false,
            viewport_size: (800.0, 600.0),
            config: ConfirmDialogConfig::default(),
            hovered: HoveredButton::None,
            on_confirm: Callback::none(),
            on_cancel: Callback::none(),
        }
    }
}

impl<M: 'static> ConfirmDialog<M> {
    /// Create a new confirm dialog
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the dialog title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the dialog description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the confirm button label
    pub fn confirm_label(mut self, label: impl Into<String>) -> Self {
        self.confirm_label = label.into();
        self
    }

    /// Set the cancel button label
    pub fn cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = label.into();
        self
    }

    /// Set whether the dialog is open
    pub fn open(mut self, is_open: bool) -> Self {
        self.is_open = is_open;
        self
    }

    /// Set the viewport size for centering
    pub fn viewport_size(mut self, width: f32, height: f32) -> Self {
        self.viewport_size = (width, height);
        self
    }

    /// Set the configuration
    pub fn config(mut self, config: ConfirmDialogConfig) -> Self {
        self.config = config;
        self
    }

    /// Set callback for confirmation
    pub fn on_confirm<F>(mut self, callback: F) -> Self
    where
        F: Fn(()) -> M + 'static,
    {
        self.on_confirm = Callback::new(callback);
        self
    }

    /// Set callback for cancellation
    pub fn on_cancel<F>(mut self, callback: F) -> Self
    where
        F: Fn(()) -> M + 'static,
    {
        self.on_cancel = Callback::new(callback);
        self
    }

    /// Calculate dialog height based on content
    fn calculate_height(&self) -> f32 {
        let mut height = self.config.padding * 2.0; // Top and bottom padding
        height += self.config.title_font_size + 12.0; // Title + spacing

        if self.description.is_some() {
            // Estimate description height (could be multi-line)
            height += self.config.description_font_size + 16.0;
        }

        height += 16.0; // Spacing before buttons
        height += self.config.button_height;

        height
    }

    /// Calculate dialog bounds (centered in viewport)
    fn calculate_bounds(&self) -> Bounds {
        let width = self.config.width;
        let height = self.calculate_height();

        let x = (self.viewport_size.0 - width) / 2.0;
        let y = (self.viewport_size.1 - height) / 2.0;

        Bounds::new(x.max(0.0), y.max(0.0), width, height)
    }

    /// Calculate confirm button bounds
    fn confirm_button_bounds(&self, dialog_bounds: Bounds) -> Bounds {
        let button_area_width = self.config.button_width * 2.0 + self.config.button_spacing;
        let button_x = dialog_bounds.x + (dialog_bounds.width - button_area_width) / 2.0;
        let button_y = dialog_bounds.bottom() - self.config.padding - self.config.button_height;

        Bounds::new(
            button_x,
            button_y,
            self.config.button_width,
            self.config.button_height,
        )
    }

    /// Calculate cancel button bounds
    fn cancel_button_bounds(&self, dialog_bounds: Bounds) -> Bounds {
        let confirm_bounds = self.confirm_button_bounds(dialog_bounds);
        Bounds::new(
            confirm_bounds.right() + self.config.button_spacing,
            confirm_bounds.y,
            self.config.button_width,
            self.config.button_height,
        )
    }
}

impl<M: 'static> Widget<M> for ConfirmDialog<M> {
    fn layout(&mut self, _available: Size) -> Size {
        // Dialog is an overlay, doesn't take up space in layout
        Size::ZERO
    }

    fn has_active_overlay(&self) -> bool {
        self.is_open
    }

    fn capture_bounds(&self, _layout_bounds: Bounds) -> Option<Bounds> {
        if self.is_open {
            // Capture entire viewport (modal backdrop)
            Some(Bounds::new(
                0.0,
                0.0,
                self.viewport_size.0,
                self.viewport_size.1,
            ))
        } else {
            None
        }
    }

    fn draw(&self, renderer: &mut Renderer, _bounds: Bounds) {
        if !self.is_open {
            return;
        }

        let dialog_bounds = self.calculate_bounds();

        // Register overlay for event routing
        renderer.register_overlay(dialog_bounds);

        // Start overlay rendering
        renderer.begin_overlay();

        let theme = current_theme();

        // Draw backdrop (semi-transparent overlay covering entire viewport)
        renderer.fill_rect(
            Bounds::new(0.0, 0.0, self.viewport_size.0, self.viewport_size.1),
            self.config.backdrop_color,
        );

        // Draw shadow and background
        renderer.draw_popup_shadow(dialog_bounds, DIALOG_CORNER_RADIUS);
        renderer.fill_rounded_rect(dialog_bounds, theme.popup_bg, DIALOG_CORNER_RADIUS);
        renderer.stroke_rounded_rect(dialog_bounds, theme.divider, DIALOG_CORNER_RADIUS, 1.0);

        // Draw title
        let title_x = dialog_bounds.x + self.config.padding;
        let title_y = dialog_bounds.y + self.config.padding;
        renderer.text(
            &self.title,
            title_x,
            title_y,
            self.config.title_font_size,
            self.config.text_color,
        );

        // Draw description if present
        if let Some(ref desc) = self.description {
            let desc_y = title_y + self.config.title_font_size + 12.0;
            renderer.text(
                desc,
                title_x,
                desc_y,
                self.config.description_font_size,
                self.config.secondary_text_color,
            );
        }

        // Draw buttons
        let confirm_bounds = self.confirm_button_bounds(dialog_bounds);
        let cancel_bounds = self.cancel_button_bounds(dialog_bounds);

        // Confirm button (destructive style)
        let confirm_bg = if self.hovered == HoveredButton::Confirm {
            self.config.confirm_button_hover
        } else {
            self.config.confirm_button_bg
        };
        renderer.fill_rounded_rect(confirm_bounds, confirm_bg, 0.0);
        renderer.stroke_rounded_rect(confirm_bounds, self.config.border_color, 0.0, 1.0);

        // Center text in confirm button
        let confirm_text_x = confirm_bounds.x
            + (confirm_bounds.width
                - self.confirm_label.len() as f32 * self.config.button_font_size * 0.5)
                / 2.0;
        let confirm_text_y =
            confirm_bounds.y + (confirm_bounds.height - self.config.button_font_size) / 2.0;
        renderer.text(
            &self.confirm_label,
            confirm_text_x,
            confirm_text_y,
            self.config.button_font_size,
            self.config.text_color,
        );

        // Cancel button
        let cancel_bg = if self.hovered == HoveredButton::Cancel {
            self.config.button_hover
        } else {
            self.config.button_bg
        };
        renderer.fill_rounded_rect(cancel_bounds, cancel_bg, 0.0);
        renderer.stroke_rounded_rect(cancel_bounds, self.config.border_color, 0.0, 1.0);

        // Center text in cancel button
        let cancel_text_x = cancel_bounds.x
            + (cancel_bounds.width
                - self.cancel_label.len() as f32 * self.config.button_font_size * 0.5)
                / 2.0;
        let cancel_text_y =
            cancel_bounds.y + (cancel_bounds.height - self.config.button_font_size) / 2.0;
        renderer.text(
            &self.cancel_label,
            cancel_text_x,
            cancel_text_y,
            self.config.button_font_size,
            self.config.text_color,
        );

        renderer.end_overlay();
    }

    fn on_event(&mut self, event: &Event, _bounds: Bounds) -> EventResult<M> {
        if !self.is_open {
            return EventResult::None;
        }

        let dialog_bounds = self.calculate_bounds();
        let confirm_bounds = self.confirm_button_bounds(dialog_bounds);
        let cancel_bounds = self.cancel_button_bounds(dialog_bounds);

        match event {
            Event::MouseMove { position, .. } => {
                // Update hover state
                if confirm_bounds.contains(position.0, position.1) {
                    self.hovered = HoveredButton::Confirm;
                } else if cancel_bounds.contains(position.0, position.1) {
                    self.hovered = HoveredButton::Cancel;
                } else {
                    self.hovered = HoveredButton::None;
                }
            }

            Event::MousePress {
                button: MouseButton::Left,
                position,
                ..
            }
            | Event::MouseRelease {
                button: MouseButton::Left,
                position,
                ..
            } => {
                // Only act on release for button clicks
                if matches!(event, Event::MouseRelease { .. }) {
                    if confirm_bounds.contains(position.0, position.1) {
                        log::debug!("Confirm dialog: confirmed");
                        self.is_open = false;
                        self.hovered = HoveredButton::None;
                        if let Some(msg) = self.on_confirm.call(()) {
                            return EventResult::Message(msg);
                        }
                    } else if cancel_bounds.contains(position.0, position.1) {
                        log::debug!("Confirm dialog: cancelled");
                        self.is_open = false;
                        self.hovered = HoveredButton::None;
                        if let Some(msg) = self.on_cancel.call(()) {
                            return EventResult::Message(msg);
                        }
                    }
                }
                // Consume clicks inside dialog to prevent pass-through
                if dialog_bounds.contains(position.0, position.1) {
                    return EventResult::None;
                }
            }

            Event::GlobalMousePress { position, .. } => {
                // Clicking outside dialog cancels it
                if OverlayCloseHelper::should_close_on_global_press(*position, dialog_bounds) {
                    log::debug!("Confirm dialog: closed via click outside");
                    self.is_open = false;
                    self.hovered = HoveredButton::None;
                    return self.on_cancel.call(()).into();
                }
            }

            Event::KeyPress {
                key: KeyCode::Escape,
                ..
            } => {
                log::debug!("Confirm dialog: cancelled via Escape");
                self.is_open = false;
                self.hovered = HoveredButton::None;
                return self.on_cancel.call(()).into();
            }

            Event::KeyPress {
                key: KeyCode::Enter,
                ..
            } => {
                // Enter confirms the action
                log::debug!("Confirm dialog: confirmed via Enter");
                self.is_open = false;
                self.hovered = HoveredButton::None;
                if let Some(msg) = self.on_confirm.call(()) {
                    return EventResult::Message(msg);
                }
            }

            Event::FocusLost => {
                log::debug!("Confirm dialog: closed via FocusLost");
                self.is_open = false;
                self.hovered = HoveredButton::None;
                return self.on_cancel.call(()).into();
            }

            _ => {}
        }

        EventResult::None
    }
}
