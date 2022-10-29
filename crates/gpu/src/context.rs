use crate::prelude::*;

use std::borrow;
use std::ffi;
use std::os::raw;
use std::default::default;

use ash::extensions::{ext, khr};
use ash::{vk, Entry, Instance};

use semver::Version;

use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};

const API_VERSION: u32 = vk::make_api_version(0, 1, 0, 0);

const DESCRIPTOR_COUNT: u32 = 64;

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number: i32 = callback_data.message_id_number as i32;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        borrow::Cow::from("")
    } else {
        ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        borrow::Cow::from("")
    } else {
        ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity,
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message,
    );

    vk::FALSE
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct Context {
    pub(crate) entry: Entry,
    pub(crate) instance: Instance,
    debug: Option<(ext::DebugUtils, vk::DebugUtilsMessengerEXT)>,
}

pub struct ContextInfo<'a> {
    pub enable_validation: bool,
    pub application_name: &'a str,
    pub application_version: Version,
    pub engine_name: &'a str,
    pub engine_version: Version,
}

impl Default for ContextInfo<'_> {
    fn default() -> Self {
        Self {
            enable_validation: false,
            application_name: "",
            application_version: Version::new(0, 1, 0),
            engine_name: "",
            engine_version: Version::new(0, 1, 0),
        }
    }
}

impl Context {
    pub fn new(info: ContextInfo<'_>) -> Result<Self> {
        let entry = Entry::linked();

        let application_name =
            ffi::CString::new(info.application_name).map_err(|_| Error::Creation)?;

        let application_version = vk::make_api_version(
            0,
            info.application_version.major as u32,
            info.application_version.minor as u32,
            info.application_version.patch as u32,
        );

        let engine_name = ffi::CString::new(info.engine_name).map_err(|_| Error::Creation)?;

        let engine_version = vk::make_api_version(
            0,
            info.engine_version.major as u32,
            info.engine_version.minor as u32,
            info.engine_version.patch as u32,
        );

        let application_info = {
            let p_application_name = application_name.as_c_str().as_ptr();
            let p_engine_name = engine_name.as_c_str().as_ptr();

            vk::ApplicationInfo {
                api_version: API_VERSION,
                p_application_name,
                application_version,
                p_engine_name,
                engine_version,
                ..default()
            }
        };

        //SAFETY String is correct
        let mut layers = vec![];

        if info.enable_validation {
            layers.push(unsafe {
                ffi::CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0")
            });
        }

        let mut extensions = vec![ext::DebugUtils::name(), khr::Surface::name()];

        #[cfg(target_os = "windows")]
        extensions.push(khr::Win32Surface::name());

        let p_application_info = &application_info;

        let enabled_layer_names = layers.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();
        let enabled_layer_count = enabled_layer_names.len() as u32;

        let enabled_extension_names = extensions.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();
        let enabled_extension_count = enabled_extension_names.len() as u32;

        let instance_create_info = {
            let pp_enabled_layer_names = enabled_layer_names.as_ptr();
            let pp_enabled_extension_names = enabled_extension_names.as_ptr();

            vk::InstanceCreateInfo {
                p_application_info,
                enabled_layer_count,
                pp_enabled_layer_names,
                enabled_extension_count,
                pp_enabled_extension_names,
                ..default()
            }
        };

        //SAFETY this is correct
        let instance = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .map_err(|_| Error::Creation)?
        };

        let debug_utils_messenger_create_info = if info.enable_validation {
            Some(vk::DebugUtilsMessengerCreateInfoEXT {
                message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                pfn_user_callback: Some(debug_callback),
                ..default()
            })
        } else {
            None
        };

        let debug = if let Some(info) = debug_utils_messenger_create_info {
            let loader = ext::DebugUtils::new(&entry, &instance);
            //SAFETY this is correct, idk what else to say lol
            let callback = unsafe {
                loader
                    .create_debug_utils_messenger(&info, None)
                    .map_err(|_| Error::Creation)?
            };
            Some((loader, callback))
        } else {
            None
        };

