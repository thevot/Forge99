use std::path::PathBuf;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_editor_state_load_unr_deck16() {
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/DM-Deck16][.unr");

    if !map_path.exists() {
        eprintln!("Map file not found: {map_path:?}, skipping");
        return;
    }

    let mut state = EditorState::default();
    assert_eq!(state.world_brushes.len(), 1, "Initial room brush");

    state.load_unr(&map_path).expect("Failed to load Deck16 into EditorState");

    assert!(state.loaded_unr.is_some(), "UnrLevel should be loaded");
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.map_name, "DM-Deck16][");
    assert_eq!(level.actors.len(), 749);
    assert_eq!(state.world_brushes.len(), 187, "Must load all 187 Deck16 brushes");
    assert!(level.lights.len() > 200, "Must have >200 lights");
    assert_eq!(level.player_starts.len(), 15, "Deck16 has 15 player starts");

    println!("Deck16 loaded successfully into EditorState!");
    println!("  Total Brushes: {}", state.world_brushes.len());
    println!("  Total Actors: {}", level.actors.len());
    println!("  Camera positioned at: {:?}", state.camera.position);
}

#[test]
fn test_editor_state_load_unr_tutorial() {
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/DM-Tutorial.unr");

    if !map_path.exists() {
        eprintln!("Map file not found: {map_path:?}, skipping");
        return;
    }

    let mut state = EditorState::default();
    state.load_unr(&map_path).expect("Failed to load Tutorial into EditorState");

    assert!(state.loaded_unr.is_some());
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(state.world_brushes.len(), 78, "Must load all 78 DM-Tutorial brushes");
    assert_eq!(level.actors.len(), 148);
}

#[test]
fn test_cycle1_t3d_and_unr_roundtrip_and_undo_redo() {
    let mut state = EditorState::default();
    assert!(state.loaded_unr.is_some(), "Fresh EditorState must have Some(UnrLevel)");

    // 1. Add actors, a semi-solid rotated brush, and a multi-keyframe Mover
    state.builder_brush.location = glam::Vec3::new(64.0, 64.0, -160.0);
    state.add_player_start_at_builder();
    state.builder_brush.location = glam::Vec3::new(0.0, 0.0, 128.0);
    state.add_light_at_builder();
    state.builder_brush.location = glam::Vec3::ZERO;
    state.add_mover_from_builder();
    state.selected_mover_index = Some(0);
    state.set_mover_key(0, 1);
    if let Some(m) = state.movers.get_mut(0) {
        m.brush.location = glam::Vec3::new(0.0, 0.0, 192.0);
        m.keyframes[1] = Some(ut99_editor::editor_state::MoverKeyframe {
            location: glam::Vec3::new(0.0, 0.0, 192.0),
            rotation: glam::Vec3::new(0.0, 90.0, 0.0),
        });
    }
    state.set_mover_key(0, 0);

    // Mark brush 0 as having a semi-solid polygon and custom texture
    state.world_brushes[0].polys[0].flags |= ut99_bsp::PF_SEMISOLID;
    state.world_brushes[0].polys[0].texture = "Indus1.Indus_Wall".to_string();
    state.world_brushes[0].rotation = glam::Vec3::new(0.0, 45.0, 0.0);

    // 2. Round-trip via T3D
    let tmp_t3d = std::env::temp_dir().join("forge99_cycle1_roundtrip.t3d");
    state.save_t3d(&tmp_t3d).expect("save_t3d failed");

    let mut loaded_t3d_state = EditorState::default();
    loaded_t3d_state.load_t3d(&tmp_t3d).expect("load_t3d failed");
    let _ = std::fs::remove_file(&tmp_t3d);

    assert_eq!(loaded_t3d_state.world_brushes[0].polys[0].flags & ut99_bsp::PF_SEMISOLID, ut99_bsp::PF_SEMISOLID);
    assert_eq!(loaded_t3d_state.world_brushes[0].polys[0].texture, "Indus1.Indus_Wall");
    assert!((loaded_t3d_state.world_brushes[0].rotation.y - 45.0).abs() < 0.5);
    assert_eq!(loaded_t3d_state.movers.len(), 1, "T3D load must reconstruct EditorMover");
    assert!(loaded_t3d_state.movers[0].keyframes[1].is_some(), "Mover keyframe 1 must be preserved in T3D");
    assert!(loaded_t3d_state.loaded_unr.as_ref().unwrap().player_starts.len() >= 1);
    assert!(loaded_t3d_state.loaded_unr.as_ref().unwrap().lights.len() >= 1);

    // 3. Round-trip via Native Binary .unr
    let tmp_unr = std::env::temp_dir().join("forge99_cycle1_roundtrip.unr");
    ut99_editor::compiler::CompilerBridge::save_unr_native(&state, &tmp_unr)
        .expect("save_unr_native failed");

    let mut loaded_unr_state = EditorState::default();
    loaded_unr_state.load_unr(&tmp_unr).expect("load_unr failed");
    let _ = std::fs::remove_file(&tmp_unr);

    assert!(!loaded_unr_state.movers.is_empty(), "UNR load must reconstruct EditorMover");
    assert!(loaded_unr_state.movers[0].keyframes[1].is_some(), "Mover keyframe 1 must survive UNR round-trip");
    assert_eq!(loaded_unr_state.world_brushes[0].polys[0].flags & ut99_bsp::PF_SEMISOLID, ut99_bsp::PF_SEMISOLID);

    // 4. Verify per-viewport shading defaults (Top/Side/Front = Wireframe, Perspective = Lit)
    assert_eq!(state.shading_for_quadrant(0), ut99_editor::camera::ViewportShading::Wireframe);
    assert_eq!(state.shading_for_quadrant(1), ut99_editor::camera::ViewportShading::Wireframe);
    assert_eq!(state.shading_for_quadrant(2), ut99_editor::camera::ViewportShading::Wireframe);
    assert_eq!(state.shading_for_quadrant(3), ut99_editor::camera::ViewportShading::Lit);

    // 5. Verify CSG Intersect, Deintersect, and 2D Shape Editor Undo/Redo
    let orig_builder_polys = state.builder_brush.polys.len();
    state.apply_shape2d_pyramid(192.0);
    assert_ne!(state.builder_brush.polys.len(), orig_builder_polys);
    state.undo();
    assert_eq!(state.builder_brush.polys.len(), orig_builder_polys);
    state.redo();
    assert_ne!(state.builder_brush.polys.len(), orig_builder_polys);
}

