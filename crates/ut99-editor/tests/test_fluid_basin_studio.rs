// crates/ut99-editor/tests/test_fluid_basin_studio.rs
//
// Integration tests for Interactive Toxic Slime Acid Basin, Lava Flow & Caustic Pool Studio.

use ut99_bsp::FluidBasinArchetype;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_fluid_basin_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is CorrosiveDeckSlimeVat
    assert_eq!(
        state.fluid_basin_settings.active_config.archetype,
        FluidBasinArchetype::CorrosiveDeckSlimeVat
    );
    assert_eq!(state.fluid_basin_settings.active_config.dps, 40.0);
    assert!(state.fluid_basin_settings.active_config.has_escape_ladder_or_ramp);

    // Apply MoltenMagmaChasm
    state.apply_fluid_basin_archetype(FluidBasinArchetype::MoltenMagmaChasm);
    assert_eq!(
        state.fluid_basin_settings.active_config.archetype,
        FluidBasinArchetype::MoltenMagmaChasm
    );
    assert_eq!(state.fluid_basin_settings.active_config.dps, 80.0);
    assert_eq!(state.fluid_basin_settings.active_config.damage_type, "Burned");

    // Apply IndustrialCoolantReservoir
    state.apply_fluid_basin_archetype(FluidBasinArchetype::IndustrialCoolantReservoir);
    assert_eq!(
        state.fluid_basin_settings.active_config.archetype,
        FluidBasinArchetype::IndustrialCoolantReservoir
    );
    assert_eq!(state.fluid_basin_settings.active_config.dps, 0.0);

    // Undo back to MoltenMagmaChasm
    state.undo();
    assert_eq!(
        state.fluid_basin_settings.active_config.archetype,
        FluidBasinArchetype::MoltenMagmaChasm
    );

    // Redo to IndustrialCoolantReservoir
    state.redo();
    assert_eq!(
        state.fluid_basin_settings.active_config.archetype,
        FluidBasinArchetype::IndustrialCoolantReservoir
    );
}

#[test]
fn test_fluid_basin_metrics_and_survival() {
    let mut state = EditorState::default();

    state.apply_fluid_basin_archetype(FluidBasinArchetype::CorrosiveDeckSlimeVat);
    state.recalculate_fluid_basin_preview();

    let metrics = &state.fluid_basin_settings.cached_metrics;
    assert_eq!(metrics.player_survival_time_secs, 2.5); // 100 / 40 = 2.5s
    assert_eq!(metrics.fluid_volume_k_cu_uu, 256.0 * 384.0 * 64.0 / 1000.0);
}

#[test]
fn test_fluid_basin_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Inescapable Submerged Death Trap
    state.fluid_basin_settings.active_config.depth_uu = 64.0;
    state.fluid_basin_settings.active_config.has_escape_ladder_or_ramp = false;
    state.recalculate_fluid_basin_preview();
    assert!(state.fluid_basin_settings.cached_hazards.iter().any(|h| h.title.contains("Inescapable Submerged Death Trap")));

    // 2. Extreme Lethal DPS
    state.fluid_basin_settings.active_config.dps = 150.0;
    state.recalculate_fluid_basin_preview();
    assert!(state.fluid_basin_settings.cached_hazards.iter().any(|h| h.title.contains("Catastrophic Instant-Kill Damage")));
}

#[test]
fn test_commit_fluid_basin_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Basin Trough & Zone Actor to World
    state.apply_fluid_basin_archetype(FluidBasinArchetype::CorrosiveDeckSlimeVat);
    let (brush_idx, zone_idx) = state.commit_fluid_basin_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(zone_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Botpack.PainZone");

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
    assert_eq!(level_redone.actors[0].class_name, "Botpack.PainZone");
}
