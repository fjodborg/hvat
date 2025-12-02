// The main HVAT application - shared between native and WASM builds

use hvat_ui::{
    widgets::*, Application, Color, Element, ImageAdjustments, ImageHandle, Length,
};
use web_time::Instant;

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
    /// The test image for the image viewer
    test_image: ImageHandle,
    /// Image manipulation settings
    brightness: f32,
    contrast: f32,
    gamma: f32,
    hue_shift: f32,
    /// Whether the image is currently being dragged
    image_dragging: bool,
    /// Last drag position for calculating delta
    image_last_drag_pos: Option<(f32, f32)>,
    /// Which slider is currently being dragged (if any)
    active_slider: Option<SliderId>,
    /// Frame count for FPS calculation
    frame_count: u32,
    /// Last FPS update time
    last_fps_time: Instant,
    /// Current FPS value
    fps: f32,
    /// Scroll offset for the main content
    scroll_offset: f32,
    /// Whether the scrollbar is being dragged
    scrollbar_dragging: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    SwitchTab(Tab),

    // Counter
    Increment,
    Decrement,
    Reset,

    // Image viewer - button controls
    ZoomIn,
    ZoomOut,
    ResetView,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,

    // Image viewer - from widget callbacks
    ImagePan((f32, f32)),
    /// (new_zoom, cursor_x, cursor_y, widget_center_x, widget_center_y)
    ImageZoomAtPoint(f32, f32, f32, f32, f32),
    ImageDragStart((f32, f32)),
    ImageDragMove((f32, f32)),
    ImageDragEnd,

    // Image manipulation - slider drag
    SliderDragStart(SliderId),
    SliderDragEnd,

    // Image manipulation - value changes
    SetBrightness(f32),
    SetContrast(f32),
    SetGamma(f32),
    SetHueShift(f32),
    ResetImageSettings,

    // Settings
    ToggleDebugInfo,
    SetTheme(Theme),

    // FPS counter
    Tick,

    // Scrolling
    Scroll(f32),
    ScrollbarDragStart,
    ScrollbarDragEnd,
}


impl Application for HvatApp {
    type Message = Message;

    fn new() -> Self {
        // Create a larger test image for performance testing (4096x4096 = 64MB RGBA)
        log::info!("Creating 4096x4096 test image...");
        let test_image = create_test_image(4096, 4096);
        log::info!("Test image created");

        Self {
            current_tab: Tab::Home,
            counter: 0,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            show_debug_info: false,
            theme: Theme::dark(),
            test_image,
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            hue_shift: 0.0,
            image_dragging: false,
            image_last_drag_pos: None,
            active_slider: None,
            frame_count: 0,
            last_fps_time: Instant::now(),
            fps: 0.0,
            scroll_offset: 0.0,
            scrollbar_dragging: false,
        }
    }

