use glam::Vec3;
use ut99_bsp::LiftBotSimulationState;
use ut99_editor::editor_state::{EditorMover, EditorState, MoverKeyframe as StateMoverKeyframe};

#[test]
fn test_lift_pathing_audit_and_1click_sync() {
    let mut state = EditorState::default();

    // 1. Create a mover elevator with 3 keyframe floors
    let brush = ut99_bsp::build_cube("MoverPlatform", ut99_bsp::CsgOperation::Active, Vec3::new(256.0, 256.0, 32.0));
    let mut mover = EditorMover::new(brush);
    mover.name = "ElevatorMover1".to_string();
    mover.tag = "Elevator1".to_string();
    mover.move_time = 3.0;
    mover.stay_open_time = 2.0;

    mover.keyframes[0] = Some(StateMoverKeyframe {
        location: Vec3::new(100.0, 200.0, 0.0),
        rotation: Vec3::ZERO,
    });
    mover.keyframes[1] = Some(StateMoverKeyframe {
        location: Vec3::new(100.0, 200.0, 500.0),
        rotation: Vec3::ZERO,
    });
    mover.keyframes[2] = Some(StateMoverKeyframe {
        location: Vec3::new(100.0, 200.0, 1000.0),
        rotation: Vec3::ZERO,
    });
    state.movers.push(mover);

    // Initial audit: should find 1 mover, but no LiftCenter or LiftExits
    let init_audit = state.collect_elevator_audit();
    assert_eq!(init_audit.total_movers, 1);
    assert_eq!(init_audit.total_lift_centers, 0);
    assert_eq!(init_audit.total_lift_exits, 0);
    assert!(!init_audit.systems[0].is_fully_synchronized);

    // 2. Perform 1-click synchronized navigation setup
    let success = state.create_synchronized_lift_setup_for_mover(0);
    assert!(success);

    // Check newly added actors
    let unr = state.loaded_unr.as_ref().unwrap();
    let lift_centers: Vec<_> = unr.actors.iter().filter(|a| a.class_name == "LiftCenter").collect();
    let lift_exits: Vec<_> = unr.actors.iter().filter(|a| a.class_name == "LiftExit").collect();
    let triggers: Vec<_> = unr.actors.iter().filter(|a| a.class_name == "Trigger").collect();

    assert_eq!(lift_centers.len(), 1, "Must generate exactly 1 LiftCenter on the platform");
    assert_eq!(lift_exits.len(), 3, "Must generate 1 LiftExit for each of the 3 keyframe floors");
    assert_eq!(triggers.len(), 3, "Must generate matching triggers for each floor");

    // Check LiftCenter positioning and LiftTag
    let center = lift_centers[0];
    assert_eq!(center.tag, "Elevator1");
    assert_eq!(center.location.z, 20.0, "LiftCenter must be on deck (+20 uu)");

    // Check LiftExit keyframe assignments
    for (i, exit) in lift_exits.iter().enumerate() {
        assert_eq!(exit.tag, "Elevator1");
        let key_prop = exit.properties.get("SuggestedKeyFrame").unwrap();
        if let ut99_format::TaggedValue::Byte(b) = key_prop.value {
            assert_eq!(b as usize, i);
        }
    }

    // 3. Re-audit should report 100% synchronization
    let final_audit = state.collect_elevator_audit();
    assert_eq!(final_audit.synchronized_systems_count, 1);
    assert_eq!(final_audit.critical_hazard_count, 0);
    assert!(final_audit.systems[0].is_fully_synchronized);
}

#[test]
fn test_lift_pathing_undo_redo_invariance() {
    let mut state = EditorState::default();

    let brush = ut99_bsp::build_cube("LiftPlat", ut99_bsp::CsgOperation::Active, Vec3::new(200.0, 200.0, 32.0));
    let mut mover = EditorMover::new(brush);
    mover.name = "LiftMoverUndo".to_string();
    mover.tag = "LiftUndoTag".to_string();
    state.movers.push(mover);

    // Generate lift navigation set
    state.create_synchronized_lift_setup_for_mover(0);
    let actors_count_after_setup = state.loaded_unr.as_ref().unwrap().actors.len();
    assert!(actors_count_after_setup >= 4); // 1 center + 2 exits + 2 triggers

    // 1. Undo
    state.undo();
    let actors_count_after_undo = state.loaded_unr.as_ref().unwrap().actors.len();
    assert_eq!(actors_count_after_undo, 0, "Undo must cleanly remove all generated actors");

    // 2. Redo
    state.redo();
    let actors_count_after_redo = state.loaded_unr.as_ref().unwrap().actors.len();
    assert_eq!(actors_count_after_redo, actors_count_after_setup, "Redo must restore all generated actors");

    // Audit remains valid after redo
    let audit = state.collect_elevator_audit();
    assert!(audit.systems[0].is_fully_synchronized);
}