#[test]
fn test_cycle2_lighting_navigation_surface_memory_and_clipboard() {
    let mut state = EditorState::default();

    // 1. BG-09: Verify Subtract room interior lighting & PF_SPECIAL_LIT channel isolation
    state.builder_brush.location = glam::Vec3::new(0.0, 0.0, 256.0);
    state.add_light_at_builder();
    state.build_lighting();

    let ambient_x = state.lighting_config.ambient_color.x;
    let room_lit_faces = state.world_brushes[0]
        .polys
        .iter()
        .filter(|p| p.vertex_colors.iter().any(|c| c[0] > ambient_x + 0.05))
        .count();
    assert!(
        room_lit_faces == 6,
        "All 6 Subtract room interior walls/floor/ceiling must receive direct lighting without self-occlusion"
    );

    // Mark face 0 as PF_SPECIAL_LIT; normal light (bSpecialLit=false) must NOT illuminate it
    state.world_brushes[0].polys[0].flags |= ut99_bsp::PF_SPECIAL_LIT;
    state.build_lighting();
    for vc in &state.world_brushes[0].polys[0].vertex_colors {
        assert!(
            (vc[0] - ambient_x).abs() < 1e-4,
            "PF_SPECIAL_LIT surface must ignore non-bSpecialLit lights"
        );
    }

    // Now set bSpecialLit=true on the light; face 0 MUST be illuminated while normal faces only get ambient
    if let Some(ref mut unr) = state.loaded_unr {
        for actor in &mut unr.actors {
            if actor.class_name.contains("Light") {
                actor.properties.insert(
                    "bSpecialLit".to_string(),
                    ut99_format::TaggedProperty {
                        name: "bSpecialLit".to_string(),
                        array_index: 0,
                        value: ut99_format::TaggedValue::Bool(true),
                    },
                );
            }
        }
    }
    state.build_lighting();
    assert!(
        state.world_brushes[0].polys[0]
            .vertex_colors
            .iter()
            .any(|c| c[0] > ambient_x + 0.05),
        "PF_SPECIAL_LIT surface must be illuminated by bSpecialLit=true light"
    );

    // 2. BG-07: Collision-aware AI path building & automatic InventorySpot generation
    state.place_actor_of_class_at("PathNode", glam::Vec3::new(-180.0, 0.0, 64.0));
    state.place_actor_of_class_at("ShockRifle", glam::Vec3::new(180.0, 0.0, 64.0));
    state.build_ai_paths();

    let has_inv_spot = state
        .loaded_unr
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .any(|a| a.class_name == "InventorySpot");
    assert!(
        has_inv_spot,
        "build_ai_paths must auto-generate InventorySpot for weapon/pickup actors"
    );
    assert!(
        !state.ai_path_links.is_empty(),
        "Clear line-of-sight inside Subtract room must generate navigation links"
    );

    // Place a solid additive wall directly between (-180, 0, 64) and (180, 0, 64)
    let mut wall = ut99_bsp::build_cube(
        "BlockingWall",
        ut99_bsp::CsgOperation::Add,
        glam::Vec3::new(32.0, 480.0, 480.0),
    );
    wall.location = glam::Vec3::new(0.0, 0.0, 256.0);
    state.world_brushes.push(wall);
    state.build_ai_paths();
    assert_eq!(
        state.ai_path_links.len(),
        0,
        "Solid additive wall must block AI navigation links through walls"
    );
    state.world_brushes.pop(); // remove wall

    // 3. BG-13: Surface Selection Memory (Shift+M, Shift+R, Shift+O, Shift+U, Shift+X)
    state.deselect_all_surfaces();
    state.select_surface(0, 0, true);
    state.select_surface(0, 1, true);
    state.memorize_surface_selection(); // Memory = {(0,0), (0,1)}
    assert_eq!(state.memorized_surfaces.len(), 2);

    state.deselect_all_surfaces();
    assert_eq!(state.selected_surfaces.len(), 0);
    state.recall_surface_selection(); // Selected = {(0,0), (0,1)}
    assert_eq!(state.selected_surfaces.len(), 2);

    state.deselect_all_surfaces();
    state.select_surface(0, 1, true);
    state.select_surface(0, 2, true); // Selected = {(0,1), (0,2)}
    state.or_surface_selection(); // OR -> {(0,0), (0,1), (0,2)}
    assert_eq!(state.selected_surfaces.len(), 3);

    state.and_surface_selection(); // AND with {(0,0), (0,1)} -> {(0,0), (0,1)}
    assert_eq!(state.selected_surfaces.len(), 2);

    state.select_surface(0, 2, true); // Selected = {(0,0), (0,1), (0,2)}
    state.xor_surface_selection(); // XOR with {(0,0), (0,1)} -> {(0,2)}
    assert_eq!(state.selected_surfaces.len(), 1);
    assert!(state.selected_surfaces.contains(&(0, 2)));

    // 4. BG-13: .t3d Clipboard text export & paste_from_t3d_text
    state.selected_brush_index = Some(0);
    let (copied_b, _) = state.copy_selected_to_clipboard();
    assert_eq!(copied_b, 1);
    assert!(state.clipboard.t3d_text.contains("Begin Map"));
    assert!(state.clipboard.t3d_text.contains("Begin PolyList"));

    let t3d_clip = state.clipboard.t3d_text.clone();
    let brushes_before = state.world_brushes.len();
    let (pasted_b, _) = state.paste_from_t3d_text(&t3d_clip);
    assert_eq!(pasted_b, 1);
    assert_eq!(state.world_brushes.len(), brushes_before + 1);

    // 5. BG-11 & BG-12: LevelInfo.Song assignment & UnrealScript .uc parser
    state.set_level_song("Foregone.umx");
    assert_eq!(state.level_song, "Foregone");
    state.undo();
    assert_ne!(state.level_song, "Foregone");
    state.redo();
    assert_eq!(state.level_song, "Foregone");

    let mut meta = ut99_editor::unrealscript::get_class_metadata("Trigger", None, Some("Engine"));
    ut99_editor::unrealscript::parse_uc_script_into_metadata(
        "var(Trigger) float RepeatTriggerTime;\ndefaultproperties\n{\n    RepeatTriggerTime=2.500000\n}\n",
        &mut meta,
    );
    assert!(meta.variables.iter().any(|v| v.name == "RepeatTriggerTime" && v.is_editable));
    assert!(meta
        .default_properties
        .iter()
        .any(|(k, v)| k == "RepeatTriggerTime" && v == "2.500000"));
}

