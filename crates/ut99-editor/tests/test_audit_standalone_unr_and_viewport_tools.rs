use glam::Vec3;
use std::fs;
use std::path::PathBuf;
use ut99_bsp::{Brush, CsgOperation, LevelDoctor, Polygon, PF_PORTAL};
use ut99_editor::editor_state::EditorState;
use ut99_format::{Package, RawPoly, RawPolys, UnrLevel};

#[test]
fn test_raw_polys_binary_roundtrip() {
    let mut original = RawPolys::default();
    original.polys.push(RawPoly {
        base: Vec3::new(10.0, 20.0, 30.0),
        normal: Vec3::Z,
        texture_u: Vec3::X,
        texture_v: Vec3::Y,
        vertices: vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(256.0, 0.0, 0.0),
            Vec3::new(256.0, 256.0, 0.0),
            Vec3::new(0.0, 256.0, 0.0),
        ],
        poly_flags: PF_PORTAL,
        actor_ref: 0,
        texture_ref: 0,
        item_name: 0,
        link_index: 0,
        brush_poly_index: 1,
        pan_u: 16,
        pan_v: -32,
    });

    let bytes = original.to_bytes().expect("Failed to serialize RawPolys to bytes");
    assert!(!bytes.is_empty(), "Bytes should not be empty");

    let deserialized = RawPolys::from_bytes(&bytes).expect("Failed to deserialize RawPolys");
    assert_eq!(deserialized.polys.len(), 1);
    let p = &deserialized.polys[0];
    assert_eq!(p.vertices.len(), 4);
    assert_eq!(p.poly_flags, PF_PORTAL);
    assert_eq!(p.pan_u, 16);
    assert_eq!(p.pan_v, -32);
    assert_eq!(p.normal, Vec3::Z);
    assert_eq!(p.base, Vec3::new(10.0, 20.0, 30.0));
}

#[test]
fn test_package_to_bytes_roundtrip() {
    let mut pkg = Package::default();
    pkg.summary.file_version = 68;
    pkg.summary.guid = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    let n_none = pkg.names.add("None", 0);
    let n_test = pkg.names.add("TestObject", 0);
    let n_class = pkg.names.add("Class", 0);
    let n_engine = pkg.names.add("Engine", 0);

    let imp_pkg = pkg.add_import(0, n_class as i32, 0, n_engine as i32);
    let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let exp_ref = pkg.add_export(n_test as i32, imp_pkg, 0, 0, 0x00000004, payload.clone());
    assert_eq!(exp_ref, 1);

    let pkg_bytes = pkg.to_bytes().expect("Package::to_bytes failed");
    assert!(!pkg_bytes.is_empty());

    let reloaded = Package::from_bytes(pkg_bytes).expect("Package::from_bytes failed");
    assert_eq!(reloaded.summary.file_version, 68);
    assert_eq!(reloaded.names.get(n_none).unwrap(), "None");
    assert_eq!(reloaded.names.get(n_test).unwrap(), "TestObject");
    assert_eq!(reloaded.imports.entries.len(), 1);
    assert_eq!(reloaded.exports.entries.len(), 1);

    let reloaded_payload = reloaded.get_export_bytes(0).expect("Export payload missing");
    assert_eq!(reloaded_payload, &payload[..]);
}

#[test]
fn test_standalone_native_unr_export_and_reopen() {
    let mut state = EditorState::default();
    // Default state starts with 1 room brush. Let's add a light and a player start.
    state.add_light_at_builder();
    state.add_player_start_at_builder();
    state.add_mover_from_builder();

    let temp_unr = PathBuf::from(std::env::temp_dir()).join("test_rea_native_export.unr");
    if temp_unr.exists() {
        let _ = fs::remove_file(&temp_unr);
    }

    let saved_path = state.save_unr_native(&temp_unr).expect("save_unr_native failed");
    assert!(saved_path.exists());
    let metadata = fs::metadata(&saved_path).expect("Failed to get saved file metadata");
    assert!(metadata.len() > 100, "File should have non-trivial size");

    // Reopen using UnrLevel::open
    let loaded = UnrLevel::open(&saved_path).expect("Failed to load back native .unr level");
    assert_eq!(loaded.map_name, "test_rea_native_export");
    assert!(!loaded.brushes.is_empty(), "Level should contain saved brushes");
    assert!(!loaded.lights.is_empty(), "Level should contain saved lights");
    assert!(!loaded.player_starts.is_empty(), "Level should contain saved player starts");

    // Clean up
    let _ = fs::remove_file(&saved_path);
    let bak = saved_path.with_extension("unr.bak");
    if bak.exists() {
        let _ = fs::remove_file(bak);
    }
}

