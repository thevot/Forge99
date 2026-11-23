use glam::Vec3;
use ut99_bsp::{build_cube, CsgOperation};
use ut99_editor::history::Action;
use ut99_editor::{CompilerBridge, EditorState};
use ut99_format::{parse_t3d, write_t3d};
mod common;

#[test]
fn test_aud001_movers_and_static_meshes_t3d_export_and_roundtrip() {
    let mut state = EditorState::default();
    state.new_map();

    // 1. Create a mover from builder brush
    state.builder_brush.location = Vec3::new(250.0, 100.0, 50.0);
    state.add_mover_from_builder();
    assert_eq!(state.movers.len(), 1);

    // Customize mover properties
    state.movers[0].tag = "SecretDoorTag".into();
    state.movers[0].event = "TriggeredEvent".into();
    state.movers[0].stay_open_time = 6.5;
    state.movers[0].move_time = 2.0;

    // 2. Import an OBJ Static Mesh
    let obj_source = r#"
o WoodenCrate
v -16.0 -16.0 0.0
v  16.0 -16.0 0.0
v  16.0  16.0 0.0
v -16.0  16.0 0.0
v -16.0 -16.0 32.0
v  16.0 -16.0 32.0
v  16.0  16.0 32.0
v -16.0  16.0 32.0
vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0
vn 0.0 0.0 1.0
usemtl Indus1.Indus_B01
f 1/1/1 2/2/1 3/3/1 4/4/1
"#;
    let crate_pos = Vec3::new(300.0, 300.0, 0.0);
    let import_res = state.import_obj_as_static_mesh("Crate01", obj_source, crate_pos);
    assert!(import_res.is_ok());
    assert_eq!(state.static_meshes.len(), 1);

    // 3. Export to T3D map structure
    let t3d_map = CompilerBridge::export_t3d_map(&state);

    // Verify Mover presence and properties
    let mover_actor = t3d_map
        .actors
        .iter()
        .find(|a| a.class == "Mover")
        .expect("T3D export must include Mover actor!");
    assert_eq!(mover_actor.name, "Mover0");
    assert_eq!(mover_actor.get_property("CsgOper"), Some("CSG_Active"));
    assert_eq!(mover_actor.get_property("Tag"), Some("\"SecretDoorTag\""));
    assert_eq!(mover_actor.get_property("Event"), Some("\"TriggeredEvent\""));
    assert_eq!(mover_actor.get_property("StayOpenTime"), Some("6.500000"));
    assert_eq!(mover_actor.get_property("MoveTime"), Some("2.000000"));
    assert!(
        mover_actor.brush.is_some(),
        "Mover must contain a valid T3dBrush geometry!"
    );
    let mover_brush = mover_actor.brush.as_ref().unwrap();
    assert!(!mover_brush.polys.is_empty(), "Mover brush polys must not be empty");

    // Verify Static Mesh presence as additive/semi-solid Brush
    let sm_actor = t3d_map
        .actors
        .iter()
        .find(|a| a.name == "Crate01")
        .expect("T3D export must include StaticMesh Brush!");
    assert_eq!(sm_actor.class, "Brush");
    assert_eq!(sm_actor.get_property("CsgOper"), Some("CSG_Add"));
    assert_eq!(sm_actor.get_property("PolyFlags"), Some("32")); // Canonical UE1 PF_SEMISOLID = 0x0000_0020 (32)
    assert!(
        sm_actor.brush.is_some(),
        "StaticMesh actor must contain a valid T3dBrush geometry!"
    );

    // 4. Test Text Serialization & Roundtrip Parsing
    let mut serialized_t3d = Vec::new();
    write_t3d(&mut serialized_t3d, &t3d_map).expect("write_t3d should succeed");
    let t3d_text = String::from_utf8(serialized_t3d).expect("T3D text should be UTF-8");

    assert!(t3d_text.contains("Begin Actor Class=Mover Name=Mover0"));
    assert!(t3d_text.contains("Begin Actor Class=Brush Name=Crate01"));
    assert!(t3d_text.contains("Tag=\"SecretDoorTag\""));
    assert!(t3d_text.contains("Event=\"TriggeredEvent\""));

    let re_parsed = parse_t3d(t3d_text.as_bytes()).expect("parse_t3d should parse exported T3D");
    let parsed_mover = re_parsed
        .actors
        .iter()
        .find(|a| a.class == "Mover")
        .expect("Parsed map must preserve Mover");
    assert_eq!(parsed_mover.name, "Mover0");
    assert!(parsed_mover.brush.is_some());
}

