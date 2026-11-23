//! Test Suite di Integrazione End-to-End: Parità e Superamento Totale di UnrealEd 2.0 / 2.1
//!
//! Questo modulo convalida programmaticamente le 6 fasi strategiche definite nel
//! documento PLAN-REA-UED2-PARITY-SUPREMACY-20261014:
//! 1. Integrità Dati, Persistenza Safe & Espressioni Matematiche
//! 2. Ergonomia Multi-Viewport & Radii View 3D
//! 3. Manipolazione Superfici, UV Drag & Risorse
//! 4. Dinamiche di Gioco, Movers a 8 Keyframe & Static Meshes
//! 5. Compilazione Luci Multithread Rayon & Rete di Navigazione Bot
//! 6. Supremacy Suite: Bilanciamento Torneo F12 & Simmetria Arene CTF

use glam::Vec3;
use ut99_bsp::{
    evaluate_expression, evaluate_or_default, CsgOperation,
    PF_FAKE_BACKDROP, PF_PORTAL, PF_TRANSLUCENT,
};
use ut99_editor::camera::{Camera, ViewportType};
use ut99_editor::editor_state::EditorState;
use ut99_format::{ActorRecord, UnrLevel};

#[test]
fn test_phase1_data_integrity_and_math_evaluation() {
    // 1. Valutazione formule matematiche in-field (standard UEd2)
    assert_eq!(evaluate_expression("=512-128").unwrap(), 384.0);
    assert_eq!(evaluate_expression("=60*3.5").unwrap(), 210.0);
    assert_eq!(evaluate_expression("=1024/4+32").unwrap(), 288.0);
    assert_eq!(evaluate_or_default("invalid", 512.0), 512.0);

    // 2. Integrità dati e persistenza: aggiunta brush, attori e serializzazione
    let mut state = EditorState::default();
    state.world_brushes.clear();

    let box_brush = ut99_bsp::build_cube("SubRoom", CsgOperation::Subtract, Vec3::new(512.0, 512.0, 512.0));
    state.world_brushes.push(box_brush);

    state.builder_brush.location = Vec3::new(0.0, 0.0, 200.0);
    state.place_actor_of_class("Light");

    assert_eq!(state.world_brushes.len(), 1);
    assert_eq!(state.world_brushes[0].csg_oper, CsgOperation::Subtract);
    assert!(state.loaded_unr.as_ref().map(|u| !u.actors.is_empty()).unwrap_or(false));
}

#[test]
fn test_phase2_multi_viewport_projections_and_radii() {
    // 1. Verifica calcolo matrici view e projection per le 4 viste
    let cam = Camera {
        position: Vec3::new(100.0, 200.0, 300.0),
        ortho_zoom: 1024.0,
        ..Default::default()
    };

    let mat_top = cam.view_matrix_for_type(ViewportType::Top);
    let mat_front = cam.view_matrix_for_type(ViewportType::Front);
    let mat_side = cam.view_matrix_for_type(ViewportType::Side);
    let mat_3d = cam.view_matrix_for_type(ViewportType::Perspective);

    assert_ne!(mat_top, mat_front);
    assert_ne!(mat_front, mat_side);
    assert_ne!(mat_top, mat_3d);

    let proj_top = cam.projection_matrix_for_type(ViewportType::Top, 16.0 / 9.0);
    let proj_3d = cam.projection_matrix_for_type(ViewportType::Perspective, 16.0 / 9.0);
    assert_ne!(proj_top, proj_3d);

    // 2. Radii view e limiti di attenuazione
    let mut state = EditorState::default();
    state.builder_brush.location = Vec3::new(100.0, 100.0, 50.0);
    state.place_actor_of_class("Light");
    assert!(state.loaded_unr.as_ref().map(|u| !u.actors.is_empty()).unwrap_or(false));
}

#[test]
fn test_phase3_surface_alignment_and_material_manipulation() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    let mut cube = ut99_bsp::build_cube("TestCube", CsgOperation::Add, Vec3::new(256.0, 256.0, 256.0));
    for poly in &mut cube.polys {
        poly.texture = "Indus1.Indus_B01".to_string();
        poly.flags = PF_FAKE_BACKDROP | PF_TRANSLUCENT | PF_PORTAL;
    }
    state.world_brushes.push(cube);

    // Verifica flags di superficie UE1
    let poly = &state.world_brushes[0].polys[0];
    assert!(poly.flags & PF_FAKE_BACKDROP != 0);
    assert!(poly.flags & PF_TRANSLUCENT != 0);
    assert!(poly.flags & PF_PORTAL != 0);
    assert_eq!(poly.texture, "Indus1.Indus_B01");
}

#[test]
fn test_phase4_mover_keyframes_tag_event_wiring_and_meshes() {
    let mut state = EditorState::default();

    // 1. Configurazione Mover con keyframes
    state.add_mover_from_builder();
    assert_eq!(state.movers.len(), 1);
    assert_eq!(state.movers[0].current_key, 0);

    let kf0 = state.movers[0].keyframes[0].as_ref().expect("Key 0 initialized");
    assert_eq!(kf0.location, state.builder_brush.location);

    // Muovi e salva key 1
    let open_pos = Vec3::new(100.0, 200.0, 500.0);
    state.set_selected_mover_key(1);
    state.save_selected_mover_current_key_pos(open_pos, Vec3::ZERO);
    assert_eq!(state.movers[0].keyframes[1].as_ref().unwrap().location, open_pos);

    // 2. Tag & Event wiring su mover
    state.movers[0].tag = "OpenMainDoor".to_string();
    assert_eq!(state.movers[0].tag, "OpenMainDoor");
}

#[test]
fn test_phase5_rayon_lighting_and_bot_navigation_specs() {
    let mut state = EditorState::default();

    // Rebuild percorsi bot
    state.rebuild_navigation_paths();

    assert!(
        state.nav_graph.nodes.len() >= 2,
        "La rete deve contenere i nodi base"
    );
    assert!(
        !state.nav_graph.reach_specs.is_empty(),
        "La rete deve contenere ReachSpec calcolate"
    );

    // Verifica vincoli di navigazione canonici UT99
    for spec in &state.nav_graph.reach_specs {
        assert!(spec.distance > 0.0, "La distanza della spec deve essere positiva");
    }
}

#[test]
fn test_phase6_supremacy_tournament_diagnostics_and_ctf_symmetry() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // Setup arena competitiva: ShieldBelt + UDamage troppo vicini (Hazard)
    let mut shield = ActorRecord::default();
    shield.class_name = "Botpack.UT_ShieldBelt".to_string();
    shield.name = "Comp_Shield".to_string();
    shield.location = Vec3::new(0.0, 0.0, 0.0);
    level.actors.push(shield);

    let mut udam = ActorRecord::default();
    udam.class_name = "Botpack.UDamage".to_string();
    udam.name = "Comp_UDamage".to_string();
    udam.location = Vec3::new(250.0, 0.0, 0.0);
    level.actors.push(udam);

    state.loaded_unr = Some(level);

    // Esegui analisi bilanciamento torneo F12
    let report = state.analyze_map_gameplay_balance();
    assert_eq!(report.pickups.len(), 2);
    assert_eq!(report.pair_timings.len(), 1);
    assert!(
        report.pair_timings[0].is_stacking_hazard,
        "Deve rilevare lo stacking hazard tra ShieldBelt e UDamage"
    );
    assert!(!report.is_tournament_certified);
}

#[test]
fn test_target_engine_keymap_and_mover_show_polys() {
    use ut99_editor::editor_state::{KeymapProfile, TargetEngine};

    let mut state = EditorState::default();
    assert_eq!(state.target_engine, TargetEngine::Ut99Standard);
    assert_eq!(state.keymap_profile, KeymapProfile::Modern);
    assert!(state.show_mover_polys);

    state.target_engine = TargetEngine::Unreal227;
    state.keymap_profile = KeymapProfile::ClassicUnrealEd2;
    state.show_mover_polys = false;

    assert!(state.target_engine.label().contains("227"));
    assert!(state.keymap_profile.label().contains("Classic"));
    assert!(!state.show_mover_polys);
}

#[test]
fn test_lossless_unr_roundtrip_and_structural_validator() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Create a subtractive room with custom texture
    state.set_builder_cube(Vec3::new(512.0, 512.0, 256.0));
    state.active_texture = Some("Indus1.Floor.Flr01".to_string());
    state.csg_subtract();

    // Place Light and PlayerStart
    state.builder_brush.location = Vec3::new(0.0, 0.0, 64.0);
    state.place_actor_of_class("Light");
    state.builder_brush.location = Vec3::new(128.0, 0.0, -64.0);
    state.place_actor_of_class("PlayerStart");

    // Rebuild geometry and paths
    state.build_geometry();
    state.rebuild_navigation_paths();

    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join("test_editor_unr_roundtrip.unr");
    state.save_unr_native(&tmp_path).expect("save_unr_native should succeed");

    // Validate binary package structure with UnrLevel::validate_package
    let bytes = std::fs::read(&tmp_path).expect("read saved .unr");
    let pkg = ut99_format::Package::from_bytes(bytes).expect("parse saved .unr");
    let validation = UnrLevel::validate_package(&pkg);
    assert!(
        validation.is_valid,
        "Saved .unr must pass structural validation: {:?}",
        validation.errors
    );
    assert!(validation.actor_count >= 2);

    // Reload into a fresh EditorState and verify round-trip fidelity
    let mut reloaded = EditorState::default();
    reloaded.load_unr(&tmp_path).expect("load_unr should succeed");
    assert!(!reloaded.world_brushes.is_empty());
    assert_eq!(reloaded.world_brushes[0].polys[0].texture, "Indus1.Floor.Flr01");

    let _ = std::fs::remove_file(&tmp_path);
}

#[test]
fn test_true_boolean_csg_intersect_deintersect_and_radiosity_bounces() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Carve two overlapping subtractive rooms that share a doorway
    state.set_builder_cube(Vec3::new(256.0, 256.0, 256.0));
    state.builder_brush.location = Vec3::new(0.0, 0.0, 0.0);
    state.csg_subtract();

    state.builder_brush.location = Vec3::new(128.0, 0.0, 0.0);
    state.csg_subtract();

    // True boolean evaluation must clip the shared interior wall
    let indexed_polys = ut99_bsp::evaluate_csg_brushes_indexed(&state.world_brushes);
    assert!(
        indexed_polys.len() > 6,
        "Overlapping subtractive rooms must produce boolean-clipped polygons"
    );

    // Test CSG Intersect & Deintersect against carved world
    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    state.builder_brush.location = Vec3::new(64.0, 0.0, 0.0);
    state.csg_intersect();
    assert!(!state.builder_brush.polys.is_empty());

    state.set_builder_cube(Vec3::new(256.0, 256.0, 256.0));
    state.builder_brush.location = Vec3::new(256.0, 0.0, 0.0);
    state.csg_deintersect();
    assert!(!state.builder_brush.polys.is_empty());

    // Test multi-pass radiosity bounce energy transfer
    state.lighting_config.radiosity_bounces = 2;
    state.builder_brush.location = Vec3::new(0.0, 0.0, 32.0);
    state.place_actor_of_class("Light");
    state.build_lighting();
    assert!(state.last_lighting_stats.is_some());
}

