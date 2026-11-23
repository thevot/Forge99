//! Test Suite for Milestone //! 3D Acoustic & Ambient Sound Studio (Attenuation & Occlusion Profiler).

use glam::Vec3;
use ut99_bsp::{build_cube, CsgOperation};
use ut99_editor::editor_state::EditorState;
use ut99_format::{ActorRecord, TaggedProperty, TaggedValue, UnrLevel};

#[test]
fn test_sound_emitter_scanning_and_radius_conversion() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // 1. AmbientSound actor
    let mut amb = ActorRecord::default();
    amb.name = "AmbientSound0".into();
    amb.class_name = "AmbientSound".into();
    amb.tag = "WaterDripLoop".into();
    amb.location = Vec3::new(100.0, 200.0, 50.0);
    amb.properties.insert(
        "AmbientSound".into(),
        TaggedProperty {
            name: "AmbientSound".into(),
            array_index: 0,
            value: TaggedValue::Name("AmbAncient.Loop.Drip".into()),
        },
    );
    amb.properties.insert(
        "SoundRadius".into(),
        TaggedProperty {
            name: "SoundRadius".into(),
            array_index: 0,
            value: TaggedValue::Byte(32), // 32 * 40.0 = 1280.0 uu
        },
    );
    amb.properties.insert(
        "SoundVolume".into(),
        TaggedProperty {
            name: "SoundVolume".into(),
            array_index: 0,
            value: TaggedValue::Byte(128),
        },
    );
    amb.properties.insert(
        "SoundPitch".into(),
        TaggedProperty {
            name: "SoundPitch".into(),
            array_index: 0,
            value: TaggedValue::Byte(64),
        },
    );

    // 2. Weapon with ambient sound
    let mut wep = ActorRecord::default();
    wep.name = "ShockRifle0".into();
    wep.class_name = "ShockRifle".into();
    wep.tag = "ShockRifle_Spawn".into();
    wep.location = Vec3::new(-500.0, 0.0, 0.0);
    wep.properties.insert(
        "AmbientSound".into(),
        TaggedProperty {
            name: "AmbientSound".into(),
            array_index: 0,
            value: TaggedValue::Name("Weapons.ShockHum".into()),
        },
    );
    wep.properties.insert(
        "SoundRadius".into(),
        TaggedProperty {
            name: "SoundRadius".into(),
            array_index: 0,
            value: TaggedValue::Byte(16), // 16 * 40.0 = 640.0 uu
        },
    );
    wep.properties.insert(
        "SoundVolume".into(),
        TaggedProperty {
            name: "SoundVolume".into(),
            array_index: 0,
            value: TaggedValue::Byte(255),
        },
    );

    level.actors.push(amb);
    level.actors.push(wep);
    state.loaded_unr = Some(level);

    let emitters = state.collect_sound_emitters();
    assert_eq!(emitters.len(), 2);

    let e_amb = emitters.iter().find(|e| e.actor_tag == "WaterDripLoop").unwrap();
    assert_eq!(e_amb.sound_name, "AmbAncient.Loop.Drip");
    assert_eq!(e_amb.sound_radius_byte, 32);
    assert_eq!(e_amb.sound_radius_units, 1280.0);
    assert!((e_amb.sound_volume_pct - (128.0 / 255.0)).abs() < 0.01);
    assert_eq!(e_amb.sound_pitch_ratio, 1.0);
    assert!(e_amb.is_ambient);

    let e_wep = emitters.iter().find(|e| e.actor_tag == "ShockRifle_Spawn").unwrap();
    assert_eq!(e_wep.sound_radius_units, 640.0);
    assert_eq!(e_wep.sound_volume_byte, 255);
}

#[test]
fn test_place_ambient_sound_at_pivot_with_undo_redo() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());

    state.builder_brush.location = Vec3::new(1024.0, 512.0, 256.0);
    let res = state.place_ambient_sound_at_pivot("AmbIndustrial.Machine.Gen1", 48, 200);
    assert!(res.is_ok());

    let emitters = state.collect_sound_emitters();
    assert_eq!(emitters.len(), 1);
    let e = &emitters[0];
    assert_eq!(e.location, Vec3::new(1024.0, 512.0, 256.0));
    assert_eq!(e.sound_name, "AmbIndustrial.Machine.Gen1");
    assert_eq!(e.sound_radius_byte, 48);
    assert_eq!(e.sound_radius_units, 48.0 * 40.0); // 1920.0 uu
    assert_eq!(e.sound_volume_byte, 200);

    // Test Undo
    state.undo();
    assert_eq!(state.collect_sound_emitters().len(), 0);

    // Test Redo
    state.redo();
    assert_eq!(state.collect_sound_emitters().len(), 1);
}

