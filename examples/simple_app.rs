/// Simple test application using hvat_ui
///
/// This demonstrates how to create a minimal application with the framework.
use hvat_ui::widgets::*;
use hvat_ui::*;

struct SimpleApp {
    counter: i32,
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
    Decrement,
}

impl Application for SimpleApp {
    type Message = Message;

    fn new() -> Self {
        Self { counter: 0 }
    }

    fn title(&self) -> String {
        "Simple hvat_ui Application".to_string()
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::Increment => {
                self.counter += 1;
                println!("✅ INCREMENT clicked! Counter: {}", self.counter);
            }
            Message::Decrement => {
                self.counter -= 1;
                println!("✅ DECREMENT clicked! Counter: {}", self.counter);
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        Element::new(
            container(Element::new(
                column()
                    .push(Element::new(
                        text("Simple Counter App")
                            .size(32.0)
                            .color(Color::WHITE),
                    ))
                    .push(Element::new(
                        text(format!("Counter: {}", self.counter))
                            .size(48.0)
                            .color(Color::rgb(0.3, 0.8, 0.3)),
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
                            .spacing(20.0),
                    ))
                    .spacing(30.0),
            ))
            .padding(50.0),
        )
    }
}

fn main() {
    if let Err(e) = run::<SimpleApp>(Settings::default()) {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}
