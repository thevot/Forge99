// crates/ut99-editor/tests/test_assault_studio.rs
//
// Integration tests for Interactive Assault Objective Studio.

use glam::Vec3;
use ut99_bsp::{AssaultArchetype, AssaultObjectiveType};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_assault_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is OverlordBeachLanding
    assert_eq!(
        state.assault_settings.active_config.archetype,
        AssaultArchetype::OverlordBeachLanding
    );
    assert_eq!(
        state.assault_settings.active_config.objective_order,
        1
    );
    assert_eq!(
        state.assault_settings.active_config.objective_type,
        AssaultObjectiveType::DestroyableTarget
    );
    assert_eq!(
        state.assault_settings.active_config.target_health,
        600.0
    );

    // Apply HiSpeedTrainInfiltration
    state.apply_assault_archetype(AssaultArchetype::HiSpeedTrainInfiltration);
    assert_eq!(
        state.assault_settings.active_config.archetype,
        AssaultArchetype::HiSpeedTrainInfiltration
    );
    assert_eq!(
        state.assault_settings.active_config.objective_order,
        2
    );
    assert_eq!(
        state.assault_settings.active_config.objective_type,
        AssaultObjectiveType::TriggerSwitch
    );

    // Apply FrigateNavalAssault
    state.apply_assault_archetype(AssaultArchetype::FrigateNavalAssault);
    assert_eq!(
        state.assault_settings.active_config.archetype,
        AssaultArchetype::FrigateNavalAssault
    );

    // Apply RookCastleSiege
    state.apply_assault_archetype(AssaultArchetype::RookCastleSiege);
    assert_eq!(
        state.assault_settings.active_config.archetype,
        AssaultArchetype::RookCastleSiege
    );

    // Apply MazonPowerExtraction
    state.apply_assault_archetype(AssaultArchetype::MazonPowerExtraction);
    assert_eq!(
        state.assault_settings.active_config.archetype,
        AssaultArchetype::MazonPowerExtraction
    );

    // Undo back to Rook
    state.undo();
    assert_eq!(
        state.assault_settings.active_config.archetype,
        AssaultArchetype::RookCastleSiege
    );

    // Redo to Mazon
    state.redo();
    assert_eq!(
        state.assault_settings.active_config.archetype,
        AssaultArchetype::MazonPowerExtraction
    );
}

#[test]
fn test_assault_metrics_and_chain_pacing() {
    let mut state = EditorState::default();

    // Populate level with an existing sequential objective
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        let mut obj = ut99_format::ActorRecord::default();
        obj.name = "AssaultObj_2".to_string();
        obj.class_name = "Botpack.FortStandard".to_string();
        obj.location = Vec3::new(750.0, 0.0, 0.0);
        obj.event = "Door2_Open".to_string();
        level.actors.push(obj);
    }

    state.recalculate_assault_preview();
    let metrics = &state.assault_settings.cached_metrics;

    assert_eq!(metrics.total_objectives_in_chain, 2);
    assert_eq!(metrics.distance_to_next_objective_uu, 750.0);
    assert_eq!(metrics.attacker_sprint_advance_time_secs, 2.0); // 800 / 400 = 2.0s
    assert!(metrics.defense_difficulty_index >= 30.0);
}

#[test]
fn test_assault_hazard_auditing() {
    let mut state = EditorState::default();

    // Invulnerable target hazard
    state.assault_settings.active_config.target_health = 0.0;
    state.recalculate_assault_preview();
    assert!(state.assault_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Invulnerable")));

    // Missing completion event hazard
    state.assault_settings.active_config.event_on_complete = "".to_string();
    state.recalculate_assault_preview();
    assert!(state.assault_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Missing Completion Event")));
}

#[test]
fn test_commit_assault_objective_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Assault Objective
    state.apply_assault_archetype(AssaultArchetype::OverlordBeachLanding);
    let (brush_idx, actor_idx) = state.commit_assault_objective_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(actor_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    let act = &level.actors[actor_idx.unwrap()];
    assert_eq!(act.class_name, "Botpack.FortStandard");
    assert_eq!(act.group, "Assault");
    assert_eq!(act.tag, "Obj1_BunkerGun");
    assert_eq!(act.event, "Door1_Open");

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
    assert_eq!(level_redone.actors[0].class_name, "Botpack.FortStandard");
}
