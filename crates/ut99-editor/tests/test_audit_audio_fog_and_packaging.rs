use glam::Vec3;
use ut99_bsp::{
    build_sheet_advanced, calculate_sound_attenuation, get_eax_preset, parse_eax_preset,
    simulate_reverb_impulse_samples, EaxEnvironment, SheetAxis, SheetPreset, PF_INVISIBLE,
    PF_MASKED, PF_MODULATED, PF_PORTAL, PF_TRANSLUCENT, PF_TWO_SIDED, PF_UNLIT,
};
use ut99_editor::editor_state::EditorState;
use ut99_editor::renderer::vertex::Uniforms;
use ut99_format::{ActorRecord, TaggedProperty, TaggedValue, UnrLevel, STOCK_PACKAGES};

#[test]
fn test_zone_acoustics_and_eax_reverb_presets() {
    // 1. Verify all 26 canonical EAX environments
    assert_eq!(EaxEnvironment::ALL.len(), 26);
    for &preset in EaxEnvironment::ALL {
        let acoustics = get_eax_preset(preset);
        assert_eq!(acoustics.preset, preset);
        assert!(acoustics.environment_size > 0.0, "Size must be positive for {:?}", preset);
        assert!(acoustics.reverb_decay_time > 0.0, "T60 must be positive for {:?}", preset);
        assert!(acoustics.high_frequency_decay_ratio > 0.0);
        assert!(!acoustics.description.is_empty());
    }

    // 2. Test string parsing of EAX preset names
    assert_eq!(parse_eax_preset("Underwater"), Some(EaxEnvironment::Underwater));
    assert_eq!(parse_eax_preset("cave"), Some(EaxEnvironment::Cave));
    assert_eq!(parse_eax_preset("Arena"), Some(EaxEnvironment::Arena));
    assert_eq!(parse_eax_preset("stone_room"), Some(EaxEnvironment::StoneRoom));
    assert_eq!(parse_eax_preset("InvalidPreset"), None);

    // 3. Physical decay verification of impulse response waveform
    let cave_acoustics = get_eax_preset(EaxEnvironment::Cave);
    let samples = simulate_reverb_impulse_samples(&cave_acoustics, 500);
    assert_eq!(samples.len(), 500);

    let early_rms: f32 = samples[0..50].iter().map(|s| s * s).sum::<f32>() / 50.0;
    let late_rms: f32 = samples[450..500].iter().map(|s| s * s).sum::<f32>() / 50.0;
    assert!(
        early_rms > late_rms * 10.0,
        "Reverb impulse response must show physical energy decay (early: {}, late: {})",
        early_rms,
        late_rms
    );

    // 4. Sound attenuation calculation
    let near_gain = calculate_sound_attenuation(50.0, 400.0);
    assert!(near_gain > 0.75, "Sound near center must have high volume");

    let distant_gain = calculate_sound_attenuation(350.0, 400.0);
    assert!(distant_gain < 0.1, "Sound near radius edge must have near-zero volume");

    let out_of_bounds = calculate_sound_attenuation(500.0, 400.0);
    assert_eq!(out_of_bounds, 0.0, "Sound outside radius must be completely silent");

    // 5. Zone Audio Reports in EditorState
    let mut state = EditorState::default();
    let default_reports = state.get_zone_audio_reports();
    assert!(!default_reports.is_empty());
    assert_eq!(default_reports[0].zone_name, "WorldAcoustics");

    // Create mock level with ZoneInfo and WaterZone
    let mut level = UnrLevel::default();
    let mut zone_actor = ActorRecord::default();
    zone_actor.class_name = "ZoneInfo".to_string();
    zone_actor.name = "ArenaZone".to_string();
    zone_actor.location = Vec3::new(100.0, 200.0, 50.0);
    zone_actor.properties.insert(
        "ZoneEffect".to_string(),
        TaggedProperty {
            name: "ZoneEffect".to_string(),
            array_index: 0,
            value: TaggedValue::Str("Arena".to_string()),
        },
    );
    level.actors.push(zone_actor);

    let mut water_actor = ActorRecord::default();
    water_actor.class_name = "WaterZone".to_string();
    water_actor.name = "SlimePool".to_string();
    water_actor.location = Vec3::new(500.0, -300.0, -100.0);
    level.actors.push(water_actor);

    state.loaded_unr = Some(level);
    let reports = state.get_zone_audio_reports();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].preset, EaxEnvironment::Arena);
    assert_eq!(reports[1].preset, EaxEnvironment::Underwater);

    // 6. Apply EAX preset to selected actor
    state.selected_actor_index = Some(0);
    let applied = state.apply_eax_preset_to_selected_zone(EaxEnvironment::ConcertHall);
    assert!(applied);
    let updated_reports = state.get_zone_audio_reports();
    assert_eq!(updated_reports[0].preset, EaxEnvironment::ConcertHall);
}

