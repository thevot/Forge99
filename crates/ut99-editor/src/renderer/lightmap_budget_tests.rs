use super::*;
fn setup() -> (wgpu::Device, wgpu::Queue, Renderer) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY, ..Default::default()
    });
    let (_, device, queue, diagnostics) = pollster::block_on(crate::graphics::initialize_device(&instance, None)).unwrap();
    eprintln!("GPU: {:?}", diagnostics.adapter);
    let renderer = Renderer::new(&device, wgpu::TextureFormat::Bgra8UnormSrgb, 64, 64);
    (device, queue, renderer)
}
fn lightmap(index: usize) -> ut99_bsp::LightmapData {
    lightmap_dims(index, 8, 8)
}

fn lightmap_dims(index: usize, w: usize, h: usize) -> ut99_bsp::LightmapData {
    let r = (index & 0xFF) as f32 / 255.0;
    let g = ((index >> 8) & 0xFF) as f32 / 255.0;
    let b = (200usize.saturating_sub(index % 128)) as f32 / 255.0;
    ut99_bsp::LightmapData {
        width: w,
        height: h,
        luxels: vec![[r, g, b]; w * h],
    }
}

#[test]
#[ignore = "manual GPU-backed lightmap upload benchmark; run serially"]
fn benchmark_lightmap_atlas_pressure() {
    let (device, queue, mut renderer) = setup();
    let lightmaps: Vec<_> = (0..120).map(lightmap).collect();
    for lm in &lightmaps {
        renderer.resolve_or_upload_lightmap_rect(&queue, lm);
    }
    device.poll(wgpu::Maintain::Wait);
    let mut times = Vec::new();
    for _ in 0..5 {
        let started = std::time::Instant::now();
        for lm in &lightmaps {
            std::hint::black_box(renderer.resolve_or_upload_lightmap_rect(&queue, lm));
        }
        times.push(started.elapsed().as_secs_f64() * 1000.0);
        device.poll(wgpu::Maintain::Wait);
    }
    times.sort_by(f64::total_cmp);
    eprintln!(
        "120 unique lightmaps, warm CPU lookup/upload submission median: {:.3} ms",
        times[2]
    );
}

#[test]
fn malformed_lightmaps_do_not_produce_cache_keys() {
    let mut lm = lightmap(0);
    lm.width = usize::MAX;
    lm.height = 2;
    assert_eq!(lightmap_content_key(&lm), None);
    lm.width = 0;
    assert_eq!(lightmap_content_key(&lm), None);
    lm.width = 16;
    lm.height = 16;
    assert_eq!(lightmap_content_key(&lm), None);
}

#[test]
#[ignore = "requires GPU texture readback; run serially"]
fn full_atlas_preserves_pixels_and_reclaims_only_inactive_slots() {
    let (device, queue, mut renderer) = setup();
    renderer.lightmap_atlas = crate::renderer::lightmap_atlas::LightmapAtlas::new_with_usage(
        &device,
        64,
        wgpu::TextureUsages::COPY_SRC,
    );
    renderer.rebuild_texture_bind_group(&device);

    // 6x6 lightmaps become 8x8 padded tiles -> 8x8 grid = 64 tiles exactly in 64x64.
    let first = renderer
        .resolve_or_upload_lightmap_rect(&queue, &lightmap_dims(42, 6, 6))
        .expect("first tile fits");
    for index in 1..64 {
        assert!(
            renderer
                .resolve_or_upload_lightmap_rect(&queue, &lightmap_dims(100 + index, 6, 6))
                .is_some(),
            "tile {index} should fit in 64x64 atlas"
        );
    }
    assert_eq!(renderer.last_lightmap_fallbacks, 0);
    for index in 64..100 {
        assert!(
            renderer
                .resolve_or_upload_lightmap_rect(&queue, &lightmap_dims(100 + index, 6, 6))
                .is_none(),
            "overflow tile {index} should fall back"
        );
    }
    assert_eq!(renderer.last_lightmap_fallbacks, 36);
    assert_eq!(renderer.lightmap_atlas.len(), 64);

    let before_inner = read_atlas_pixel(&device, &queue, &renderer.lightmap_atlas.texture, first.x + 1, first.y + 1);
    let before_tl = read_atlas_pixel(&device, &queue, &renderer.lightmap_atlas.texture, first.x, first.y);
    let before_br = read_atlas_pixel(
        &device,
        &queue,
        &renderer.lightmap_atlas.texture,
        first.x + first.width + 1,
        first.y + first.height + 1,
    );
    assert_eq!(before_inner[0], 42);
    assert_eq!(before_tl, before_inner);
    assert_eq!(before_br, before_inner);

    // A new world keeps 1 surviving + 1 new lightmap, compacts away dead slots, and packs both with exact RGB.
    let mut brush = ut99_bsp::build_cube("Replacement", CsgOperation::Add, Vec3::splat(64.0));
    brush.polys[0].lightmap = Some(lightmap_dims(120, 6, 6));
    brush.polys[1].lightmap = Some(lightmap_dims(42, 6, 6));
    renderer.cached_carved_polys = brush.to_world_polys().into_iter().map(|poly| (0, poly)).collect();
    renderer.refresh_cached_lightmap_keys();
    assert_eq!(renderer.lightmap_atlas.len(), 0);

    let survived = renderer
        .resolve_or_upload_lightmap_rect(&queue, &lightmap_dims(42, 6, 6))
        .expect("surviving lightmap re-packs after compaction");
    let replacement = renderer
        .resolve_or_upload_lightmap_rect(&queue, &lightmap_dims(120, 6, 6))
        .expect("new lightmap packs after reclaiming dead tiles");
    assert_ne!((survived.x, survived.y), (replacement.x, replacement.y));
    assert_eq!(
        read_atlas_pixel(&device, &queue, &renderer.lightmap_atlas.texture, survived.x + 1, survived.y + 1),
        before_inner
    );
    assert_eq!(
        read_atlas_pixel(&device, &queue, &renderer.lightmap_atlas.texture, replacement.x + 1, replacement.y + 1)[0],
        120
    );
    assert_eq!(
        read_atlas_pixel(&device, &queue, &renderer.lightmap_atlas.texture, replacement.x, replacement.y)[0],
        120
    );
}