#[test]
fn test_lift_pathing_autofix_mismatched_tags() {
    let mut state = EditorState::default();

    let brush = ut99_bsp::build_cube("MoverFix", ut99_bsp::CsgOperation::Active, Vec3::new(200.0, 200.0, 32.0));
    let mut mover = EditorMover::new(brush);
    mover.name = "MoverFix0".to_string();
    mover.tag = "TargetLiftTag".to_string();
    state.movers.push(mover);

    // Place a LiftCenter and LiftExit with broken / mismatched tags
    if state.loaded_unr.is_none() {
        state.loaded_unr = Some(ut99_format::UnrLevel::default());
    }
    let unr = state.loaded_unr.as_mut().unwrap();

    let mut center = ut99_format::ActorRecord::default();
    center.class_name = "LiftCenter".to_string();
    center.tag = "WrongTag".to_string();
    center.properties.insert(
        "LiftTag".to_string(),
        ut99_format::unr::tagged_property::TaggedProperty {
            name: "LiftTag".to_string(),
            array_index: 0,
            value: ut99_format::unr::tagged_property::TaggedValue::Str("WrongTag".to_string()),
        },
    );
    unr.actors.push(center);

    let mut exit = ut99_format::ActorRecord::default();
    exit.class_name = "LiftExit".to_string();
    exit.tag = "AnotherWrongTag".to_string();
    exit.properties.insert(
        "LiftTag".to_string(),
        ut99_format::unr::tagged_property::TaggedProperty {
            name: "LiftTag".to_string(),
            array_index: 0,
            value: ut99_format::unr::tagged_property::TaggedValue::Str("AnotherWrongTag".to_string()),
        },
    );
    unr.actors.push(exit);

    // Auto-fix
    let fixes = state.auto_fix_elevator_issues("TargetLiftTag");
    assert_eq!(fixes, 2, "Must repair both mismatched actors");

    // Verify properties updated
    let unr_fixed = state.loaded_unr.as_ref().unwrap();
    for actor in &unr_fixed.actors {
        assert_eq!(actor.tag, "TargetLiftTag");
        if let Some(prop) = actor.properties.get("LiftTag") {
            if let ut99_format::TaggedValue::Str(ref s) = prop.value {
                assert_eq!(s, "TargetLiftTag");
            }
        }
    }
}

#[test]
fn test_lift_simulation_step_timeline() {
    let mut state = EditorState::default();

    let brush = ut99_bsp::build_cube("SimPlat", ut99_bsp::CsgOperation::Active, Vec3::new(200.0, 200.0, 32.0));
    let mut mover = EditorMover::new(brush);
    mover.name = "SimElevator".to_string();
    mover.tag = "SimTag".to_string();
    mover.move_time = 2.0;
    mover.stay_open_time = 1.0;
    state.movers.push(mover);

    state.create_synchronized_lift_setup_for_mover(0);
    state.collect_elevator_audit();

    // Check cached simulation steps
    let steps = &state.lift_pathing_studio_settings.cached_simulation;
    assert!(!steps.is_empty(), "Simulation must generate time-sampled flight steps");

    let states: Vec<_> = steps.iter().map(|s| s.bot_state).collect();
    assert!(states.contains(&LiftBotSimulationState::BoardingPlatform));
    assert!(states.contains(&LiftBotSimulationState::Ascending));
    assert!(states.contains(&LiftBotSimulationState::ExitingPlatform));

    // Test playback step advancement
    state.lift_pathing_studio_settings.playback_time = 0.0;
    state.lift_pathing_studio_settings.is_playing = true;
    state.lift_pathing_studio_settings.playback_speed = 1.0;

    state.step_elevator_simulation(0.5);
    assert_eq!(state.lift_pathing_studio_settings.playback_time, 0.5);

    // Test loop wraparound
    let total_cycle = state.lift_pathing_studio_settings.cached_audit.as_ref().unwrap().systems[0].total_cycle_time;
    state.lift_pathing_studio_settings.playback_time = total_cycle - 0.1;
    state.step_elevator_simulation(0.5);
    assert!(state.lift_pathing_studio_settings.playback_time < total_cycle, "Must loop back to start");
}
