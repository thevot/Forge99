use glam::Vec3;
use ut99_bsp::{simulate_jumppad_trajectory, TacticalCoverRating, R_SPECIAL, R_WALK};
use ut99_editor::editor_state::EditorState;
use ut99_format::{ActorRecord, UnrLevel};

#[test]
fn test_jumppad_ballistics_and_parabolic_trajectory() {
    let source_loc = Vec3::new(0.0, 0.0, 100.0);
    let kick_vel = Vec3::new(400.0, 0.0, 800.0);
    let target_loc = Vec3::new(600.0, 0.0, 100.0);

    let arc = simulate_jumppad_trajectory(
        "UT_JumpPad0",
        source_loc,
        kick_vel,
        Some("PathNode_Landing".to_string()),
        Some(target_loc),
        -950.0,
    );

    assert_eq!(arc.source_name, "UT_JumpPad0");
    assert_eq!(arc.trajectory_points.len(), 25);
    assert!(arc.flight_time_secs > 0.5 && arc.flight_time_secs < 2.5);
    assert!(arc.apex_height_z > source_loc.z, "Apex ({}) must be higher than launch ({})", arc.apex_height_z, source_loc.z);

    // Verify arc points start at source
    assert!((arc.trajectory_points[0] - source_loc).length() < 1e-2);
}

#[test]
fn test_lift_mover_synchronization_validation() {
    let mut state = EditorState::default();

    let mut level = UnrLevel::default();
    level.map_name = "DM-ElevatorTest".to_string();

    // 1. LiftCenter with matching Mover and 2 LiftExits
    let mut lift_center = ActorRecord::default();
    lift_center.class_name = "LiftCenter".to_string();
    lift_center.name = "LiftCenter0".to_string();
    lift_center.tag = "Elevator1".to_string();
    lift_center.location = Vec3::new(0.0, 0.0, 0.0);
    level.actors.push(lift_center);

    let mut mover = ActorRecord::default();
    mover.class_name = "Mover".to_string();
    mover.name = "Mover0".to_string();
    mover.tag = "Elevator1".to_string();
    level.actors.push(mover);

    let mut exit_bottom = ActorRecord::default();
    exit_bottom.class_name = "LiftExit".to_string();
    exit_bottom.name = "LiftExit_Bottom".to_string();
    exit_bottom.tag = "Elevator1".to_string();
    exit_bottom.location = Vec3::new(64.0, 0.0, 0.0);
    level.actors.push(exit_bottom);

    let mut exit_top = ActorRecord::default();
    exit_top.class_name = "LiftExit".to_string();
    exit_top.name = "LiftExit_Top".to_string();
    exit_top.tag = "Elevator1".to_string();
    exit_top.location = Vec3::new(64.0, 0.0, 256.0);
    level.actors.push(exit_top);

    // 2. Broken LiftCenter without Mover
    let mut broken_lift = ActorRecord::default();
    broken_lift.class_name = "LiftCenter".to_string();
    broken_lift.name = "LiftCenter_Broken".to_string();
    broken_lift.tag = "OrphanTag".to_string();
    level.actors.push(broken_lift);

    state.loaded_unr = Some(level);

    let reports = state.validate_lift_synchronization();
    assert_eq!(reports.len(), 2);

    let good_report = reports.iter().find(|r| r.lift_tag == "Elevator1").unwrap();
    assert!(good_report.is_synchronized, "Elevator1 must be fully synchronized");
    assert_eq!(good_report.associated_exits.len(), 2);
    assert_eq!(good_report.associated_mover_name.as_deref(), Some("Mover0"));

    let bad_report = reports.iter().find(|r| r.lift_tag == "OrphanTag").unwrap();
    assert!(!bad_report.is_synchronized, "Orphan lift must fail synchronization");
    assert!(bad_report.issues.iter().any(|i| i.contains("No Mover found")));
}

#[test]
fn test_tactical_cover_and_sightline_ratings() {
    let mut state = EditorState::default();

    // Node 0: Shielded alcove far away with only 1 visible neighbor (Hallway Entrance) -> SolidCover
    state.nav_graph.add_node("PathNode_Alcove", Vec3::new(-2000.0, 0.0, 50.0), "PathNode");
    state.nav_graph.add_node("PathNode_HallwayEntrance", Vec3::new(-1600.0, 0.0, 50.0), "PathNode");

    // Open courtyard cluster (4+ nodes close to each other)
    state.nav_graph.add_node("PathNode_Courtyard", Vec3::new(0.0, 0.0, 50.0), "PathNode");
    state.nav_graph.add_node("PathNode_C1", Vec3::new(100.0, 50.0, 50.0), "PathNode");
    state.nav_graph.add_node("PathNode_C2", Vec3::new(100.0, -50.0, 50.0), "PathNode");
    state.nav_graph.add_node("PathNode_C3", Vec3::new(-100.0, 50.0, 50.0), "PathNode");

    // High sniper vantage perch
    state.nav_graph.add_node("PathNode_SniperPerch", Vec3::new(0.0, 0.0, 400.0), "PathNode");

    let entries = state.compute_tactical_cover_ratings();
    assert_eq!(entries.len(), 7);

    let alcove = entries.iter().find(|e| e.node_name == "PathNode_Alcove").unwrap();
    assert_eq!(alcove.cover_rating, TacticalCoverRating::SolidCover);

    let sniper = entries.iter().find(|e| e.node_name == "PathNode_SniperPerch").unwrap();
    assert!(sniper.is_high_vantage_perch, "Node with Z = 400 must be marked as high vantage perch");
}

#[test]
fn test_bot_nav_studio_autopaths_and_manual_linking() {
    let mut state = EditorState::default();

    // Add nodes in a walkable line
    state.nav_graph.add_node("Node0", Vec3::new(0.0, 0.0, 0.0), "PathNode");
    state.nav_graph.add_node("Node1", Vec3::new(300.0, 0.0, 0.0), "PathNode");
    state.nav_graph.add_node("Node2", Vec3::new(600.0, 0.0, 0.0), "PathNode");

    // Build auto paths
    let report = state.build_auto_paths(450.0, 54.0, 96.0);
    assert_eq!(report.total_nodes, 3);
    assert!(report.created_reachspecs >= 4, "Must link 0<->1 and 1<->2");
    assert_eq!(report.isolated_nodes.len(), 0);

    // Link manual special path between 0 and 2
    let linked = state.link_nav_nodes(0, 2, R_SPECIAL | R_WALK);
    assert!(linked);
    let has_special_link = state.nav_graph.reach_specs.iter().any(|s| s.from == 0 && s.to == 2 && (s.reach_flags & R_SPECIAL) != 0);
    assert!(has_special_link, "Manual special reachspec must be present in NavGraph");
}