    fn title(&self) -> String {
        "HVAT - Hyperspectral Annotation Tool".to_string()
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            // Navigation
            Message::SwitchTab(tab) => {
                log::debug!("🔄 Switching to tab: {:?}", tab);
                self.current_tab = tab;
            }

            // Counter
            Message::Increment => {
                self.counter += 1;
                log::debug!("➕ Counter incremented: {}", self.counter);
            }
            Message::Decrement => {
                self.counter -= 1;
                log::debug!("➖ Counter decremented: {}", self.counter);
            }
            Message::Reset => {
                self.counter = 0;
                log::debug!("🔄 Counter reset");
            }

            // Image viewer
            Message::ZoomIn => {
                self.zoom = (self.zoom * 1.2).min(5.0);
                log::debug!("🔍 Zoom in: {:.2}x", self.zoom);
            }
            Message::ZoomOut => {
                self.zoom = (self.zoom / 1.2).max(0.2);
                log::debug!("🔍 Zoom out: {:.2}x", self.zoom);
            }
            Message::ResetView => {
                self.zoom = 1.0;
                self.pan_x = 0.0;
                self.pan_y = 0.0;
                log::debug!("🔄 View reset");
            }
            Message::PanLeft => {
                self.pan_x -= 10.0;
                log::debug!("⬅️  Pan left: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            Message::PanRight => {
                self.pan_x += 10.0;
                log::debug!("➡️  Pan right: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            Message::PanUp => {
                self.pan_y -= 10.0;
                log::debug!("⬆️  Pan up: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }
            Message::PanDown => {
                self.pan_y += 10.0;
                log::debug!("⬇️  Pan down: ({:.0}, {:.0})", self.pan_x, self.pan_y);
            }

            // Image viewer - from widget callbacks
            Message::ImagePan(pan) => {
                self.pan_x = pan.0;
                self.pan_y = pan.1;
                // Don't log every pan event to avoid spam
            }
            Message::ImageZoomAtPoint(new_zoom, cursor_x, cursor_y, widget_center_x, widget_center_y) => {
                // Zoom-to-cursor algorithm:
                // The point under the cursor should stay in the same position after zooming.
                //
                // Current image point under cursor (in image-space relative to widget center):
                //   img_x = (cursor_x - widget_center_x - pan_x) / old_zoom
                //   img_y = (cursor_y - widget_center_y - pan_y) / old_zoom
                //
                // After zoom, we want the same image point to be under the cursor:
                //   cursor_x - widget_center_x = img_x * new_zoom + new_pan_x
                //   cursor_y - widget_center_y = img_y * new_zoom + new_pan_y
                //
                // Solving for new_pan:
                //   new_pan_x = (cursor_x - widget_center_x) - img_x * new_zoom
                //   new_pan_y = (cursor_y - widget_center_y) - img_y * new_zoom

                let old_zoom = self.zoom;

                // Cursor position relative to widget center
                let cursor_rel_x = cursor_x - widget_center_x;
                let cursor_rel_y = cursor_y - widget_center_y;

                // Image-space point under cursor
                let img_x = (cursor_rel_x - self.pan_x) / old_zoom;
                let img_y = (cursor_rel_y - self.pan_y) / old_zoom;

                // Update zoom
                self.zoom = new_zoom;

                // Calculate new pan to keep the image point under cursor
                self.pan_x = cursor_rel_x - img_x * new_zoom;
                self.pan_y = cursor_rel_y - img_y * new_zoom;

                log::debug!("🔍 Zoom-to-cursor: {:.2}x at ({:.1}, {:.1}), pan: ({:.1}, {:.1})",
                    self.zoom, cursor_x, cursor_y, self.pan_x, self.pan_y);
            }
            Message::ImageDragStart(pos) => {
                self.image_dragging = true;
                self.image_last_drag_pos = Some(pos);
                log::debug!("Pan drag started at ({:.1}, {:.1})", pos.0, pos.1);
            }
            Message::ImageDragMove(pos) => {
                if self.image_dragging {
                    if let Some(last_pos) = self.image_last_drag_pos {
                        let dx = pos.0 - last_pos.0;
                        let dy = pos.1 - last_pos.1;
                        self.pan_x += dx;
                        self.pan_y += dy;
                        self.image_last_drag_pos = Some(pos);
                        // Log pan movement (delta)
                        if dx.abs() > 1.0 || dy.abs() > 1.0 {
                            log::debug!("🖐️ Panning: delta({:.1}, {:.1}) -> pan({:.1}, {:.1})", dx, dy, self.pan_x, self.pan_y);
                        }
                    }
                }
            }
            Message::ImageDragEnd => {
                self.image_dragging = false;
                self.image_last_drag_pos = None;
                log::debug!("Pan drag ended");
            }

            // Slider drag state
            Message::SliderDragStart(id) => {
                self.active_slider = Some(id);
                log::debug!("Slider drag started: {:?}", id);
            }
            Message::SliderDragEnd => {
                self.active_slider = None;
                log::debug!("Slider drag ended");
            }

            // Image manipulation
            Message::SetBrightness(value) => {
                self.brightness = value;
                log::debug!("☀️  Brightness: {:.2}", self.brightness);
            }
            Message::SetContrast(value) => {
                self.contrast = value;
                log::debug!("🎛️  Contrast: {:.2}", self.contrast);
            }
            Message::SetGamma(value) => {
                self.gamma = value;
                log::debug!("📊 Gamma: {:.2}", self.gamma);
            }
            Message::SetHueShift(value) => {
                self.hue_shift = value;
                log::debug!("🎨 Hue shift: {:.2}", self.hue_shift);
            }
            Message::ResetImageSettings => {
                self.brightness = 0.0;
                self.contrast = 1.0;
                self.gamma = 1.0;
                self.hue_shift = 0.0;
                log::debug!("🔄 Image settings reset");
            }

            // Settings
            Message::ToggleDebugInfo => {
                self.show_debug_info = !self.show_debug_info;
                log::debug!("🐛 Debug info: {}", if self.show_debug_info { "ON" } else { "OFF" });
            }
            Message::SetTheme(theme) => {
                self.theme = theme.clone();
                log::debug!("🎨 Theme changed to: {:?}", self.theme.choice);
            }

            // FPS counter - called every frame
            Message::Tick => {
                self.frame_count += 1;
                let elapsed = self.last_fps_time.elapsed();
                // Update FPS every second
                if elapsed.as_secs_f32() >= 1.0 {
                    self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
                    self.frame_count = 0;
                    self.last_fps_time = Instant::now();
                }
            }

            // Scrolling
            Message::Scroll(offset) => {
                self.scroll_offset = offset;
                log::debug!("📜 Scroll offset: {:.1}", self.scroll_offset);
            }
            Message::ScrollbarDragStart => {
                self.scrollbar_dragging = true;
                log::debug!("📜 Scrollbar drag started");
            }
            Message::ScrollbarDragEnd => {
                self.scrollbar_dragging = false;
                log::debug!("📜 Scrollbar drag ended");
            }
        }
    }

    fn tick(&self) -> Option<Self::Message> {
        Some(Message::Tick)
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let _bg_color = self.theme.background_color();
        let text_color = self.theme.text_color();

        // Header with title and navigation
        let header_row = row()
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
            // FPS counter
            .push(Element::new(
                text(format!("FPS: {:.0}", self.fps))
                    .size(14.0)
                    .color(Color::rgb(0.5, 0.8, 0.5)),
            ))
            .spacing(10.0);

        // Wrap header in container with background to cover any content that bleeds through
        let header = container(Element::new(header_row))
            .padding(5.0)
            .background(self.theme.background_color());

        // Content based on current tab
        let content = match self.current_tab {
            Tab::Home => self.view_home(text_color),
            Tab::Counter => self.view_counter(text_color),
            Tab::ImageViewer => self.view_image_viewer(text_color),
            Tab::Settings => self.view_settings(text_color),
        };

        // Wrap content in scrollable with a fixed viewport size
        let scrollable_content = scrollable(Element::new(content))
            .scroll_offset(self.scroll_offset)
            .dragging(self.scrollbar_dragging)
            .width(800.0)  // Fixed width viewport (includes scrollbar)
            .height(600.0) // Fixed height viewport
            .on_scroll(Message::Scroll)
            .on_drag_start(|| Message::ScrollbarDragStart)
            .on_drag_end(|| Message::ScrollbarDragEnd);

        Element::new(
            container(Element::new(
                column()
                    .push(Element::new(header))
                    .push(Element::new(scrollable_content))
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

    fn view_image_viewer(&self, text_color: Color) -> Column<'_, Message> {
        // Create image adjustments from current settings
        let adjustments = ImageAdjustments {
            brightness: self.brightness,
            contrast: self.contrast,
            gamma: self.gamma,
            hue_shift: self.hue_shift,
        };

        // Create the pan/zoom image widget
        let image_widget = pan_zoom_image(self.test_image.clone())
            .pan((self.pan_x, self.pan_y))
            .zoom(self.zoom)
            .dragging(self.image_dragging)
            .adjustments(adjustments)
            .width(Length::Units(600.0))
            .height(Length::Units(400.0))
            .on_drag_start(Message::ImageDragStart)
            .on_drag_move(Message::ImageDragMove)
            .on_drag_end(|| Message::ImageDragEnd)
            .on_zoom(|new_zoom, cursor_x, cursor_y, widget_cx, widget_cy| {
                Message::ImageZoomAtPoint(new_zoom, cursor_x, cursor_y, widget_cx, widget_cy)
            });

        column()
            .push(Element::new(
                text("Image Viewer")
                    .size(24.0)
                    .color(text_color),
            ))
            .push(Element::new(
                text(format!("Zoom: {:.2}x | Pan: ({:.0}, {:.0})", self.zoom, self.pan_x, self.pan_y))
                    .size(14.0)
                    .color(text_color),
            ))
            // Image display area with border
            .push(Element::new(
                container(Element::new(image_widget))
                    .padding(4.0)
                    .border(Color::rgb(0.4, 0.4, 0.4))
                    .border_width(2.0),
            ))
            .push(Element::new(
                text("Drag to pan, scroll to zoom")
                    .size(12.0)
                    .color(Color::rgb(0.6, 0.6, 0.6)),
            ))
            // Zoom/pan button controls
            .push(Element::new(
                row()
                    .push(Element::new(
                        button("Zoom In")
                            .on_press(Message::ZoomIn)
                            .width(90.0),
                    ))
                    .push(Element::new(
                        button("Zoom Out")
                            .on_press(Message::ZoomOut)
                            .width(90.0),
                    ))
                    .push(Element::new(
                        button("Reset")
                            .on_press(Message::ResetView)
                            .width(90.0),
                    ))
                    .spacing(10.0),
            ))
            // Image manipulation controls with sliders
            .push(Element::new(
                text("Image Settings:")
                    .size(14.0)
                    .color(self.theme.accent_color()),
            ))
            // Brightness slider
            .push(Element::new(
                row()
                    .push(Element::new(
                        text(format!("Brightness: {:.2}", self.brightness))
                            .size(12.0)
                            .color(text_color),
                    ))
                    .push(Element::new(
                        slider(-1.0, 1.0, self.brightness)
                            .id(SliderId::Brightness)
                            .dragging(self.active_slider == Some(SliderId::Brightness))
                            .width(Length::Units(200.0))
                            .on_drag_start(Message::SliderDragStart)
                            .on_change(Message::SetBrightness)
                            .on_drag_end(|| Message::SliderDragEnd),
                    ))
                    .spacing(10.0),
            ))
            // Contrast slider
            .push(Element::new(
                row()
                    .push(Element::new(
                        text(format!("Contrast:   {:.2}", self.contrast))
                            .size(12.0)
                            .color(text_color),
                    ))
                    .push(Element::new(
                        slider(0.1, 3.0, self.contrast)
                            .id(SliderId::Contrast)
                            .dragging(self.active_slider == Some(SliderId::Contrast))
                            .width(Length::Units(200.0))
                            .on_drag_start(Message::SliderDragStart)
                            .on_change(Message::SetContrast)
                            .on_drag_end(|| Message::SliderDragEnd),
                    ))
                    .spacing(10.0),
            ))
            // Gamma slider
            .push(Element::new(
                row()
                    .push(Element::new(
                        text(format!("Gamma:      {:.2}", self.gamma))
                            .size(12.0)
                            .color(text_color),
                    ))
                    .push(Element::new(
                        slider(0.1, 3.0, self.gamma)
                            .id(SliderId::Gamma)
                            .dragging(self.active_slider == Some(SliderId::Gamma))
                            .width(Length::Units(200.0))
                            .on_drag_start(Message::SliderDragStart)
                            .on_change(Message::SetGamma)
                            .on_drag_end(|| Message::SliderDragEnd),
                    ))
                    .spacing(10.0),
            ))
            // Hue shift slider
            .push(Element::new(
                row()
                    .push(Element::new(
                        text(format!("Hue Shift:  {:.0}", self.hue_shift))
                            .size(12.0)
                            .color(text_color),
                    ))
                    .push(Element::new(
                        slider(-180.0, 180.0, self.hue_shift)
                            .id(SliderId::HueShift)
                            .dragging(self.active_slider == Some(SliderId::HueShift))
                            .width(Length::Units(200.0))
                            .on_drag_start(Message::SliderDragStart)
                            .on_change(Message::SetHueShift)
                            .on_drag_end(|| Message::SliderDragEnd),
                    ))
                    .spacing(10.0),
            ))
            // Reset button
            .push(Element::new(
                button("Reset Image Settings")
                    .on_press(Message::ResetImageSettings)
                    .width(180.0),
            ))
            .spacing(8.0)
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

/// Create a test image with a gradient pattern for demonstration.
fn create_test_image(width: u32, height: u32) -> ImageHandle {
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            // Create a colorful gradient pattern
            let fx = x as f32 / width as f32;
            let fy = y as f32 / height as f32;

            // Create a checkerboard pattern with gradients
            let checker = ((x / 32) + (y / 32)) % 2 == 0;

            let r = if checker {
                (fx * 255.0) as u8
            } else {
                ((1.0 - fx) * 255.0) as u8
            };
            let g = (fy * 255.0) as u8;
            let b = if checker {
                ((fx + fy) / 2.0 * 255.0) as u8
            } else {
                (((1.0 - fx) + (1.0 - fy)) / 2.0 * 255.0) as u8
            };

            // Add some circular pattern in the center
            let cx = (x as f32 - width as f32 / 2.0) / (width as f32 / 2.0);
            let cy = (y as f32 - height as f32 / 2.0) / (height as f32 / 2.0);
            let dist = (cx * cx + cy * cy).sqrt();

            let (r, g, b) = if dist < 0.3 {
                // Inner circle - bright color
                (255, 200, 100)
            } else if dist < 0.5 {
                // Ring
                (100, 150, 255)
            } else {
                (r, g, b)
            };

            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255); // Alpha
        }
    }

    ImageHandle::from_rgba8(data, width, height)
}
