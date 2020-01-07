extern crate ash;
extern crate core;
extern crate jpeg_decoder as jpeg;
extern crate nalgebra_glm as glm;
extern crate num;
extern crate png;
extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::time::Duration;

use std::ffi::CString;

use core::convert::Into;

use ash::version::DeviceV1_0;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::vk::Handle;

#[repr(C)]
struct MyPointData {
    position: glm::Vec3,
    color: glm::Vec3,
    uv: glm::Vec2,
}

#[repr(C)]
struct MyUniformBuffer {
    m_model: glm::Mat4,
    m_view: glm::Mat4,
    m_projection: glm::Mat4,
}

unsafe fn create_instance(entry: &ash::Entry, v_extensions: Vec<&str>) -> ash::Instance {
    let v_layers =
        vec![CString::new("VK_LAYER_KHRONOS_validation").expect("Cannot validation layer name")];
    let application_name = CString::new("003_swapchain").expect("Cannot create application name");
    let engine_name = CString::new("Not Unreal Engine 4").expect("Cannot create engine name");
    let application_info = ash::vk::ApplicationInfo {
        s_type: ash::vk::StructureType::APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: application_name.as_ptr(),
        application_version: ash::vk_make_version!(1, 0, 0),
        p_engine_name: engine_name.as_ptr(),
        engine_version: ash::vk_make_version!(0, 0, 1),
        api_version: ash::vk_make_version!(1, 0, 0),
    };
    let v_extensions_c: Vec<*const u8> = v_extensions.iter().map(|ss| ss.as_ptr()).collect();
    let instance_create_info = ash::vk::InstanceCreateInfo {
        s_type: ash::vk::StructureType::INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        p_application_info: &application_info,
        enabled_layer_count: v_layers.len() as u32,
        pp_enabled_layer_names: v_layers.as_ptr() as *const *const i8,
        enabled_extension_count: v_extensions_c.len() as u32,
        pp_enabled_extension_names: v_extensions_c.as_ptr() as *const *const i8,
    };
    entry
        .create_instance(&instance_create_info, None)
        .expect("Cannot create instance")
}

unsafe fn pick_up_one_gpu(instance: &ash::Instance) -> Option<ash::vk::PhysicalDevice> {
    match instance.enumerate_physical_devices() {
        Ok(ref gpus) if gpus.len() > 0 => Some(gpus[0]),
        Ok(_) => None,
        Err(_e) => None,
    }
}

unsafe fn lookup_queue_family_index(
    instance: &ash::Instance,
    gpu: &ash::vk::PhysicalDevice,
) -> Result<usize, &'static str> {
    let queue_family_properties = instance.get_physical_device_queue_family_properties(*gpu);
    for i in 0..queue_family_properties.len() {
        if queue_family_properties[i]
            .queue_flags
            .contains(ash::vk::QueueFlags::GRAPHICS)
        {
            return Ok(i);
        }
    }
    Err("Queue family not found")
}

unsafe fn create_logical_device(
    instance: &ash::Instance,
    gpu: &ash::vk::PhysicalDevice,
    index_of_queue_family: usize,
) -> Result<ash::Device, ash::vk::Result> {
    let priority = 1.0_f32;
    let queue_create_info = ash::vk::DeviceQueueCreateInfo {
        s_type: ash::vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        queue_family_index: index_of_queue_family as u32,
        queue_count: 1,
        p_queue_priorities: &priority,
    };

    let mut v_extensions = Vec::new();
    v_extensions.push(ash::extensions::khr::Swapchain::name());
    let v_extensions_c = v_extensions.iter().map(|e| e.as_ptr() as *const i8);

    let physical_device_features = ash::vk::PhysicalDeviceFeatures::builder()
        .sampler_anisotropy(true)
        .build();

    let device_create_info = ash::vk::DeviceCreateInfo {
        s_type: ash::vk::StructureType::DEVICE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        queue_create_info_count: 1,
        p_queue_create_infos: &queue_create_info,
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: v_extensions_c.len() as u32,
        pp_enabled_extension_names: v_extensions.as_ptr() as *const *const i8,
        p_enabled_features: &physical_device_features,
    };
    instance.create_device(*gpu, &device_create_info, None)
}

unsafe fn create_shader_module(
    logical_device: &ash::Device,
    shaderPath: &str,
) -> ash::vk::ShaderModule {
    let mut shader_files =
        std::fs::File::open(shaderPath).expect("Something went wrong when opening shader");
    let shader_instructions =
        ash::util::read_spv(&mut shader_files).expect("Failed to read shader spv file");
    let shader_module_create_infos =
        ash::vk::ShaderModuleCreateInfo::builder().code(shader_instructions.as_slice());
    logical_device
        .create_shader_module(&shader_module_create_infos, None)
        .expect("Cannot create shader module")
}

unsafe fn search_physical_device_memory_type(
    instance: &ash::Instance,
    gpu: &ash::vk::PhysicalDevice,
    requirements: &ash::vk::MemoryRequirements,
    type_to_find: ash::vk::MemoryPropertyFlags,
) -> Result<usize, &'static str> {
    let memory_properties = instance.get_physical_device_memory_properties(*gpu);
    for (index, memory_type) in memory_properties.memory_types.iter().enumerate() {
        if requirements.memory_type_bits & (1 << index) > 0
            && memory_type.property_flags.contains(type_to_find)
        {
            return Ok(index);
        }
    }
    Err("Cannot find device memory type")
}

fn search_format(
    v_surface_formats: &Vec<ash::vk::SurfaceFormatKHR>,
) -> Result<&ash::vk::SurfaceFormatKHR, &'static str> {
    for format in v_surface_formats {
        if format.format == ash::vk::Format::B8G8R8A8_UNORM
            && format.color_space == ash::vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return Ok(format);
        }
    }
    Err("Cannot find surface format")
}

fn choose_swapchain_present_mode(
    v_present_modes: &Vec<ash::vk::PresentModeKHR>,
) -> ash::vk::PresentModeKHR {
    return match v_present_modes.iter().find(|mode| {
        return **mode == ash::vk::PresentModeKHR::MAILBOX;
    }) {
        Some(mode) => *mode,
        None => ash::vk::PresentModeKHR::FIFO,
    };
}

fn handle_events(event_pump: &mut sdl2::EventPump) -> bool {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => return false,
            _ => return true,
        }
    }
    true
}

