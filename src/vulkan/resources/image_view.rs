use std::{ops::Deref, rc::Rc, slice::Iter};

use ash::vk;

use crate::error::VResult;

use super::{device::Device, image::Image};

pub struct ImageView {
    device: Rc<Device>,
    image_view: vk::ImageView,
}

impl Deref for ImageView {
    type Target = vk::ImageView;

    fn deref(&self) -> &Self::Target {
        &self.image_view
    }
}

impl ImageView {
    pub unsafe fn new(
        device: &Rc<Device>,
        image: &Image,
        format: vk::Format,
        image_subresource_range: &vk::ImageSubresourceRange,
    ) -> VResult<Rc<Self>> {
        let device = device.clone();
        let component_mapping = vk::ComponentMapping::default();

        let create_view_info = vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .components(component_mapping)
            .subresource_range(*image_subresource_range)
            .image(**image);
        let image_view = device.create_image_view(&create_view_info, None)?;

        Ok(Rc::new(Self { device, image_view }))
    }

    pub unsafe fn many(
        device: &Rc<Device>,
        images: Iter<impl Deref<Target = Image>>,
        format: vk::Format,
        image_subresource_range: &vk::ImageSubresourceRange,
    ) -> VResult<Vec<Rc<Self>>> {
        images
            .map(|image| Self::new(device, image, format, image_subresource_range))
            .collect()
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(**self, None);
        };
    }
}
