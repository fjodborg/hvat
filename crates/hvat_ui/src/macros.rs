//! Macros for reducing boilerplate in widget implementations.

/// Generates a builder-style setter method for a field.
///
/// # Usage
///
/// ```ignore
/// // When method name matches field name:
/// builder_field!(width, f32);
/// // Generates: pub fn width(mut self, value: f32) -> Self { self.width = value; self }
///
/// // When method name differs from field name:
/// builder_field!(bg_color, bg_color, Color);
/// // Generates: pub fn bg_color(mut self, value: Color) -> Self { self.bg_color = value; self }
/// ```
#[macro_export]
macro_rules! builder_field {
    // Method name matches field name
    ($name:ident, $type:ty) => {
        pub fn $name(mut self, value: $type) -> Self {
            self.$name = value;
            self
        }
    };
    // Method name differs from field name
    ($method:ident, $field:ident, $type:ty) => {
        pub fn $method(mut self, value: $type) -> Self {
            self.$field = value;
            self
        }
    };
}

/// Generates a builder-style setter method for an Option field.
///
/// # Usage
///
/// ```ignore
/// builder_option!(width, f32);
/// // Generates: pub fn width(mut self, value: f32) -> Self { self.width = Some(value); self }
/// ```
#[macro_export]
macro_rules! builder_option {
    ($name:ident, $type:ty) => {
        pub fn $name(mut self, value: $type) -> Self {
            self.$name = Some(value);
            self
        }
    };
    ($method:ident, $field:ident, $type:ty) => {
        pub fn $method(mut self, value: $type) -> Self {
            self.$field = Some(value);
            self
        }
    };
}

/// Generates a callback setter method for widgets.
///
/// # Usage
///
/// ```ignore
/// // For callbacks with no parameters:
/// callback_setter!(on_press);
/// // Generates: pub fn on_press<F>(mut self, f: F) -> Self where F: Fn() -> Message + 'static
///
/// // For callbacks with parameters:
/// callback_setter!(on_change, f32);
/// // Generates: pub fn on_change<F>(mut self, f: F) -> Self where F: Fn(f32) -> Message + 'static
///
/// // For callbacks with tuple parameters:
/// callback_setter!(on_drag_start, (f32, f32));
/// // Generates: pub fn on_drag_start<F>(mut self, f: F) -> Self where F: Fn((f32, f32)) -> Message + 'static
/// ```
#[macro_export]
macro_rules! callback_setter {
    // Callback with no parameters
    ($name:ident) => {
        pub fn $name<F>(mut self, f: F) -> Self
        where
            F: Fn() -> Message + 'static,
        {
            self.$name = Some(Box::new(f));
            self
        }
    };
    // Callback with single parameter
    ($name:ident, $param:ty) => {
        pub fn $name<F>(mut self, f: F) -> Self
        where
            F: Fn($param) -> Message + 'static,
        {
            self.$name = Some(Box::new(f));
            self
        }
    };
    // Callback with multiple parameters (tuple-style input, expanded in closure)
    ($name:ident, $($param:ty),+) => {
        pub fn $name<F>(mut self, f: F) -> Self
        where
            F: Fn($($param),+) -> Message + 'static,
        {
            self.$name = Some(Box::new(f));
            self
        }
    };
}
