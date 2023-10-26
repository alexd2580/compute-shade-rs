use log::debug;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::{dpi::PhysicalPosition, window::CursorGrabMode};

use crate::{error::VResult, event_loop::EventLoop, vulkan::resources::instance::Instance};
use ash::vk::SurfaceKHR as VkSurface;

pub struct Window(winit::window::Window);

impl Window {
    pub fn new(event_loop: &EventLoop) -> VResult<Self> {
        debug!("Initializing video system");

        let window = winit::window::WindowBuilder::new()
            .with_resizable(false)
            .with_title("visualize-rs")
            .build(event_loop)?;

        Ok(Window(window))
    }

    pub fn enumerate_required_extensions(&self) -> VResult<Vec<*const i8>> {
        let raw_handle = self.0.raw_display_handle();
        let extensions = ash_window::enumerate_required_extensions(raw_handle)?;
        Ok(extensions.to_vec())
    }

    pub fn create_surface(&self, entry: &ash::Entry, instance: &Instance) -> VResult<VkSurface> {
        unsafe {
            Ok(ash_window::create_surface(
                entry,
                instance,
                self.0.raw_display_handle(),
                self.0.raw_window_handle(),
                None,
            )?)
        }
    }

    pub fn set_cursor_grab(&self, lock: bool) {
        let _ = if lock {
            self.0
                .set_cursor_grab(CursorGrabMode::Confined)
                .or_else(|_| self.0.set_cursor_grab(CursorGrabMode::Locked))
        } else {
            self.0.set_cursor_grab(CursorGrabMode::None)
        }
        .map(|_| self.0.set_cursor_visible(!lock));
    }

    pub fn set_cursor_position(&self, x: u32, y: u32) {
        let _ = self.0.set_cursor_position(PhysicalPosition::new(x, y));
    }

    pub fn size(&self) -> (u32, u32) {
        let size = self.0.inner_size();
        (size.width, size.height)
    }
}
