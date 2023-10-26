use std::{ffi::c_void, ops::Deref, rc::Rc};

use ash::{self, vk};

use crate::error::VResult;

use super::{device::Device, device_memory::DeviceMemory};

pub struct MemoryMapping {
    device: Rc<Device>,
    memory: Rc<DeviceMemory>,
    mapped: *mut c_void,
}

impl Deref for MemoryMapping {
    type Target = *mut c_void;

    fn deref(&self) -> &Self::Target {
        &self.mapped
    }
}

impl MemoryMapping {
    pub unsafe fn new(device: &Rc<Device>, memory: &Rc<DeviceMemory>) -> VResult<Rc<Self>> {
        let device = device.clone();
        let memory = memory.clone();
        // https://stackoverflow.com/questions/64296581/do-i-need-to-memory-map-unmap-a-buffer-every-time-the-content-of-the-buffer-chan
        let mapped = device.map_memory(
            **memory,
            0,
            vk::DeviceSize::try_from(memory.size()).unwrap(),
            vk::MemoryMapFlags::empty(),
        )?;

        Ok(Rc::new(Self {
            device,
            memory,
            mapped,
        }))
    }
}

impl Drop for MemoryMapping {
    fn drop(&mut self) {
        unsafe {
            self.device.unmap_memory(**self.memory);
        }
    }
}
