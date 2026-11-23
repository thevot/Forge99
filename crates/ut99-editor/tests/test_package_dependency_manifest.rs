use glam::Vec3;
use std::fs;
use ut99_bsp::{build_cube, CsgOperation};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_vanilla_manifest_generation() {
    let mut state = EditorState::default();
    state.map_path = Some(std::path::PathBuf::from("DM-VanillaArena.unr"));
    state.level_title = "Vanilla Arena".to_string();
    state.level_author = "Epic Mapper".to_string();
    state.level_ideal_player_count = "2-4 Players".to_string();
    state.level_song = "Cannon.umx".to_string();

    // Add brush with stock texture
    let mut brush = build_cube("Room", CsgOperation::Add, Vec3::splat(256.0));
    for p in &mut brush.polys {
        p.texture = "Indus1.Indus_B01".to_string();
    }
    state.world_brushes.push(brush);

    let manifest = state.generate_map_manifest();

    assert_eq!(manifest.map_name, "DM-VanillaArena");
    assert_eq!(manifest.title, "Vanilla Arena");
    assert_eq!(manifest.author, "Epic Mapper");
    assert_eq!(manifest.song, "Cannon.umx");
    assert!(manifest.is_vanilla_compatible(), "Map with only stock assets must be 100% vanilla compatible");
    assert!(manifest.custom_dependencies.is_empty(), "No custom dependencies should exist");

    // Stock dependencies must include Indus1, Botpack, Cannon, Core, Engine
    assert!(manifest.stock_dependencies.iter().any(|d| d.name.eq_ignore_ascii_case("indus1")));
    assert!(manifest.stock_dependencies.iter().any(|d| d.name.eq_ignore_ascii_case("cannon")));

    let readme = manifest.generate_readme();
    assert!(readme.contains("Map Title: Vanilla Arena"));
    assert!(readme.contains("Filename:  DM-VanillaArena.unr"));
    assert!(readme.contains("Standard Stock Packages Only"));
}

#[test]
fn test_custom_package_detection() {
    let mut state = EditorState::default();
    state.map_path = Some(std::path::PathBuf::from("CTF-CyberDusk.unr"));
    state.level_title = "Cyber Dusk CTF".to_string();
    state.level_author = "Community Star".to_string();
    state.level_song = "CyberGroove.umx".to_string(); // Custom non-stock track

    // Add brush with third-party custom texture
    let mut brush = build_cube("CyberTower", CsgOperation::Add, Vec3::splat(512.0));
    for p in &mut brush.polys {
        p.texture = "NeonCityTextures.HoloGrid_01".to_string(); // Custom non-stock package
    }
    state.world_brushes.push(brush);

    let manifest = state.generate_map_manifest();

    assert!(!manifest.is_vanilla_compatible(), "Map with custom texture & music must not be vanilla compatible");
    assert_eq!(manifest.custom_dependencies.len(), 2, "Must detect 2 custom dependencies");

    let has_texture_dep = manifest.custom_dependencies.iter().any(|d| d.name == "NeonCityTextures" && d.package_type == "Texture");
    assert!(has_texture_dep, "Must detect NeonCityTextures as custom Texture package");

    let has_music_dep = manifest.custom_dependencies.iter().any(|d| d.name == "CyberGroove" && d.package_type == "Music");
    assert!(has_music_dep, "Must detect CyberGroove as custom Music package");

    let readme = manifest.generate_readme();
    assert!(readme.contains("NeonCityTextures (Textures/NeonCityTextures)"));
    assert!(readme.contains("CyberGroove (Music/CyberGroove)"));
    assert!(readme.contains("CRITICAL: The following custom packages must be copied"));
}

#[test]
fn test_manifest_readme_export_and_sync() {
    let mut state = EditorState::default();
    state.map_path = Some(std::path::PathBuf::from("DOM-TestLab.unr"));
    state.level_title = "Test Laboratory".to_string();
    state.level_author = "Scientist".to_string();

    let temp_dir = std::env::temp_dir();
    let readme_path = temp_dir.join("test_dom_lab_readme.txt");

    state.refresh_map_manifest();
    assert!(state.cached_map_manifest.is_some());

    let export_res = state.export_manifest_readme(&readme_path);
    assert!(export_res.is_ok(), "Readme export to disk must succeed");
    assert!(readme_path.exists());

    let file_text = fs::read_to_string(&readme_path).expect("Must read exported file");
    assert!(file_text.contains("Map Title: Test Laboratory"));
    assert!(file_text.contains("Filename:  DOM-TestLab.unr"));

    let clipboard_text = state.get_manifest_readme_text();
    assert_eq!(file_text, clipboard_text, "Disk text and clipboard text must match identically");

    let _ = fs::remove_file(&readme_path);
}
