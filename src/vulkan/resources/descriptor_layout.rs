use std::{collections::HashMap, ops::Deref, rc::Rc};

use log::debug;

use ash::vk;

use crate::{
    error::{Error, VResult},
    vulkan::resources::descriptors::DescriptorBinding,
};

use super::{descriptors::Descriptors, device::Device};

pub struct DescriptorLayout {
    device: Rc<Device>,
    layout: vk::DescriptorSetLayout,
}

impl Deref for DescriptorLayout {
    type Target = vk::DescriptorSetLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

impl DescriptorLayout {
    pub unsafe fn new(device: &Rc<Device>, descriptors: &Descriptors) -> VResult<Rc<Self>> {
        debug!("Creating descriptor layouts");

        let mut used_bindings = HashMap::new();
        for descriptor in descriptors.iter() {
            let binding = descriptor.binding();
            let name = &descriptor.name;
            if let Some(prev) = used_bindings.get(&binding) {
                let msg = format!(
                    "Binding {} is shared by {} and {}. All bindings must be unique",
                    binding, descriptor.name, prev
                );
                return Err(Error::Local(msg));
            }
            used_bindings.insert(binding, name);
        }

        let device = device.clone();
        let bindings = descriptors
            .iter()
            .map(DescriptorBinding::as_descriptor_set_layout_binding)
            .collect::<Vec<_>>();
        let descriptor_layout_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .flags(vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR)
            .bindings(&bindings);
        let layout = device.create_descriptor_set_layout(&descriptor_layout_create_info, None)?;

        Ok(Rc::new(DescriptorLayout { device, layout }))
    }
}

impl Drop for DescriptorLayout {
    fn drop(&mut self) {
        debug!("Dropping descriptor set layout");
        unsafe {
            self.device.destroy_descriptor_set_layout(**self, None);
        }
    }
}