#[test]
fn test_aud001_save_t3d_exports_authoritative_live_state() {
    let mut state = EditorState::default();
    state.new_map();

    // Add a box brush
    let box_brush = build_cube("CustomBox", CsgOperation::Add, Vec3::new(128.0, 128.0, 128.0));
    state.world_brushes.push(box_brush);

    // Add a mover
    state.builder_brush.location = Vec3::new(50.0, 50.0, 50.0);
    state.add_mover_from_builder();

    let temp_dir = std::env::temp_dir();
    let temp_t3d_path = temp_dir.join("test_authoritative_export.t3d");

    let save_res = state.save_t3d(&temp_t3d_path);
    assert!(save_res.is_ok(), "save_t3d should succeed");

    let file_content = std::fs::read_to_string(&temp_t3d_path).expect("Read saved file");
    assert!(file_content.contains("Begin Actor Class=Brush Name=CustomBox"));
    assert!(file_content.contains("Begin Actor Class=Mover Name=Mover0"));

    // Cleanup
    let _ = std::fs::remove_file(temp_t3d_path);
}

#[test]
fn test_aud002_expanded_undo_redo_suite() {
    let mut state = EditorState::default();
    state.new_map();

    let initial_brush_count = state.world_brushes.len();
    assert_eq!(initial_brush_count, 1);

    // 1. Test RotateBrush Undo/Redo
    let old_rot = state.world_brushes[0].rotation;
    let new_rot = Vec3::new(0.0, 90.0, 0.0);
    state.history.push(Action::RotateBrush {
        brush_index: 0,
        old_rotation: old_rot,
        new_rotation: new_rot,
    });
    state.world_brushes[0].rotation = new_rot;
    assert_eq!(state.world_brushes[0].rotation, new_rot);

    state.undo();
    assert_eq!(state.world_brushes[0].rotation, old_rot);
    state.redo();
    assert_eq!(state.world_brushes[0].rotation, new_rot);

    // 2. Test AddMover and Undo/Redo
    let mover_count_before = state.movers.len();
    state.builder_brush.location = Vec3::new(10.0, 20.0, 30.0);
    state.add_mover_from_builder();
    assert_eq!(state.movers.len(), mover_count_before + 1);

    state.undo();
    assert_eq!(state.movers.len(), mover_count_before);
    state.redo();
    assert_eq!(state.movers.len(), mover_count_before + 1);

    // 3. Test MoveMover Undo/Redo
    let old_pos = state.movers[0].brush.location;
    let moved_pos = old_pos + Vec3::new(100.0, 0.0, 0.0);
    state.history.push(Action::MoveMover {
        mover_index: 0,
        old_location: old_pos,
        new_location: moved_pos,
    });
    state.movers[0].brush.location = moved_pos;
    assert_eq!(state.movers[0].brush.location, moved_pos);

    state.undo();
    assert_eq!(state.movers[0].brush.location, old_pos);
    state.redo();
    assert_eq!(state.movers[0].brush.location, moved_pos);

    // 4. Test ModifyBrushGeometry (Apply texture / vertex edit) Undo/Redo
    state.selected_brush_index = Some(0);
    state.active_texture = Some("Indus1.Indus_Met01".to_string());
    let orig_tex = state.world_brushes[0].polys[0].texture.clone();

    state.apply_active_texture_to_selected_brush();
    assert_eq!(
        state.world_brushes[0].polys[0].texture,
        "Indus1.Indus_Met01"
    );

    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].texture, orig_tex);
    state.redo();
    assert_eq!(
        state.world_brushes[0].polys[0].texture,
        "Indus1.Indus_Met01"
    );

    // 5. Test AddStaticMesh Undo/Redo
    let sm_count_before = state.static_meshes.len();
    let obj_box = r#"
o SmallBox
v -5.0 -5.0 0.0
v  5.0 -5.0 0.0
v  5.0  5.0 0.0
v -5.0  5.0 0.0
f 1 2 3 4
"#;
    let _ = state.import_obj_as_static_mesh("SmallBox01", obj_box, Vec3::ZERO);
    assert_eq!(state.static_meshes.len(), sm_count_before + 1);

    state.undo();
    assert_eq!(state.static_meshes.len(), sm_count_before);
    state.redo();
    assert_eq!(state.static_meshes.len(), sm_count_before + 1);
}

#[test]
fn test_aud003_native_save_reports_invalid_destination() {
    let state = EditorState::default();
    let dir = common::TestDir::new();
    let parent_file = dir.path().join("not-a-directory");
    std::fs::write(&parent_file, b"keep").unwrap();
    assert!(CompilerBridge::save_unr_native(&state, &parent_file.join("map.unr")).is_err());
    assert_eq!(std::fs::read(parent_file).unwrap(), b"keep");
}
