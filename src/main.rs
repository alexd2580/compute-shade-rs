use std::{rc::Rc, time};

use cell::Cell;

mod cell;
mod error;
mod ring_buffer;
mod thread_shared;
mod timer;
mod utils;
mod vulkan;
mod window;

// Required to use run_return on event loop.
use winit::platform::run_return::EventLoopExtRunReturn;

struct Visualizer {
    signal_gpu: Rc<vulkan::multi_buffer::MultiBuffer>,
    signal_dft_gpu: Rc<vulkan::multi_buffer::MultiBuffer>,

    low_pass_gpu: Rc<vulkan::multi_buffer::MultiBuffer>,
    low_pass_dft_gpu: Rc<vulkan::multi_buffer::MultiBuffer>,

    high_pass_gpu: Rc<vulkan::multi_buffer::MultiBuffer>,
    high_pass_dft_gpu: Rc<vulkan::multi_buffer::MultiBuffer>,

    // These should be dropped last.
    images: Vec<Rc<vulkan::multi_image::MultiImage>>,
    vulkan: vulkan::Vulkan,
}

impl Visualizer {
    fn reinitialize_images(&mut self) -> error::VResult<()> {
        // Drop old images.
        self.images.clear();

        let vulkan = &mut self.vulkan;
        let image_size = vulkan.surface_info.surface_resolution;

        let intermediate = vulkan.new_multi_image("intermediate", image_size, None)?;
        let intermediate_prev = vulkan.prev_shift(&intermediate, "intermediate_prev");
        self.images.push(intermediate);
        self.images.push(intermediate_prev);

        let highlights = vulkan.new_multi_image("highlights", image_size, None)?;
        self.images.push(highlights);
        let bloom_h = vulkan.new_multi_image("bloom_h", image_size, None)?;
        self.images.push(bloom_h);
        let bloom_hv = vulkan.new_multi_image("bloom_hv", image_size, None)?;
        self.images.push(bloom_hv);
        let result = vulkan.new_multi_image("result", image_size, None)?;
        let result_prev = vulkan.prev_shift(&result, "result_prev");
        self.images.push(result);
        self.images.push(result_prev);

        Ok(())
    }

    fn new(args: &Args) -> error::VResult<(winit::event_loop::EventLoop<()>, Visualizer)> {
        let (event_loop, window) = window::Window::new()?;
        let mut vulkan = vulkan::Vulkan::new(&window, &args.shader_paths, !args.no_vsync)?;

        let mut visualizer = Self {
            images: Vec::new(),
            vulkan,
        };

        visualizer.reinitialize_images()?;
        Ok((event_loop, visualizer))
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

    fn tick(&mut self, analysis: &analysis::Analysis) -> winit::event_loop::ControlFlow {
        use vulkan::Value::{Bool, F32};

        let read_index = analysis.read_index;
        let write_index = analysis.write_index;

        analysis
            .audio
            .signal
            .write_to_pointer(read_index, write_index, self.signal_gpu.mapped(0));

        let mut push_constant_values = std::collections::HashMap::new();

        let is_beat = analysis.beat_detectors[0].is_beat;
        push_constant_values.insert("is_beat".to_owned(), Bool(is_beat));
        let now = analysis.epoch.elapsed().as_secs_f32();
        push_constant_values.insert("now".to_owned(), F32(now));

        let result = match self.run_vulkan(push_constant_values) {
            Ok(()) => winit::event_loop::ControlFlow::Poll,
            Err(err) => {
                log::error!("{err}");
                winit::event_loop::ControlFlow::ExitWithCode(1)
            }
        };

        self.vulkan.num_frames += 1;

        result
    }
}

impl Drop for Visualizer {
    fn drop(&mut self) {
        self.vulkan.wait_idle();
    }
}

fn run_main(args: &Args) -> error::VResult<()> {
    // The visualizer should also be ticked once per frame.
    let visualizer = (!args.headless)
        .then(|| Visualizer::new(&args, &analysis.as_ref()))
        .transpose()?;

    // Choose the mainloop.
    if let Some((mut event_loop, visualizer)) = visualizer {
        // Use the visual winit-based mainloop.
        let visualizer = Cell::new(visualizer);
        event_loop.run_return(|event, &_, control_flow| {
            *control_flow = window::handle_event(&event, &|| {
                analysis.as_mut_ref().tick();
                visualizer.as_mut_ref().tick(&analysis.as_ref())
            });
        });
    } else {
        // Use a custom headless one.
        let run = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        ctrlc::set_handler({
            let run = run.clone();
            move || {
                run.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        })
        .expect("Error setting Ctrl-C handler");
        while run.load(std::sync::atomic::Ordering::SeqCst) {
            analysis.as_mut_ref().tick();
            std::thread::sleep(time::Duration::from_millis(16));
        }
    }

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