#[test]
fn test_distance_fog_and_zone_fog_synchronization() {
    let mut state = EditorState::default();
    assert!(!state.distance_fog_enabled);

    // Verify Uniforms size alignment (128 bytes = 8x 16-byte vec4s)
    assert_eq!(std::mem::size_of::<Uniforms>(), 128);

    // Set up mock level with ZoneInfo containing FogDistance
    let mut level = UnrLevel::default();
    let mut zone_actor = ActorRecord::default();
    zone_actor.class_name = "ZoneInfo".to_string();
    zone_actor.name = "MistySwampZone".to_string();
    zone_actor.properties.insert(
        "FogDistance".to_string(),
        TaggedProperty {
            name: "FogDistance".to_string(),
            array_index: 0,
            value: TaggedValue::Float(3200.0),
        },
    );
    level.actors.push(zone_actor);
    state.loaded_unr = Some(level);

    // Synchronize fog from ZoneInfo
    let synced = state.sync_fog_from_selected_zone();
    assert!(synced);
    assert!(state.distance_fog_enabled);
    assert_eq!(state.fog_end, 3200.0);
    assert!(state.fog_start < state.fog_end);
}

#[test]
fn test_2d_sheet_advanced_builder_axes_and_presets() {
    // 1. SheetAxis::XY (Horizontal: normal along Z)
    let sheet_xy = build_sheet_advanced("WaterSheet", ut99_bsp::CsgOperation::Add, 256.0, 512.0, SheetAxis::XY, SheetPreset::Water);
    assert_eq!(sheet_xy.polys.len(), 2, "Sheet must be double-sided with 2 polygons");
    let p0 = &sheet_xy.polys[0];
    assert!((p0.plane.normal.z.abs() - 1.0).abs() < 1e-3, "XY Sheet normal must point along Z");
    assert_eq!(p0.flags & PF_TRANSLUCENT, PF_TRANSLUCENT);
    assert_eq!(p0.flags & PF_TWO_SIDED, PF_TWO_SIDED);
    assert_eq!(p0.flags & PF_MODULATED, PF_MODULATED);

    // 2. SheetAxis::XZ (Vertical Front: normal along Y)
    let sheet_xz = build_sheet_advanced("BannerSheet", ut99_bsp::CsgOperation::Add, 128.0, 256.0, SheetAxis::XZ, SheetPreset::Banner);
    let p_xz = &sheet_xz.polys[0];
    assert!((p_xz.plane.normal.y.abs() - 1.0).abs() < 1e-3, "XZ Sheet normal must point along Y");
    assert_eq!(p_xz.flags & PF_MASKED, PF_MASKED);
    assert_eq!(p_xz.flags & PF_TWO_SIDED, PF_TWO_SIDED);

    // 3. SheetAxis::YZ (Vertical Side: normal along X)
    let sheet_yz = build_sheet_advanced("PortalSheet", ut99_bsp::CsgOperation::Add, 200.0, 300.0, SheetAxis::YZ, SheetPreset::ZonePortal);
    let p_yz = &sheet_yz.polys[0];
    assert!((p_yz.plane.normal.x.abs() - 1.0).abs() < 1e-3, "YZ Sheet normal must point along X");
    assert_eq!(p_yz.flags & PF_PORTAL, PF_PORTAL);
    assert_eq!(p_yz.flags & PF_INVISIBLE, PF_INVISIBLE);

    // 4. AcidSlime Preset
    let sheet_slime = build_sheet_advanced("SlimeSheet", ut99_bsp::CsgOperation::Add, 100.0, 100.0, SheetAxis::XY, SheetPreset::AcidSlime);
    let p_slime = &sheet_slime.polys[0];
    assert_eq!(p_slime.flags & PF_TRANSLUCENT, PF_TRANSLUCENT);
    assert_eq!(p_slime.flags & PF_UNLIT, PF_UNLIT);

    // 5. Test EditorState integration
    let mut state = EditorState::default();
    state.set_builder_sheet_advanced(384.0, 768.0, SheetAxis::XZ, SheetPreset::Banner);
    assert_eq!(state.builder_sheet_axis, SheetAxis::XZ);
    assert_eq!(state.builder_sheet_preset, SheetPreset::Banner);
    assert_eq!(state.builder_sheet_params, (384.0, 768.0));
    assert_eq!(state.builder_brush.polys.len(), 2);
}

