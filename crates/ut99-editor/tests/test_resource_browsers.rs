use glam::Vec3;
use ut99_editor::EditorState;
use ut99_format::package::resource::{
    build_class_tree, parse_music_export, parse_sound_export, ClassResource,
};

#[test]
fn test_actor_class_browser_and_placement() {
    let mut state = EditorState::default();

    // Verify actor classes are populated (either from System/*.u or canonical fallback)
    assert!(
        !state.all_classes.is_empty(),
        "Actor classes must be loaded in state.all_classes"
    );
    println!("Loaded {} actor classes", state.all_classes.len());

    let class_names: Vec<&str> = state.all_classes.iter().map(|c| c.name.as_str()).collect();
    assert!(class_names.contains(&"Actor"), "Tree must contain Actor");
    assert!(class_names.contains(&"Pawn"), "Tree must contain Pawn");
    assert!(class_names.contains(&"PathNode"), "Tree must contain PathNode");
    assert!(class_names.contains(&"Light"), "Tree must contain Light");
    assert!(class_names.contains(&"Mover"), "Tree must contain Mover");
    assert!(class_names.contains(&"Inventory"), "Tree must contain Inventory");

    // Verify class tree root
    let tree = state.class_tree.as_ref().expect("Class tree should be built");
    assert_eq!(tree.name, "Object");
    assert!(!tree.children.is_empty());

    // Test custom class hierarchy builder
    let custom_classes = vec![
        ClassResource {
            name: "Actor".into(),
            super_name: "Object".into(),
            package: "Engine".into(),
        },
        ClassResource {
            name: "Pawn".into(),
            super_name: "Actor".into(),
            package: "Engine".into(),
        },
        ClassResource {
            name: "Bot".into(),
            super_name: "Pawn".into(),
            package: "Botpack".into(),
        },
    ];
    let custom_tree = build_class_tree(&custom_classes);
    assert_eq!(custom_tree.name, "Object");
    assert_eq!(custom_tree.children.len(), 1);
    assert_eq!(custom_tree.children[0].name, "Actor");
    assert_eq!(custom_tree.children[0].children[0].name, "Pawn");
    assert_eq!(custom_tree.children[0].children[0].children[0].name, "Bot");

    // Test placing an actor of class
    let initial_actor_count = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);
    let spawn_pos = Vec3::new(256.0, 512.0, 128.0);
    state.builder_brush.location = spawn_pos;
    state.place_actor_of_class("UT_ShieldBelt");

    let unr = state.loaded_unr.as_ref().expect("UNR level must be initialized");
    assert_eq!(
        unr.actors.len(),
        initial_actor_count + 1,
        "Actor should be placed in loaded_unr"
    );
    let placed = unr.actors.last().unwrap();
    assert_eq!(placed.class_name, "UT_ShieldBelt");
    assert_eq!(placed.location, spawn_pos);
    assert_eq!(placed.name, "UT_ShieldBelt_0");
}

