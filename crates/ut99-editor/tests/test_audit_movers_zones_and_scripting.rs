use glam::Vec3;
use ut99_bsp::{
    build_cube, CsgOperation, MoverGlideType, NavGraph, R_JUMP, R_WALK,
};
use ut99_editor::editor_state::{EditorMover, EditorState, MoverKeyframe as EditorMoverKeyframe};
use ut99_format::{ActorRecord, TaggedProperty, TaggedValue, UnrLevel};

#[test]
fn test_zone_leak_detector_and_portal_watertightness() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Fully enclosed room brush around origin
    let room = build_cube("Room", CsgOperation::Add, Vec3::new(1000.0, 1000.0, 1000.0));
    state.world_brushes.push(room);

    let mut level = UnrLevel::default();

    // Contained water zone inside the room
    let mut water_zone = ActorRecord::default();
    water_zone.class_name = "Engine.WaterZone".to_string();
    water_zone.name = "WaterPool_Contained".to_string();
    water_zone.location = Vec3::new(0.0, 0.0, 0.0);
    level.actors.push(water_zone);

    // Uncontained lava zone far out in empty unsealed space
    let mut lava_zone = ActorRecord::default();
    lava_zone.class_name = "Engine.LavaZone".to_string();
    lava_zone.name = "LavaChamber_Leaking".to_string();
    lava_zone.location = Vec3::new(10000.0, 10000.0, 0.0);
    level.actors.push(lava_zone);

    state.loaded_unr = Some(level);

    let reports = state.analyze_zone_seals();
    assert_eq!(reports.len(), 2);

    // Contained zone check
    let r_contained = reports.iter().find(|r| r.zone_name == "WaterPool_Contained").unwrap();
    assert!(r_contained.is_watertight);
    assert!(r_contained.detected_leaks.is_empty());
    assert!(r_contained.recommended_action.contains("sealed"));

    // Leaking zone check
    let r_leaking = reports.iter().find(|r| r.zone_name == "LavaChamber_Leaking").unwrap();
    assert!(!r_leaking.is_watertight);
    assert!(!r_leaking.detected_leaks.is_empty());
    assert!(r_leaking.recommended_action.contains("CRITICAL"));
    assert!(r_leaking.detected_leaks[0].leak_distance > 0.0);
}

