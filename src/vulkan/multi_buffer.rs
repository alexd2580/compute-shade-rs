use std::{ffi::c_void, ops::Deref, rc::Rc};

use log::debug;

use crate::error::VResult;

use super::{
    resources::{
        buffer::{Buffer, BufferUsage}, device::Device, device_memory::DeviceMemory, memory_mapping::MemoryMapping,
        physical_device::PhysicalDevice,
    },
    Vulkan,
};

#[allow(clippy::module_name_repetitions)]
pub struct MultiBufferUnit {
    pub buffer: Rc<Buffer>,
    pub memory: Rc<DeviceMemory>,
    pub mapping: Rc<MemoryMapping>,
}

impl MultiBufferUnit {
    unsafe fn new(
        physical_device: &PhysicalDevice,
        device: &Rc<Device>,
        usage: BufferUsage,
        size: usize,
    ) -> VResult<Self> {
        let buffer = Buffer::new(device, usage, size)?;
        let memory = DeviceMemory::new(
            physical_device.buffer_memory_type_index,
            device,
            buffer.get_required_memory_size(),
        )?;
        let mapping = MemoryMapping::new(device, &memory)?;

        device.bind_buffer_memory(**buffer, **memory, 0)?;

        Ok(MultiBufferUnit {
            buffer,
            memory,
            mapping,
        })
    }
}

/// A buffer is composed of multiple device buffers used for multi-buffering (i.e.
/// triple-buffering). These buffers are automatically mapped to system memory to be written to,
/// and unmapped when the object is dropped.
pub struct MultiBuffer(Vec<MultiBufferUnit>);

impl Deref for MultiBuffer {
    type Target = [MultiBufferUnit];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MultiBuffer {
    unsafe fn new(
        physical_device: &Rc<PhysicalDevice>,
        device: &Rc<Device>,
        usage: BufferUsage,
        size: usize,
        num_buffers: usize,
    ) -> VResult<Rc<Self>> {
        debug!("Creating buffer of size {}", size);
        let buffers = (0..num_buffers)
            .map(|_| MultiBufferUnit::new(physical_device, device, usage, size))
            .collect::<VResult<Vec<_>>>()?;
        Ok(Rc::new(MultiBuffer(buffers)))
    }

    #[must_use]
    pub fn mapped(&self, index: usize) -> *mut c_void {
        **self[index].mapping
    }
}

impl Drop for MultiBuffer {
    fn drop(&mut self) {
        debug!("Destroying buffer");
    }
}

impl Vulkan {
    pub fn new_multi_buffer(
        &mut self,
        name: &str,
        usage: BufferUsage,
        size: usize,
        num_buffers: Option<usize>,
    ) -> VResult<Rc<MultiBuffer>> {
        unsafe {
            let num_buffers = num_buffers.unwrap_or(self.surface_info.desired_image_count);
            let buffer = MultiBuffer::new(&self.physical_device, &self.device, usage, size, num_buffers)?;
            let buffers = buffer
                .iter()
                .map(|unit| unit.buffer.clone())
                .collect::<Vec<_>>();
            self.register_buffer(name, &buffers);
            Ok(buffer)
        }
    }
}
