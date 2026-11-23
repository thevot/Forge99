use ut99_bsp::{ProceduralColorPalette, ProceduralTextureSimulator, ProceduralTextureType};
use ut99_editor::EditorState;

#[test]
fn test_procedural_fire_and_spark_injection() {
    let mut fire = ProceduralTextureSimulator::new_fire(64, 64);
    assert_eq!(fire.tex_type, ProceduralTextureType::Fire);
    assert_eq!(fire.palette, ProceduralColorPalette::FireThermal);
    assert_eq!(fire.width, 64);
    assert_eq!(fire.height, 64);

    // Inject manual heat spark at center
    fire.add_heat_spark(32, 32, 220.0);

    // Advance 6 simulation steps
    for _ in 0..6 {
        fire.step();
    }

    let rgba = fire.to_rgba8();
    assert_eq!(rgba.len(), 64 * 64 * 4);

    // Verify heated ignited pixels exist in the red and orange channels
    let has_flame = rgba.chunks_exact(4).any(|p| p[0] > 120);
    assert!(has_flame, "Fire simulation must have ignited flame pixels");
}

#[test]
fn test_procedural_water_and_droplet_propagation() {
    let mut water = ProceduralTextureSimulator::new_water(64, 64);
    assert_eq!(water.tex_type, ProceduralTextureType::WaterRipples);
    assert_eq!(water.palette, ProceduralColorPalette::WaterAzure);

    // Inject droplet at center
    water.add_droplet(32, 32, 150.0);

    // Advance 8 steps of 2D wave equation
    for _ in 0..8 {
        water.step();
    }

    let rgba = water.to_rgba8();
    assert_eq!(rgba.len(), 64 * 64 * 4);

    // Azure water has dominant blue and cyan channels
    let has_water_pixels = rgba.chunks_exact(4).any(|p| p[2] > 140 && p[1] > 80);
    assert!(has_water_pixels, "Water simulation must generate blue/cyan ripple pixels");
}

#[test]
fn test_palette_color_generation() {
    let mut plasma = ProceduralTextureSimulator::new_fire(32, 32);
    plasma.palette = ProceduralColorPalette::BluePlasma;
    plasma.add_heat_spark(16, 16, 250.0);
    for _ in 0..4 {
        plasma.step();
    }
    let rgba_plasma = plasma.to_rgba8();
    let has_blue_plasma = rgba_plasma.chunks_exact(4).any(|p| p[2] > 180 && p[2] >= p[0]);
    assert!(has_blue_plasma, "Blue Plasma palette must have dominant blue intensity");

    let mut toxic = ProceduralTextureSimulator::new_fire(32, 32);
    toxic.palette = ProceduralColorPalette::GreenToxic;
    toxic.add_heat_spark(16, 16, 250.0);
    for _ in 0..4 {
        toxic.step();
    }
    let rgba_toxic = toxic.to_rgba8();
    let has_green_toxic = rgba_toxic.chunks_exact(4).any(|p| p[1] > 160 && p[1] >= p[0]);
    assert!(has_green_toxic, "Green Toxic palette must have dominant green intensity");
}

#[test]
fn test_apply_procedural_as_active_texture() {
    let mut state = EditorState::default();
    state.procedural_texture_name = "TorchFireLive".to_string();

    // Advance simulation
    for _ in 0..5 {
        state.step_procedural_texture();
    }

    state.apply_procedural_texture_as_active();

    assert_eq!(state.active_texture.as_deref(), Some("Procedural.TorchFireLive"));

    let tex_opt = state.texture_manager.get_texture("Procedural", "TorchFireLive");
    assert!(tex_opt.is_some(), "Procedural texture must be registered in TextureManager");
    let tex = tex_opt.unwrap();
    assert_eq!(tex.width, 64);
    assert_eq!(tex.height, 64);
    assert_eq!(tex.rgba_pixels.len(), 64 * 64 * 4);
}

#[test]
fn test_export_procedural_animation_strip() {
    let mut state = EditorState::default();
    state.procedural_texture_name = "WaterTestFrames".to_string();
    state.procedural_simulator = ProceduralTextureSimulator::new_water(32, 32);
    state.procedural_simulator.add_droplet(16, 16, 120.0);

    let test_dir = std::env::temp_dir().join(format!(
        "forge99_proctex_test_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));

    let exported_count = state.export_procedural_animation_strip(&test_dir, 4)
        .expect("Exporting procedural animation frames should succeed");

    assert_eq!(exported_count, 4, "Must export exactly 4 frames");

    for i in 0..4 {
        let frame_path = test_dir.join(format!("WaterTestFrames_{:03}.png", i));
        assert!(frame_path.exists(), "Frame file {:?} must exist on disk", frame_path);
        let img = image::open(&frame_path).expect("Frame must be a valid readable PNG image");
        assert_eq!(img.width(), 32);
        assert_eq!(img.height(), 32);
    }

    // Clean up
    let _ = std::fs::remove_dir_all(&test_dir);
}
