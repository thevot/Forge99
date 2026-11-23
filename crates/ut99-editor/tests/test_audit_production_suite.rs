use glam::Vec3;
use ut99_bsp::{build_cube, CsgOperation, PF_SEMISOLID};
use ut99_editor::editor_state::EditorState;
use ut99_format::unr::actor::ActorRecord;
use ut99_format::unr::level::UnrLevel;

#[test]
fn test_camera_bookmarks_slots_and_jump() {
    let mut state = EditorState::default();

    // Position camera at a specific point of interest
    state.camera.position = Vec3::new(1024.0, 512.0, 256.0);
    state.camera.yaw = 90.0f32.to_radians();
    state.camera.pitch = -15.0f32.to_radians();
    state.camera.ortho_zoom = 2048.0;

    // Save to slot 0 (Bookmark 1)
    state.set_camera_bookmark(0, Some("Red Flag Base".to_string()));

    assert!(state.camera_bookmarks[0].is_some());
    let bm = state.camera_bookmarks[0].as_ref().unwrap();
    assert_eq!(bm.name, "Red Flag Base");
    assert_eq!(bm.position, Vec3::new(1024.0, 512.0, 256.0));

    // Move camera far away
    state.camera.position = Vec3::new(-5000.0, -5000.0, 0.0);
    state.camera.yaw = 0.0;
    state.camera.pitch = 0.0;
    state.camera.ortho_zoom = 512.0;

    // Jump back to bookmark slot 0
    let jumped = state.goto_camera_bookmark(0);
    assert!(jumped, "Goto valid bookmark should return true");
    assert_eq!(state.camera.position, Vec3::new(1024.0, 512.0, 256.0));
    assert_eq!(state.camera.yaw, 90.0f32.to_radians());
    assert_eq!(state.camera.pitch, -15.0f32.to_radians());
    assert_eq!(state.camera.ortho_zoom, 2048.0);

    // Jump to empty slot 3 should fail gracefully
    assert!(!state.goto_camera_bookmark(3), "Goto empty slot should return false");

    // Clear bookmark slot 0
    state.clear_camera_bookmark(0);
    assert!(state.camera_bookmarks[0].is_none());
}

#[test]
fn test_map_engine_budget_telemetry_profiler() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());

    // Clear default initial room to test exact counts
    state.world_brushes.clear();

    // 1. Add 2 Additive cubes
    let cube1 = build_cube("Floor", CsgOperation::Add, Vec3::new(512.0, 512.0, 32.0));
    let cube2 = build_cube("Wall", CsgOperation::Add, Vec3::new(32.0, 512.0, 256.0));
    state.world_brushes.push(cube1);
    state.world_brushes.push(cube2);

    // 2. Add 1 Subtractive cube
    let alcove = build_cube("Alcove", CsgOperation::Subtract, Vec3::new(64.0, 64.0, 64.0));
    state.world_brushes.push(alcove);

    // 3. Add 1 Semisolid pillar
    let mut pillar = build_cube("Pillar", CsgOperation::Add, Vec3::new(32.0, 32.0, 128.0));
    for p in &mut pillar.polys {
        p.flags |= PF_SEMISOLID;
    }
    state.world_brushes.push(pillar);

    // 4. Add diverse gameplay actors
    let unr = state.loaded_unr.as_mut().unwrap();

    let mut light = ActorRecord::default();
    light.class_name = "Light".to_string();
    unr.actors.push(light);

    let mut path_node = ActorRecord::default();
    path_node.class_name = "PathNode".to_string();
    unr.actors.push(path_node);

    let mut shock_rifle = ActorRecord::default();
    shock_rifle.class_name = "ShockRifle".to_string();
    unr.actors.push(shock_rifle);

    let mut trigger = ActorRecord::default();
    trigger.class_name = "Trigger".to_string();
    unr.actors.push(trigger);

    let mut zone = ActorRecord::default();
    zone.class_name = "ZoneInfo".to_string();
    zone.group = "WaterZone1".to_string();
    unr.actors.push(zone);

    // Query engine budget stats
    let stats = state.get_map_budget_stats();

    assert_eq!(stats.total_brushes, 4);
    assert_eq!(stats.additive_brushes, 3);
    assert_eq!(stats.subtractive_brushes, 1);
    assert_eq!(stats.semisolid_brushes, 1);
    assert_eq!(stats.total_polygons, 24); // 4 cubes * 6 faces
    assert_eq!(stats.estimated_bsp_nodes, (24.0 * 1.5) as usize); // 36 nodes

    assert_eq!(stats.total_actors, 5);
    assert_eq!(stats.light_actors, 1);
    assert_eq!(stats.nav_actors, 1);
    assert_eq!(stats.pickup_actors, 1);
    assert_eq!(stats.trigger_actors, 1);
    assert_eq!(stats.total_zones, 2); // Default Zone0 + WaterZone1

    // Ratios should be valid numbers <= 1.0
    assert!(stats.node_budget_ratio > 0.0 && stats.node_budget_ratio < 0.05);
    assert!(stats.brush_budget_ratio > 0.0 && stats.brush_budget_ratio < 0.05);
    assert!(stats.actor_budget_ratio > 0.0 && stats.actor_budget_ratio < 0.05);
}

