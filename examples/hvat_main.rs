/// Main HVAT Application - Comprehensive example showing all hvat_ui features
///
/// This application demonstrates:
/// - Multiple widget types (text, buttons, containers, rows, columns)
/// - State management with messages
/// - Layout and styling
/// - Interactive UI elements
use hvat_ui::widgets::*;
use hvat_ui::*;

struct HvatApp {
    // View state
    current_tab: Tab,

    // Counter demo state
    counter: i32,

    // Image viewer state
    image_handle: Option<ImageHandle>,
    zoom: f32,
    pan_x: f32,
    pan_y: f32,

    // Settings state
    show_debug_info: bool,
    theme: Theme,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Home,
    Counter,
    ImageViewer,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Theme {
    Dark,
    Light,
}

impl Theme {
    fn background_color(&self) -> Color {
        match self {
            Theme::Dark => Color::rgb(0.1, 0.1, 0.1),
            Theme::Light => Color::rgb(0.95, 0.95, 0.95),
        }
    }

    fn text_color(&self) -> Color {
        match self {
            Theme::Dark => Color::WHITE,
            Theme::Light => Color::BLACK,
        }
    }

    fn button_color(&self) -> Color {
        match self {
            Theme::Dark => Color::rgb(0.2, 0.4, 0.6),
            Theme::Light => Color::rgb(0.6, 0.7, 0.9),
        }
    }

