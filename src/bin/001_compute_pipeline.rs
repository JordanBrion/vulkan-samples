extern crate ash;

use ash::version::DeviceV1_0;
use ash::version::DeviceV1_1;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::vk;
use ash::vk::Device;
use ash::vk::PhysicalDevice;
use std::default::Default;
use std::ffi::CString;

use ash::extensions::khr::Swapchain;

use std::ffi::CStr;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::os::raw::c_char;

unsafe fn print_instance_layers(entry: &ash::Entry) {
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
        println!("instance xtensions name = {}", c_str);
    }
}

fn create_application() -> vk::ApplicationInfo {
    let application = vk::ApplicationInfo {
        s_type: vk::StructureType::APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: CString::new("My 1st Vulkan Rust application")
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
    application_info: vk::ApplicationInfo,
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

unsafe fn pick_up_one_gpu(instance: &ash::Instance) -> Result<vk::PhysicalDevice, &'static str> {
    let physical_devices = instance
        .enumerate_physical_devices()
        .expect("Cannot enumerate physical devices");
    println!("number of physical device {}", physical_devices.len());
    if physical_devices.is_empty() {
        Err("Cannot get physical devices because none was found")
    } else {
        Ok(physical_devices[0])
    }
}

fn pick_up_one_queue_family(
    queue_families_properties: Vec<vk::QueueFamilyProperties>,
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
    let device_queue_create_info = vk::DeviceQueueCreateInfo {
        s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        queue_family_index: index_of_queue_family as u32,
        queue_count: 1,
        p_queue_priorities: &queue_priority as *const f32,
    };
    let device_features = instance.get_physical_device_features(gpu);
    let device_create_info = vk::DeviceCreateInfo {
        s_type: vk::StructureType::DEVICE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        queue_create_info_count: 1 as u32,
        p_queue_create_infos: &device_queue_create_info as *const vk::DeviceQueueCreateInfo,
        enabled_layer_count: 0 as u32,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 0 as u32,
        pp_enabled_extension_names: std::ptr::null(),
        p_enabled_features: &device_features as *const vk::PhysicalDeviceFeatures,
    };
    let logical_device: ash::Device = instance
        .create_device(gpu, &device_create_info, None)
        .expect("Cannot create device");
    logical_device
}

unsafe fn get_memory_type_index(
    flags: vk::MemoryPropertyFlags,
    instance: &ash::Instance,
    gpu: vk::PhysicalDevice,
) -> Result<usize, &'static str> {
    let memory_properties = instance.get_physical_device_memory_properties(gpu);
    for i in 0..memory_properties.memory_types.len() {
        let memory_type: &vk::MemoryType = &memory_properties.memory_types[i];
        if memory_type.property_flags.contains(flags) {
            return Ok(i);
        }
    }
    return Err("Cannot find memory type!");
}

struct Buffer {
    vk_buffer_handle: ash::vk::Buffer,
    size: usize,
}

impl Buffer {
    unsafe fn new(logical_device: &ash::Device, size: usize) -> Self {
        let buffer_create_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::BufferCreateFlags::empty(),
            size: size as u64,
            usage: vk::BufferUsageFlags::STORAGE_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };
        let buffer = logical_device
            .create_buffer(&buffer_create_info, None)
            .expect("Failed to create Vertex Buffer");
        Buffer {
            vk_buffer_handle: buffer,
            size: size,
        }
    }
}

struct DeviceMemoryAllocator<'a> {
    size: usize,
    memory: vk::DeviceMemory,
    logical_device: &'a ash::Device,
}

impl<'a> DeviceMemoryAllocator<'a> {
    unsafe fn new(size: usize, memory_type_index: usize, logical_device: &'a ash::Device) -> Self {
        let memory_allocate_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: size as u64,
            memory_type_index: memory_type_index as u32,
        };
        let memory: vk::DeviceMemory = logical_device
            .allocate_memory(&memory_allocate_info, None)
            .expect("Cannot allocate device memomry!");
        DeviceMemoryAllocator {
            size: size,
            memory: memory,
            logical_device: logical_device,
        }
    }
    unsafe fn bind(&self, buffer: ash::vk::Buffer, offset: usize) {
        println!("size aaa {}", self.size);
        self.logical_device
            .bind_buffer_memory(buffer, self.memory, offset as u64)
            .expect("Cannot bind memory");
    }
}

unsafe fn create_command_pool(
    logical_device: &ash::Device,
    index_of_queue_family: usize,
) -> ash::vk::CommandPool {
    let command_pool_create_info = vk::CommandPoolCreateInfo {
        s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        queue_family_index: index_of_queue_family as u32,
    };
    logical_device
        .create_command_pool(&command_pool_create_info, None)
        .expect("Cannot create command pool")
}