#[test]
fn test_update_sound_emitter_properties_with_undo_redo() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());

    state.place_ambient_sound_at_pivot("InitialSound", 20, 100).unwrap();

    let tag = state.collect_sound_emitters()[0].actor_tag.clone();

    // Update properties
    let upd_res = state.update_sound_emitter_properties(&tag, "ModifiedSound.Loop", 64, 255, 32);
    assert!(upd_res.is_ok());

    let emitters = state.collect_sound_emitters();
    let e = &emitters[0];
    assert_eq!(e.sound_name, "ModifiedSound.Loop");
    assert_eq!(e.sound_radius_byte, 64);
    assert_eq!(e.sound_radius_units, 2560.0);
    assert_eq!(e.sound_volume_byte, 255);
    assert_eq!(e.sound_pitch_ratio, 0.5);

    // Test Undo
    state.undo();
    let reverted = state.collect_sound_emitters();
    assert_eq!(reverted[0].sound_name, "InitialSound");
    assert_eq!(reverted[0].sound_radius_byte, 20);

    // Test Redo
    state.redo();
    let redone = state.collect_sound_emitters();
    assert_eq!(redone[0].sound_name, "ModifiedSound.Loop");
    assert_eq!(redone[0].sound_radius_byte, 64);
}

#[test]
fn test_acoustic_occlusion_and_wall_attenuation() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());
    state.world_brushes.clear();

    // Place sound emitter at origin (0, 0, 0)
    state.builder_brush.location = Vec3::ZERO;
    state.place_ambient_sound_at_pivot("TestAmb", 64, 255).unwrap(); // Radius = 2560.0 uu
    let tag = state.collect_sound_emitters()[0].actor_tag.clone();

    // Case 1: Listener in direct line-of-sight at distance 500 uu (no walls)
    let listener_clear = Vec3::new(500.0, 0.0, 0.0);
    let occ_clear = state
        .compute_acoustic_occlusion_for_emitter(&tag, listener_clear)
        .expect("Occlusion should be computed");

    assert!(occ_clear.is_in_range);
    assert_eq!(occ_clear.walls_occluding_count, 0);
    assert_eq!(occ_clear.occlusion_attenuation_db, 0.0);
    assert_eq!(occ_clear.low_pass_cutoff_hz, 22000.0);
    assert!(occ_clear.direct_volume_db > -20.0);

    // Case 2: Place a solid wall at X = 250 (between origin and listener at X = 500)
    let mut wall_brush = build_cube("SolidWall", CsgOperation::Add, Vec3::new(32.0, 500.0, 500.0));
    wall_brush.location = Vec3::new(250.0, 0.0, 0.0);
    state.world_brushes.push(wall_brush);

    let occ_occluded = state
        .compute_acoustic_occlusion_for_emitter(&tag, listener_clear)
        .expect("Occlusion should be computed");

    assert!(occ_occluded.is_in_range);
    assert!(occ_occluded.walls_occluding_count >= 1, "Must detect intersecting wall brush");
    assert!(occ_occluded.occlusion_attenuation_db >= 6.0, "Must attenuate by at least 6 dB per wall");
    assert!(occ_occluded.final_effective_volume_db < occ_clear.direct_volume_db);
    assert!(occ_occluded.low_pass_cutoff_hz < 22000.0, "Low-pass filter must absorb high frequencies");

    // Case 3: Listener out of range at distance 4000 uu (> 2560 uu)
    let listener_far = Vec3::new(4000.0, 0.0, 0.0);
    let occ_far = state.compute_acoustic_occlusion_for_emitter(&tag, listener_far).unwrap();
    assert!(!occ_far.is_in_range);
    assert_eq!(occ_far.direct_volume_db, -96.0);
    assert_eq!(occ_far.final_effective_volume_db, -96.0);
}
