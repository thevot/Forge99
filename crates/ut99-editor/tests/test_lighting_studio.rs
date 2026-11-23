use glam::Vec3;
use ut99_bsp::LightingBakeQuality;
use ut99_editor::editor_state::{EditorState, TournamentLightPreset};
use ut99_format::{ActorRecord, UnrLevel};

fn make_test_light(name: &str, loc: Vec3, brightness: u8, radius: u8, hue: u8, sat: u8) -> ActorRecord {
    let mut actor = ActorRecord::default();
    actor.name = name.to_string();
    actor.class_name = "Engine.Light".to_string();
    actor.location = loc;
    actor.light_brightness = brightness;
    actor.light_radius = radius;
    actor.light_hue = hue;
    actor.light_saturation = sat;
    actor
}

#[test]
fn test_full_scene_lighting_bake_with_undo_redo() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    let light1 = make_test_light("Light_Main", Vec3::new(0.0, 0.0, 100.0), 128, 32, 0, 255);
    let light2 = make_test_light("Light_Accent", Vec3::new(200.0, 0.0, 100.0), 100, 24, 160, 40);
    level.actors = vec![light1, light2].into();
    state.loaded_unr = Some(level);

    // Initial state: brush polys should have default / empty lightmaps
    let initial_lm_count: usize = state.world_brushes.iter()
        .flat_map(|b| &b.polys)
        .filter(|p| p.lightmap.is_some())
        .count();
    assert_eq!(initial_lm_count, 0);

    // Run Full Scene Bake
    state.lighting_config.quality = LightingBakeQuality::StandardTournament;
    state.lighting_config.enable_shadows = true;
    state.build_lighting();

    assert!(state.last_lighting_stats.is_some());
    let stats = state.last_lighting_stats.as_ref().unwrap();
    assert!(stats.total_luxels > 0);
    assert!(stats.total_polygons > 0);

    let baked_lm_count: usize = state.world_brushes.iter()
        .flat_map(|b| &b.polys)
        .filter(|p| p.lightmap.is_some())
        .count();
    assert!(baked_lm_count > 0, "Baked polygons must have computed lightmaps");

    // Test Undo: reverts all polygons to unbaked state
    assert!(state.history.can_undo());
    state.undo();
    let undone_lm_count: usize = state.world_brushes.iter()
        .flat_map(|b| &b.polys)
        .filter(|p| p.lightmap.is_some())
        .count();
    assert_eq!(undone_lm_count, 0, "Undo must revert lightmaps to unbaked state");

    // Test Redo: restores baked lighting
    assert!(state.history.can_redo());
    state.redo();
    let redone_lm_count: usize = state.world_brushes.iter()
        .flat_map(|b| &b.polys)
        .filter(|p| p.lightmap.is_some())
        .count();
    assert_eq!(redone_lm_count, baked_lm_count, "Redo must restore baked lightmaps");
}

#[test]
fn test_differential_lighting_bake_selected_lights_only() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Brush A at (0, 0, 0)
    let mut b_a = ut99_bsp::build_cube("Brush_A", ut99_bsp::CsgOperation::Add, Vec3::new(256.0, 256.0, 256.0));
    b_a.location = Vec3::ZERO;
    // Brush B at far distance (5000, 5000, 0)
    let mut b_b = ut99_bsp::build_cube("Brush_B", ut99_bsp::CsgOperation::Add, Vec3::new(256.0, 256.0, 256.0));
    b_b.location = Vec3::new(5000.0, 5000.0, 0.0);

    state.world_brushes = vec![b_a, b_b].into();

    let mut level = UnrLevel::default();
    // Light A close to Brush A
    let light_a = make_test_light("Light_Near_A", Vec3::new(0.0, 0.0, 100.0), 128, 16, 0, 255);
    // Light B close to Brush B
    let light_b = make_test_light("Light_Near_B", Vec3::new(5000.0, 5000.0, 100.0), 128, 16, 0, 255);
    level.actors = vec![light_a, light_b].into();
    state.loaded_unr = Some(level);

    // Select Light A ONLY
    state.selected_actor_index = Some(0);

    let res = state.bake_selected_lights_only();
    assert!(res.is_ok(), "Selective bake must succeed for selected light");

    // Brush A (index 0) must be baked
    let brush_a_baked = state.world_brushes[0].polys.iter().any(|p| p.lightmap.is_some());
    assert!(brush_a_baked, "Brush A must be baked differentially");

    // Brush B (index 1) was far out of Light A's radius, so it should NOT have been touched
    let brush_b_baked = state.world_brushes[1].polys.iter().any(|p| p.lightmap.is_some());
    assert!(!brush_b_baked, "Brush B far away must NOT be recomputed in differential bake");

    // Test Undo of differential bake
    assert!(state.history.can_undo());
    state.undo();
    assert!(!state.world_brushes[0].polys.iter().any(|p| p.lightmap.is_some()));
}

#[test]
fn test_tournament_light_presets_batch_application() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    let l1 = make_test_light("Light_1", Vec3::new(0.0, 0.0, 0.0), 64, 32, 0, 255);
    let l2 = make_test_light("Light_2", Vec3::new(100.0, 0.0, 0.0), 64, 32, 0, 255);
    level.actors = vec![l1, l2].into();
    state.loaded_unr = Some(level);

    // 1. Apply FacingWorldsMidnight (Deep Azure)
    let modified = state.apply_light_preset_to_selected(TournamentLightPreset::FacingWorldsMidnight);
    assert_eq!(modified, 2);

    let unr = state.loaded_unr.as_ref().unwrap();
    let (exp_h, exp_s, exp_b, exp_r) = TournamentLightPreset::FacingWorldsMidnight.values();
    assert_eq!(unr.actors[0].light_hue, exp_h);
    assert_eq!(unr.actors[0].light_saturation, exp_s);
    assert_eq!(unr.actors[0].light_brightness, exp_b);
    assert_eq!(unr.actors[0].light_radius, exp_r);

    // 2. Select only light 1 and apply Deck16ToxicSlime
    state.selected_actor_index = Some(0);
    let modified_single = state.apply_light_preset_to_selected(TournamentLightPreset::Deck16ToxicSlime);
    assert_eq!(modified_single, 1);

    let unr2 = state.loaded_unr.as_ref().unwrap();
    let (slime_h, slime_s, slime_b, slime_r) = TournamentLightPreset::Deck16ToxicSlime.values();
    assert_eq!(unr2.actors[0].light_hue, slime_h);
    assert_eq!(unr2.actors[0].light_saturation, slime_s);
    assert_eq!(unr2.actors[0].light_brightness, slime_b);
    assert_eq!(unr2.actors[0].light_radius, slime_r);

    // Light 2 must remain Midnight
    assert_eq!(unr2.actors[1].light_hue, exp_h);
}

#[test]
fn test_luxmeter_coverage_statistics_and_ratings() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    let light = make_test_light("TournamentLight", Vec3::new(0.0, 0.0, 150.0), 200, 48, 0, 255);
    level.actors = vec![light].into();
    state.loaded_unr = Some(level);

    // Bake lighting to produce luxels
    state.build_lighting();

    let stats = state.compute_luxmeter_stats();
    assert!(stats.total_surfaces > 0);
    assert!(stats.lit_surfaces > 0);
    assert!(stats.total_luxels > 0);
    assert!(stats.average_lux >= 0.0);

    // Verify all samples add up correctly
    let sample_sum = stats.under_lit_count
        + stats.shadow_count
        + stats.tournament_ideal_count
        + stats.bright_count
        + stats.overbright_count;
    assert!(sample_sum > 0);
}