#[test]
fn test_cycle3_surface_shortcuts_diagnostics_console_and_defaults() {
    let mut state = EditorState::default();

    // 1. Surface selection shortcuts: Shift+G (Groups), Shift+I (Items), Shift+J (Adjacent All)
    state.brush_groups.insert(0, "BaseFloor".to_string());
    state.deselect_all_surfaces();
    state.select_surface(0, 0, true);
    state.select_surfaces_matching_groups();
    assert_eq!(
        state.selected_surfaces.len(),
        6,
        "Shift+G must select all 6 faces of brush 0 in group BaseFloor"
    );

    state.deselect_all_surfaces();
    state.select_surface(0, 0, true);
    state.select_surfaces_matching_items();
    assert_eq!(
        state.selected_surfaces.len(),
        6,
        "Shift+I must select all surfaces with matching CSG/PolyFlags item archetype"
    );

    state.deselect_all_surfaces();
    state.select_surface(0, 0, true);
    state.select_surfaces_adjacent_all();
    assert_eq!(
        state.selected_surfaces.len(),
        6,
        "Shift+J must flood-fill all edge-sharing surfaces of the cube"
    );

    // 2. DefaultProperties pre-population & AmbientSound placement
    state.builder_brush.location = glam::Vec3::new(64.0, 64.0, 128.0);
    state.place_ambient_sound_at_builder("AmbAncient.Waterfall");
    let ambient_actor = state
        .loaded_unr
        .as_ref()
        .unwrap()
        .actors
        .last()
        .expect("AmbientSound actor must be spawned");
    assert_eq!(ambient_actor.class_name, "AmbientSound");
    assert_eq!(
        ambient_actor.properties.get("AmbientSound").map(|p| &p.value),
        Some(&ut99_format::TaggedValue::Str("Sound'AmbAncient.Waterfall'".to_string()))
    );
    assert!(
        ambient_actor.properties.contains_key("SoundRadius"),
        "DefaultProperties & AmbientSound properties must be pre-populated"
    );

    // 3. TextureManager::sample_texture_rgba & "In Use" group
    let custom_tex = ut99_format::UtxTexture {
        name: "TestBrick".to_string(),
        group: "Wall".to_string(),
        width: 2,
        height: 2,
        format: 0,
        masked: false,
        rgba_pixels: vec![
            200, 100, 50, 255, 200, 100, 50, 255, 200, 100, 50, 255, 200, 100, 50, 255,
        ],
    };
    state.texture_manager.upsert_texture("CustomPkg", custom_tex);
    let groups = state.texture_manager.get_package_groups("CustomPkg");
    assert!(groups.contains(&"In Use".to_string()));
    let sampled = state
        .texture_manager
        .sample_texture_rgba("CustomPkg.TestBrick", 1.0, 1.0)
        .expect("Must sample loaded .utx RGBA texture");
    assert!((sampled[0] - 200.0 / 255.0).abs() < 0.01);

    // 4. LevelDoctor Zero-HOM checks: non-planar polygon & Semi-Solid intersecting Portal
    let mut twisted_brush = state.world_brushes[0].clone();
    twisted_brush.name = "TwistedBrush".to_string();
    let n0 = twisted_brush.polys[0].plane.normal;
    twisted_brush.polys[0].vertices[0] += n0 * 4.0;

    let mut portal_brush = ut99_bsp::build_cube("PortalSheet", ut99_bsp::CsgOperation::Add, glam::Vec3::new(256.0, 256.0, 16.0));
    portal_brush.location = glam::Vec3::new(0.0, 0.0, 256.0);
    for p in &mut portal_brush.polys {
        p.flags |= ut99_bsp::PF_PORTAL;
    }
    let mut semisolid_brush =
        ut99_bsp::build_cube("SemiPillar", ut99_bsp::CsgOperation::Add, glam::Vec3::splat(128.0));
    semisolid_brush.location = glam::Vec3::new(0.0, 0.0, 256.0);
    for p in &mut semisolid_brush.polys {
        p.flags |= ut99_bsp::PF_SEMISOLID;
    }

    let issues = ut99_bsp::LevelDoctor::audit_brushes(&[
        twisted_brush.clone(),
        portal_brush,
        semisolid_brush,
    ]);
    assert!(
        issues.iter().any(|i| i.description.contains("non-planar")),
        "LevelDoctor must detect non-planar polygon"
    );
    assert!(
        issues.iter().any(|i| i.description.contains("intersects Zone Portal")),
        "LevelDoctor must detect Semi-Solid intersecting Zone Portal (HOM risk)"
    );

    let fixes = ut99_bsp::LevelDoctor::autofix_brush(&mut twisted_brush, "DefaultTexture");
    assert!(fixes >= 1, "autofix_brush must project non-planar vertices back onto plane");

    // 5. UnrealEd 2.0 Console Command Executor
    let resp_all = state.execute_unrealed_command("POLY SELECT ALL");
    assert!(resp_all.contains("POLY SELECT ALL"));
    assert_eq!(state.selected_surfaces.len(), 6);

    let resp_none = state.execute_unrealed_command("POLY SELECT NONE");
    assert!(resp_none.contains("0 surfaces"));
    assert_eq!(state.selected_surfaces.len(), 0);

    let _ = state.execute_unrealed_command("RMODE 6");
    assert_eq!(state.viewport_shading, ut99_editor::camera::ViewportShading::Lit);
}