unsafe fn allocate_command_buffers(
    logical_device: &ash::Device,
    command_pool: ash::vk::CommandPool,
    count: u32,
) -> Vec<vk::CommandBuffer> {
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        command_pool: command_pool,
        level: ash::vk::CommandBufferLevel::PRIMARY,
        command_buffer_count: count,
    };
    logical_device
        .allocate_command_buffers(&command_buffer_allocate_info)
        .expect("Cannot create command buffer")
}

unsafe fn create_descriptor_set_layout(
    logical_device: &ash::Device,
) -> ash::vk::DescriptorSetLayout {
    let descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding {
        binding: 5,
        descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::ALL,
        p_immutable_samplers: std::ptr::null(), // * const Sampler
    };
    let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
        s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        binding_count: 1,
        p_bindings: &descriptor_set_layout_binding as *const vk::DescriptorSetLayoutBinding,
    };
    logical_device
        .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
        .expect("Cannot create descriptor set layout")
}

unsafe fn create_pipeline_layout(
    logical_device: &ash::Device,
    descriptor_set_layout: ash::vk::DescriptorSetLayout,
) -> vk::PipelineLayout {
    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
        s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::PipelineLayoutCreateFlags::default(),
        set_layout_count: 1,
        p_set_layouts: &descriptor_set_layout,
        /*as *const DescriptorSetLayout*/
        push_constant_range_count: 0,
        p_push_constant_ranges: std::ptr::null(),
        /*as *const PushConstantRange*/
    };
    logical_device
        .create_pipeline_layout(&pipeline_layout_create_info, None)
        .expect("Cannot create pipeline layout")
}

unsafe fn create_compute_pipeline(
    logical_device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
) -> vk::Pipeline {
    let mut file = std::fs::File::open(
        "/home/jordanbrion/Documents/rust/vulkan-samples/shaders/001_compute_pipeline.comp.spv",
    )
    .expect("Something went wrong opening the shader");
    let spirv_data =
        ash::util::read_spv(&mut file).expect("Failed to read compute shader spv file");
    let shader_module_create_info =
        vk::ShaderModuleCreateInfo::builder().code(spirv_data.as_slice());
    let shader_module = logical_device
        .create_shader_module(&shader_module_create_info, None)
        .expect("Cannot create shader module");
    let shader_function_name = CString::new("main").expect("Shader function name not valid");
    let pipeline_shader_create_info = vk::PipelineShaderStageCreateInfo {
        s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        stage: vk::ShaderStageFlags::COMPUTE,
        module: shader_module,
        p_name: shader_function_name.as_ptr(),
        p_specialization_info: std::ptr::null(),
    };
    let compute_pipeline_create_info = vk::ComputePipelineCreateInfo {
        s_type: vk::StructureType::COMPUTE_PIPELINE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::PipelineCreateFlags::DISABLE_OPTIMIZATION,
        stage: pipeline_shader_create_info,
        layout: pipeline_layout,
        base_pipeline_handle: vk::Pipeline::null(),
        base_pipeline_index: -1,
    };
    let compute_pipeline_create_infos = &[compute_pipeline_create_info];
    logical_device
        .create_compute_pipelines(
            vk::PipelineCache::null(),
            compute_pipeline_create_infos,
            None,
        )
        .expect("Cannot get compute pipelines")[0]
}

unsafe fn create_descriptor_pool(logical_device: &ash::Device) -> ash::vk::DescriptorPool {
    let descriptor_pool_size = ash::vk::DescriptorPoolSize {
        ty: ash::vk::DescriptorType::STORAGE_BUFFER,
        descriptor_count: 1,
    };
    let descriptor_pool_create_info = ash::vk::DescriptorPoolCreateInfo {
        s_type: ash::vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(), //ash::vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
        max_sets: 1,
        pool_size_count: 1,
        p_pool_sizes: &descriptor_pool_size,
    };
    let descriptor_pool = logical_device
        .create_descriptor_pool(&descriptor_pool_create_info, None)
        .expect("Cannot create descriptor pool");
    descriptor_pool
}

unsafe fn allocate_descriptor_set(
    logical_device: &ash::Device,
    descriptor_pool: ash::vk::DescriptorPool,
    descriptor_set_layout: ash::vk::DescriptorSetLayout,
) -> ash::vk::DescriptorSet {
    let descriptor_set_allocate_infos = ash::vk::DescriptorSetAllocateInfo {
        s_type: ash::vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        descriptor_pool: descriptor_pool,
        descriptor_set_count: 1,
        p_set_layouts: &descriptor_set_layout,
    };
    logical_device
        .allocate_descriptor_sets(&descriptor_set_allocate_infos)
        .expect("Cannot allocate descriptor set")[0]
}

