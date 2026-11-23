use super::*;

#[test]
fn oversized_vertex_storage_is_released() {
    const TEST_CAP: usize = 32 * 1024 * 1024;
    let vertices = Vec::<u8>::with_capacity(TEST_CAP + 1);
    assert_eq!(retain_vertex_storage(vertices, TEST_CAP).capacity(), 0);
    let vertices = Vec::<u8>::with_capacity(1024);
    let pointer = vertices.as_ptr();
    let retained = retain_vertex_storage(vertices, TEST_CAP);
    assert_eq!(retained.as_ptr(), pointer);
    assert_eq!(retained.capacity(), 1024);
}

#[test]
#[ignore = "requires a graphics adapter; run serially"]
fn reused_storage_matches_fresh_geometry_after_scene_and_view_changes() {
    use crate::camera::{ViewportLayout, ViewportShading};
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY, ..Default::default()
    });
    let (_, device, queue, diagnostics) = pollster::block_on(
        crate::graphics::initialize_device(&instance, None),
    ).unwrap();
    eprintln!("GPU: {:?}", diagnostics.adapter);
    let mut renderer = Renderer::new(&device, wgpu::TextureFormat::Bgra8UnormSrgb, 64, 64);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vertex reuse regression target"),
        size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut state = crate::EditorState::default();
    state.world_brushes.clear();
    for index in 0..16 {
        let mut brush = ut99_bsp::build_cube("Reuse", CsgOperation::Add, Vec3::splat(64.0));
        brush.location = Vec3::new(256.0, index as f32 * 80.0, 0.0);
        state.world_brushes.push(brush);
    }
    // Selected brushes bypass frustum culling, ensuring both vertex lists are exercised.
    state.multi_selected_brushes.extend(0..16);
    for layout in [ViewportLayout::Single, ViewportLayout::FourViewports] {
        state.viewport_layout = layout;
        for shading in [ViewportShading::Wireframe, ViewportShading::Unlit, ViewportShading::DepthComplexity, ViewportShading::ZonePortal] {
            state.viewport_shading = shading;
            state.viewport_shadings = [shading; 4];
            renderer.render(&device, &queue, &view, &state);
            let line_pointer = renderer.frame_lines.as_ptr();
            let solid_pointer = renderer.frame_solids.as_ptr();
            let solid_idx_pointer = renderer.frame_solid_indices.as_ptr();
            let expected_lines = bytemuck::cast_slice::<LineVertex, u8>(&renderer.frame_lines).to_vec();
            let expected_solids = bytemuck::cast_slice::<SolidVertex, u8>(&renderer.frame_solids).to_vec();
            let expected_solid_indices = bytemuck::cast_slice::<u32, u8>(&renderer.frame_solid_indices).to_vec();
            assert!(!expected_lines.is_empty());
            if shading == ViewportShading::Unlit { assert!(!expected_solids.is_empty() && !expected_solid_indices.is_empty()); }
            renderer.render(&device, &queue, &view, &state);
            assert_eq!(renderer.frame_lines.as_ptr(), line_pointer);
            assert_eq!(renderer.frame_solids.as_ptr(), solid_pointer);
            assert_eq!(renderer.frame_solid_indices.as_ptr(), solid_idx_pointer);
            assert_eq!(bytemuck::cast_slice::<LineVertex, u8>(&renderer.frame_lines), expected_lines);
            assert_eq!(bytemuck::cast_slice::<SolidVertex, u8>(&renderer.frame_solids), expected_solids);
            assert_eq!(bytemuck::cast_slice::<u32, u8>(&renderer.frame_solid_indices), expected_solid_indices);
        }
    }
    state.world_brushes.clear();
    state.multi_selected_brushes.clear();
    renderer.render(&device, &queue, &view, &state);
    let small_lines = bytemuck::cast_slice::<LineVertex, u8>(&renderer.frame_lines).to_vec();
    let small_solids = bytemuck::cast_slice::<SolidVertex, u8>(&renderer.frame_solids).to_vec();
    let small_solid_indices = bytemuck::cast_slice::<u32, u8>(&renderer.frame_solid_indices).to_vec();
    renderer.frame_lines = Vec::new();
    renderer.frame_solids = Vec::new();
    renderer.frame_solid_indices = Vec::new();
    renderer.render(&device, &queue, &view, &state);
    assert_eq!(bytemuck::cast_slice::<LineVertex, u8>(&renderer.frame_lines), small_lines);
    assert_eq!(bytemuck::cast_slice::<SolidVertex, u8>(&renderer.frame_solids), small_solids);
    assert_eq!(bytemuck::cast_slice::<u32, u8>(&renderer.frame_solid_indices), small_solid_indices);
    device.poll(wgpu::Maintain::Wait);
}