#[test]
fn test_cycle4_scale_lights_clear_paths_mylevel_and_brush_interop() {
    let mut state = EditorState::default();
    state.new_map();
    state.csg_subtract();

    // 1. Scale Lights... (Percentage & Literal with Undo/Redo)
    state.add_light_at_builder();
    assert!(state.scale_lights_brightness(200.0, true, false) >= 1);
    let get_brightness = |st: &EditorState| -> i32 {
        let unr = st.loaded_unr.as_ref().unwrap();
        let light = unr.actors.iter().rev().find(|a| a.class_name == "Light").unwrap();
        light.light_brightness as i32
    };
    // Default LightBrightness is 128 or 64; scaling by 200% clamps to <= 255
    let scaled_b = get_brightness(&state);
    assert!(scaled_b >= 128 && scaled_b <= 255);
    state.undo();
    let restored_b = get_brightness(&state);
    assert!(restored_b < scaled_b || scaled_b == 255);
    state.redo();
    assert_eq!(get_brightness(&state), scaled_b);

    // Literal delta via console command or direct API
    state.scale_lights_brightness(-32.0, false, false);
    assert_eq!(get_brightness(&state), (scaled_b - 32).max(0));

    // 2. Build & Clear AI Paths (PATHS DEFINE / PATHS UNDEFINE)
    state.add_weapon_or_pickup_at_builder("ShockRifle");
    state.builder_brush.location = glam::Vec3::new(128.0, 0.0, 0.0);
    state.add_path_node_at_builder();
    state.build_ai_paths();
    let spot_count_after_build = state
        .loaded_unr
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .filter(|a| a.class_name.eq_ignore_ascii_case("InventorySpot"))
        .count();
    assert!(
        spot_count_after_build >= 1,
        "build_ai_paths must auto-generate InventorySpot for ShockRifle"
    );
    let cleared = state.clear_ai_paths();
    assert!(cleared >= 1);
    let spot_count_after_clear = state
        .loaded_unr
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .filter(|a| a.class_name.eq_ignore_ascii_case("InventorySpot"))
        .count();
    assert_eq!(
        spot_count_after_clear, spot_count_after_build,
        "clear_ai_paths must preserve InventorySpot actors"
    );
    assert!(state.ai_path_links.is_empty());

    // 3. MyLevel Embedded Texture Import & Pinning
    let full_ref = state
        .texture_manager
        .import_rgba_to_package(
            "MyLevel",
            "Embedded",
            "ScreenshotShot",
            2,
            2,
            false,
            vec![
                64, 128, 255, 255, 64, 128, 255, 255, 64, 128, 255, 255, 64, 128, 255, 255,
            ],
        )
        .expect("Should import RGBA texture into MyLevel");
    assert_eq!(full_ref, "MyLevel.ScreenshotShot");
    assert!(
        state.texture_manager.is_package_pinned("MyLevel"),
        "MyLevel must be pinned against LRU eviction"
    );
    let sampled = state
        .texture_manager
        .sample_texture_rgba("MyLevel.ScreenshotShot", 0.5, 0.5)
        .expect("Should sample MyLevel texture");
    assert!((sampled[2] - 1.0).abs() < 0.01);

    // 4. Brush .t3d and .dxf Round-Trip Import/Export with Undo/Redo
    let t3d_text = state.export_builder_brush_to_t3d();
    assert!(t3d_text.contains("Begin Brush"));
    assert!(t3d_text.contains("Begin PolyList"));

    let dxf_text = state.export_builder_brush_to_dxf();
    assert!(dxf_text.contains("3DFACE"));

    // Modify builder brush to a cylinder, then import the 6-face cube back from T3D and DXF
    state.builder_brush = ut99_bsp::build_cylinder(
        "Cyl",
        ut99_bsp::CsgOperation::Active,
        128.0,
        64.0,
        8,
    );
    assert_eq!(state.builder_brush.polys.len(), 10);

    let imported_t3d_polys = state
        .import_t3d_to_builder_brush(&t3d_text)
        .expect("T3D brush import should succeed");
    assert_eq!(imported_t3d_polys, 6);
    assert_eq!(state.builder_brush.polys.len(), 6);
    state.undo();
    assert_eq!(state.builder_brush.polys.len(), 10);

    let imported_dxf_polys = state
        .import_dxf_to_builder_brush(&dxf_text)
        .expect("DXF 3DFACE brush import should succeed");
    assert_eq!(imported_dxf_polys, 6);
    assert_eq!(state.builder_brush.polys.len(), 6);
}

