use std::{ops::Deref, rc::Rc};

use ash::{self, vk};

use crate::error::VResult;

use super::device::Device;

pub struct Buffer {
    pub size: usize,
    device: Rc<Device>,
    buffer: vk::Buffer,
}

impl Deref for Buffer {
    type Target = vk::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

#[derive(Clone, Copy)]
pub enum BufferUsage {
    Storage,
    Uniform,
}

impl From<BufferUsage> for vk::BufferUsageFlags {
    fn from(value: BufferUsage) -> Self {
        match value {
            BufferUsage::Storage => vk::BufferUsageFlags::STORAGE_BUFFER,
            BufferUsage::Uniform => vk::BufferUsageFlags::UNIFORM_BUFFER,
        }
    }
}

impl Buffer {
    pub unsafe fn new(device: &Rc<Device>, usage: BufferUsage, size: usize) -> VResult<Rc<Self>> {
        let device = device.clone();
        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(vk::DeviceSize::try_from(size).unwrap())
            .usage(usage.into())
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = device.create_buffer(&buffer_create_info, None)?;

        Ok(Rc::new(Self {
            size,
            device,
            buffer,
        }))
    }

    #[must_use]
    pub unsafe fn get_required_memory_size(&self) -> usize {
        usize::try_from(self.device.get_buffer_memory_requirements(**self).size).unwrap()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
        }
    }
}