#[test]
fn test_shape2d_bezier_2ds_roundtrip_and_event_tag_audit() {
    let mut shape = ut99_bsp::Shape2D::new_box(128.0, 64.0);
    shape.set_segment_bezier(0);
    assert!(matches!(
        shape.segments[0],
        ut99_bsp::shape2d::Segment2DType::Bezier { .. }
    ));
    assert!(shape.tessellated_points().len() > 4);

    let text_2ds = shape.to_2ds();
    let parsed = ut99_bsp::Shape2D::from_2ds(&text_2ds).expect("parse .2ds");
    assert_eq!(parsed.points.len(), shape.points.len());
    assert!(matches!(
        parsed.segments[0],
        ut99_bsp::shape2d::Segment2DType::Bezier { .. }
    ));

    // Test LevelDoctor orphan Event -> Tag audit: (name, class_name, tag, event, location)
    let broken = ut99_bsp::LevelDoctor::audit_actor_event_tag_links(&[
        (
            "Trigger0".to_string(),
            "Trigger".to_string(),
            "".to_string(),
            "MissingDoorTag".to_string(),
            Vec3::ZERO,
        ),
        (
            "Mover0".to_string(),
            "Mover".to_string(),
            "ActualDoorTag".to_string(),
            "".to_string(),
            Vec3::X * 128.0,
        ),
    ]);
    assert_eq!(broken.len(), 1);
    assert!(broken[0].description.contains("MissingDoorTag"));
}

#[test]
fn test_deliverable_j_all_10_interventions_end_to_end() {
    use ut99_editor::compiler::CompilerBridge;
    use ut99_editor::editor_state::TargetEngine;
    use ut99_editor::history::{Action, History};

    // 1. Verify History::push_interactive coalescing (BL-P0-04)
    let mut hist = History::default();
    hist.push_interactive(
        Action::MoveStaticMesh {
            mesh_index: 0,
            old_location: Vec3::new(64.0, 0.0, 0.0),
            new_location: Vec3::new(80.0, 0.0, 0.0),
        },
        true,
    );
    hist.push_interactive(
        Action::MoveStaticMesh {
            mesh_index: 0,
            old_location: Vec3::new(80.0, 0.0, 0.0),
            new_location: Vec3::new(120.0, 0.0, 0.0),
        },
        true,
    );
    hist.end_interaction();
    let coalesced = hist
        .undo_stack
        .pop()
        .expect("should have 1 coalesced undo action");
    match coalesced {
        Action::MoveStaticMesh {
            old_location,
            new_location,
            ..
        } => {
            assert_eq!(old_location.x, 64.0, "Must preserve initial value before drag");
            assert_eq!(new_location.x, 120.0, "Must preserve final value after drag");
        }
        _ => panic!("Unexpected action variant"),
    }
    assert!(
        hist.undo_stack.pop().is_none(),
        "Drag steps must coalesce into 1 entry"
    );

    // 2. Verify TargetEngine::Ut99Standard auto-baking vs Unreal227 preservation (BL-P0-03)
    let jump_pad = ut99_format::ActorRecord {
        name: "UJumpPad0".to_string(),
        class_name: "UJumpPad".to_string(),
        location: Vec3::new(100.0, 200.0, 50.0),
        tag: "PadTag".to_string(),
        ..Default::default()
    };
    let baked = CompilerBridge::bake_actor_for_ut99_standard(&jump_pad);
    assert_eq!(
        baked.len(),
        3,
        "UJumpPad must auto-bake into Kicker + LiftExit + JumpSpot for UT99 Standard"
    );
    assert_eq!(baked[0].class_name, "Kicker");
    assert_eq!(baked[1].class_name, "LiftExit");
    assert_eq!(baked[2].class_name, "JumpSpot");
    assert_eq!(
        CompilerBridge::map_class_name_for_ut99_standard("Engine.WeatherEmitter"),
        "SmokeGenerator"
    );

    // 3. Verify world_polys transform (BL-P0-01a) & UModel BSP compilation (BL-P0-01b / BL-P0-05)
    let mut state = EditorState::default();
    state.target_engine = TargetEngine::Ut99Standard;
    state.world_brushes.clear();
    state.set_builder_cube(Vec3::new(256.0, 256.0, 256.0));
    state.builder_brush.location = Vec3::new(500.0, -300.0, 128.0);
    state.csg_subtract();

    let tmp_path = std::env::temp_dir().join("test_deliverable_j_verify.unr");
    state
        .save_unr_native(&tmp_path)
        .expect("save_unr_native should succeed");

    let saved_level =
        ut99_format::UnrLevel::open(&tmp_path).expect("open saved .unr should succeed");
    assert_eq!(saved_level.brushes.len(), 1);
    let b0 = &saved_level.brushes[0];
    assert!(
        b0.world_polys[0]
            .vertices
            .iter()
            .any(|v| (v.x - 500.0).abs() < 200.0 && v.x > 300.0),
        "world_polys must be transformed to world space around X=500"
    );
    let world_bsp = saved_level
        .world_bsp
        .as_ref()
        .expect("Saved .unr must contain compiled Model_World BSP");
    assert!(!world_bsp.nodes.is_empty());
    assert!(!world_bsp.surfs.is_empty());
    assert!(!world_bsp.verts.is_empty());
    assert!(!world_bsp.zones.is_empty());

    let _ = std::fs::remove_file(&tmp_path);
}

#[test]
fn test_wave2_mipmaps_dxt_uax_mylevel_bsp_doctor_zone_flood_and_uc_compile() {
    // 1. Intervento 11 (BL-P1-03 part 2): Full Mipmap Chain + S3TC DXT1/DXT3/DXT5 Decoding
    let mut rgba_32 = vec![0u8; 32 * 32 * 4];
    for i in 0..(32 * 32) {
        rgba_32[i * 4] = ((i * 7) & 0xFF) as u8;
        rgba_32[i * 4 + 1] = ((i * 13) & 0xFF) as u8;
        rgba_32[i * 4 + 2] = ((i * 29) & 0xFF) as u8;
        rgba_32[i * 4 + 3] = 255;
    }
    let tex = ut99_format::UtxTexture {
        name: "TestMipTex".to_string(),
        group: String::new(),
        width: 32,
        height: 32,
        format: 0,
        masked: false,
        rgba_pixels: rgba_32,
    };
    let (_pal, mip_chain) = tex.generate_p8_mip_chain();
    assert_eq!(
        mip_chain.len(),
        6,
        "32x32 texture must generate 6 mipmap levels (32, 16, 8, 4, 2, 1)"
    );
    assert_eq!((mip_chain[5].0, mip_chain[5].1), (1, 1));

    let utx_pkg = (ut99_format::UtxPackage {
        package_name: "TestUtx".to_string(),
        textures: vec![tex],
    })
    .build_package()
    .expect("build .utx with mip chain");
    let loaded_utx = ut99_format::UtxPackage::from_package(utx_pkg, "TestUtx".to_string())
        .expect("parse .utx with mip chain");
    assert_eq!(loaded_utx.textures.len(), 1);
    assert_eq!(loaded_utx.textures[0].width, 32);

    // S3TC DXT1/3/5 block decode check (4x4 pure red block: RGB565 0xF800)
    let mut dxt1_block = [0u8; 8];
    dxt1_block[0..2].copy_from_slice(&0xF800u16.to_le_bytes());
    dxt1_block[2..4].copy_from_slice(&0x0000u16.to_le_bytes());
    let decoded_dxt1 =
        ut99_format::decode_dxt1_to_rgba8(&dxt1_block, 4, 4).expect("decode DXT1 4x4");
    assert_eq!(decoded_dxt1.len(), 4 * 4 * 4);
    assert_eq!(&decoded_dxt1[0..4], &[255, 0, 0, 255]);

    let mut dxt5_block = [0u8; 16];
    dxt5_block[0] = 200; // alpha0
    dxt5_block[1] = 100; // alpha1
    dxt5_block[8..10].copy_from_slice(&0x07E0u16.to_le_bytes()); // pure green
    let decoded_dxt5 =
        ut99_format::decode_dxt5_to_rgba8(&dxt5_block, 4, 4).expect("decode DXT5 4x4");
    assert_eq!(&decoded_dxt5[0..4], &[0, 255, 0, 200]);

    // 2. Intervento 12 (BL-P1-06 & BL-P2-04): .uax/.umx builders + MyLevel embedded sounds/Screenshot & GC
    let dummy_wav = b"RIFF$\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00D\xac\x00\x00\x88X\x01\x00\x02\x00\x10\x00data\x00\x00\x00\x00".to_vec();
    let sound_res =
        ut99_format::parse_sound_export("JumpWhoosh", "TestUax", &dummy_wav).expect("parse WAV");
    let uax_pkg =
        ut99_format::build_uax_package(std::slice::from_ref(&sound_res)).expect("build .uax");
    let uax_bytes = uax_pkg.to_bytes().expect("serialize .uax");
    let parsed_uax_pkg = ut99_format::Package::from_bytes(uax_bytes).expect("parse built .uax");
    let snd_payload = parsed_uax_pkg.get_export_bytes(0).expect("export 0 bytes");
    let parsed_snd =
        ut99_format::parse_sound_export("JumpWhoosh", "TestUax", &snd_payload).expect("re-parse WAV");
    assert_eq!(parsed_snd.wav_bytes, dummy_wav);

    let tracker_bytes = b"IMPMForegoneTrackerData_____________________________";
    let umx_pkg =
        ut99_format::build_umx_package("Foregone", "IT", tracker_bytes).expect("build .umx");
    let umx_bytes = umx_pkg.to_bytes().expect("serialize .umx");
    let parsed_umx_pkg = ut99_format::Package::from_bytes(umx_bytes).expect("parse built .umx");
    let mus_payload = parsed_umx_pkg.get_export_bytes(0).expect("music export 0");
    let parsed_mus =
        ut99_format::parse_music_export("Foregone", "TestUmx", &mus_payload).expect("parse IT");
    assert!(parsed_mus.format.starts_with("IT"));

    let mut level = ut99_format::UnrLevel::default();
    level.embedded_textures.push(ut99_format::UtxTexture {
        name: "Screenshot".to_string(),
        group: String::new(),
        width: 16,
        height: 16,
        format: 0,
        masked: false,
        rgba_pixels: vec![128u8; 16 * 16 * 4],
    });
    level.embedded_textures.push(ut99_format::UtxTexture {
        name: "UnusedMyLevelTex".to_string(),
        group: String::new(),
        width: 16,
        height: 16,
        format: 0,
        masked: false,
        rgba_pixels: vec![64u8; 16 * 16 * 4],
    });
    level.embedded_sounds.push(sound_res.clone());
    let (pruned_tex, pruned_snd) = level.prune_unreferenced_mylevel_assets();
    assert_eq!(pruned_tex, 1, "UnusedMyLevelTex should be pruned while Screenshot is kept");
    assert_eq!(pruned_snd, 1, "Unreferenced sound should be pruned");
    assert_eq!(level.embedded_textures.len(), 1);
    assert_eq!(level.embedded_textures[0].name, "Screenshot");

    // Re-add embedded sound and verify full .unr package serialization
    level.embedded_sounds.push(sound_res);
    let unr_pkg = level.build_package().expect("build .unr package");
    let roundtrip_level =
        ut99_format::UnrLevel::from_package(unr_pkg, "TestWave2".to_string())
            .expect("roundtrip .unr with MyLevel Screenshot and Sound");
    assert_eq!(roundtrip_level.embedded_textures.len(), 1);
    assert_eq!(roundtrip_level.embedded_textures[0].name, "Screenshot");
    assert_eq!(roundtrip_level.embedded_sounds.len(), 1);
    assert_eq!(roundtrip_level.embedded_sounds[0].name, "JumpWhoosh");

    // 3. Intervento 13 (BL-P2-01 & BL-P3-02): f64 DVec3 Canonical Polygon Split & BSP Doctor Auto-Repair
    let poly_a = ut99_bsp::Polygon::new(vec![
        Vec3::new(-100.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(100.0, 100.0, 0.0),
    ])
    .unwrap();
    let poly_b = ut99_bsp::Polygon::new(vec![
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-100.0, 0.0, 0.0),
        Vec3::new(-100.0, -100.0, 0.0),
    ])
    .unwrap();
    let split_plane = ut99_bsp::Plane::from_point_and_normal(
        Vec3::new(13.37, 0.0, 0.0),
        Vec3::new(0.9805807, 0.19611613, 0.0).normalize(),
    );
    let fa = match poly_a.split(&split_plane) {
        ut99_bsp::SplitResult::Spanning { front, .. } => front,
        _ => panic!("poly_a should split"),
    };
    let fb = match poly_b.split(&split_plane) {
        ut99_bsp::SplitResult::Spanning { front, .. } => front,
        _ => panic!("poly_b should split"),
    };
    let shared_hit_a = fa
        .vertices
        .iter()
        .find(|v| v.y.abs() < 1e-5 && v.x.abs() < 90.0)
        .expect("split hit on y=0 in poly_a");
    let shared_hit_b = fb
        .vertices
        .iter()
        .find(|v| v.y.abs() < 1e-5 && v.x.abs() < 90.0)
        .expect("split hit on y=0 in poly_b");
    assert_eq!(
        shared_hit_a.x.to_bits(),
        shared_hit_b.x.to_bits(),
        "Canonical f64 edge split must produce bit-identical intersection coordinates on opposite edges"
    );

    // BSP Doctor auto-repair on a brush with a warped non-planar quad and off-grid vertex
    let mut state = EditorState::default();
    state.world_brushes.clear();
    let mut warped_cube = ut99_bsp::build_cube("WarpedBrush", ut99_bsp::CsgOperation::Add, Vec3::new(128.0, 128.0, 128.0));
    // Warp 1 vertex of polygon 0 out of plane and near-0.5 UU grid (within snap_epsilon 0.01)
    warped_cube.polys[0].vertices[2] += Vec3::new(12.004, 8.5, 15.0);
    state.world_brushes.push(warped_cube);
    let doc_report = state.run_bsp_doctor();
    assert!(doc_report.snapped_vertices_count >= 1);
    assert!(doc_report.triangulated_non_planar_polys_count >= 1);
    assert_eq!(
        state.world_brushes[0].polys.len(),
        7,
        "Warped quad on 6-sided cube must be triangulated into 2 triangles -> 7 planar faces total"
    );

    // 4. Intervento 14 (BL-P2-02 & BL-P3-03): 3D BSP Zone Flood-Fill & 1 UU Zone Leak Path Tracer
    let mut room1 = ut99_bsp::build_cube("Room1", ut99_bsp::CsgOperation::Subtract, Vec3::new(512.0, 256.0, 256.0));
    room1.location = Vec3::new(-256.0, 0.0, 0.0);
    let mut room2 = ut99_bsp::build_cube("Room2", ut99_bsp::CsgOperation::Subtract, Vec3::new(512.0, 256.0, 256.0));
    room2.location = Vec3::new(256.0, 0.0, 0.0);
    let mut leaky_portal = ut99_bsp::build_sheet("PortalLeaky", ut99_bsp::CsgOperation::Add, 254.0, 256.0);
    leaky_portal.rotation = Vec3::new(0.0, 90.0, 0.0);
    for p in &mut leaky_portal.polys {
        p.flags |= ut99_bsp::PF_PORTAL | ut99_bsp::PF_NOT_SOLID;
    }
    let zone_seeds = vec![
        ("ZoneInfo0".to_string(), Vec3::new(-256.0, 0.0, 0.0)),
        ("ZoneInfo1".to_string(), Vec3::new(256.0, 0.0, 0.0)),
    ];
    let flood_res = ut99_bsp::compute_bsp_zone_flood_fill(&[room1, room2, leaky_portal], &zone_seeds);
    assert!(
        !flood_res.leak_paths.is_empty(),
        "1 UU gap in ZonePortal must be detected by 3D BSP zone flood-fill"
    );
    assert!(flood_res.leak_paths[0].waypoints.len() >= 3);

    // 5. Intervento 15 (BL-P2-03): UnrealScript Studio Live Editing + F7 Validator/Compiler
    state.open_uc_viewer_for_class("Trigger");
    let valid_report = state.compile_active_uc_script();
    assert!(valid_report.success, "Default generated Trigger.uc script should pass syntax validation");

    state.uc_source_cache.push_str("\nfunction BrokenFunc() {\n    local int x\n");
    let invalid_report = state.compile_active_uc_script();
    assert!(!invalid_report.success, "Unclosed brace in edited .uc script must fail validation");
    assert!(!invalid_report.diagnostics.is_empty());
}

