extern crate ash;
extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::time::Duration;

use std::ffi::CString;

use ash::version::DeviceV1_0;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;

unsafe fn create_instance(entry: &ash::Entry) -> ash::Instance {
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
    let instance_create_info = ash::vk::InstanceCreateInfo {
        s_type: ash::vk::StructureType::INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: Default::default(),
        p_application_info: &application_info,
        enabled_layer_count: v_layers.len() as u32,
        pp_enabled_layer_names: v_layers.as_ptr() as *const *const i8,
        enabled_extension_count: 0,
        pp_enabled_extension_names: std::ptr::null(),
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

fn main() {
    unsafe {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let window = video_subsystem
            .window("rust-sdl2 demo", 800, 600)
            .position_centered()
            .build()
            .unwrap();

        let entry = ash::Entry::new().expect("Cannot create entry");
        let instance = create_instance(&entry);
        let gpu = pick_up_one_gpu(&instance).expect("Cannot find GPU");
        let index_of_queue_family =
            lookup_queue_family_index(&instance, &gpu).expect("Cannot find graphics queue family");
        let logical_device = create_logical_device(&instance, &gpu, index_of_queue_family)
            .expect("Cannot create logical device");

        let shader_entry_name =
            CString::new("main").expect("Cannot create vertex shader entry name");
        let v_pipeline_shader_stage_create_infos = [
                    ash::vk::PipelineShaderStageCreateInfo {
                        s_type: ash::vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                        p_next: std::ptr::null(),
                        flags: Default::default(),
                        stage: ash::vk::ShaderStageFlags::VERTEX,
                        // TODO path
                        module: create_shader_module(&logical_device, "/home/jordanbrion/Documents/rust/vk_001_compute_pipeline/shaders/002_compute_pipeline_2_buffers.comp.spv"),
                        p_name: shader_entry_name.as_ptr(),
                        p_specialization_info: std::ptr::null(),
                    },
                    ash::vk::PipelineShaderStageCreateInfo {
                        s_type: ash::vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                        p_next: std::ptr::null(),
                        flags: Default::default(),
                        stage: ash::vk::ShaderStageFlags::FRAGMENT,
                        // TODO path
                        module: create_shader_module(&logical_device, "/home/jordanbrion/Documents/rust/vk_001_compute_pipeline/shaders/002_compute_pipeline_2_buffers.comp.spv"),
                        p_name: shader_entry_name.as_ptr(),
                        p_specialization_info: std::ptr::null(),
                    },
                ];

        let window_width = 1280;
        let window_height = 720;

        let viewport = ash::vk::Viewport {
            x: 0f32,
            y: 0f32,
            width: window_width as f32,
            height: window_height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let viewport_state_create_info = ash::vk::PipelineViewportStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            viewport_count: 1,
            p_viewports: &viewport,
            scissor_count: 0,
            p_scissors: std::ptr::null(),
        };

        let rasterization_state_create_info = ash::vk::PipelineRasterizationStateCreateInfo {
            s_type: ash::vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            depth_clamp_enable: ash::vk::TRUE,
            rasterizer_discard_enable: ash::vk::TRUE,
            polygon_mode: ash::vk::PolygonMode::FILL,
            cull_mode: ash::vk::CullModeFlags::BACK,
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
stencil_test_enable: ash::vk::
front: StencilOpState
back: StencilOpState
min_depth_bounds: f32
max_depth_bounds: f32
        };

        //                 let graphics_pipeline_create_info = ash::vk::GraphicsPipelineCreateInfo {
        //         s_type: ash::vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
        //         p_next: std::ptr::null(),
        //         flags: ash::vk::PipelineCreateFlags::DISABLE_OPTIMIZATION,
        //         stage_count: v_pipeline_shader_stage_create_infos.len(),
        //         p_stages: v_pipeline_shader_stage_create_infos.as_ptr(),
        //         p_vertex_input_state: std::ptr::null(),
        //         p_input_assembly_state: std::ptr::null(),
        //         p_tessellation_state: std::ptr::null(),
        //         p_viewport_state: &viewport_state_create_info,
        //         p_rasterization_state: &rasterization_state_create_info,
        //         p_multisample_state: &multisample_state_create_info,
        // p_depth_stencil_state: *const PipelineDepthStencilStateCreateInfo
        // p_color_blend_state: *const PipelineColorBlendStateCreateInfo
        // p_dynamic_state: *const PipelineDynamicStateCreateInfo
        // layout: PipelineLayout
        // render_pass: RenderPass
        // subpass: u32
        // base_pipeline_handle: Pipeline
        // base_pipeline_index: i32
        //         };
        // logical_device.create_graphics_pipelines(ash::vk::PipelineCache::null(),
        //                                         &[graphics_pipeline_create_info],
        //                                         None);

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
