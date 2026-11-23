use std::fs;
use ut99_editor::{CompilerBridge, EditorState};
use ut99_format::UnrLevel;
mod common;

#[test]
fn test_new_map_initialization() {
    let mut state = EditorState::default();
    state.new_map();

    assert_eq!(state.world_brushes.len(), 1);
    assert_eq!(state.world_brushes[0].csg_oper, ut99_bsp::CsgOperation::Subtract);
    assert!(state.loaded_unr.is_some());

    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.player_starts.len(), 1);
    assert_eq!(unr.lights.len(), 1);
}

#[test]
fn test_t3d_export_and_safe_unr_compilation() {
    let mut state = EditorState::default();
    state.new_map();

    let t3d = CompilerBridge::export_t3d_map(&state);
    assert!(t3d.actors.iter().any(|a| a.class == "LevelInfo"));
    assert!(t3d.actors.iter().any(|a| a.class == "Brush"));
    assert!(t3d.actors.iter().any(|a| a.class == "PlayerStart"));
    assert!(t3d.actors.iter().any(|a| a.class == "Light"));

    let Some(ut_dir) = std::env::var_os("FORGE99_TEST_UT_DIR").map(std::path::PathBuf::from) else {
        eprintln!("Skipping live UCC test: set FORGE99_TEST_UT_DIR to an isolated installation");
        return;
    };
    let repo_ut = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../UnrealTournament");
    assert_ne!(ut_dir.canonicalize().unwrap(), repo_ut.canonicalize().unwrap(), "engine test must use an isolated installation");
    // `UT99_DIR` must also select this installation for the save API.
    assert_eq!(CompilerBridge::find_ut_dir().unwrap().canonicalize().unwrap(), ut_dir.canonicalize().unwrap());
    let output = common::TestDir::new();
    let test_map_path = output.path().join("TestAutomatedNewMap.unr");
    let test_bak_path = output.path().join("TestAutomatedNewMap.unr.bak");

    // 1. First save: creates TestAutomatedNewMap.unr
    let res1 = CompilerBridge::save_unr_with_report(&state, &test_map_path)
        .expect("First engine save failed");
    assert_eq!(res1.backend, ut99_editor::compiler::UnrSaveBackend::Engine,
        "Isolated engine test must not silently pass via native fallback: {:?}", res1.native_reason);
    assert!(test_map_path.exists(), "test_map_path does not exist");
    assert!(!test_bak_path.exists(), "test_bak_path should not exist on first save");

    // 2. Second save: should trigger automatic .bak creation!
    let res2 = state.save_unr(&test_map_path);
    assert!(res2.is_ok(), "Second save_unr failed: {:?}", res2.err());
    assert!(test_map_path.exists(), "test_map_path does not exist on second save");
    assert!(test_bak_path.exists(), "test_bak_path was NOT created on overwrite!");

    // 3. Verify that the compiled .unr can be read back by UnrLevel::open!
    let reloaded = UnrLevel::open(&test_map_path);
    assert!(reloaded.is_ok(), "Failed to open newly saved .unr level: {:?}", reloaded.err());
    let level = reloaded.unwrap();
    assert!(!level.actors.is_empty(), "Level has no actors");
    println!("Compiled map verified: {} actors, {} brushes", level.actors.len(), level.brushes.len());

    // Clean up
    let _ = fs::remove_file(&test_map_path);
    let _ = fs::remove_file(&test_bak_path);
}

#[test]
fn test_wizard_verified_export_with_isolated_engine() {
    let Some(ut_dir) = std::env::var_os("FORGE99_TEST_UT_DIR").map(std::path::PathBuf::from) else {
        eprintln!("Skipping engine wizard test: set FORGE99_TEST_UT_DIR to an isolated installation");
        return;
    };
    let repo_ut = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../UnrealTournament");
    assert_ne!(ut_dir.canonicalize().unwrap(), repo_ut.canonicalize().unwrap());
    assert_eq!(CompilerBridge::find_ut_dir().unwrap().canonicalize().unwrap(), ut_dir.canonicalize().unwrap());
    let output = common::TestDir::new();
    let path = output.path().join("CTF-VerifiedEngine.unr");
    let mut state = EditorState::default();
    state.map_wizard_config.game_mode = ut99_bsp::WizardGameMode::CaptureTheFlag;
    state.map_wizard_config.layout_strategy = ut99_bsp::WizardLayoutStrategy::ProceduralSeed;
    state.map_wizard_config.seed = 42;
    state.map_wizard_config.map_name = "CTF-VerifiedEngine".into();
    state.map_wizard_config.sync_mode_conventions();
    state.generate_wizard_map_verified().expect("wizard audit must succeed");
    let report = state.save_wizard_unr_verified(&path).expect("verified engine save failed");
    assert_eq!(report.backend, ut99_editor::compiler::UnrSaveBackend::Engine, "fallback: {:?}", report.native_reason);
    assert!(report.engine_warning.is_none(), "engine cleanup warning: {:?}", report.engine_warning);
    let level = UnrLevel::open(&report.path).expect("engine output must reopen");
    assert!(UnrLevel::validate_package(&level.package).is_valid);
    assert!(level.world_bsp.is_some(), "engine output must contain world BSP");
}

