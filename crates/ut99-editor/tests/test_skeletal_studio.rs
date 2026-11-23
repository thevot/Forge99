use glam::Vec3;
use ut99_bsp::{CanonicalAnimType, SkeletalMesh};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_skeletal_humanoid_hierarchy_and_fk_kinematics() {
    let humanoid = SkeletalMesh::create_canonical_ut99_humanoid();
    assert_eq!(humanoid.bones.len(), 17);

    // Root at Z = 40.0
    let root_idx = humanoid.bones.iter().position(|b| b.name == "Root").expect("Root bone missing");
    let root_pos = humanoid.bones[root_idx].world_transform.transform_point3(Vec3::ZERO);
    assert!((root_pos.z - 40.0).abs() < 1e-3);

    // Head position: Root (40) + Spine (16) + Chest (16) + Neck (8) + Head (12) = 92
    let head_idx = humanoid.bones.iter().position(|b| b.name == "Head").expect("Head bone missing");
    let head_pos = humanoid.bones[head_idx].world_transform.transform_point3(Vec3::ZERO);
    assert!((head_pos.z - 92.0).abs() < 1e-2);

    // Lateral arm symmetry: L_Shoulder (-16, 0, 6) vs R_Shoulder (16, 0, 6) relative to Chest (Z = 72)
    let l_sh_idx = humanoid.bones.iter().position(|b| b.name == "L_Shoulder").unwrap();
    let r_sh_idx = humanoid.bones.iter().position(|b| b.name == "R_Shoulder").unwrap();
    let l_sh_pos = humanoid.bones[l_sh_idx].world_transform.transform_point3(Vec3::ZERO);
    let r_sh_pos = humanoid.bones[r_sh_idx].world_transform.transform_point3(Vec3::ZERO);

    assert!((l_sh_pos.x + 16.0).abs() < 1e-2);
    assert!((r_sh_pos.x - 16.0).abs() < 1e-2);
    assert!((l_sh_pos.z - r_sh_pos.z).abs() < 1e-2);
}

#[test]
fn test_canonical_animations_and_pose_interpolation() {
    let mut humanoid = SkeletalMesh::create_canonical_ut99_humanoid();
    assert_eq!(humanoid.animations.len(), 7);

    // Ensure all canonical profiles exist
    let anim_names: Vec<&str> = humanoid.animations.iter().map(|a| a.name.as_str()).collect();
    for &anim_type in CanonicalAnimType::all() {
        let expected_name = match anim_type {
            CanonicalAnimType::Idle => "Idle",
            CanonicalAnimType::Run => "Run",
            CanonicalAnimType::Walk => "Walk",
            CanonicalAnimType::Jump => "Jump",
            CanonicalAnimType::Fire => "Fire",
            CanonicalAnimType::Death => "Death",
            CanonicalAnimType::Taunt => "Taunt",
        };
        assert!(anim_names.contains(&expected_name));
    }

    // Evaluate pose at t = 0.0 vs t = 0.4 on Run sequence
    assert!(humanoid.apply_pose_at_time("Run", 0.0));
    let r_hand_idx = humanoid.bones.iter().position(|b| b.name == "R_Hand").unwrap();
    let r_hand_t0 = humanoid.bones[r_hand_idx].world_transform.transform_point3(Vec3::ZERO);

    assert!(humanoid.apply_pose_at_time("Run", 0.4));
    let r_hand_t4 = humanoid.bones[r_hand_idx].world_transform.transform_point3(Vec3::ZERO);

    // Arm has swung during run cycle
    assert!(r_hand_t0.distance(r_hand_t4) > 1.0);

    // Intermediate pose interpolation at t = 0.2
    assert!(humanoid.apply_pose_at_time("Run", 0.2));
    let r_hand_t2 = humanoid.bones[r_hand_idx].world_transform.transform_point3(Vec3::ZERO);
    assert!(r_hand_t2.is_finite());
}

#[test]
fn test_socket_mounting_world_transform_and_undo_redo() {
    let mut state = EditorState::default();
    assert_eq!(state.skeletal_meshes.len(), 1);

    let mesh = &state.skeletal_meshes[0];
    assert_eq!(mesh.sockets.len(), 3);

    // WeaponPoint_R attached to R_Hand
    let sock = mesh.sockets.iter().find(|s| s.name == "WeaponPoint_R").unwrap();
    assert_eq!(sock.bone_name, "R_Hand");

    let sock_mat = mesh.get_socket_world_transform("WeaponPoint_R").expect("Socket transform must exist");
    let sock_pos = sock_mat.transform_point3(Vec3::ZERO);

    // Hand position
    let hand_idx = mesh.bones.iter().position(|b| b.name == "R_Hand").unwrap();
    let hand_pos = mesh.bones[hand_idx].world_transform.transform_point3(Vec3::ZERO);
    // Socket offset is (0, 6, 0)
    assert!((sock_pos.distance(hand_pos) - 6.0).abs() < 1e-2);

    // 1. Modify socket attachment
    state.set_skeletal_socket_attachment(0, "WeaponPoint_R", Some("SniperRifle".to_string()));
    assert_eq!(
        state.skeletal_meshes[0].sockets.iter().find(|s| s.name == "WeaponPoint_R").unwrap().attached_mesh_name.as_deref(),
        Some("SniperRifle")
    );

    // 2. Undo
    state.undo();
    assert_eq!(
        state.skeletal_meshes[0].sockets.iter().find(|s| s.name == "WeaponPoint_R").unwrap().attached_mesh_name.as_deref(),
        Some("ShockRifle")
    );

    // 3. Redo
    state.redo();
    assert_eq!(
        state.skeletal_meshes[0].sockets.iter().find(|s| s.name == "WeaponPoint_R").unwrap().attached_mesh_name.as_deref(),
        Some("SniperRifle")
    );
}

#[test]
fn test_skeletal_timeline_scrubber_and_level_placement() {
    let mut state = EditorState::default();

    // 1. Scrubbing timeline simulation
    state.skeletal_settings.active_animation = Some("Run".to_string());
    state.skeletal_settings.is_playing = true;
    state.skeletal_settings.play_speed = 1.0;
    state.skeletal_settings.loop_preview = true;
    state.skeletal_settings.timeline_time = 0.0;

    state.update_skeletal_animation_timeline(0.3);
    assert!((state.skeletal_settings.timeline_time - 0.3).abs() < 1e-3);

    // Run duration is 0.8s, advancing 0.7s wraps around to 0.2s
    state.update_skeletal_animation_timeline(0.7);
    assert!((state.skeletal_settings.timeline_time - 0.2).abs() < 1e-3);

    // 2. Place SkeletalMeshActor into level at pivot
    state.builder_brush.location = Vec3::new(128.0, 256.0, 64.0);
    let actor_idx = state.place_skeletal_mesh_at_pivot(0);

    let level = state.loaded_unr.as_ref().unwrap();
    let actor = &level.actors[actor_idx];
    assert_eq!(actor.class_name, "SkeletalMeshActor");
    assert_eq!(actor.location, Vec3::new(128.0, 256.0, 64.0));
    assert!(actor.properties.contains_key("Mesh"));
    assert!(actor.properties.contains_key("AnimSequence"));

    // 3. Undo placement
    state.undo();
    let level_undone = state.loaded_unr.as_ref().unwrap();
    assert!(level_undone.actors.is_empty());
}