#[test]
fn test_cycle5_level_document_select_inside_poly_merge_and_asc() {
    let mut state = EditorState::default();
    state.new_map();
    state.csg_subtract();

    // 1. Polygons -> Separate & Polygons -> Merge on Selected Brush with Undo/Redo
    state.selected_brush_index = Some(0);
    assert_eq!(state.world_brushes[0].polys.len(), 6);

    let added_tris = state.separate_selected_brush_polygons();
    assert_eq!(added_tris, 6);
    assert_eq!(state.world_brushes[0].polys.len(), 12);

    // Undo / Redo Separate
    state.undo();
    assert_eq!(state.world_brushes[0].polys.len(), 6);
    state.redo();
    assert_eq!(state.world_brushes[0].polys.len(), 12);

    // Merge the 12 coplanar triangles back into 6 convex quads
    let merged_pairs = state.merge_selected_brush_polygons();
    assert_eq!(merged_pairs, 6);
    assert_eq!(state.world_brushes[0].polys.len(), 6);

    // Undo / Redo Merge
    state.undo();
    assert_eq!(state.world_brushes[0].polys.len(), 12);
    state.redo();
    assert_eq!(state.world_brushes[0].polys.len(), 6);

    // 2. Polygons -> To Brush & Polygons -> From Brush with Undo/Redo
    state.builder_brush = ut99_bsp::build_cylinder(
        "Cyl",
        ut99_bsp::CsgOperation::Active,
        128.0,
        64.0,
        8,
    );
    assert_eq!(state.builder_brush.polys.len(), 10);
    assert!(state.polygons_from_brush());
    assert_eq!(state.world_brushes[0].polys.len(), 10);
    state.undo();
    assert_eq!(state.world_brushes[0].polys.len(), 6);

    assert!(state.polygons_to_brush());
    assert_eq!(state.builder_brush.polys.len(), 6);
    state.undo();
    assert_eq!(state.builder_brush.polys.len(), 10);
    state.redo();
    assert_eq!(state.builder_brush.polys.len(), 6);

    // 3. 3D Studio .asc Export & Import with Undo/Redo
    let asc_text = state.export_builder_brush_to_asc();
    assert!(asc_text.contains("Tri-mesh, Vertices:"));
    assert!(asc_text.contains("Face list:"));

    state.builder_brush = ut99_bsp::build_cylinder(
        "Cyl",
        ut99_bsp::CsgOperation::Active,
        128.0,
        64.0,
        8,
    );
    let imported_asc_faces = state
        .import_asc_to_builder_brush(&asc_text)
        .expect("3D Studio .asc import should succeed");
    assert_eq!(imported_asc_faces, 12);
    assert_eq!(state.builder_brush.polys.len(), 12);
    // Merge the 12 imported .asc triangles on the Red Builder Brush into 6 quads!
    state.selected_brush_index = None;
    let merged_builder = state.merge_selected_brush_polygons();
    assert_eq!(merged_builder, 6);
    assert_eq!(state.builder_brush.polys.len(), 6);

    // 4. Select Inside Red Builder Brush (`ACTOR SELECT INSIDE`)
    state.builder_brush = ut99_bsp::build_cube(
        "BigBox",
        ut99_bsp::CsgOperation::Active,
        glam::Vec3::splat(1024.0),
    );
    state.builder_brush.location = glam::Vec3::ZERO;
    state.add_path_node_at_builder();
    let resp = state.execute_unrealed_command("ACTOR SELECT INSIDE");
    assert!(resp.contains("ACTOR SELECT INSIDE"));
    assert!(!state.multi_selected_actors.is_empty());
    assert!(!state.multi_selected_brushes.is_empty());

    // 5. LevelDocument SSOT Snapshot, Summary & Round-Trip (`Deliverable F.1`)
    state.add_mover_from_builder();
    let doc = state.to_level_document();
    let summary = doc.summary();
    assert!(summary.total_brushes >= 1);
    assert_eq!(summary.movers_count, 1);
    assert!(summary.lights_count >= 1);
    assert!(summary.nav_points_count >= 1);

    let t3d_doc = doc.to_t3d_string(&state.builder_brush);
    let roundtrip_doc =
        ut99_editor::LevelDocument::from_t3d_str(&t3d_doc).expect("LevelDocument T3D roundtrip");
    assert_eq!(roundtrip_doc.world_brushes.len(), doc.world_brushes.len());
    assert_eq!(roundtrip_doc.movers.len(), 1);
}

