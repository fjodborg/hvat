/// Widget gallery example - demonstrates all hvat_ui widgets
use hvat_ui::widgets::*;
use hvat_ui::*;

struct WidgetGallery {
    button_clicks: usize,
    canvas_program: TestCanvasProgram,
}

#[derive(Debug, Clone)]
enum Message {
    ButtonClicked,
}

struct TestCanvasProgram {
    mouse_pos: Point,
}

impl Program<Message> for TestCanvasProgram {
    fn update(&mut self, event: &Event, _bounds: Rectangle) -> Option<Message> {
        match event {
            Event::MouseMoved { position } => {
                self.mouse_pos = *position;
                None
            }
            _ => None,
        }
    }

    fn draw(&self, renderer: &mut Renderer, bounds: Rectangle) {
        // Draw a simple background
        renderer.fill_rect(bounds, Color::rgb(0.15, 0.15, 0.15));

        // Draw some text at mouse position
        renderer.draw_text(
            &format!("Mouse: ({:.0}, {:.0})", self.mouse_pos.x, self.mouse_pos.y),
            Point::new(bounds.x + 10.0, bounds.y + 10.0),
            Color::WHITE,
            14.0,
        );
    }
}

impl Application for WidgetGallery {
    type Message = Message;

    fn new() -> Self {
        Self {
            button_clicks: 0,
            canvas_program: TestCanvasProgram {
                mouse_pos: Point::zero(),
            },
        }
    }

    fn title(&self) -> String {
        "hvat_ui Widget Gallery".to_string()
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::ButtonClicked => {
                self.button_clicks += 1;
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        // Build the layout - demonstrating basic widgets
        Element::new(
            column()
                .push(Element::new(
                    text("hvat_ui Widget Gallery")
                        .size(24.0)
                        .color(Color::WHITE)
                ))
                .push(Element::new(
                    button("Click Me!")
                        .on_press(Message::ButtonClicked)
                        .width(150.0)
                ))
                .push(Element::new(
                    text(format!("Clicked {} times", self.button_clicks))
                        .size(16.0)
                        .color(Color::WHITE)
                ))
                .spacing(10.0),
        )
    }
}

fn main() {
    // Run the application
    if let Err(e) = hvat_ui::run::<WidgetGallery>(Settings::default()) {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}
