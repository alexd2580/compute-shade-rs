use std::{ops::Deref, rc::Rc};

use ash::{self, vk};

use crate::error::VResult;

use super::device::Device;

pub struct DeviceMemory {
    device: Rc<Device>,
    size: usize,
    memory: vk::DeviceMemory,
}

impl Deref for DeviceMemory {
    type Target = vk::DeviceMemory;

    fn deref(&self) -> &Self::Target {
        &self.memory
    }
}

impl DeviceMemory {
    pub unsafe fn new(
        memory_type_index: u32,
        device: &Rc<Device>,
        size: usize,
    ) -> VResult<Rc<Self>> {
        let device = device.clone();
        let memory_alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(vk::DeviceSize::try_from(size).unwrap())
            .memory_type_index(memory_type_index);
        let memory = device.allocate_memory(&memory_alloc_info, None)?;
        Ok(Rc::new(Self {
            device,
            size,
            memory,
        }))
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for DeviceMemory {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(**self, None);
        }
    }
}
