use glam::Vec3;
use std::fs;
use std::path::PathBuf;
use ut99_bsp::{build_cube, CsgOperation};
use ut99_editor::editor_state::{EditorState, SavePhase};

#[test]
fn test_manifest_metadata_editing_and_level_info_sync() {
    let mut state = EditorState::default();
    state.new_map();
    state.map_path = Some(PathBuf::from("UnrealTournament/Maps/DM-OldName.unr"));
    state.level_title = "Old Arena Title".to_string();
    state.level_author = "Original Author".to_string();
    state.level_ideal_player_count = "2-4 Players".to_string();
    state.level_song = "Cannon.umx".to_string();
    state.level_game_type = "Botpack.DeathMatchPlus".to_string();
    state.level_enter_text = "Welcome to the old arena".to_string();

    // Verify initial manifest
    let initial_manifest = state.generate_map_manifest();
    assert_eq!(initial_manifest.map_name, "DM-OldName");
    assert_eq!(initial_manifest.title, "Old Arena Title");
    assert_eq!(initial_manifest.author, "Original Author");

    // Apply edited manifest metadata (renaming map and modifying author/title/music/rules)
    state.apply_manifest_metadata(
        "DM-HyperOrbit",
        "Hyper Orbit Station",
        "Master Architect",
        "4-12 Players",
        "Foregone.umx",
        Some("Botpack.CTFGame"),
        Some("Zero gravity space station combat"),
    );

    // 1. Verify EditorState fields updated
    assert_eq!(state.level_title, "Hyper Orbit Station");
    assert_eq!(state.level_author, "Master Architect");
    assert_eq!(state.level_ideal_player_count, "4-12 Players");
    assert_eq!(state.level_song, "Foregone.umx");
    assert_eq!(state.level_game_type, "Botpack.CTFGame");
    assert_eq!(state.level_enter_text, "Zero gravity space station combat");
    assert_eq!(
        state.map_path.as_ref().and_then(|p| p.file_stem()).unwrap(),
        "DM-HyperOrbit"
    );
    assert_eq!(
        state.map_path.as_ref().and_then(|p| p.file_name()).unwrap(),
        "DM-HyperOrbit.unr"
    );
    assert!(state.map_is_dirty, "Map must be marked dirty after metadata changes");

    // 2. Verify LevelInfo actor synchronization in loaded_unr
    if let Some(ref unr) = state.loaded_unr {
        if let Some(info) = unr.actors.iter().find(|a| {
            a.class_name.eq_ignore_ascii_case("LevelInfo")
                || a.class_name.eq_ignore_ascii_case("Engine.LevelInfo")
        }) {
            if let Some(prop) = info.properties.get("Title") {
                if let ut99_format::TaggedValue::Str(val) = &prop.value {
                    assert_eq!(val, "Hyper Orbit Station");
                }
            }
            if let Some(prop) = info.properties.get("Author") {
                if let ut99_format::TaggedValue::Str(val) = &prop.value {
                    assert_eq!(val, "Master Architect");
                }
            }
            if let Some(prop) = info.properties.get("IdealPlayerCount") {
                if let ut99_format::TaggedValue::Str(val) = &prop.value {
                    assert_eq!(val, "4-12 Players");
                }
            }
            if let Some(prop) = info.properties.get("Song") {
                if let ut99_format::TaggedValue::Str(val) = &prop.value {
                    assert_eq!(val, "Foregone.umx");
                }
            }
            if let Some(prop) = info.properties.get("DefaultGameType") {
                if let ut99_format::TaggedValue::Str(val) = &prop.value {
                    assert_eq!(val, "Botpack.CTFGame");
                }
            }
        }
    }

    // 3. Verify Readme dynamic generation from edited metadata
    let manifest = state.generate_map_manifest();
    assert_eq!(manifest.map_name, "DM-HyperOrbit");
    assert_eq!(manifest.title, "Hyper Orbit Station");
    assert_eq!(manifest.author, "Master Architect");
    assert_eq!(manifest.ideal_player_count, "4-12 Players");
    assert_eq!(manifest.song, "Foregone.umx");

    let readme = manifest.generate_readme();
    assert!(readme.contains("Map Title: Hyper Orbit Station"));
    assert!(readme.contains("Filename:  DM-HyperOrbit.unr"));
    assert!(readme.contains("Author:    Master Architect"));
    assert!(readme.contains("Players:   4-12 Players"));
    assert!(readme.contains("Music:     Foregone.umx"));
}

