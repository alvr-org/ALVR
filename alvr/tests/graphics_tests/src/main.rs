mod compositor;

use alvr_common::prelude::*;
use compositor::{Compositor, Context};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

fn run() -> StrResult {
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    let compositor = Compositor::new(Context::new_any()?, (400, 300), None, None, 1);

    let context = compositor.context();

    let surface = unsafe { context.instance().create_surface(&window) };

    event_loop.run(move |event, _, control| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control = ControlFlow::Exit,
        Event::WindowEvent { .. } => (),
        _ => (),
    })
}

fn main() {
    show_err(run());
}