#[test]
#[ignore = "renders depth-complexity geometry on a real GPU; run serially"]
fn depth_colors_match_legacy_overlap_counts_after_geometry_and_visibility_changes() {
    use crate::camera::ViewportShading;
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY, ..Default::default()
    });
    let (_, device, queue, _) = pollster::block_on(crate::graphics::initialize_device(&instance, None)).unwrap();
    let mut renderer = Renderer::new(&device, wgpu::TextureFormat::Bgra8UnormSrgb, 64, 64);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut state = crate::EditorState::default();
    state.world_brushes = (0..4).map(|index| {
        let mut brush = ut99_bsp::build_cube("Depth", CsgOperation::Add, Vec3::splat(64.0));
        brush.location = Vec3::new(index as f32 * 32.0, 0.0, 0.0); brush
    }).collect();
    state.viewport_shading = ViewportShading::DepthComplexity;
    state.show_builder_brush = false;
    for step in 0..3 {
        if step == 1 { state.world_brushes[0].location = Vec3::splat(512.0); state.geometry_revision += 1; }
        if step == 2 { state.hidden_brushes.insert(1); }
        renderer.render(&device, &queue, &view, &state);
        let bounds: Vec<_> = state.world_brushes.iter().map(|brush| brush.world_bounds()).collect();
        let mut expected = Vec::new();
        for (index, poly) in &renderer.cached_carved_polys {
            if !state.is_brush_visible(*index) || poly.vertices.len() < 3 { continue; }
            let (min, max) = bounds[*index];
            let overlaps = bounds.iter().enumerate().filter(|&(other, &(other_min, other_max))| {
                other != *index && min.x <= other_max.x && max.x >= other_min.x
                    && min.y <= other_max.y && max.y >= other_min.y
                    && min.z <= other_max.z && max.z >= other_min.z
            }).count();
            let color = depth_complexity_color(poly.vertices.len(), overlaps);
            for triangle in 1..poly.vertices.len() - 1 {
                for vertex in [0, triangle, triangle + 1] {
                    expected.push((poly.vertices[vertex].to_array(), color));
                }
            }
        }
        assert!(!expected.is_empty());
        assert!(renderer.frame_solid_indices.len() >= expected.len());
        for (&v_idx, (position, color)) in renderer.frame_solid_indices.iter().zip(expected) {
            let actual = &renderer.frame_solids[v_idx as usize];
            assert_eq!(actual.position, position);
            assert_eq!(actual.color, SolidVertex::pack_color(color));
        }
    }
    device.poll(wgpu::Maintain::Wait);
}

#[test]
#[ignore = "renders cached grids on a real GPU; run serially"]
fn cached_grid_prefix_matches_legacy_in_each_viewport_and_after_settings_changes() {
    use crate::camera::{ViewportLayout, ViewportType};
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY, ..Default::default()
    });
    let (_, device, queue, _) = pollster::block_on(crate::graphics::initialize_device(&instance, None)).unwrap();
    let mut renderer = Renderer::new(&device, wgpu::TextureFormat::Bgra8UnormSrgb, 64, 64);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut state = crate::EditorState::default();
    for (step, extent) in [(64.0, 256.0), (32.0, 512.0), (64.0, 256.0)] {
        state.grid.step = step; state.grid.extent = extent;
        for layout in [ViewportLayout::Single, ViewportLayout::FourViewports] {
            state.viewport_layout = layout;
            for viewport in [ViewportType::Top, ViewportType::Side, ViewportType::Front, ViewportType::Perspective] {
                state.active_viewport_type = viewport;
                let mut expected = Vec::new();
                let planes = if layout == ViewportLayout::FourViewports {
                    vec![ViewportType::Top, ViewportType::Side, ViewportType::Front]
                } else { vec![viewport] };
                for plane in planes { expected.extend(state.grid.generate_vertices_for_viewport(plane)); }
                renderer.render(&device, &queue, &view, &state);
                assert_eq!(bytemuck::cast_slice::<LineVertex, u8>(&renderer.frame_lines[..expected.len()]),
                    bytemuck::cast_slice::<LineVertex, u8>(&expected));
                assert!(renderer.grid_cache_bytes() <= 3 * 1024 * 1024);
            }
        }
    }
    device.poll(wgpu::Maintain::Wait);
}

