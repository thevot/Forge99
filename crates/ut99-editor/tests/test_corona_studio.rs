use glam::Vec3;
use ut99_bsp::{
    calculate_corona_occlusion, project_lens_flare, Brush, CoronaPreset, CsgOperation, FlareShape,
    Polygon,
};
use ut99_editor::editor_state::EditorState;
use ut99_format::TaggedValue;

#[test]
fn test_corona_presets_and_flare_generation() {
    let presets = CoronaPreset::all();
    assert_eq!(presets.len(), 6, "Must provide 6 distinct corona and lens flare presets");

    for &preset in presets {
        assert!(!preset.display_name().is_empty());
        assert!(!preset.description().is_empty());

        let config = preset.to_config();
        assert!(config.is_enabled);
        assert!(!config.texture_name.is_empty());
        assert!(config.draw_scale >= 0.5 && config.draw_scale <= 5.0);
        assert!(config.intensity >= 0.5 && config.intensity <= 3.0);

        let elements = preset.generate_flare_chain();
        assert!(elements.len() >= 3, "Must generate at least 3 lens flare elements in chain");

        // Verify shape variety in the chain
        let has_ring_or_streak = elements.iter().any(|e| {
            matches!(e.shape, FlareShape::RingHalo | FlareShape::AnamorphicStreak | FlareShape::HexagonGhost)
        });
        assert!(has_ring_or_streak, "Flare chain must feature chromatic or geometric elements");
    }
}

#[test]
fn test_project_lens_flare_geometry() {
    let cam_pos = Vec3::new(0.0, 0.0, 0.0);
    let cam_forward = Vec3::new(0.0, 1.0, 0.0);
    let cam_right = Vec3::new(1.0, 0.0, 0.0);
    let cam_up = Vec3::new(0.0, 0.0, 1.0);
    let fov_deg = 90.0_f32;
    let aspect_ratio = 16.0 / 9.0;
    let preset = CoronaPreset::BlazingDesertSun;

    // 1. Source directly ahead along camera forward
    let source_ahead = Vec3::new(0.0, 500.0, 0.0);
    let proj_ahead = project_lens_flare(
        source_ahead,
        cam_pos,
        cam_forward,
        cam_right,
        cam_up,
        fov_deg,
        aspect_ratio,
        preset,
    );
    assert!(proj_ahead.is_some(), "Light directly in front of camera must project");
    let proj = proj_ahead.unwrap();
    assert!(proj.is_in_front);
    let (ndc_x, ndc_y) = proj.screen_pos_ndc;
    assert!(ndc_x.abs() < 1e-3, "Directly centered light must have ndc_x = 0");
    assert!(ndc_y.abs() < 1e-3, "Directly centered light must have ndc_y = 0");
    assert!(proj.angular_fade > 0.9, "Centered light must have near-maximum angular fade");
    assert!(!proj.elements.is_empty(), "Projected flare must contain flare chain elements");

    // 2. Source behind camera
    let source_behind = Vec3::new(0.0, -200.0, 0.0);
    let proj_behind = project_lens_flare(
        source_behind,
        cam_pos,
        cam_forward,
        cam_right,
        cam_up,
        fov_deg,
        aspect_ratio,
        preset,
    );
    assert!(proj_behind.is_none(), "Light behind camera must be clipped out");

    // 3. Source angled off to the right
    let source_right = Vec3::new(200.0, 500.0, 0.0);
    let proj_right = project_lens_flare(
        source_right,
        cam_pos,
        cam_forward,
        cam_right,
        cam_up,
        fov_deg,
        aspect_ratio,
        preset,
    );
    assert!(proj_right.is_some());
    let (x_right, _) = proj_right.unwrap().screen_pos_ndc;
    assert!(x_right > 0.0, "Source to the right must produce ndc_x > 0");
}

