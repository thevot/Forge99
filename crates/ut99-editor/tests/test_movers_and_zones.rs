use glam::Vec3;
use ut99_bsp::poly::*;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_mover_creation_and_keyframes() {
    let mut state = EditorState::default();
    assert_eq!(state.movers.len(), 0);

    // Place builder brush at (100, 200, 300)
    state.builder_brush.location = Vec3::new(100.0, 200.0, 300.0);

    // Add mover from builder brush
    state.add_mover_from_builder();
    assert_eq!(state.movers.len(), 1);
    assert_eq!(state.selected_mover_index, Some(0));

    let mover = &state.movers[0];
    assert_eq!(mover.name, "Mover0");
    assert_eq!(mover.current_key, 0);

    // Key 0 is at builder position
    let kf0 = mover.keyframes[0].as_ref().expect("Key 0 should be initialized");
    assert_eq!(kf0.location, Vec3::new(100.0, 200.0, 300.0));

    // Key 1 defaults to 128 units above
    let kf1 = mover.keyframes[1].as_ref().expect("Key 1 should be initialized");
    assert_eq!(kf1.location, Vec3::new(100.0, 200.0, 428.0));

    // Switch to Key 1
    state.set_selected_mover_key(1);
    assert_eq!(state.movers[0].current_key, 1);
    assert_eq!(state.movers[0].brush.location, Vec3::new(100.0, 200.0, 428.0));

    // Move to open position (e.g. sliding door 256 units to the right) and save
    let open_pos = Vec3::new(356.0, 200.0, 300.0);
    state.save_selected_mover_current_key_pos(open_pos, Vec3::ZERO);
    assert_eq!(state.movers[0].keyframes[1].as_ref().unwrap().location, open_pos);
    assert_eq!(state.movers[0].brush.location, open_pos);

    // Switch to new Key 2 (which should inherit from Key 0)
    state.set_selected_mover_key(2);
    assert_eq!(state.movers[0].current_key, 2);
    assert_eq!(state.movers[0].brush.location, Vec3::new(100.0, 200.0, 300.0));
}

#[test]
fn test_special_brush_presets_and_flags() {
    let mut state = EditorState::default();
    let initial_count = state.world_brushes.len();

    // 1. SemiSolid Brush (Pillars, architectural trim)
    state.add_special_brush(PF_SEMISOLID, false);
    assert_eq!(state.world_brushes.len(), initial_count + 1);
    let semi_brush = state.world_brushes.last().unwrap();
    assert_eq!(semi_brush.csg_oper, ut99_bsp::CsgOperation::Add);
    for poly in &semi_brush.polys {
        assert_eq!(poly.flags & PF_SEMISOLID, PF_SEMISOLID);
    }

    // 2. Zone Portal Brush (Cut sheets dividing zones)
    state.add_special_brush(PF_PORTAL | PF_TWO_SIDED, false);
    assert_eq!(state.world_brushes.len(), initial_count + 2);
    let portal_brush = state.world_brushes.last().unwrap();
    for poly in &portal_brush.polys {
        assert_eq!(poly.flags & (PF_PORTAL | PF_TWO_SIDED), PF_PORTAL | PF_TWO_SIDED);
    }

    // 3. Fake Backdrop Skybox Brush
    state.add_special_brush(PF_FAKE_BACKDROP | PF_UNLIT, false);
    assert_eq!(state.world_brushes.len(), initial_count + 3);
    let sky_brush = state.world_brushes.last().unwrap();
    for poly in &sky_brush.polys {
        assert_eq!(poly.flags & (PF_FAKE_BACKDROP | PF_UNLIT), PF_FAKE_BACKDROP | PF_UNLIT);
    }

    // 4. Non-Solid Decorative Sheet / Water
    state.add_special_brush(PF_NOT_SOLID | PF_TRANSLUCENT | PF_TWO_SIDED, false);
    assert_eq!(state.world_brushes.len(), initial_count + 4);
    let water_brush = state.world_brushes.last().unwrap();
    for poly in &water_brush.polys {
        assert_eq!(poly.flags & (PF_NOT_SOLID | PF_TRANSLUCENT | PF_TWO_SIDED), PF_NOT_SOLID | PF_TRANSLUCENT | PF_TWO_SIDED);
    }
}

#[test]
fn test_zone_and_skybox_actors() {
    let mut state = EditorState::default();

    // Add SkyZoneInfo
    state.builder_brush.location = Vec3::new(5000.0, 5000.0, -1000.0);
    state.add_sky_zone_info_at_builder();

    let unr = state.loaded_unr.as_ref().expect("Level should be created");
    let sky_actor = unr.actors.iter().find(|a| a.class_name == "SkyZoneInfo").expect("SkyZoneInfo should exist");
    assert_eq!(sky_actor.location, Vec3::new(5000.0, 5000.0, -1000.0));

    // Add WaterZone
    state.builder_brush.location = Vec3::new(0.0, 0.0, -200.0);
    state.add_water_zone_at_builder();
    let unr = state.loaded_unr.as_ref().unwrap();
    let water_actor = unr.actors.iter().find(|a| a.class_name == "WaterZone").expect("WaterZone should exist");
    assert_eq!(water_actor.location, Vec3::new(0.0, 0.0, -200.0));

    // Add ZoneInfo with tag
    state.builder_brush.location = Vec3::new(256.0, 256.0, 100.0);
    state.add_zone_info_at_builder("ToxicSlimeChamber");
    let unr = state.loaded_unr.as_ref().unwrap();
    let zone_actor = unr.actors.iter().find(|a| a.group == "ToxicSlimeChamber").expect("ZoneInfo with tag should exist");
    assert_eq!(zone_actor.class_name, "ZoneInfo");
    assert_eq!(zone_actor.location, Vec3::new(256.0, 256.0, 100.0));
}