#[test]
fn test_save_feedback_lifecycle_and_progress_state_machine() {
    let mut state = EditorState::default();
    state.new_map();
    let temp_unr = std::env::temp_dir().join("test_save_feedback_map.unr");

    // Phase 0: Trigger save
    state.trigger_save_unr_native(&temp_unr);

    assert!(state.save_feedback.is_some());
    assert!(state.pending_save_op.is_some());

    let fb = state.save_feedback.as_ref().unwrap();
    assert_eq!(fb.phase, SavePhase::PendingPreparation);
    assert_eq!(fb.filename, "test_save_feedback_map.unr");
    assert!(fb.completed_at.is_none());
    assert!(fb.file_size_bytes.is_none());

    // Phase 1: Simulate Frame 0 step (advancing to Executing)
    let ctx = egui::Context::default();
    state.process_pending_save_operation(&ctx);

    let fb_exec = state.save_feedback.as_ref().unwrap();
    assert_eq!(fb_exec.phase, SavePhase::Executing);

    // Phase 2: Simulate Frame 1 step (running actual save execution)
    state.process_pending_save_operation(&ctx);

    let fb_done = state.save_feedback.as_ref().unwrap();
    assert_eq!(fb_done.phase, SavePhase::Completed);
    assert!(fb_done.completed_at.is_some());
    assert!(fb_done.file_size_bytes.is_some());
    assert!(fb_done.file_size_bytes.unwrap() > 0);
    assert!(temp_unr.exists());

    // Clean up
    let _ = fs::remove_file(&temp_unr);
    let bak = temp_unr.with_extension("unr.bak");
    if bak.exists() {
        let _ = fs::remove_file(&bak);
    }
}

#[test]
fn test_metadata_persistence_in_native_unr_save() {
    let mut state = EditorState::default();
    state.new_map();

    // Add a cube room
    let mut brush = build_cube("SubRoom", CsgOperation::Subtract, Vec3::splat(512.0));
    for p in &mut brush.polys {
        p.texture = "Indus1.Floor.Panel01".to_string();
    }
    state.world_brushes.push(brush);

    // Set custom metadata
    let save_path = std::env::temp_dir().join("DM-TestMetadataPersist.unr");
    state.apply_manifest_metadata(
        "DM-TestMetadataPersist",
        "Persistent Citadel",
        "Test Engineer",
        "6-10 Players",
        "Cannon.umx",
        Some("Botpack.DeathMatchPlus"),
        Some("Heavy metal combat"),
    );

    // Save map
    let saved = state.save_unr_native(&save_path).expect("Save native unr must succeed");
    assert!(saved.exists());
    assert!(fs::metadata(&saved).unwrap().len() > 0);

    // Read back LevelInfo properties from saved package using ut99_format
    let unr = ut99_format::UnrLevel::open(&saved).expect("Read saved unr level");

    let level_info = unr.actors.iter().find(|a| {
        a.class_name.eq_ignore_ascii_case("LevelInfo")
            || a.class_name.eq_ignore_ascii_case("Engine.LevelInfo")
    });
    assert!(level_info.is_some(), "LevelInfo0 actor must be present in saved .unr");

    let info = level_info.unwrap();
    if let Some(prop) = info.properties.get("Title") {
        if let ut99_format::TaggedValue::Str(title) = &prop.value {
            assert_eq!(title, "Persistent Citadel");
        }
    }
    if let Some(prop) = info.properties.get("Author") {
        if let ut99_format::TaggedValue::Str(author) = &prop.value {
            assert_eq!(author, "Test Engineer");
        }
    }
    if let Some(prop) = info.properties.get("IdealPlayerCount") {
        if let ut99_format::TaggedValue::Str(players) = &prop.value {
            assert_eq!(players, "6-10 Players");
        }
    }

    // Clean up
    let _ = fs::remove_file(&saved);
    let bak = save_path.with_extension("unr.bak");
    if bak.exists() {
        let _ = fs::remove_file(&bak);
    }
}

#[test]
fn test_default_maps_directory_resolution() {
    let state = EditorState::default();
    let maps_dir = state.default_maps_directory();
    assert!(!maps_dir.as_os_str().is_empty());
    // Either points to an existing installation / current workspace or standard fallback
    assert!(maps_dir.to_string_lossy().contains("Maps") || maps_dir.exists());
}

#[test]
fn test_save_feedback_t3d_and_gltf_export_state_machines() {
    let mut state = EditorState::default();
    state.new_map();
    let ctx = egui::Context::default();

    // 1. T3D Export feedback test
    let temp_t3d = std::env::temp_dir().join("test_export_async.t3d");
    state.trigger_save_t3d(&temp_t3d);
    assert!(state.save_feedback.is_some());
    assert_eq!(state.save_feedback.as_ref().unwrap().phase, SavePhase::PendingPreparation);
    assert_eq!(state.save_feedback.as_ref().unwrap().total_stages, 2);

    state.process_pending_save_operation(&ctx);
    assert_eq!(state.save_feedback.as_ref().unwrap().phase, SavePhase::Executing);

    state.process_pending_save_operation(&ctx);
    assert_eq!(state.save_feedback.as_ref().unwrap().phase, SavePhase::Completed);
    assert!(temp_t3d.exists());
    let _ = fs::remove_file(&temp_t3d);

    // Reset feedback
    state.save_feedback = None;

    // 2. GLTF Export feedback test
    let temp_gltf = std::env::temp_dir().join("test_export_async.gltf");
    state.trigger_export_gltf(&temp_gltf);
    assert!(state.save_feedback.is_some());
    assert_eq!(state.save_feedback.as_ref().unwrap().phase, SavePhase::PendingPreparation);

    state.process_pending_save_operation(&ctx);
    assert_eq!(state.save_feedback.as_ref().unwrap().phase, SavePhase::Executing);

    state.process_pending_save_operation(&ctx);
    assert_eq!(state.save_feedback.as_ref().unwrap().phase, SavePhase::Completed);
    assert!(temp_gltf.exists());
    let _ = fs::remove_file(&temp_gltf);
}
