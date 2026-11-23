//! Tests for Epic Games ActorX (.psk, .psa) interoperability and SkelEdit HitBoxes

use glam::Vec3;
use ut99_bsp::actorx::{ActorXPskMesh, PskBone, PskFace, PskMaterial, PskPoint, PskWedge, PskWeight};
use ut99_bsp::skeletal_mesh::SkeletalMesh;

#[test]
fn test_actorx_psk_chunk_roundtrip() {
    let mut mesh = ActorXPskMesh::default();
    mesh.points.push(PskPoint { point: Vec3::new(15.0, 25.0, 35.0) });
    mesh.wedges.push(PskWedge { point_index: 0, u: 0.25, v: 0.75, material_index: 0 });
    mesh.faces.push(PskFace { wedge_indices: [0, 0, 0], mat_index: 0, aux_mat_index: 0, smoothing_groups: 1 });
    mesh.materials.push(PskMaterial { name: "Soldier_Chest".to_string(), texture_index: 0, poly_flags: 0 });
    mesh.bones.push(PskBone {
        name: "Pelvis".to_string(),
        flags: 0,
        num_children: 0,
        parent_index: -1,
        orientation: glam::Quat::IDENTITY,
        position: Vec3::new(0.0, 0.0, 40.0),
        length: 12.0,
        size: Vec3::ONE,
    });
    mesh.weights.push(PskWeight { weight: 1.0, point_index: 0, bone_index: 0 });

    let bytes = mesh.to_bytes();
    assert!(bytes.len() >= 32 * 7);

    let parsed = ActorXPskMesh::from_bytes(&bytes).expect("ActorX PSK bytes must parse successfully");
    assert_eq!(parsed.points.len(), 1);
    assert_eq!(parsed.points[0].point, Vec3::new(15.0, 25.0, 35.0));
    assert_eq!(parsed.bones[0].name, "Pelvis");
    assert_eq!(parsed.materials[0].name, "Soldier_Chest");
}

#[test]
fn test_canonical_humanoid_hitbox_damage_system() {
    let humanoid = SkeletalMesh::create_canonical_ut99_humanoid();
    assert_eq!(humanoid.hitboxes.len(), 7);

    // 1. Headshot test (Z = 94.0)
    let head_ray_orig = Vec3::new(0.0, 80.0, 94.0);
    let head_ray_dir = Vec3::new(0.0, -1.0, 0.0);
    let head_hit = humanoid.test_hitbox_ray(head_ray_orig, head_ray_dir).expect("Sniper ray must hit head hitbox");
    assert_eq!(head_hit.hitbox_name, "Head");
    assert_eq!(head_hit.bone_name, "Head");
    assert!(head_hit.is_critical_headshot);
    assert!((head_hit.damage_multiplier - 2.5).abs() < 1e-3);

    // 2. Torso body shot test (Z = 72.0)
    let torso_ray_orig = Vec3::new(0.0, 80.0, 72.0);
    let torso_ray_dir = Vec3::new(0.0, -1.0, 0.0);
    let torso_hit = humanoid.test_hitbox_ray(torso_ray_orig, torso_ray_dir).expect("Ray must hit Torso hitbox");
    assert_eq!(torso_hit.hitbox_name, "Torso");
    assert!(!torso_hit.is_critical_headshot);
    assert!((torso_hit.damage_multiplier - 1.0).abs() < 1e-3);

    // 3. Miss test (Aiming into empty space)
    let miss_ray_orig = Vec3::new(100.0, 100.0, 200.0);
    let miss_ray_dir = Vec3::new(0.0, -1.0, 0.0);
    assert!(humanoid.test_hitbox_ray(miss_ray_orig, miss_ray_dir).is_none());
}

#[test]
fn test_skeletal_mesh_export_to_actorx_psk() {
    let humanoid = SkeletalMesh::create_canonical_ut99_humanoid();
    let psk_bytes = humanoid.to_actorx_psk();

    assert!(!psk_bytes.is_empty());
    assert_eq!(&psk_bytes[0..8], b"ACTRHEAD");

    let parsed = ActorXPskMesh::from_bytes(&psk_bytes).expect("Generated ActorX PSK should parse back cleanly");
    assert_eq!(parsed.bones.len(), humanoid.bones.len());
    assert_eq!(parsed.bones[0].name, "Root");
}
