// The main HVAT application - shared between native and WASM builds

use hvat_ui::{
    widgets::*, Application, Color, Element, ImageHandle,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Home,
    Counter,
    ImageViewer,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeChoice {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub struct Theme {
    choice: ThemeChoice,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            choice: ThemeChoice::Dark,
        }
    }

    pub fn light() -> Self {
        Self {
            choice: ThemeChoice::Light,
        }
    }

    pub fn background_color(&self) -> Color {
        match self.choice {
            ThemeChoice::Dark => Color::rgb(0.15, 0.15, 0.15),
            ThemeChoice::Light => Color::rgb(0.95, 0.95, 0.95),
        }
    }

    pub fn text_color(&self) -> Color {
        match self.choice {
            ThemeChoice::Dark => Color::rgb(0.9, 0.9, 0.9),
            ThemeChoice::Light => Color::rgb(0.1, 0.1, 0.1),
        }
    }

    pub fn accent_color(&self) -> Color {
        Color::rgb(0.3, 0.6, 0.9)
    }

    pub fn button_color(&self) -> Color {
        match self.choice {
            ThemeChoice::Dark => Color::rgb(0.25, 0.25, 0.25),
            ThemeChoice::Light => Color::rgb(0.85, 0.85, 0.85),
        }
    }
}

pub struct HvatApp {
    current_tab: Tab,
    counter: i32,
    zoom: f32,
    pan_x: f32,
    pan_y: f32,
    show_debug_info: bool,
    theme: Theme,
    image_handle: Option<ImageHandle>,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    SwitchTab(Tab),

    // Counter
    Increment,
    Decrement,
    Reset,

    // Image viewer
    ZoomIn,
    ZoomOut,
    ResetView,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,

    // Settings
    ToggleDebugInfo,
    SetTheme(Theme),
}


impl Application for HvatApp {
    type Message = Message;

    fn new() -> Self {
        Self {
            current_tab: Tab::Home,
            counter: 0,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            show_debug_info: false,
            theme: Theme::dark(),
            image_handle: None,
        }
    }

