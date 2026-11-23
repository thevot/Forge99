// crates/ut99-editor/tests/test_trigger_network_studio.rs
//
// Integration tests for Interactive Pressure Plate & Proximity Trigger Network Studio.

use glam::Vec3;
use ut99_bsp::{TriggerActivationType, TriggerNetworkArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_trigger_network_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is PressureChamberDecompressorSwitch
    assert_eq!(
        state.trigger_network_settings.active_config.archetype,
        TriggerNetworkArchetype::PressureChamberDecompressorSwitch
    );
    assert_eq!(
        state.trigger_network_settings.active_config.trigger_type,
        TriggerActivationType::UseKeyInteractiveConsole
    );
    assert_eq!(state.trigger_network_settings.active_config.event_name, "DecompressChamber");

    // Apply TrapdoorHiddenPitTrigger
    state.apply_trigger_network_archetype(TriggerNetworkArchetype::TrapdoorHiddenPitTrigger);
    assert_eq!(
        state.trigger_network_settings.active_config.archetype,
        TriggerNetworkArchetype::TrapdoorHiddenPitTrigger
    );
    assert_eq!(
        state.trigger_network_settings.active_config.trigger_type,
        TriggerActivationType::DepressibleFloorPad
    );
    assert_eq!(state.trigger_network_settings.active_config.event_name, "OpenPitTrapdoor");

    // Apply IntruderAlarmTripwire
    state.apply_trigger_network_archetype(TriggerNetworkArchetype::IntruderAlarmTripwire);
    assert_eq!(
        state.trigger_network_settings.active_config.archetype,
        TriggerNetworkArchetype::IntruderAlarmTripwire
    );
    assert_eq!(
        state.trigger_network_settings.active_config.trigger_type,
        TriggerActivationType::BeamBreakInfraredTripwire
    );
    assert_eq!(state.trigger_network_settings.active_config.team_filter, 2);

    // Undo back to TrapdoorHiddenPitTrigger
    state.undo();
    assert_eq!(
        state.trigger_network_settings.active_config.archetype,
        TriggerNetworkArchetype::TrapdoorHiddenPitTrigger
    );

    // Redo to IntruderAlarmTripwire
    state.redo();
    assert_eq!(
        state.trigger_network_settings.active_config.archetype,
        TriggerNetworkArchetype::IntruderAlarmTripwire
    );
}

#[test]
fn test_trigger_network_metrics_and_wiring_resolution() {
    let mut state = EditorState::default();

    state.apply_trigger_network_archetype(TriggerNetworkArchetype::PressureChamberDecompressorSwitch);
    state.trigger_network_settings.active_config.position = Vec3::new(0.0, 0.0, 0.0);

    // Populate level with a recipient Mover actor matching "DecompressChamber"
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        let mut mover = ut99_format::ActorRecord::default();
        mover.name = "BlastDoorMover_0".to_string();
        mover.class_name = "Engine.Mover".to_string();
        mover.tag = "DecompressChamber".to_string();
        mover.location = Vec3::new(250.0, 0.0, 0.0);
        level.actors.push(mover);
    }

    state.recalculate_trigger_network_preview();

    let metrics = &state.trigger_network_settings.cached_metrics;
    assert_eq!(metrics.total_target_actors_wired, 1);
    assert_eq!(metrics.network_link_distance_uu, 250.0);
    assert!(metrics.trigger_volume_cu_uu > 10_000.0);
}

#[test]
fn test_trigger_network_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Test Orphan Event Detection
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        let mut dummy = ut99_format::ActorRecord::default();
        dummy.name = "UnrelatedLight".to_string();
        dummy.class_name = "Engine.Light".to_string();
        dummy.tag = "UnrelatedTag".to_string();
        level.actors.push(dummy);
    }

    state.trigger_network_settings.active_config.event_name = "NonExistentEvent".to_string();
    state.recalculate_trigger_network_preview();
    assert!(state.trigger_network_settings.cached_hazards.iter().any(|h| h.title.contains("Orphan Event Wire")));

    // 2. Test Feedback Loop Detection
    {
        let level = state.loaded_unr.as_mut().unwrap();
        level.actors.clear();

        let mut looping_mover = ut99_format::ActorRecord::default();
        looping_mover.name = "LoopingDoor".to_string();
        looping_mover.class_name = "Engine.Mover".to_string();
        looping_mover.tag = "CycleEvent".to_string();
        looping_mover.event = "CycleEvent".to_string(); // Loops onto itself!
        level.actors.push(looping_mover);
    }
    state.trigger_network_settings.active_config.event_name = "CycleEvent".to_string();
    state.recalculate_trigger_network_preview();
    assert!(state.trigger_network_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Trigger Feedback Loop")));
}

#[test]
fn test_commit_trigger_network_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Trigger Network
    state.apply_trigger_network_archetype(TriggerNetworkArchetype::PressureChamberDecompressorSwitch);
    let (brush_idx, actor_idx) = state.commit_trigger_network_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(actor_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    let act = &level.actors[actor_idx.unwrap()];
    assert_eq!(act.class_name, "Engine.Trigger");
    assert_eq!(act.event, "DecompressChamber");
    assert_eq!(act.group, "Logic");

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
    assert_eq!(level_redone.actors[0].class_name, "Engine.Trigger");
}