fn read_atlas_pixel(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    x: u32,
    y: u32,
) -> [u8; 4] {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    receiver.recv().unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range();
    let pixel = mapped[..4].try_into().unwrap();
    drop(mapped);
    buffer.unmap();
    pixel
}

#[test]
#[ignore = "renders a lightmapped world on a real GPU; run serially"]
fn world_lightmaps_refresh_after_edits_shading_changes_and_new_map() {
    use crate::camera::ViewportShading;
    let (device, queue, mut renderer) = setup();
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut state = crate::EditorState::default();
    state.world_brushes.clear();
    state.viewport_shading = ViewportShading::Lit;
    for index in 0..84 {
        let mut brush = ut99_bsp::build_cube("Budget", CsgOperation::Add, Vec3::splat(64.0));
        brush.location = Vec3::new(256.0, index as f32 * 256.0, 0.0);
        for (poly_index, poly) in brush.polys.iter_mut().enumerate() {
            poly.lightmap = Some(lightmap_dims(index * 6 + poly_index, 16, 16));
            poly.vertex_colors = vec![[0.7; 3]; poly.vertices.len()];
        }
        state.world_brushes.push(brush);
    }
    state.multi_selected_brushes.extend(0..84);
    renderer.render(&device, &queue, &view, &state);
    assert_eq!(renderer.lightmap_atlas.len(), 504);
    assert_eq!(renderer.last_lightmap_fallbacks, 0);

    let expected = bytemuck::cast_slice::<SolidVertex, u8>(&renderer.frame_solids).to_vec();
    renderer.render(&device, &queue, &view, &state);
    assert_eq!(
        bytemuck::cast_slice::<SolidVertex, u8>(&renderer.frame_solids),
        expected
    );
    assert_eq!(renderer.last_frame_stats().uploaded_solid_bytes, 0);

    let cached_key = *renderer.lightmap_atlas.keys().next().unwrap();
    assert!(renderer.lightmap_atlas.contains_key(cached_key));
    state.viewport_shading = ViewportShading::Unlit;
    for brush in &mut state.world_brushes {
        for poly in &mut brush.polys {
            poly.lightmap = Some(lightmap(200));
        }
    }
    state.geometry_revision += 1;
    renderer.render(&device, &queue, &view, &state);
    state.viewport_shading = ViewportShading::Lit;
    renderer.render(&device, &queue, &view, &state);
    assert_eq!(renderer.last_lightmap_fallbacks, 0);
    assert_eq!(renderer.lightmap_atlas.len(), 1);
    assert!(!renderer.lightmap_atlas.contains_key(cached_key));
    assert!(renderer
        .lightmap_atlas
        .contains_key(lightmap_content_key(&lightmap(200)).unwrap()));

    state.world_brushes.clear();
    state.multi_selected_brushes.clear();
    state.geometry_revision += 1;
    renderer.render(&device, &queue, &view, &state);
    assert!(renderer.lightmap_atlas.is_empty());
    assert!(renderer.cached_lightmap_keys.is_empty());
    device.poll(wgpu::Maintain::Wait);
}