#[test]
fn test_cycle6_smart_prefabs_freehand_background_and_master_browser() {
    let mut state = EditorState::default();
    state.new_map();
    let initial_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().unwrap().actors.len();

    // 1. Data-Driven SmartPrefabLibrary Catalog & Instantiation + Undo/Redo (`Deliverable E.4` & `F.1`)
    let catalog = ut99_editor::SmartPrefabLibrary::catalog();
    assert_eq!(catalog.len(), 12);

    let (b_cnt, m_cnt, a_cnt) = state
        .instantiate_smart_prefab("mover_lift_platform", glam::Vec3::new(0.0, 0.0, 64.0))
        .expect("Smart prefab 'mover_lift_platform' should instantiate");
    assert_eq!((b_cnt, m_cnt, a_cnt), (0, 1, 3));
    assert_eq!(state.movers.len(), 1);
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors.len(),
        initial_actors + 3
    );

    // Undo / Redo atomic compound instantiation
    state.undo();
    assert_eq!(state.movers.len(), 0);
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors.len(),
        initial_actors
    );
    state.redo();
    assert_eq!(state.movers.len(), 1);
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors.len(),
        initial_actors + 3
    );

    // Instantiate a CTF Flag Base prefab
    let (b2, m2, a2) = state
        .instantiate_smart_prefab("gameplay_ctf_flag_base", glam::Vec3::new(512.0, 0.0, 0.0))
        .expect("Smart prefab 'gameplay_ctf_flag_base' should instantiate");
    assert_eq!((b2, m2, a2), (1, 0, 2));
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);

    // 2. Freehand Polygon Drawing & Extrusion to Red Builder Brush with Undo/Redo (`Deliverable B Categoria A.3 Gruppo 1`)
    state.snap_to_grid = false;
    state.clear_freehand_polygon();
    state.add_freehand_vertex(glam::Vec3::new(-128.0, -128.0, 0.0));
    state.add_freehand_vertex(glam::Vec3::new(128.0, -128.0, 0.0));
    state.add_freehand_vertex(glam::Vec3::new(128.0, 128.0, 0.0));
    state.add_freehand_vertex(glam::Vec3::new(0.0, 256.0, 0.0));
    state.add_freehand_vertex(glam::Vec3::new(-128.0, 128.0, 0.0));
    assert_eq!(state.freehand_points.len(), 5);

    let extruded_polys = state
        .extrude_freehand_polygon_to_builder(192.0)
        .expect("5-vertex pentagon extrusion should succeed");
    // 2 caps + 5 side quads = 7 polygons
    assert_eq!(extruded_polys, 7);
    assert_eq!(state.builder_brush.polys.len(), 7);

    state.undo();
    assert_eq!(state.builder_brush.polys.len(), 6);
    state.redo();
    assert_eq!(state.builder_brush.polys.len(), 7);

    // 3. Background Image & Master Browser Tabs (`Deliverable B Categoria A.1` & `H.1`)
    assert_eq!(state.master_browser_tab, ut99_editor::MasterBrowserTab::Textures);
    assert_eq!(ut99_editor::MasterBrowserTab::all().len(), 8);
    state.show_master_browser = true;
    state.master_browser_tab = ut99_editor::MasterBrowserTab::SmartPrefabs;

    state.set_background_image(
        Some("Floorplan_Deck16.png".to_string()),
        ut99_editor::BackgroundDisplayMode::Tile,
    );
    assert_eq!(
        state.background_image_path.as_deref(),
        Some("Floorplan_Deck16.png")
    );
    assert_eq!(
        state.background_display_mode,
        ut99_editor::BackgroundDisplayMode::Tile
    );
    state.set_background_image(None, ut99_editor::BackgroundDisplayMode::Center);
    assert!(state.background_image_path.is_none());
}

