// crates/ut99-editor/tests/test_light_shaft_studio.rs
//
// Integration tests for Interactive Volumetric Light Shaft, Window God Ray & Atmospheric Beam Studio.

use ut99_bsp::{
    calculate_optical_transmission, LightShaftArchetype, LightShaftCrossSection,
    PF_NOT_SOLID, PF_TRANSLUCENT, PF_TWO_SIDED, PF_UNLIT,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_light_shaft_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is CathedralStainedGlassBeam
    assert_eq!(
        state.light_shaft_settings.active_config.archetype,
        LightShaftArchetype::CathedralStainedGlassBeam
    );
    assert_eq!(
        state.light_shaft_settings.active_config.cross_section,
        LightShaftCrossSection::Pyramid4
    );

    // Apply IndustrialGratingSlats
    state.apply_light_shaft_archetype(LightShaftArchetype::IndustrialGratingSlats);
    assert_eq!(
        state.light_shaft_settings.active_config.archetype,
        LightShaftArchetype::IndustrialGratingSlats
    );
    assert_eq!(
        state.light_shaft_settings.active_config.cross_section,
        LightShaftCrossSection::ParallelSlats
    );

    // Apply SkylightConicalShaft
    state.apply_light_shaft_archetype(LightShaftArchetype::SkylightConicalShaft);
    assert_eq!(
        state.light_shaft_settings.active_config.archetype,
        LightShaftArchetype::SkylightConicalShaft
    );

    // Apply CaveFissureSunbeam
    state.apply_light_shaft_archetype(LightShaftArchetype::CaveFissureSunbeam);
    assert_eq!(
        state.light_shaft_settings.active_config.archetype,
        LightShaftArchetype::CaveFissureSunbeam
    );

    // Apply SearchlightVolumetricBeam
    state.apply_light_shaft_archetype(LightShaftArchetype::SearchlightVolumetricBeam);
    assert_eq!(
        state.light_shaft_settings.active_config.archetype,
        LightShaftArchetype::SearchlightVolumetricBeam
    );

    // Undo returns to CaveFissureSunbeam
    state.undo();
    assert_eq!(
        state.light_shaft_settings.active_config.archetype,
        LightShaftArchetype::CaveFissureSunbeam
    );

    // Redo returns to SearchlightVolumetricBeam
    state.redo();
    assert_eq!(
        state.light_shaft_settings.active_config.archetype,
        LightShaftArchetype::SearchlightVolumetricBeam
    );
}

#[test]
fn test_optical_transmission_and_geometry_cross_sections() {
    let mut state = EditorState::default();

    // 1. Pyramid4 with floor disc and no core hotspot
    state.light_shaft_settings.active_config.cross_section = LightShaftCrossSection::Pyramid4;
    state.light_shaft_settings.active_config.has_core_hotspot = false;
    state.light_shaft_settings.active_config.has_floor_light_disc = true;
    state.recalculate_light_shaft_geometry();

    let brush = state.light_shaft_settings.cached_preview_brush.as_ref().unwrap();
    assert_eq!(brush.polys.len(), 5, "4 lateral faces + 1 floor disc = 5 polys");

    let required_flags = PF_TRANSLUCENT | PF_UNLIT | PF_TWO_SIDED | PF_NOT_SOLID;
    for poly in &brush.polys {
        assert_eq!(poly.flags & required_flags, required_flags);
    }

    // 2. ParallelSlats
    state.light_shaft_settings.active_config.cross_section = LightShaftCrossSection::ParallelSlats;
    state.light_shaft_settings.active_config.slat_count = 8;
    state.recalculate_light_shaft_geometry();

    let slats_brush = state.light_shaft_settings.cached_preview_brush.as_ref().unwrap();
    assert_eq!(slats_brush.polys.len(), 8);

    // 3. Optical Transmission Decay
    let cfg = &state.light_shaft_settings.active_config;
    let t0 = calculate_optical_transmission(cfg, 0.0);
    let t1 = calculate_optical_transmission(cfg, 0.5);
    let t2 = calculate_optical_transmission(cfg, 1.0);
    assert!(t0 > t1, "Intensity must decrease over distance");
    assert!(t1 > t2, "Intensity must continue decreasing to ground");
}

#[test]
fn test_commit_light_shaft_to_builder_brush_and_world_with_undo_redo() {
    let mut state = EditorState::default();

    state.apply_light_shaft_archetype(LightShaftArchetype::CathedralStainedGlassBeam);
    state.light_shaft_settings.active_config.beam_length = 640.0;
    state.recalculate_light_shaft_geometry();

    let expected_polys = state.light_shaft_settings.cached_preview_brush.as_ref().unwrap().polys.len();
    assert!(expected_polys > 0);

    // 1. Commit to Builder Brush
    let old_builder = state.builder_brush.clone();
    state.commit_light_shaft_to_builder_brush();
    assert_eq!(state.builder_brush.polys.len(), expected_polys);
    assert!(state.map_is_dirty);

    // Undo restores previous builder brush
    state.undo();
    assert_eq!(state.builder_brush.polys.len(), old_builder.polys.len());

    // Redo restores light shaft builder brush
    state.redo();
    assert_eq!(state.builder_brush.polys.len(), expected_polys);

    // 2. Commit to World
    let initial_world_count = state.world_brushes.len();
    let brush_idx = state.commit_light_shaft_to_world();
    assert_eq!(brush_idx, initial_world_count);
    assert_eq!(state.world_brushes.len(), initial_world_count + 1);
    assert_eq!(state.world_brushes[brush_idx].polys.len(), expected_polys);

    // Undo removes world brush
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_count);

    // Redo re-inserts world brush
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_count + 1);
}

#[test]
fn test_light_shaft_safety_audit_and_dust_motes() {
    let mut state = EditorState::default();

    // Test extreme length warning
    state.light_shaft_settings.active_config.beam_length = 5000.0;
    state.recalculate_light_shaft_geometry();
    let has_warn = state.light_shaft_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Excessive"));
    assert!(has_warn, "Must warn for beam lengths > 4096 uu");

    // Test degenerate length critical hazard
    state.light_shaft_settings.active_config.beam_length = 32.0;
    state.recalculate_light_shaft_geometry();
    let has_crit = state.light_shaft_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Degenerate"));
    assert!(has_crit, "Must flag critical hazard for degenerate beam length");

    // Test ambient dust motes
    state.apply_light_shaft_archetype(LightShaftArchetype::CaveFissureSunbeam);
    state.recalculate_light_shaft_geometry();
    let motes = &state.light_shaft_settings.cached_dust_motes;
    assert_eq!(motes.len(), 35);
    for mote in motes {
        assert!(mote.brightness >= 0.0 && mote.brightness <= 1.0);
        assert!(mote.size > 0.0);
    }
}
