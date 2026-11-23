use glam::Vec3;
use ut99_bsp::{build_cube, CsgOperation, LevelDoctor, NavGraph, NavReachSpec};
use ut99_editor::editor_state::{lux_to_heatmap_color, EditorState};
use ut99_format::T3dActor;

#[test]
fn test_kicker_ballistic_trajectory_simulation() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Add a solid floor brush at Z = -16 (top surface at Z = 0)
    let mut floor_brush = build_cube("Floor", CsgOperation::Add, Vec3::new(4000.0, 4000.0, 32.0));
    floor_brush.location = Vec3::new(0.0, 0.0, -16.0);
    state.world_brushes.push(floor_brush);

    // 2. Add a Kicker actor launched horizontally and upward
    let mut kicker = T3dActor::new("Kicker", "JumpPad0");
    kicker.set_location(Vec3::new(0.0, 0.0, 50.0));
    kicker.set_property("KickVelocity", "(X=600.0,Y=0.0,Z=900.0)");
    state.map.actors.push(kicker);

    // 3. Compute trajectories
    let arcs = state.compute_trajectories();
    assert_eq!(arcs.len(), 1, "Should find and compute trajectory for the kicker");

    let arc = &arcs[0];
    assert!(!arc.is_teleport);
    assert_eq!(arc.source_name, "JumpPad0");
    assert!(arc.points.len() > 10, "Trajectory must be discretized into multiple flight steps");

    // Flight time under gravity g = -950 uu/s^2:
    // With Vz = 900, apex is ~0.95s, flight time to floor is ~1.9s - 2.0s
    assert!(arc.flight_time > 1.0 && arc.flight_time < 3.0, "Expected flight time between 1.0 and 3.0s, got {:.2}", arc.flight_time);

    // Check that apex height is reached and then drops
    let max_z = arc.points.iter().map(|p| p.z).fold(f32::NEG_INFINITY, f32::max);
    assert!(max_z > 400.0, "Kicker apex should exceed 400 uu (got {:.1})", max_z);

    // Verify landing point is on or near the floor surface (Z = 0)
    assert!(arc.landing_point.is_some(), "Should find a floor landing point");
    let landing = arc.landing_point.unwrap();
    assert!(landing.x > 500.0, "Kicker should land forward along X (got {:.1})", landing.x);
    assert!((landing.z - 0.0).abs() < 5.0, "Landing Z should intersect floor at Z ~ 0 (got {:.2})", landing.z);
}

#[test]
fn test_teleporter_bezier_arc_linking() {
    let mut state = EditorState::default();

    // Add Teleporter A pointing to RedBase
    let mut tele_a = T3dActor::new("Teleporter", "TeleporterA");
    tele_a.set_location(Vec3::new(-500.0, 0.0, 64.0));
    tele_a.set_property("URL", "RedBase");
    state.map.actors.push(tele_a);

    // Add Teleporter B tagged RedBase
    let mut tele_b = T3dActor::new("Teleporter", "TeleporterB");
    tele_b.set_location(Vec3::new(500.0, 0.0, 64.0));
    tele_b.set_property("Tag", "RedBase");
    state.map.actors.push(tele_b);

    let arcs = state.compute_trajectories();
    assert_eq!(arcs.len(), 1, "Should find matched teleporter pair and generate Bezier arc");

    let arc = &arcs[0];
    assert!(arc.is_teleport);
    assert_eq!(arc.source_name, "TeleporterA");
    assert_eq!(arc.points.len(), 33, "Quadratic Bezier should generate 32 segments (33 points)");

    // Start and end points
    assert_eq!(arc.points.first().unwrap().x, -500.0);
    assert_eq!(arc.points.last().unwrap().x, 500.0);

    // Midpoint should arch upward gracefully
    let mid_pt = arc.points[16];
    assert!((mid_pt.x - 0.0).abs() < 10.0, "Midpoint X should be near 0");
    assert!(mid_pt.z > 200.0, "Midpoint Z should be elevated above ground (got {:.1})", mid_pt.z);
}

#[test]
fn test_luxmeter_false_color_bands() {
    // 1. Underexposed / Pitch Black (< 0.15 / < 15 Lux): Deep Blue / Indigo
    let col_under = lux_to_heatmap_color(0.05);
    assert!(col_under[2] > col_under[0] && col_under[2] > col_under[1], "Blue channel must dominate under 15 Lux");

    // 2. Low shadow (0.15 .. 0.50 / 15..50 Lux): Cyan / Teal
    let col_shadow = lux_to_heatmap_color(0.30);
    assert!(col_shadow[1] > 0.3 && col_shadow[2] > 0.3, "Cyan/Teal (G and B) must be strong for shadow");

    // 3. Ideal tournament gameplay (0.50 .. 1.00 / 50..100 Lux): Vibrant Green
    let col_ideal = lux_to_heatmap_color(0.75);
    assert!(col_ideal[1] > 0.7 && col_ideal[1] > col_ideal[0], "Green channel must dominate in ideal tournament band");

    // 4. Direct illumination (1.00 .. 1.80 / 100..180 Lux): Yellow / Amber
    let col_direct = lux_to_heatmap_color(1.40);
    assert!(col_direct[0] > 0.8 && col_direct[1] > 0.4, "Red & Green channels must form Amber/Yellow");

    // 5. Overexposed / Glare hazard (>= 1.80 / > 180 Lux): Scarlet Red / White
    let col_over = lux_to_heatmap_color(2.00);
    assert!(col_over[0] > 0.9, "Red channel must be near max for overexposure");
}