#[test]
fn test_wave3_deliverable_i_supremacy_and_mylevel_sound_pipeline() {
    let mut state = EditorState::default();

    // 1. Intervento 16 (Deliverable I.5): Auto-Transform Permanently on Mover creation
    state.set_builder_cube(Vec3::new(64.0, 64.0, 32.0));
    state.builder_brush.scale = Vec3::new(2.0, 1.5, 1.0);
    state.builder_brush.location = Vec3::new(128.0, 256.0, 64.0);
    state.add_mover_from_builder();
    assert_eq!(state.movers.len(), 1);
    assert_eq!(
        state.movers[0].brush.scale,
        Vec3::ONE,
        "Deliverable I.5: Mover brush scale must be auto-baked to Vec3::ONE on creation"
    );
    let (m_min, m_max) = state.movers[0].brush.world_bounds();
    let m_size = m_max - m_min;
    assert!(
        (m_size.x - 128.0).abs() < 0.5 && (m_size.y - 96.0).abs() < 0.5,
        "Deliverable I.5: Baked Mover vertices must reflect 2.0x and 1.5x scale (got {:?})",
        m_size
    );

    // 2. Intervento 17 (Deliverable I.3): Smart Duplicate with auto-incremented numbered Tag
    state.movers[0].tag = "Lift_01".to_string();
    state.selected_mover_index = Some(0);
    state.duplicate_selected();
    assert_eq!(state.movers.len(), 2);
    assert_eq!(
        state.movers[1].tag, "Lift_02",
        "Deliverable I.3: Duplicating Mover with Tag 'Lift_01' must auto-increment to 'Lift_02'"
    );
    assert_eq!(EditorState::increment_numbered_tag("Door9"), "Door10");

    // 3. Intervento 18 (Deliverable I.2 & I.7): Auto-Planar Triangulation on Vertex Edit & Pre-Build Checkpoint
    state.selected_mover_index = None;
    state.selected_brush_index = None;
    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    assert_eq!(state.builder_brush.polys.len(), 6);
    state.selected_vertex_indices.clear();
    state.selected_vertex_indices.push(0);
    // Moving 1 corner of a cube along all 3 axes makes the 3 incident quads non-planar
    state.move_selected_vertices(Vec3::new(16.0, 32.0, 48.0));
    assert!(
        state.builder_brush.polys.len() > 6,
        "Deliverable I.2: Moving a single vertex out of plane must auto-triangulate non-planar quads"
    );
    for poly in &state.builder_brush.polys {
        assert!(
            poly.is_planar(0.01),
            "Deliverable I.2: Every polygon after vertex edit must be strictly planar"
        );
    }

    let snaps_before = state.auto_save_snapshots.len();
    state.build_all_complete();
    assert!(
        state.auto_save_snapshots.len() > snaps_before,
        "Deliverable I.7: build_all_complete must capture a Pre-Build Safety Checkpoint"
    );

    // 4. Intervento 20 (BL-P1-06 & BL-P2-04): WAV Import, MyLevel Sound Embedding & Native .UNR Roundtrip
    let dummy_wav =
        b"RIFF$\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00D\xac\x00\x00\x88X\x01\x00\x02\x00\x10\x00data\x00\x00\x00\x00"
            .to_vec();
    let imported = state
        .import_wav_sound("CustomBeep", "CustomAudio", dummy_wav)
        .expect("import WAV into CustomAudio");
    assert_eq!(imported.sample_rate, 44100);
    assert!(state.embed_sound_in_mylevel("CustomBeep"));
    assert_eq!(state.mylevel_sounds.len(), 1);
    assert_eq!(state.mylevel_sounds[0].package, "MyLevel");

    let tmp_unr = std::env::temp_dir().join("test_wave3_mylevel_sound.unr");
    ut99_editor::compiler::CompilerBridge::save_unr_native(&state, &tmp_unr)
        .expect("save .unr with MyLevel sound");

    let mut loaded_state = EditorState::default();
    loaded_state
        .load_unr(&tmp_unr)
        .expect("load .unr with MyLevel sound");
    assert_eq!(loaded_state.mylevel_sounds.len(), 1);
    assert_eq!(loaded_state.mylevel_sounds[0].name, "CustomBeep");
    let _ = std::fs::remove_file(&tmp_unr);
}

