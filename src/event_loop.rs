use std::ops::{Deref, DerefMut};

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, VirtualKeyCode};
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::{event, event_loop};

pub enum ControlFlow {
    Continue,
    Exit(i32),
}

impl From<ControlFlow> for event_loop::ControlFlow {
    fn from(value: ControlFlow) -> Self {
        match value {
            ControlFlow::Continue => event_loop::ControlFlow::Poll,
            ControlFlow::Exit(code) => event_loop::ControlFlow::ExitWithCode(code),
        }
    }
}

pub enum Event {
    Close,
    Resize,
    Key(ElementState, VirtualKeyCode),
    MouseButton(ElementState, MouseButton),
    MouseMove(f32, f32),
}

impl<'a> TryFrom<&'a event::Event<'a, ()>> for Event {
    type Error = ();

    fn try_from(value: &event::Event<()>) -> Result<Self, Self::Error> {
        match value {
            event::Event::WindowEvent {
                event: event::WindowEvent::CloseRequested,
                ..
            } => Ok(Event::Close),
            event::Event::WindowEvent {
                event:
                    event::WindowEvent::KeyboardInput {
                        input:
                            event::KeyboardInput {
                                state,
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    },
                ..
            } => Ok(Event::Key(*state, *key)),
            event::Event::WindowEvent {
                event: event::WindowEvent::MouseInput { state, button, .. },
                ..
            } => Ok(Event::MouseButton(*state, *button)),
            event::Event::WindowEvent {
                event:
                    event::WindowEvent::CursorMoved {
                        position: PhysicalPosition { x, y },
                        ..
                    },
                ..
            } => Ok(Event::MouseMove(*x as f32, *y as f32)),
            _ => Err(()),
        }
    }
}

pub trait App {
    fn tick(&mut self) -> ControlFlow;
    fn handle_event(&mut self, event: &Event) -> ControlFlow;
}

pub struct EventLoop {
    inner: event_loop::EventLoop<()>,
}

impl Default for EventLoop {
    fn default() -> Self {
        EventLoop {
            inner: event_loop::EventLoop::new(),
        }
    }
}

impl Deref for EventLoop {
    type Target = event_loop::EventLoop<()>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for EventLoop {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

fn handle_event<A: App>(event: &event::Event<()>, app: &mut A) -> ControlFlow {
    match event {
        event::Event::MainEventsCleared => app.tick(),
        _ => event.try_into().map_or_else(
            |_| ControlFlow::Continue,
            |nice_event| app.handle_event(&nice_event),
        ),
    }
}

impl EventLoop {
    pub fn run<A: App>(mut self, app: &mut A) -> i32 {
        self.run_return(|event, &_, control_flow| {
            *control_flow = handle_event(&event, app).into();
        })
    }
}