#[test]
fn test_auto_save_snapshot_rolling_and_recovery() {
    let mut state = EditorState::default();

    // Setup initial geometry
    state.world_brushes.clear();
    let cube = build_cube("Room", CsgOperation::Add, Vec3::new(256.0, 256.0, 256.0));
    state.world_brushes.push(cube);

    assert_eq!(state.world_brushes.len(), 1);

    // Trigger auto-save snapshot #1
    let msg1 = state.trigger_auto_save();
    assert!(msg1.contains("Auto-Save"));
    assert_eq!(state.auto_save_snapshots.len(), 1);

    // Add another brush
    let cube2 = build_cube("Pillar", CsgOperation::Add, Vec3::new(32.0, 32.0, 128.0));
    state.world_brushes.push(cube2);
    assert_eq!(state.world_brushes.len(), 2);

    // Trigger auto-save snapshot #2
    state.trigger_auto_save();
    assert_eq!(state.auto_save_snapshots.len(), 2);

    // Test restoring to snapshot #1 (which had 1 brush)
    let res = state.restore_auto_save_snapshot(0);
    assert!(res.is_ok(), "Restoring valid auto-save snapshot must succeed");
    assert_eq!(state.world_brushes.len(), 1, "Restored state must match snapshot 1 geometry");

    // Test rolling limit: trigger 5 more snapshots (total 7 attempted, max 5 kept)
    for _ in 0..5 {
        state.trigger_auto_save();
    }
    assert_eq!(state.auto_save_snapshots.len(), 5, "Auto-save must cap snapshots at rolling 5");

    // Test periodic auto-save timer
    state.auto_save_interval_secs = 60.0;
    state.auto_save_timer = 0.0;
    state.auto_save_enabled = true;

    // Advance 30 seconds -> should not trigger yet
    let triggered = state.check_auto_save(30.0);
    assert!(!triggered);
    assert_eq!(state.auto_save_timer, 30.0);

    // Advance another 35 seconds -> total 65s >= 60s -> should trigger!
    let triggered_now = state.check_auto_save(35.0);
    assert!(triggered_now);
    assert_eq!(state.auto_save_timer, 0.0, "Timer resets after auto-save");
}

#[test]
fn test_ut99_launch_settings_and_fail_safe() {
    let mut state = EditorState::default();

    // Verify default configuration
    assert!(state.ut99_executable_path.contains("UnrealTournament.exe"));
    assert!(state.ut99_launch_args.contains("Botpack.DeathMatchPlus"));

    // Configure custom launch parameters
    state.ut99_executable_path = "D:/CustomGames/UT99/System/UnrealTournament.exe".to_string();
    state.ut99_launch_args = "?game=Botpack.CTFGame?mutator=Botpack.InstaGibDM -log -windowed".to_string();

    // Attempting to launch when exe does not exist must return Err gracefully without panic
    let res = state.play_map_in_ut99();
    assert!(res.is_err());
    let err_str = res.err().unwrap();
    assert!(err_str.contains("UT99 Executable not found"));
}