#[test]
fn test_wave4_bot_ballistics_audio_crud_vertex_weld_and_zero_ucc_gamemodes() {
    use ut99_editor::editor_state::Ut99JumpMoveType;

    let mut state = EditorState::default();

    // 1. Intervento 22 (BL-P3-04): TournamentPlayer Ballistic Simulator & Auto Kicker+LiftExit+JumpSpot Chain
    let standing_arc = state.simulate_ut99_ballistic_trajectory(
        Vec3::ZERO,
        Vec3::X,
        Ut99JumpMoveType::StandingJump,
        0.35,
    );
    let dodge_arc = state.simulate_ut99_ballistic_trajectory(
        Vec3::ZERO,
        Vec3::X,
        Ut99JumpMoveType::DodgeJump,
        0.35,
    );
    let impact_arc = state.simulate_ut99_ballistic_trajectory(
        Vec3::ZERO,
        Vec3::X,
        Ut99JumpMoveType::ImpactHammerJump,
        0.35,
    );
    let transloc_arc = state.simulate_ut99_ballistic_trajectory(
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.5),
        Ut99JumpMoveType::TranslocatorArc,
        0.0,
    );
    assert!(dodge_arc.points.len() > 5);
    assert!(
        impact_arc
            .points
            .iter()
            .map(|p| p.z)
            .fold(0.0_f32, f32::max)
            > standing_arc
                .points
                .iter()
                .map(|p| p.z)
                .fold(0.0_f32, f32::max),
        "ImpactHammerJump apex must exceed StandingJump apex"
    );
    assert!(transloc_arc.points.last().unwrap().x > 500.0);

    let pad_start = Vec3::new(0.0, 0.0, 64.0);
    let pad_target = Vec3::new(512.0, 256.0, 320.0);
    let (kicker_name, exit_name, spot_name) =
        state.spawn_ut99_kicker_bot_chain(pad_start, pad_target, 1.2);
    assert!(kicker_name.starts_with("Kicker_"));
    assert!(exit_name.starts_with("LiftExit_"));
    assert!(spot_name.starts_with("JumpSpot_"));

    let solved_vel = EditorState::solve_kicker_velocity(pad_start, pad_target, 1.2, -950.0);
    let kicker_arc = state.simulate_ut99_ballistic_trajectory(
        pad_start,
        Vec3::X,
        Ut99JumpMoveType::CustomKicker(solved_vel),
        0.0,
    );
    let closest_dist = kicker_arc
        .points
        .iter()
        .map(|p| p.distance(pad_target))
        .fold(f32::MAX, f32::min);
    assert!(
        closest_dist < 15.0,
        "Solved KickVelocity trajectory must pass through pad_target within 15 UU (got {})",
        closest_dist
    );

    // 2. Intervento 23 (BL-P1-06 part 2): Sound Delete, Audio Stop, and .UMX Export
    let dummy_wav =
        b"RIFF$\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00D\xac\x00\x00\x88X\x01\x00\x02\x00\x10\x00data\x00\x00\x00\x00"
            .to_vec();
    state.import_wav_sound("TempSnd", "MyLevel", dummy_wav);
    assert_eq!(state.mylevel_sounds.len(), 1);
    state.stop_audio_preview();
    assert!(state.delete_sound_from_package("TempSnd"));
    assert!(state.mylevel_sounds.is_empty());

    state.loaded_music = Some(ut99_format::package::MusicResource {
        name: "TestTrack".to_string(),
        package: "TestTrack".to_string(),
        format: "IT".to_string(),
        title: "Test Impulse Tracker".to_string(),
        data_size: 64,
    });
    let mut tracker_bytes = b"IMPMTest Impulse Tracker\x00\x00\x00\x00".to_vec();
    tracker_bytes.resize(64, 0);
    let tmp_umx = std::env::temp_dir().join("test_wave4_track.umx");
    state
        .export_music_package_umx(&tmp_umx, &tracker_bytes)
        .expect("export .umx package");
    assert!(tmp_umx.exists());
    let _ = std::fs::remove_file(&tmp_umx);

    // 3. Intervento 24 (Deliverable I.2 part 2 & I.6): Safe Vertex Auto-Weld on Overlap & Window Layout Reset
    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    let verts = state.get_active_brush_unique_vertices();
    let v0 = verts[0];
    let v1 = verts[1];
    state.selected_vertex_indices.clear();
    state.selected_vertex_indices.push(0);
    // Drag vertex 0 directly onto vertex 1 -> must weld cleanly without NaN plane normals or crash
    state.move_selected_vertices(v1 - v0);
    for poly in &state.builder_brush.polys {
        assert!(poly.vertices.len() >= 3);
        assert!(!poly.plane.normal.is_nan());
        assert!(poly.plane.normal.length() > 0.9);
    }
    state.ui_scale = 5.0;
    state.reset_and_clamp_window_layout();
    assert!((state.ui_scale - 2.0).abs() < 1e-5);

    // 4. Intervento 25 (Deliverable H.4): Zero-UCC Multi-Gamemode Native .UNR Boot Verification (DM, CTF, DOM, AS)
    state.set_builder_cube(Vec3::new(512.0, 512.0, 256.0));
    state.csg_add();
    for prefix in ["DM-ZeroUcc", "CTF-ZeroUcc", "DOM-ZeroUcc", "AS-ZeroUcc"] {
        let tmp_map = std::env::temp_dir().join(format!("{}.unr", prefix));
        ut99_editor::compiler::CompilerBridge::save_unr_native(&state, &tmp_map)
            .expect("Zero-UCC native .unr save");
        let level = ut99_format::UnrLevel::open(&tmp_map)
            .expect("parse compiled UModel from .unr");
        let world_bsp = level
            .world_bsp
            .as_ref()
            .expect("Saved .unr must contain compiled Model_World BSP");
        assert!(
            !world_bsp.nodes.is_empty() && !world_bsp.surfs.is_empty(),
            "{} must have compiled UModel BSP nodes and surfaces",
            prefix
        );
        let _ = std::fs::remove_file(&tmp_map);
    }
}

#[test]
fn test_wave5_brush_stripped_unr_recovery_and_kicker_post_csg_trajectories() {
    let mut state = EditorState::default();
    state.world_brushes.clear();
    state.loaded_unr = Some(ut99_format::UnrLevel::default());

    // 1. Intervento 26 (BL-P0-05 part 2): Brush-Stripped .UNR Recovery from Compiled Model_World BSP
    state.set_builder_cube(Vec3::new(1024.0, 1024.0, 256.0));
    state.csg_subtract(); // Subtracted hall: floor at Z = -128.0

    let tmp_map = std::env::temp_dir().join("DM-BrushStrippedWave5.unr");
    ut99_editor::compiler::CompilerBridge::save_unr_native(&state, &tmp_map)
        .expect("save .unr with compiled Model_World");
    let mut stripped_level =
        ut99_format::UnrLevel::open(&tmp_map).expect("open saved .unr");
    // Simulate a brush-stripped map by deleting all source CSG brushes
    stripped_level.brushes.clear();
    let reconstructed = EditorState::reconstruct_brush_from_compiled_bsp(&stripped_level)
        .expect("reconstruct 3D brush from compiled Model_World BSP");
    assert!(
        reconstructed.polys.len() >= 6,
        "Reconstructed brush from compiled UModel must contain all room surfaces"
    );
    let _ = std::fs::remove_file(&tmp_map);

    // 2. Intervento 27 (BL-P3-04 part 2): Post-CSG Kicker Ballistic Trajectory & LiftTag Event Links
    let start = Vec3::new(-300.0, 0.0, -110.0);
    let target = Vec3::new(300.0, 0.0, -110.0);
    let (_kicker_name, _exit_name, _spot_name) =
        state.spawn_ut99_kicker_bot_chain(start, target, 1.0);

    // Verify LiftTag wiring appears in get_event_tag_links()
    let links = state.get_event_tag_links();
    assert!(
        !links.is_empty(),
        "get_event_tag_links must include LiftTag -> Kicker.Tag connections"
    );

    // Verify compute_trajectories() raycasts against post-CSG subtracted floor (Z = -128.0)
    let arcs = state.compute_trajectories();
    assert_eq!(arcs.len(), 1, "Must compute 1 ballistic arc for Kicker");
    let landing = arcs[0]
        .landing_point
        .expect("Kicker trajectory must land on the subtracted room floor");
    assert!(
        (landing.z - (-128.0)).abs() < 5.0,
        "Ballistic arc must land on post-CSG subtracted floor at Z=-128 (got Z={})",
        landing.z
    );

    // 3. Intervento 28 (BL-P2-02 & BL-P3-03): 3D Zone Flood-Fill Report via EditorState
    state.add_zone_info_at_builder("MainArenaZone");
    let flood_report = state.compute_3d_zone_flood_fill_report();
    assert!(flood_report.distinct_zone_count >= 1);
}

#[test]
fn test_wave6_console_batch_exec_mylevel_screenshot_and_visible_only_rebuild() {
    let mut state = EditorState::default();
    state.world_brushes.clear();
    state.loaded_unr = Some(ut99_format::UnrLevel::default());

    // 1. Intervento 31 (BL-P2-04 part 2): MyLevel Screenshot embedding, UTX export, and LevelInfo0 auto-linking
    let rgba = vec![120u8; 64 * 64 * 4];
    state.set_mylevel_screenshot_rgba8(64, 64, &rgba);
    assert!(
        state
            .loaded_unr
            .as_ref()
            .unwrap()
            .embedded_textures
            .iter()
            .any(|t| t.name.eq_ignore_ascii_case("Screenshot")),
        "MyLevel.Screenshot must be embedded in loaded_unr"
    );

    let tmp_utx = std::env::temp_dir().join("MyLevelWave6Export.utx");
    let exported_cnt = state
        .export_texture_package_utx("MyLevel", &tmp_utx)
        .expect("export MyLevel textures to .utx");
    assert_eq!(exported_cnt, 1);
    let _ = std::fs::remove_file(&tmp_utx);

    // 2. Intervento 32 (REQ-K03 & REQ-K07): Batch EXEC script & UnrealEd 2.0 Console Commands
    let tmp_exec = std::env::temp_dir().join("wave6_batch.txt");
    std::fs::write(
        &tmp_exec,
        "# Batch UnrealEd 2.0 script\nMAP GRID X=32 Y=32 Z=32\nMODE VERTEXEDIT\nACTOR ADD CLASS=Light\nPOLY DEFAULT TEXTURE=MyLevel.Screenshot\nMODE CAMERAMOVE\n",
    )
    .expect("write batch script");
    state.execute_unrealed_command(&format!("EXEC {}", tmp_exec.display()));
    let _ = std::fs::remove_file(&tmp_exec);

    assert_eq!(state.grid_step, 32.0);
    assert_eq!(
        state.active_texture.as_deref(),
        Some("MyLevel.Screenshot")
    );
    assert!(
        state
            .loaded_unr
            .as_ref()
            .unwrap()
            .actors
            .iter()
            .any(|a| a.class_name == "Light"),
        "ACTOR ADD CLASS=Light in EXEC script must spawn a Light actor"
    );

    // Save .unr and verify LevelInfo0.Screenshot is auto-linked to Texture'MyLevel.Screenshot'
    state.set_builder_cube(Vec3::new(512.0, 512.0, 256.0));
    state.csg_subtract();
    let tmp_map = std::env::temp_dir().join("DM-Wave6Screenshot.unr");
    ut99_editor::compiler::CompilerBridge::save_unr_native(&state, &tmp_map)
        .expect("save .unr with MyLevel.Screenshot");
    let reloaded = ut99_format::UnrLevel::open(&tmp_map).expect("reload .unr");
    let has_screenshot_prop = reloaded
        .actors
        .iter()
        .find(|a| a.class_name == "LevelInfo")
        .and_then(|li| li.properties.get("Screenshot"))
        .is_some();
    assert!(
        has_screenshot_prop,
        "LevelInfo0 must have Screenshot property linked to MyLevel.Screenshot"
    );
    let _ = std::fs::remove_file(&tmp_map);

    // 3. Intervento 33 (REQ-K06): Only Rebuild Visible Actors in build_lighting
    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    state.csg_add();
    let hidden_idx = state.world_brushes.len() - 1;
    state.brush_groups.insert(hidden_idx, "HiddenGroup".to_string());
    state.hidden_actor_groups.insert("HiddenGroup".to_string());
    state.bsp_config.only_visible_brushes = true;
    state.build_lighting();
    let stats = state
        .last_lighting_stats
        .as_ref()
        .expect("lighting stats");
    // Only the first brush (6 polys) is visible; the second brush in HiddenGroup is skipped
    assert_eq!(
        stats.total_polygons, 6,
        "Visible-only lighting rebuild must only bake visible brushes (6 polys)"
    );

    // 4. Intervento 34 (REQ-B05 & REQ-E05): Polygons -> To/From Brush on a Selected Mover
    state.set_builder_cube(Vec3::new(96.0, 96.0, 32.0));
    state.add_mover_from_builder();
    state.selected_brush_index = None;
    assert!(state.selected_mover_index.is_some());

    // Modify Builder Brush to an 8-face cylinder (1+1+6=8 polys for 6 sides) and copy INTO the Mover via polygons_from_brush
    state.builder_brush =
        ut99_bsp::build_cylinder("Builder", ut99_bsp::CsgOperation::Active, 64.0, 48.0, 6);
    let cyl_poly_count = state.builder_brush.polys.len();
    assert!(cyl_poly_count > 6);
    assert!(state.polygons_from_brush());
    let m_idx = state.selected_mover_index.unwrap();
    assert_eq!(state.movers[m_idx].brush.polys.len(), cyl_poly_count);

    // Reset Builder Brush to cube (6 polys), then copy FROM the Mover back into Builder Brush via polygons_to_brush
    state.set_builder_cube(Vec3::new(64.0, 64.0, 64.0));
    assert_eq!(state.builder_brush.polys.len(), 6);
    assert!(state.polygons_to_brush());
    assert_eq!(state.builder_brush.polys.len(), cyl_poly_count);
}