#[test]
fn test_sound_resource_extraction_and_waveform() {
    let mut state = EditorState::default();

    // 1. Synthetic RIFF WAVE buffer
    let mut wav_data = Vec::new();
    wav_data.extend_from_slice(b"RIFF");
    let file_size: u32 = 44 + 100 - 8;
    wav_data.extend_from_slice(&file_size.to_le_bytes());
    wav_data.extend_from_slice(b"WAVE");
    wav_data.extend_from_slice(b"fmt ");
    let fmt_chunk_size: u32 = 16;
    wav_data.extend_from_slice(&fmt_chunk_size.to_le_bytes());
    let audio_format: u16 = 1; // PCM
    wav_data.extend_from_slice(&audio_format.to_le_bytes());
    let channels: u16 = 1; // Mono
    wav_data.extend_from_slice(&channels.to_le_bytes());
    let sample_rate: u32 = 22050;
    wav_data.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate: u32 = 22050 * 2;
    wav_data.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align: u16 = 2;
    wav_data.extend_from_slice(&block_align.to_le_bytes());
    let bits_per_sample: u16 = 16;
    wav_data.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav_data.extend_from_slice(b"data");
    let data_bytes_len: u32 = 100;
    wav_data.extend_from_slice(&data_bytes_len.to_le_bytes());

    // 50 samples of 16-bit PCM (100 bytes)
    for i in 0..50 {
        let sample: i16 = ((i as f32 / 50.0 * std::f32::consts::PI * 2.0).sin() * 32000.0) as i16;
        wav_data.extend_from_slice(&sample.to_le_bytes());
    }

    let parsed = parse_sound_export("TestGunShot", "Weapons", &wav_data);
    assert!(parsed.is_some(), "Should parse valid synthetic RIFF WAVE");
    let snd = parsed.unwrap();
    assert_eq!(snd.name, "TestGunShot");
    assert_eq!(snd.package, "Weapons");
    assert_eq!(snd.channels, 1);
    assert_eq!(snd.sample_rate, 22050);
    assert_eq!(snd.bits_per_sample, 16);
    assert!((snd.duration_seconds - (50.0 / 22050.0)).abs() < 1e-4);

    // 2. Discover sound packages
    state.scan_sound_packages();
    println!("Discovered {} sound packages:", state.sound_packages.len());
    for p in state.sound_packages.iter().take(5) {
        println!(" - {}", p);
    }

    if let Some(first_pkg) = state.sound_packages.first().cloned() {
        println!("Testing loading sound package: {}", first_pkg);
        state.load_sound_package(&first_pkg);
        println!(
            "Loaded {} sounds from package {}",
            state.loaded_sounds.len(),
            first_pkg
        );
        if let Some(first_snd) = state.loaded_sounds.first() {
            println!(
                "First sound: {} ({} Hz, {} ch, {:.2}s)",
                first_snd.name,
                first_snd.sample_rate,
                first_snd.channels,
                first_snd.duration_seconds
            );
            assert!(!first_snd.wav_bytes.is_empty());
        }
    }
}

#[test]
fn test_music_resource_tracker_formats() {
    let mut state = EditorState::default();

    // 1. Synthetic tracker module detections
    // IT (Impulse Tracker)
    let mut it_data = vec![0u8; 100];
    it_data[0..4].copy_from_slice(b"IMPM");
    it_data[4..20].copy_from_slice(b"Foregone Song   ");
    let it_res = parse_music_export("Foregone", "Botpack", &it_data).unwrap();
    assert_eq!(it_res.format, "IT (Impulse Tracker)");

    // XM (FastTracker II)
    let mut xm_data = vec![0u8; 200];
    xm_data[0..16].copy_from_slice(b"Extended Module:");
    xm_data[17..33].copy_from_slice(b"Super UnrealSong");
    let xm_res = parse_music_export("UnrealTrack", "Unreal", &xm_data).unwrap();
    assert_eq!(xm_res.format, "XM (FastTracker II)");

    // S3M (ScreamTracker 3)
    let mut s3m_data = vec![0u8; 100];
    s3m_data[44..48].copy_from_slice(b"SCRM");
    let s3m_res = parse_music_export("Stalker", "Unreal", &s3m_data).unwrap();
    assert_eq!(s3m_res.format, "S3M (ScreamTracker 3)");

    // Ogg Vorbis
    let mut ogg_data = vec![0u8; 100];
    ogg_data[0..4].copy_from_slice(b"OggS");
    let ogg_res = parse_music_export("NewTrack", "v227", &ogg_data).unwrap();
    assert_eq!(ogg_res.format, "OGG Vorbis");

    // MOD (ProTracker)
    let mut mod_data = vec![0u8; 1100];
    mod_data[0..8].copy_from_slice(b"Chiptune");
    mod_data[1080..1084].copy_from_slice(b"M.K.");
    let mod_res = parse_music_export("Retro", "OldPkg", &mod_data).unwrap();
    assert_eq!(mod_res.format, "MOD (ProTracker)");

    // 2. Discover music packages
    state.scan_music_packages();
    println!("Discovered {} music packages:", state.music_packages.len());
    for p in state.music_packages.iter().take(5) {
        println!(" - {}", p);
    }

    if let Some(first_music_pkg) = state.music_packages.first().cloned() {
        println!("Testing loading music package: {}", first_music_pkg);
        state.load_music_package(&first_music_pkg);
        if let Some(ref track) = state.loaded_music {
            println!(
                "Loaded music track: {} ({}) - {}",
                track.name, track.format, track.title
            );
        }
    }
}

