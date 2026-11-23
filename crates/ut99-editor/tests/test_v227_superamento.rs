use glam::Vec3;
use ut99_bsp::CsgOperation;
use ut99_editor::EditorState;

#[test]
fn test_static_mesh_obj_import_and_export() {
    let mut state = EditorState::default();

    let obj_source = r#"
# Test Pillar Mesh
o DecorativePillar
v -32.0 -32.0 0.0
v  32.0 -32.0 0.0
v  32.0  32.0 0.0
v -32.0  32.0 0.0
v -32.0 -32.0 256.0
v  32.0 -32.0 256.0
v  32.0  32.0 256.0
v -32.0  32.0 256.0

vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0

vn 0.0 0.0 -1.0
vn 0.0 0.0  1.0

usemtl Indus1.Indus_B01
f 1/1/1 2/2/1 3/3/1 4/4/1
f 5/1/2 6/2/2 7/3/2 8/4/2
"#;

    // 1. Import OBJ into Builder Brush
    let res = state.import_obj_to_builder(obj_source);
    assert!(res.is_ok(), "Importing valid OBJ should succeed");
    assert_eq!(state.builder_brush.polys.len(), 2);
    assert_eq!(state.builder_brush.polys[0].vertices.len(), 4);
    assert_eq!(state.builder_brush.polys[0].texture, "Indus1.Indus_B01");

    // 2. Import OBJ as StaticMesh Actor
    let sm_pos = Vec3::new(500.0, 500.0, 0.0);
    let sm_res = state.import_obj_as_static_mesh("Pillar01", obj_source, sm_pos);
    assert!(sm_res.is_ok(), "Importing StaticMesh should succeed");
    assert_eq!(state.static_meshes.len(), 1);
    assert_eq!(state.static_meshes[0].name, "Pillar01");
    assert_eq!(state.static_meshes[0].location, sm_pos);

    // 3. Export entire map geometry to Wavefront OBJ
    let exported_map_obj = state.export_map_to_obj("TestMapExport");
    assert!(exported_map_obj.contains("o TestMapExport"));
    assert!(exported_map_obj.contains("g Brush_"));
    assert!(exported_map_obj.contains("v "));
    assert!(exported_map_obj.contains("vt "));
    assert!(exported_map_obj.contains("vn "));
    assert!(exported_map_obj.contains("usemtl "));
    assert!(exported_map_obj.contains("f "));

    // 4. Export selected brush to OBJ
    state.selected_static_mesh_index = Some(0);
    let exported_brush_obj = state.export_selected_brush_to_obj();
    assert!(exported_brush_obj.is_some());
    assert!(exported_brush_obj.unwrap().contains("o Pillar01"));
}

#[test]
fn test_projector_decal_placement_and_baking() {
    let mut state = EditorState::default();

    // Clear world brushes and add a room wall facing -X at X = 200
    state.world_brushes.clear();
    let mut wall_brush = ut99_bsp::build_cube("WallBrush", CsgOperation::Add, Vec3::new(32.0, 512.0, 512.0));
    wall_brush.location = Vec3::new(200.0, 0.0, 0.0);
    state.world_brushes.push(wall_brush);

    // Place builder brush at origin facing +X (yaw = 0)
    state.builder_brush.location = Vec3::new(0.0, 0.0, 0.0);
    state.builder_brush.rotation = Vec3::ZERO;
    state.active_texture = Some("Botpack.BloodSplat".to_string());

    // Add Projector
    state.add_projector_at_builder();
    assert_eq!(state.projectors.len(), 1);
    assert_eq!(state.projectors[0].location, Vec3::ZERO);
    assert_eq!(state.projectors[0].texture, "Botpack.BloodSplat");

    // Decals are baked automatically upon addition
    assert!(
        !state.active_projector_decals.is_empty(),
        "Should generate at least 1 decal fragment on the wall"
    );

    let decal = &state.active_projector_decals[0];
    assert_eq!(decal.texture, "Botpack.BloodSplat");
    assert_eq!(decal.vertices.len(), 4);

    // Verify all vertices are slightly offset from the wall face (wall X min is around 200 - 16 = 184)
    for v in &decal.vertices {
        assert!(v.x > 180.0 && v.x < 185.0);
    }
}

#[test]
fn test_bot_navigation_graph_and_simulation() {
    let mut state = EditorState::default();

    // Rebuild paths
    state.rebuild_navigation_paths();

    assert!(
        state.nav_graph.nodes.len() >= 2,
        "Navigation network should have nodes"
    );
    assert!(
        !state.nav_graph.reach_specs.is_empty(),
        "Navigation network should have reach specs"
    );

    // Start simulation from node 0 to node 2
    let started = state.start_bot_simulation(0, 2);
    assert!(started, "Bot simulation should find path and start");
    assert!(state.bot_simulator.is_some());

    let bot = state.bot_simulator.as_ref().unwrap();
    assert!(!bot.reached_destination);
    assert_eq!(bot.path.len(), 3); // Node 0 -> Node 1 -> Node 2

    // Step simulation over multiple frames (2 seconds total at 400 uu/sec -> covers 800 uu, enough for 512 uu)
    for _ in 0..20 {
        state.step_bot_simulation(0.1);
    }

    let bot_after = state.bot_simulator.as_ref().unwrap();
    assert!(
        bot_after.reached_destination,
        "Bot should reach target node after 2.0s"
    );
}

#[test]
fn test_level_doctor_diagnostics_and_autofix() {
    let mut state = EditorState::default();

    // Add a brush with a degenerate poly and a missing texture poly
    let mut bad_brush = ut99_bsp::Brush::new("CorruptBrush", CsgOperation::Add);
    let p_valid_untextured = ut99_bsp::Polygon::new(vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(100.0, 100.0, 0.0),
    ])
    .unwrap();
    bad_brush.polys.push(p_valid_untextured);

    let p_degen = ut99_bsp::Polygon {
        vertices: vec![Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), Vec3::new(20.0, 0.0, 0.0)],
        plane: ut99_bsp::Plane {
            normal: Vec3::Z,
            dist: 0.0,
        },
        texture: String::new(),
        flags: 0,
        base: Vec3::ZERO,
        texture_u: Vec3::X,
        texture_v: Vec3::Y,
        pan_u: 0,
        pan_v: 0,
        vertex_colors: Vec::new(),
        lightmap: None,
    };
    bad_brush.polys.push(p_degen);
    state.world_brushes.push(bad_brush);

    // 1. Run audit
    state.run_level_doctor_audit();
    assert!(
        !state.map_audit_issues.is_empty(),
        "Level Doctor should detect errors"
    );

    let has_geom = state.map_audit_issues.iter().any(|i| i.category == "Geometry");
    let has_tex = state.map_audit_issues.iter().any(|i| i.category == "Texture");
    assert!(has_geom, "Should flag degenerate geometry");
    assert!(has_tex, "Should flag missing texture");

    // 2. Auto-fix
    state.active_texture = Some("Indus1.Indus_B01".to_string());
    let fixes = state.autofix_level_issues();
    assert!(fixes >= 2, "Auto-fix should resolve degenerate poly and missing texture");

    // Verify corrupt brush is cleaned
    let last_brush = state.world_brushes.last().unwrap();
    assert_eq!(last_brush.polys.len(), 1, "Degenerate poly must be removed");
    assert_eq!(last_brush.polys[0].texture, "Indus1.Indus_B01");
}
