//! Test Suite for Milestone //! SkyBox 3D Real-time Parallax & Multi-Zone Environmental Studio.

use glam::Vec3;
use ut99_bsp::{PF_FAKE_BACKDROP, PF_UNLIT};
use ut99_editor::editor_state::{EditorState, ZonePhysicsPreset};
use ut99_format::{ActorRecord, TaggedProperty, TaggedValue, UnrLevel};

#[test]
fn test_skyzone_detection_and_anchor_placement_with_undo() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());

    // 1. Initial level has no SkyZoneInfo
    assert!(state.find_active_sky_zone().is_none());

    // 2. Set builder brush location and place SkyZoneInfo
    state.builder_brush.location = Vec3::new(4096.0, -8192.0, 1024.0);
    let name_res = state.place_sky_zone_anchor_at_pivot();
    assert!(name_res.is_ok());

    let (idx, pos, tag) = state.find_active_sky_zone().expect("SkyZoneInfo should be detected");
    assert_eq!(pos, Vec3::new(4096.0, -8192.0, 1024.0));
    assert_eq!(tag, "SkyZone");

    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors[idx].class_name, "SkyZoneInfo");

    // 3. Test Undo
    state.undo();
    assert!(state.find_active_sky_zone().is_none(), "Undo should remove the added SkyZoneInfo");

    // 4. Test Redo
    state.redo();
    assert!(state.find_active_sky_zone().is_some(), "Redo should restore the SkyZoneInfo");
}

#[test]
fn test_skybox_camera_calculation_and_parallax() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    let mut sky_actor = ActorRecord::default();
    sky_actor.name = "SkyZoneInfo0".into();
    sky_actor.class_name = "SkyZoneInfo".into();
    sky_actor.tag = "SkyZone".into();
    sky_actor.location = Vec3::new(5000.0, 5000.0, -2000.0);
    level.actors.push(sky_actor);

    state.loaded_unr = Some(level);

    let mut main_cam = state.camera.clone();
    main_cam.position = Vec3::new(200.0, 400.0, 100.0);
    main_cam.yaw = 0.85;
    main_cam.pitch = -0.25;

    // Case A: Parallax scale = 0.0 (fixed infinite horizon)
    state.sky_zone_settings.parallax_scale = 0.0;
    state.sky_zone_settings.custom_anchor = Some(Vec3::ZERO);

    let sky_cam_a = state.calculate_skybox_camera(&main_cam).expect("Sky camera must be computed");
    assert_eq!(sky_cam_a.position, Vec3::new(5000.0, 5000.0, -2000.0));
    assert_eq!(sky_cam_a.yaw, 0.85);
    assert_eq!(sky_cam_a.pitch, -0.25);

    // Case B: Parallax scale = 0.1 (10% mountain/distant cloud parallax)
    state.sky_zone_settings.parallax_scale = 0.1;
    let sky_cam_b = state.calculate_skybox_camera(&main_cam).unwrap();
    let expected_pos = Vec3::new(5000.0 + 20.0, 5000.0 + 40.0, -2000.0 + 10.0);
    assert!((sky_cam_b.position - expected_pos).length() < 0.001);

    // Case C: Disabled
    state.sky_zone_settings.enabled = false;
    assert!(state.calculate_skybox_camera(&main_cam).is_none());
}

#[test]
fn test_mark_selected_surfaces_as_sky_with_undo_redo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    let mut brush = ut99_bsp::build_cube("TestCube", ut99_bsp::CsgOperation::Add, Vec3::splat(64.0));
    brush.location = Vec3::ZERO;
    for poly in &mut brush.polys {
        poly.flags = 0;
    }
    state.world_brushes.push(brush);

    // Select poly 1 on brush 0
    state.selected_surfaces.insert((0, 1));

    // Mark as sky backdrop
    let modified = state.mark_selected_surfaces_as_sky(true);
    assert_eq!(modified, 1);

    let brush = &state.world_brushes[0];
    assert_eq!(brush.polys[0].flags, 0);
    assert!((brush.polys[1].flags & PF_FAKE_BACKDROP) != 0);
    assert!((brush.polys[1].flags & PF_UNLIT) != 0);

    // Test Undo
    state.undo();
    assert_eq!(state.world_brushes[0].polys[1].flags, 0);

    // Test Redo
    state.redo();
    assert!((state.world_brushes[0].polys[1].flags & PF_FAKE_BACKDROP) != 0);

    // Unmark
    state.mark_selected_surfaces_as_sky(false);
    assert_eq!(state.world_brushes[0].polys[1].flags & PF_FAKE_BACKDROP, 0);
}

#[test]
fn test_zone_profiles_collection_and_physics_presets_with_undo() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // 1. ZoneInfo
    let mut zone1 = ActorRecord::default();
    zone1.name = "ZoneInfo0".into();
    zone1.class_name = "ZoneInfo".into();
    zone1.tag = "MainArenaZone".into();
    zone1.properties.insert(
        "ZoneGravity".into(),
        TaggedProperty {
            name: "ZoneGravity".into(),
            array_index: 0,
            value: TaggedValue::Vector(Vec3::new(0.0, 0.0, -950.0)),
        },
    );

    // 2. WaterZone
    let mut water_zone = ActorRecord::default();
    water_zone.name = "WaterZone0".into();
    water_zone.class_name = "WaterZone".into();
    water_zone.tag = "AcidPool".into();
    water_zone.properties.insert(
        "bWaterZone".into(),
        TaggedProperty {
            name: "bWaterZone".into(),
            array_index: 0,
            value: TaggedValue::Bool(true),
        },
    );

    level.actors.push(zone1);
    level.actors.push(water_zone);
    state.loaded_unr = Some(level);

    // Collect profiles
    let profiles = state.collect_level_zone_profiles();
    assert_eq!(profiles.len(), 2);

    let p_main = profiles.iter().find(|p| p.actor_tag == "MainArenaZone").unwrap();
    assert!(!p_main.is_water_zone);
    assert_eq!(p_main.gravity.z, -950.0);

    let p_water = profiles.iter().find(|p| p.actor_tag == "AcidPool").unwrap();
    assert!(p_water.is_water_zone);

    // Apply ToxicSlime preset to AcidPool
    let res = state.apply_zone_physics_preset("AcidPool", ZonePhysicsPreset::ToxicSlime);
    assert!(res.is_ok());

    let updated_profiles = state.collect_level_zone_profiles();
    let p_acid = updated_profiles.iter().find(|p| p.actor_tag == "AcidPool").unwrap();
    assert_eq!(p_acid.damage_per_sec, 10);
    assert_eq!(p_acid.damage_type, "Corroded");
    assert_eq!(p_acid.fluid_friction, 1.5);
    assert!(p_acid.is_water_zone);

    // Test Undo
    state.undo();
    let reverted_profiles = state.collect_level_zone_profiles();
    let p_reverted = reverted_profiles.iter().find(|p| p.actor_tag == "AcidPool").unwrap();
    assert_eq!(p_reverted.damage_per_sec, 0);

    // Test Redo
    state.redo();
    let redone_profiles = state.collect_level_zone_profiles();
    let p_redone = redone_profiles.iter().find(|p| p.actor_tag == "AcidPool").unwrap();
    assert_eq!(p_redone.damage_per_sec, 10);
}
