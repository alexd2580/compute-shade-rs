use std::{mem, rc::Rc};

use compute_shade_rs as comp;

// Required to use run_return on event loop.
use winit::platform::run_return::EventLoopExtRunReturn;

struct App {
    gpu_buffer_1: Rc<comp::vulkan::multi_buffer::MultiBuffer>,
    gpu_buffer_2: Rc<comp::vulkan::multi_buffer::MultiBuffer>,

    images: Vec<Rc<comp::vulkan::multi_image::MultiImage>>,

    vulkan: comp::vulkan::Vulkan,
    window: comp::window::Window,
}

impl App {
    fn reinitialize_images(&mut self) -> comp::error::VResult<()> {
        self.images.clear();

        // let vulkan = &mut self.vulkan;
        // let image_size = vulkan.surface_info.surface_resolution;

        // let image = vulkan.new_multi_image("image", image_size, None)?;
        // let image_prev = vulkan.prev_shift(&image, "image_prev");
        // self.images = vec![image, image_prev];

        Ok(())
    }

    fn new() -> comp::error::VResult<(winit::event_loop::EventLoop<()>, Self)> {
        let shader_paths = vec![std::path::Path::new("examples/shaders/compute.comp")];
        let (event_loop, window) = comp::window::Window::new()?;
        let mut vulkan = comp::vulkan::Vulkan::new(&window, &shader_paths, true)?;

        // TODO
        let gpu_buffer_1 =
            vulkan.new_multi_buffer("buffer_1", 100 * mem::size_of::<i32>(), Some(1))?;
        let gpu_buffer_2 =
            vulkan.new_multi_buffer("buffer_2", 100 * mem::size_of::<f32>(), Some(1))?;

        let mut app = Self {
            gpu_buffer_1,
            gpu_buffer_2,
            images: Vec::new(),
            vulkan,
            window,
        };
        app.reinitialize_images()?;
        Ok((event_loop, app))
    }

    fn run_vulkan(
        &mut self,
        push_constant_values: std::collections::HashMap<String, comp::vulkan::Value>,
    ) -> comp::error::VResult<()> {
        match unsafe { self.vulkan.tick(&push_constant_values)? } {
            None => (),
            Some(comp::vulkan::Event::Resized) => self.reinitialize_images()?,
        }
        Ok(())
    }

    fn tick(&mut self) -> winit::event_loop::ControlFlow {
        use comp::vulkan::Value::{Bool, F32};

        let target_1 = self.gpu_buffer_1.mapped(0);
        let target_2 = self.gpu_buffer_2.mapped(0);
        let offset = self.vulkan.num_frames % 100;
        unsafe {
            let itarget = target_1.add(offset * mem::size_of::<i32>());
            *itarget.cast::<i32>() = self.vulkan.num_frames as i32;
            let ftarget = target_2.add(offset * mem::size_of::<f32>());
            *ftarget.cast::<f32>() = self.vulkan.num_frames as f32;
        }

        let push_constant_values = std::collections::HashMap::from([
            ("bool_value".to_owned(), Bool(false)),
            ("float_value".to_owned(), F32(1.5)),
        ]);

        let result = match self.run_vulkan(push_constant_values) {
            Ok(()) => winit::event_loop::ControlFlow::Poll,
            Err(err) => {
                log::error!("{err}");
                winit::event_loop::ControlFlow::ExitWithCode(1)
            }
        };

        // Shouldn't vulkan do this?
        self.vulkan.num_frames += 1;

        result
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.vulkan.wait_idle();
    }
}

fn run_main() -> comp::error::VResult<()> {
    let (mut event_loop, app) = App::new()?;
    let app = comp::cell::Cell::new(app);
    event_loop.run_return(|event, &_, control_flow| {
        *control_flow = comp::window::handle_event(&event, &|| app.as_mut_ref().tick());
    });
    Ok(())
}

fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    log::info!("Initializing...");
    if let Err(err) = run_main() {
        log::error!("{}", err);
    }
    log::info!("Terminating...");
}
