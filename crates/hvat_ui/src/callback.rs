//! Callback abstraction for widget event handlers
//!
//! This module provides a type-safe wrapper around callback functions that are used
//! throughout the widget system. Instead of manually writing `Option<Box<dyn Fn(T) -> M>>`
//! repeatedly, widgets can use `Callback<T, M>` which encapsulates this pattern.
//!
//! # Examples
//!
//! ```ignore
//! use hvat_ui::Callback;
//!
//! struct MyWidget<M> {
//!     on_change: Callback<f32, M>,
//!     on_click: Callback<(), M>,
//! }
//!
//! impl<M> MyWidget<M> {
//!     fn new() -> Self {
//!         Self {
//!             on_change: Callback::none(),
//!             on_click: Callback::none(),
//!         }
//!     }
//!
//!     fn on_change<F>(mut self, callback: F) -> Self
//!     where
//!         F: Fn(f32) -> M + 'static,
//!     {
//!         self.on_change = Callback::new(callback);
//!         self
//!     }
//!
//!     fn trigger_change(&self, value: f32) -> Option<M> {
//!         self.on_change.call(value)
//!     }
//! }
//! ```

use std::fmt;

/// A callback wrapper that encapsulates optional event handlers.
///
/// This type provides a cleaner API for widgets that need to support optional callbacks.
/// Instead of manually managing `Option<Box<dyn Fn(T) -> M>>`, widgets can use this
/// type which provides convenient construction and calling methods.
///
/// # Type Parameters
///
/// - `T`: The input type for the callback (e.g., slider value, state, etc.)
/// - `M`: The message type returned by the callback
pub struct Callback<T, M> {
    f: Option<Box<dyn Fn(T) -> M>>,
}

impl<T, M> Callback<T, M> {
    /// Create a new callback from a function.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let callback = Callback::new(|value: f32| Message::ValueChanged(value));
    /// ```
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(T) -> M + 'static,
    {
        Self {
            f: Some(Box::new(f)),
        }
    }

    /// Create an empty callback (no handler).
    ///
    /// This is equivalent to `Default::default()` but more explicit.
    pub fn none() -> Self {
        Self { f: None }
    }

    /// Call the callback with a value, if it exists.
    ///
    /// Returns `Some(message)` if the callback is set, or `None` if no callback is registered.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(msg) = callback.call(42.0) {
    ///     // Handle the message
    /// }
    /// ```
    pub fn call(&self, value: T) -> Option<M> {
        self.f.as_ref().map(|f| f(value))
    }

    /// Check if the callback is set.
    pub fn is_some(&self) -> bool {
        self.f.is_some()
    }

    /// Check if the callback is not set.
    pub fn is_none(&self) -> bool {
        self.f.is_none()
    }
}

impl<T, M> Default for Callback<T, M> {
    fn default() -> Self {
        Self::none()
    }
}

impl<T, M> Clone for Callback<T, M> {
    fn clone(&self) -> Self {
        // We can't actually clone the boxed closure, so we return an empty callback.
        // This is fine because callbacks are typically set via builder methods and
        // cloning is only used internally by the widget system.
        Self::none()
    }
}

impl<T, M> fmt::Debug for Callback<T, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Callback")
            .field("set", &self.is_some())
            .finish()
    }
}

// =============================================================================
// Unit-like Callback (Callback0)
// =============================================================================

/// A callback that takes no parameters.
///
/// This is a convenience type alias for callbacks that don't need input values,
/// such as `on_undo_point` or simple button clicks.
///
/// # Example
///
/// ```ignore
/// use hvat_ui::Callback0;
///
/// struct Button<M> {
///     on_click: Callback0<M>,
/// }
/// ```
pub type Callback0<M> = Callback<(), M>;

impl<M> Callback0<M> {
    /// Call the callback without any parameters.
    ///
    /// This is a convenience method that's equivalent to `call(())`.
    pub fn emit(&self) -> Option<M> {
        self.call(())
    }
}

// =============================================================================
// Side-effect Callback (SideEffect)
// =============================================================================

/// A side-effect callback that doesn't return a message.
///
/// This is used for callbacks that perform side effects only, such as
/// saving undo points, logging, or other operations that don't produce
/// application messages.
///
/// # Example
///
/// ```ignore
/// use hvat_ui::SideEffect;
///
/// struct Slider<M> {
///     on_undo_point: SideEffect,
/// }
/// ```
pub struct SideEffect {
    f: Option<Box<dyn Fn()>>,
}

impl SideEffect {
    /// Create a new side-effect callback from a function.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() + 'static,
    {
        Self {
            f: Some(Box::new(f)),
        }
    }

    /// Create an empty side-effect callback.
    pub fn none() -> Self {
        Self { f: None }
    }

    /// Call the side-effect callback, if it exists.
    pub fn emit(&self) {
        if let Some(ref f) = self.f {
            f();
        }
    }

    /// Check if the callback is set.
    pub fn is_some(&self) -> bool {
        self.f.is_some()
    }

    /// Check if the callback is not set.
    pub fn is_none(&self) -> bool {
        self.f.is_none()
    }
}

impl Default for SideEffect {
    fn default() -> Self {
        Self::none()
    }
}

impl Clone for SideEffect {
    fn clone(&self) -> Self {
        // Can't clone the boxed closure, return empty callback
        Self::none()
    }
}

impl fmt::Debug for SideEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SideEffect")
            .field("set", &self.is_some())
            .finish()
    }
}
