extern crate ash;

use ash::version::DeviceV1_0;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::c_char;

#[allow(dead_code)]
unsafe fn print_instance_layers(entry: &ash::Entry) {
    //&dyn ash::version::EntryV1_1<Instance = ash::vk::Instance>) {
    let instance_layers_properties = entry
        .enumerate_instance_layer_properties()
        .expect("Cannot find instance layer properties");

    for i in 0..instance_layers_properties.len() {
        let layer_name: [c_char; 256] = instance_layers_properties[i].layer_name;
        let c_str = CStr::from_ptr(layer_name.as_ptr())
            .to_str()
            .expect("Cannot convert instance layer");
        println!("instance layer name = {}", c_str);
    }
}

#[allow(dead_code)]
unsafe fn print_instance_extensions(entry: &ash::Entry) {
    let instance_extensions = entry
        .enumerate_instance_extension_properties()
        .expect("Cannot retrieve instance extension properties");
    println!("instance extensions: {}", instance_extensions.len());
    for i in 0..instance_extensions.len() {
        let extension_properties: ash::vk::ExtensionProperties = instance_extensions[i];
        let extension_name = extension_properties.extension_name;
        let c_str = CStr::from_ptr(extension_name.as_ptr())
            .to_str()
            .expect("Cannot convert instance extensions");
        println!("instance extensions name = {}", c_str);
    }
}

fn create_application() -> ash::vk::ApplicationInfo {
    let application = ash::vk::ApplicationInfo {
        s_type: ash::vk::StructureType::APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: std::ffi::CString::new("My 1st Vulkan Rust application")
            .expect("CString::new failed")
            .as_ptr(),
        application_version: ash::vk_make_version!(0, 0, 1),
        p_engine_name: CString::new("Unreal Engine 4")
            .expect("CString::new failed")
            .as_ptr(),
        engine_version: ash::vk_make_version!(0, 0, 1),
        api_version: ash::vk_make_version!(1, 0, 0),
    };
    application
}

unsafe fn create_instance(
    entry: &ash::Entry,
    application_info: ash::vk::ApplicationInfo,
) -> ash::Instance {
    let layers = vec![CString::new("VK_LAYER_KHRONOS_validation").expect("Cannot create c-string")];
    let instance_create_info = ash::vk::InstanceCreateInfo {
        s_type: ash::vk::StructureType::INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        p_application_info: &application_info,
        enabled_layer_count: layers.len() as u32,
        pp_enabled_layer_names: layers.as_ptr() as *const *const c_char,
        enabled_extension_count: 0,
        pp_enabled_extension_names: std::ptr::null(),
    };
    let instance: ash::Instance = entry
        .create_instance(&instance_create_info, None)
        .expect("Instance creation error");
    instance
}

unsafe fn pick_up_one_gpu(
    instance: &ash::Instance,
) -> Result<ash::vk::PhysicalDevice, &'static str> {
    let physical_devices = instance
        .enumerate_physical_devices()
        .expect("Cannot enumerate physical devices");
    if physical_devices.is_empty() {
        Err("Cannot get physical devices because none was found")
    } else {
        Ok(physical_devices[0])
    }
}

fn pick_up_one_queue_family(
    queue_families_properties: Vec<ash::vk::QueueFamilyProperties>,
    flags: ash::vk::QueueFlags,
) -> Result<usize, &'static str> {
    for i in 0..queue_families_properties.len() {
        if queue_families_properties[i].queue_flags & flags == flags {
            return Ok(i);
        }
    }
    Err("No suitable queue family found")
}

unsafe fn create_logical_device(
    instance: &ash::Instance,
    gpu: ash::vk::PhysicalDevice,
    index_of_queue_family: usize,
) -> ash::Device {
    let queue_priority = 1.0_f32;
    let device_queue_create_info = ash::vk::DeviceQueueCreateInfo {
        s_type: ash::vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        queue_family_index: index_of_queue_family as u32,
        queue_count: 1,
        p_queue_priorities: &queue_priority as *const f32,
    };
    let device_features = instance.get_physical_device_features(gpu);
    let device_create_info = ash::vk::DeviceCreateInfo {
        s_type: ash::vk::StructureType::DEVICE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        queue_create_info_count: 1 as u32,
        p_queue_create_infos: &device_queue_create_info as *const ash::vk::DeviceQueueCreateInfo,
        enabled_layer_count: 0 as u32,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 0 as u32,
        pp_enabled_extension_names: std::ptr::null(),
        p_enabled_features: &device_features as *const ash::vk::PhysicalDeviceFeatures,
    };
    let logical_device: ash::Device = instance
        .create_device(gpu, &device_create_info, None)
        .expect("Cannot create device");
    logical_device
}

fn main() {
    unsafe {
        let entry = ash::Entry::new().expect("Cannot create entry");
        // print_instance_extensions(&entry);
        // print_instance_layers(&entry);
        let instance = create_instance(&entry, create_application());
        let gpu = pick_up_one_gpu(&instance).expect("Cannot get physical device");
        let queue_families_properties = instance.get_physical_device_queue_family_properties(gpu);
        let index_of_queue_family =
            pick_up_one_queue_family(queue_families_properties, ash::vk::QueueFlags::COMPUTE)
                .expect("Cannot get queue family property");
        let logical_device = create_logical_device(&instance, gpu, index_of_queue_family);

        // line is commented to trigger error from validation layer
        // logical_device.destroy_device(None);
        
        instance.destroy_instance(None);
    }
}
