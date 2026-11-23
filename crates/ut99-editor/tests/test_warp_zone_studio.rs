use glam::Vec3;
use ut99_editor::editor_state::{EditorState, WarpConnectionStatus};
use ut99_format::{ActorRecord, TaggedProperty, TaggedValue, UnrLevel};

fn make_warp_actor(
    export_index: usize,
    name: &str,
    tag: &str,
    this_tag: &str,
    other_side: &str,
    loc: Vec3,
    yaw_rot: i32,
) -> ActorRecord {
    let mut actor = ActorRecord::default();
    actor.export_index = export_index;
    actor.name = name.to_string();
    actor.class_name = "Engine.WarpZoneInfo".to_string();
    actor.tag = tag.to_string();
    actor.location = loc;
    actor.rotation = (0, yaw_rot, 0);

    if !this_tag.is_empty() {
        actor.properties.insert(
            "ThisTag".to_string(),
            TaggedProperty {
                name: "ThisTag".to_string(),
                array_index: 0,
                value: TaggedValue::Str(this_tag.to_string()),
            },
        );
    }

    if !other_side.is_empty() {
        actor.properties.insert(
            "OtherSideURL".to_string(),
            TaggedProperty {
                name: "OtherSideURL".to_string(),
                array_index: 0,
                value: TaggedValue::Str(other_side.to_string()),
            },
        );
    }

    actor
}

#[test]
fn test_warpzone_topology_detection_and_metrics() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // 1. Paired bi-directional portals (A <-> B) with 180° opposing yaw (0 vs 32768)
    let warp_a = make_warp_actor(0, "WarpZone_East", "WarpA", "WarpA", "WarpB", Vec3::new(500.0, 0.0, 0.0), 0);
    let warp_b = make_warp_actor(1, "WarpZone_West", "WarpB", "WarpB", "WarpA", Vec3::new(-500.0, 0.0, 0.0), 32768);

    // 2. Loop chain portals (C -> D -> E -> C)
    let warp_c = make_warp_actor(2, "Warp_LoopC", "LoopC", "LoopC", "LoopD", Vec3::new(0.0, 1000.0, 0.0), 0);
    let warp_d = make_warp_actor(3, "Warp_LoopD", "LoopD", "LoopD", "LoopE", Vec3::new(0.0, 2000.0, 0.0), 0);
    let warp_e = make_warp_actor(4, "Warp_LoopE", "LoopE", "LoopE", "LoopC", Vec3::new(0.0, 3000.0, 0.0), 0);

    // 3. Orphan broken portal (F -> None)
    let warp_f = make_warp_actor(5, "Warp_Orphan", "OrphanF", "OrphanF", "UnexistingVoid", Vec3::new(100.0, 100.0, 100.0), 0);

    level.actors = vec![warp_a, warp_b, warp_c, warp_d, warp_e, warp_f].into();
    state.loaded_unr = Some(level);

    let topology = state.collect_warpzone_topology();
    assert_eq!(topology.len(), 6);

    // Verify A <-> B
    let entry_a = topology.iter().find(|e| e.name == "WarpZone_East").expect("WarpZone_East not found");
    assert_eq!(entry_a.status, WarpConnectionStatus::MatchedBiDirectional);
    assert_eq!(entry_a.target_name.as_deref(), Some("WarpZone_West"));
    assert!((entry_a.distance_uu.unwrap() - 1000.0).abs() < 1e-2);
    assert!(entry_a.is_seamless_mirror, "Portals opposing at 180° must be marked as seamless mirrors");

    let entry_b = topology.iter().find(|e| e.name == "WarpZone_West").expect("WarpZone_West not found");
    assert_eq!(entry_b.status, WarpConnectionStatus::MatchedBiDirectional);
    assert_eq!(entry_b.target_name.as_deref(), Some("WarpZone_East"));

    // Verify Loop C -> D -> E -> C
    let entry_c = topology.iter().find(|e| e.name == "Warp_LoopC").expect("LoopC not found");
    assert_eq!(entry_c.status, WarpConnectionStatus::LoopChain);
    let entry_d = topology.iter().find(|e| e.name == "Warp_LoopD").expect("LoopD not found");
    assert_eq!(entry_d.status, WarpConnectionStatus::LoopChain);
    let entry_e = topology.iter().find(|e| e.name == "Warp_LoopE").expect("LoopE not found");
    assert_eq!(entry_e.status, WarpConnectionStatus::LoopChain);

    // Verify Orphan
    let entry_f = topology.iter().find(|e| e.name == "Warp_Orphan").expect("Orphan not found");
    assert_eq!(entry_f.status, WarpConnectionStatus::OrphanBroken);
    assert!(entry_f.target_name.is_none());
}

