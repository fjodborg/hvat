/// Image widget demo - demonstrates the Image widget with Length sizing
use hvat_ui::widgets::*;
use hvat_ui::*;

struct ImageDemo {
    image_handle: ImageHandle,
}

#[derive(Debug, Clone)]
enum Message {
    // No messages needed for this simple demo
}

impl Application for ImageDemo {
    type Message = Message;

    fn new() -> Self {
        // Create a simple test image (100x100 red square)
        let width = 100;
        let height = 100;
        let mut data = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            for x in 0..width {
                // Create a gradient pattern
                let r = ((x as f32 / width as f32) * 255.0) as u8;
                let g = ((y as f32 / height as f32) * 255.0) as u8;
                let b = 128;
                let a = 255;

                data.push(r);
                data.push(g);
                data.push(b);
                data.push(a);
            }
        }

        let image_handle = ImageHandle::from_rgba8(data, width, height);

        Self { image_handle }
    }

    fn title(&self) -> String {
        "Image Widget Demo".to_string()
    }

    fn update(&mut self, _message: Self::Message) {
        // No state updates needed
    }

    fn view(&self) -> Element<Self::Message> {
        Element::new(
            column()
                .push(Element::new(
                    text("Image Widget Demo")
                        .size(24.0)
                        .color(Color::WHITE),
                ))
                .push(Element::new(
                    text("Intrinsic size (Shrink):")
                        .size(14.0)
                        .color(Color::rgb(0.7, 0.7, 0.7)),
                ))
                .push(Element::new(image(self.image_handle.clone())))
                .push(Element::new(
                    text("Fixed size (200x200):")
                        .size(14.0)
                        .color(Color::rgb(0.7, 0.7, 0.7)),
                ))
                .push(Element::new(
                    image(self.image_handle.clone())
                        .width(Length::Units(200.0))
                        .height(Length::Units(200.0)),
                ))
                .push(Element::new(
                    text("Fill width, maintain aspect:")
                        .size(14.0)
                        .color(Color::rgb(0.7, 0.7, 0.7)),
                ))
                .push(Element::new(
                    image(self.image_handle.clone()).width(Length::Fill),
                ))
                .spacing(10.0),
        )
    }
}

fn main() {
    // Run the application
    if let Err(e) = hvat_ui::run::<ImageDemo>(Settings::default()) {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}