#[test]
fn test_sound_assignment_to_movers() {
    let mut state = EditorState::default();

    // Add a mover
    state.add_mover_from_builder();
    assert_eq!(state.movers.len(), 1);

    // Assign sounds to the mover
    {
        let mover = &mut state.movers[0];
        mover.opening_sound = "Doors.DoorOpen01".to_string();
        mover.closed_sound = "Doors.DoorClosed01".to_string();
    }

    let mover = &state.movers[0];
    assert_eq!(mover.opening_sound, "Doors.DoorOpen01");
    assert_eq!(mover.closed_sound, "Doors.DoorClosed01");
}

#[test]
fn test_texture_group_filtering_and_export() {
    use ut99_format::{UtxPackage, UtxTexture};

    let mut state = EditorState::default();
    assert_eq!(state.selected_texture_group, "All", "Default selected group must be 'All'");

    // 1. Build a synthetic package with textures in different groups
    let t1 = UtxTexture {
        name: "WallBrick01".to_string(),
        group: "Wall".to_string(),
        width: 2,
        height: 2,
        format: 1,
        masked: false,
        rgba_pixels: vec![255, 0, 0, 255, 200, 0, 0, 255, 150, 0, 0, 255, 100, 0, 0, 255],
    };
    let t2 = UtxTexture {
        name: "FloorTile01".to_string(),
        group: "Floor".to_string(),
        width: 2,
        height: 2,
        format: 1,
        masked: false,
        rgba_pixels: vec![0, 255, 0, 255, 0, 200, 0, 255, 0, 150, 0, 255, 0, 100, 0, 255],
    };
    let t3 = UtxTexture {
        name: "IconLogo".to_string(),
        group: "".to_string(), // Ungrouped / None
        width: 2,
        height: 2,
        format: 1,
        masked: false,
        rgba_pixels: vec![0, 0, 255, 255, 0, 0, 200, 255, 0, 0, 150, 255, 0, 0, 100, 255],
    };

    let pkg = UtxPackage {
        package_name: "SyntheticTestPack".to_string(),
        textures: vec![t1, t2, t3],
    };

    state.texture_manager.insert_package("SyntheticTestPack".to_string(), pkg);

    // 2. Test group extraction
    let groups = state.texture_manager.get_package_groups("SyntheticTestPack");
    assert_eq!(groups, vec!["All", "In Use", "Floor", "None", "Wall"], "Groups should include 'All', 'In Use', sorted unique groups, and 'None' for ungrouped");

    // 3. Test single texture export to PNG
    let test_dir = std::env::temp_dir().join(format!("forge99_tex_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&test_dir).expect("Failed to create test temp directory");

    let single_export_path = test_dir.join("WallBrick01.png");
    let export_res = state.texture_manager.export_texture_to_file("SyntheticTestPack", "WallBrick01", &single_export_path);
    assert!(export_res.is_ok(), "Single texture export should succeed: {:?}", export_res.err());
    assert!(single_export_path.exists(), "Exported PNG file must exist on disk");

    // Verify written PNG image content via image crate
    let loaded_img = image::open(&single_export_path).expect("Exported image must be a valid readable PNG");
    assert_eq!(loaded_img.width(), 2);
    assert_eq!(loaded_img.height(), 2);

    // 4. Test entire package export to directory
    let package_export_dir = test_dir.join("FullExport");
    let total_exported = state.texture_manager.export_package_to_directory("SyntheticTestPack", &package_export_dir)
        .expect("Full package export should succeed");
    assert_eq!(total_exported, 3, "All 3 textures must be exported");

    assert!(package_export_dir.join("Wall").join("WallBrick01.png").exists(), "Grouped texture in 'Wall' must exist");
    assert!(package_export_dir.join("Floor").join("FloorTile01.png").exists(), "Grouped texture in 'Floor' must exist");
    assert!(package_export_dir.join("IconLogo.png").exists(), "Ungrouped texture must exist at root of export dir");

    // Clean up
    let _ = std::fs::remove_dir_all(&test_dir);
}

#[test]
fn test_texture_manager_memory_telemetry_and_cache_cleanup() {
    use ut99_format::{UtxPackage, UtxTexture};

    let mut state = EditorState::default();
    let initial_packages = state.texture_manager.loaded_package_count();

    // Insert synthetic package
    let t = UtxTexture {
        name: "TelemetryTex".to_string(),
        group: "UI".to_string(),
        width: 4,
        height: 4,
        format: 1,
        masked: false,
        rgba_pixels: vec![255; 4 * 4 * 4],
    };
    let pkg = UtxPackage {
        package_name: "TelemetryPkg".to_string(),
        textures: vec![t],
    };
    state.texture_manager.insert_package("TelemetryPkg".to_string(), pkg);

    assert_eq!(state.texture_manager.loaded_package_count(), initial_packages + 1);

    // Initial thumbnail cache is empty
    assert_eq!(state.texture_manager.cached_thumbnail_count(), 0);

    // Clear thumbnail cache
    state.texture_manager.clear_thumbnail_cache();
    assert_eq!(state.texture_manager.cached_thumbnail_count(), 0);
}

#[test]
fn test_fase_0_to_4_modernization_and_unr_roundtrip() {
    use glam::Vec3;

    let mut state = EditorState::default();
    state.new_map();

    // 1. Add a Mover with OpeningSound and ClosedSound
    state.set_builder_cube(Vec3::new(128.0, 128.0, 64.0));
    state.add_mover_from_builder();
    assert_eq!(state.movers.len(), 1);
    state.movers[0].opening_sound = "DoorsAnc.LiftStart".to_string();
    state.movers[0].closed_sound = "DoorsAnc.LiftStop".to_string();

    // 2. Add two PathNodes and build AI paths (ReachSpecs)
    state.builder_brush.location = Vec3::new(-128.0, 0.0, -192.0);
    state.add_path_node_at_builder();
    state.builder_brush.location = Vec3::new(128.0, 0.0, -192.0);
    state.add_path_node_at_builder();
    state.build_ai_paths();
    assert!(!state.nav_graph.reach_specs.is_empty());

    // 3. Test true 3D CSG Intersect & Deintersect on Red Builder Brush
    state.set_builder_cube(Vec3::splat(256.0));
    state.builder_brush.location = Vec3::ZERO;
    state.csg_deintersect();
    assert!(!state.builder_brush.polys.is_empty());

    // 4. Test native .unr save & load round-trip (LevelInfo0 at Actors[0], ReachSpecs, Mover sounds)
    let tmp_unr = std::env::temp_dir().join(format!(
        "forge99_roundtrip_{}.unr",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    state.save_unr_native(&tmp_unr).expect("Native UNR save must succeed");
    assert!(tmp_unr.exists());

    let mut loaded_state = EditorState::default();
    loaded_state.load_unr(&tmp_unr).expect("Native UNR load must succeed");
    assert!(!loaded_state.movers.is_empty());
    assert!(loaded_state.movers[0].opening_sound.contains("LiftStart"));
    assert!(loaded_state.movers[0].closed_sound.contains("LiftStop"));
    assert!(!loaded_state.nav_graph.reach_specs.is_empty());

    let _ = std::fs::remove_file(&tmp_unr);
}

