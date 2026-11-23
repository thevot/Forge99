// crates/ut99-editor/tests/test_elevator_studio.rs
//
// Integration tests for Interactive Elevator, LiftCenter/LiftExit & Vertical Transit Studio.

use glam::Vec3;
use ut99_bsp::{ElevatorArchetype, EncroachBehavior};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_elevator_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is StandardHydraulicLift
    assert_eq!(
        state.elevator_settings.active_config.archetype,
        ElevatorArchetype::StandardHydraulicLift
    );
    assert_eq!(state.elevator_settings.active_config.travel_height_uu, 384.0);
    assert_eq!(state.elevator_settings.active_config.move_time_secs, 2.5);
    assert_eq!(state.elevator_settings.active_config.tag, "DeckLift0");

    // Apply DoubleDeckerExpress
    state.apply_elevator_archetype(ElevatorArchetype::DoubleDeckerExpress);
    assert_eq!(
        state.elevator_settings.active_config.archetype,
        ElevatorArchetype::DoubleDeckerExpress
    );
    assert_eq!(state.elevator_settings.active_config.travel_height_uu, 640.0);
    assert_eq!(state.elevator_settings.active_config.tag, "ExpressLift");

    // Apply SubterraneanCargoDrop
    state.apply_elevator_archetype(ElevatorArchetype::SubterraneanCargoDrop);
    assert_eq!(
        state.elevator_settings.active_config.archetype,
        ElevatorArchetype::SubterraneanCargoDrop
    );
    assert_eq!(state.elevator_settings.active_config.encroach_damage, 150.0);
    assert_eq!(state.elevator_settings.active_config.encroach_behavior, EncroachBehavior::CrushWhenEncroached);

    // Undo back to DoubleDeckerExpress
    state.undo();
    assert_eq!(
        state.elevator_settings.active_config.archetype,
        ElevatorArchetype::DoubleDeckerExpress
    );

    // Redo to SubterraneanCargoDrop
    state.redo();
    assert_eq!(
        state.elevator_settings.active_config.archetype,
        ElevatorArchetype::SubterraneanCargoDrop
    );
}

#[test]
fn test_elevator_metrics_and_waypoint_resolution() {
    let mut state = EditorState::default();

    state.apply_elevator_archetype(ElevatorArchetype::StandardHydraulicLift);
    state.elevator_settings.active_config.tag = "LiftA".to_string();

    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        // LiftCenter
        let mut center = ut99_format::ActorRecord::default();
        center.name = "LiftCenter_0".to_string();
        center.class_name = "Botpack.LiftCenter".to_string();
        center.tag = "LiftA".to_string();
        center.location = Vec3::new(0.0, 0.0, 36.0);
        level.actors.push(center);

        // LiftExit Bottom
        let mut exit0 = ut99_format::ActorRecord::default();
        exit0.name = "LiftExit_Bottom".to_string();
        exit0.class_name = "Botpack.LiftExit".to_string();
        exit0.tag = "LiftA".to_string();
        exit0.location = Vec3::new(120.0, 0.0, 16.0);
        level.actors.push(exit0);

        // LiftExit Top
        let mut exit1 = ut99_format::ActorRecord::default();
        exit1.name = "LiftExit_Top".to_string();
        exit1.class_name = "Botpack.LiftExit".to_string();
        exit1.tag = "LiftA".to_string();
        exit1.location = Vec3::new(120.0, 0.0, 400.0);
        level.actors.push(exit1);
    }

    state.recalculate_elevator_preview();

    let metrics = &state.elevator_settings.cached_metrics;
    assert!(metrics.has_lift_center);
    assert_eq!(metrics.total_lift_exits_wired, 2);
    assert_eq!(metrics.ascent_velocity_uu_per_sec, 384.0 / 2.5);
    assert_eq!(metrics.platform_area_sq_uu, 160.0 * 160.0);
}

#[test]
fn test_elevator_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Missing LiftCenter and Missing LiftExits
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();
        let mut dummy = ut99_format::ActorRecord::default();
        dummy.name = "UnrelatedActor".to_string();
        dummy.class_name = "Engine.Light".to_string();
        level.actors.push(dummy);
    }

    state.elevator_settings.active_config.tag = "DeckLift0".to_string();
    state.recalculate_elevator_preview();

    assert!(state.elevator_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Missing LiftCenter")));
    assert!(state.elevator_settings.cached_hazards.iter().any(|h| h.title.contains("Incomplete LiftExit Landings")));

    // 2. Dangerous Transit Velocity & Lethal Crush
    state.elevator_settings.active_config.move_time_secs = 0.25; // 384 / 0.25 = 1536 uu/s
    state.elevator_settings.active_config.encroach_damage = 150.0;
    state.elevator_settings.active_config.encroach_behavior = EncroachBehavior::CrushWhenEncroached;
    state.recalculate_elevator_preview();

    assert!(state.elevator_settings.cached_hazards.iter().any(|h| h.title.contains("Dangerous Transit Velocity")));
    assert!(state.elevator_settings.cached_hazards.iter().any(|h| h.title.contains("Instant Lethal Crush Hazard")));
}

#[test]
fn test_commit_elevator_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Elevator
    state.apply_elevator_archetype(ElevatorArchetype::StandardHydraulicLift);
    let (brush_idx, center_idx, exit_indices) = state.commit_elevator_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(center_idx.is_some());
    assert_eq!(exit_indices.len(), 2);

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 3);
    assert_eq!(level.actors[center_idx.unwrap()].class_name, "Botpack.LiftCenter");
    assert_eq!(level.actors[exit_indices[0]].class_name, "Botpack.LiftExit");
    assert_eq!(level.actors[exit_indices[1]].class_name, "Botpack.LiftExit");

    // 2. Undo Commit
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    let level_undone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_undone.actors.len(), 0);

    // 3. Redo Commit
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    let level_redone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_redone.actors.len(), 3);
    assert_eq!(level_redone.actors[0].class_name, "Botpack.LiftCenter");
    assert_eq!(level_redone.actors[1].class_name, "Botpack.LiftExit");
    assert_eq!(level_redone.actors[2].class_name, "Botpack.LiftExit");
}