#[test]
fn test_corona_occlusion_raycast() {
    let cam_pos = Vec3::new(0.0, 0.0, 0.0);
    let light_pos = Vec3::new(0.0, 1000.0, 0.0);

    // 1. Unobstructed line of sight
    let empty_brushes: Vec<Brush> = Vec::new();
    let vis_unobstructed = calculate_corona_occlusion(light_pos, cam_pos, &empty_brushes, 32.0);
    assert_eq!(vis_unobstructed, 1.0, "Empty world must have 100% corona visibility");

    // 2. Obstructed by a solid wall brush between camera and light (at Y=500)
    let poly = Polygon::new(vec![
        Vec3::new(-500.0, 500.0, -500.0),
        Vec3::new(500.0, 500.0, -500.0),
        Vec3::new(500.0, 500.0, 500.0),
        Vec3::new(-500.0, 500.0, 500.0),
    ]).expect("Valid polygon");

    let mut blocker = Brush::new("SolidWall", CsgOperation::Add);
    blocker.location = Vec3::ZERO;
    blocker.polys.push(poly);

    let vis_obstructed = calculate_corona_occlusion(light_pos, cam_pos, &[blocker], 32.0);
    assert_eq!(vis_obstructed, 0.0, "Solid wall must fully occlude corona");
}

#[test]
fn test_editor_corona_studio_placement_and_undo_redo() {
    let mut state = EditorState::default();
    state.builder_brush.location = Vec3::new(128.0, 256.0, 512.0);

    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    // 1. Place Corona Light at Pivot
    let light_idx = state.place_corona_light_at_pivot(CoronaPreset::CyberpunkSodiumStreetlamp);

    let unr = state.loaded_unr.as_ref().expect("Level must exist");
    assert_eq!(unr.actors.len(), initial_actors + 1);
    let placed = &unr.actors[light_idx];
    assert_eq!(placed.class_name, "Light");
    assert_eq!(placed.location, Vec3::new(128.0, 256.0, 512.0));

    let b_corona = placed.properties.get("bCorona").map(|p| &p.value);
    assert_eq!(b_corona, Some(&TaggedValue::Bool(true)));

    let corona_tex = placed.properties.get("Corona").map(|p| &p.value);
    assert_eq!(corona_tex, Some(&TaggedValue::Str("GenIn.LensFl1".to_string())));

    // Test Undo placement
    assert!(state.history.can_undo());
    state.undo();
    let unr_after_undo = state.loaded_unr.as_ref().expect("Level must exist");
    assert_eq!(unr_after_undo.actors.len(), initial_actors);

    // Test Redo placement
    assert!(state.history.can_redo());
    state.redo();
    let unr_after_redo = state.loaded_unr.as_ref().expect("Level must exist");
    assert_eq!(unr_after_redo.actors.len(), initial_actors + 1);

    // 2. Select light and apply Corona Preset (BlazingDesertSun)
    state.selected_actor_index = Some(light_idx);
    let modified = state.apply_corona_preset_to_selected(CoronaPreset::BlazingDesertSun);
    assert_eq!(modified, 1);

    let unr_mod = state.loaded_unr.as_ref().expect("Level must exist");
    let sun_tex = unr_mod.actors[light_idx].properties.get("Corona").map(|p| &p.value);
    assert_eq!(sun_tex, Some(&TaggedValue::Str("GenIn.SunCorona".to_string())));

    // Test Undo preset application
    assert!(state.history.can_undo());
    state.undo();
    let unr_reverted = state.loaded_unr.as_ref().expect("Level must exist");
    let reverted_tex = unr_reverted.actors[light_idx].properties.get("Corona").map(|p| &p.value);
    assert_eq!(reverted_tex, Some(&TaggedValue::Str("GenIn.LensFl1".to_string())));

    // Test Redo preset application
    assert!(state.history.can_redo());
    state.redo();
    let unr_redone = state.loaded_unr.as_ref().expect("Level must exist");
    let redone_tex = unr_redone.actors[light_idx].properties.get("Corona").map(|p| &p.value);
    assert_eq!(redone_tex, Some(&TaggedValue::Str("GenIn.SunCorona".to_string())));

    // 3. Update corona settings directly
    state.update_selected_light_corona(true, "GenFX.Corona.AlienPulse", 3.5);
    let unr_custom = state.loaded_unr.as_ref().expect("Level must exist");
    let custom_tex = unr_custom.actors[light_idx].properties.get("Corona").map(|p| &p.value);
    assert_eq!(custom_tex, Some(&TaggedValue::Str("GenFX.Corona.AlienPulse".to_string())));

    // Test Undo custom update
    assert!(state.history.can_undo());
    state.undo();
    let unr_revert_custom = state.loaded_unr.as_ref().expect("Level must exist");
    let prev_tex = unr_revert_custom.actors[light_idx].properties.get("Corona").map(|p| &p.value);
    assert_eq!(prev_tex, Some(&TaggedValue::Str("GenIn.SunCorona".to_string())));
}