#[test]
fn test_coordinated_warp_pair_generation_and_undo_redo() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());

    let base_origin = Vec3::new(100.0, 200.0, 0.0);
    let sep = Vec3::new(1024.0, 0.0, 0.0);

    let (idx_a, idx_b) = state.create_coordinated_warp_pair(base_origin, sep);
    assert_ne!(idx_a, idx_b);

    // Check actors exist in level
    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors.len(), 2);
    let a = &unr.actors[idx_a];
    let b = &unr.actors[idx_b];

    assert_eq!(a.location, base_origin);
    assert_eq!(b.location, base_origin + sep);
    assert_eq!(a.rotation.1, 0);
    assert_eq!(b.rotation.1, 32768, "B must have 180° opposing yaw");

    // Check topology is immediately resolved as a perfect bi-directional pair
    let topology = state.collect_warpzone_topology();
    assert_eq!(topology.len(), 2);
    assert_eq!(topology[0].status, WarpConnectionStatus::MatchedBiDirectional);
    assert_eq!(topology[1].status, WarpConnectionStatus::MatchedBiDirectional);
    assert!(topology[0].is_seamless_mirror);
    assert!((topology[0].distance_uu.unwrap() - 1024.0).abs() < 1e-3);

    // Test Undo (two additions undone)
    assert!(state.history.can_undo());
    state.undo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);
    state.undo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // Test Redo
    assert!(state.history.can_redo());
    state.redo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);
    state.redo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);
}

#[test]
fn test_auto_pair_and_orientation_inversion_with_undo() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // Two initially unlinked WarpZones
    let wz1 = make_warp_actor(0, "ZoneAlfa", "AlphaTag", "AlphaTag", "", Vec3::new(0.0, 0.0, 0.0), 0);
    let wz2 = make_warp_actor(1, "ZoneBeta", "BetaTag", "BetaTag", "", Vec3::new(500.0, 0.0, 0.0), 0);
    level.actors = vec![wz1, wz2].into();
    state.loaded_unr = Some(level);

    // Initially both should be Orphan/Broken because OtherSideURL is empty
    let initial_top = state.collect_warpzone_topology();
    assert_eq!(initial_top[0].status, WarpConnectionStatus::OrphanBroken);
    assert_eq!(initial_top[1].status, WarpConnectionStatus::OrphanBroken);

    // Auto-pair them bi-directionally
    let pair_res = state.auto_pair_warp_zones(0, 1);
    assert!(pair_res.is_ok());

    let paired_top = state.collect_warpzone_topology();
    assert_eq!(paired_top[0].status, WarpConnectionStatus::MatchedBiDirectional);
    assert_eq!(paired_top[1].status, WarpConnectionStatus::MatchedBiDirectional);

    // Invert orientation on ZoneBeta (+180° Yaw)
    let inv_res = state.invert_warp_orientation(1);
    assert!(inv_res.is_ok());

    let inverted_top = state.collect_warpzone_topology();
    assert_eq!(inverted_top[1].rotation.1, 32768);
    assert!(inverted_top[0].is_seamless_mirror);

    // Test Undo of inversion
    state.undo();
    let undone_top = state.collect_warpzone_topology();
    assert_eq!(undone_top[1].rotation.1, 0, "Yaw must be reverted to 0");

    // Test Undo of pairing
    state.undo();
    let unlinked_top = state.collect_warpzone_topology();
    assert_eq!(unlinked_top[0].status, WarpConnectionStatus::OrphanBroken);
    assert_eq!(unlinked_top[1].status, WarpConnectionStatus::OrphanBroken);

    // Test Redo
    state.redo();
    let repared_top = state.collect_warpzone_topology();
    assert_eq!(repared_top[0].status, WarpConnectionStatus::MatchedBiDirectional);
}

#[test]
fn test_camera_teleportation_through_portal() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // Source at (0, 0, 0) Yaw 0, Target at (2000, 1000, 500) Yaw 32768 (180°)
    let portal_src = make_warp_actor(0, "Portal_In", "In", "In", "Out", Vec3::new(0.0, 0.0, 0.0), 0);
    let portal_tgt = make_warp_actor(1, "Portal_Out", "Out", "Out", "In", Vec3::new(2000.0, 1000.0, 500.0), 32768);
    level.actors = vec![portal_src, portal_tgt].into();
    state.loaded_unr = Some(level);

    // Place camera 100 units in front of source portal looking along +X (Yaw 0)
    state.camera.position = Vec3::new(-100.0, 0.0, 0.0);
    state.camera.yaw = 0.0;
    state.camera.pitch = 0.0;

    let res = state.teleport_camera_through_warp_zone(0);
    assert!(res.is_ok(), "Camera teleportation through paired portal must succeed");

    // Destination target is at (2000, 1000, 500).
    // Because target has opposing Yaw 180° (+X turned into continuous exit flow),
    // camera position is projected to the target portal's exit space
    let new_cam = state.camera.position;
    assert!((new_cam.x - 2000.0).abs() < 250.0);
    assert!((new_cam.y - 1000.0).abs() < 50.0);
    assert!((new_cam.z - 500.0).abs() < 50.0);
}
