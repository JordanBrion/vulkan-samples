extern crate ash;
extern crate core;
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
    let device_create_info = ash::vk::DeviceCreateInfo {
        s_type: ash::vk::StructureType::DEVICE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        queue_create_info_count: 1,
        p_queue_create_infos: &queue_create_info,
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 0,
        pp_enabled_extension_names: std::ptr::null(),
        p_enabled_features: std::ptr::null(),
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

fn choose_swapchain_present_mode(v_present_mode: &Vec<ash::vk::PresentModeKHR>) -> ash::vk::PresentModeKHR
{
    return match v_present_mode.iter().find(|mode| mode == ash::vk::PresentModeKHR::FIFO) {
        Some(mode) => *mode,
        None => ash::vk::PresentModeKHR::MAILBOX
    };
}

struct Fibonacci {
    curr: u32,
    next: u32,
}

// Implement `Iterator` for `Fibonacci`.
// The `Iterator` trait only requires a method to be defined for the `next` element.
impl Iterator for Fibonacci {
    type Item = u32;
    
    // Here, we define the sequence using `.curr` and `.next`.
    // The return type is `Option<T>`:
    //     * When the `Iterator` is finished, `None` is returned.
    //     * Otherwise, the next value is wrapped in `Some` and returned.
    fn next(&mut self) -> Option<u32> {
        let new_next = self.curr + self.next;

        self.curr = self.next;
        self.next = new_next;

        // Since there's no endpoint to a Fibonacci sequence, the `Iterator` 
        // will never return `None`, and `Some` is always returned.
        Some(self.curr)
    }
}

fn main() {

    let fibo = Fibonacci { curr: 9, next: 12 };
    fibo.next()

    unsafe {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let window = video_subsystem
            .window("rust-sdl2 demo", 800, 600)
            .vulkan()
            .position_centered()
            .build()
            .unwrap();

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
        let v_surface_present_modes =
            surface_loader.get_physical_device_surface_present_modes(gpu, surface).expect("Cannot get surface present mode");
        let available_format =
            search_format(&v_surface_formats).expect("Cannot find surface format");
        let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &logical_device);
        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR {
            s_type: ash::vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            p_next: std::ptr::null(),
            flags: ash::vk::SwapchainCreateFlagsKHR::SPLIT_INSTANCE_BIND_REGIONS,
            surface: surface,
            min_image_count: surface_capabilities.min_image_count + 1,
            image_format: available_format.format,
            image_color_space: available_format.color_space,
            image_extent: surface_capabilities.current_extent,
            image_array_layers: surface_capabilities.max_image_array_layers,
            image_usage: ash::vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: ash::vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            pre_transform: ash::vk::SurfaceTransformFlagsKHR::IDENTITY,
            composite_alpha: ash::vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: choose_swapchain_present_mode(&v_surface_present_modes),
            clipped: ash::vk::TRUE,
            old_swapchain: ash::vk::SwapchainKHR::null(),
        };
        let swapchain = swapchain_loader.create_swapchain(&swapchain_create_info, None);

        // let s = ash::extensions::khr::Surface::new(entry, instance);

        // let shader_entry_name =
        //     CString::new("main").expect("Cannot create vertex shader entry name");
        // let v_pipeline_shader_stage_create_infos = [
        //             ash::vk::PipelineShaderStageCreateInfo {
        //                 s_type: ash::vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
        //                 p_next: std::ptr::null(),
        //                 flags: Default::default(),
        //                 stage: ash::vk::ShaderStageFlags::VERTEX,
        //                 // TODO path
        //                 module: create_shader_module(&logical_device, "/home/jordanbrion/Documents/rust/vk_001_compute_pipeline/shaders/002_compute_pipeline_2_buffers.comp.spv"),
        //                 p_name: shader_entry_name.as_ptr(),
        //                 p_specialization_info: std::ptr::null(),
        //             },
        //             ash::vk::PipelineShaderStageCreateInfo {
        //                 s_type: ash::vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
        //                 p_next: std::ptr::null(),
        //                 flags: Default::default(),
        //                 stage: ash::vk::ShaderStageFlags::FRAGMENT,
        //                 // TODO path
        //                 module: create_shader_module(&logical_device, "/home/jordanbrion/Documents/rust/vk_001_compute_pipeline/shaders/002_compute_pipeline_2_buffers.comp.spv"),
        //                 p_name: shader_entry_name.as_ptr(),
        //                 p_specialization_info: std::ptr::null(),
        //             },
        //         ];

        // let window_width = 1280;
        // let window_height = 720;

        // let viewport = ash::vk::Viewport {
        //     x: 0f32,
        //     y: 0f32,
        //     width: window_width as f32,
        //     height: window_height as f32,
        //     min_depth: 0.0,
        //     max_depth: 1.0,
        // };
        // let viewport_state_create_info = ash::vk::PipelineViewportStateCreateInfo {
        //     s_type: ash::vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
        //     p_next: std::ptr::null(),
        //     flags: Default::default(),
        //     viewport_count: 1,
        //     p_viewports: &viewport,
        //     scissor_count: 0,
        //     p_scissors: std::ptr::null(),
        // };

        // let rasterization_state_create_info = ash::vk::PipelineRasterizationStateCreateInfo {
        //     s_type: ash::vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
        //     p_next: std::ptr::null(),
        //     flags: Default::default(),
        //     depth_clamp_enable: ash::vk::TRUE,
        //     rasterizer_discard_enable: ash::vk::TRUE,
        //     polygon_mode: ash::vk::PolygonMode::FILL,
        //     cull_mode: ash::vk::CullModeFlags::BACK,
        //     front_face: ash::vk::FrontFace::CLOCKWISE,
        //     depth_bias_enable: ash::vk::FALSE,
        //     depth_bias_constant_factor: 0f32,
        //     depth_bias_clamp: 0f32,
        //     depth_bias_slope_factor: 0f32,
        //     line_width: 1f32,
        // };

        // let multisample_state_create_info = ash::vk::PipelineMultisampleStateCreateInfo {
        //     s_type: ash::vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
        //     p_next: std::ptr::null(),
        //     flags: Default::default(),
        //     rasterization_samples: ash::vk::SampleCountFlags::TYPE_1,
        //     sample_shading_enable: ash::vk::FALSE,
        //     min_sample_shading: 0f32,
        //     p_sample_mask: std::ptr::null(),
        //     alpha_to_coverage_enable: ash::vk::FALSE,
        //     alpha_to_one_enable: ash::vk::FALSE,
        // };

        // let depth_stencil_state_create_info = ash::vk::PipelineDepthStencilStateCreateInfo {
        //     s_type: ash::vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
        //     p_next: std::ptr::null(),
        //     flags: Default::default(),
        //     depth_test_enable: ash::vk::TRUE,
        //     depth_write_enable: ash::vk::TRUE,
        //     depth_compare_op: ash::vk::CompareOp::LESS,
        //     depth_bounds_test_enable: ash::vk::FALSE,
        //     stencil_test_enable: ash::vk::FALSE,
        //     front: Default::default(),
        //     back: Default::default(),
        //     min_depth_bounds: 0f32,
        //     max_depth_bounds: 1f32,
        // };

        // let color_blend_state_create_info = ash::vk::PipelineColorBlendStateCreateInfo {
        //     s_type: ash::vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
        //     p_next: std::ptr::null(),
        //     flags: Default::default(),
        //     logic_op_enable: ash::vk::FALSE,
        //     logic_op: ash::vk::LogicOp::NO_OP,
        //     attachment_count: 0 as u32,
        //     p_attachments: std::ptr::null(),
        //     blend_constants: [0f32; 4],
        // };

        // let dynamic_state_create_info = ash::vk::PipelineDynamicStateCreateInfo {
        //     s_type: ash::vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
        //     p_next: std::ptr::null(),
        //     flags: Default::default(),
        //     dynamic_state_count: 0 as u32,
        //     p_dynamic_states: std::ptr::null(),
        // };

        // let pipeline_layout_create_info = ash::vk::PipelineLayoutCreateInfo {
        //     s_type: ash::vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
        //     p_next: std::ptr::null(),
        //     flags: Default::default(),
        //     set_layout_count: 0,
        //     p_set_layouts: std::ptr::null(),
        //     push_constant_range_count: 0,
        //     p_push_constant_ranges: std::ptr::null(),
        // };

        // let pipeline_layout = logical_device
        //     .create_pipeline_layout(&pipeline_layout_create_info, None)
        //     .expect("Cannot create pipeline layout");

        // let attachment_description = ash::vk::AttachmentDescription {
        //     flags: ash::vk::AttachmentDescriptionFlags::MAY_ALIAS,
        //     format: ash::vk::Format::R8G8B8_UINT,
        //     samples: ash::vk::SampleCountFlags::TYPE_1,
        //     load_op: ash::vk::AttachmentLoadOp::CLEAR,
        //     store_op: ash::vk::AttachmentStoreOp::STORE,
        //     stencil_load_op: ash::vk::AttachmentLoadOp::DONT_CARE,
        //     stencil_store_op: ash::vk::AttachmentStoreOp::DONT_CARE,
        //     initial_layout: ash::vk::ImageLayout::UNDEFINED,
        //     final_layout: ash::vk::ImageLayout::PRESENT_SRC_KHR,
        // };

        // let color_attachment_reference = ash::vk::AttachmentReference {
        //     attachment: 0,
        //     layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        // };

        // let depth_attachment_reference = ash::vk::AttachmentReference {
        //     attachment: 0,
        //     layout: ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        // };

        // let subpass_description = ash::vk::SubpassDescription {
        //     flags: Default::default(),
        //     pipeline_bind_point: ash::vk::PipelineBindPoint::GRAPHICS,
        //     input_attachment_count: 0,
        //     p_input_attachments: std::ptr::null(),
        //     color_attachment_count: 1,
        //     p_color_attachments: &color_attachment_reference,
        //     p_resolve_attachments: std::ptr::null(),
        //     p_depth_stencil_attachment: &depth_attachment_reference,
        //     preserve_attachment_count: 0,
        //     p_preserve_attachments: std::ptr::null(),
        // };

        // let render_pass_create_info = ash::vk::RenderPassCreateInfo {
        //     s_type: ash::vk::StructureType::RENDER_PASS_CREATE_INFO,
        //     p_next: std::ptr::null(),
        //     flags: Default::default(),
        //     attachment_count: 1,
        //     p_attachments: &attachment_description,
        //     subpass_count: 1,
        //     p_subpasses: &subpass_description,
        //     dependency_count: 0,
        //     p_dependencies: std::ptr::null(),
        // };
        // let render_pass = logical_device
        //     .create_render_pass(&render_pass_create_info, None)
        //     .expect("Cannot create render pass");

        // let graphics_pipeline_create_info = ash::vk::GraphicsPipelineCreateInfo {
        //     s_type: ash::vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
        //     p_next: std::ptr::null(),
        //     flags: ash::vk::PipelineCreateFlags::DISABLE_OPTIMIZATION,
        //     stage_count: v_pipeline_shader_stage_create_infos.len() as u32,
        //     p_stages: v_pipeline_shader_stage_create_infos.as_ptr(),
        //     p_vertex_input_state: std::ptr::null(),
        //     p_input_assembly_state: std::ptr::null(),
        //     p_tessellation_state: std::ptr::null(),
        //     p_viewport_state: &viewport_state_create_info,
        //     p_rasterization_state: &rasterization_state_create_info,
        //     p_multisample_state: &multisample_state_create_info,
        //     p_depth_stencil_state: &depth_stencil_state_create_info,
        //     p_color_blend_state: &color_blend_state_create_info,
        //     p_dynamic_state: &dynamic_state_create_info,
        //     layout: pipeline_layout,
        //     render_pass: render_pass,
        //     subpass: 0,
        //     base_pipeline_handle: ash::vk::Pipeline::null(),
        //     base_pipeline_index: -1,
        // };

        // let v_graphics_pipelines = logical_device
        //     .create_graphics_pipelines(
        //         ash::vk::PipelineCache::null(),
        //         &[graphics_pipeline_create_info],
        //         None,
        //     )
        //     .expect("Cannot create graphics pipeline");

        // let graphics_pipeline = v_graphics_pipelines[0];

        // let command_pool_create_info = ash::vk::CommandPoolCreateInfo {
        //     s_type: ash::vk::StructureType::COMMAND_POOL_CREATE_INFO,
        //     p_next: std::ptr::null(),
        //     flags: ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        //     queue_family_index: index_of_queue_family as u32,
        // };

        // let command_pool = logical_device
        //     .create_command_pool(&command_pool_create_info, None)
        //     .expect("Cannot create command pool");
        // let command_buffer_allocate_info = ash::vk::CommandBufferAllocateInfo {
        //     s_type: ash::vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
        //     p_next: std::ptr::null(),
        //     command_pool: command_pool,
        //     level: ash::vk::CommandBufferLevel::PRIMARY,
        //     command_buffer_count: 1,
        // };

        // let v_command_buffers = logical_device
        //     .allocate_command_buffers(&command_buffer_allocate_info)
        //     .expect("Cannot allocate command buffer");

        // let command_buffer = v_command_buffers[0];

        // let command_buffer_begin_info = ash::vk::CommandBufferBeginInfo {
        //     s_type: ash::vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
        //     p_next: std::ptr::null(),
        //     flags: ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        //     p_inheritance_info: std::ptr::null(),
        // };
        // logical_device
        //     .begin_command_buffer(command_buffer, &command_buffer_begin_info)
        //     .expect("Cannot begin command buffer");
        // logical_device.cmd_bind_pipeline(
        //     command_buffer,
        //     ash::vk::PipelineBindPoint::GRAPHICS,
        //     graphics_pipeline,
        // );
        // logical_device
        //     .end_command_buffer(command_buffer)
        //     .expect("Cannot end command buffer");

        // let submit_info = ash::vk::SubmitInfo {
        //     s_type: ash::vk::StructureType::SUBMIT_INFO,
        //     p_next: std::ptr::null(),
        //     wait_semaphore_count: 0,
        //     p_wait_semaphores: std::ptr::null(),
        //     p_wait_dst_stage_mask: std::ptr::null(),
        //     command_buffer_count: 1,
        //     p_command_buffers: &command_buffer,
        //     signal_semaphore_count: 0,
        //     p_signal_semaphores: std::ptr::null(),
        // };
        // logical_device
        //     .queue_submit(queue, &[submit_info], ash::vk::Fence::null())
        //     .expect("Cannot submit queue");

        // uncomment those lines
        // let mut canvas = window.into_canvas().build().unwrap();
        // canvas.set_draw_color(Color::RGB(0, 255, 255));
        // canvas.clear();
        // canvas.present();
        // let mut event_pump = sdl_context.event_pump().unwrap();
        // let mut i = 0;
        // 'running: loop {
        //     i = (i + 1) % 255;
        //     canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        //     canvas.clear();
        //     for event in event_pump.poll_iter() {
        //         match event {
        //             Event::Quit { .. }
        //             | Event::KeyDown {
        //                 keycode: Some(Keycode::Escape),
        //                 ..
        //             } => break 'running,
        //             _ => {}
        //         }
        //     }
        //     // The rest of the game loop goes here...

        //     canvas.present();
        //     ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        // }
    }
}