fn create_parity_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("static cache parity target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    (target, view)
}

fn read_target_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let unpadded_row = width as usize * 4;
    let padded_row = unpadded_row.div_ceil(align) * align;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (padded_row * height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row as u32),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range();
    let mut pixels = vec![0u8; unpadded_row * height as usize];
    for y in 0..height as usize {
        let src = &mapped[y * padded_row..y * padded_row + unpadded_row];
        pixels[y * unpadded_row..(y + 1) * unpadded_row].copy_from_slice(src);
    }
    drop(mapped);
    buffer.unmap();
    pixels
}

fn assert_rgba_parity(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    cached_target: &wgpu::Texture,
    state: &crate::EditorState,
    width: u32,
    height: u32,
) {
    let cached_px = read_target_rgba(device, queue, cached_target, width, height);
    let mut cold = Renderer::new(device, wgpu::TextureFormat::Bgra8UnormSrgb, width, height);
    let (cold_target, cold_view) = create_parity_target(device, width, height);
    cold.render(device, queue, &cold_view, state);
    let cold_px = read_target_rgba(device, queue, &cold_target, width, height);
    let mut diff_pixels = 0usize;
    let mut first_diff = None;
    for (idx, (a, b)) in cached_px.chunks_exact(4).zip(cold_px.chunks_exact(4)).enumerate() {
        if a != b {
            diff_pixels += 1;
            if first_diff.is_none() {
                let x = idx % width as usize;
                let y = idx / width as usize;
                first_diff = Some((x, y, a.to_vec(), b.to_vec()));
            }
        }
    }
    assert_eq!(
        diff_pixels, 0,
        "[{label}] cached vs cold Renderer pixel mismatch: {diff_pixels} differing pixels (first={first_diff:?})"
    );
}