unsafe fn update_uniform_buffer(
    logical_device: &ash::Device,
    memory: &ash::vk::DeviceMemory,
    matrices: &mut MyUniformBuffer,
) {
    matrices.m_model = glm::rotate(&matrices.m_model, 0.01, &glm::vec3(0.0, 1.0, 0.0));
    let p_data = logical_device
        .map_memory(
            *memory,
            0,
            std::mem::size_of::<MyUniformBuffer>() as ash::vk::DeviceSize,
            Default::default(),
        )
        .expect("Cannot map device memory");
    std::ptr::copy_nonoverlapping(
        matrices as *const MyUniformBuffer as *const std::ffi::c_void,
        p_data,
        std::mem::size_of::<MyUniformBuffer>(),
    );
    logical_device.unmap_memory(*memory);
}

unsafe fn change_image_layout(
    logical_device: &ash::Device,
    command_pool: &ash::vk::CommandPool,
    image: &ash::vk::Image,
    src_access: ash::vk::AccessFlags,
    dst_access: ash::vk::AccessFlags,
    src_pipeline_stage: ash::vk::PipelineStageFlags,
    dst_pipeline_stage: ash::vk::PipelineStageFlags,
    old_layout: ash::vk::ImageLayout,
    new_layout: ash::vk::ImageLayout,
    queue: &ash::vk::Queue,
) {
    let command_buffer_allocate_info = ash::vk::CommandBufferAllocateInfo {
        s_type: ash::vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        command_pool: *command_pool,
        level: ash::vk::CommandBufferLevel::PRIMARY,
        command_buffer_count: 1,
    };
    let command_buffer = logical_device
        .allocate_command_buffers(&command_buffer_allocate_info)
        .expect("Cannot allocate command buffers to change image layout")[0];
    let command_buffer_begin_info = ash::vk::CommandBufferBeginInfo {
        s_type: ash::vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        p_inheritance_info: std::ptr::null(),
    };
    logical_device
        .begin_command_buffer(command_buffer, &command_buffer_begin_info)
        .expect("Cannot begin command buffer to change image layout");
    let image_resource_range = ash::vk::ImageSubresourceRange {
        aspect_mask: ash::vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    };
    let image_memory_barrier = ash::vk::ImageMemoryBarrier {
        s_type: ash::vk::StructureType::IMAGE_MEMORY_BARRIER,
        p_next: std::ptr::null(),
        src_access_mask: src_access,
        dst_access_mask: dst_access,
        old_layout: old_layout,
        new_layout: new_layout,
        src_queue_family_index: ash::vk::QUEUE_FAMILY_IGNORED,
        dst_queue_family_index: ash::vk::QUEUE_FAMILY_IGNORED,
        image: *image,
        subresource_range: image_resource_range,
    };
    logical_device.cmd_pipeline_barrier(
        command_buffer,
        src_pipeline_stage,
        dst_pipeline_stage,
        Default::default(),
        &[],
        &[],
        &[image_memory_barrier],
    );
    logical_device
        .end_command_buffer(command_buffer)
        .expect("Cannot end command buffer to change image layout");
    let submit_info = ash::vk::SubmitInfo {
        s_type: ash::vk::StructureType::SUBMIT_INFO,
        p_next: std::ptr::null(),
        wait_semaphore_count: 0,
        p_wait_semaphores: std::ptr::null(),
        p_wait_dst_stage_mask: std::ptr::null(),
        command_buffer_count: 1,
        p_command_buffers: &command_buffer,
        signal_semaphore_count: 0,
        p_signal_semaphores: std::ptr::null(),
    };
    logical_device
        .queue_submit(*queue, &[submit_info], ash::vk::Fence::null())
        .expect("Cannot submit command to change image layout");
    logical_device
        .queue_wait_idle(*queue)
        .expect("Cannot wait for queue to change image layout");
    logical_device.free_command_buffers(*command_pool, &[command_buffer]);
}

unsafe fn copy_buffer_to_image(
    logical_device: &ash::Device,
    command_pool: &ash::vk::CommandPool,
    src_buffer: &ash::vk::Buffer,
    dst_image: &ash::vk::Image,
    image_layout: ash::vk::ImageLayout,
    image_width: u32,
    image_height: u32,
    queue: &ash::vk::Queue,
) {
    let command_buffer_copy_image_allocate_info = ash::vk::CommandBufferAllocateInfo {
        s_type: ash::vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        command_pool: *command_pool,
        level: ash::vk::CommandBufferLevel::PRIMARY,
        command_buffer_count: 1,
    };
    let command_buffer_copy_image = logical_device
        .allocate_command_buffers(&command_buffer_copy_image_allocate_info)
        .expect("Cannot allocate command buffer to copy texture image")[0];

    let command_buffer_copy_image_begin_info = ash::vk::CommandBufferBeginInfo {
        s_type: ash::vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
        p_next: std::ptr::null(),
        flags: ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        p_inheritance_info: std::ptr::null(),
    };
    logical_device
        .begin_command_buffer(
            command_buffer_copy_image,
            &command_buffer_copy_image_begin_info,
        )
        .expect("Cannot begin command buffer to copy texture buffer to image");
    let buffer_image_copy_region = ash::vk::BufferImageCopy {
        buffer_offset: 0,
        buffer_row_length: 0,
        buffer_image_height: 0,
        image_subresource: ash::vk::ImageSubresourceLayers {
            aspect_mask: ash::vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        },
        image_offset: ash::vk::Offset3D { x: 0, y: 0, z: 0 },
        image_extent: ash::vk::Extent3D {
            width: image_width,
            height: image_height,
            depth: 1,
        },
    };

    logical_device.cmd_copy_buffer_to_image(
        command_buffer_copy_image,
        *src_buffer,
        *dst_image,
        image_layout,
        &[buffer_image_copy_region],
    );

    logical_device
        .end_command_buffer(command_buffer_copy_image)
        .expect("Cannot end command buffer to copy texture buffer to image");

    let submit_info = ash::vk::SubmitInfo {
        s_type: ash::vk::StructureType::SUBMIT_INFO,
        p_next: std::ptr::null(),
        wait_semaphore_count: 0,
        p_wait_semaphores: std::ptr::null(),
        p_wait_dst_stage_mask: std::ptr::null(),
        command_buffer_count: 1,
        p_command_buffers: &command_buffer_copy_image,
        signal_semaphore_count: 0,
        p_signal_semaphores: std::ptr::null(),
    };

    logical_device
        .queue_submit(*queue, &[submit_info], ash::vk::Fence::null())
        .expect("Cannot submit command to change image layout");

    logical_device
        .queue_wait_idle(*queue)
        .expect("Cannot wait for queue to change image layout");
    logical_device.free_command_buffers(*command_pool, &[command_buffer_copy_image]);
}