#[test]
fn test_moved_procedural_room_connection_on_isolated_engine_bsp() {
    let Some(ut_dir) = std::env::var_os("FORGE99_TEST_UT_DIR").map(std::path::PathBuf::from) else {
        eprintln!("Skipping engine test: set FORGE99_TEST_UT_DIR to an isolated installation");
        return;
    };
    let repo_ut = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../UnrealTournament");
    assert_ne!(ut_dir.canonicalize().unwrap(), repo_ut.canonicalize().unwrap());
    assert_eq!(CompilerBridge::find_ut_dir().unwrap().canonicalize().unwrap(), ut_dir.canonicalize().unwrap());
    let mut state = EditorState::default();
    state.map_wizard_config.layout_strategy = ut99_bsp::WizardLayoutStrategy::ProceduralSeed;
    state.map_wizard_config.game_mode = ut99_bsp::WizardGameMode::DeathMatch;
    state.map_wizard_config.seed = 42;
    state.map_wizard_config.procedural_room_count = 6;
    state.map_wizard_config.procedural_vertical_tiers = 1;
    state.map_wizard_config.map_name = "DM-MovedRoomEngine".into();
    state.map_wizard_config.auto_run_build_and_audit = false;
    let initial = ut99_bsp::preview_wizard_topology(&state.map_wizard_config);
    let room = initial.rooms.iter().find(|room| room.label == "ProcChamber_1").unwrap();
    state.map_wizard_config.room_overrides.push(ut99_bsp::WizardRoomOverride {
        room_name: room.label.clone(), center_offset: room.center + glam::Vec3::new(8192.0, 4096.0, 0.0),
        size: room.size, environment: ut99_bsp::WizardRoomEnvironment::Normal,
    });
    let preview = ut99_bsp::preview_wizard_topology(&state.map_wizard_config);
    let room = preview.rooms.iter().find(|room| room.label == "ProcChamber_1").unwrap();
    let graph = preview.procedural_graph.as_ref().unwrap();
    let index = graph.edges.iter().position(|(a, b)| a == &room.label || b == &room.label).unwrap();
    let segment = preview.rooms.iter().find(|segment| {
        (segment.label == format!("ProcCorridorX_{index}") || segment.label == format!("ProcCorridorY_{index}"))
            && (segment.center.x - room.center.x).abs() < (segment.size.x + room.size.x) * 0.5
            && (segment.center.y - room.center.y).abs() < (segment.size.y + room.size.y) * 0.5
    }).expect("a corridor segment must overlap the moved room");
    let min = (segment.center - segment.size * 0.5).max(room.center - room.size * 0.5);
    let max = (segment.center + segment.size * 0.5).min(room.center + room.size * 0.5);
    let sample = glam::Vec3::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5, min.z + 64.0);
    state.generate_map_from_wizard();
    let output = common::TestDir::new();
    let path = output.path().join("DM-MovedRoomEngine.unr");
    let report = CompilerBridge::save_unr_with_report(&state, &path).expect("isolated engine export failed");
    assert_eq!(report.backend, ut99_editor::compiler::UnrSaveBackend::Engine, "fallback: {:?}", report.native_reason);
    assert!(report.engine_warning.is_none(), "engine cleanup warning: {:?}", report.engine_warning);
    let level = UnrLevel::open(path).expect("engine output must reopen");
    let bsp = level.world_bsp.as_ref().expect("engine output must contain world BSP");
    assert!(!bsp.point_is_solid(sample), "sampled room-corridor overlap is solid at {sample:?}");
    assert!(level.floor_distance(sample, 120.0, 1.0).is_some(), "sampled overlap has no nearby floor at {sample:?}");
}