    fn title(&self) -> String {
        "HVAT - Hyperspectral Annotation Tool".to_string()
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            // Navigation
            Message::SwitchTab(tab) => {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("🔄 Switching to tab: {:?}", tab).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("🔄 Switching to tab: {:?}", tab);
                self.current_tab = tab;
            }

            // Counter
            Message::Increment => {
                self.counter += 1;
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("➕ Counter incremented: {}", self.counter).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("➕ Counter incremented: {}", self.counter);
            }
            Message::Decrement => {
                self.counter -= 1;
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("➖ Counter decremented: {}", self.counter).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("➖ Counter decremented: {}", self.counter);
            }
            Message::Reset => {
                self.counter = 0;
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"🔄 Counter reset".into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("🔄 Counter reset");
            }

            // Image viewer
            Message::ZoomIn => {
                self.zoom = (self.zoom * 1.2).min(5.0);
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("🔍 Zoom in: {:.2}x", self.zoom).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("🔍 Zoom in: {:.2}x", self.zoom);
            }
            Message::ZoomOut => {
                self.zoom = (self.zoom / 1.2).max(0.2);
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("🔍 Zoom out: {:.2}x", self.zoom).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("🔍 Zoom out: {:.2}x", self.zoom);
            }
            Message::ResetView => {
                self.zoom = 1.0;
                self.pan_x = 0.0;
                self.pan_y = 0.0;
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"🔄 View reset".into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("🔄 View reset");
            }
            Message::PanLeft => {
                self.pan_x -= 10.0;
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("⬅️  Pan left: ({:.0}, {:.0})", self.pan_x, self.pan_y).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("⬅️  Pan left: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            Message::PanRight => {
                self.pan_x += 10.0;
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("➡️  Pan right: ({:.0}, {:.0})", self.pan_x, self.pan_y).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("➡️  Pan right: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            Message::PanUp => {
                self.pan_y -= 10.0;
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("⬆️  Pan up: ({:.0}, {:.0})", self.pan_x, self.pan_y).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("⬆️  Pan up: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            Message::PanDown => {
                self.pan_y += 10.0;
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("⬇️  Pan down: ({:.0}, {:.0})", self.pan_x, self.pan_y).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("⬇️  Pan down: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }

            // Settings
            Message::ToggleDebugInfo => {
                self.show_debug_info = !self.show_debug_info;
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("🐛 Debug info: {}", if self.show_debug_info { "ON" } else { "OFF" }).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("🐛 Debug info: {}", if self.show_debug_info { "ON" } else { "OFF" });
            }
            Message::SetTheme(theme) => {
                self.theme = theme.clone();
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("🎨 Theme changed to: {:?}", self.theme.choice).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("🎨 Theme changed to: {:?}", self.theme.choice);
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let _bg_color = self.theme.background_color();
        let text_color = self.theme.text_color();

        // Header with title and navigation
        let header = row()
            .push(Element::new(
                text("HVAT")
                    .size(20.0)
                    .color(self.theme.accent_color()),
            ))
            .push(Element::new(
                button("Home")
                    .on_press(Message::SwitchTab(Tab::Home))
                    .width(100.0),
            ))
            .push(Element::new(
                button("Counter")
                    .on_press(Message::SwitchTab(Tab::Counter))
                    .width(100.0),
            ))
            .push(Element::new(
                button("Image")
                    .on_press(Message::SwitchTab(Tab::ImageViewer))
                    .width(100.0),
            ))
            .push(Element::new(
                button("Settings")
                    .on_press(Message::SwitchTab(Tab::Settings))
                    .width(100.0),
            ))
            .spacing(10.0);

        // Content based on current tab
        let content = match self.current_tab {
            Tab::Home => self.view_home(text_color),
            Tab::Counter => self.view_counter(text_color),
            Tab::ImageViewer => self.view_image_viewer(text_color),
            Tab::Settings => self.view_settings(text_color),
        };

        Element::new(
            container(Element::new(
                column()
                    .push(Element::new(header))
                    .push(Element::new(content))
                    .spacing(20.0),
            ))
            .padding(30.0),
        )
    }
}

impl HvatApp {
    fn view_home(&self, text_color: Color) -> Column<'static, Message> {
        column()
            .push(Element::new(
                text("Welcome to HVAT")
                    .size(28.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("A GPU-accelerated hyperspectral image annotation tool")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("Features:")
                    .size(16.0)
                    .color(self.theme.accent_color()),
            ))
            .push(Element::new(
                text("• Fast GPU rendering with wgpu")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("• Cross-platform (native + WASM)")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("• Pan and zoom")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("• Custom UI framework")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text("Navigate using the tabs above to explore features")
                    .size(14.0)
                    .color(self.theme.accent_color()),
            ))
            .spacing(20.0)
    }

    fn view_counter(&self, text_color: Color) -> Column<'static, Message> {
        column()
            .push(Element::new(
                text("Counter Demo")
                    .size(24.0)
                    .color(text_color),
            ))
            .push(Element::new(
                container(Element::new(
                    text(format!("{}", self.counter))
                        .size(48.0)
                        .color(self.theme.accent_color()),
                ))
                .padding(20.0),
            ))
            .push(Element::new(
                row()
                    .push(Element::new(
                        button("Increment")
                            .on_press(Message::Increment)
                            .width(150.0),
                    ))
                    .push(Element::new(
                        button("Decrement")
                            .on_press(Message::Decrement)
                            .width(150.0),
                    ))
                    .push(Element::new(
                        button("Reset")
                            .on_press(Message::Reset)
                            .width(150.0),
                    ))
                    .spacing(15.0),
            ))
            .spacing(30.0)
    }

    fn view_image_viewer(&self, text_color: Color) -> Column<'static, Message> {
        column()
            .push(Element::new(
                text("Image Viewer")
                    .size(24.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text(format!("Zoom: {:.1}x | Pan: ({:.0}, {:.0})", self.zoom, self.pan_x, self.pan_y))
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                row()
                    .push(Element::new(
                        button("Zoom In")
                            .on_press(Message::ZoomIn)
                            .width(100.0),
                    ))
                    .push(Element::new(
                        button("Zoom Out")
                            .on_press(Message::ZoomOut)
                            .width(100.0),
                    ))
                    .push(Element::new(
                        button("Reset View")
                            .on_press(Message::ResetView)
                            .width(100.0),
                    ))
                    .spacing(10.0),
            ))
            .push(Element::new(
                text("Pan Controls:")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                row()
                    .push(Element::new(
                        button("← Left")
                            .on_press(Message::PanLeft)
                            .width(80.0),
                    ))
                    .push(Element::new(
                        button("↑ Up")
                            .on_press(Message::PanUp)
                            .width(80.0),
                    ))
                    .push(Element::new(
                        button("↓ Down")
                            .on_press(Message::PanDown)
                            .width(80.0),
                    ))
                    .push(Element::new(
                        button("→ Right")
                            .on_press(Message::PanRight)
                            .width(80.0),
                    ))
                    .spacing(10.0),
            ))
            .push(Element::new(
                text("Note: Image rendering not yet fully implemented")
                    .size(12.0)
                    .color(Color::rgb(0.8, 0.5, 0.2)),
            ))
            .spacing(20.0)
    }

    fn view_settings(&self, text_color: Color) -> Column<'static, Message> {
        column()
            .push(Element::new(
                text("Settings")
                    .size(24.0)
                    .color(text_color),
            ))
            .push(Element::new(
                container(Element::new(
                    column()
                        .push(Element::new(
                            text("Theme")
                                .size(16.0)
                                .color(self.theme.accent_color()),
                        ))
                        .push(Element::new(
                            row()
                                .push(Element::new(
                                    button("Dark Theme")
                                        .on_press(Message::SetTheme(Theme::dark()))
                                        .width(120.0),
                                ))
                                .push(Element::new(
                                    button("Light Theme")
                                        .on_press(Message::SetTheme(Theme::light()))
                                        .width(120.0),
                                ))
                                .spacing(10.0),
                        ))
                        .spacing(15.0),
                ))
                .padding(20.0),
            ))
            .push(Element::new(
                container(Element::new(
                    column()
                        .push(Element::new(
                            text("Debug")
                                .size(16.0)
                                .color(self.theme.accent_color()),
                        ))
                        .push(Element::new(
                            button(if self.show_debug_info {
                                "Hide Debug Info"
                            } else {
                                "Show Debug Info"
                            })
                            .on_press(Message::ToggleDebugInfo)
                            .width(150.0),
                        ))
                        .spacing(15.0),
                ))
                .padding(20.0),
            ))
            .spacing(20.0)
    }
}