    fn accent_color(&self) -> Color {
        match self {
            Theme::Dark => Color::rgb(0.3, 0.8, 0.3),
            Theme::Light => Color::rgb(0.2, 0.6, 0.2),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    // Navigation
    SwitchTab(Tab),

    // Counter actions
    Increment,
    Decrement,
    Reset,

    // Image viewer actions
    ZoomIn,
    ZoomOut,
    ResetView,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,

    // Settings actions
    ToggleDebugInfo,
    SetTheme(Theme),
}

impl Application for HvatApp {
    type Message = Message;

    fn new() -> Self {
        // Create a test image for the image viewer
        let width = 200;
        let height = 200;
        let mut data = Vec::with_capacity((width * height * 4) as usize);

        // Create a colorful gradient test pattern
        for y in 0..height {
            for x in 0..width {
                let r = ((x as f32 / width as f32) * 255.0) as u8;
                let g = ((y as f32 / height as f32) * 255.0) as u8;
                let b = (((x + y) as f32 / (width + height) as f32) * 255.0) as u8;
                let a = 255;

                data.push(r);
                data.push(g);
                data.push(b);
                data.push(a);
            }
        }

        let image_handle = ImageHandle::from_rgba8(data, width, height);

        Self {
            current_tab: Tab::Home,
            counter: 0,
            image_handle: Some(image_handle),
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            show_debug_info: true,
            theme: Theme::Dark,
        }
    }

    fn title(&self) -> String {
        "HVAT - Hyperspectral Annotation Tool".to_string()
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            // Navigation
            Message::SwitchTab(tab) => {
                println!("🔄 Switching to tab: {:?}", tab);
                self.current_tab = tab;
            }

            // Counter
            Message::Increment => {
                self.counter += 1;
                println!("➕ Counter incremented: {}", self.counter);
            }
            Message::Decrement => {
                self.counter -= 1;
                println!("➖ Counter decremented: {}", self.counter);
            }
            Message::Reset => {
                self.counter = 0;
                println!("🔄 Counter reset");
            }

            // Image viewer
            Message::ZoomIn => {
                self.zoom = (self.zoom * 1.2).min(5.0);
                println!("🔍 Zoom in: {:.2}x", self.zoom);
            }
            Message::ZoomOut => {
                self.zoom = (self.zoom / 1.2).max(0.2);
                println!("🔍 Zoom out: {:.2}x", self.zoom);
            }
            Message::ResetView => {
                self.zoom = 1.0;
                self.pan_x = 0.0;
                self.pan_y = 0.0;
                println!("🔄 View reset");
            }
            Message::PanLeft => {
                self.pan_x -= 10.0;
                println!("⬅️  Pan left: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            Message::PanRight => {
                self.pan_x += 10.0;
                println!("➡️  Pan right: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            Message::PanUp => {
                self.pan_y -= 10.0;
                println!("⬆️  Pan up: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            Message::PanDown => {
                self.pan_y += 10.0;
                println!("⬇️  Pan down: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }

            // Settings
            Message::ToggleDebugInfo => {
                self.show_debug_info = !self.show_debug_info;
                println!("🐛 Debug info: {}", if self.show_debug_info { "ON" } else { "OFF" });
            }
            Message::SetTheme(theme) => {
                self.theme = theme;
                println!("🎨 Theme changed to: {:?}", theme);
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let bg_color = self.theme.background_color();
        let text_color = self.theme.text_color();

        // Header with title and navigation
        let header = column()
            .push(Element::new(
                text("HVAT - Hyperspectral Annotation Tool")
                    .size(32.0)
                    .color(text_color),
            ))
            .push(Element::new(
                row()
                    .push(Element::new(
                        button(if self.current_tab == Tab::Home { "● Home" } else { "Home" })
                            .on_press(Message::SwitchTab(Tab::Home))
                            .width(120.0),
                    ))
                    .push(Element::new(
                        button(if self.current_tab == Tab::Counter { "● Counter" } else { "Counter" })
                            .on_press(Message::SwitchTab(Tab::Counter))
                            .width(120.0),
                    ))
                    .push(Element::new(
                        button(if self.current_tab == Tab::ImageViewer { "● Viewer" } else { "Viewer" })
                            .on_press(Message::SwitchTab(Tab::ImageViewer))
                            .width(120.0),
                    ))
                    .push(Element::new(
                        button(if self.current_tab == Tab::Settings { "● Settings" } else { "Settings" })
                            .on_press(Message::SwitchTab(Tab::Settings))
                            .width(120.0),
                    ))
                    .spacing(10.0),
            ))
            .spacing(20.0);

        // Content based on current tab
        let content = match self.current_tab {
            Tab::Home => self.view_home(text_color),
            Tab::Counter => self.view_counter(text_color),
            Tab::ImageViewer => self.view_image_viewer(text_color),
            Tab::Settings => self.view_settings(text_color),
        };

        // Debug info
        let debug_section = if self.show_debug_info {
            Element::new(
                container(Element::new(
                    column()
                        .push(Element::new(
                            text("Debug Info:")
                                .size(12.0)
                                .color(Color::rgb(0.5, 0.5, 0.5)),
                        ))
                        .push(Element::new(
                            text(format!("Tab: {:?}", self.current_tab))
                                .size(12.0)
                                .color(Color::rgb(0.5, 0.5, 0.5)),
                        ))
                        .push(Element::new(
                            text(format!("Theme: {:?}", self.theme))
                                .size(12.0)
                                .color(Color::rgb(0.5, 0.5, 0.5)),
                        ))
                        .spacing(5.0),
                ))
                .padding(10.0),
            )
        } else {
            Element::new(container(Element::new(text("").size(1.0).color(text_color))))
        };

        // Main layout
        Element::new(
            container(Element::new(
                column()
                    .push(Element::new(header))
                    .push(Element::new(
                        container(Element::new(content)).padding(20.0),
                    ))
                    .push(debug_section)
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
                    .size(18.0)
                    .color(Color::rgb(0.7, 0.7, 0.7)),
            ))
            .push(Element::new(
                container(Element::new(
                    column()
                        .push(Element::new(
                            text("Features:")
                                .size(20.0)
                                .color(text_color),
                        ))
                        .push(Element::new(
                            text("• Immediate-mode UI framework")
                                .size(16.0)
                                .color(text_color),
                        ))
                        .push(Element::new(
                            text("• GPU-accelerated rendering with wgpu")
                                .size(16.0)
                                .color(text_color),
                        ))
                        .push(Element::new(
                            text("• Cross-platform (Native & WASM)")
                                .size(16.0)
                                .color(text_color),
                        ))
                        .push(Element::new(
                            text("• Flexible layout system")
                                .size(16.0)
                                .color(text_color),
                        ))
                        .push(Element::new(
                            text("• Type-safe message-driven architecture")
                                .size(16.0)
                                .color(text_color),
                        ))
                        .spacing(10.0),
                ))
                .padding(20.0),
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
                    text(format!("Count: {}", self.counter))
                        .size(48.0)
                        .color(if self.counter > 0 {
                            self.theme.accent_color()
                        } else if self.counter < 0 {
                            Color::RED
                        } else {
                            text_color
                        }),
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
                    .color(Color::rgb(0.7, 0.7, 0.7)),
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
                            .width(120.0),
                    ))
                    .spacing(10.0),
            ))
            .push(Element::new(
                text("Pan controls:")
                    .size(14.0)
                    .color(text_color),
            ))
            .push(Element::new(
                column()
                    .push(Element::new(
                        button("↑")
                            .on_press(Message::PanUp)
                            .width(50.0),
                    ))
                    .push(Element::new(
                        row()
                            .push(Element::new(
                                button("←")
                                    .on_press(Message::PanLeft)
                                    .width(50.0),
                            ))
                            .push(Element::new(
                                container(Element::new(
                                    text("⊙").size(20.0).color(text_color),
                                ))
                                .padding(5.0),
                            ))
                            .push(Element::new(
                                button("→")
                                    .on_press(Message::PanRight)
                                    .width(50.0),
                            ))
                            .spacing(10.0),
                    ))
                    .push(Element::new(
                        button("↓")
                            .on_press(Message::PanDown)
                            .width(50.0),
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
                            text("Theme:")
                                .size(18.0)
                                .color(text_color),
                        ))
                        .push(Element::new(
                            row()
                                .push(Element::new(
                                    button(if self.theme == Theme::Dark { "● Dark" } else { "Dark" })
                                        .on_press(Message::SetTheme(Theme::Dark))
                                        .width(120.0),
                                ))
                                .push(Element::new(
                                    button(if self.theme == Theme::Light { "● Light" } else { "Light" })
                                        .on_press(Message::SetTheme(Theme::Light))
                                        .width(120.0),
                                ))
                                .spacing(15.0),
                        ))
                        .spacing(15.0),
                ))
                .padding(20.0),
            ))
            .push(Element::new(
                container(Element::new(
                    column()
                        .push(Element::new(
                            text("Debug:")
                                .size(18.0)
                                .color(text_color),
                        ))
                        .push(Element::new(
                            button(if self.show_debug_info { "Hide Debug Info" } else { "Show Debug Info" })
                                .on_press(Message::ToggleDebugInfo)
                                .width(200.0),
                        ))
                        .spacing(15.0),
                ))
                .padding(20.0),
            ))
            .push(Element::new(
                container(Element::new(
                    column()
                        .push(Element::new(
                            text("About:")
                                .size(18.0)
                                .color(text_color),
                        ))
                        .push(Element::new(
                            text("HVAT UI Framework v0.1.0")
                                .size(14.0)
                                .color(Color::rgb(0.7, 0.7, 0.7)),
                        ))
                        .push(Element::new(
                            text("Built with Rust, wgpu, and winit")
                                .size(14.0)
                                .color(Color::rgb(0.7, 0.7, 0.7)),
                        ))
                        .spacing(10.0),
                ))
                .padding(20.0),
            ))
            .spacing(20.0)
    }
}

fn main() {
    if let Err(e) = hvat_ui::run::<HvatApp>(Settings::default()) {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}
