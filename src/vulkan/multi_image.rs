use std::{ops::Deref, rc::Rc};

use ash::vk;
use log::debug;

use crate::error::VResult;

use super::{
    resources::{
        device::Device, device_memory::DeviceMemory, image::Image, image_view::ImageView,
        physical_device::PhysicalDevice,
    },
    Vulkan,
};

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct MultiImageUnit {
    pub image: Rc<Image>,
    pub memory: Rc<DeviceMemory>,
    pub view: Rc<ImageView>,
}

impl MultiImageUnit {
    pub unsafe fn new(
        physical_device: &PhysicalDevice,
        device: &Rc<Device>,
        format: vk::Format,
        size: vk::Extent2D,
        image_subresource_range: &vk::ImageSubresourceRange,
    ) -> VResult<Self> {
        let image = Image::new(device, format, size)?;
        let required_memory_size = image.get_required_memory_size().unwrap();
        let memory = DeviceMemory::new(
            physical_device.image_memory_type_index,
            device,
            required_memory_size,
        )?;

        device.bind_image_memory(**image, **memory, 0)?;

        let view = ImageView::new(device, &image, format, image_subresource_range)?;

        Ok(Self {
            image,
            memory,
            view,
        })
    }
}

pub struct MultiImage(Vec<MultiImageUnit>);

impl Deref for MultiImage {
    type Target = [MultiImageUnit];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MultiImage {
    pub unsafe fn new(
        physical_device: &Rc<PhysicalDevice>,
        device: &Rc<Device>,
        format: vk::Format,
        image_subresource_range: &vk::ImageSubresourceRange,
        size: vk::Extent2D,
        num_images: usize,
    ) -> VResult<Rc<Self>> {
        debug!("Creating image of size {:?}", size);
        let images = (0..num_images)
            .map(|_| {
                MultiImageUnit::new(
                    physical_device,
                    device,
                    format,
                    size,
                    image_subresource_range,
                )
            })
            .collect::<VResult<Vec<_>>>()?;
        Ok(Rc::new(Self(images)))
    }
}

impl Drop for MultiImage {
    fn drop(&mut self) {
        debug!("Destroying image");
    }
}

impl Vulkan {
    pub fn new_multi_image(
        &mut self,
        name: &str,
        format: vk::Format,
        size: vk::Extent2D,
        num_images: Option<usize>,
    ) -> VResult<Rc<MultiImage>> {
        unsafe {
            let num_images = num_images.unwrap_or(self.surface_info.desired_image_count);
            let image = MultiImage::new(
                &self.physical_device,
                &self.device,
                format,
                &self.image_subresource_range,
                size,
                num_images,
            )?;

            for image_unit in image.iter() {
                self.stale_images.push((
                    name.to_owned(),
                    image_unit.image.clone(),
                    vk::ImageLayout::UNDEFINED,
                    vk::ImageLayout::GENERAL,
                ));
            }

            let views_and_samplers = image
                .iter()
                .map(|unit| (unit.view.clone(), self.sampler.clone()))
                .collect::<Vec<_>>();
            self.register_image(name, &views_and_samplers);

            Ok(image)
        }
    }

    pub fn prev_shift(&mut self, multi_image: &MultiImage, name: &str) -> Rc<MultiImage> {
        let last_index = multi_image.len() - 1;
        let reordered_images = multi_image[last_index..]
            .iter()
            .chain(multi_image[..last_index].iter())
            .cloned()
            .collect();
        let multi_image = Rc::new(MultiImage(reordered_images));

        // I don't need to mark these images as stale, because they are shared with the original
        // image, which should have already been transitioned.

        let views_and_samplers = multi_image
            .iter()
            .map(|unit| (unit.view.clone(), self.sampler.clone()))
            .collect::<Vec<_>>();
        self.register_image(name, &views_and_samplers);

        multi_image
    }
}
