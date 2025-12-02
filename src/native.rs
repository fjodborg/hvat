use std::sync::Arc;
use winit::{event_loop::EventLoop, window::WindowBuilder};

use crate::app::{initialize_app, run_event_loop};

pub fn run() {
    println!("HVAT - Native Application");
    println!("Controls: Drag to pan, scroll to zoom, R to reset");
    println!();

    let event_loop = EventLoop::new().expect("couldn't create event loop");

    let window = WindowBuilder::new()
        .with_title("HVAT - GPU Test (Native)")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .expect("couldn't create window");

    let window = Arc::new(window);

    // Initialize app (blocking on native with pollster)
    let state = match pollster::block_on(initialize_app(window.clone())) {
        Ok(state) => state,
        Err(e) => {
            eprintln!("Failed to initialize app: {}", e);
            return;
        }
    };

    println!();
    run_event_loop(event_loop, window, state);
}
