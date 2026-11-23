// crates/ut99-editor/tests/test_skylight_studio.rs
//
// Integration tests for Interactive Glass Skylight, Observation Dome & Structural Truss Studio.

use ut99_bsp::SkylightArchetype;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_skylight_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is CurvedObservationDome
    assert_eq!(
        state.skylight_settings.active_config.archetype,
        SkylightArchetype::CurvedObservationDome
    );
    assert_eq!(state.skylight_settings.active_config.width_uu, 256.0);
    assert_eq!(state.skylight_settings.active_config.glass_transmittance_pct, 85.0);

    // Apply IndustrialSawtoothSkylight
    state.apply_skylight_archetype(SkylightArchetype::IndustrialSawtoothSkylight);
    assert_eq!(
        state.skylight_settings.active_config.archetype,
        SkylightArchetype::IndustrialSawtoothSkylight
    );
    assert_eq!(state.skylight_settings.active_config.length_uu, 384.0);
    assert!(!state.skylight_settings.active_config.is_bulletproof);

    // Apply VaultedAtriumBarrelVault
    state.apply_skylight_archetype(SkylightArchetype::VaultedAtriumBarrelVault);
    assert_eq!(
        state.skylight_settings.active_config.archetype,
        SkylightArchetype::VaultedAtriumBarrelVault
    );
    assert_eq!(state.skylight_settings.active_config.rib_count, 5);

    // Undo back to IndustrialSawtoothSkylight
    state.undo();
    assert_eq!(
        state.skylight_settings.active_config.archetype,
        SkylightArchetype::IndustrialSawtoothSkylight
    );

    // Redo to VaultedAtriumBarrelVault
    state.redo();
    assert_eq!(
        state.skylight_settings.active_config.archetype,
        SkylightArchetype::VaultedAtriumBarrelVault
    );
}

#[test]
fn test_skylight_metrics_and_solar_transmittance() {
    let mut state = EditorState::default();

    state.apply_skylight_archetype(SkylightArchetype::CurvedObservationDome);
    state.recalculate_skylight_preview();

    let metrics = &state.skylight_settings.cached_metrics;
    assert!(metrics.total_glazed_surface_area_sq_uu > 256.0 * 256.0);
    assert_eq!(metrics.structural_aspect_ratio, 1.0);
    assert_eq!(metrics.lux_solar_transmittance_rating, 0.85 * 220.0);
}

#[test]
fn test_skylight_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Excessive unreinforced span (> 320 uu with < 3 ribs)
    state.skylight_settings.active_config.width_uu = 400.0;
    state.skylight_settings.active_config.rib_count = 1;
    state.recalculate_skylight_preview();
    assert!(state.skylight_settings.cached_hazards.iter().any(|h| h.title.contains("Excessive Unreinforced Structural Span")));

    // 2. Opaque zero-transmittance
    state.skylight_settings.active_config.glass_transmittance_pct = 0.0;
    state.recalculate_skylight_preview();
    assert!(state.skylight_settings.cached_hazards.iter().any(|h| h.title.contains("Opaque Zero-Transmittance Glazing")));
}

#[test]
fn test_commit_skylight_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Skylight Brush & Sunlight Actor to World
    state.apply_skylight_archetype(SkylightArchetype::CurvedObservationDome);
    let (brush_idx, light_idx) = state.commit_skylight_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(light_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Engine.Light");

    // 2. Undo Commit
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    let level_undone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_undone.actors.len(), 0);

    // 3. Redo Commit
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    let level_redone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_redone.actors.len(), 1);
    assert_eq!(level_redone.actors[0].class_name, "Engine.Light");
}