#[test]
fn test_map_packaging_manifest_and_stock_audit() {
    // 1. Check stock database coverage
    assert!(STOCK_PACKAGES.contains(&"botpack"));
    assert!(STOCK_PACKAGES.contains(&"core"));
    assert!(STOCK_PACKAGES.contains(&"engine"));
    assert!(STOCK_PACKAGES.contains(&"indus1"));
    assert!(STOCK_PACKAGES.contains(&"ambancient"));
    assert!(STOCK_PACKAGES.contains(&"foregone"));

    // 2. Build mock level with both stock and custom packages
    let mut level = UnrLevel::default();

    // Stock LevelInfo
    let mut level_info = ActorRecord::default();
    level_info.class_name = "Engine.LevelInfo".to_string();
    level_info.properties.insert("Title".to_string(), TaggedProperty {
        name: "Title".to_string(),
        array_index: 0,
        value: TaggedValue::Str("Gothic Crypt Arena".to_string()),
    });
    level_info.properties.insert("Author".to_string(), TaggedProperty {
        name: "Author".to_string(),
        array_index: 0,
        value: TaggedValue::Str("ProMapper99".to_string()),
    });
    level_info.properties.insert("Song".to_string(), TaggedProperty {
        name: "Song".to_string(),
        array_index: 0,
        value: TaggedValue::Str("Music'Colossus.Colossus'".to_string()),
    });
    level.actors.push(level_info);

    // Custom Weapon Actor
    let mut custom_actor = ActorRecord::default();
    custom_actor.class_name = "CustomWeaponsPack.Railgun".to_string();
    level.actors.push(custom_actor);

    // Actor with custom sound
    let mut sound_actor = ActorRecord::default();
    sound_actor.class_name = "AmbientSound".to_string();
    sound_actor.properties.insert("AmbientSound".to_string(), TaggedProperty {
        name: "AmbientSound".to_string(),
        array_index: 0,
        value: TaggedValue::Str("Sound'CommunityAudio.CreepyDrone'".to_string()),
    });
    level.actors.push(sound_actor);

    let mut state = EditorState::default();
    state.loaded_unr = Some(level);

    let manifest = state.generate_map_manifest();
    assert_eq!(manifest.title, "Gothic Crypt Arena");
    assert_eq!(manifest.author, "ProMapper99");
    assert_eq!(manifest.song, "Music'Colossus.Colossus'");

    // Stock dependencies should contain Colossus (stock music)
    assert!(manifest.stock_dependencies.iter().any(|d| d.name.eq_ignore_ascii_case("Colossus")));

    // Custom dependencies should detect CustomWeaponsPack and CommunityAudio
    assert!(!manifest.is_vanilla_compatible(), "Map with custom railgun must NOT be vanilla compatible");
    assert!(manifest.custom_dependencies.iter().any(|d| d.name == "CustomWeaponsPack"));
    assert!(manifest.custom_dependencies.iter().any(|d| d.name == "CommunityAudio"));

    // Verify Readme generation
    let readme = manifest.generate_readme();
    assert!(readme.contains("Gothic Crypt Arena"));
    assert!(readme.contains("ProMapper99"));
    assert!(readme.contains("CustomWeaponsPack"));
    assert!(readme.contains("CommunityAudio"));
    assert!(readme.contains("Sounds/CommunityAudio"));
    assert!(readme.contains("UnrealTournament/Maps/"));
}