#[test]
fn test_in_viewport_uv_drag_and_undo() {
    let mut state = EditorState::default();
    assert!(!state.world_brushes.is_empty());
    state.selected_brush_index = Some(0);

    let initial_base = state.world_brushes[0].polys[0].base;

    // Pan texture UV
    state.pan_selected_brush_texture(64.0, 32.0);
    let panned_base = state.world_brushes[0].polys[0].base;
    assert_ne!(initial_base, panned_base, "Texture base should change after panning");

    // Scale texture UV
    let initial_u = state.world_brushes[0].polys[0].texture_u;
    state.scale_selected_brush_texture(2.0, 2.0);
    let scaled_u = state.world_brushes[0].polys[0].texture_u;
    assert_ne!(initial_u, scaled_u, "Texture U vector should change after scaling");

    // Undo scaling
    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].texture_u, initial_u, "Undo should restore U vector");

    // Undo panning
    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].base, initial_base, "Undo should restore base coordinates");
}

#[test]
fn test_zone_portal_diagnostics_and_visualizer() {
    let mut brush = Brush::new("PortalSheet", CsgOperation::Add);
    let portal_poly = Polygon::new(vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(128.0, 0.0, 0.0),
        Vec3::new(128.0, 0.0, 256.0),
        Vec3::new(0.0, 0.0, 256.0),
    ])
    .unwrap()
    .with_flags(PF_PORTAL);
    brush.polys.push(portal_poly);

    let issues = LevelDoctor::audit_zone_portals(&[brush]);
    assert!(
        issues.iter().any(|i| i.category == "ZonePortals" && i.description.contains("Detected 1 zone portal")),
        "Should report detection of active zone portal"
    );

    // Now test a non-planar / degenerate portal
    let mut bad_brush = Brush::new("BadPortal", CsgOperation::Add);
    let bad_poly = Polygon {
        vertices: vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(128.0, 0.0, 0.0),
            Vec3::new(128.0, 50.0, 256.0), // warped out of plane
            Vec3::new(0.0, 0.0, 256.0),
        ],
        plane: ut99_bsp::math::Plane { normal: Vec3::Y, dist: 0.0 },
        texture: String::new(),
        flags: PF_PORTAL,
        base: Vec3::ZERO,
        texture_u: Vec3::X,
        texture_v: Vec3::Z,
        pan_u: 0,
        pan_v: 0,
        vertex_colors: Vec::new(),
        lightmap: None,
    };
    bad_brush.polys.push(bad_poly);

    let bad_issues = LevelDoctor::audit_zone_portals(&[bad_brush]);
    assert!(
        bad_issues.iter().any(|i| i.category == "ZonePortals" && i.description.contains("non-planar")),
        "Should flag warning for non-planar zone portal"
    );
}

#[test]
fn test_save_unr_native_from_t3d_map() {
    let mut state = EditorState::default();
    state.loaded_unr = None; // simulate T3D-only loaded map
    state.world_brushes.clear();

    // Add a cube world brush
    let mut cube = ut99_bsp::build_cube("MainHall", CsgOperation::Add, Vec3::new(512.0, 512.0, 256.0));
    cube.location = Vec3::new(0.0, 0.0, 128.0);
    state.world_brushes.push(cube);

    // Populate state.map with T3D actors
    let mut t3d_light = ut99_format::t3d::T3dActor::new("Light", "Light1");
    t3d_light.properties.insert("Location".to_string(), "(X=100.0,Y=100.0,Z=200.0)".to_string());
    t3d_light.properties.insert("LightBrightness".to_string(), "128".to_string());
    t3d_light.properties.insert("LightRadius".to_string(), "64".to_string());
    state.map.actors.push(t3d_light);

    let mut t3d_start = ut99_format::t3d::T3dActor::new("PlayerStart", "PlayerStart1");
    t3d_start.properties.insert("Location".to_string(), "(X=0.0,Y=0.0,Z=50.0)".to_string());
    state.map.actors.push(t3d_start);

    let mut t3d_path = ut99_format::t3d::T3dActor::new("PathNode", "PathNode1");
    t3d_path.properties.insert("Location".to_string(), "(X=200.0,Y=200.0,Z=50.0)".to_string());
    state.map.actors.push(t3d_path);

    let temp_dir = std::env::temp_dir();
    let temp_unr = temp_dir.join("test_t3d_native_save.unr");

    let saved_path = ut99_editor::compiler::CompilerBridge::save_unr_native(&state, &temp_unr)
        .expect("Native UNR save from T3D must succeed");
    assert!(saved_path.exists());
    assert!(std::fs::metadata(&saved_path).unwrap().len() > 0);

    // Load back and verify actor preservation
    let pkg = Package::open(&saved_path).expect("Must parse saved UNR package");
    let loaded_lvl = UnrLevel::from_package(pkg, "TestT3DMap".to_string())
        .expect("Must deserialize UnrLevel");

    assert_eq!(loaded_lvl.brushes.len(), 1, "Should have 1 world brush");
    assert_eq!(loaded_lvl.lights.len(), 1, "Should preserve Light actor from T3D");
    assert_eq!(loaded_lvl.player_starts.len(), 1, "Should preserve PlayerStart actor from T3D");
    assert_eq!(loaded_lvl.path_nodes.len(), 1, "Should preserve PathNode actor from T3D");

    let _ = std::fs::remove_file(&saved_path);
}
