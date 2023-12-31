use std::{mem, rc::Rc};

use compute_shade_rs::{error, event_loop, vulkan, window, winit};

struct App {
    gpu_buffer_1: Rc<vulkan::multi_buffer::MultiBuffer>,
    gpu_buffer_2: Rc<vulkan::multi_buffer::MultiBuffer>,
    globals_1: Rc<vulkan::multi_buffer::MultiBuffer>,
    globals_2: Rc<vulkan::multi_buffer::MultiBuffer>,

    images: Vec<Rc<vulkan::multi_image::MultiImage>>,

    vulkan: vulkan::Vulkan,
    _window: window::Window,
}

impl App {
    fn reinitialize_images(&mut self) -> error::VResult<()> {
        self.images.clear();

        // let vulkan = &mut self.vulkan;
        // let image_size = vulkan.surface_info.surface_resolution;

        // let image = vulkan.new_multi_image("image", image_size, None)?;
        // let image_prev = vulkan.prev_shift(&image, "image_prev");
        // self.images = vec![image, image_prev];

        Ok(())
    }

    fn new(event_loop: &event_loop::EventLoop) -> error::VResult<Self> {
        use vulkan::resources::buffer::BufferUsage;

        let shader_paths = vec![std::path::Path::new("examples/shaders/compute.comp")];
        let window = window::Window::new(event_loop, true)?;
        let mut vulkan = vulkan::Vulkan::new(&window, &shader_paths, true)?;

        let buffer_size = 100;

        // TODO
        let gpu_buffer_1 = vulkan.new_multi_buffer(
            "buffer_1",
            BufferUsage::Storage,
            buffer_size * mem::size_of::<i32>(),
            Some(1),
        )?;
        let gpu_buffer_2 = vulkan.new_multi_buffer(
            "FloatBuffer",
            BufferUsage::Storage,
            buffer_size * mem::size_of::<f32>(),
            Some(1),
        )?;

        let globals_1 = vulkan.new_multi_buffer(
            "Globals",
            BufferUsage::Uniform,
            mem::size_of::<u32>(),
            Some(1),
        )?;

        let globals_2 = vulkan.new_multi_buffer(
            "globals",
            BufferUsage::Uniform,
            mem::size_of::<u32>(),
            Some(1),
        )?;

        let x = 23;

        let target_1 = globals_1.mapped(0);
        let target_2 = globals_2.mapped(0);
        unsafe {
            *target_1.cast::<u32>() = 100 - x;
            *target_2.cast::<u32>() = 100 - (100 - x);
        }

        let mut app = Self {
            gpu_buffer_1,
            gpu_buffer_2,
            globals_1,
            globals_2,
            images: Vec::new(),
            vulkan,
            _window: window,
        };
        app.reinitialize_images()?;
        Ok(app)
    }

    fn run_vulkan(
        &mut self,
        push_constant_values: std::collections::HashMap<String, vulkan::Value>,
    ) -> error::VResult<()> {
        match unsafe { self.vulkan.tick(&push_constant_values)? } {
            None => (),
            Some(vulkan::Event::Resized) => self.reinitialize_images()?,
        }
        Ok(())
    }
}

impl event_loop::App for App {
    fn tick(&mut self) -> event_loop::ControlFlow {
        use vulkan::Value::{Bool, F32, U32};

        let target_1 = self.gpu_buffer_1.mapped(0);
        let target_2 = self.gpu_buffer_2.mapped(0);
        let offset = self.vulkan.num_frames % 100;
        unsafe {
            let itarget = target_1.add(offset * mem::size_of::<i32>());
            *itarget.cast::<i32>() = ((self.vulkan.num_frames as i32 % 200) - 100).abs();
            let ftarget = target_2.add(offset * mem::size_of::<f32>());
            *ftarget.cast::<f32>() = ((self.vulkan.num_frames as f32) / 100.0).sin() * 0.5 + 0.5;
        }

        let push_constant_values = std::collections::HashMap::from([
            (
                "bool_value".to_owned(),
                Bool(self.vulkan.num_frames % 120 > 60),
            ),
            (
                "float_value".to_owned(),
                F32(((self.vulkan.num_frames as f32) / 5.0).sin()),
            ),
        ]);

        match self.run_vulkan(push_constant_values) {
            Ok(()) => event_loop::ControlFlow::Continue,
            Err(err) => {
                log::error!("{err}");
                event_loop::ControlFlow::Exit(1)
            }
        }
    }

    fn handle_event(&mut self, event: &event_loop::Event) -> event_loop::ControlFlow {
        match event {
            event_loop::Event::Close => event_loop::ControlFlow::Exit(0),
            event_loop::Event::Key(_, winit::event::VirtualKeyCode::Q) => {
                event_loop::ControlFlow::Exit(0)
            }
            _ => event_loop::ControlFlow::Continue,
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.vulkan.wait_idle();
    }
}

fn run_main() -> error::VResult<i32> {
    let event_loop = event_loop::EventLoop::default();
    let mut app = App::new(&event_loop)?;
    Ok(event_loop.run(&mut app))
}

fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    log::info!("Initializing...");
    if let Err(err) = run_main() {
        log::error!("{}", err);
    }
    log::info!("Terminating...");
}
