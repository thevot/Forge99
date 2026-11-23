// crates/ut99-editor/tests/test_sniper_vantage_studio.rs
//
// Integration tests for Interactive Sniper Vantage Point, Headshot Angle & Occlusion Nest Studio.

use ut99_bsp::SniperVantageArchetype;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_sniper_vantage_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is HighAltitudeTowerSpire
    assert_eq!(
        state.sniper_vantage_settings.active_config.archetype,
        SniperVantageArchetype::HighAltitudeTowerSpire
    );
    assert_eq!(state.sniper_vantage_settings.active_config.max_effective_range_uu, 4500.0);
    assert_eq!(state.sniper_vantage_settings.active_config.zoom_fov_deg, 15.0);

    // Apply ElevatedCorridorCatwalk
    state.apply_sniper_vantage_archetype(SniperVantageArchetype::ElevatedCorridorCatwalk);
    assert_eq!(
        state.sniper_vantage_settings.active_config.archetype,
        SniperVantageArchetype::ElevatedCorridorCatwalk
    );
    assert_eq!(state.sniper_vantage_settings.active_config.max_effective_range_uu, 2500.0);

    // Apply ShadowedRecessedAlcove
    state.apply_sniper_vantage_archetype(SniperVantageArchetype::ShadowedRecessedAlcove);
    assert_eq!(
        state.sniper_vantage_settings.active_config.archetype,
        SniperVantageArchetype::ShadowedRecessedAlcove
    );
    assert_eq!(state.sniper_vantage_settings.active_config.coverage_angle_deg, 45.0);

    // Undo back to ElevatedCorridorCatwalk
    state.undo();
    assert_eq!(
        state.sniper_vantage_settings.active_config.archetype,
        SniperVantageArchetype::ElevatedCorridorCatwalk
    );

    // Redo to ShadowedRecessedAlcove
    state.redo();
    assert_eq!(
        state.sniper_vantage_settings.active_config.archetype,
        SniperVantageArchetype::ShadowedRecessedAlcove
    );
}

#[test]
fn test_sniper_vantage_metrics_and_sightlines() {
    let mut state = EditorState::default();

    state.apply_sniper_vantage_archetype(SniperVantageArchetype::HighAltitudeTowerSpire);
    state.recalculate_sniper_vantage_preview();

    let metrics = &state.sniper_vantage_settings.cached_metrics;
    assert_eq!(metrics.max_sightline_uu, 4500.0);
    assert_eq!(metrics.headshot_clearance_uu, 20.0); // 64 - 44
    assert_eq!(metrics.cover_exposure_pct, 25.0); // 90 / 360
}

#[test]
fn test_sniper_vantage_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Obstructed Parapet Wall Height
    state.sniper_vantage_settings.active_config.parapet_height_uu = 66.0;
    state.recalculate_sniper_vantage_preview();
    assert!(state.sniper_vantage_settings.cached_hazards.iter().any(|h| h.title.contains("Obstructed Parapet Battlement")));

    // 2. Extreme Sightline Range
    state.sniper_vantage_settings.active_config.max_effective_range_uu = 5500.0;
    state.recalculate_sniper_vantage_preview();
    assert!(state.sniper_vantage_settings.cached_hazards.iter().any(|h| h.title.contains("Extreme Sightline Range")));
}

#[test]
fn test_commit_sniper_vantage_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Balcony & Sniper Weapon to World
    state.apply_sniper_vantage_archetype(SniperVantageArchetype::HighAltitudeTowerSpire);
    let (brush_idx, weapon_idx) = state.commit_sniper_vantage_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(weapon_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Botpack.SniperRifle");

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
    assert_eq!(level_redone.actors[0].class_name, "Botpack.SniperRifle");
}
