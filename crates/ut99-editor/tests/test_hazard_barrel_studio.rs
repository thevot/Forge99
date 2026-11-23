// crates/ut99-editor/tests/test_hazard_barrel_studio.rs
//
// Integration tests for Interactive Explosive Barrel, Toxic Drum & Hazard Pyrotechnics Studio.

use ut99_bsp::BarrelHazardType;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_hazard_barrel_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is VolatileNapalmFuelDrum
    assert_eq!(
        state.hazard_barrel_settings.active_config.archetype,
        BarrelHazardType::VolatileNapalmFuelDrum
    );
    assert_eq!(state.hazard_barrel_settings.active_config.blast_damage, 160.0);
    assert_eq!(state.hazard_barrel_settings.active_config.blast_radius_uu, 320.0);

    // Apply CorrosiveSludgeAcidCanister
    state.apply_hazard_barrel_archetype(BarrelHazardType::CorrosiveSludgeAcidCanister);
    assert_eq!(
        state.hazard_barrel_settings.active_config.archetype,
        BarrelHazardType::CorrosiveSludgeAcidCanister
    );
    assert!(state.hazard_barrel_settings.active_config.is_toxic_spill);
    assert_eq!(state.hazard_barrel_settings.active_config.toxic_dps, 15.0);

    // Apply ReinforcedIndustrialKeg
    state.apply_hazard_barrel_archetype(BarrelHazardType::ReinforcedIndustrialKeg);
    assert_eq!(
        state.hazard_barrel_settings.active_config.archetype,
        BarrelHazardType::ReinforcedIndustrialKeg
    );
    assert_eq!(state.hazard_barrel_settings.active_config.health, 100.0);
    assert_eq!(state.hazard_barrel_settings.active_config.blast_damage, 220.0);

    // Undo back to CorrosiveSludgeAcidCanister
    state.undo();
    assert_eq!(
        state.hazard_barrel_settings.active_config.archetype,
        BarrelHazardType::CorrosiveSludgeAcidCanister
    );

    // Redo to ReinforcedIndustrialKeg
    state.redo();
    assert_eq!(
        state.hazard_barrel_settings.active_config.archetype,
        BarrelHazardType::ReinforcedIndustrialKeg
    );
}

#[test]
fn test_hazard_barrel_metrics_and_chain_reactions() {
    let mut state = EditorState::default();

    state.apply_hazard_barrel_archetype(BarrelHazardType::VolatileNapalmFuelDrum);
    state.recalculate_hazard_barrel_preview();

    let metrics = &state.hazard_barrel_settings.cached_metrics;
    assert_eq!(metrics.lethal_kill_radius_uu, 120.0); // 320 * (1 - 100/160) = 120
    assert!(metrics.impulse_ejection_velocity > 1000.0);
}

#[test]
fn test_hazard_barrel_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Extreme Blast Radius
    state.hazard_barrel_settings.active_config.blast_radius_uu = 650.0;
    state.recalculate_hazard_barrel_preview();
    assert!(state.hazard_barrel_settings.cached_hazards.iter().any(|h| h.title.contains("Catastrophic Blast Radius")));

    // 2. Lethal Area Denial Contamination
    state.hazard_barrel_settings.active_config.is_toxic_spill = true;
    state.hazard_barrel_settings.active_config.toxic_dps = 30.0;
    state.hazard_barrel_settings.active_config.toxic_duration_secs = 12.0;
    state.recalculate_hazard_barrel_preview();
    assert!(state.hazard_barrel_settings.cached_hazards.iter().any(|h| h.title.contains("Lethal Area Denial Contamination")));
}

#[test]
fn test_commit_hazard_barrel_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Barrel to World
    state.apply_hazard_barrel_archetype(BarrelHazardType::VolatileNapalmFuelDrum);
    let (brush_idx, barrel_idx) = state.commit_hazard_barrel_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(barrel_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Botpack.UT_SteelBarrel");

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
    assert_eq!(level_redone.actors[0].class_name, "Botpack.UT_SteelBarrel");
}