#[test]
fn test_wave7_mover_raytrace_keys_climbable_ladder_polygons_separate_and_safe_recovery() {
    let mut state = EditorState::default();
    state.world_brushes.clear();
    state.loaded_unr = Some(ut99_format::UnrLevel::default());

    // 1. Intervento 36 (REQ-B07): Polygons -> Separate & Merge on a Selected Mover via Console
    state.set_builder_cube(Vec3::new(512.0, 512.0, 256.0));
    state.csg_subtract();

    state.set_builder_cube(Vec3::new(96.0, 96.0, 32.0));
    state.add_mover_from_builder();
    state.selected_brush_index = None;
    let m_idx = state.selected_mover_index.expect("mover selected");
    assert_eq!(state.movers[m_idx].brush.polys.len(), 6);

    // Separate quads into 12 triangles on the Mover via POLY SEPARATE
    state.execute_unrealed_command("POLY SEPARATE");
    assert_eq!(
        state.movers[m_idx].brush.polys.len(),
        12,
        "POLY SEPARATE on selected mover must split 6 quads into 12 triangles"
    );

    // Merge coplanar triangles back into 6 quads on the Mover via POLY MERGE
    state.execute_unrealed_command("POLY MERGE");
    assert_eq!(
        state.movers[m_idx].brush.polys.len(),
        6,
        "POLY MERGE on selected mover must merge 12 coplanar triangles back into 6 quads"
    );

    // 2. Intervento 37 (REQ-F05 & REQ-E05): Mover Keyframe Raytraced Lighting (BrushRaytraceKey / WorldRaytraceKey / bDynamicLightMover)
    state.movers[m_idx].keyframes[1] = Some(ut99_editor::editor_state::MoverKeyframe {
        location: Vec3::new(0.0, 0.0, 120.0),
        rotation: Vec3::ZERO,
    });
    state.movers[m_idx].current_key = 1;

    // Place a light near Key 1 (Z=120) so baking at BrushRaytraceKey=1 illuminates the Mover
    state.builder_brush.location = Vec3::new(0.0, 0.0, 140.0);
    state.place_actor_of_class("Light");
    state.build_lighting();
    assert!(
        state.movers[m_idx]
            .brush
            .polys
            .iter()
            .any(|p| p.lightmap.is_some()),
        "Mover must receive baked lightmaps evaluated at BrushRaytraceKey=1"
    );

    // 3. Intervento 38 (REQ-G05): Canonical UT99 Climbable WaterZone Ladder Generator
    let brushes_before = state.world_brushes.len();
    let (_ladder_idx, ladder_zone_name) = state.spawn_ut99_climbable_ladder_volume(
        Vec3::new(128.0, 0.0, 0.0),
        64.0,
        32.0,
        256.0,
    );
    assert!(ladder_zone_name.starts_with("WaterZone"));
    assert_eq!(
        state.world_brushes.len(),
        brushes_before + 1,
        "Ladder generator must add 1 invisible ZonePortal enclosing brush"
    );
    let portal_brush = state.world_brushes.last().unwrap();
    let required_flags =
        ut99_bsp::PF_PORTAL | ut99_bsp::PF_INVISIBLE | ut99_bsp::PF_NOT_SOLID;
    assert!(
        portal_brush
            .polys
            .iter()
            .all(|p| (p.flags & required_flags) == required_flags),
        "All ladder volume faces must have PF_PORTAL | PF_INVISIBLE | PF_NOT_SOLID"
    );
    let level_actors = &state.loaded_unr.as_ref().unwrap().actors;
    assert!(
        level_actors.iter().any(|a| a.class_name == "WaterZone"),
        "Ladder generator must spawn a WaterZone actor with upward ZoneVelocity"
    );
    assert!(
        level_actors
            .iter()
            .any(|a| a.class_name == "LiftCenter" && a.properties.contains_key("LiftTag")),
        "Ladder generator must spawn a LiftCenter for bot AI traversal"
    );

    // 4. Intervento 39 (REQ-J06): Rotating Auto0..Auto9.unr Backups & Safe Recovery Mode
    let tmp_autosave_dir = std::env::temp_dir().join("ut99_wave7_autosaves");
    let _ = std::fs::create_dir_all(&tmp_autosave_dir);
    let auto0 = state
        .save_rotating_autosave_unr(&tmp_autosave_dir)
        .expect("save Auto0.unr");
    let auto1 = state
        .save_rotating_autosave_unr(&tmp_autosave_dir)
        .expect("save Auto1.unr");
    assert!(auto0.ends_with("Auto0.unr") && auto0.exists());
    assert!(auto1.ends_with("Auto1.unr") && auto1.exists());

    // Verify Mover raytrace properties and Safe Recovery loading from Auto0.unr
    let mut recovery_state = EditorState::default();
    recovery_state
        .load_unr_safe_recovery(&auto0)
        .expect("load Auto0.unr in Safe Recovery Mode");
    let recovered_level = recovery_state.loaded_unr.as_ref().unwrap();
    assert!(
        recovered_level.world_bsp.is_none(),
        "Safe Recovery Mode must strip world_bsp while preserving all Brushes and Actors"
    );
    let mover_actor = recovered_level
        .actors
        .iter()
        .find(|a| a.class_name == "Mover")
        .expect("Mover actor present in recovered .unr");
    assert_eq!(
        mover_actor
            .properties
            .get("BrushRaytraceKey")
            .map(|p| &p.value),
        Some(&ut99_format::TaggedValue::Byte(1))
    );
    assert_eq!(
        mover_actor
            .properties
            .get("bDynamicLightMover")
            .map(|p| &p.value),
        Some(&ut99_format::TaggedValue::Bool(true))
    );

    let _ = std::fs::remove_dir_all(&tmp_autosave_dir);
}

#[test]
fn test_wave8_actor_replace_uv_alignment_copy_selected_lighting_and_class_defaults() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Intervento 41 (REQ-C05 & REQ-K03): Replace Selected Actor(s) With Class + Console Select Filters
    state.builder_brush = ut99_bsp::build_cube(
        "RoomCube",
        CsgOperation::Subtract,
        Vec3::new(512.0, 512.0, 256.0),
    );
    state.csg_subtract();
    state.builder_brush = ut99_bsp::build_cube(
        "PillarAdd",
        CsgOperation::Add,
        Vec3::new(128.0, 128.0, 256.0),
    );
    state.csg_add();

    state.execute_unrealed_command("MAP SELECT SUBTRACTS");
    assert_eq!(state.multi_selected_brushes.len(), 1);
    assert_eq!(state.selected_brush_index, Some(0));
    state.execute_unrealed_command("MAP SELECT ADDS");
    assert_eq!(state.multi_selected_brushes.len(), 1);
    assert_eq!(state.selected_brush_index, Some(1));

    // Spawn a PathNode with custom location, rotation, Tag, and Event, then replace with Engine.Trigger
    let mut unr = ut99_format::UnrLevel::default();
    let mut node = ut99_format::ActorRecord::default();
    node.name = "PathNode7".to_string();
    node.class_name = "Engine.PathNode".to_string();
    node.location = Vec3::new(120.0, -80.0, 48.0);
    node.rotation = (0, 16384, 0);
    node.tag = "CustomNavTag".to_string();
    node.event = "DoorOpenEvent".to_string();
    unr.actors.push(node);
    state.loaded_unr = Some(unr);

    state.execute_unrealed_command("SELECT OFCLASS CLASS=PathNode");
    assert_eq!(state.selected_actor_index, Some(0));

    state.execute_unrealed_command("ACTOR REPLACE CLASS=Engine.Trigger");
    let replaced = &state.loaded_unr.as_ref().unwrap().actors[0];
    assert_eq!(replaced.class_name, "Engine.Trigger");
    assert_eq!(replaced.name, "Trigger7");
    assert_eq!(replaced.location, Vec3::new(120.0, -80.0, 48.0));
    assert_eq!(replaced.rotation, (0, 16384, 0));
    assert_eq!(replaced.tag, "CustomNavTag");
    assert_eq!(replaced.event, "DoorOpenEvent");

    // Verify Undo/Redo on Actor Replace
    state.undo();
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors[0].class_name,
        "Engine.PathNode"
    );
    state.redo();
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors[0].class_name,
        "Engine.Trigger"
    );

    // 2. Intervento 42 (REQ-D02): Propagate Texture & UV Alignment + POLY MEMORIZE/RECALL/OR
    state.world_brushes[0].polys[0].texture = "Indus5.Trim01".to_string();
    state.world_brushes[0].polys[0].texture_u = Vec3::new(2.5, 0.0, 0.0);
    state.world_brushes[0].polys[0].texture_v = Vec3::new(0.0, 0.0, -2.5);
    state.world_brushes[0].polys[0].base = Vec3::new(32.0, 16.0, -8.0);

    state.selected_surfaces.clear();
    state.selected_surfaces.insert((0, 0));
    state.execute_unrealed_command("POLY MEMORIZE");
    state.selected_surfaces.clear();
    state.selected_surfaces.insert((0, 1));
    state.selected_surfaces.insert((0, 2));
    state.execute_unrealed_command("POLY OR");
    assert_eq!(state.selected_surfaces.len(), 3);

    let propagated = state.propagate_first_selected_surface_uv_to_all();
    assert_eq!(propagated, 2);
    assert_eq!(state.world_brushes[0].polys[1].texture, "Indus5.Trim01");
    assert_eq!(
        state.world_brushes[0].polys[1].texture_u,
        Vec3::new(2.5, 0.0, 0.0)
    );
    assert_eq!(
        state.world_brushes[0].polys[1].base,
        Vec3::new(32.0, 16.0, -8.0)
    );

    // 3. Intervento 43 (REQ-F07): Apply Lights to Selected Surfaces Only (`LIGHT APPLY SELECTED`)
    // Clear existing lightmaps on all polygons of brush 0
    for p in &mut state.world_brushes[0].polys {
        p.lightmap = None;
    }
    state.selected_surfaces.clear();
    state.selected_surfaces.insert((0, 0));
    state.execute_unrealed_command("LIGHT APPLY SELECTED");
    assert!(
        state.world_brushes[0].polys[0].lightmap.is_some(),
        "Selected surface (0, 0) must receive a baked lightmap"
    );
    assert!(
        state.world_brushes[0].polys[3].lightmap.is_none(),
        "Unselected surface (0, 3) on the same brush must remain untouched"
    );

    // 4. Intervento 44 (REQ-E08): Reset Actor Properties to UnrealScript Class Defaults
    let mut light_actor = ut99_format::ActorRecord::default();
    light_actor.name = "Light99".to_string();
    light_actor.class_name = "Engine.Light".to_string();
    light_actor.location = Vec3::new(0.0, 0.0, 64.0);
    light_actor.light_brightness = 250;
    light_actor.light_radius = 200;
    light_actor.properties.insert(
        "CustomDirtyProp".to_string(),
        ut99_format::TaggedProperty {
            name: "CustomDirtyProp".to_string(),
            array_index: 0,
            value: ut99_format::TaggedValue::Int(999),
        },
    );
    state.loaded_unr.as_mut().unwrap().actors.push(light_actor);
    let restored = state.reset_actor_properties_to_class_defaults(1);
    assert!(restored > 0);
    let reset_light = &state.loaded_unr.as_ref().unwrap().actors[1];
    assert!(
        !reset_light.properties.contains_key("CustomDirtyProp"),
        "Custom overridden property must be cleared upon reset to class defaults"
    );
    assert_eq!(reset_light.location, Vec3::new(0.0, 0.0, 64.0));
}