#[test]
fn test_cycle7_utx_mylevel_binary_umesh_and_freehand_mode() {
    let mut state = EditorState::default();

    // 1. EditorMode::FreehandPolygon mode selection
    state.editor_mode = ut99_editor::EditorMode::FreehandPolygon;
    assert_eq!(state.editor_mode, ut99_editor::EditorMode::FreehandPolygon);

    // 2. Import RGBA texture into MyLevel and verify standalone .utx binary save/load roundtrip
    let mut pixels = Vec::with_capacity(4 * 4 * 4);
    for i in 0..16u8 {
        pixels.extend_from_slice(&[i * 12, 200 - i * 8, 60 + i * 4, 255]);
    }
    let full_ref = state
        .texture_manager
        .import_rgba_to_package("MyLevel", "Walls", "NeonTrim", 4, 4, false, pixels.clone())
        .expect("Should import 4x4 RGBA texture into MyLevel");
    assert_eq!(full_ref, "MyLevel.NeonTrim");

    let temp_dir = std::env::temp_dir().join("forge99_cycle7_test");
    let _ = std::fs::create_dir_all(&temp_dir);
    let utx_path = temp_dir.join("CustomPack.utx");
    let saved_count = state
        .export_texture_package_utx("MyLevel", &utx_path)
        .expect("Should save MyLevel as standalone .utx");
    assert_eq!(saved_count, 1);

    let reopened_utx = ut99_format::UtxPackage::open(&utx_path)
        .expect("Saved .utx file must be valid UE1 package readable by UtxPackage::open");
    assert_eq!(reopened_utx.textures.len(), 1);
    assert_eq!(reopened_utx.textures[0].name, "NeonTrim");
    assert_eq!(reopened_utx.textures[0].width, 4);
    assert_eq!(reopened_utx.textures[0].height, 4);
    assert_eq!(reopened_utx.textures[0].rgba_pixels, pixels);

    // 3. Save .unr natively with embedded MyLevel texture and reload into a fresh EditorState
    let unr_path = temp_dir.join("DM-Cycle7MyLevel.unr");
    state
        .save_unr_native(&unr_path)
        .expect("Native .unr save with embedded MyLevel texture should succeed");

    let mut state2 = EditorState::default();
    assert!(state2.texture_manager.get_package("MyLevel").is_none());
    state2
        .load_unr(&unr_path)
        .expect("Reloading .unr with embedded MyLevel texture should succeed");
    let mylevel_reloaded = state2
        .texture_manager
        .get_texture("MyLevel", "NeonTrim")
        .expect("MyLevel.NeonTrim should be restored from binary .unr");
    assert_eq!(mylevel_reloaded.width, 4);
    assert_eq!(mylevel_reloaded.height, 4);
    assert_eq!(mylevel_reloaded.rgba_pixels, pixels);

    // 4. Binary UMesh 11-11-10 bit FMeshVert + 20-byte FMeshTri encode/decode & MeshLibrary import
    let tetra_verts = vec![
        [0.0, 0.0, 64.0],
        [-48.0, -32.0, -32.0],
        [48.0, -32.0, -32.0],
        [0.0, 48.0, -32.0],
    ];
    let tetra_tris = vec![[0u16, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]];
    let raw_umesh = ut99_format::package::encode_umesh_geometry(&tetra_verts, &tetra_tris);
    let (dec_verts, dec_tris) = ut99_format::package::decode_umesh_geometry(&raw_umesh)
        .expect("Should decode binary UMesh geometry");
    assert_eq!(dec_verts, tetra_verts);
    assert_eq!(dec_tris, tetra_tris);

    let mesh_data = ut99_format::package::PackageMeshData {
        name: "Botpack.TrophyTetra".to_string(),
        class_name: "LodMesh".to_string(),
        serial_size: raw_umesh.len(),
        vertices: dec_verts,
        triangles: dec_tris,
    };
    assert!(state2.mesh_library.import_package_mesh_data("Botpack", &mesh_data));
    let imported_mesh = state2
        .mesh_library
        .get_mesh("Botpack.TrophyTetra")
        .expect("MeshLibrary should contain imported Botpack.TrophyTetra");
    assert_eq!(imported_mesh.poly_count(), 4);

    let _ = std::fs::remove_dir_all(&temp_dir);
}