fn main() {
    unsafe {
        let entry = ash::Entry::new().expect("Cannot create entry");
        let instance = create_instance(&entry, create_application());
        // print_instance_extensions(&entry);
        // print_instance_layers(&entry);
        let gpu = pick_up_one_gpu(&instance).expect("Cannot get physical device");
        let queue_families_properties = instance.get_physical_device_queue_family_properties(gpu);
        let index_of_queue_family =
            pick_up_one_queue_family(queue_families_properties, ash::vk::QueueFlags::COMPUTE)
                .expect("Cannot get queue family property");
        let logical_device = create_logical_device(&instance, gpu, index_of_queue_family);
        let number_of_elements = 1024;
        let buffer_size_bytes = number_of_elements * std::mem::size_of::<u32>();
        let buffer = Buffer::new(&logical_device, buffer_size_bytes);
        let memory_type_index =
            get_memory_type_index(vk::MemoryPropertyFlags::HOST_VISIBLE, &instance, gpu)
                .expect("no memory type found");
        let allocator = DeviceMemoryAllocator::new(buffer.size, memory_type_index, &logical_device);
        allocator.bind(buffer.vk_buffer_handle, 0);
        let command_pool = create_command_pool(&logical_device, index_of_queue_family);
        let command_buffers = allocate_command_buffers(&logical_device, command_pool, 1);
        let dispatch_command_buffer = command_buffers[0];
        let queue = logical_device.get_device_queue(index_of_queue_family as u32, 0);
        let descriptor_set_layout = create_descriptor_set_layout(&logical_device);
        let pipeline_layout = create_pipeline_layout(&logical_device, descriptor_set_layout);
        let compute_pipeline = create_compute_pipeline(&logical_device, pipeline_layout);
        let descriptor_pool = create_descriptor_pool(&logical_device);
        let descriptor_set =
            allocate_descriptor_set(&logical_device, descriptor_pool, descriptor_set_layout);
        let descriptor_buffer_info = ash::vk::DescriptorBufferInfo {
            buffer: buffer.vk_buffer_handle,
            offset: 0,
            range: ash::vk::WHOLE_SIZE,
        };
        let descriptor_write = vk::WriteDescriptorSet {
            s_type: ash::vk::StructureType::WRITE_DESCRIPTOR_SET,
            p_next: std::ptr::null(),
            dst_set: descriptor_set,
            dst_binding: 5,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: ash::vk::DescriptorType::STORAGE_BUFFER,
            p_image_info: std::ptr::null(),
            p_buffer_info: &descriptor_buffer_info,
            p_texel_buffer_view: std::ptr::null(),
        };
        logical_device.update_descriptor_sets(&[descriptor_write], &[]);
        let command_buffer_begin_info = ash::vk::CommandBufferBeginInfo {
            s_type: ash::vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            p_inheritance_info: std::ptr::null(),
        };
        logical_device
            .begin_command_buffer(dispatch_command_buffer, &command_buffer_begin_info)
            .expect("Cannot begin command buffer");
        logical_device.cmd_bind_pipeline(
            dispatch_command_buffer,
            ash::vk::PipelineBindPoint::COMPUTE,
            compute_pipeline,
        );
        logical_device.cmd_bind_descriptor_sets(
            dispatch_command_buffer,
            ash::vk::PipelineBindPoint::COMPUTE,
            pipeline_layout,
            0,
            &[descriptor_set],
            &[],
        );
        logical_device.cmd_dispatch(dispatch_command_buffer, 1, 1, 1);
        logical_device
            .end_command_buffer(dispatch_command_buffer)
            .expect("Cannot end command buffer");
        let queue_submit_info = ash::vk::SubmitInfo {
            s_type: ash::vk::StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &dispatch_command_buffer,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        logical_device
            .queue_submit(queue, &[queue_submit_info], ash::vk::Fence::null())
            .expect("Cannot submit queue");
        logical_device
            .queue_wait_idle(queue)
            .expect("Cannot wait for queue");
        let p_memory = logical_device
            .map_memory(
                allocator.memory,
                0,
                ash::vk::WHOLE_SIZE,
                ash::vk::MemoryMapFlags::empty(),
            )
            .expect("Cannot map memory");
        println!("pointer value = {}", p_memory as u64);
        let gpu_array = std::slice::from_raw_parts(p_memory as *const u32, number_of_elements);
        for a in gpu_array {
            println!("value is {}", a);
        }
    }
}