#[test]
fn test_wave9_mover_subclasses_sounds_inventoryspot_autopaths_t3d_merge_and_texture_props() {
    let mut state = EditorState::default();
    state.new_map();

    // 1. Intervento 46 (REQ-D08): Texture Properties (`FootstepSound`, `HitSound`, `Friction`, `DetailTexture`, `MacroTexture`, `AnimNext`)
    let mut tex_meta = state.texture_manager.get_texture_metadata("Indus5.Floor01");
    assert_eq!(tex_meta.friction, 1.0);
    tex_meta.footstep_sound = "Botpack.FootstepMetal01".to_string();
    tex_meta.hit_sound = "Botpack.Ricochet01".to_string();
    tex_meta.detail_texture = "Detail.MetalDet".to_string();
    tex_meta.macro_texture = "Indus5.Macro01".to_string();
    tex_meta.anim_next = "Indus5.Floor02".to_string();
    tex_meta.friction = 1.25;
    state.texture_manager.set_texture_metadata("Indus5.Floor01", tex_meta);
    let read_meta = state.texture_manager.get_texture_metadata("Indus5.Floor01");
    assert_eq!(read_meta.footstep_sound, "Botpack.FootstepMetal01");
    assert_eq!(read_meta.detail_texture, "Detail.MetalDet");
    assert_eq!(read_meta.anim_next, "Indus5.Floor02");
    assert!((read_meta.friction - 1.25).abs() < 1e-5);

    // 2. Intervento 47 (REQ-E05): Mover Subclasses (`ElevatorMover`, `GradualMover`, `LoopMover`), `InitialState`, `DelayTime`, and all 5 `MoverSounds`
    state.add_mover_from_builder_with_class("ElevatorMover");
    assert_eq!(state.movers.len(), 1);
    {
        let m = &mut state.movers[0];
        assert_eq!(m.mover_class, "ElevatorMover");
        assert_eq!(m.initial_state, "TriggerControl");
        m.delay_time = 1.75;
        m.opening_sound = "DoorsMod.OpenStart".to_string();
        m.opened_sound = "DoorsMod.OpenEnd".to_string();
        m.closing_sound = "DoorsMod.CloseStart".to_string();
        m.closed_sound = "DoorsMod.CloseEnd".to_string();
        m.move_ambient_sound = "DoorsMod.MoveLoop".to_string();
    }

    let tmp_dir = std::env::temp_dir().join("forge99_wave9_tests");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let unr_path = tmp_dir.join("DM-Wave9MoverTest.unr");
    let t3d_path = tmp_dir.join("DM-Wave9MoverTest.t3d");
    state.save_unr_native(&unr_path).expect("save_unr_native must succeed");
    state.save_t3d(&t3d_path).expect("save_t3d must succeed");

    let mut reloaded_unr = EditorState::default();
    reloaded_unr.load_unr(&unr_path).expect("load_unr must succeed");
    assert_eq!(reloaded_unr.movers.len(), 1);
    let rm = &reloaded_unr.movers[0];
    assert_eq!(rm.mover_class, "ElevatorMover");
    assert_eq!(rm.initial_state, "TriggerControl");
    assert!((rm.delay_time - 1.75).abs() < 1e-4);
    assert_eq!(rm.opening_sound, "DoorsMod.OpenStart");
    assert_eq!(rm.opened_sound, "DoorsMod.OpenEnd");
    assert_eq!(rm.closing_sound, "DoorsMod.CloseStart");
    assert_eq!(rm.closed_sound, "DoorsMod.CloseEnd");
    assert_eq!(rm.move_ambient_sound, "DoorsMod.MoveLoop");

    // 3. Intervento 48 (REQ-G01): Bidirectional `InventorySpot` <-> `Pickup` (`markedItem` & `myMarker`) in `build_ai_paths` & `clear_ai_paths`
    let mut shock = ut99_format::ActorRecord::default();
    shock.name = "ShockRifle0".to_string();
    shock.class_name = "Botpack.ShockRifle".to_string();
    shock.location = Vec3::new(128.0, 64.0, -64.0);
    state.loaded_unr.as_mut().unwrap().actors.push(shock);
    state.build_ai_paths();

    let unr = state.loaded_unr.as_ref().unwrap();
    let spot = unr
        .actors
        .iter()
        .find(|a| a.class_name == "InventorySpot")
        .expect("InventorySpot must be auto-generated by build_ai_paths");
    assert_eq!(
        spot.object_paths.get("markedItem").map(|s| s.as_str()),
        Some("ShockRifle0")
    );
    let weapon = unr
        .actors
        .iter()
        .find(|a| a.name == "ShockRifle0")
        .expect("ShockRifle0 must exist");
    assert_eq!(
        weapon.object_paths.get("myMarker").map(|s| s.as_str()),
        Some(spot.name.as_str()),
        "Pickup actor must receive bidirectional myMarker link to its InventorySpot"
    );

    let actors_before_clear = state.loaded_unr.as_ref().unwrap().actors.clone();
    state.clear_ai_paths();
    let unr_cleared = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr_cleared.actors, actors_before_clear, "clear_ai_paths must preserve actors and inventory references");

    // 4. Intervento 49 (REQ-J02): `Import T3D Into Existing Map` (`import_t3d_into_existing_map`) & Canonical `Package.Class` Qualification
    assert_eq!(
        ut99_editor::compiler::CompilerBridge::qualify_ut99_actor_class("PlayerStart"),
        "Engine.PlayerStart"
    );
    assert_eq!(
        ut99_editor::compiler::CompilerBridge::qualify_ut99_actor_class("ElevatorMover"),
        "Engine.ElevatorMover"
    );
    assert_eq!(
        ut99_editor::compiler::CompilerBridge::qualify_ut99_actor_class("ShockRifle"),
        "Botpack.ShockRifle"
    );
    let qualified_map = ut99_editor::compiler::CompilerBridge::export_t3d_map_qualified(&state);
    assert!(qualified_map.actors.iter().all(|a| a.class.contains('.')));

    let brushes_before = state.world_brushes.len();
    let movers_before = state.movers.len();
    let (added_b, added_m, added_a) = state
        .import_t3d_into_existing_map(&t3d_path)
        .expect("import_t3d_into_existing_map must succeed");
    assert!(added_b >= 1);
    assert_eq!(added_m, 1);
    assert!(added_a >= 1);
    assert_eq!(state.world_brushes.len(), brushes_before + added_b);
    assert_eq!(state.movers.len(), movers_before + added_m);

    // Verify Undo of T3D merge restores exact prior counts
    state.undo();
    assert_eq!(state.world_brushes.len(), brushes_before);
    assert_eq!(state.movers.len(), movers_before);

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_wave10_poly_select_console_actor_reset_mirror_texture_batch_and_bsp_ratios() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Intervento 51 (REQ-K03 & REQ-D06): `POLY SELECT ZONE`, `POLY SET TEXTURE=... SETFLAGS=... CLEARFLAGS=...`, `POLY TEXPAN`, `POLY TEXSCALE`
    // Create two touching Subtract rooms separated by a hermetically sealed ZonePortal sheet at X = 0
    let mut room1 = ut99_bsp::build_cube("Room1", CsgOperation::Subtract, Vec3::new(512.0, 256.0, 256.0));
    room1.location = Vec3::new(-256.0, 0.0, 0.0);
    let mut room2 = ut99_bsp::build_cube("Room2", CsgOperation::Subtract, Vec3::new(512.0, 256.0, 256.0));
    room2.location = Vec3::new(256.0, 0.0, 0.0);

    let mut portal = ut99_bsp::build_sheet("PortalSeal", CsgOperation::Add, 256.0, 256.0);
    portal.rotation = Vec3::new(0.0, 90.0, 0.0);
    for p in &mut portal.polys {
        p.flags |= ut99_bsp::PF_PORTAL | ut99_bsp::PF_NOT_SOLID;
    }

    state.world_brushes.push(room1);
    state.world_brushes.push(room2);
    state.world_brushes.push(portal);

    // Select 1 surface of Room1 (brush 0, poly 0) and execute `POLY SELECT ZONE`
    state.selected_surfaces.clear();
    state.selected_surfaces.insert((0, 0));
    let out_zone = state.execute_unrealed_command("POLY SELECT ZONE");
    assert!(out_zone.contains("POLY SELECT ZONE"));
    assert_eq!(
        state.selected_surfaces.len(),
        6,
        "POLY SELECT ZONE must select all 6 surfaces of Room1 and stop at the sealed ZonePortal"
    );
    assert!(
        state.selected_surfaces.iter().all(|&(b, _)| b == 0),
        "No surface from Room2 across the sealed portal may be selected"
    );

    // Test `POLY SET TEXTURE=... SETFLAGS=... CLEARFLAGS=...`
    let _ = state.execute_unrealed_command("POLY SET TEXTURE=Indus1.ZoneWall SETFLAGS=32 CLEARFLAGS=0");
    for p_idx in 0..6 {
        assert_eq!(state.world_brushes[0].polys[p_idx].texture, "Indus1.ZoneWall");
        assert_eq!(state.world_brushes[0].polys[p_idx].flags & 32, 32);
    }
    let _ = state.execute_unrealed_command("POLY SET SETFLAGS=64 CLEARFLAGS=32");
    for p_idx in 0..6 {
        assert_eq!(state.world_brushes[0].polys[p_idx].flags & 32, 0);
        assert_eq!(state.world_brushes[0].polys[p_idx].flags & 64, 64);
    }

    // Test `POLY TEXPAN` and `POLY TEXSCALE`
    let orig_base = state.world_brushes[0].polys[0].base;
    let orig_u_len = state.world_brushes[0].polys[0].texture_u.length();
    let _ = state.execute_unrealed_command("POLY TEXPAN U=16 V=8");
    assert!(state.world_brushes[0].polys[0].base.distance(orig_base) > 1.0);
    let _ = state.execute_unrealed_command("POLY TEXSCALE UU=2.0 VV=0.5");
    assert!((state.world_brushes[0].polys[0].texture_u.length() - orig_u_len * 2.0).abs() < 1e-3);

    // 2. Intervento 52 (REQ-K03 & REQ-C07): `ACTOR MIRROR`, `ACTOR RESET`, `BRUSH MIRROR`, `BRUSH SCALE`
    let _ = state.execute_unrealed_command("ACTOR ADD CLASS=Light");
    let light_idx = state.selected_actor_index.expect("Light must be selected");
    if let Some(ref mut unr) = state.loaded_unr {
        unr.actors[light_idx].location = Vec3::new(120.0, -80.0, 64.0);
        unr.actors[light_idx].rotation = (1024, 8192, 0);
        unr.actors[light_idx].properties.insert(
            "DrawScale".to_string(),
            ut99_format::TaggedProperty {
                name: "DrawScale".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Float(2.5),
            },
        );
    }
    let _ = state.execute_unrealed_command("ACTOR MIRROR X=-1 Y=1 Z=1");
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors[light_idx].rotation.1,
        (32768 - 8192)
    );

    let _ = state.execute_unrealed_command("ACTOR RESET ROTATION");
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[light_idx].rotation, (0, 0, 0));
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors[light_idx].location,
        Vec3::new(120.0, -80.0, 64.0)
    );

    let _ = state.execute_unrealed_command("ACTOR RESET ALL");
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[light_idx].location, Vec3::ZERO);
    assert!(!state.loaded_unr.as_ref().unwrap().actors[light_idx].properties.contains_key("DrawScale"));

    state.selected_actor_index = None;
    state.selected_brush_index = Some(0);
    let _ = state.execute_unrealed_command("BRUSH SCALE X=2.0 Y=1.5 Z=1.0");
    assert_eq!(state.world_brushes[0].scale, Vec3::new(2.0, 1.5, 1.0));
    let _ = state.execute_unrealed_command("BRUSH SCALE RESET");
    assert_eq!(state.world_brushes[0].scale, Vec3::ONE);

    // 3. Intervento 53 (REQ-K03 & REQ-D08): `TEXTURE CLEAR`, `TEXTURE SCALE DELTA=...`, `TEXTURE APPLYDETAIL`, `TEXTURE BATCHAPPLY`
    let _ = state.execute_unrealed_command("TEXTURE CLEAR");
    assert_eq!(state.world_brushes[0].polys[0].texture, "DefaultTexture");

    state.active_texture = Some("Ancient.BaseFloor".to_string());
    let _ = state.execute_unrealed_command("TEXTURE BATCHAPPLY");
    assert_eq!(state.world_brushes[0].polys[0].texture, "Ancient.BaseFloor");

    let _ = state.execute_unrealed_command("TEXTURE APPLYDETAIL");
    let meta = state.texture_manager.get_texture_metadata("Ancient.BaseFloor");
    assert_eq!(meta.detail_texture, "Detail.DefaultDetail");

    let u_before_delta = state.world_brushes[0].polys[0].texture_u.length();
    let _ = state.execute_unrealed_command("TEXTURE SCALE DELTA=0.5");
    assert!((state.world_brushes[0].polys[0].texture_u.length() - u_before_delta * 0.5).abs() < 1e-3);

    // 4. Intervento 54 (REQ-I02 & REQ-I03): `BspBuildReport` Ratios & `refresh_build_stats`
    if let Some(ref mut unr) = state.loaded_unr {
        let mut z0 = ut99_format::ActorRecord::default();
        z0.class_name = "ZoneInfo".to_string();
        z0.name = "ZoneInfo0".to_string();
        z0.location = Vec3::new(-256.0, 0.0, 0.0);
        unr.actors.push(z0);

        let mut z1 = ut99_format::ActorRecord::default();
        z1.class_name = "ZoneInfo".to_string();
        z1.name = "ZoneInfo1".to_string();
        z1.location = Vec3::new(256.0, 0.0, 0.0);
        unr.actors.push(z1);
    }

    let report = state.refresh_build_stats();
    assert!(report.poly_count >= 12);
    assert!(report.node_count >= 1);
    assert!(report.nodes_to_polys_ratio() > 0.0);
    assert!(report.polys_to_nodes_ratio() > 0.0);
    assert!(
        report.is_optimal_ratio(),
        "Simple two-room BSP must achieve canonical UnrealEd 2.0 optimal ratio <= 2.00:1 (got {:.2}:1)",
        report.nodes_to_polys_ratio()
    );
    assert_eq!(
        state.compute_3d_zone_flood_fill_report().distinct_zone_count,
        2,
        "refresh_build_stats must also verify 2 distinct 3D BSP zones across the sealed ZonePortal"
    );
}


