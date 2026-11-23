use ut99_editor::renderer::shaders::{LINE_SHADER, SOLID_SHADER};
use ut99_editor::renderer::vertex::{LineVertex, SolidVertex, Uniforms};

#[test]
fn test_shader_wgsl_integrity_and_uniform_structures() {
    // 1. Verify LINE_SHADER
    assert!(LINE_SHADER.contains("struct Uniforms"), "LINE_SHADER must declare struct Uniforms");
    assert!(LINE_SHADER.contains("view_proj: mat4x4<f32>"), "LINE_SHADER uniforms must include view_proj");
    assert!(LINE_SHADER.contains("light_dir: vec4<f32>"), "LINE_SHADER uniforms must include light_dir");
    assert!(LINE_SHADER.contains("fog_params: vec4<f32>"), "LINE_SHADER uniforms must include fog_params");
    assert!(LINE_SHADER.contains("fog_color: vec4<f32>"), "LINE_SHADER uniforms must include fog_color");
    assert!(LINE_SHADER.contains("cam_pos: vec4<f32>"), "LINE_SHADER uniforms must include cam_pos");
    assert!(LINE_SHADER.contains("@group(0) @binding(0)"), "LINE_SHADER must bind uniforms at group 0, binding 0");
    assert!(LINE_SHADER.contains("@vertex"), "LINE_SHADER must have a vertex entrypoint");
    assert!(LINE_SHADER.contains("fn vs_main"), "LINE_SHADER vertex entrypoint must be vs_main");
    assert!(LINE_SHADER.contains("@fragment"), "LINE_SHADER must have a fragment entrypoint");
    assert!(LINE_SHADER.contains("fn fs_main"), "LINE_SHADER fragment entrypoint must be fs_main");

    // 2. Verify SOLID_SHADER
    assert!(SOLID_SHADER.contains("struct Uniforms"), "SOLID_SHADER must declare struct Uniforms");
    assert!(SOLID_SHADER.contains("view_proj: mat4x4<f32>"), "SOLID_SHADER uniforms must include view_proj");
    assert!(SOLID_SHADER.contains("light_dir: vec4<f32>"), "SOLID_SHADER uniforms must include light_dir");
    assert!(SOLID_SHADER.contains("fog_params: vec4<f32>"), "SOLID_SHADER uniforms must include fog_params");
    assert!(SOLID_SHADER.contains("fog_color: vec4<f32>"), "SOLID_SHADER uniforms must include fog_color");
    assert!(SOLID_SHADER.contains("cam_pos: vec4<f32>"), "SOLID_SHADER uniforms must include cam_pos");
    assert!(SOLID_SHADER.contains("@group(0) @binding(0)"), "SOLID_SHADER must bind uniforms at group 0, binding 0");
    assert!(SOLID_SHADER.contains("@vertex"), "SOLID_SHADER must have a vertex entrypoint");
    assert!(SOLID_SHADER.contains("fn vs_main"), "SOLID_SHADER vertex entrypoint must be vs_main");
    assert!(SOLID_SHADER.contains("@fragment"), "SOLID_SHADER must have a fragment entrypoint");
    assert!(SOLID_SHADER.contains("fn fs_main"), "SOLID_SHADER fragment entrypoint must be fs_main");

    // Lighting, Texture Array, and Distance Fog assertions in SOLID_SHADER
    assert!(SOLID_SHADER.contains("t_diffuse: texture_2d_array<f32>"), "SOLID_SHADER must bind 2D texture array");
    assert!(SOLID_SHADER.contains("uniforms.light_dir.xyz"), "SOLID_SHADER must compute directional lighting");
    assert!(SOLID_SHADER.contains("mix(final_color.rgb, uniforms.fog_color.rgb, fog_factor)"), "SOLID_SHADER must simulate distance fog");
}

