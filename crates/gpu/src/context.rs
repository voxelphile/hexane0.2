use crate::{Error, Result};

use std::ffi;
use std::os::raw;

use ash::extensions::ext;
use ash::{vk, Entry, Instance};

use semver::Version;

const API_VERSION: u32 = vk::make_api_version(0, 1, 0, 0);

unsafe extern "system" fn debug_callback(
    _message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    _callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut raw::c_void,
) -> vk::Bool32 {
    true as vk::Bool32
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct Context {
    pub(crate) instance: Instance,
    debug_utils: Option<(ext::DebugUtils, vk::DebugUtilsMessengerEXT)>,
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
                ..Default::default()
            }
        };

        //SAFETY String is correct
        let layers = unsafe {
            [ffi::CStr::from_bytes_with_nul_unchecked(
                b"VK_LAYER_KHRONOS_validation\0",
            )]
        };

        let extensions = [ext::DebugUtils::name()];

        let p_application_info = &application_info;

        let enabled_layer_names = layers.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();
        let enabled_layer_count = enabled_layer_names.len() as u32;

        let enabled_extension_names = extensions.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();
        let enabled_extension_count = enabled_extension_names.len() as u32;

        let instance_create_info = {
            let pp_enabled_layer_names = enabled_layer_names.as_ptr();
            let pp_enabled_extension_names = enabled_extension_names.as_ptr();

            vk::InstanceCreateInfo {
                s_type: vk::StructureType::INSTANCE_CREATE_INFO,
                p_application_info,
                enabled_layer_count,
                pp_enabled_layer_names,
                enabled_extension_count,
                pp_enabled_extension_names,
                ..Default::default()
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
                s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
                message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                pfn_user_callback: Some(debug_callback),
                ..Default::default()
            })
        } else {
            None
        };

        let debug_utils = if let Some(info) = debug_utils_messenger_create_info {
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
            instance,
            debug_utils,
        })
    }
}