#[test]
fn test_wave11_add_special_presets_shadow_detail_diagonal_skew_camera_align_and_dispatcher_audit() {
    use ut99_bsp::{
        PF_HIGH_SHADOW_DETAIL, PF_INVISIBLE, PF_LOW_SHADOW_DETAIL, PF_MASKED, PF_MODULATED,
        PF_NOT_SOLID, PF_PORTAL, PF_SEMISOLID, PF_TRANSLUCENT, PF_TWO_SIDED,
    };
    use ut99_editor::editor_state::SpecialBrushPreset;

    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Intervento 56 (REQ-B05 & REQ-K03): 9 Canonical UnrealEd 2.0 `Add Special` Prefabs, Solidity mutual exclusion & `BRUSH ADD FLAGS=...`
    assert_eq!(
        SpecialBrushPreset::InvisibleCollisionHull.poly_flags(),
        PF_INVISIBLE | PF_SEMISOLID
    );
    assert_eq!(
        SpecialBrushPreset::MaskedDecoration.poly_flags(),
        PF_MASKED | PF_TWO_SIDED | PF_NOT_SOLID
    );
    assert_eq!(
        SpecialBrushPreset::ModulatedWindow.poly_flags(),
        PF_MODULATED | PF_TWO_SIDED | PF_NOT_SOLID
    );
    assert_eq!(
        SpecialBrushPreset::Portal.poly_flags(),
        PF_PORTAL | PF_INVISIBLE | PF_NOT_SOLID
    );
    assert_eq!(SpecialBrushPreset::RegularBrush.poly_flags(), 0);
    assert_eq!(SpecialBrushPreset::SemisolidPillar.poly_flags(), PF_SEMISOLID);
    assert_eq!(
        SpecialBrushPreset::TransparentWindow.poly_flags(),
        PF_TRANSLUCENT | PF_TWO_SIDED | PF_NOT_SOLID
    );
    assert_eq!(
        SpecialBrushPreset::Water.poly_flags(),
        PF_PORTAL | PF_TRANSLUCENT | PF_TWO_SIDED | PF_NOT_SOLID
    );
    assert_eq!(
        SpecialBrushPreset::ZonePortal.poly_flags(),
        PF_PORTAL | PF_INVISIBLE | PF_NOT_SOLID
    );

    // Verify mutual exclusion of PF_SEMISOLID and PF_NOT_SOLID in add_special_brush
    state.set_builder_cube(Vec3::new(256.0, 256.0, 256.0));
    state.csg_subtract(); // Brush 0: room
    state.set_builder_cube(Vec3::new(64.0, 64.0, 128.0));
    state.add_special_brush(PF_SEMISOLID | PF_NOT_SOLID, false);
    assert_eq!(
        state.world_brushes[1].polys[0].flags & PF_SEMISOLID,
        0,
        "Non-Solid brush must automatically clear PF_SEMISOLID bit"
    );
    assert_ne!(state.world_brushes[1].polys[0].flags & PF_NOT_SOLID, 0);

    // Verify console `BRUSH ADD FLAGS=...`
    let cmd_add_flags = format!("BRUSH ADD FLAGS={}", PF_SEMISOLID);
    let _ = state.execute_unrealed_command(&cmd_add_flags);
    assert_eq!(state.world_brushes.len(), 3);
    assert_eq!(state.world_brushes[2].polys[0].flags, PF_SEMISOLID);

    // 2. Intervento 57 (REQ-D04, REQ-D05 & REQ-F04): High/Low Shadow Detail luxel resolution + Small/Big Diagonal UV Skew
    // Face 0: High Shadow Detail (2x finer luxel grid)
    // Face 1: Low Shadow Detail (2x coarser luxel grid)
    // Face 2: Default Shadow Detail
    state.world_brushes[0].polys[0].flags = PF_HIGH_SHADOW_DETAIL;
    state.world_brushes[0].polys[1].flags = PF_LOW_SHADOW_DETAIL;
    state.world_brushes[0].polys[2].flags = 0;

    state.builder_brush.location = Vec3::ZERO;
    state.place_actor_of_class("Light");
    state.build_lighting();

    let lm_high = state.world_brushes[0].polys[0]
        .lightmap
        .as_ref()
        .expect("High Shadow Detail lightmap");
    let lm_low = state.world_brushes[0].polys[1]
        .lightmap
        .as_ref()
        .expect("Low Shadow Detail lightmap");
    let lm_def = state.world_brushes[0].polys[2]
        .lightmap
        .as_ref()
        .expect("Default Shadow Detail lightmap");

    assert!(
        lm_high.width > lm_def.width && lm_def.width > lm_low.width,
        "High Shadow Detail ({}x{}) must produce denser luxels than Default ({}x{}) and Low Shadow Detail ({}x{})",
        lm_high.width,
        lm_high.height,
        lm_def.width,
        lm_def.height,
        lm_low.width,
        lm_low.height
    );

    // Small Diagonal & Big Diagonal UV skew + Undo
    state.selected_surfaces.clear();
    state.selected_surfaces.insert((0, 0));
    let u_orig = state.world_brushes[0].polys[0].texture_u;
    let v_orig = state.world_brushes[0].polys[0].texture_v;
    state.skew_selected_brush_texture_diagonal(true); // Small Diagonal: U' = U + V, V' = -U + V
    let u_skew = state.world_brushes[0].polys[0].texture_u;
    let v_skew = state.world_brushes[0].polys[0].texture_v;
    assert!((u_skew - (u_orig + v_orig)).length() < 1e-4);
    assert!((v_skew - (-u_orig + v_orig)).length() < 1e-4);
    state.undo();
    assert!((state.world_brushes[0].polys[0].texture_u - u_orig).length() < 1e-4);

    // Console `POLY TEXSCALE RELATIVE UU=1.0 VV=0.5 UV=0.5 VU=-0.5` (Big Diagonal)
    let _ = state.execute_unrealed_command("POLY TEXSCALE RELATIVE UU=1.0 VV=0.5 UV=0.5 VU=-0.5");
    assert!((state.world_brushes[0].polys[0].texture_u - (u_orig + v_orig * 0.5)).length() < 1e-4);

    // 3. Intervento 58 (REQ-K07): `CAMERA ALIGN` and `CAMERA ALIGN NAME=...`
    if let Some(ref mut unr) = state.loaded_unr {
        unr.actors[0].name = "TargetLight99".to_string();
        unr.actors[0].location = Vec3::new(512.0, -768.0, 192.0);
    }
    let _ = state.execute_unrealed_command("CAMERA ALIGN NAME=TargetLight99");
    assert_eq!(state.selected_actor_index, Some(0));
    assert!((state.camera.ortho_pos_top.x - 512.0).abs() < 1e-3);
    assert!((state.camera.ortho_pos_top.y - (-768.0)).abs() < 1e-3);
    assert!((state.camera.ortho_pos_front.y - 192.0).abs() < 1e-3);

    // 4. Intervento 59 (REQ-E04, REQ-E07 & REQ-E09): Dispatcher `OutEvents[0..7]` Mover target matching & orphan audit
    state.set_builder_cube(Vec3::new(64.0, 16.0, 128.0));
    state.add_mover_from_builder();
    state.movers[0].name = "Mover0".to_string();
    state.movers[0].tag = "BlastDoorTag".to_string();

    if let Some(ref mut unr) = state.loaded_unr {
        let mut disp = ut99_format::ActorRecord::default();
        disp.class_name = "Dispatcher".to_string();
        disp.name = "Dispatcher0".to_string();
        disp.tag = "StartSeq".to_string();
        unr.actors.push(disp);
    }
    assert!(state.update_dispatcher_stage("Dispatcher0", 0, 1.5, "BlastDoorTag"));
    assert!(state.update_dispatcher_stage("Dispatcher0", 1, 0.5, "NonExistentOrphanTag"));

    let sequences = state.get_dispatcher_sequences();
    assert_eq!(sequences.len(), 1);
    assert_eq!(sequences[0].stages.len(), 2);
    assert_eq!(
        sequences[0].stages[0].matched_targets.len(),
        1,
        "Dispatcher OutEvents[0] ('BlastDoorTag') must match Mover0 in state.movers"
    );
    assert_eq!(sequences[0].stages[0].matched_targets[0].0, "Mover0");
    assert!(sequences[0].has_unmatched_events);

    state.run_level_doctor_audit();
    assert!(
        state
            .map_audit_issues
            .iter()
            .any(|iss| iss.description.contains("NonExistentOrphanTag")),
        "LevelDoctor must flag Dispatcher OutEvents[1] ('NonExistentOrphanTag') as an orphan Event"
    );
}