const FRAME_COUNT: usize = 2;
fn main() {
    unsafe {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let window_width = 1280;
        let window_height = 720;
        let window = video_subsystem
            .window("rust-sdl2 demo", window_width, window_height)
            .vulkan()
            .position_centered()
            .build()
            .expect("Cannot build window!");

        let entry = ash::Entry::new().expect("Cannot create entry");
        let instance = create_instance(
            &entry,
            window
                .vulkan_instance_extensions()
                .expect("Cannot get instance extensions!"),
        );
        let gpu = pick_up_one_gpu(&instance).expect("Cannot find GPU");
        let index_of_queue_family =
            lookup_queue_family_index(&instance, &gpu).expect("Cannot find graphics queue family");
        let logical_device = create_logical_device(&instance, &gpu, index_of_queue_family)
            .expect("Cannot create logical device");
        let queue = logical_device.get_device_queue(index_of_queue_family as u32, 0);

        let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);
        let surface_handle = window
            .vulkan_create_surface(instance.handle().as_raw() as usize)
            .expect("Cannot create surface");
        let surface = ash::vk::SurfaceKHR::from_raw(surface_handle);

        let presentation_supported = surface_loader.get_physical_device_surface_support(
            gpu,
            index_of_queue_family as u32,
            surface,
        );
        if !presentation_supported {
            println!("Presentation not supported !");
            return;
        }

        let surface_capabilities = surface_loader
            .get_physical_device_surface_capabilities(gpu, surface)
            .expect("Cannot get surface capabilities");
        let v_surface_formats = surface_loader
            .get_physical_device_surface_formats(gpu, surface)
            .expect("Cannot get physical device surface formats");
        let v_surface_present_modes = surface_loader
            .get_physical_device_surface_present_modes(gpu, surface)
            .expect("Cannot get surface present mode");
        let available_format =
            search_format(&v_surface_formats).expect("Cannot find surface format");
        let image_count = if surface_capabilities.max_image_count > 0
            && surface_capabilities.min_image_count + 1 > surface_capabilities.max_image_count
        {
            surface_capabilities.max_image_count
        } else {
            surface_capabilities.min_image_count + 1
        };

        let extent = if surface_capabilities.current_extent.width != !(0 as u32) {
            surface_capabilities.current_extent
        } else {
            ash::vk::Extent2D {
                width: num::clamp(
                    window_width,
                    surface_capabilities.min_image_extent.width,
                    surface_capabilities.max_image_extent.width,
                ),
                height: num::clamp(
                    window_height,
                    surface_capabilities.min_image_extent.height,
                    surface_capabilities.max_image_extent.height,
                ),
            }
        };

        let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &logical_device);
        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR {
            s_type: ash::vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            p_next: std::ptr::null(),
            flags: Default::default(),
            surface: surface,
            min_image_count: image_count,
            image_format: available_format.format,
            image_color_space: available_format.color_space,
            image_extent: extent,
            image_array_layers: 1,
            image_usage: ash::vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: ash::vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            pre_transform: surface_capabilities.current_transform,
            composite_alpha: ash::vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: choose_swapchain_present_mode(&v_surface_present_modes),
            clipped: ash::vk::TRUE,
            old_swapchain: ash::vk::SwapchainKHR::null(),
        };

        let swapchain = swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .expect("Cannot create swapchain");
        let v_swapchain_images = swapchain_loader
            .get_swapchain_images(swapchain)
            .expect("Cannot get swapchain images");
        let swapchain_size = v_swapchain_images.len();
        let component_mapping = ash::vk::ComponentMapping {
            r: ash::vk::ComponentSwizzle::IDENTITY,
            g: ash::vk::ComponentSwizzle::IDENTITY,
            b: ash::vk::ComponentSwizzle::IDENTITY,
            a: ash::vk::ComponentSwizzle::IDENTITY,
        };

        let subresource_range = ash::vk::ImageSubresourceRange {
            aspect_mask: ash::vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };
        let shader_entry_name =
            CString::new("main").expect("Cannot create vertex shader entry name");
        let v_pipeline_shader_stage_create_infos = [
                    ash::vk::PipelineShaderStageCreateInfo {
                        s_type: ash::vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                        p_next: std::ptr::null(),
                        flags: Default::default(),
                        stage: ash::vk::ShaderStageFlags::VERTEX,
                        module: create_shader_module(&logical_device, "shaders/007_textured_triangle.vert.spv"),
                        p_name: shader_entry_name.as_ptr(),
                        p_specialization_info: std::ptr::null(),
                    },
                    ash::vk::PipelineShaderStageCreateInfo {
                        s_type: ash::vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                        p_next: std::ptr::null(),
                        flags: Default::default(),
                        stage: ash::vk::ShaderStageFlags::FRAGMENT,
                        module: create_shader_module(&logical_device, "shaders/007_textured_triangle.frag.spv"),
                        p_name: shader_entry_name.as_ptr(),
                        p_specialization_info: std::ptr::null(),
                    },
                ];

        let vertex_input_binding_description = ash::vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<MyPointData>() as u32,
            input_rate: ash::vk::VertexInputRate::VERTEX,
        };

        let v_vertex_input_attribute_description = &[
            ash::vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: ash::vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            ash::vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: ash::vk::Format::R32G32B32_SFLOAT,
                offset: std::mem::size_of::<glm::Vec3>() as u32,
            },
            ash::vk::VertexInputAttributeDescription {
                location: 3,
                binding: 0,
                format: ash::vk::Format::R32G32_SFLOAT,
                offset: (std::mem::size_of::<glm::Vec3>() + std::mem::size_of::<glm::Vec3>())
                    as u32,
            },
        ];

        let vertex_input_state_create_info = ash::vk::PipelineVertexInputStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: &vertex_input_binding_description,
            vertex_attribute_description_count: v_vertex_input_attribute_description.len() as u32,
            p_vertex_attribute_descriptions: v_vertex_input_attribute_description.as_ptr(),
        };

        let input_assembly_state_create_info = ash::vk::PipelineInputAssemblyStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            topology: ash::vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: ash::vk::FALSE,
        };

        let viewport = ash::vk::Viewport {
            x: 0f32,
            y: 0f32,
            width: window_width as f32,
            height: window_height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };

        let scissor = ash::vk::Rect2D {
            offset: ash::vk::Offset2D { x: 0, y: 0 },
            extent: ash::vk::Extent2D {
                width: extent.width,
                height: extent.height,
            },
        };

        let viewport_state_create_info = ash::vk::PipelineViewportStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            viewport_count: 1,
            p_viewports: &viewport,
            scissor_count: 1,
            p_scissors: &scissor,
        };

        let rasterization_state_create_info = ash::vk::PipelineRasterizationStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            depth_clamp_enable: ash::vk::FALSE,
            rasterizer_discard_enable: ash::vk::FALSE,
            polygon_mode: ash::vk::PolygonMode::FILL,
            cull_mode: ash::vk::CullModeFlags::NONE,
            front_face: ash::vk::FrontFace::CLOCKWISE,
            depth_bias_enable: ash::vk::FALSE,
            depth_bias_constant_factor: 0f32,
            depth_bias_clamp: 0f32,
            depth_bias_slope_factor: 0f32,
            line_width: 1f32,
        };

        let multisample_state_create_info = ash::vk::PipelineMultisampleStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            rasterization_samples: ash::vk::SampleCountFlags::TYPE_1,
            sample_shading_enable: ash::vk::FALSE,
            min_sample_shading: 0f32,
            p_sample_mask: std::ptr::null(),
            alpha_to_coverage_enable: ash::vk::FALSE,
            alpha_to_one_enable: ash::vk::FALSE,
        };

        let depth_stencil_state_create_info = ash::vk::PipelineDepthStencilStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            depth_test_enable: ash::vk::TRUE,
            depth_write_enable: ash::vk::TRUE,
            depth_compare_op: ash::vk::CompareOp::LESS,
            depth_bounds_test_enable: ash::vk::FALSE,
            stencil_test_enable: ash::vk::FALSE,
            front: Default::default(),
            back: Default::default(),
            min_depth_bounds: 0f32,
            max_depth_bounds: 1f32,
        };

        let color_blend_attachment = ash::vk::PipelineColorBlendAttachmentState {
            blend_enable: ash::vk::FALSE,
            src_color_blend_factor: ash::vk::BlendFactor::ONE,
            dst_color_blend_factor: ash::vk::BlendFactor::ZERO,
            color_blend_op: ash::vk::BlendOp::ADD,
            src_alpha_blend_factor: ash::vk::BlendFactor::ONE,
            dst_alpha_blend_factor: ash::vk::BlendFactor::ZERO,
            alpha_blend_op: ash::vk::BlendOp::ADD,
            color_write_mask: ash::vk::ColorComponentFlags::R
                | ash::vk::ColorComponentFlags::G
                | ash::vk::ColorComponentFlags::B
                | ash::vk::ColorComponentFlags::A,
        };

        let color_blend_state_create_info = ash::vk::PipelineColorBlendStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            logic_op_enable: ash::vk::FALSE,
            logic_op: ash::vk::LogicOp::COPY,
            attachment_count: 1,
            p_attachments: &color_blend_attachment,
            blend_constants: [0f32; 4],
        };

        let dynamic_state_create_info = ash::vk::PipelineDynamicStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            dynamic_state_count: 0 as u32,
            p_dynamic_states: std::ptr::null(),
        };

        let uniform_buffer_binding_number = 5;
        let texture_image_binding_number = 10;
        let v_descriptor_set_layout_binding = &[
            ash::vk::DescriptorSetLayoutBinding {
                binding: uniform_buffer_binding_number,
                descriptor_type: ash::vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: ash::vk::ShaderStageFlags::VERTEX,
                p_immutable_samplers: std::ptr::null(),
            },
            ash::vk::DescriptorSetLayoutBinding {
                binding: texture_image_binding_number,
                descriptor_type: ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: ash::vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
            },
        ];

        let descriptor_set_layout_create_info = ash::vk::DescriptorSetLayoutCreateInfo {
            s_type: ash::vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            binding_count: v_descriptor_set_layout_binding.len() as u32,
            p_bindings: v_descriptor_set_layout_binding.as_ptr(),
        };

        let descriptor_set_layout = logical_device
            .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
            .expect("Cannot create descriptor set layout");

        let v_descriptor_pool_size = &[
            ash::vk::DescriptorPoolSize {
                ty: ash::vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: swapchain_size as u32,
            },
            ash::vk::DescriptorPoolSize {
                ty: ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: swapchain_size as u32,
            },
        ];

        let descriptor_pool_create_info = ash::vk::DescriptorPoolCreateInfo {
            s_type: ash::vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: ash::vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
            max_sets: swapchain_size as u32,
            pool_size_count: v_descriptor_pool_size.len() as u32,
            p_pool_sizes: v_descriptor_pool_size.as_ptr(),
        };
        let descriptor_pool = logical_device
            .create_descriptor_pool(&descriptor_pool_create_info, None)
            .expect("Cannot create descriptor pool");

        let v_descriptor_set_layout_refs = vec![descriptor_set_layout; swapchain_size];
        let descriptor_set_allocate_info = ash::vk::DescriptorSetAllocateInfo {
            s_type: ash::vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            descriptor_pool: descriptor_pool,
            descriptor_set_count: v_descriptor_set_layout_refs.len() as u32,
            p_set_layouts: v_descriptor_set_layout_refs.as_ptr(),
        };
        let v_descriptor_sets = logical_device
            .allocate_descriptor_sets(&descriptor_set_allocate_info)
            .expect("Cannot allocate descriptor set");

        let pipeline_layout_create_info = ash::vk::PipelineLayoutCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            set_layout_count: 1,
            p_set_layouts: &descriptor_set_layout,
            push_constant_range_count: 0,
            p_push_constant_ranges: std::ptr::null(),
        };

        let pipeline_layout = logical_device
            .create_pipeline_layout(&pipeline_layout_create_info, None)
            .expect("Cannot create pipeline layout");

        let attachment_description = ash::vk::AttachmentDescription {
            flags: Default::default(),
            format: available_format.format,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
            stencil_load_op: ash::vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: ash::vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: ash::vk::ImageLayout::UNDEFINED,
            final_layout: ash::vk::ImageLayout::PRESENT_SRC_KHR,
        };

        let color_attachment_reference = ash::vk::AttachmentReference {
            attachment: 0,
            layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let subpass_description = ash::vk::SubpassDescription {
            flags: Default::default(),
            pipeline_bind_point: ash::vk::PipelineBindPoint::GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: std::ptr::null(),
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_reference,
            p_resolve_attachments: std::ptr::null(),
            p_depth_stencil_attachment: std::ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: std::ptr::null(),
        };

        let render_pass_create_info = ash::vk::RenderPassCreateInfo {
            s_type: ash::vk::StructureType::RENDER_PASS_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            attachment_count: 1,
            p_attachments: &attachment_description,
            subpass_count: 1,
            p_subpasses: &subpass_description,
            dependency_count: 0,
            p_dependencies: std::ptr::null(),
        };
        let render_pass = logical_device
            .create_render_pass(&render_pass_create_info, None)
            .expect("Cannot create render pass");

        let graphics_pipeline_create_info = ash::vk::GraphicsPipelineCreateInfo {
            s_type: ash::vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: ash::vk::PipelineCreateFlags::DISABLE_OPTIMIZATION,
            stage_count: v_pipeline_shader_stage_create_infos.len() as u32,
            p_stages: v_pipeline_shader_stage_create_infos.as_ptr(),
            p_vertex_input_state: &vertex_input_state_create_info,
            p_input_assembly_state: &input_assembly_state_create_info,
            p_tessellation_state: std::ptr::null(),
            p_viewport_state: &viewport_state_create_info,
            p_rasterization_state: &rasterization_state_create_info,
            p_multisample_state: &multisample_state_create_info,
            p_depth_stencil_state: &depth_stencil_state_create_info,
            p_color_blend_state: &color_blend_state_create_info,
            p_dynamic_state: &dynamic_state_create_info,
            layout: pipeline_layout,
            render_pass: render_pass,
            subpass: 0,
            base_pipeline_handle: ash::vk::Pipeline::null(),
            base_pipeline_index: -1,
        };

        let v_graphics_pipelines = logical_device
            .create_graphics_pipelines(
                ash::vk::PipelineCache::null(),
                &[graphics_pipeline_create_info],
                None,
            )
            .expect("Cannot create graphics pipeline");

        let graphics_pipeline = v_graphics_pipelines[0];

        let mut v_image_views = Vec::with_capacity(v_swapchain_images.len());
        for image in &v_swapchain_images {
            let image_view_create_info = ash::vk::ImageViewCreateInfo {
                s_type: ash::vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: Default::default(),
                image: *image,
                view_type: ash::vk::ImageViewType::TYPE_2D,
                format: available_format.format,
                components: component_mapping,
                subresource_range: subresource_range,
            };
            v_image_views.push(
                logical_device
                    .create_image_view(&image_view_create_info, None)
                    .expect("Cannot create image view"),
            );
        }

        let mut v_framebuffers = Vec::with_capacity(swapchain_size);
        for i in 0..swapchain_size {
            let framebuffer_create_info = ash::vk::FramebufferCreateInfo {
                s_type: ash::vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: Default::default(),
                render_pass: render_pass,
                attachment_count: 1,
                p_attachments: &v_image_views[i],
                width: extent.width,
                height: extent.height,
                layers: 1,
            };
            v_framebuffers.push(
                logical_device
                    .create_framebuffer(&framebuffer_create_info, None)
                    .expect("Cannot create framebuffer"),
            );
        }

        let command_pool_create_info = ash::vk::CommandPoolCreateInfo {
            s_type: ash::vk::StructureType::COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index: index_of_queue_family as u32,
        };

        let command_pool = logical_device
            .create_command_pool(&command_pool_create_info, None)
            .expect("Cannot create command pool");

        let vertex_buffer_content = vec![
            MyPointData {
                position: glm::vec3(0.0, 0.0, 0.0),
                color: glm::vec3(1.0, 0.0, 0.0),
                uv: glm::vec2(0.0, 0.0),
            },
            MyPointData {
                position: glm::vec3(0.0, 1.0, 0.0),
                color: glm::vec3(0.0, 1.0, 0.0),
                uv: glm::vec2(0.0, 1.0),
            },
            MyPointData {
                position: glm::vec3(1.0, 0.0, 0.0),
                color: glm::vec3(0.0, 0.0, 1.0),
                uv: glm::vec2(1.0, 0.0),
            },
        ];

        let vertex_buffer_bytes_size =
            std::mem::size_of::<MyPointData>() * vertex_buffer_content.len();

        // VERTEX ATTRIBUTES: STAGING BUFFER CREATION
        let staging_buffer_create_info = ash::vk::BufferCreateInfo {
            s_type: ash::vk::StructureType::BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            size: vertex_buffer_bytes_size as u64,
            usage: ash::vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: ash::vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };

        let staging_buffer = logical_device
            .create_buffer(&staging_buffer_create_info, None)
            .expect("Cannot staging buffer");

        let staging_buffer_memory_requirements =
            logical_device.get_buffer_memory_requirements(staging_buffer);
        let staging_buffer_memory_type_index = search_physical_device_memory_type(
            &instance,
            &gpu,
            &staging_buffer_memory_requirements,
            ash::vk::MemoryPropertyFlags::HOST_COHERENT
                | ash::vk::MemoryPropertyFlags::HOST_VISIBLE,
        )
        .unwrap();

        let memory_allocate_info_for_staging_buffer = ash::vk::MemoryAllocateInfo {
            s_type: ash::vk::StructureType::MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: staging_buffer_memory_requirements.size,
            memory_type_index: staging_buffer_memory_type_index as u32,
        };
        let device_memory_for_staging_buffer = logical_device
            .allocate_memory(&memory_allocate_info_for_staging_buffer, None)
            .expect("Cannot allocate memory for staging buffer");

        let vertex_buffer_offset = 0 as ash::vk::DeviceSize;
        logical_device
            .bind_buffer_memory(
                staging_buffer,
                device_memory_for_staging_buffer,
                vertex_buffer_offset,
            )
            .expect("Cannot bind memory for staging buffer");

        let p_data = logical_device
            .map_memory(
                device_memory_for_staging_buffer,
                vertex_buffer_offset,
                staging_buffer_create_info.size,
                Default::default(),
            )
            .expect("Cannot map memory");
        std::ptr::copy_nonoverlapping(
            vertex_buffer_content.as_ptr() as *const std::ffi::c_void,
            p_data,
            vertex_buffer_bytes_size,
        );
        logical_device.unmap_memory(device_memory_for_staging_buffer);

        // VERTEX ATTRIBUTES: VERTEX BUFFER CREATION
        let vertex_buffer_create_info = ash::vk::BufferCreateInfo {
            s_type: ash::vk::StructureType::BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            size: vertex_buffer_bytes_size as u64,
            usage: ash::vk::BufferUsageFlags::VERTEX_BUFFER
                | ash::vk::BufferUsageFlags::TRANSFER_DST,
            sharing_mode: ash::vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };

        let vertex_buffer = logical_device
            .create_buffer(&vertex_buffer_create_info, None)
            .expect("Cannot create vertex buffer");

        let vertex_buffer_memory_requirements =
            logical_device.get_buffer_memory_requirements(vertex_buffer);

        let vertex_buffer_memory_type_index = search_physical_device_memory_type(
            &instance,
            &gpu,
            &vertex_buffer_memory_requirements,
            ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .unwrap();

        let memory_allocate_info_for_vertex_buffer = ash::vk::MemoryAllocateInfo {
            s_type: ash::vk::StructureType::MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: vertex_buffer_memory_requirements.size,
            memory_type_index: vertex_buffer_memory_type_index as u32,
        };

        let device_memory_for_vertex_buffer = logical_device
            .allocate_memory(&memory_allocate_info_for_vertex_buffer, None)
            .expect("Cannot allocate memory for vertex buffer");
        logical_device
            .bind_buffer_memory(
                vertex_buffer,
                device_memory_for_vertex_buffer,
                vertex_buffer_offset,
            )
            .expect("Cannot bind memory for vertex buffer");

        let copy_command_buffer_allocate_info = ash::vk::CommandBufferAllocateInfo {
            s_type: ash::vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool: command_pool,
            level: ash::vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
        };

        let command_buffer_copy_buffer = logical_device
            .allocate_command_buffers(&copy_command_buffer_allocate_info)
            .expect("Cannot allocate command buffer to copy staging buffer")[0];
        let command_buffer_begin_info = ash::vk::CommandBufferBeginInfo {
            s_type: ash::vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            p_inheritance_info: std::ptr::null(),
        };
        logical_device
            .begin_command_buffer(command_buffer_copy_buffer, &command_buffer_begin_info)
            .expect("Cannot begin command buffer to copy staging buffer");
        let buffer_copy = ash::vk::BufferCopy {
            src_offset: vertex_buffer_offset,
            dst_offset: vertex_buffer_offset,
            size: vertex_buffer_bytes_size as u64,
        };
        logical_device.cmd_copy_buffer(
            command_buffer_copy_buffer,
            staging_buffer,
            vertex_buffer,
            &[buffer_copy],
        );
        logical_device
            .end_command_buffer(command_buffer_copy_buffer)
            .expect("Cannot end command buffer to copy staging buffer");
        let copy_buffer_submit_info = ash::vk::SubmitInfo {
            s_type: ash::vk::StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &command_buffer_copy_buffer,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        logical_device
            .queue_submit(queue, &[copy_buffer_submit_info], ash::vk::Fence::null())
            .expect("Cannot submit command buffer to copy staging buffer");
        logical_device
            .queue_wait_idle(queue)
            .expect("Cannot wait for queue to copy staging buffer");

        logical_device.free_command_buffers(command_pool, &[command_buffer_copy_buffer]);
        logical_device.destroy_buffer(staging_buffer, None);
        logical_device.free_memory(device_memory_for_staging_buffer, None);

        // TEXTURE: staging buffer creation
        let jpg_file = std::fs::File::open("textures/texture.jpg")
            .expect("failed to open .jpg texture");
        let mut decoder = jpeg::Decoder::new(std::io::BufReader::new(jpg_file));
        let raw_texture_data = decoder.decode().expect("failed to decode jpg texture");
        let jpg_metadata = decoder.info().unwrap();
        let texture_pixel_width = jpg_metadata.width as usize;
        let texture_pixel_height = jpg_metadata.height as usize;
        let one_texture_pixel_bytes_size = 4;
        let texture_bytes_count =
            texture_pixel_width * one_texture_pixel_bytes_size * texture_pixel_height;
        let mut texture_data = vec![255 as u8; texture_bytes_count];

        for height_idx in 0..texture_pixel_height {
            for width_idx in 0..texture_pixel_width {
                let src_idx = height_idx * (texture_pixel_width * 3) + (width_idx * 3);
                let dst_idx = height_idx * (texture_pixel_width * 4) + (width_idx * 4);
                texture_data[dst_idx] = raw_texture_data[src_idx];
                texture_data[dst_idx + 1] = raw_texture_data[src_idx + 1];
                texture_data[dst_idx + 2] = raw_texture_data[src_idx + 2];
                texture_data[dst_idx + 3] = 255;
            }
        }

        let texture_staging_buffer_create_info = ash::vk::BufferCreateInfo {
            s_type: ash::vk::StructureType::BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            size: texture_bytes_count as ash::vk::DeviceSize,
            usage: ash::vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: ash::vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };

        let texture_staging_buffer = logical_device
            .create_buffer(&texture_staging_buffer_create_info, None)
            .expect("Cannot create staging buffer for texture");

        let texture_staging_buffer_requirements =
            logical_device.get_buffer_memory_requirements(texture_staging_buffer);

        let texture_memory_staging_buffer_allocate_info = ash::vk::MemoryAllocateInfo {
            s_type: ash::vk::StructureType::MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: texture_staging_buffer_requirements.size,
            memory_type_index: search_physical_device_memory_type(
                &instance,
                &gpu,
                &texture_staging_buffer_requirements,
                ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                    | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
            )
            .expect("Cannot find memory type for texture staging buffer")
                as u32,
        };

        let texture_memory_staging_buffer = logical_device
            .allocate_memory(&texture_memory_staging_buffer_allocate_info, None)
            .expect("Cannot allocate memory for texture staging buffer");

        logical_device
            .bind_buffer_memory(texture_staging_buffer, texture_memory_staging_buffer, 0)
            .expect("Cannot bind texture buffer to its memory");

        let p_texture_gpu_data = logical_device
            .map_memory(
                texture_memory_staging_buffer,
                0,
                texture_staging_buffer_create_info.size,
                Default::default(),
            )
            .expect("Cannot map memory for texture staging buffer");

        std::ptr::copy_nonoverlapping(
            texture_data.as_ptr() as *const std::ffi::c_void,
            p_texture_gpu_data,
            texture_staging_buffer_create_info.size as usize,
        );
        logical_device.unmap_memory(texture_memory_staging_buffer);

        // TEXTURE: image creation
        let texture_extent = ash::vk::Extent3D {
            width: texture_pixel_width as u32,
            height: texture_pixel_height as u32,
            depth: 1,
        };
        let texture_image_create_info = ash::vk::ImageCreateInfo {
            s_type: ash::vk::StructureType::IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            image_type: ash::vk::ImageType::TYPE_2D,
            format: ash::vk::Format::R8G8B8A8_UNORM,
            extent: texture_extent,
            mip_levels: 1,
            array_layers: 1,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            tiling: ash::vk::ImageTiling::OPTIMAL,
            usage: ash::vk::ImageUsageFlags::TRANSFER_DST | ash::vk::ImageUsageFlags::SAMPLED,
            sharing_mode: ash::vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: ash::vk::ImageLayout::UNDEFINED,
        };

        let texture_image = logical_device
            .create_image(&texture_image_create_info, None)
            .expect("Cannot create texture image");
        let texture_image_memory_requirements =
            logical_device.get_image_memory_requirements(texture_image);
        let texture_image_memory_allocate_info = ash::vk::MemoryAllocateInfo {
            s_type: ash::vk::StructureType::MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: texture_image_memory_requirements.size,
            memory_type_index: search_physical_device_memory_type(
                &instance,
                &gpu,
                &texture_image_memory_requirements,
                ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .expect("Cannot get memory type for texture image")
                as u32,
        };
        let texture_image_memory = logical_device
            .allocate_memory(&texture_image_memory_allocate_info, None)
            .expect("Cannot allocate texture image memory");

        logical_device
            .bind_image_memory(texture_image, texture_image_memory, 0)
            .expect("Cannot bind image texture to its memory");

        change_image_layout(
            &logical_device,
            &command_pool,
            &texture_image,
            ash::vk::AccessFlags::empty(),
            ash::vk::AccessFlags::TRANSFER_WRITE,
            ash::vk::PipelineStageFlags::TOP_OF_PIPE,
            ash::vk::PipelineStageFlags::TRANSFER,
            ash::vk::ImageLayout::UNDEFINED,
            ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &queue,
        );

        copy_buffer_to_image(
            &logical_device,
            &command_pool,
            &texture_staging_buffer,
            &texture_image,
            ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            texture_pixel_width as u32,
            texture_pixel_height as u32,
            &queue,
        );

        change_image_layout(
            &logical_device,
            &command_pool,
            &texture_image,
            ash::vk::AccessFlags::TRANSFER_WRITE,
            ash::vk::AccessFlags::SHADER_READ,
            ash::vk::PipelineStageFlags::TRANSFER,
            ash::vk::PipelineStageFlags::FRAGMENT_SHADER,
            ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            &queue,
        );

        let texture_view_components = ash::vk::ComponentMapping {
            r: ash::vk::ComponentSwizzle::IDENTITY,
            g: ash::vk::ComponentSwizzle::IDENTITY,
            b: ash::vk::ComponentSwizzle::IDENTITY,
            a: ash::vk::ComponentSwizzle::IDENTITY,
        };
        let texture_view_range = ash::vk::ImageSubresourceRange {
            aspect_mask: ash::vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };
        let texture_image_view_create_info = ash::vk::ImageViewCreateInfo {
            s_type: ash::vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            image: texture_image,
            view_type: ash::vk::ImageViewType::TYPE_2D,
            format: ash::vk::Format::R8G8B8A8_UNORM,
            components: texture_view_components,
            subresource_range: texture_view_range,
        };
        let texture_image_view = logical_device
            .create_image_view(&texture_image_view_create_info, None)
            .expect("Cannot create image texture view");

        let texture_sampler_create_info = ash::vk::SamplerCreateInfo {
            s_type: ash::vk::StructureType::SAMPLER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            mag_filter: ash::vk::Filter::LINEAR,
            min_filter: ash::vk::Filter::LINEAR,
            mipmap_mode: ash::vk::SamplerMipmapMode::LINEAR,
            address_mode_u: ash::vk::SamplerAddressMode::REPEAT,
            address_mode_v: ash::vk::SamplerAddressMode::REPEAT,
            address_mode_w: ash::vk::SamplerAddressMode::REPEAT,
            mip_lod_bias: 0.0,
            anisotropy_enable: ash::vk::TRUE,
            max_anisotropy: 16.0,
            compare_enable: ash::vk::FALSE,
            compare_op: ash::vk::CompareOp::ALWAYS,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: ash::vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: 0,
        };
        let texture_image_sampler = logical_device
            .create_sampler(&texture_sampler_create_info, None)
            .expect("Cannot create image texture sampler");

        // UNIFORM BUFFERS
        let mut matrices = MyUniformBuffer {
            m_model: glm::identity(),
            m_view: glm::look_at(
                &glm::vec3(0.0, 0.0, 4.0),
                &glm::vec3(0.0, 0.0, 0.0),
                &glm::vec3(0.0, 1.0, 0.0),
            ),
            m_projection: glm::perspective(16.0f32 / 9.0f32, 45.0f32, 1.0f32, 100.0f32),
        };
        let uniform_buffer_size = std::mem::size_of::<MyUniformBuffer>();
        let mut v_uniform_buffers = Vec::with_capacity(swapchain_size);
        let mut v_memory_uniform_buffers = Vec::with_capacity(swapchain_size);
        for i in 0..swapchain_size {
            let buffer_create_info = ash::vk::BufferCreateInfo {
                s_type: ash::vk::StructureType::BUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: Default::default(),
                size: uniform_buffer_size as ash::vk::DeviceSize,
                usage: ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
                sharing_mode: ash::vk::SharingMode::EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
            };
            v_uniform_buffers.push(
                logical_device
                    .create_buffer(&buffer_create_info, None)
                    .expect("Cannot create uniform buffer"),
            );
            let buffer_requirements =
                logical_device.get_buffer_memory_requirements(v_uniform_buffers[i]);
            let memory_allocate_info = ash::vk::MemoryAllocateInfo {
                s_type: ash::vk::StructureType::MEMORY_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                allocation_size: buffer_requirements.size,
                memory_type_index: search_physical_device_memory_type(
                    &instance,
                    &gpu,
                    &buffer_requirements,
                    ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                        | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
                )
                .expect("Cannot find memory type for uniform buffer memory")
                    as u32,
            };
            v_memory_uniform_buffers.push(
                logical_device
                    .allocate_memory(&memory_allocate_info, None)
                    .expect("Cannot allocate memory for uniform buffer"),
            );
            logical_device
                .bind_buffer_memory(v_uniform_buffers[i], v_memory_uniform_buffers[i], 0)
                .expect("Cannot bind uniform buffer to its memory");

            update_uniform_buffer(&logical_device, &v_memory_uniform_buffers[i], &mut matrices);

            let descriptor_buffer_info = ash::vk::DescriptorBufferInfo {
                buffer: v_uniform_buffers[i],
                offset: 0,
                range: ash::vk::WHOLE_SIZE,
            };
            let descriptor_image_info = ash::vk::DescriptorImageInfo {
                sampler: texture_image_sampler,
                image_view: texture_image_view,
                image_layout: ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            };

            let v_descriptor_writes = &[
                ash::vk::WriteDescriptorSet {
                    s_type: ash::vk::StructureType::WRITE_DESCRIPTOR_SET,
                    p_next: std::ptr::null(),
                    dst_set: v_descriptor_sets[i],
                    dst_binding: uniform_buffer_binding_number,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: ash::vk::DescriptorType::UNIFORM_BUFFER,
                    p_image_info: std::ptr::null(),
                    p_buffer_info: &descriptor_buffer_info,
                    p_texel_buffer_view: std::ptr::null(),
                },
                ash::vk::WriteDescriptorSet {
                    s_type: ash::vk::StructureType::WRITE_DESCRIPTOR_SET,
                    p_next: std::ptr::null(),
                    dst_set: v_descriptor_sets[i],
                    dst_binding: texture_image_binding_number,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &descriptor_image_info,
                    p_buffer_info: std::ptr::null(),
                    p_texel_buffer_view: std::ptr::null(),
                },
            ];
            logical_device.update_descriptor_sets(v_descriptor_writes, &[]);
        }

        let command_buffer_allocate_info = ash::vk::CommandBufferAllocateInfo {
            s_type: ash::vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool: command_pool,
            level: ash::vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: swapchain_size as u32,
        };

        let v_command_buffers = logical_device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .expect("Cannot allocate command buffer");

        for (index, command_buffer) in (&v_command_buffers).iter().enumerate() {
            let render_area = ash::vk::Rect2D {
                offset: ash::vk::Offset2D { x: 0, y: 0 },
                extent: extent,
            };
            let clear_values = ash::vk::ClearValue {
                color: ash::vk::ClearColorValue {
                    float32: [1.0, 0.0, 1.0, 1.0],
                },
            };
            let command_buffer_begin_info = ash::vk::CommandBufferBeginInfo {
                s_type: ash::vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
                p_next: std::ptr::null(),
                flags: Default::default(),
                p_inheritance_info: std::ptr::null(),
            };

            logical_device
                .begin_command_buffer(*command_buffer, &command_buffer_begin_info)
                .expect("Cannot begin command buffer");

            let render_pass_begin_info = ash::vk::RenderPassBeginInfo {
                s_type: ash::vk::StructureType::RENDER_PASS_BEGIN_INFO,
                p_next: std::ptr::null(),
                render_pass: render_pass,
                framebuffer: v_framebuffers[index],
                render_area: render_area,
                clear_value_count: 1,
                p_clear_values: &clear_values,
            };

            logical_device.cmd_begin_render_pass(
                *command_buffer,
                &render_pass_begin_info,
                ash::vk::SubpassContents::INLINE,
            );

            logical_device.cmd_bind_pipeline(
                *command_buffer,
                ash::vk::PipelineBindPoint::GRAPHICS,
                graphics_pipeline,
            );

            logical_device.cmd_bind_descriptor_sets(
                *command_buffer,
                ash::vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &[v_descriptor_sets[index]],
                &[],
            );

            logical_device.cmd_bind_vertex_buffers(
                *command_buffer,
                0,
                &[vertex_buffer],
                &[vertex_buffer_offset],
            );
            logical_device.cmd_draw(*command_buffer, vertex_buffer_content.len() as u32, 1, 0, 0);
            logical_device.cmd_end_render_pass(*command_buffer);
            logical_device
                .end_command_buffer(*command_buffer)
                .expect("Cannot end command buffer");
        }

        let fence_create_info = ash::vk::FenceCreateInfo {
            s_type: ash::vk::StructureType::FENCE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: ash::vk::FenceCreateFlags::SIGNALED,
        };

        let semaphore_acquired_image_create_info = ash::vk::SemaphoreCreateInfo {
            s_type: ash::vk::StructureType::SEMAPHORE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
        };

        let semaphore_pipeline_done_create_info = ash::vk::SemaphoreCreateInfo {
            s_type: ash::vk::StructureType::SEMAPHORE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
        };

        let v_fences_wait_gpu = [
            logical_device
                .create_fence(&fence_create_info, None)
                .expect("Cannot create fence"),
            logical_device
                .create_fence(&fence_create_info, None)
                .expect("Cannot create fence"),
        ];
        let mut v_fences_ref_wait_gpu = vec![ash::vk::Fence::null(); swapchain_size];
        let mut v_semaphores_acquired_image = Vec::with_capacity(FRAME_COUNT);
        let mut v_semaphores_pipeline_done = Vec::with_capacity(FRAME_COUNT);

        for _ in 0..FRAME_COUNT {
            v_semaphores_acquired_image.push(
                logical_device
                    .create_semaphore(&semaphore_acquired_image_create_info, None)
                    .expect("Cannot create sempahore"),
            );
            v_semaphores_pipeline_done.push(
                logical_device
                    .create_semaphore(&semaphore_pipeline_done_create_info, None)
                    .expect("Cannot create sempahore"),
            );
        }

        let mut event_pump = sdl_context.event_pump().expect("Cannot get sdl event pump");
        let mut go = true;
        let mut current_frame = 0;

        while go {
            go = handle_events(&mut event_pump);

            logical_device
                .wait_for_fences(&[v_fences_wait_gpu[current_frame]], true, !(0 as u64))
                .expect("Cannot wait for fences");

            let infos_of_acquired_image = swapchain_loader
                .acquire_next_image(
                    swapchain,
                    !(0 as u64),
                    v_semaphores_acquired_image[current_frame],
                    ash::vk::Fence::null(),
                )
                .expect("Cannot acquire next image");

            let index_of_acquired_image = infos_of_acquired_image.0 as usize;

            if v_fences_ref_wait_gpu[index_of_acquired_image] != ash::vk::Fence::null() {
                logical_device
                    .wait_for_fences(
                        &[v_fences_ref_wait_gpu[index_of_acquired_image]],
                        true,
                        !(0 as u64),
                    )
                    .expect("Cannot wait for fences");
            }

            v_fences_ref_wait_gpu[index_of_acquired_image] = v_fences_wait_gpu[current_frame];

            logical_device
                .reset_fences(&[v_fences_ref_wait_gpu[current_frame]])
                .expect("Cannot reset fences");

            let wait_stage_submit_info = ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
            let submit_info = ash::vk::SubmitInfo {
                s_type: ash::vk::StructureType::SUBMIT_INFO,
                p_next: std::ptr::null(),
                wait_semaphore_count: 1,
                p_wait_semaphores: &v_semaphores_acquired_image[current_frame],
                p_wait_dst_stage_mask: &wait_stage_submit_info
                    as *const ash::vk::PipelineStageFlags,
                command_buffer_count: 1,
                p_command_buffers: &v_command_buffers[index_of_acquired_image],
                signal_semaphore_count: 1,
                p_signal_semaphores: &v_semaphores_pipeline_done[current_frame],
            };
            logical_device
                .queue_submit(queue, &[submit_info], v_fences_ref_wait_gpu[current_frame])
                .expect("Cannot submit queue");

            let present_info = ash::vk::PresentInfoKHR {
                s_type: ash::vk::StructureType::PRESENT_INFO_KHR,
                p_next: std::ptr::null(),
                wait_semaphore_count: 1,
                p_wait_semaphores: &v_semaphores_pipeline_done[current_frame],
                swapchain_count: 1,
                p_swapchains: &swapchain,
                p_image_indices: &infos_of_acquired_image.0,
                p_results: std::ptr::null_mut(),
            };
            swapchain_loader
                .queue_present(queue, &present_info)
                .expect("Cannot present image");

            current_frame = (current_frame + 1) % FRAME_COUNT;
        }
    }
}
