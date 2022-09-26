use crate::{context, Error, Result};

use std::ffi;
use std::os::raw;

use ash::extensions::ext;
use ash::{vk, Entry, Instance};

pub struct Device<'a> {
    pub(crate) device: &'a context::Context,
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) logical_device: vk::Device,
}

pub struct DeviceInfo<'a> {
    pub context: &'a context::Context,
    pub rank: ops::Fn(Details) -> usize,
}

pub struct Details {
    pub properties: Properties,
    pub features: Features,
}

pub struct Properties {
    pub device_type: DeviceType,
    pub limits: Limits,
}

impl From<vk::PhysicalDeviceProperties> for Properties {
    fn from(properties: vk::PhysicalDeviceProperties) -> Self {
        Self {
            device_type: properties.device_type.into(),
            limits: properties.limits.into(),
        }
    }
}

pub struct Features {
}

impl From<vk::PhysicalDeviceFeatures> for Features {
    fn from(features: vk::PhysicalDeviceFeatures) -> Self {
        Self {} 
    }
}

pub enum DeviceType {
    Other,
    Integrated,
    Discrete,
}

impl From<vk::PhysicalDeviceType> for DeviceType {
    fn from(ty: vk::PhysicalDeviceType) -> Self {
        match ty {
            vk::PhysicalDeviceType::Integrated => Self::Integrated,
            vk::PhysicalDeviceType::Discrete => Self::Discrete,
            _ => Self::Other,
        }
    }
}

pub struct Limits {
    pub max_image_dimension_2d: usize,
}

impl From<vk::PhysicalDeviceLimits> for Limits {
    fn from(limits: vk::PhysicalDeviceLimits) -> Self {
        Self {
            max_image_dimension_2d: limits.max_image_dimension_2d,
        }
    }
}

impl Device<'_> {
    pub fn select(info: DeviceInfo) -> Result<Self> {
        let context = info.context.clone();

        let context::Context {
            instance
        } = &context;

        //SAFETY instance is initialized
        let physical_devices = unsafe { instance.enumerate_physical_devices() }
            .map_err(|_| Error::Creation)?
            .into_iter()
            .map(|physical_device| {
                let properties = instance.get_physical_device_properties(physical_device).into();
                let features = instance.get_physical_device_features(physical_device).into();

                let details = Details {
                    properties,
                    features,
                }

                let score = info.rank(details);

                (score, physical_device)
            })
            .collect::<Vec<_>>();

        physical_devices.sort_by(|(a, _), (b, _)| b.cmp(a));

        let physical_device = physical_devices.pop();
    }
}
