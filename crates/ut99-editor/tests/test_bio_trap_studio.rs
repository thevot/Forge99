// crates/ut99-editor/tests/test_bio_trap_studio.rs
//
// Integration tests for Interactive Bio-Rifle Slime Trap & Volumetric Hazard Field Studio.

use glam::Vec3;
use ut99_bsp::{BioDispersionMode, BioTrapArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_bio_trap_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is ChokepointDoorwayAmbushTrap
    assert_eq!(
        state.bio_trap_settings.active_config.archetype,
        BioTrapArchetype::ChokepointDoorwayAmbushTrap
    );
    assert_eq!(
        state.bio_trap_settings.active_config.dispersion_mode,
        BioDispersionMode::ConcentratedGlob
    );
    assert_eq!(state.bio_trap_settings.active_config.glob_charge_count, 10);

    // Apply FlagPedestalSurroundMinefield
    state.apply_bio_trap_archetype(BioTrapArchetype::FlagPedestalSurroundMinefield);
    assert_eq!(
        state.bio_trap_settings.active_config.archetype,
        BioTrapArchetype::FlagPedestalSurroundMinefield
    );
    assert_eq!(
        state.bio_trap_settings.active_config.dispersion_mode,
        BioDispersionMode::DispersedMinefield
    );

    // Apply ElevatorShaftToxicWeb
    state.apply_bio_trap_archetype(BioTrapArchetype::ElevatorShaftToxicWeb);
    assert_eq!(
        state.bio_trap_settings.active_config.archetype,
        BioTrapArchetype::ElevatorShaftToxicWeb
    );
    assert_eq!(
        state.bio_trap_settings.active_config.dispersion_mode,
        BioDispersionMode::WallAdhesiveWeb
    );
    assert_eq!(state.bio_trap_settings.active_config.surface_normal, Vec3::new(1.0, 0.0, 0.0));

    // Apply ReactorVatSpillZone
    state.apply_bio_trap_archetype(BioTrapArchetype::ReactorVatSpillZone);
    assert_eq!(
        state.bio_trap_settings.active_config.archetype,
        BioTrapArchetype::ReactorVatSpillZone
    );
    assert_eq!(
        state.bio_trap_settings.active_config.dispersion_mode,
        BioDispersionMode::CorrosivePool
    );
    assert_eq!(state.bio_trap_settings.active_config.continuous_dps, 45.0);

    // Undo back to ElevatorShaftToxicWeb
    state.undo();
    assert_eq!(
        state.bio_trap_settings.active_config.archetype,
        BioTrapArchetype::ElevatorShaftToxicWeb
    );

    // Redo to ReactorVatSpillZone
    state.redo();
    assert_eq!(
        state.bio_trap_settings.active_config.archetype,
        BioTrapArchetype::ReactorVatSpillZone
    );
}

#[test]
fn test_bio_trap_metrics_and_lethality() {
    let mut state = EditorState::default();

    state.apply_bio_trap_archetype(BioTrapArchetype::ChokepointDoorwayAmbushTrap);
    state.recalculate_bio_trap_preview();

    let metrics = &state.bio_trap_settings.cached_metrics;
    assert_eq!(metrics.total_burst_damage_hp, 220.0);
    assert!(metrics.time_to_lethal_kill_secs < 0.5);
    assert!(metrics.hazard_footprint_area_sq_uu > 100_000.0);
}

#[test]
fn test_bio_trap_safety_auditing() {
    let mut state = EditorState::default();

    state.bio_trap_settings.active_config.position = Vec3::new(0.0, 0.0, 0.0);
    state.bio_trap_settings.active_config.splash_radius_uu = 240.0;

    // Place a PlayerStart inside lethal toxic range (200 uu < 240 + 80 = 320 uu)
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        let mut p_start = ut99_format::ActorRecord::default();
        p_start.name = "PlayerStart_Red_0".to_string();
        p_start.class_name = "Engine.PlayerStart".to_string();
        p_start.location = Vec3::new(200.0, 0.0, 0.0);
        level.actors.push(p_start);
    }

    state.recalculate_bio_trap_preview();
    assert!(state.bio_trap_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Spawn Point Contamination")));
}

#[test]
fn test_commit_bio_trap_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Bio Trap
    state.apply_bio_trap_archetype(BioTrapArchetype::ChokepointDoorwayAmbushTrap);
    let (brush_idx, actor_idx) = state.commit_bio_trap_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(actor_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    let act = &level.actors[actor_idx.unwrap()];
    assert_eq!(act.class_name, "Botpack.BioGel");
    assert_eq!(act.group, "Hazard");
    assert_eq!(act.tag, "DoorAmbush_Slime");

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
    assert_eq!(level_redone.actors[0].class_name, "Botpack.BioGel");
}