#[test]
#[ignore = "renders static scene cache invalidation and pixel parity on a real GPU; run serially"]
fn static_scene_cache_zero_upload_on_orbit_and_invalidates_on_all_edits() {
    use crate::camera::{ViewportLayout, ViewportShading};
    use crate::editor_state::EditorMover;

    let (w, h) = (128u32, 128u32);
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let (_, device, queue, diagnostics) =
        pollster::block_on(crate::graphics::initialize_device(&instance, None)).unwrap();
    eprintln!("GPU: {:?}", diagnostics.adapter);

    let mut renderer = Renderer::new(&device, wgpu::TextureFormat::Bgra8UnormSrgb, w, h);
    let (target, view) = create_parity_target(&device, w, h);

    let mut state = crate::EditorState::default();
    state.world_brushes.clear();
    let room = ut99_bsp::build_cube("Room", CsgOperation::Subtract, Vec3::splat(512.0));
    let mut block_a = ut99_bsp::build_cube("BlockA", CsgOperation::Add, Vec3::splat(72.0));
    block_a.location = Vec3::new(140.0, 20.0, -32.0);
    for poly in &mut block_a.polys {
        poly.vertex_colors = vec![[0.85, 0.75, 0.60]; poly.vertices.len()];
    }
    let mut block_b = ut99_bsp::build_cube("BlockB", CsgOperation::Add, Vec3::splat(64.0));
    block_b.location = Vec3::new(40.0, -140.0, 24.0);
    state.world_brushes.push(room);
    state.world_brushes.push(block_a);
    state.world_brushes.push(block_b);

    let mut mover_brush = ut99_bsp::build_cube("Mover0", CsgOperation::Add, Vec3::splat(48.0));
    mover_brush.location = Vec3::new(96.0, -48.0, -40.0);
    state.movers.push(EditorMover::new(mover_brush));
    state.show_mover_polys = true;

    let mut level = ut99_format::UnrLevel::default();
    let mut light = ut99_format::unr::actor::ActorRecord::default();
    light.name = "Light0".into();
    light.class_name = "Light".into();
    light.location = Vec3::new(0.0, 0.0, 96.0);
    light.light_brightness = 200;
    let mut spawn = ut99_format::unr::actor::ActorRecord::default();
    spawn.name = "PlayerStart0".into();
    spawn.class_name = "PlayerStart".into();
    spawn.location = Vec3::new(64.0, 64.0, -16.0);
    let mut sky = ut99_format::unr::actor::ActorRecord::default();
    sky.name = "SkyZoneInfo0".into();
    sky.class_name = "SkyZoneInfo".into();
    sky.location = Vec3::new(-96.0, 64.0, 64.0);
    level.actors.push(light);
    level.actors.push(spawn);
    level.actors.push(sky);
    state.loaded_unr = Some(level);

    state.viewport_layout = ViewportLayout::Single;
    state.viewport_shading = ViewportShading::Lit;
    state.viewport_shadings = [ViewportShading::Lit; 4];
    state.show_skybox_links = true;
    state.show_actor_radii = true;
    state.show_builder_brush = false;
    state.corona_studio_settings.show_occlusion_rays_viewport = false;
    state.sky_zone_settings.show_frustum = false;
    state.selected_brush_index = None;
    state.selected_actor_index = None;
    state.selected_mover_index = None;
    state.multi_selected_brushes.clear();
    state.multi_selected_actors.clear();
    state.selected_surfaces.clear();
    state.camera.position = Vec3::new(-180.0, 0.0, 32.0);
    state.camera.yaw = 0.0;
    state.camera.pitch = -0.1;

    // Cold initial frame
    renderer.render(&device, &queue, &view, &state);
    let cold_stats = renderer.last_frame_stats();
    assert!(cold_stats.uploaded_solid_bytes > 0);
    assert!(cold_stats.uploaded_line_bytes > 0);
    assert!(cold_stats.csg_rebuilt);
    assert_rgba_parity(&device, &queue, "cold_initial", &target, &state, w, h);

    // Orbit poses 0..=3 with camera LoS ray off: zero GPU upload, constant TrackedVec revs, unchanged CPU pointers, 0 pixel diff
    let revs_before = (
        state.world_brushes.rev(),
        state.movers.rev(),
        state.static_meshes.rev(),
        state.projectors.rev(),
        state.active_projector_decals.rev(),
        state.ai_path_links.rev(),
        state.map.actors.rev(),
        state.loaded_unr.as_ref().unwrap().actors.rev(),
    );
    let line_ptr = renderer.frame_lines.as_ptr();
    let solid_ptr = renderer.frame_solids.as_ptr();
    let studio_line_ptr = renderer.frame_studio_lines.as_ptr();

    let orbit_poses = [
        (Vec3::new(-160.0, 60.0, 40.0), -0.35_f32, -0.15_f32),
        (Vec3::new(-120.0, -90.0, 20.0), 0.45, -0.05),
        (Vec3::new(-80.0, 120.0, 80.0), -0.75, -0.35),
        (Vec3::new(-140.0, -40.0, -24.0), 0.20, 0.12),
    ];
    for (pose_idx, &(pos, yaw, pitch)) in orbit_poses.iter().enumerate() {
        state.camera.position = pos;
        state.camera.yaw = yaw;
        state.camera.pitch = pitch;
        renderer.render(&device, &queue, &view, &state);
        let stats = renderer.last_frame_stats();
        assert_eq!(stats.uploaded_solid_bytes, 0, "orbit pose {pose_idx} uploaded solids");
        assert_eq!(stats.uploaded_line_bytes, 0, "orbit pose {pose_idx} uploaded lines");
        assert!(!stats.csg_rebuilt, "orbit pose {pose_idx} rebuilt CSG");
        assert_eq!(renderer.frame_lines.as_ptr(), line_ptr);
        assert_eq!(renderer.frame_solids.as_ptr(), solid_ptr);
        assert_eq!(renderer.frame_studio_lines.as_ptr(), studio_line_ptr);
        assert_eq!(
            (
                state.world_brushes.rev(),
                state.movers.rev(),
                state.static_meshes.rev(),
                state.projectors.rev(),
                state.active_projector_decals.rev(),
                state.ai_path_links.rev(),
                state.map.actors.rev(),
                state.loaded_unr.as_ref().unwrap().actors.rev(),
            ),
            revs_before,
            "orbit pose {pose_idx} bumped TrackedVec::rev"
        );
        assert_rgba_parity(&device, &queue, &format!("orbit_{pose_idx}"), &target, &state, w, h);
    }

    // Orbit pose 4 with default Corona LoS ray (2 verts) + SkyBox frustum (20 verts) enabled: only dynamic rays upload on camera move
    state.corona_studio_settings.show_occlusion_rays_viewport = true;
    state.sky_zone_settings.show_frustum = true;
    renderer.render(&device, &queue, &view, &state);
    assert_rgba_parity(&device, &queue, "orbit_4_enable_dyn_overlays", &target, &state, w, h);
    let studio_line_ptr_los = renderer.frame_studio_lines.as_ptr();
    state.camera.position = Vec3::new(-180.0, 10.0, 36.0);
    state.camera.yaw = -0.05;
    state.camera.pitch = -0.10;
    renderer.render(&device, &queue, &view, &state);
    let stats = renderer.last_frame_stats();
    assert_eq!(stats.uploaded_solid_bytes, 0, "orbit pose 4 uploaded solids");
    assert_eq!(
        stats.uploaded_line_bytes,
        (20 + 2) * std::mem::size_of::<LineVertex>(),
        "orbit pose 4 should upload only the 20-vertex SkyBox frustum + 2-vertex Corona camera ray"
    );
    assert!(!stats.csg_rebuilt);
    assert_eq!(renderer.frame_lines.as_ptr(), line_ptr);
    assert_eq!(renderer.frame_solids.as_ptr(), solid_ptr);
    assert_eq!(renderer.frame_studio_lines.as_ptr(), studio_line_ptr_los);
    assert_rgba_parity(&device, &queue, "orbit_4_dyn_overlays_only", &target, &state, w, h);

    // Return camera-dependent overlays off and settle before mutation checks
    state.corona_studio_settings.show_occlusion_rays_viewport = false;
    state.sky_zone_settings.show_frustum = false;
    renderer.render(&device, &queue, &view, &state);
    assert_rgba_parity(&device, &queue, "orbit_settle", &target, &state, w, h);

    // 1. Direct brush geometry mutation without touching geometry_revision
    let prev_geom_rev = state.geometry_revision;
    state.world_brushes[1].location.x += 64.0;
    assert_eq!(state.geometry_revision, prev_geom_rev);
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.csg_rebuilt && st.uploaded_solid_bytes > 0 && st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_1_brush_geom", &target, &state, w, h);

    // 2. Direct surface texture/flag edit
    state.world_brushes[1].polys[0].texture = "EditedTex".into();
    state.world_brushes[1].polys[0].flags |= ut99_bsp::PF_TRANSLUCENT;
    renderer.render(&device, &queue, &view, &state);
    assert!(renderer.last_frame_stats().uploaded_solid_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_2_surface_tex_flag", &target, &state, w, h);

    // 3. Hide + unhide brush
    state.toggle_brush_visibility(1);
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0 && st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_3a_hide_brush", &target, &state, w, h);
    state.toggle_brush_visibility(1);
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0 && st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_3b_unhide_brush", &target, &state, w, h);

    // 4. Brush + surface selection change (solids stay cached, lines update)
    state.selected_brush_index = Some(1);
    state.selected_surfaces.insert((1, 0));
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert_eq!(st.uploaded_solid_bytes, 0);
    assert!(st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_4a_select", &target, &state, w, h);
    state.selected_brush_index = None;
    state.selected_surfaces.clear();
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert_eq!(st.uploaded_solid_bytes, 0);
    assert!(st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_4b_deselect", &target, &state, w, h);

    // 5. Direct UnrLevel::actors mutation
    state.loaded_unr.as_mut().unwrap().actors[0].location.z += 32.0;
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert_eq!(st.uploaded_solid_bytes, 0);
    assert!(st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_5_actor_move", &target, &state, w, h);

    // 6. Mover edit
    state.movers[0].brush.location.x += 48.0;
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0 && st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_6_mover_edit", &target, &state, w, h);

    // 7. Shading switch (Lit -> Unlit -> ZonePortal -> DepthComplexity -> Lit)
    for (idx, shading) in [
        ViewportShading::Unlit,
        ViewportShading::ZonePortal,
        ViewportShading::DepthComplexity,
        ViewportShading::Lit,
    ]
    .into_iter()
    .enumerate()
    {
        state.viewport_shading = shading;
        state.viewport_shadings = [shading; 4];
        renderer.render(&device, &queue, &view, &state);
        let st = renderer.last_frame_stats();
        assert!(st.uploaded_solid_bytes > 0, "shading switch {shading:?} must rebuild solids");
        assert!(st.uploaded_line_bytes > 0, "shading switch {shading:?} must rebuild lines");
        assert_rgba_parity(&device, &queue, &format!("mut_7_shading_{idx}_{shading:?}"), &target, &state, w, h);
    }

    // 8. Layout switch (Single -> FourViewports -> Single)
    state.viewport_layout = ViewportLayout::FourViewports;
    renderer.render(&device, &queue, &view, &state);
    assert!(renderer.last_frame_stats().uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_8a_four_viewports", &target, &state, w, h);
    state.viewport_layout = ViewportLayout::Single;
    renderer.render(&device, &queue, &view, &state);
    assert!(renderer.last_frame_stats().uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_8b_single_viewport", &target, &state, w, h);

    // 9. Overlay toggle (show_actor_radii, show_skybox_links)
    state.show_actor_radii = false;
    state.show_skybox_links = false;
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert_eq!(st.uploaded_solid_bytes, 0);
    assert!(st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_9a_overlays_off", &target, &state, w, h);
    state.show_actor_radii = true;
    state.show_skybox_links = true;
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert_eq!(st.uploaded_solid_bytes, 0);
    assert!(st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_9b_overlays_on", &target, &state, w, h);

    // 10. Texture manager reload (resource_revision)
    state.texture_manager.resource_revision += 1;
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0);
    assert_eq!(st.uploaded_line_bytes, 0);
    assert_rgba_parity(&device, &queue, "mut_10_texture_rev", &target, &state, w, h);

    // 11. Studio cache invalidation without rebuilding world static lines
    let static_lines_ptr_before_studio = renderer.frame_lines.as_ptr();
    state.water_surface_studio_settings.sheet_center.z += 32.0;
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert_eq!(st.uploaded_solid_bytes, 0);
    assert!(st.uploaded_line_bytes > 0);
    assert_eq!(renderer.frame_lines.as_ptr(), static_lines_ptr_before_studio);
    assert_rgba_parity(&device, &queue, "mut_11_studio_water", &target, &state, w, h);

    // 11b. Acoustic studio radii spheres & selection invalidation without rebuilding world lines
    let mut amb = ut99_format::ActorRecord::default();
    amb.name = "AmbientSound0".to_string();
    amb.tag = "AmbientSound0".to_string();
    amb.class_name = "Engine.AmbientSound".to_string();
    amb.location = glam::Vec3::new(0.0, 0.0, 64.0);
    state.loaded_unr.as_mut().unwrap().actors.push(amb);
    renderer.render(&device, &queue, &view, &state);
    assert_rgba_parity(&device, &queue, "mut_11b_amb_added", &target, &state, w, h);

    let static_lines_ptr_before_acoustic = renderer.frame_lines.as_ptr();
    state.selected_sound_emitter_tag = Some("AmbientSound0".to_string());
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert_eq!(st.uploaded_solid_bytes, 0);
    assert!(st.uploaded_line_bytes > 0);
    assert_eq!(renderer.frame_lines.as_ptr(), static_lines_ptr_before_acoustic);
    assert_rgba_parity(&device, &queue, "mut_11c_amb_selected", &target, &state, w, h);

    state.acoustic_settings.show_radii_spheres = false;
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert_eq!(st.uploaded_solid_bytes, 0);
    assert!(st.uploaded_line_bytes > 0);
    assert_eq!(renderer.frame_lines.as_ptr(), static_lines_ptr_before_acoustic);
    assert_rgba_parity(&device, &queue, "mut_11d_amb_spheres_off", &target, &state, w, h);

    // 12. Static mesh mutations (add, move, hide, unhide)
    let mut sm = ut99_bsp::build_cube("SM0", CsgOperation::Add, Vec3::splat(44.0));
    sm.location = Vec3::new(32.0, 48.0, -8.0);
    state.static_meshes.push(sm);
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0 && st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_13a_static_mesh_add", &target, &state, w, h);

    state.static_meshes[0].location.z += 28.0;
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0 && st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_13b_static_mesh_move", &target, &state, w, h);

    state.hidden_static_meshes.insert(0);
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0 && st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_13c_static_mesh_hide", &target, &state, w, h);

    state.hidden_static_meshes.remove(&0);
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0 && st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_13d_static_mesh_unhide", &target, &state, w, h);

    // 13. Projector decal push & clear
    state.active_projector_decals.push(ut99_bsp::ProjectedDecalPoly {
        vertices: vec![
            Vec3::new(16.0, -32.0, -16.0),
            Vec3::new(16.0, 32.0, -16.0),
            Vec3::new(16.0, 32.0, 48.0),
            Vec3::new(16.0, -32.0, 48.0),
        ],
        uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        normal: Vec3::NEG_X,
        texture: String::new(),
        blend_mode: ut99_bsp::ProjectorBlendMode::Modulated,
    });
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0 && st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_14a_decal_add", &target, &state, w, h);

    state.active_projector_decals.clear();
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0 && st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_14b_decal_clear", &target, &state, w, h);

    // 14. Builder brush toggle & move (line-only invalidation, solids stay cached)
    state.show_builder_brush = true;
    state.builder_brush.location = Vec3::ZERO;
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert_eq!(st.uploaded_solid_bytes, 0);
    assert!(st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_15a_builder_show", &target, &state, w, h);

    state.builder_brush.location += Vec3::new(48.0, -24.0, 16.0);
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert_eq!(st.uploaded_solid_bytes, 0);
    assert!(st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_15b_builder_move", &target, &state, w, h);

    state.show_builder_brush = false;
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert_eq!(st.uploaded_solid_bytes, 0);
    assert!(st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_15c_builder_hide", &target, &state, w, h);

    // 15. Lightmap attach & luxel mutation on world_brushes[1]
    for poly in &mut state.world_brushes[1].polys {
        poly.lightmap = Some(ut99_bsp::LightmapData {
            width: 4,
            height: 4,
            luxels: vec![[0.95, 0.25, 0.15]; 16],
        });
    }
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0 && renderer.lightmap_atlas.len() > 0);
    assert_rgba_parity(&device, &queue, "mut_16a_lightmap_attach", &target, &state, w, h);

    for poly in &mut state.world_brushes[1].polys {
        if let Some(ref mut lm) = poly.lightmap {
            lm.luxels.fill([0.15, 0.85, 0.95]);
        }
    }
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_16b_lightmap_update", &target, &state, w, h);

    // 16. new_map() invalidation via map_instance_id
    state.new_map();
    renderer.render(&device, &queue, &view, &state);
    let st = renderer.last_frame_stats();
    assert!(st.uploaded_solid_bytes > 0 && st.uploaded_line_bytes > 0);
    assert_rgba_parity(&device, &queue, "mut_12_new_map", &target, &state, w, h);

    device.poll(wgpu::Maintain::Wait);
}