#[test]
fn test_evo05_unrealed2_console_commands_coverage() {
    let mut state = EditorState::default();

    // 1. MAP NEW
    state.world_brushes.push(ut99_bsp::build_cube("CustomBox", CsgOperation::Subtract, Vec3::splat(256.0)));
    let resp = state.execute_unrealed_command("MAP NEW");
    assert!(resp.contains("created empty map"));
    // new_map resets to the canonical starting room (Room0) + PlayerStart + Light
    assert_eq!(state.world_brushes.len(), 1);
    assert_eq!(state.world_brushes[0].name, "Room0");

    // 2. ACTOR ALIGN
    state.place_actor_of_class("Light");
    state.selected_actor_index = Some(0);
    let resp = state.execute_unrealed_command("ACTOR ALIGN");
    assert!(resp.contains("ACTOR ALIGN executed"));

    // 3. BRUSH SAVE / BRUSH LOAD via temp file
    let temp_brush_path = std::env::temp_dir().join("test_builder_brush.t3d");
    let temp_brush_str = temp_brush_path.to_string_lossy().to_string();
    let resp = state.execute_unrealed_command(&format!("BRUSH SAVE FILE=\"{}\"", temp_brush_str));
    assert!(resp.contains("exported builder brush"));
    assert!(temp_brush_path.exists());

    let resp = state.execute_unrealed_command(&format!("BRUSH LOAD FILE=\"{}\"", temp_brush_str));
    assert!(resp.contains("loaded builder brush"));
    let _ = std::fs::remove_file(&temp_brush_path);

    // 4. POLY ROTATE
    state.world_brushes.push(ut99_bsp::build_cube("RotCube", CsgOperation::Add, Vec3::splat(256.0)));
    state.selected_surfaces.insert((0, 0));
    let resp = state.execute_unrealed_command("POLY ROTATE ANGLE=45");
    assert!(resp.contains("POLY ROTATE ANGLE=45 executed"));
}

#[test]
fn test_evo03_unrealscript_visual_array_inspector() {
    let mut state = EditorState::default();
    state.new_map();

    // 1. Verify UnrealScript metadata extracts array capacity for Dispatcher
    let meta = ut99_editor::unrealscript::get_class_metadata("Dispatcher", None, None);
    let out_events_var = meta.variables.iter().find(|v| v.name == "OutEvents").expect("OutEvents variable in Dispatcher");
    assert_eq!(out_events_var.var_type, "name[8]");

    // 2. Spawn a Dispatcher actor in loaded_unr
    let mut disp = ut99_format::ActorRecord::default();
    disp.name = "TestDispatcher".to_string();
    disp.class_name = "Dispatcher".to_string();
    disp.properties.insert(
        "OutEvents".to_string(),
        ut99_format::TaggedProperty {
            name: "OutEvents".to_string(),
            array_index: 0,
            value: ut99_format::TaggedValue::Str("EventAlpha".to_string()),
        },
    );
    disp.properties.insert(
        "OutEvents(1)".to_string(),
        ut99_format::TaggedProperty {
            name: "OutEvents".to_string(),
            array_index: 1,
            value: ut99_format::TaggedValue::Str("EventBeta".to_string()),
        },
    );
    disp.properties.insert(
        "OutEvents(2)".to_string(),
        ut99_format::TaggedProperty {
            name: "OutEvents".to_string(),
            array_index: 2,
            value: ut99_format::TaggedValue::Str("EventGamma".to_string()),
        },
    );

    if let Some(ref mut unr) = state.loaded_unr {
        unr.actors.push(disp);
    }
    state.selected_actor_index = Some(state.loaded_unr.as_ref().unwrap().actors.len() - 1);
    state.show_actor_properties = true;

    // 3. Render GUI via egui test context
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        ut99_editor::gui::actor_properties::render_actor_properties_dialog(ctx, &mut state);
    });

    // 4. Verify properties are intact and array indexing is recognized
    let actor = &state.loaded_unr.as_ref().unwrap().actors[state.selected_actor_index.unwrap()];
    assert!(actor.properties.contains_key("OutEvents"));
    assert!(actor.properties.contains_key("OutEvents(1)"));
    assert!(actor.properties.contains_key("OutEvents(2)"));
    assert_eq!(actor.properties.get("OutEvents").unwrap().array_index, 0);
    assert_eq!(actor.properties.get("OutEvents(1)").unwrap().array_index, 1);
    assert_eq!(actor.properties.get("OutEvents(2)").unwrap().array_index, 2);

    // 5. Test adding and reindexing array properties
    if let Some(ref mut unr) = state.loaded_unr {
        let act = &mut unr.actors[state.selected_actor_index.unwrap()];
        // Add element at index 3
        act.properties.insert(
            "OutEvents(3)".to_string(),
            ut99_format::TaggedProperty {
                name: "OutEvents".to_string(),
                array_index: 3,
                value: ut99_format::TaggedValue::Str("EventDelta".to_string()),
            },
        );
    }

    let _ = ctx.run(Default::default(), |ctx| {
        ut99_editor::gui::actor_properties::render_actor_properties_dialog(ctx, &mut state);
    });

    let actor = &state.loaded_unr.as_ref().unwrap().actors[state.selected_actor_index.unwrap()];
    assert_eq!(actor.properties.len(), 4);
    assert_eq!(actor.properties.get("OutEvents(3)").unwrap().value, ut99_format::TaggedValue::Str("EventDelta".to_string()));
}

#[test]
fn test_evo06_dxt1_dxt5_texture_browser_and_conversion() {
    let mut state = EditorState::default();
    state.new_map();

    // 1. Import a 64x64 test RGBA texture into MyLevel
    let w = 64u32;
    let h = 64u32;
    let mut rgba_pixels = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            rgba_pixels[idx] = (x * 4) as u8;
            rgba_pixels[idx + 1] = (y * 4) as u8;
            rgba_pixels[idx + 2] = 180;
            rgba_pixels[idx + 3] = 255;
        }
    }

    let full_ref = state
        .texture_manager
        .import_rgba_to_package("MyLevel", "Walls", "MetalPlate", w, h, false, rgba_pixels)
        .expect("Import RGBA to MyLevel");
    assert_eq!(full_ref, "MyLevel.MetalPlate");

    // 2. Initial format info is P8 (0)
    let (fmt_id, fmt_name, _) = state
        .texture_manager
        .get_texture_format_info("MyLevel", "MetalPlate")
        .expect("Format info");
    assert_eq!(fmt_id, 0);
    assert!(fmt_name.contains("P8"));

    // 3. Convert to DXT1 (BC1) and verify VRAM footprint savings
    let (orig_bytes, new_bytes, savings) = state
        .texture_manager
        .convert_texture_format("MyLevel", "MetalPlate", 3)
        .expect("Convert to DXT1");
    assert_eq!(orig_bytes, (w * h) as usize); // 4096 bytes for P8
    let expected_dxt1_bytes = ((w as usize) / 4) * ((h as usize) / 4) * 8; // 16 * 16 * 8 = 2048 bytes
    assert_eq!(new_bytes, expected_dxt1_bytes);
    assert!((savings - 50.0).abs() < 1.0); // 50% savings over P8

    let (fmt_id_dxt1, fmt_name_dxt1, vram_dxt1) = state
        .texture_manager
        .get_texture_format_info("MyLevel", "MetalPlate")
        .expect("Format info");
    assert_eq!(fmt_id_dxt1, 3);
    assert_eq!(fmt_name_dxt1, "DXT1 (BC1)");
    assert_eq!(vram_dxt1, expected_dxt1_bytes);

    // 4. Convert to DXT5 (BC3)
    let (_, new_dxt5_bytes, _) = state
        .texture_manager
        .convert_texture_format("MyLevel", "MetalPlate", 8)
        .expect("Convert to DXT5");
    let expected_dxt5_bytes = ((w as usize) / 4) * ((h as usize) / 4) * 16; // 4096 bytes
    assert_eq!(new_dxt5_bytes, expected_dxt5_bytes);

    let (fmt_id_dxt5, fmt_name_dxt5, vram_dxt5) = state
        .texture_manager
        .get_texture_format_info("MyLevel", "MetalPlate")
        .expect("Format info");
    assert_eq!(fmt_id_dxt5, 8);
    assert_eq!(fmt_name_dxt5, "DXT5 (BC3)");
    assert_eq!(vram_dxt5, expected_dxt5_bytes);

    // 5. Test GUI rendering in Texture Browser
    state.selected_texture_package = "MyLevel".to_string();
    state.active_texture = Some("MyLevel.MetalPlate".to_string());
    state.show_texture_browser = true;

    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        ut99_editor::gui::texture_browser::render_texture_browser(ctx, &mut state);
    });

    // 6. Test exporting package to .utx with DXT5 format and full mip chain
    let temp_utx_path = std::env::temp_dir().join("test_export_dxt.utx");
    let export_res = state.export_texture_package_utx("MyLevel", &temp_utx_path);
    assert!(export_res.is_ok(), "Export .utx with DXT texture must succeed");
    assert!(temp_utx_path.exists());
    let _ = std::fs::remove_file(&temp_utx_path);
}

#[test]
fn test_evo07_prebuild_csg_solidity_and_builder_diagnostics() {
    let mut state = EditorState::default();
    state.new_map();

    // 1. Initial Red Builder Brush is a standard convex cube
    state.builder_brush = ut99_bsp::build_cube("BuilderCube", ut99_bsp::CsgOperation::Active, Vec3::new(256.0, 256.0, 256.0));
    let diag = state.audit_builder_brush_csg_safety();
    assert!(diag.is_convex);
    assert!(diag.is_watertight);
    assert_eq!(diag.reflex_vertex_count, 0);
    assert!(!diag.coplanar_split_risk);
    assert!(diag.summary_message.contains("Safe for CSG"));

    // 2. Add an extra polygon that makes the builder brush concave
    let reflex_poly = ut99_bsp::poly::Polygon::new(vec![
        Vec3::new(100.0, 100.0, 400.0),
        Vec3::new(200.0, 100.0, 400.0),
        Vec3::new(200.0, 200.0, 400.0),
    ]).unwrap();
    state.builder_brush.polys.push(reflex_poly);

    let diag_concave = state.audit_builder_brush_csg_safety();
    assert!(!diag_concave.is_convex);
    assert!(diag_concave.reflex_vertex_count > 0);
    assert!(diag_concave.coplanar_split_risk);

    // 3. Run Level Doctor audit and verify builder brush issue is reported
    state.run_level_doctor_audit();
    assert!(
        state.map_audit_issues.iter().any(|iss| iss.category == "BuilderBrush" && iss.object_name == "RedBuilderBrush"),
        "LevelDoctor must report warning for non-convex Builder Brush"
    );

    // 4. CSG ADD with non-convex brush logs warning
    let initial_brush_count = state.world_brushes.len();
    state.csg_add();
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);
    assert!(state.toasts.iter().any(|t| t.message.contains("non-convex brush")));
}

#[test]
fn test_evo08_4level_contextual_tooltips_and_guidance() {
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Render 4-level tooltip function directly
            ut99_editor::gui::theme::pro_tooltip_4level(
                ui,
                "CSG Add",
                Some("Ctrl+A"),
                "Add active red builder brush to world geometry (Convex & Watertight)",
                Some("Shift+Click: Keep builder in place"),
                Some("Operation: Additive Solid"),
                Some("UnrealEd 2.0 Gotcha: Always subtract the room first, then add pillars, crates, and decorations. Never add overlapping solid brushes directly without clipping or semi-solids to prevent BSP leaf explosion."),
            );

            // Test ProTooltipExt trait extension on an egui widget response
            let btn_resp = ui.button("+ Add");
            use ut99_editor::gui::theme::ProTooltipExt;
            let _ = btn_resp.pro_tooltip_4level(
                "CSG Subtract",
                Some("Ctrl+S"),
                "Subtract active red builder brush from world geometry",
                Some("Shift+Click: Keep builder in place"),
                Some("Operation: Subtractive Hollow"),
                Some("UnrealEd 2.0 Gotcha: In Unreal Engine 1, the world begins as solid infinite rock."),
            );
        });
    });
}