#[test]
#[ignore = "verifies 1-texel gutter prevents neighbor tile bleed on GPU; run serially"]
fn atlas_gutter_prevents_neighbor_bleeding_on_rendered_quad() {
    use crate::camera::{ViewportLayout, ViewportShading};
    let (device, queue, mut renderer) = setup();
    // 64x64 atlas with 4x4 lightmaps (6x6 padded tiles) -> 10 tiles per row so neighboring rows/cols touch directly.
    renderer.lightmap_atlas = crate::renderer::lightmap_atlas::LightmapAtlas::new(&device, 64);
    renderer.rebuild_texture_bind_group(&device);

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("lightmap gutter bleed check"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
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

    let mut state = crate::EditorState::default();
    state.world_brushes.clear();
    state.viewport_layout = ViewportLayout::Single;
    state.viewport_shading = ViewportShading::Lit;
    state.show_builder_brush = false;
    state.sky_zone_settings.show_frustum = false;
    state.corona_studio_settings.show_occlusion_rays_viewport = false;
    state.camera.position = Vec3::new(-48.0, 0.0, 2000.0);
    state.camera.yaw = 0.0;
    state.camera.pitch = 0.0;
    state.texture_manager.insert_package(
        "GutterPack".to_string(),
        ut99_format::UtxPackage {
            package_name: "GutterPack".to_string(),
            textures: vec![ut99_format::UtxTexture {
                name: "White".into(),
                group: "Base".into(),
                width: 4,
                height: 4,
                format: 1,
                masked: false,
                rgba_pixels: vec![255; 4 * 4 * 4],
            }],
        },
    );

    // Surround the hero red tile on row 1 col 1 by packing distinct bright-green tiles before and after it behind the camera.
    let green_lm = |id: usize| ut99_bsp::LightmapData {
        width: 4,
        height: 4,
        luxels: vec![[0.0, 1.0, (id as f32) / 255.0]; 16],
    };
    let red_lm = ut99_bsp::LightmapData {
        width: 4,
        height: 4,
        luxels: vec![[1.0, 0.0, 0.0]; 16],
    };

    for idx in 0..25usize {
        if idx == 11 {
            let mut hero = ut99_bsp::build_cube("HeroRed", CsgOperation::Add, Vec3::splat(160.0));
            hero.location = Vec3::new(112.0, 0.0, 2000.0);
            for p in &mut hero.polys {
                p.texture = "GutterPack.Base.White".into();
                p.lightmap = Some(red_lm.clone());
            }
            state.world_brushes.push(hero);
        } else {
            let mut decoy = ut99_bsp::build_cube(
                format!("DecoyGreen_{idx}"),
                CsgOperation::Add,
                Vec3::splat(32.0),
            );
            decoy.location = Vec3::new(-400.0 - idx as f32 * 48.0, 0.0, 2000.0);
            for p in &mut decoy.polys {
                p.lightmap = Some(green_lm(idx));
            }
            state.world_brushes.push(decoy);
        }
    }

    renderer.render(&device, &queue, &view, &state);
    assert_eq!(renderer.last_lightmap_fallbacks, 0);
    assert_eq!(renderer.lightmap_atlas.len(), 25);

    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let unpadded = 64usize * 4;
    let padded = unpadded.div_ceil(align) * align;
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (padded * 64) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(64),
            },
        },
        wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buf.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();
    let mapped = buf.slice(..).get_mapped_range();

    let mut red_pixels = 0usize;
    for y in 0..64usize {
        let row = &mapped[y * padded..y * padded + unpadded];
        for px in row.chunks_exact(4) {
            let (b, g, r) = (px[0], px[1], px[2]);
            if r > 120 && r > b.saturating_add(40) {
                red_pixels += 1;
                assert!(
                    g <= 2,
                    "neighbor green tile bled into red lightmapped surface: rgb=({r},{g},{b})"
                );
            }
        }
    }
    drop(mapped);
    buf.unmap();
    assert!(red_pixels > 200, "expected lit red surface pixels, got {red_pixels}");
}
