use super::*;

#[test]
#[ignore = "manual CPU cache-access benchmark; run with --ignored --nocapture"]
fn benchmark_clone_vs_reuse_zone_cache() {
    use std::{hint::black_box, time::Instant};
    for polygon_count in [10_000, 100_000] {
        let flood = ut99_bsp::ZoneFloodFillResult {
            poly_zones: vec![(1, 2); polygon_count],
            actor_zone_ids: (0..64).map(|i| (format!("ZoneInfo{i}"), i)).collect(),
            leak_paths: (0..64).map(|i| ut99_bsp::ZoneLeakPath {
                zone_a_name: format!("ZoneInfo{i}"), zone_b_name: format!("ZoneInfo{}", i + 1),
                leak_gap_point: Vec3::ZERO, waypoints: vec![Vec3::ONE; 128],
            }).collect(),
            connectivity_mask: vec![0; 64], distinct_zone_count: 64,
        };
        let mut cache = Some((Vec::<ut99_bsp::Plane>::new(), flood));
        let mut clone_times = Vec::new(); let mut reuse_times = Vec::new();
        for round in 0..5 {
            for clone in if round % 2 == 0 { [true, false] } else { [false, true] } {
                let start = Instant::now();
                for _ in 0..200 {
                    let current = if clone { black_box(cache.as_ref().unwrap()).clone() }
                        else { cache.take().unwrap() };
                    black_box(&current);
                    if !clone { cache = Some(current); }
                }
                let microseconds = start.elapsed().as_secs_f64() * 1_000_000.0 / 200.0;
                if clone { clone_times.push(microseconds); } else { reuse_times.push(microseconds); }
            }
        }
        clone_times.sort_by(f64::total_cmp); reuse_times.sort_by(f64::total_cmp);
        eprintln!("polygons={polygon_count}, 64 leak paths x 128 points: cache clone={:.3} us, reuse={:.3} us",
            clone_times[2], reuse_times[2]);
    }
}

#[test]
#[ignore = "renders Zone Portal on a real GPU; run serially"]
fn warm_zone_cache_matches_recalculation_after_zone_and_geometry_edits() {
    use crate::camera::ViewportShading;
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY, ..Default::default()
    });
    let (_, device, queue, diagnostics) = pollster::block_on(crate::graphics::initialize_device(&instance, None)).unwrap();
    eprintln!("GPU: {:?}", diagnostics.adapter);
    let mut renderer = Renderer::new(&device, wgpu::TextureFormat::Bgra8UnormSrgb, 64, 64);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut state = crate::EditorState::default();
    state.viewport_shading = ViewportShading::ZonePortal;
    let mut brush = ut99_bsp::build_cube("Portal", CsgOperation::Add, Vec3::splat(256.0));
    brush.polys[0].flags |= ut99_bsp::PF_PORTAL;
    state.world_brushes = vec![brush].into();
    let mut level = ut99_format::UnrLevel::default();
    for index in 0..2 {
        let mut actor = ut99_format::unr::actor::ActorRecord::default();
        actor.name = format!("ZoneInfo{index}"); actor.class_name = "ZoneInfo".into();
        actor.location = Vec3::new(512.0 + index as f32 * 256.0, 0.0, 0.0); level.actors.push(actor);
    }
    state.loaded_unr = Some(level);
    for step in 0..4 {
        if step == 1 { state.loaded_unr.as_mut().unwrap().actors[0].location.x += 128.0; }
        if step == 2 { state.loaded_unr.as_mut().unwrap().actors[1].name = "RenamedZone".into(); }
        if step == 3 { state.world_brushes[0].location.z += 64.0; state.geometry_revision += 1; }
        renderer.render(&device, &queue, &view, &state);
        let expected_cache = renderer.cached_zone_flood.clone().expect("cache restored after frame");
        if step == 0 { assert!(!expected_cache.1.leak_paths.is_empty(), "fixture must exercise leak-path rendering"); }
        let expected_signature = renderer.cached_zone_flood_sig;
        let pointer = renderer.cached_zone_flood.as_ref().unwrap().1.poly_zones.as_ptr();
        let lines = bytemuck::cast_slice::<LineVertex, u8>(&renderer.frame_lines).to_vec();
        let solids = bytemuck::cast_slice::<SolidVertex, u8>(&renderer.frame_solids).to_vec();
        renderer.render(&device, &queue, &view, &state);
        assert_eq!(renderer.cached_zone_flood.as_ref().unwrap().1.poly_zones.as_ptr(), pointer);
        assert_eq!(bytemuck::cast_slice::<LineVertex, u8>(&renderer.frame_lines), lines);
        assert_eq!(bytemuck::cast_slice::<SolidVertex, u8>(&renderer.frame_solids), solids);
        // Recalculation must produce the exact same flood data and GPU vertex streams.
        renderer.cached_zone_flood = None;
        renderer.render(&device, &queue, &view, &state);
        assert_eq!(renderer.cached_zone_flood.as_ref(), Some(&expected_cache));
        assert_eq!(renderer.cached_zone_flood_sig, expected_signature);
        assert_eq!(bytemuck::cast_slice::<LineVertex, u8>(&renderer.frame_lines), lines);
        assert_eq!(bytemuck::cast_slice::<SolidVertex, u8>(&renderer.frame_solids), solids);
    }
    let signature = renderer.cached_zone_flood_sig;
    state.viewport_shading = ViewportShading::Unlit;
    renderer.render(&device, &queue, &view, &state);
    assert!(renderer.cached_zone_flood.is_some()); assert_eq!(renderer.cached_zone_flood_sig, signature);
    state.viewport_shading = ViewportShading::ZonePortal;
    state.world_brushes.clear(); state.loaded_unr.as_mut().unwrap().actors.clear();
    state.geometry_revision += 1;
    renderer.render(&device, &queue, &view, &state);
    assert!(renderer.cached_zone_flood.as_ref().unwrap().1.poly_zones.is_empty());
    device.poll(wgpu::Maintain::Wait);
}