#[test]
fn test_mover_8keyframe_trajectories_and_pose_scrubbing() {
    let mut state = EditorState::default();
    state.movers.clear();

    // Create a 4-keyframe mover (Elevator / Sliding Door)
    let brush = build_cube("LiftBrush", CsgOperation::Active, Vec3::new(128.0, 128.0, 16.0));
    let mut mover = EditorMover::new(brush);
    mover.name = "FreightElevator".to_string();
    mover.tag = "LiftTrigger".to_string();
    mover.move_time = 3.0;
    mover.stay_open_time = 2.0;
    mover.glide_type = "MV_Smooth".to_string();

    let mut keyframes: [Option<EditorMoverKeyframe>; 8] = Default::default();
    keyframes[0] = Some(EditorMoverKeyframe { location: Vec3::new(0.0, 0.0, 0.0), rotation: Vec3::ZERO });
    keyframes[1] = Some(EditorMoverKeyframe { location: Vec3::new(0.0, 0.0, 128.0), rotation: Vec3::new(0.0, 45.0, 0.0) });
    keyframes[2] = Some(EditorMoverKeyframe { location: Vec3::new(100.0, 0.0, 256.0), rotation: Vec3::new(0.0, 90.0, 0.0) });
    keyframes[3] = Some(EditorMoverKeyframe { location: Vec3::new(200.0, 0.0, 384.0), rotation: Vec3::new(0.0, 180.0, 0.0) });
    mover.keyframes = keyframes;

    state.movers.push(mover);

    let trajectories = state.get_mover_trajectories();
    assert_eq!(trajectories.len(), 1);

    let traj = &trajectories[0];
    assert_eq!(traj.mover_name, "FreightElevator");
    assert_eq!(traj.mover_tag, "LiftTrigger");
    assert_eq!(traj.num_keys, 4);
    assert_eq!(traj.glide_type, MoverGlideType::Smooth);
    assert!(traj.total_path_length_uu > 384.0);
    assert!(!traj.spline_points.is_empty());

    // Test pose scrubbing at different times (0.0, 0.5, 1.0)
    let (p_start, r_start) = traj.evaluate_pose(0.0);
    assert_eq!(p_start, Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(r_start, Vec3::ZERO);

    let (p_end, r_end) = traj.evaluate_pose(1.0);
    assert_eq!(p_end, Vec3::new(200.0, 0.0, 384.0));
    assert_eq!(r_end, Vec3::new(0.0, 180.0, 0.0));

    // Midpoint pose (between key 1 and key 2)
    let (p_mid, _) = traj.evaluate_pose(0.5);
    assert!(p_mid.z > 128.0 && p_mid.z < 256.0);
}

#[test]
fn test_auto_pathing_builder_and_reachspec_generation() {
    let mut state = EditorState::default();
    state.nav_graph = NavGraph::new();

    // 1. Corridor path nodes
    state.nav_graph.add_node("Path0", Vec3::new(0.0, 0.0, 0.0), "PathNode");
    state.nav_graph.add_node("Path1", Vec3::new(300.0, 0.0, 10.0), "PathNode");
    state.nav_graph.add_node("Path2", Vec3::new(600.0, 0.0, 20.0), "PathNode");

    // 2. Elevated ledge node (+75 uu jump, within jump height 96 uu)
    state.nav_graph.add_node("LedgeAmmo", Vec3::new(800.0, 0.0, 95.0), "InventorySpot");

    // 3. Isolated node outside maximum link distance
    state.nav_graph.add_node("IsolatedNode", Vec3::new(8000.0, 8000.0, 0.0), "PathNode");

    let report = state.build_auto_paths(1000.0, 54.0, 96.0);
    assert_eq!(report.total_nodes, 5);
    assert!(report.created_reachspecs >= 6);
    assert!(report.walk_reachspecs > 0);
    assert!(report.jump_reachspecs > 0);
    assert_eq!(report.isolated_nodes, vec!["IsolatedNode".to_string()]);

    // Check reach flags on generated reachspecs
    let jump_spec = state.nav_graph.reach_specs.iter().find(|s| (s.reach_flags & R_JUMP) != 0);
    assert!(jump_spec.is_some(), "Must have reachspec flagged with R_JUMP");

    let walk_spec = state.nav_graph.reach_specs.iter().find(|s| (s.reach_flags & R_WALK) != 0);
    assert!(walk_spec.is_some(), "Must have reachspec flagged with R_WALK");
}

#[test]
fn test_dispatcher_timeline_and_trigger_sequence_validation() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // 1. Dispatcher actor with 3 stages
    let mut disp = ActorRecord::default();
    disp.class_name = "Engine.Dispatcher".to_string();
    disp.name = "MainSequenceDispatcher".to_string();
    disp.tag = "StageTrigger".to_string();
    disp.location = Vec3::new(0.0, 0.0, 0.0);

    // Delays: 0.5s, 1.5s, 2.0s
    disp.properties.insert("OutDelays[0]".to_string(), TaggedProperty {
        name: "OutDelays[0]".to_string(), array_index: 0, value: TaggedValue::Float(0.5)
    });
    disp.properties.insert("OutDelays[1]".to_string(), TaggedProperty {
        name: "OutDelays[1]".to_string(), array_index: 1, value: TaggedValue::Float(1.5)
    });
    disp.properties.insert("OutDelays[2]".to_string(), TaggedProperty {
        name: "OutDelays[2]".to_string(), array_index: 2, value: TaggedValue::Float(2.0)
    });

    // Events: "BlastDoor", "WarningKlaxon", "NonExistentTrap"
    disp.properties.insert("OutEvents[0]".to_string(), TaggedProperty {
        name: "OutEvents[0]".to_string(), array_index: 0, value: TaggedValue::Str("BlastDoor".to_string())
    });
    disp.properties.insert("OutEvents[1]".to_string(), TaggedProperty {
        name: "OutEvents[1]".to_string(), array_index: 1, value: TaggedValue::Str("WarningKlaxon".to_string())
    });
    disp.properties.insert("OutEvents[2]".to_string(), TaggedProperty {
        name: "OutEvents[2]".to_string(), array_index: 2, value: TaggedValue::Str("NonExistentTrap".to_string())
    });
    level.actors.push(disp);

    // 2. Target receiver actors
    let mut door = ActorRecord::default();
    door.name = "HeavyDoor_1".to_string();
    door.tag = "BlastDoor".to_string();
    door.location = Vec3::new(500.0, 0.0, 0.0);
    level.actors.push(door);

    let mut klaxon1 = ActorRecord::default();
    klaxon1.name = "Siren_Alpha".to_string();
    klaxon1.tag = "WarningKlaxon".to_string();
    klaxon1.location = Vec3::new(200.0, 300.0, 100.0);
    level.actors.push(klaxon1);

    let mut klaxon2 = ActorRecord::default();
    klaxon2.name = "Siren_Beta".to_string();
    klaxon2.tag = "WarningKlaxon".to_string();
    klaxon2.location = Vec3::new(200.0, -300.0, 100.0);
    level.actors.push(klaxon2);

    state.loaded_unr = Some(level);

    let sequences = state.get_dispatcher_sequences();
    assert_eq!(sequences.len(), 1);

    let seq = &sequences[0];
    assert_eq!(seq.dispatcher_name, "MainSequenceDispatcher");
    assert_eq!(seq.tag, "StageTrigger");
    assert_eq!(seq.stages.len(), 3);
    assert_eq!(seq.total_duration_secs, 4.0); // 0.5 + 1.5 + 2.0
    assert!(seq.has_unmatched_events); // "NonExistentTrap" has no target
    assert!(!seq.has_recursive_loop);

    // Verify stage timings and targets
    assert_eq!(seq.stages[0].cumulative_time_secs, 0.5);
    assert_eq!(seq.stages[0].matched_targets.len(), 1);
    assert_eq!(seq.stages[0].matched_targets[0].0, "HeavyDoor_1");

    assert_eq!(seq.stages[1].cumulative_time_secs, 2.0); // 0.5 + 1.5
    assert_eq!(seq.stages[1].matched_targets.len(), 2);

    assert_eq!(seq.stages[2].cumulative_time_secs, 4.0); // 2.0 + 2.0
    assert_eq!(seq.stages[2].matched_targets.len(), 0);
}
