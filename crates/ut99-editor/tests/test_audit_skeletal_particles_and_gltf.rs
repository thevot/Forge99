use glam::Vec3;
use ut99_bsp::{
    build_cube, CsgOperation, ParticleEmitterType, ProceduralTextureSimulator,
    ProceduralTextureType, SkeletalMesh,
};
use ut99_editor::editor_state::{EditorMover, EditorState};
use ut99_format::{ActorRecord, UnrLevel};

#[test]
fn test_skeletal_mesh_humanoid_rig_and_skinning() {
    let state = EditorState::default();
    let meshes = state.get_skeletal_meshes();
    assert_eq!(meshes.len(), 1, "Default state should include canonical UT99 player humanoid");

    let humanoid = &meshes[0];
    assert_eq!(humanoid.bones.len(), 17);

    // Verify bone names and hierarchy
    let bone_names: Vec<&str> = humanoid.bones.iter().map(|b| b.name.as_str()).collect();
    assert!(bone_names.contains(&"Root"));
    assert!(bone_names.contains(&"Spine"));
    assert!(bone_names.contains(&"Chest"));
    assert!(bone_names.contains(&"Head"));
    assert!(bone_names.contains(&"L_Hand"));
    assert!(bone_names.contains(&"R_Hand"));
    assert!(bone_names.contains(&"L_Foot"));
    assert!(bone_names.contains(&"R_Foot"));

    // Verify Forward Kinematics world positions
    let root_pos = humanoid.bones[0].world_transform.transform_point3(Vec3::ZERO);
    assert!((root_pos.z - 40.0).abs() < 1e-2);

    let head_idx = humanoid.bones.iter().position(|b| b.name == "Head").unwrap();
    let head_pos = humanoid.bones[head_idx].world_transform.transform_point3(Vec3::ZERO);
    assert!((head_pos.z - 92.0).abs() < 1e-2);

    // Test Linear Blend Skinning on mesh vertices in local bone space
    let mut rigged_mesh = SkeletalMesh::create_canonical_ut99_humanoid();
    rigged_mesh.base_vertices.push(Vec3::new(0.0, 0.0, 0.0)); // Local head origin
    rigged_mesh.bone_indices.push(head_idx);
    rigged_mesh.bone_weights.push(1.0);

    let skinned = rigged_mesh.compute_skinned_vertices();
    assert_eq!(skinned.len(), 1);
    assert!((skinned[0].z - 92.0).abs() < 1e-2);
}

#[test]
fn test_particle_emitter_simulation_and_gravity() {
    let mut state = EditorState::default();
    state.particle_emitters.clear();

    // 1. Add smoke and spark emitters
    state.add_particle_emitter("IndustrialStack", Vec3::new(0.0, 0.0, 100.0), ParticleEmitterType::Smoke);
    state.add_particle_emitter("WiringShort", Vec3::new(200.0, 0.0, 150.0), ParticleEmitterType::Sparks);

    assert_eq!(state.particle_emitters.len(), 2);

    // Advance simulation by 0.3s
    state.update_particles(0.3);

    // Verify smoke emitter
    let smoke_emitter = &state.particle_emitters[0];
    assert!(!smoke_emitter.particles.is_empty(), "Smoke particles must spawn");
    for p in &smoke_emitter.particles {
        assert!(p.position.z >= 100.0, "Smoke must rise upward");
        assert!(p.color[3] > 0.0 && p.color[3] <= 1.0);
    }

    // Verify spark emitter
    let spark_emitter = &state.particle_emitters[1];
    assert!(!spark_emitter.particles.is_empty(), "Spark particles must spawn");
    assert!(spark_emitter.gravity.z < 0.0, "Sparks must be accelerated downward by gravity");
}

#[test]
fn test_procedural_texture_fire_and_water_simulation() {
    // 1. FireTexture simulation
    let mut fire = ProceduralTextureSimulator::new_fire(32, 32);
    assert_eq!(fire.tex_type, ProceduralTextureType::Fire);

    for _ in 0..6 {
        fire.step();
    }

    let fire_rgba = fire.to_rgba8();
    assert_eq!(fire_rgba.len(), 32 * 32 * 4);
    let has_ignited = fire_rgba.iter().any(|&p| p > 100);
    assert!(has_ignited, "FireTexture must produce ignited flame pixels");

    // 2. WaterTexture simulation
    let mut water = ProceduralTextureSimulator::new_water(32, 32);
    assert_eq!(water.tex_type, ProceduralTextureType::WaterRipples);

    // Inject droplet at center
    water.add_droplet(16, 16, 120.0);
    for _ in 0..5 {
        water.step();
    }

    let water_rgba = water.to_rgba8();
    assert_eq!(water_rgba.len(), 32 * 32 * 4);
    // Water has blue/cyan dominant channels
    assert!(water_rgba[2] > 0, "Water must have positive blue channel");
}

#[test]
fn test_full_map_gltf_2_export() {
    let mut state = EditorState::default();
    state.world_brushes.clear();
    state.movers.clear();

    // 1. Add World Geometry
    let room = build_cube("CentralHall", CsgOperation::Add, Vec3::new(512.0, 512.0, 256.0));
    state.world_brushes.push(room);

    // 2. Add Mover
    let mover_brush = build_cube("DoorMover", CsgOperation::Active, Vec3::new(64.0, 16.0, 128.0));
    let mut mover = EditorMover::new(mover_brush);
    mover.name = "AirlockDoor".to_string();
    mover.tag = "AirlockTrigger".to_string();
    state.movers.push(mover);

    // 3. Add Level Actors
    let mut level = UnrLevel::default();
    let mut start = ActorRecord::default();
    start.name = "PlayerStart_Alpha".to_string();
    start.class_name = "Engine.PlayerStart".to_string();
    start.location = Vec3::new(100.0, 50.0, 20.0);
    level.actors.push(start);

    let mut light = ActorRecord::default();
    light.name = "CeilingLight_1".to_string();
    light.class_name = "Engine.Light".to_string();
    light.location = Vec3::new(0.0, 0.0, 220.0);
    level.actors.push(light);

    state.loaded_unr = Some(level);

    // 4. Generate GLTF 2.0 Export
    let gltf_json = state.export_map_to_gltf();

    // Verify GLTF 2.0 Schema Conformance
    assert!(gltf_json.contains("\"version\": \"2.0\""));
    assert!(gltf_json.contains("\"CentralHall\""));
    assert!(gltf_json.contains("\"Mover_AirlockDoor\""));
    assert!(gltf_json.contains("\"PlayerStart_Alpha\""));
    assert!(gltf_json.contains("\"CeilingLight_1\""));
    assert!(gltf_json.contains("\"M_UT99_DefaultWorld\""));

    // 5. Test File Export
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_ut99_export.gltf");
    let result = state.export_map_to_gltf_file(&temp_file);
    assert!(result.is_ok());
    assert!(temp_file.exists());
    let _ = std::fs::remove_file(temp_file);
}
