//! Automated Integration Tests for Interactive Dynamic Zone Water Surface & Wave Reflection Studio

use glam::Vec3;
use ut99_bsp::WaterPreset;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_water_presets_and_config_transitions() {
    let mut state = EditorState::default();

    // Verify initial default preset
    assert_eq!(state.water_surface_studio_settings.selected_preset, WaterPreset::CrystalClearPool);
    assert_eq!(state.water_surface_studio_settings.active_config.wave_speed, 1.2);
    assert_eq!(state.water_surface_studio_settings.active_config.refraction_index, 1.333);

    // 1. Switch to FastRiverRapids
    state.apply_water_preset(WaterPreset::FastRiverRapids);
    assert_eq!(state.water_surface_studio_settings.selected_preset, WaterPreset::FastRiverRapids);
    assert_eq!(state.water_surface_studio_settings.active_config.wave_speed, 1.8);
    assert!(state.water_surface_studio_settings.active_config.flow_velocity.x > 50.0);

    // 2. Switch to LavaMagmaCrust
    state.apply_water_preset(WaterPreset::LavaMagmaCrust);
    assert_eq!(state.water_surface_studio_settings.selected_preset, WaterPreset::LavaMagmaCrust);
    assert_eq!(state.water_surface_studio_settings.active_config.wave_speed, 0.4);
    assert_eq!(state.water_surface_studio_settings.active_config.refraction_index, 1.550);

    // 3. Test Undo
    state.undo();
    assert_eq!(state.water_surface_studio_settings.selected_preset, WaterPreset::FastRiverRapids);
    assert_eq!(state.water_surface_studio_settings.active_config.wave_speed, 1.8);

    // 4. Test Redo
    state.redo();
    assert_eq!(state.water_surface_studio_settings.selected_preset, WaterPreset::LavaMagmaCrust);
    assert_eq!(state.water_surface_studio_settings.active_config.wave_speed, 0.4);
}

#[test]
fn test_wave_simulation_integration_and_ripples() {
    let mut state = EditorState::default();

    // Ensure sim starts flat
    assert_eq!(state.water_surface_studio_settings.simulation.current_heights[16 * 32 + 16], 0.0);

    // Inject UV ripple at center
    state.inject_water_ripple_uv(0.5, 0.5, 20.0);
    let center_h = state.water_surface_studio_settings.simulation.current_heights[16 * 32 + 16];
    assert!(center_h > 15.0, "Center height must be displaced by impulse");

    // Step simulation 10 times
    for _ in 0..10 {
        state.step_water_surface_simulation(0.016);
    }

    // Verify wave spread to adjacent cells
    let neighbor_h = state.water_surface_studio_settings.simulation.current_heights[16 * 32 + 18];
    assert!(neighbor_h.abs() > 0.01, "Wave energy must propagate through the grid");

    // Test world-space ripple injection
    state.water_surface_studio_settings.sheet_center = Vec3::new(100.0, 200.0, 0.0);
    state.water_surface_studio_settings.sheet_size_x = 512.0;
    state.water_surface_studio_settings.sheet_size_y = 512.0;
    state.inject_water_ripple_world(Vec3::new(100.0, 200.0, 0.0), 30.0);

    let re_center_h = state.water_surface_studio_settings.simulation.current_heights[16 * 32 + 16];
    assert!(re_center_h > 20.0, "World position ripple must map to UV center correctly");

    // Test Reset
    state.reset_water_simulation();
    for &h in &state.water_surface_studio_settings.simulation.current_heights {
        assert_eq!(h, 0.0);
    }
    for &c in &state.water_surface_studio_settings.simulation.caustics_intensity_map {
        assert_eq!(c, 1.0);
    }
}

#[test]
fn test_watertight_water_sheet_brush_generation() {
    let mut state = EditorState::default();
    let initial_count = state.world_brushes.len();

    // Configure water sheet
    state.water_surface_studio_settings.sheet_center = Vec3::new(256.0, 512.0, 32.0);
    state.water_surface_studio_settings.sheet_size_x = 1024.0;
    state.water_surface_studio_settings.sheet_size_y = 1024.0;
    state.water_surface_studio_settings.subdivisions_x = 4;
    state.water_surface_studio_settings.subdivisions_y = 4;
    state.water_surface_studio_settings.sheet_texture_name = "GenFluid.Water.WetWater1".to_string();

    let brush_idx = state.generate_water_sheet_brush();
    assert_eq!(brush_idx, initial_count);
    assert_eq!(state.world_brushes.len(), initial_count + 1);

    let brush = &state.world_brushes[brush_idx];
    assert!(brush.name.starts_with("WaterSheet_"));
    assert_eq!(brush.location, Vec3::new(256.0, 512.0, 32.0));
    assert_eq!(brush.polys.len(), 4 * 4 * 2, "4x4 grid of quads -> 32 triangles");

    for poly in &brush.polys {
        assert_eq!(poly.plane.normal, Vec3::Z, "Water sheet polygon normal must face upward");
        assert_eq!(poly.texture, "GenFluid.Water.WetWater1");
        assert_ne!(poly.flags & 0x00000004, 0, "PF_Translucent flag must be set");
        assert_ne!(poly.flags & 0x00000100, 0, "PF_TwoSided flag must be set");
    }
}

#[test]
fn test_water_surface_studio_undo_redo_invariance() {
    let mut state = EditorState::default();
    let initial_count = state.world_brushes.len();

    state.water_surface_studio_settings.sheet_center = Vec3::new(0.0, 0.0, 100.0);
    state.water_surface_studio_settings.subdivisions_x = 2;
    state.water_surface_studio_settings.subdivisions_y = 2;

    // 1. Generate brush
    state.generate_water_sheet_brush();
    assert_eq!(state.world_brushes.len(), initial_count + 1);

    // 2. Undo creation
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_count, "Undo must remove generated water sheet brush");

    // 3. Redo creation
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_count + 1, "Redo must restore generated water sheet brush");
    assert_eq!(state.world_brushes[initial_count].location, Vec3::new(0.0, 0.0, 100.0));
}