#[test]
fn test_vertex_layouts_memory_invariance() {
    // LineVertex: 3 floats position (12 bytes) + Unorm8x4 color (4 bytes) = 16 bytes
    assert_eq!(std::mem::size_of::<LineVertex>(), 16);
    let line_desc = LineVertex::desc();
    assert_eq!(line_desc.array_stride, 16);
    assert_eq!(line_desc.attributes.len(), 2);
    assert_eq!(line_desc.attributes[0].shader_location, 0);
    assert_eq!(line_desc.attributes[0].offset, 0);
    assert_eq!(line_desc.attributes[1].shader_location, 1);
    assert_eq!(line_desc.attributes[1].offset, 12);

    // SolidVertex: position (12) + normal Snorm8x4 (4) + color Float16x4 (8) + uv (8) + lm_uv Unorm16x2 (4) + packed_meta (4) = 40 bytes
    assert_eq!(std::mem::size_of::<SolidVertex>(), 40);
    let solid_desc = SolidVertex::desc();
    assert_eq!(solid_desc.array_stride, 40);
    assert_eq!(solid_desc.attributes.len(), 6);
    assert_eq!(solid_desc.attributes[0].shader_location, 0);
    assert_eq!(solid_desc.attributes[0].offset, 0);
    assert_eq!(solid_desc.attributes[1].shader_location, 1);
    assert_eq!(solid_desc.attributes[1].offset, 12);
    assert_eq!(solid_desc.attributes[2].shader_location, 2);
    assert_eq!(solid_desc.attributes[2].offset, 16);
    assert_eq!(solid_desc.attributes[3].shader_location, 3);
    assert_eq!(solid_desc.attributes[3].offset, 24);
    assert_eq!(solid_desc.attributes[4].shader_location, 4);
    assert_eq!(solid_desc.attributes[4].offset, 32);
    assert_eq!(solid_desc.attributes[5].shader_location, 5);
    assert_eq!(solid_desc.attributes[5].offset, 36);

    // Uniforms: mat4x4 (64) + 4 * vec4 (4 * 16 = 64) = 128 bytes
    assert_eq!(std::mem::size_of::<Uniforms>(), 128);
    assert_eq!(std::mem::size_of::<Uniforms>() % 16, 0, "Uniform buffer size must be a multiple of 16 for WebGPU alignment");
}

#[test]
fn test_wgpu_shader_module_compilation_if_device_available() {
    // If a graphics adapter is available in the test environment, validate the shader directly with WGPU/Naga
    let instance = wgpu::Instance::default();
    let adapter_opt = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }));

    if let Some(adapter) = adapter_opt {
        let (device, _queue) = match pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Test Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )) {
            Ok(pair) => pair,
            Err(_) => return, // Device request not supported in this headless environment
        };

        // Create shader modules; WGPU/Naga validates WGSL syntax upon creation
        let line_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader Test"),
            source: wgpu::ShaderSource::Wgsl(LINE_SHADER.into()),
        });
        drop(line_module);

        let solid_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Solid Shader Test"),
            source: wgpu::ShaderSource::Wgsl(SOLID_SHADER.into()),
        });
        drop(solid_module);
    }
}

#[test]
fn test_evo02_mover_3d_continuous_ribbon_trajectory() {
    let mut state = ut99_editor::EditorState::default();
    state.new_map();

    let brush = ut99_bsp::build_cube("MoverPlatform", ut99_bsp::CsgOperation::Active, glam::Vec3::new(256.0, 256.0, 32.0));
    let mut mover = ut99_editor::editor_state::EditorMover::new(brush);
    mover.name = "ElevatorMover".to_string();
    mover.tag = "LiftTag".to_string();
    mover.move_time = 3.0;
    mover.stay_open_time = 2.0;
    mover.glide_type = "Smooth".to_string();

    mover.keyframes[0] = Some(ut99_editor::editor_state::MoverKeyframe {
        location: glam::Vec3::new(0.0, 0.0, 0.0),
        rotation: glam::Vec3::ZERO,
    });
    mover.keyframes[1] = Some(ut99_editor::editor_state::MoverKeyframe {
        location: glam::Vec3::new(256.0, 128.0, 64.0),
        rotation: glam::Vec3::new(0.0, 45.0, 0.0),
    });
    mover.keyframes[2] = Some(ut99_editor::editor_state::MoverKeyframe {
        location: glam::Vec3::new(512.0, 0.0, 128.0),
        rotation: glam::Vec3::new(0.0, 90.0, 0.0),
    });
    state.movers.push(mover);

    state.show_mover_trajectories = true;
    let trajs = state.get_mover_trajectories();
    assert_eq!(trajs.len(), 1);
    assert_eq!(trajs[0].keyframes.len(), 3);
    assert!(trajs[0].spline_points.len() >= 32);
    assert!(trajs[0].total_path_length_uu > 500.0);
}

