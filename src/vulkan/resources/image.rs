use std::{ops::Deref, rc::Rc};

use ash::{extensions::khr::Swapchain as SwapchainLoader, vk};

use crate::error::VResult;

use super::{device::Device, swapchain::Swapchain};

#[allow(clippy::module_name_repetitions)]
pub struct RegularImage {
    device: Rc<Device>,
    image: vk::Image,
}

#[allow(clippy::module_name_repetitions)]
pub struct SwapchainImage {
    image: vk::Image,
}

pub enum Image {
    Regular(RegularImage),
    Swapchain(SwapchainImage),
}

impl Deref for Image {
    type Target = vk::Image;

    fn deref(&self) -> &Self::Target {
        match self {
            Image::Regular(RegularImage { image, .. })
            | Image::Swapchain(SwapchainImage { image }) => image,
        }
    }
}

impl Image {
    pub unsafe fn new(
        device: &Rc<Device>,
        format: vk::Format,
        size: vk::Extent2D,
    ) -> VResult<Rc<Self>> {
        let device = device.clone();
        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(size.into())
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = device.create_image(&image_create_info, None)?;
        let image = Self::Regular(RegularImage { device, image });
        Ok(Rc::new(image))
    }

    pub unsafe fn many_from_swapchain(
        swapchain_loader: &SwapchainLoader,
        swapchain: &Swapchain,
    ) -> VResult<Vec<Rc<Self>>> {
        let images = swapchain_loader
            .get_swapchain_images(**swapchain)?
            .into_iter()
            .map(|image| Rc::new(Self::Swapchain(SwapchainImage { image })))
            .collect();
        Ok(images)
    }

    #[must_use]
    pub unsafe fn get_required_memory_size(&self) -> Option<usize> {
        match self {
            Self::Regular(RegularImage { device, image }) => {
                let size = device.get_image_memory_requirements(*image).size;
                Some(usize::try_from(size).unwrap())
            }
            Self::Swapchain(..) => None,
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        match self {
            Self::Regular(RegularImage { device, image }) => unsafe {
                device.destroy_image(*image, None);
            },
            Self::Swapchain(..) => (),
        }
    }
}