        Ok(Self {
            entry,
            instance,
            debug,
        })
    }
    pub fn create_device(&self, info: DeviceInfo<'_>) -> Result<Device<'_>> {
        let Context {
            entry, instance, ..
        } = &self;

        let surface_loader = khr::Surface::new(entry, instance);
        let surface_handle =
            unsafe { ash_window::create_surface(entry, instance, info.display, info.window, None) }
                .map_err(|_| Error::Creation)?;

        //SAFETY instance is initialized
        let mut physical_devices = unsafe { instance.enumerate_physical_devices() }
            .map_err(|_| Error::Creation)?
            .into_iter()
            .filter_map(|physical_device| {
                unsafe { instance.get_physical_device_queue_family_properties(physical_device) }
                    .into_iter()
                    .enumerate()
                    .find_map(|(index, info)| {
                        let graphics_support = info.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                        let compute_support = info.queue_flags.contains(vk::QueueFlags::COMPUTE);
                        let surface_support = unsafe {
                            surface_loader.get_physical_device_surface_support(
                                physical_device,
                                index as u32,
                                surface_handle,
                            )
                        }
                        .ok()?;

                        if graphics_support && compute_support && surface_support {
                            Some((physical_device, index))
                        } else {
                            None
                        }
                    })
            })
            .map(|(physical_device, index)| {
                let properties =
                    unsafe { instance.get_physical_device_properties(physical_device) }.into();
                let features =
                    unsafe { instance.get_physical_device_features(physical_device) }.into();

                let details = crate::device::Details {
                    properties,
                    features,
                };

                let selector = info.selector;

                let score = selector(details);

                (score, physical_device, index)
            })
            .collect::<Vec<_>>();

        physical_devices.sort_by(|(a, _, _), (b, _, _)| b.cmp(a));

        let Some((_, physical_device, queue_family_index)) = physical_devices.pop() else {
            panic!("no suitable device found");
        };

        let queue_family_index = queue_family_index as u32;

        let queue_family_indices = vec![queue_family_index];

        let mut layers = vec![];

        if self.debug.is_some() {
            layers.push(unsafe {
                ffi::CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0")
            });
        }

        let extensions = [khr::Swapchain::name()];

        let features = info.features.into();

        let priorities = [1.0];

        let device_queue_create_infos = [{
            let queue_count = 1;
            let p_queue_priorities = priorities.as_ptr();

            vk::DeviceQueueCreateInfo {
                queue_family_index,
                queue_count,
                p_queue_priorities,
                ..default()
            }
        }];

        let enabled_layer_names = layers.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();
        let enabled_layer_count = enabled_layer_names.len() as u32;

        let enabled_extension_names = extensions.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();
        let enabled_extension_count = enabled_extension_names.len() as u32;

        let device_create_info = {
            let queue_create_info_count = device_queue_create_infos.len() as _;
            let p_queue_create_infos = device_queue_create_infos.as_ptr();

            let pp_enabled_layer_names = enabled_layer_names.as_ptr();
            let pp_enabled_extension_names = enabled_extension_names.as_ptr();

            let p_enabled_features = &features;

            vk::DeviceCreateInfo {
                queue_create_info_count,
                p_queue_create_infos,
                enabled_layer_count,
                pp_enabled_layer_names,
                enabled_extension_count,
                pp_enabled_extension_names,
                p_enabled_features,
                ..default()
            }
        };

        let logical_device = unsafe {
            self.instance
                .create_device(physical_device, &device_create_info, None)
        }
        .map_err(|_| Error::Creation)?;

        let descriptor_pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLER,
                descriptor_count: DESCRIPTOR_COUNT,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: DESCRIPTOR_COUNT,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: DESCRIPTOR_COUNT,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: DESCRIPTOR_COUNT,
            },
        ];

        let descriptor_pool_create_info = {
            let flags = vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND;

            let max_sets = 1;

            let pool_size_count = descriptor_pool_sizes.len() as u32;

            let p_pool_sizes = descriptor_pool_sizes.as_ptr();

            vk::DescriptorPoolCreateInfo {
                flags,
                max_sets,
                pool_size_count,
                p_pool_sizes,
                ..default()
            }
        };

        let descriptor_pool = unsafe { logical_device.create_descriptor_pool(&descriptor_pool_create_info, None) }.map_err(|_| Error::Creation)?;

        let descriptor_set_layout_bindings = [
            vk::DescriptorSetLayoutBinding {
                descriptor_type: vk::DescriptorType::SAMPLER,
                descriptor_count: DESCRIPTOR_COUNT,
                stage_flags: vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT,
                ..default()
            },
            vk::DescriptorSetLayoutBinding {
                descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: DESCRIPTOR_COUNT,
                stage_flags: vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT,
                ..default()
            },
            vk::DescriptorSetLayoutBinding {
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: DESCRIPTOR_COUNT,
                stage_flags: vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT,
                ..default()
            },
            vk::DescriptorSetLayoutBinding {
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: DESCRIPTOR_COUNT,
                stage_flags: vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT,
                ..default()
            },
        ]; 

        let descriptor_set_layout_create_info = {
            let binding_count = descriptor_set_layout_bindings.len() as u32;

            let p_bindings = descriptor_set_layout_bindings.as_ptr();

            vk::DescriptorSetLayoutCreateInfo {
                binding_count,
                p_bindings,
                ..default()
            }
        };

        let descriptor_set_layout = unsafe { logical_device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None) }.map_err(|_| Error::Creation)?;

        let descriptor_set_allocate_info = {
            let descriptor_set_count = 1;

            let set_layouts = [descriptor_set_layout];

            let p_set_layouts = set_layouts.as_ptr();

            vk::DescriptorSetAllocateInfo {
                descriptor_pool,
                descriptor_set_count,
                p_set_layouts,
                ..default()
            }
        };

        let descriptor_set = unsafe { logical_device.allocate_descriptor_sets(&descriptor_set_allocate_info) }.map_err(|_| Error::Creation)?[0];
        
        Ok(Device {
            context: &self,
            surface: (surface_loader, surface_handle),
            physical_device,
            logical_device,
            queue_family_indices,
            descriptor_pool,
            descriptor_set,
        })
    }
}