#[test]
fn test_multi_object_selection_and_batch_operations() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Add 3 brushes
    let b0 = build_cube("Brush0", CsgOperation::Add, Vec3::new(100.0, 100.0, 100.0));
    let b1 = build_cube("Brush1", CsgOperation::Add, Vec3::new(100.0, 100.0, 100.0));
    let b2 = build_cube("Brush2", CsgOperation::Add, Vec3::new(100.0, 100.0, 100.0));
    state.world_brushes.push(b0);
    state.world_brushes.push(b1);
    state.world_brushes.push(b2);

    // Initial state: nothing selected
    assert!(!state.has_multi_selection());
    assert_eq!(state.total_selected_count(), 0);

    // Single select Brush 0
    state.select_brush_toggle(0, false);
    assert!(!state.has_multi_selection());
    assert!(state.is_brush_selected(0));
    assert!(!state.is_brush_selected(1));

    // Multi-select Brush 1 with Ctrl (multi = true)
    state.select_brush_toggle(1, true);
    assert!(state.has_multi_selection());
    assert_eq!(state.total_selected_count(), 2);
    assert!(state.is_brush_selected(0));
    assert!(state.is_brush_selected(1));
    assert!(!state.is_brush_selected(2));

    // 1. Batch Texture Apply
    state.active_texture = Some("Indus1.MetalWall01".to_string());
    let applied = state.apply_active_texture_to_multi_selected();
    assert_eq!(applied, 2, "Should apply texture to both selected brushes");
    assert_eq!(state.world_brushes[0].polys[0].texture, "Indus1.MetalWall01");
    assert_eq!(state.world_brushes[1].polys[0].texture, "Indus1.MetalWall01");
    assert_ne!(state.world_brushes[2].polys[0].texture, "Indus1.MetalWall01");

    // 2. Batch Prefab Creation
    let prefab_created = state.create_prefab_from_selection("TestArchPrefab", "Pillars");
    assert!(prefab_created);
    assert!(state.prefabs.iter().any(|p| p.name == "TestArchPrefab" && p.brushes.len() == 2));

    // 3. Batch Duplication
    let (new_b, _) = state.duplicate_selected_batch(Vec3::new(64.0, 0.0, 0.0));
    assert_eq!(new_b, 2, "Should duplicate 2 brushes");
    assert_eq!(state.world_brushes.len(), 5, "Total brushes should be 3 + 2 = 5");

    // 4. Batch Delete
    let (del_b, _) = state.delete_selected_batch();
    assert_eq!(del_b, 2, "Should delete the newly duplicated and selected 2 brushes");
    assert_eq!(state.world_brushes.len(), 3, "Total brushes returned to original 3");
    assert!(!state.has_multi_selection());
}

#[test]
fn test_level_doctor_tournament_gameplay_clearance() {
    let mut graph = NavGraph::new();

    // 1. Corridor Bottleneck (< 38 uu)
    let n0 = graph.add_node("ChokeNode0", Vec3::new(0.0, 0.0, 0.0), "PathNode");
    let n1 = graph.add_node("ChokeNode1", Vec3::new(30.0, 0.0, 0.0), "PathNode");
    graph.reach_specs.push(NavReachSpec {
        from: n0,
        to: n1,
        distance: 30.0, // Bottleneck (< 38 uu)
        reach_flags: 1,
    });

    // 2. Lethal Drop Hazard (> 800 uu)
    let n2 = graph.add_node("HighCliff", Vec3::new(500.0, 0.0, 1000.0), "PathNode");
    let n3 = graph.add_node("DeathPit", Vec3::new(500.0, 100.0, 100.0), "PathNode"); // drop of 900 uu
    graph.reach_specs.push(NavReachSpec {
        from: n2,
        to: n3,
        distance: 905.0,
        reach_flags: 1,
    });

    // 3. Embedded PlayerStart in solid brush
    let mut solid_brush = build_cube("SolidWall", CsgOperation::Add, Vec3::new(200.0, 200.0, 200.0));
    solid_brush.location = Vec3::new(1000.0, 1000.0, 0.0);

    let actors = vec![
        ("PlayerStart_Embedded".to_string(), "Botpack.PlayerStart".to_string(), Vec3::new(1000.0, 1000.0, 0.0)),
    ];

    let issues = LevelDoctor::audit_gameplay_clearance(&[solid_brush], &graph, &actors);

    assert!(issues.iter().any(|i| i.category == "GameplayClearance" && i.description.contains("Corridor bottleneck")), "Must flag corridor bottleneck < 38 uu");
    assert!(issues.iter().any(|i| i.category == "GameplayHazard" && i.description.contains("Lethal drop")), "Must flag lethal drop > 800 uu");
    assert!(issues.iter().any(|i| i.category == "ActorClearance" && i.description.contains("embedded")), "Must flag embedded PlayerStart actor");
}
