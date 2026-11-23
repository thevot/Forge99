// crates/ut99-editor/tests/test_map_generator_wizard.rs
//
// End-to-End Integration Tests for the New Valid UT99 Map Generator Wizard & Official .unr DNA Cloner.
// Tests:
// 1. All Game Modes (DM, TDM, 1v1, CTF, DOM, AS) & Canonical Archetypes in EditorState
// 2. Zero LevelDoctor Errors + Populated Bot AI ReachSpecs
// 3. Procedural Seed Generator & Symmetry Modes
// 4. Official .unr Map DNA Forensic Extraction & Cloning into Generated Maps
// 5. Atomic Undo / Redo via Action::ReplaceEntireLevel
// 6. Native Binary .unr Save, Reload & Package Integrity Validation

use ut99_bsp::{
    WizardArchitecturalDetail,
    IssueSeverity, MapWizardConfig, WizardArchetype, WizardFeatureToggles, WizardGameMode,
    WizardLayoutStrategy, WizardMapScale, WizardRoomShape, WizardSymmetryMode, WizardThemePreset,
};
use ut99_editor::EditorState;
use ut99_format::unr::UnrLevel;
mod common;

#[test]
fn test_wizard_new_archetypes_octagons_ramps_and_teleporters_zero_errors() {
    let mut state = EditorState::default();
    let combos = [
        (
            WizardGameMode::DeathMatch,
            WizardArchetype::VerticalTowerShaft,
            WizardRoomShape::Octagonal,
        ),
        (
            WizardGameMode::Domination,
            WizardArchetype::ThreeWayRingArena,
            WizardRoomShape::Mixed,
        ),
        (
            WizardGameMode::Assault,
            WizardArchetype::FortressSiegeLine,
            WizardRoomShape::Chamfered,
        ),
    ];

    for (mode, arch, shape) in combos {
        state.map_wizard_config = MapWizardConfig {
            game_mode: mode,
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: arch,
            room_shape: shape,
            scale: WizardMapScale::Standard,
            toggles: WizardFeatureToggles::full_playtest_ready(),
            auto_run_build_and_audit: true,
            ..Default::default()
        };
        state.map_wizard_config.sync_mode_conventions();
        let (b_cnt, _, a_cnt) = state.generate_map_from_wizard();
        assert!(b_cnt >= 4 && a_cnt >= 10);

        let errors: Vec<_> = state
            .map_audit_issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "Mode {:?} + Archetype {:?} + Shape {:?} produced LevelDoctor errors: {:?}",
            mode,
            arch,
            shape,
            errors
        );
    }
}

#[test]
fn test_wizard_all_game_modes_and_presets_zero_errors() {
    let mut state = EditorState::default();

    for preset_idx in 0..6 {
        state.apply_map_wizard_quick_preset(preset_idx);
        let (b_cnt, _m_cnt, a_cnt) = state.generate_map_from_wizard();

        assert!(
            b_cnt >= 2 && !state.world_brushes.is_empty(),
            "Preset {preset_idx} must generate CSG brushes"
        );
        assert!(
            a_cnt >= 6,
            "Preset {preset_idx} must generate actors (LevelInfo, PlayerStarts, etc.)"
        );
        let unr = state
            .loaded_unr
            .as_ref()
            .expect("Preset must populate loaded_unr");
        assert!(
            unr.player_starts.len() >= 4,
            "Preset {preset_idx} must place at least 4 PlayerStarts"
        );
        assert!(
            !state.nav_graph.nodes.is_empty(),
            "Preset {preset_idx} must build Bot AI navigation nodes"
        );
        assert!(
            !state.nav_graph.reach_specs.is_empty(),
            "Preset {preset_idx} must build Bot AI ReachSpecs"
        );

        let errors: Vec<_> = state
            .map_audit_issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "Preset {preset_idx} ({}) produced LevelDoctor errors: {:?}",
            state.map_wizard_config.title,
            errors
        );
    }
}

#[test]
fn test_wizard_procedural_seed_and_symmetry_modes() {
    let mut state = EditorState::default();
    state.map_wizard_config = MapWizardConfig {
        map_name: "CTF-ProceduralSeedTest".to_string(),
        title: "Procedural Seed CTF".to_string(),
        game_mode: WizardGameMode::CaptureTheFlag,
        layout_strategy: WizardLayoutStrategy::ProceduralSeed,
        scale: WizardMapScale::Standard,
        seed: 1999_09_30,
        procedural_room_count: 5,
        procedural_vertical_tiers: 2,
        procedural_loop_ratio: 0.85,
        symmetry_mode: WizardSymmetryMode::MirrorX,
        theme: WizardThemePreset::CoretTechFacility,
        toggles: WizardFeatureToggles::full_playtest_ready(),
        auto_run_build_and_audit: true,
        ..Default::default()
    };

    let (b_a, m_a, a_a) = state.generate_map_from_wizard();
    let brush_count_a = state.world_brushes.len();
    let actor_count_a = state
        .loaded_unr
        .as_ref()
        .map(|u| u.actors.len())
        .unwrap_or(0);

    // Re-generate with exact same seed and verify deterministic output
    let (b_b, m_b, a_b) = state.generate_map_from_wizard();
    assert_eq!(b_a, b_b);
    assert_eq!(m_a, m_b);
    assert_eq!(a_a, a_b);
    assert_eq!(brush_count_a, state.world_brushes.len());
    assert_eq!(
        actor_count_a,
        state
            .loaded_unr
            .as_ref()
            .map(|u| u.actors.len())
            .unwrap_or(0)
    );

    let flag_bases = state
        .loaded_unr
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .filter(|a| a.class_name == "FlagBase")
        .count();
    assert_eq!(flag_bases, 2, "CTF must have 2 FlagBases");
}

#[test]
fn test_wizard_official_map_dna_cloning_end_to_end() {
    let mut state = EditorState::default();
    state.scan_official_maps_for_wizard();

    if state.available_official_maps.is_empty() {
        // Skip only if UnrealTournament/Maps is not present in the test environment
        return;
    }

    let target_maps = [
        ("DM-Morpheus.unr", WizardGameMode::DeathMatch, WizardArchetype::OrbitalOutpostSkybox),
        ("CTF-Face.unr", WizardGameMode::CaptureTheFlag, WizardArchetype::SymmetricalTwinBases),
        ("DOM-Sesmar.unr", WizardGameMode::Domination, WizardArchetype::InterconnectedComplex),
        ("AS-Frigate.unr", WizardGameMode::Assault, WizardArchetype::InterconnectedComplex),
    ];

    for (map_file, mode, archetype) in target_maps {
        if !state
            .available_official_maps
            .iter()
            .any(|m| m.eq_ignore_ascii_case(map_file))
        {
            continue;
        }

        let dna = state
            .clone_dna_from_official_map(map_file)
            .unwrap_or_else(|e| panic!("Failed to clone DNA from {map_file}: {e}"));

        assert!(
            !dna.textures.wall_textures.is_empty(),
            "{map_file} DNA should have extracted wall textures"
        );

        // Verify forensic Markdown report generation
        let md_report = state
            .format_cloned_dna_markdown_report()
            .expect("Expected forensic DNA markdown report after cloning");
        assert!(
            md_report.contains(&dna.map_name),
            "Markdown report should contain map name"
        );
        assert!(
            md_report.contains("CSG & BSP Architecture"),
            "Markdown report should contain CSG & BSP Architecture section"
        );

        // Verify automatic topology & mode inference from cloned DNA
        assert!(
            state.apply_cloned_dna_topology_and_mode(),
            "Failed to auto-match topology from {map_file}"
        );
        assert_eq!(
            state.map_wizard_config.game_mode, mode,
            "Auto-matched game mode for {map_file} should match expected mode"
        );
        assert_eq!(
            state.map_wizard_config.archetype, archetype,
            "Auto-matched archetype for {map_file} should match expected archetype"
        );

        let (b_cnt, _, _) = state.generate_map_from_wizard();
        assert!(b_cnt >= 2);

        let errors: Vec<_> = state
            .map_audit_issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "Generated map with cloned DNA from {map_file} had LevelDoctor errors: {:?}",
            errors
        );
    }

    // Verify full official corpus study & markdown report generation
    let analyzed_count = state
        .analyze_official_map_corpus()
        .expect("Expected analyze_official_map_corpus to succeed");
    assert!(
        analyzed_count >= 10,
        "Expected at least 10 official UT99 maps in corpus, got {analyzed_count}"
    );
    assert!(
        !state.official_corpus_stats.is_empty(),
        "Expected per-mode corpus stats to be populated"
    );
    let corpus_md = state
        .format_official_corpus_markdown_report()
        .expect("Expected corpus Markdown report");
    assert!(
        corpus_md.contains("Map Corpus Summary") && corpus_md.contains("provenance and decoding completeness are not guaranteed"),
        "Corpus Markdown report should contain title header"
    );

    // Also verify Escape dialog stack closes show_map_wizard_dialog
    state.show_map_wizard_dialog = true;
    assert!(state.close_one_floating_dialog());
    assert!(!state.show_map_wizard_dialog);
}

#[test]
fn test_wizard_atomic_undo_and_redo() {
    let mut state = EditorState::default();
    state.new_map();
    state.level_title = "Initial Blank Map".to_string();
    let initial_brush_count = state.world_brushes.len();
    let initial_mover_count = state.movers.len();

    // Generate a full CTF map via Wizard
    state.apply_map_wizard_quick_preset(3); // Symmetrical CTF Citadel
    state.map_wizard_config.title = "CTF-WizardUndoTest".to_string();
    let (b_cnt, _, _) = state.generate_map_from_wizard();
    assert_eq!(state.level_audit_freshness(), ut99_editor::editor_state::LevelAuditFreshness::Current);

    let wizard_brush_count = state.world_brushes.len();
    let wizard_mover_count = state.movers.len();
    assert_eq!(b_cnt, wizard_brush_count);
    assert!(wizard_brush_count > initial_brush_count);
    assert_eq!(state.level_title, "CTF-WizardUndoTest");

    // Undo should atomically restore the initial level state
    state.undo();
    assert_eq!(state.level_audit_freshness(), ut99_editor::editor_state::LevelAuditFreshness::Stale);
    assert_eq!(state.world_brushes.len(), initial_brush_count);
    assert_eq!(state.movers.len(), initial_mover_count);
    assert_eq!(state.level_title, "Initial Blank Map");

    // Redo should atomically restore the wizard-generated level
    state.redo();
    assert_eq!(state.world_brushes.len(), wizard_brush_count);
    assert_eq!(state.movers.len(), wizard_mover_count);
    assert_eq!(state.level_title, "CTF-WizardUndoTest");
}

#[test]
fn test_wizard_save_unr_and_reload_package_validation() {
    let mut state = EditorState::default();
    state.apply_map_wizard_quick_preset(0); // 100% Playtest Ready
    state.map_wizard_config.map_name = "DM-WizardRoundTrip".to_string();
    state.map_wizard_config.title = "DM-WizardRoundTrip".to_string();

    let (b_cnt, _, _) = state.generate_map_from_wizard();
    assert!(b_cnt >= 4);

    let temp_dir = common::TestDir::new();
    let out_path = temp_dir.path().join("DM-WizardRoundTrip_Test.unr");

    state
        .save_unr_native(&out_path)
        .expect("Saving wizard-generated map to .unr failed");
    assert!(out_path.exists(), "Saved .unr file must exist on disk");

    // Reload via UnrLevel::open and validate binary package integrity
    let reloaded =
        UnrLevel::open(&out_path).expect("UnrLevel::open failed on wizard-generated map");
    let pkg_report = UnrLevel::validate_package(&reloaded.package);
    assert!(
        pkg_report.is_valid && pkg_report.errors.is_empty(),
        "Saved .unr package had structural errors: {:?}",
        pkg_report.errors
    );

    assert!(
        reloaded.actors.iter().any(|a| a.class_name == "LevelInfo"),
        "Reloaded .unr must contain LevelInfo"
    );
    assert!(
        reloaded
            .actors
            .iter()
            .filter(|a| a.class_name == "PlayerStart")
            .count()
            >= 4,
        "Reloaded .unr must contain at least 4 PlayerStarts"
    );
    assert!(
        !reloaded.brushes.is_empty() && reloaded.world_bsp.is_some(),
        "Reloaded .unr must contain compiled UModel brushes and world_bsp"
    );

    let visible_teles: Vec<_> = reloaded
        .actors
        .iter()
        .filter(|a| a.class_name.eq_ignore_ascii_case("VisibleTeleporter"))
        .collect();
    assert_eq!(
        visible_teles.len(),
        2,
        "Reloaded .unr must contain 2 VisibleTeleporter actors so portals are visible in-game"
    );
    for vt in &visible_teles {
        assert_eq!(
            vt.object_paths.get("__class_package").map(|s| s.as_str()),
            Some("BotPack"),
            "VisibleTeleporter must be imported from BotPack"
        );
    }

}

#[test]
fn test_wizard_gui_encoding_no_mojibake_or_fe0f() {
    let dialog_src = include_str!("../src/gui/map_wizard_dialog.rs");
    let armory_src = include_str!("../../ut99-bsp/src/armory.rs");

    for (label, src) in [("map_wizard_dialog.rs", dialog_src), ("armory.rs", armory_src)] {
        for ch in src.chars() {
            let code = ch as u32;
            assert!(
                !(0x80..=0x9F).contains(&code),
                "{label} contains C1 control character U+{code:04X} (mojibake corruption)"
            );
            assert_ne!(
                code, 0xFE0F,
                "{label} contains U+FE0F variation selector which renders as a missing-glyph box in egui"
            );
        }
        for bad_seq in ["\u{00f0}\u{0178}", "\u{00c3}\u{2014}", "\u{00e2}\u{2014}", "\u{00e2}\u{0153}"] {
            assert!(
                !src.contains(bad_seq),
                "{label} contains double-encoded UTF-8 mojibake sequence"
            );
        }
    }
}

#[test]
fn test_regenerate_ctf_kulo_with_visible_teleporters() {
    let mut state = EditorState::default();
    state.apply_map_wizard_quick_preset(3); // Symmetrical CTF
    state.map_wizard_config.map_name = "CTF-KULO".to_string();
    state.map_wizard_config.title = "CTF-KULO".to_string();
    state.generate_map_from_wizard();

    {
        let temp_dir = common::TestDir::new();
        let out_path = temp_dir.path().join("CTF-KULO.unr");
        state
            .save_unr_native(&out_path)
            .expect("Failed to save regenerated CTF-KULO.unr");
        let reloaded = UnrLevel::open(&out_path).expect("Failed to reopen CTF-KULO.unr");
        let vis_count = reloaded
            .actors
            .iter()
            .filter(|a| a.class_name.to_ascii_lowercase().contains("visibleteleporter"))
            .count();
        assert_eq!(
            vis_count, 2,
            "CTF-KULO.unr must contain 2 VisibleTeleporter actors"
        );
    }
}

#[test]
fn test_generate_and_export_ctf_lavagiant_symmetric() {
    let mut state = EditorState::default();
    state.map_wizard_config.game_mode = WizardGameMode::CaptureTheFlag;
    state.map_wizard_config.archetype = WizardArchetype::CanyonCrestCitadels;
    state.map_wizard_config.theme = WizardThemePreset::VolcanicCaldera;
    state.map_wizard_config.map_name = "CTF-LavaGiantSymmetric".to_string();
    state.map_wizard_config.title = "Lava Giant (Symmetric Tournament Edition)".to_string();
    state.map_wizard_config.author = "Forge99 Community Contributors".to_string();
    state.map_wizard_config.ideal_player_count = "8-16".to_string();
    state.map_wizard_config.toggles.include_weapons_and_pickups = true;
    state.map_wizard_config.toggles.include_skybox = true;
    state.map_wizard_config.toggles.include_environmental_hazards = true;
    state.map_wizard_config.toggles.include_ramps_and_bridges = true;

    let (b_cnt, _m_cnt, a_cnt) = state.generate_map_from_wizard();
    assert!(b_cnt >= 8, "Expected at least 8 brushes for CanyonCrestCitadels, got {}", b_cnt);
    assert!(a_cnt >= 20, "Expected at least 20 actors, got {}", a_cnt);

    {
        let temp_dir = common::TestDir::new();
        let unr_path = temp_dir.path().join("CTF-LavaGiantSymmetric.unr");
        let t3d_path = temp_dir.path().join("CTF-LavaGiantSymmetric.t3d");

        state.save_unr_native(&unr_path).expect("Failed to save CTF-LavaGiantSymmetric.unr");
        state.save_t3d(&t3d_path).expect("Failed to save CTF-LavaGiantSymmetric.t3d");

        assert!(unr_path.exists(), "CTF-LavaGiantSymmetric.unr must exist on disk");
        assert!(t3d_path.exists(), "CTF-LavaGiantSymmetric.t3d must exist on disk");

        // Verify package integrity with UnrLevel
        let reloaded = UnrLevel::open(&unr_path).expect("Failed to open exported CTF-LavaGiantSymmetric.unr");
        let pkg_report = UnrLevel::validate_package(&reloaded.package);
        assert!(pkg_report.is_valid, "Package validation failed: {:?}", pkg_report.errors);

        // Verify flagbases
        let flags: Vec<_> = reloaded.actors.iter().filter(|a| a.class_name.contains("FlagBase")).collect();
        assert_eq!(flags.len(), 2, "Expected exactly 2 FlagBases");

        // Verify PlayerStarts: all 8 starts must be at ground floor (Z < 100) inside fortress keeps (X.abs >= 2500)
        let player_starts: Vec<_> = reloaded.player_starts.iter().collect();
        assert_eq!(player_starts.len(), 8, "Expected exactly 8 PlayerStarts in exported level");
        for ps in &player_starts {
            println!("  Exported PlayerStart '{}' at ({:.1}, {:.1}, {:.1})", ps.name, ps.location.x, ps.location.y, ps.location.z);
            assert!(ps.location.x.abs() >= 2500.0, "PlayerStart {:?} must be in fortress keep", ps.location);
            assert!(ps.location.z < 100.0, "PlayerStart {:?} must be on ground level", ps.location);
        }

        // Verify Redeemer
        let redeemer = reloaded.actors.iter().find(|a| a.class_name.contains("WarHeadLauncher") || a.name.contains("Redeemer"));
        assert!(redeemer.is_some(), "Expected Redeemer WarHeadLauncher in map");
        let r = redeemer.unwrap();
        assert_eq!(r.location.x, 0.0, "Redeemer must be centered on X=0");
        assert_eq!(r.location.y, 0.0, "Redeemer must be centered on Y=0");
    }
}

#[test]
fn test_wizard_all_40_predefined_layouts_and_random_names_zero_errors() {
    let mut state = EditorState::default();
    assert_eq!(WizardArchetype::all().len(), 40);

    let mut seen_names = std::collections::HashSet::new();

    for (idx, &arch) in WizardArchetype::all().iter().enumerate() {
        let mode = WizardGameMode::all()[idx % WizardGameMode::all().len()];
        state.map_wizard_config = MapWizardConfig {
            game_mode: mode,
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: arch,
            room_shape: WizardRoomShape::Rectangular,
            scale: WizardMapScale::Standard,
            toggles: WizardFeatureToggles::full_playtest_ready(),
            auto_run_build_and_audit: true,
            ..Default::default()
        };
        state.map_wizard_config.randomize_map_identity();
        seen_names.insert(state.map_wizard_config.map_name.clone());

        let (b_cnt, _m_cnt, a_cnt) = state.generate_map_from_wizard();
        assert!(
            b_cnt >= 4 && a_cnt >= 10,
            "Archetype {:?} produced too few brushes ({b_cnt}) or actors ({a_cnt})",
            arch
        );

        let errors: Vec<_> = state
            .map_audit_issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "Archetype {:?} (mode {:?}) produced LevelDoctor errors: {:?}",
            arch,
            mode,
            errors
        );
    }

    assert!(
        seen_names.len() >= 16,
        "Expected random names across 18 archetypes to be mostly unique, got {}",
        seen_names.len()
    );
}

#[test]
fn test_wizard_one_click_full_randomizer_zero_errors() {
    let mut state = EditorState::default();
    let mut generated_names = std::collections::HashSet::new();

    for iter in 0..24 {
        state.map_wizard_lock_game_mode = iter >= 18;
        if state.map_wizard_lock_game_mode {
            state.map_wizard_config.game_mode = WizardGameMode::CaptureTheFlag;
        }

        state.randomize_all_wizard_settings();
        if state.map_wizard_lock_game_mode {
            assert_eq!(
                state.map_wizard_config.game_mode,
                WizardGameMode::CaptureTheFlag
            );
        }
        generated_names.insert(state.map_wizard_config.map_name.clone());

        let (b_cnt, _m_cnt, a_cnt) = state.generate_map_from_wizard();
        assert!(
            b_cnt >= 3 && a_cnt >= 10,
            "Randomized map #{} ({}) produced too few brushes ({b_cnt}) or actors ({a_cnt})",
            iter,
            state.map_wizard_config.map_name
        );

        let errors: Vec<_> = state
            .map_audit_issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "One-click randomized map #{} ({}, mode={:?}, strategy={:?}, arch={:?}) produced LevelDoctor errors: {:?}
config: {}",
            iter,
            state.map_wizard_config.map_name,
            state.map_wizard_config.game_mode,
            state.map_wizard_config.layout_strategy,
            state.map_wizard_config.archetype,
            errors,
            serde_json::to_string(&state.map_wizard_config).unwrap_or_default()
        );
    }

    assert!(
        generated_names.len() >= 22,
        "Expected >= 22 unique map names across 24 one-click random rolls, got {}",
        generated_names.len()
    );
}

#[test]
fn test_wizard_assault_mission_chain_and_gating_zero_errors() {
    let mut state = EditorState::default();
    let archetypes = [
        WizardArchetype::GatehouseFuseAssault,
        WizardArchetype::LinearInfiltrationVessel,
        WizardArchetype::CastleKeepWinch,
        WizardArchetype::FortressSiegeLine,
        WizardArchetype::CanyonSerpentineRun,
    ];

    for arch in archetypes {
        for obj_count in [2, 3, 4, 5] {
            state.map_wizard_config = MapWizardConfig {
                game_mode: WizardGameMode::Assault,
                layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
                archetype: arch,
                assault_objective_count: obj_count,
                toggles: WizardFeatureToggles::full_playtest_ready(),
                auto_run_build_and_audit: true,
                ..Default::default()
            };
            state.map_wizard_config.sync_mode_conventions();
            let (b_cnt, m_cnt, a_cnt) = state.generate_map_from_wizard();
            assert!(b_cnt >= 4 && a_cnt >= 12);
            assert!(m_cnt >= 1, "Assault map should have movers (gates/switches)");

            let errors: Vec<_> = state
                .map_audit_issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Error)
                .collect();
            assert!(
                errors.is_empty(),
                "Assault map {:?} (obj_count={}) produced LevelDoctor errors: {:?}",
                arch,
                obj_count,
                errors
            );

            let lvl = state.loaded_unr.as_ref().unwrap();
            let forts: Vec<_> = lvl
                .actors
                .iter()
                .filter(|a| a.class_name == "FortStandard")
                .collect();
            assert!(forts.len() >= 2);
            let final_forts: Vec<_> = forts
                .iter()
                .filter(|f| {
                    f.properties
                        .get("bFinalFort")
                        .map(|p| matches!(p.value, ut99_format::TaggedValue::Bool(true)))
                        .unwrap_or(false)
                })
                .collect();
            assert_eq!(
                final_forts.len(),
                1,
                "Assault map must have exactly 1 final fort"
            );
        }
    }
}

#[test]
fn test_wizard_domination_triangle_and_zero_errors() {
    let mut state = EditorState::default();

    let dom_archetypes = [
        WizardArchetype::TriadSpireAtrium,
        WizardArchetype::VerticalTierCinder,
        WizardArchetype::GearsIndustrialFurnace,
    ];

    for arch in dom_archetypes {
        state.map_wizard_config = MapWizardConfig {
            game_mode: WizardGameMode::Domination,
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: arch,
            theme: WizardThemePreset::Deck16Industrial,
            scale: WizardMapScale::Standard,
            domination_point_count: 3,
            domination_naming_style: ut99_bsp::DominationNamingStyle::ElevationVerticality,
            domination_dais_style: ut99_bsp::DominationDaisStyle::SteppedOctagon,
            domination_include_defense_points: true,
            domination_contested_center_powerup: true,
            toggles: WizardFeatureToggles::full_playtest_ready(),
            auto_run_build_and_audit: true,
            ..Default::default()
        };

        state.generate_map_from_wizard();

        let errors: Vec<_> = state
            .map_audit_issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        assert_eq!(
            errors.len(),
            0,
            "Domination map archetype {:?} produced Level Doctor errors: {:?}",
            arch,
            errors
        );

        let lvl = state.loaded_unr.as_ref().expect("Level must be loaded");
        let cps: Vec<_> = lvl
            .actors
            .iter()
            .filter(|a| a.class_name.eq_ignore_ascii_case("ControlPoint"))
            .collect();
        assert_eq!(cps.len(), 3, "Archetype {:?} must have 3 ControlPoints", arch);

        // Verify ControlPoint PointName and unique positions
        let mut cp_pos = Vec::new();
        for cp in &cps {
            let pt_name = cp.properties.get("PointName").and_then(|p| match &p.value {
                ut99_format::TaggedValue::Str(s) => Some(s.clone()),
                _ => None,
            });
            assert!(pt_name.is_some(), "ControlPoint must have PointName assigned");
            assert!(!pt_name.unwrap().trim().is_empty());
            cp_pos.push(cp.location);
        }

        // Verify minimum standoff distance from PlayerStarts >= 480 UU
        let spawns: Vec<_> = lvl
            .actors
            .iter()
            .filter(|a| a.class_name.eq_ignore_ascii_case("PlayerStart"))
            .collect();
        assert!(spawns.len() >= 4, "Must have sufficient PlayerStarts");
        for sp in &spawns {
            for cp_p in &cp_pos {
                let dist = (sp.location - *cp_p).length();
                assert!(
                    dist >= 450.0,
                    "PlayerStart at {:?} is too close to ControlPoint at {:?} (dist = {:.1} < 450 UU)",
                    sp.location,
                    cp_p,
                    dist
                );
            }
        }

        // Verify DefensePoints exist
        let def_points: Vec<_> = lvl
            .actors
            .iter()
            .filter(|a| a.class_name.eq_ignore_ascii_case("DefensePoint"))
            .collect();
        assert!(
            def_points.len() >= 3,
            "Archetype {:?} must contain at least 1 DefensePoint per ControlPoint",
            arch
        );

        // Verify Contested Center ShieldBelt powerup exists
        let shield_belts: Vec<_> = lvl
            .actors
            .iter()
            .filter(|a| a.class_name.eq_ignore_ascii_case("UT_ShieldBelt"))
            .collect();
        assert!(
            !shield_belts.is_empty(),
            "Domination map must generate contested central UT_ShieldBelt"
        );
    }
}

#[test]
fn test_wizard_ctf_symmetry_routes_and_zero_errors() {
    let mut state = EditorState::default();
    let ctf_archetypes = [
        WizardArchetype::FacingWorldsOrbitalSpire,
        WizardArchetype::HydroSubmarineDock,
        WizardArchetype::CanyonCrestCitadels,
        WizardArchetype::SymmetricalTwinBases,
        WizardArchetype::ParallelTrenchWarfare,
        WizardArchetype::TwinSpireBridges,
        WizardArchetype::HourglassNexusCTF,
    ];

    for arch in ctf_archetypes {
        state.map_wizard_config = MapWizardConfig {
            game_mode: WizardGameMode::CaptureTheFlag,
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: arch,
            room_shape: WizardRoomShape::Rectangular,
            scale: WizardMapScale::Standard,
            toggles: WizardFeatureToggles::full_playtest_ready(),
            auto_run_build_and_audit: true,
            ctf_dais_style: ut99_bsp::CtfDaisStyle::SteppedTeamPedestal,
            ctf_include_sniper_perches: true,
            ctf_contested_center_powerup: true,
            ..Default::default()
        };
        state.map_wizard_config.enforce_mode_invariants();

        let (b_cnt, _m_cnt, a_cnt) = state.generate_map_from_wizard();
        assert!(b_cnt >= 4, "CTF {:?} must generate CSG brushes, got {}", arch, b_cnt);
        assert!(a_cnt >= 12, "CTF {:?} must generate actors, got {}", arch, a_cnt);

        // 1. LevelDoctor Error Audit must be 100% clean
        let errors: Vec<_> = state
            .map_audit_issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "CTF {:?} produced LevelDoctor errors: {:?}",
            arch,
            errors
        );

        let lvl = state.loaded_unr.as_ref().expect("CTF map must populate loaded_unr");

        // 2. Exactly 2 FlagBases (Red team 0, Blue team 1)
        let flags: Vec<_> = lvl
            .actors
            .iter()
            .filter(|a| a.class_name.eq_ignore_ascii_case("FlagBase"))
            .collect();
        assert_eq!(flags.len(), 2, "CTF {:?} must have exactly 2 FlagBases", arch);

        let red_flag = flags.iter().find(|f| {
            f.properties.get("Team").and_then(|p| match p.value {
                ut99_format::TaggedValue::Byte(b) => Some(b),
                ut99_format::TaggedValue::Int(i) => Some(i as u8),
                _ => None,
            }) == Some(0)
        }).expect("Missing Red FlagBase (Team 0)");

        let blue_flag = flags.iter().find(|f| {
            f.properties.get("Team").and_then(|p| match p.value {
                ut99_format::TaggedValue::Byte(b) => Some(b),
                ut99_format::TaggedValue::Int(i) => Some(i as u8),
                _ => None,
            }) == Some(1)
        }).expect("Missing Blue FlagBase (Team 1)");

        let flag_dist = (red_flag.location - blue_flag.location).length();
        assert!(
            flag_dist >= 2200.0,
            "CTF {:?} flag distance too short: {:.1} UU (must be >= 2200 UU)",
            arch,
            flag_dist
        );

        // 3. PlayerStarts: symmetrical and safe standoff
        let spawns: Vec<_> = lvl
            .actors
            .iter()
            .filter(|a| a.class_name.eq_ignore_ascii_case("PlayerStart"))
            .collect();

        let mut red_spawns = 0;
        let mut blue_spawns = 0;
        for s in &spawns {
            let team = s.properties.get("TeamNumber").and_then(|p| match p.value {
                ut99_format::TaggedValue::Byte(b) => Some(b),
                ut99_format::TaggedValue::Int(i) => Some(i as u8),
                _ => None,
            }).unwrap_or(255);
            if team == 0 {
                red_spawns += 1;
                let d_own = (s.location - red_flag.location).length();
                assert!(
                    d_own >= 350.0,
                    "Red spawn too close to own flag: {:.1} UU in {:?}",
                    d_own,
                    arch
                );
            } else if team == 1 {
                blue_spawns += 1;
                let d_own = (s.location - blue_flag.location).length();
                assert!(
                    d_own >= 350.0,
                    "Blue spawn too close to own flag: {:.1} UU in {:?}",
                    d_own,
                    arch
                );
            }
        }

        assert!(red_spawns >= 4, "CTF {:?} must have >= 4 Red spawns, got {}", arch, red_spawns);
        assert!(blue_spawns >= 4, "CTF {:?} must have >= 4 Blue spawns, got {}", arch, blue_spawns);
        assert_eq!(red_spawns, blue_spawns, "CTF {:?} spawns asymmetric: {} Red vs {} Blue", arch, red_spawns, blue_spawns);

        // 4. DefensePoints for both teams
        let def_points: Vec<_> = lvl
            .actors
            .iter()
            .filter(|a| a.class_name.eq_ignore_ascii_case("DefensePoint"))
            .collect();
        assert!(
            def_points.iter().any(|d| {
                d.properties.get("team").or_else(|| d.properties.get("Team")).and_then(|p| match p.value {
                    ut99_format::TaggedValue::Byte(b) => Some(b),
                    ut99_format::TaggedValue::Int(i) => Some(i as u8),
                    _ => None,
                }) == Some(0)
            }),
            "Missing Red DefensePoints in {:?}",
            arch
        );
        assert!(
            def_points.iter().any(|d| {
                d.properties.get("team").or_else(|| d.properties.get("Team")).and_then(|p| match p.value {
                    ut99_format::TaggedValue::Byte(b) => Some(b),
                    ut99_format::TaggedValue::Int(i) => Some(i as u8),
                    _ => None,
                }) == Some(1)
            }),
            "Missing Blue DefensePoints in {:?}",
            arch
        );

        // 5. AlternatePaths for both teams
        let alt_paths: Vec<_> = lvl
            .actors
            .iter()
            .filter(|a| a.class_name.eq_ignore_ascii_case("AlternatePath"))
            .collect();
        assert!(!alt_paths.is_empty(), "CTF {:?} must generate AlternatePaths", arch);
        assert!(
            alt_paths.iter().any(|p| {
                p.properties.get("team").or_else(|| p.properties.get("Team")).and_then(|pr| match pr.value {
                    ut99_format::TaggedValue::Byte(b) => Some(b),
                    ut99_format::TaggedValue::Int(i) => Some(i as u8),
                    _ => None,
                }) == Some(0)
            }),
            "Missing Red AlternatePath in {:?}",
            arch
        );
        assert!(
            alt_paths.iter().any(|p| {
                p.properties.get("team").or_else(|| p.properties.get("Team")).and_then(|pr| match pr.value {
                    ut99_format::TaggedValue::Byte(b) => Some(b),
                    ut99_format::TaggedValue::Int(i) => Some(i as u8),
                    _ => None,
                }) == Some(1)
            }),
            "Missing Blue AlternatePath in {:?}",
            arch
        );

        // 6. Sniper Rifles on perches
        let snipers: Vec<_> = lvl
            .actors
            .iter()
            .filter(|a| a.class_name.eq_ignore_ascii_case("SniperRifle"))
            .collect();
        assert!(snipers.len() >= 2, "CTF {:?} must have >= 2 SniperRifles", arch);
    }
}


#[test]
fn test_wizard_default_author_and_texture_preview_resolution() {
    let mut state = EditorState::default();

    // 1. Verify default author in MapWizardConfig
    assert_eq!(
        state.map_wizard_config.author,
        "UT Modern Map Editor [by VoT]",
        "Wizard default author must be 'UT Modern Map Editor [by VoT]'"
    );

    // 2. Verify identity randomization preserves author
    state.map_wizard_config.randomize_step1_identity(false);
    assert_eq!(
        state.map_wizard_config.author,
        "UT Modern Map Editor [by VoT]",
        "randomize_step1_identity must preserve wizard author"
    );

    // 3. Verify generating a map from wizard propagates the author to state.level_author
    state.map_wizard_config.auto_run_build_and_audit = false;
    state.generate_map_from_wizard();
    assert_eq!(
        state.level_author,
        "UT Modern Map Editor [by VoT]",
        "generate_map_from_wizard must set state.level_author to the wizard author"
    );

    // 4. Verify TextureManager resolves wall textures across all 15 curated theme presets
    let themes = WizardThemePreset::all();
    assert_eq!(themes.len(), 15);
    for &th in themes {
        let pal = th.palette();
        assert!(!pal.wall_texture.is_empty(), "Theme {:?} must define a wall texture", th);
        assert!(!pal.floor_texture.is_empty(), "Theme {:?} must define a floor texture", th);

        // Check dimensions query
        let dims = state.texture_manager.get_texture_dimensions_by_ref(&pal.wall_texture);
        println!("Theme {:?} wall texture '{}' dimensions: {:?}", th, pal.wall_texture, dims);
        if let Some((w, h)) = dims {
            assert!(w > 0 && h > 0, "Texture dimensions must be positive");
        }
    }
}

#[test]
fn test_wizard_minimap_room_transform_and_object_manipulation_and_stress_randomize() {
    let mut state = EditorState::default();

    // 1. Stress test: 100 consecutive rapid theme randomizations & official DNA extractions
    for i in 0..100 {
        state.map_wizard_config.randomize_step3_theme();
        let pal = state.map_wizard_config.active_palette();
        assert!(!pal.wall_texture.is_empty());
        assert!(!pal.floor_texture.is_empty());

        // Exercise texture manager queries under rapid churn
        let _ = state.texture_manager.get_texture_dimensions_by_ref(&pal.wall_texture);
        let _ = state.texture_manager.get_texture_dimensions_by_ref(&pal.floor_texture);
        let _ = state.texture_manager.get_texture_dimensions_by_ref(&pal.secondary_wall_texture);

        // Also alternate cloning official DNA if maps exist
        if i % 10 == 0 && !state.available_official_maps.is_empty() {
            let map_name = state.available_official_maps[i % state.available_official_maps.len()].clone();
            let _ = state.clone_dna_from_official_map(&map_name);
        }
    }

    // 2. Minimap Room Transformation (Resizing and Moving via Gizmo Controls)
    state.map_wizard_config = MapWizardConfig {
        game_mode: WizardGameMode::DeathMatch,
        archetype: WizardArchetype::CentralArenaMultiTier,
        auto_run_build_and_audit: true,
        ..Default::default()
    };
    state.map_wizard_config.sync_mode_conventions();

    let initial_preview = ut99_bsp::preview_wizard_topology(&state.map_wizard_config);
    assert!(!initial_preview.rooms.is_empty());
    let target_room = &initial_preview.rooms[0];
    let room_name = target_room.label.clone();
    let original_center = target_room.center;
    let original_size = target_room.size;

    let new_center = original_center + glam::Vec3::new(128.0, -64.0, 0.0);
    let new_size = original_size + glam::Vec3::new(256.0, 512.0, 128.0);

    // Apply room transformation
    state.update_wizard_room_transform(&room_name, new_center, new_size);

    // Verify preview reflects the transformed archetype room
    let transformed_preview = ut99_bsp::preview_wizard_topology(&state.map_wizard_config);
    let transformed_room = transformed_preview.rooms.iter().find(|r| r.label == room_name).unwrap();
    assert_eq!(transformed_room.center, new_center);
    assert_eq!(transformed_room.size, new_size);

    // 3. Object Creation, Movement, and Deletion via 2D Minimap
    // Add custom ShockRifle
    let shock_label = state.add_wizard_custom_marker("ShockRifle", 256.0, 128.0);
    assert_eq!(state.map_wizard_config.custom_markers.len(), 1);

    // Add custom Health pickup
    let health_label = state.add_wizard_custom_marker("Health", -256.0, 128.0);
    assert_eq!(state.map_wizard_config.custom_markers.len(), 2);

    // Move ShockRifle to new location
    let new_shock_loc = glam::Vec3::new(320.0, 192.0, 64.0);
    state.move_wizard_marker(&shock_label, new_shock_loc);

    // Verify move in preview
    let marker_preview = ut99_bsp::preview_wizard_topology(&state.map_wizard_config);
    let shock_m = marker_preview.markers.iter().find(|m| m.label == shock_label).unwrap();
    assert_eq!(shock_m.location, new_shock_loc);

    // Delete custom Health pickup
    state.wizard_minimap_selected = Some(ut99_editor::editor_state::WizardMinimapSelection::Marker(health_label.clone()));
    state.delete_wizard_selection();
    assert_eq!(state.map_wizard_config.custom_markers.len(), 1);
    assert!(!state.map_wizard_config.custom_markers.iter().any(|m| m.label == health_label));

    // Suppress an archetype marker
    if let Some(arch_m) = initial_preview.markers.first() {
        let arch_label = arch_m.label.clone();
        state.wizard_minimap_selected = Some(ut99_editor::editor_state::WizardMinimapSelection::Marker(arch_label.clone()));
        state.delete_wizard_selection();
        assert!(state.map_wizard_config.suppressed_markers.contains(&arch_label));

        let suppressed_preview = ut99_bsp::preview_wizard_topology(&state.map_wizard_config);
        assert!(!suppressed_preview.markers.iter().any(|m| m.label == arch_label));
    }

    // 4. Generate map with transformations and custom objects, verify zero errors
    let (brushes, movers, actors) = state.generate_map_from_wizard();
    assert!(brushes >= 4 && actors >= 8);
    println!("Generated transformed map: {} brushes, {} movers, {} actors", brushes, movers, actors);

    let errors: Vec<_> = state
        .map_audit_issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Transformed map produced LevelDoctor errors: {:?}",
        errors
    );

    // Verify custom ShockRifle actor was materialized into the map
    let lvl = state.loaded_unr.as_ref().unwrap();
    let shock_actor = lvl
        .actors
        .iter()
        .find(|a| a.tag == shock_label || a.name == shock_label || (a.group == "MinimapCustom" && a.class_name.contains("ShockRifle")))
        .expect("Custom placed ShockRifle actor must exist in materialized level");
    assert!((shock_actor.location.x - new_shock_loc.x).abs() < 1.0);
    assert!((shock_actor.location.y - new_shock_loc.y).abs() < 1.0);
}

#[test]
fn test_wizard_preset_json_export_import_roundtrip() {
    let mut state = EditorState::default();
    state.map_wizard_config.map_name = "DM-JSONTest".to_string();
    state.map_wizard_config.title = "JSON Test Arena".to_string();
    state.map_wizard_config.music_track = "Foregone.Foregone".to_string();
    state.map_wizard_config.distance_fog = ut99_bsp::WizardFogPreset::CryptMurk;
    state.map_wizard_config.procedural_loop_ratio = 0.0;

    // Add custom room with custom environment
    let cr_name = state.add_wizard_custom_room(512.0, 512.0);
    state.set_wizard_room_environment(&cr_name, ut99_bsp::WizardRoomEnvironment::Water);

    // Add custom marker
    let cm_label = state.add_wizard_custom_marker("FlakCannon", 256.0, 256.0);

    // Add custom connection with blast door
    let conn_id = state.add_wizard_custom_connection(&cr_name, "Main Hall").expect("Connection created");
    state.update_wizard_custom_connection(&conn_id, 384.0, true);

    // Export preset JSON
    let json = state.export_wizard_preset_json().expect("Export JSON must succeed");
    let original_config = state.map_wizard_config.clone();
    assert!(json.contains("DM-JSONTest"));
    assert!(json.contains("Foregone.Foregone"));
    assert!(json.contains("CryptMurk"));
    assert!(json.contains("FlakCannon"));
    assert!(json.contains("has_blast_door"));

    // Create fresh state and import
    let mut state2 = EditorState::default();
    state2.import_wizard_preset_json(&json).expect("Import JSON must succeed");

    assert_eq!(state2.map_wizard_config.map_name, "DM-JSONTest");
    assert_eq!(state2.map_wizard_config.title, "JSON Test Arena");
    assert_eq!(state2.map_wizard_config.music_track, "Foregone.Foregone");
    assert_eq!(state2.map_wizard_config.distance_fog, ut99_bsp::WizardFogPreset::CryptMurk);
    assert_eq!(state2.map_wizard_config.custom_rooms.len(), 1);
    assert_eq!(state2.map_wizard_config.custom_rooms[0].environment, ut99_bsp::WizardRoomEnvironment::Water);
    assert_eq!(state2.map_wizard_config.custom_markers.len(), 1);
    assert_eq!(state2.map_wizard_config.custom_markers[0].label, cm_label);
    assert_eq!(state2.map_wizard_config.custom_connections.len(), 1);
    assert!(state2.map_wizard_config.custom_connections[0].has_blast_door);
    assert!(state2.map_wizard_config == original_config, "valid preset fields changed during import (loop ratio: {} -> {})", original_config.procedural_loop_ratio, state2.map_wizard_config.procedural_loop_ratio);

    for ratio in [0.0, 0.1, 0.199, 0.2, 0.35, 1.0] {
        let mut config = original_config.clone();
        config.procedural_loop_ratio = ratio;
        state2.import_wizard_preset_json(&serde_json::to_string(&config).unwrap()).unwrap();
        assert!(state2.map_wizard_config == config, "valid ratio {ratio} changed during import");
    }

    let mut out_of_range = original_config.clone();
    out_of_range.procedural_loop_ratio = 2.0;
    state2.import_wizard_preset_json(&serde_json::to_string(&out_of_range).unwrap()).unwrap();
    assert_eq!(state2.map_wizard_config.procedural_loop_ratio, 1.0);
    assert!(state2.console_messages.iter().any(|message| message.contains("adjusted settings") && message.contains("procedural_loop_ratio")));
    let before_invalid_json = state2.map_wizard_config.clone();
    assert!(state2.import_wizard_preset_json("{invalid json").is_err());
    assert!(state2.map_wizard_config == before_invalid_json, "invalid JSON must leave the current configuration intact");
}

#[test]
fn test_wizard_room_environments_distance_fog_and_blast_doors() {
    let mut state = EditorState::default();
    state.map_wizard_config.game_mode = WizardGameMode::DeathMatch;
    state.map_wizard_config.archetype = WizardArchetype::CentralArenaMultiTier;
    state.map_wizard_config.music_track = "Botmca9.Botmca9".to_string();
    state.map_wizard_config.distance_fog = ut99_bsp::WizardFogPreset::LavaGlow;
    state.map_wizard_config.auto_run_build_and_audit = true;

    // Set an archetype room to LowGravity
    let preview = ut99_bsp::preview_wizard_topology(&state.map_wizard_config);
    if let Some(r) = preview.rooms.first() {
        state.set_wizard_room_environment(&r.label, ut99_bsp::WizardRoomEnvironment::LowGravity);
    }

    // Add custom water room
    let cr_name = state.add_wizard_custom_room(1024.0, 0.0);
    state.set_wizard_room_environment(&cr_name, ut99_bsp::WizardRoomEnvironment::Water);

    // Add corridor with sliding blast door mover
    let conn_id = state.add_wizard_custom_connection(&preview.rooms[0].label, &cr_name).expect("Connection created");
    state.update_wizard_custom_connection(&conn_id, 320.0, true);

    let (brushes, movers, actors) = state.generate_map_from_wizard();
    assert!(brushes >= 4 && actors >= 8);
    assert!(movers >= 1, "Must generate sliding blast door mover");

    let lvl = state.loaded_unr.as_ref().unwrap();

    // Verify Level song was synchronized to state.level_song
    assert_eq!(state.level_song, "Botmca9.Botmca9");

    // Verify ZoneInfo actors have distance fog or low gravity/water
    let zone_infos: Vec<_> = lvl.actors.iter().filter(|a| a.class_name == "ZoneInfo").collect();
    assert!(!zone_infos.is_empty(), "ZoneInfo actors must exist for rooms with custom environment or fog");

    // Verify blast door mover actor
    let blast_mover = lvl.actors.iter().find(|a| a.class_name == "Mover" && a.tag.contains("BlastDoor"));
    assert!(blast_mover.is_some(), "Sliding blast door mover actor must exist in level");
}

#[test]
fn test_wizard_duplication_selection() {
    let mut state = EditorState::default();
    state.wizard_minimap_grid_snap = 128.0;

    // Duplicate custom marker
    let m_label = state.add_wizard_custom_marker("RocketLauncher", 128.0, 256.0);
    state.wizard_minimap_selected = Some(ut99_editor::editor_state::WizardMinimapSelection::Marker(m_label.clone()));

    let dup_sel = state.duplicate_wizard_selection().expect("Duplication must succeed");
    assert_eq!(state.map_wizard_config.custom_markers.len(), 2);
    if let ut99_editor::editor_state::WizardMinimapSelection::Marker(ref dup_label) = dup_sel {
        assert_ne!(dup_label, &m_label);
        let dup_m = state.map_wizard_config.custom_markers.iter().find(|m| &m.label == dup_label).unwrap();
        assert_eq!(dup_m.kind, "RocketLauncher");
        assert_eq!(dup_m.location.x, 128.0 + 128.0);
        assert_eq!(dup_m.location.y, 256.0 + 128.0);
    } else {
        panic!("Expected marker selection after duplicating marker");
    }

    // Duplicate custom room
    let r_name = state.add_wizard_custom_room(0.0, 0.0);
    state.wizard_minimap_selected = Some(ut99_editor::editor_state::WizardMinimapSelection::Room(r_name.clone()));

    let dup_r_sel = state.duplicate_wizard_selection().expect("Room duplication must succeed");
    assert_eq!(state.map_wizard_config.custom_rooms.len(), 2);
    if let ut99_editor::editor_state::WizardMinimapSelection::Room(ref dup_r_name) = dup_r_sel {
        assert_ne!(dup_r_name, &r_name);
        let dup_r = state.map_wizard_config.custom_rooms.iter().find(|r| &r.name == dup_r_name).unwrap();
        assert_eq!(dup_r.center.x, 0.0 + 256.0);
        assert_eq!(dup_r.center.y, 0.0 + 256.0);
    } else {
        panic!("Expected room selection after duplicating room");
    }
}

#[test]
fn test_wizard_teleporter_safety_controls_and_auto_fix_integration() {
    let mut state = EditorState::default();
    state.map_wizard_config.game_mode = WizardGameMode::DeathMatch;
    state.map_wizard_config.layout_strategy = WizardLayoutStrategy::CanonicalArchetype;
    state.map_wizard_config.archetype = WizardArchetype::InterconnectedComplex;
    state.map_wizard_config.toggles.include_teleporters = false; // Start without archetype teleporters

    // 1. Initial state: 0 teleporters -> Audit is healthy
    let initial_audit = state.audit_wizard_teleporters();
    assert_eq!(initial_audit.teleporter_count, 0);
    assert!(initial_audit.is_healthy);

    // 2. User adds 1 lone teleporter marker on the 2D minimap
    let tele1_label = state.add_wizard_custom_marker("Teleporter", -384.0, 0.0);
    assert_eq!(state.map_wizard_config.custom_markers.len(), 1);

    // 3. Audit detects LoneTeleporter anomaly
    let lone_audit = state.audit_wizard_teleporters();
    assert_eq!(lone_audit.teleporter_count, 1);
    assert!(!lone_audit.is_healthy);
    assert!(lone_audit.issues.iter().any(|i| i.kind == ut99_bsp::WizardTeleportIssueKind::LoneTeleporter));

    // Topology preview also reflects the audit status
    let prev = ut99_bsp::preview_wizard_topology(&state.map_wizard_config);
    assert!(!prev.teleport_audit.is_healthy);
    assert!(prev.teleport_audit.issues.iter().any(|i| i.kind == ut99_bsp::WizardTeleportIssueKind::LoneTeleporter));

    // 4. Trigger Auto-Fix
    let fixes = state.auto_fix_wizard_teleporters();
    assert!(!fixes.is_empty(), "Auto-fix should take action to resolve the lone teleporter");

    // 5. Verify teleporters are now paired, connected, and healthy
    let repaired_audit = state.audit_wizard_teleporters();
    assert!(repaired_audit.is_healthy, "Repaired teleporter audit must be 100% healthy: {:?}", repaired_audit.issues);
    assert_eq!(repaired_audit.teleporter_count, 2);
    assert_eq!(repaired_audit.valid_links.len(), 2, "Reciprocal links must be established");
    assert!(repaired_audit.valid_links[0].distance >= 96.0);

    // 6. Test manual property update via inspector
    state.update_wizard_teleporter_properties(&tele1_label, "CustomTagA", "CustomTagB");
    let cm1 = state.map_wizard_config.custom_markers.iter().find(|m| m.label == tele1_label).unwrap();
    assert_eq!(cm1.teleporter_tag, "CustomTagA");
    assert_eq!(cm1.teleporter_url, "CustomTagB");

    // 7. Generate full valid map: auto-sanitizer runs and populates real UT99 actors
    let (b_cnt, _m_cnt, a_cnt) = state.generate_map_from_wizard();
    assert!(b_cnt >= 3 && a_cnt >= 10);

    // Validate that the generated level contains actual Teleporter actors with URL and Tag
    let unr = state.loaded_unr.as_ref().expect("Level must be loaded");
    let tele_actors: Vec<_> = unr.actors.iter().filter(|a| a.class_name.contains("Teleporter")).collect();
    assert!(tele_actors.len() >= 2, "Generated map must contain at least 2 teleporter actors");

    // Verify Tag and URL properties on teleporter actors
    let has_valid_urls = tele_actors.iter().all(|t| {
        t.properties.contains_key("URL") || t.object_paths.contains_key("URL")
    });
    assert!(has_valid_urls, "All teleporter actors must have configured URL destinations");

    // Level Doctor audit must pass with 0 errors
    let errors: Vec<_> = state.map_audit_issues.iter().filter(|i| i.severity == ut99_bsp::IssueSeverity::Error).collect();
    assert!(errors.is_empty(), "Generated map with auto-fixed teleporters must have 0 Level Doctor errors: {:?}", errors);
}

#[test]
fn test_wizard_dm_never_generates_domination_artifacts() {
    let mut state = EditorState::default();

    // 1. Verify prefix inference transitions mode properly
    state.map_wizard_config.map_name = "DM-Sanctuary".to_string();
    state.map_wizard_config.infer_mode_from_map_name_prefix();
    assert_eq!(state.map_wizard_config.game_mode, WizardGameMode::DeathMatch);

    // 2. Set an illegal Domination archetype on DeathMatch, then enforce invariants
    state.map_wizard_config.archetype = WizardArchetype::ThreeWayRingArena;
    state.map_wizard_config.enforce_mode_invariants();
    assert_ne!(
        state.map_wizard_config.archetype,
        WizardArchetype::ThreeWayRingArena,
        "DM mode must reject 3-way Domination archetypes"
    );

    // 3. Test multiple procedural seeds in DeathMatch mode
    let test_seeds = [1337u64, 4242u64, 99999u64];
    for seed in test_seeds {
        state.map_wizard_config = MapWizardConfig {
            map_name: format!("DM-Arena-{}", seed),
            game_mode: WizardGameMode::DeathMatch,
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: WizardArchetype::CrucibleDuelPit,
            room_shape: WizardRoomShape::Rectangular,
            scale: WizardMapScale::Standard,
            seed,
            toggles: WizardFeatureToggles::full_playtest_ready(),
            auto_run_build_and_audit: true,
            ..Default::default()
        };
        state.map_wizard_config.sync_mode_conventions();

        let (b_cnt, _m_cnt, a_cnt) = state.generate_map_from_wizard();
        assert!(b_cnt >= 4 && a_cnt >= 8);

        // Verify LevelInfo DefaultGameType is Botpack.DeathMatchPlus
        assert_eq!(
            state.level_game_type, "Botpack.DeathMatchPlus",
            "DM maps must strictly set DefaultGameType to Botpack.DeathMatchPlus"
        );

        // Verify actors in UNR
        let unr = state.loaded_unr.as_ref().expect("Level must be loaded");

        let cp_count = unr.actors.iter().filter(|a| a.class_name.eq_ignore_ascii_case("ControlPoint")).count();
        assert_eq!(
            cp_count, 0,
            "DeathMatch maps must NEVER contain ControlPoint actors, found {}",
            cp_count
        );

        let fb_count = unr.actors.iter().filter(|a| a.class_name.eq_ignore_ascii_case("FlagBase")).count();
        assert_eq!(
            fb_count, 0,
            "DeathMatch maps must NEVER contain FlagBase actors, found {}",
            fb_count
        );

        let fs_count = unr.actors.iter().filter(|a| a.class_name.eq_ignore_ascii_case("FortStandard")).count();
        assert_eq!(
            fs_count, 0,
            "DeathMatch maps must NEVER contain FortStandard actors, found {}",
            fs_count
        );

        // Verify Level Doctor found 0 GameModeMismatch errors
        let mismatch_errors: Vec<_> = state
            .map_audit_issues
            .iter()
            .filter(|i| i.category == "GameModeMismatch")
            .collect();
        assert!(
            mismatch_errors.is_empty(),
            "DM map must have 0 GameModeMismatch issues: {:?}",
            mismatch_errors
        );
    }

    // 4. Test Mode Lock during randomizations
    state.map_wizard_lock_game_mode = true;
    state.map_wizard_config.game_mode = WizardGameMode::DeathMatch;
    state.map_wizard_config.map_name = "DM-LockedOutpost".to_string();

    // Randomize identity & layout multiple times; game_mode must stay DeathMatch
    for r_seed in [101u64, 202u64, 303u64, 404u64] {
        state.map_wizard_config.seed = r_seed;
        // In the editor GUI, when mode is locked, randomize_all preserves game mode
        let locked_mode = state.map_wizard_config.game_mode;
        state.map_wizard_config.randomize_step2_layout();
        state.map_wizard_config.enforce_mode_invariants();
        assert_eq!(state.map_wizard_config.game_mode, locked_mode);
        assert!(!matches!(
            state.map_wizard_config.archetype,
            WizardArchetype::ThreeWayRingArena
                | WizardArchetype::TriadSpireAtrium
                | WizardArchetype::VerticalTierCinder
                | WizardArchetype::GearsIndustrialFurnace
        ));
    }
}

#[test]
fn test_gui_randomize_everything_crash_reproduction() {
    let mut state = EditorState::default();
    println!("Available packages: {}", state.texture_manager.available_packages().len());
    state.show_map_wizard_dialog = true;
    let ctx = egui::Context::default();

    for step in 0..=5 {
        state.map_wizard_step = step;
        for iter in 0..50 {
            state.map_wizard_lock_game_mode = iter % 2 == 0;
            state.randomize_all_wizard_settings();

            let _ = ctx.run(Default::default(), |ctx| {
                ut99_editor::gui::render_ui(ctx, &mut state);
            });
        }
    }
}


#[test]
fn test_wizard_architectural_detail_presets_coffers_portals_and_trims() {
    let default_cfg = MapWizardConfig::default();
    assert_eq!(
        default_cfg.architectural_detail,
        WizardArchitecturalDetail::StandardTournament,
        "Default architectural detail preset must be StandardTournament"
    );

    let mut state = EditorState::default();

    // 1. Clean Preset: minimal brushes, no coffers
    state.map_wizard_config.game_mode = WizardGameMode::DeathMatch;
    state.map_wizard_config.archetype = WizardArchetype::InterconnectedComplex;
    state.map_wizard_config.architectural_detail = WizardArchitecturalDetail::Clean;
    let (clean_b, _, _) = state.generate_map_from_wizard();
    let clean_coffers = state.world_brushes.iter().filter(|b| b.name.contains("CeilingCoffer")).count();
    assert_eq!(clean_coffers, 0, "Clean preset must not have ceiling coffers");

    let clean_errors: Vec<_> = state
        .map_audit_issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert!(clean_errors.is_empty(), "Clean preset LevelDoctor errors: {:?}", clean_errors);

    // 2. StandardTournament Preset (Default): coffers, unlit panels, doorway frames, zone portals, baseboards
    state.map_wizard_config.architectural_detail = WizardArchitecturalDetail::StandardTournament;
    let (std_b, _, _) = state.generate_map_from_wizard();
    assert!(
        std_b > clean_b,
        "StandardTournament ({std_b}) must produce more detail brushes than Clean ({clean_b})"
    );

    let std_coffers = state.world_brushes.iter().filter(|b| b.name.contains("CeilingCoffer")).count();
    assert!(std_coffers >= 1, "StandardTournament must generate ceiling coffers, got {std_coffers}");

    let std_panels = state.world_brushes.iter().filter(|b| b.name.contains("CeilingLightPanel")).count();
    assert!(std_panels >= 1, "StandardTournament must generate unlit ceiling light panels");

    let std_portals = state.world_brushes.iter().filter(|b| b.name.contains("ZonePortal")).count();
    assert!(std_portals >= 2, "StandardTournament must generate zone portal sheets across corridors");

    let std_jambs = state.world_brushes.iter().filter(|b| b.name.contains("Jamb")).count();
    assert!(std_jambs >= 4, "StandardTournament must generate doorway jambs");

    let std_baseboards = state.world_brushes.iter().filter(|b| b.name.contains("Baseboard")).count();
    assert!(std_baseboards >= 4, "StandardTournament must generate baseboard trims");

    let std_errors: Vec<_> = state
        .map_audit_issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert!(std_errors.is_empty(), "StandardTournament preset LevelDoctor errors: {:?}", std_errors);

    // 3. EpicHighDetail Preset: inner coffers, corridor ribs, maximum fidelity
    state.map_wizard_config.architectural_detail = WizardArchitecturalDetail::EpicHighDetail;
    let (epic_b, _, _) = state.generate_map_from_wizard();
    assert!(
        epic_b > std_b,
        "EpicHighDetail ({epic_b}) must produce more detail brushes than StandardTournament ({std_b})"
    );

    let inner_coffers = state.world_brushes.iter().filter(|b| b.name.contains("CeilingInnerCoffer")).count();
    assert!(inner_coffers >= 1, "EpicHighDetail must generate inner ceiling coffers");

    let corridor_ribs = state.world_brushes.iter().filter(|b| b.name.contains("Rib")).count();
    assert!(corridor_ribs >= 2, "EpicHighDetail must generate support ribs along long corridors");

    let epic_errors: Vec<_> = state
        .map_audit_issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert!(epic_errors.is_empty(), "EpicHighDetail preset LevelDoctor errors: {:?}", epic_errors);
}

#[test]
fn test_wizard_phase3_verticality_catwalks_stairs_and_sightlines() {
    let mut state = EditorState::default();

    // 1. Test CentralArenaMultiTier: tall arena should generate catwalk gallery + stairs + vantage pickup
    state.map_wizard_config = MapWizardConfig {
        game_mode: WizardGameMode::DeathMatch,
        layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
        archetype: WizardArchetype::CentralArenaMultiTier,
        architectural_detail: WizardArchitecturalDetail::StandardTournament,
        toggles: WizardFeatureToggles::full_playtest_ready(),
        auto_run_build_and_audit: true,
        ..Default::default()
    };
    state.map_wizard_config.sync_mode_conventions();
    let (b_cnt, _, a_cnt) = state.generate_map_from_wizard();
    assert!(b_cnt >= 20 && a_cnt >= 10);

    let catwalk_decks = state.world_brushes.iter().filter(|b| b.name.contains("CatwalkDeck")).count();
    assert!(catwalk_decks >= 1, "Expected at least 1 catwalk deck in tall arena, got {catwalk_decks}");

    let catwalk_rails = state.world_brushes.iter().filter(|b| b.name.contains("CatwalkRail")).count();
    assert!(catwalk_rails >= 1, "Expected at least 1 catwalk guard rail");

    let catwalk_stairs = state.world_brushes.iter().filter(|b| b.name.contains("CatwalkStairs")).count();
    assert!(catwalk_stairs >= 1, "Expected at least 1 stepped flight of access stairs to catwalk");

    let errors: Vec<_> = state
        .map_audit_issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert!(errors.is_empty(), "LevelDoctor errors with catwalks and stairs: {:?}", errors);

    // 2. Test VerticalTierCinder: multi-tier elevation changes should generate TransitStair + TransitStairCol
    state.map_wizard_config = MapWizardConfig {
        game_mode: WizardGameMode::Domination,
        layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
        archetype: WizardArchetype::VerticalTierCinder,
        architectural_detail: WizardArchitecturalDetail::StandardTournament,
        toggles: WizardFeatureToggles::full_playtest_ready(),
        auto_run_build_and_audit: true,
        ..Default::default()
    };
    state.map_wizard_config.sync_mode_conventions();
    state.generate_map_from_wizard();

    let transit_stairs = state.world_brushes.iter().filter(|b| b.name.contains("TransitStair_")).count();
    let transit_cols = state.world_brushes.iter().filter(|b| b.name.contains("TransitStairCol_")).count();
    assert!(transit_stairs >= 1, "Expected at least 1 transit stair in multi-tier map, got {transit_stairs}");
    assert_eq!(transit_stairs, transit_cols, "Every transit stair flight must have a matching invisible collision hull");

    let cinder_errors: Vec<_> = state
        .map_audit_issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert!(cinder_errors.is_empty(), "LevelDoctor errors on VerticalTierCinder: {:?}", cinder_errors);
}

#[test]
fn test_wizard_phase4_skybox_multilayer_and_physical_fixtures_coronas() {
    let mut state = EditorState::default();

    // 1. StandardTournament Preset with Skybox
    state.map_wizard_config = MapWizardConfig {
        game_mode: WizardGameMode::DeathMatch,
        layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
        archetype: WizardArchetype::CentralArenaMultiTier,
        architectural_detail: WizardArchitecturalDetail::StandardTournament,
        theme: WizardThemePreset::CodexGothicCrypt,
        toggles: WizardFeatureToggles::full_playtest_ready(),
        auto_run_build_and_audit: true,
        ..Default::default()
    };
    state.map_wizard_config.sync_mode_conventions();
    state.generate_map_from_wizard();

    // Skybox chamber & multi-layer elements
    let skybox_chamber = state.world_brushes.iter().filter(|b| b.name == "SkyBoxReferenceChamber").count();
    assert_eq!(skybox_chamber, 1);

    let cloud_sheets = state.world_brushes.iter().filter(|b| b.name.contains("SkyBoxCloudSheet")).count();
    assert!(cloud_sheets >= 1, "Expected at least 1 celestial cloud sheet, got {cloud_sheets}");

    let celestial_body = state.world_brushes.iter().filter(|b| b.name == "SkyBoxCelestialBody").count();
    assert_eq!(celestial_body, 1, "Expected 1 celestial moon/planetoid in skybox");

    // Skybox actors: SkyZoneInfo + primary light + fill light
    let lvl = state.loaded_unr.as_ref().unwrap();
    assert!(lvl.actors.iter().any(|a| a.class_name == "SkyZoneInfo"));
    assert!(lvl.actors.iter().any(|a| a.name == "SkyBoxLight0"));
    assert!(lvl.actors.iter().any(|a| a.name == "SkyBoxFillLight"));

    // Physical light fixture brackets on corner accent lights
    let fixture_brackets = state.world_brushes.iter().filter(|b| b.name.contains("LightFixture_Accent")).count();
    assert!(fixture_brackets >= 2, "Expected corner light fixture brackets, got {fixture_brackets}");

    let errors: Vec<_> = state
        .map_audit_issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert!(errors.is_empty(), "LevelDoctor errors on Phase 4 Standard: {:?}", errors);

    // 2. EpicHighDetail Preset: secondary high-altitude cloud sheet
    state.map_wizard_config.architectural_detail = WizardArchitecturalDetail::EpicHighDetail;
    state.generate_map_from_wizard();

    let epic_cloud_sheets = state.world_brushes.iter().filter(|b| b.name.contains("SkyBoxCloudSheet")).count();
    assert!(epic_cloud_sheets >= 2, "EpicHighDetail must generate dual cloud sheets, got {epic_cloud_sheets}");

    let epic_errors: Vec<_> = state
        .map_audit_issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert!(epic_errors.is_empty(), "LevelDoctor errors on Phase 4 Epic: {:?}", epic_errors);
}

#[test]
fn test_wizard_hazard_basin_water_containment_and_floor_visibility() {
    let mut state = EditorState::default();
    state.apply_map_wizard_quick_preset(3); // Symmetrical CTF Citadel
    state.map_wizard_config.toggles.include_environmental_hazards = true;
    state.map_wizard_config.auto_run_build_and_audit = false;
    state.generate_map_from_wizard();

    // 1. Verify FluidSheet brushes do NOT carry PF_PORTAL
    for b in &state.world_brushes {
        if b.name.contains("FluidSheet") {
            for p in &b.polys {
                assert_eq!(
                    p.flags & ut99_bsp::PF_PORTAL,
                    0,
                    "FluidSheet '{}' must never carry PF_PORTAL flag",
                    b.name
                );
            }
        }
    }

    // 2. Verify no WaterZone actor is placed inside unsealed hazard pits in normal environments
    let hazard_water_zones: Vec<_> = state
        .loaded_unr
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .filter(|a| a.class_name.contains("WaterZone") && a.name.contains("MidfieldNexus"))
        .collect();
    assert!(
        hazard_water_zones.is_empty(),
        "Hazard basins in normal environments must not spawn an unsealed WaterZone: {:?}",
        hazard_water_zones
    );

    // Engine builds write staging files in the UT installation. Only run them
    // against an explicitly provided isolated installation.
    let Some(ut_dir) = std::env::var_os("FORGE99_TEST_UT_DIR").map(std::path::PathBuf::from) else {
        eprintln!("Skipping engine portion: set FORGE99_TEST_UT_DIR to an isolated UT installation");
        return;
    };
    let temp_dir = common::TestDir::new();
    let out = temp_dir.path().join("forge99_test_ctf_water_containment.unr");
    ut99_editor::compiler::CompilerBridge::build_unr_with_engine(&state, &ut_dir, &out)
        .expect("Engine build failed");
    let lvl = ut99_format::UnrLevel::open(&out).expect("Failed to open built map");

    // 4. Verify PlayerStarts are in normal zones, NOT a WaterZone
    for ps in &lvl.player_starts {
        // PlayerStart location should have normal zone
        if let Some(bsp) = &lvl.world_bsp {
            let (is_solid, _zone_idx) = bsp.classify_point(ps.location);
            assert!(!is_solid, "PlayerStart at {:?} is inside solid geometry", ps.location);
            // Verify there is a floor underneath PlayerStart
            let floor_d = bsp.floor_distance(ps.location, 120.0, 1.0);
            assert!(
                floor_d.is_some(),
                "PlayerStart at {:?} must have a floor underneath (not floating)",
                ps.location
            );
        }
    }
}

#[test]
fn test_wizard_pre_export_audit_and_autofix_all_game_modes() {
    let modes = [
        ut99_bsp::WizardGameMode::DeathMatch,
        ut99_bsp::WizardGameMode::CaptureTheFlag,
        ut99_bsp::WizardGameMode::Domination,
        ut99_bsp::WizardGameMode::Assault,
    ];

    for mode in &modes {
        let mut state = EditorState::default();
        state.map_wizard_config.game_mode = *mode;
        state.map_wizard_config.layout_strategy = ut99_bsp::WizardLayoutStrategy::ProceduralSeed;
        state.map_wizard_config.seed = 1337;
        state.map_wizard_config.sync_mode_conventions();

        let (b_cnt, _m_cnt, a_cnt) = state.generate_map_from_wizard();
        assert!(b_cnt >= 3 && a_cnt >= 8, "Mode {:?} must generate valid brushes and actors", mode);

        // Pre-export audit & auto-fix verification
        let audit_res = state.audit_and_autofix_before_export();
        assert!(
            audit_res.is_ok(),
            "Pre-export audit failed for mode {:?}: {:?}",
            mode,
            audit_res.err()
        );

        let error_issues: Vec<_> = state
            .map_audit_issues
            .iter()
            .filter(|i| i.severity == ut99_bsp::IssueSeverity::Error)
            .collect();
        assert!(
            error_issues.is_empty(),
            "Mode {:?} still had Level Doctor errors after pre-export audit: {:?}",
            mode,
            error_issues
        );
    }
}

#[test]
fn test_fresh_ctf_map_zero_errors_and_verified_clean() {
    let mut state = EditorState::default();
    state.map_wizard_config.game_mode = ut99_bsp::WizardGameMode::CaptureTheFlag;
    state.map_wizard_config.layout_strategy = ut99_bsp::WizardLayoutStrategy::ProceduralSeed;
    state.map_wizard_config.seed = 42;
    state.map_wizard_config.auto_run_build_and_audit = true;
    state.map_wizard_config.sync_mode_conventions();
    state.generate_map_from_wizard();

    let error_issues: Vec<_> = state
        .map_audit_issues
        .iter()
        .filter(|i| i.severity == ut99_bsp::IssueSeverity::Error)
        .collect();
    assert!(
        error_issues.is_empty(),
        "Fresh CTF map produced Level Doctor errors: {:?}",
        error_issues
    );

    let audit_res = state.audit_and_autofix_before_export();
    assert!(audit_res.is_ok(), "Pre-export audit must succeed: {:?}", audit_res.err());

    // Verify PlayerStarts have valid floor support
    if let Some(ref unr) = state.loaded_unr {
        assert!(
            unr.player_starts.len() >= 4,
            "Fresh CTF map must have at least 4 PlayerStarts"
        );
        assert!(
            !state.nav_graph.reach_specs.is_empty(),
            "Fresh CTF map must have compiled AI ReachSpecs"
        );
    }
}

#[test]
fn verified_generation_runs_one_audit_and_reports_blocking_errors() {
    for auto_audit in [false, true] {
        let mut state = EditorState::default();
        state.map_path = Some(std::path::PathBuf::from("previous-map.unr"));
        state.ut99_launch_args = "?game=Previous.Game".into();
        state.map_wizard_config.game_mode = WizardGameMode::CaptureTheFlag;
        state.map_wizard_config.layout_strategy = WizardLayoutStrategy::ProceduralSeed;
        state.map_wizard_config.seed = 42;
        state.map_wizard_config.auto_run_build_and_audit = auto_audit;
        state.map_wizard_config.sync_mode_conventions();
        let result = state.generate_wizard_map_verified();
        assert!(result.is_ok(), "verified generation failed: {:?}", result.err());
        assert_eq!(state.history.undo_stack.len(), 1, "generation and its repairs must share one undo action");
        assert_eq!(state.console_messages.iter().filter(|m| m.contains("Commencing pre-export audit")).count(), 1);
        assert_eq!(state.level_audit_freshness(), ut99_editor::editor_state::LevelAuditFreshness::Current);
        assert!(!state.map_audit_issues.iter().any(|issue| issue.category == "InventoryLinks"), "generated inventory references should be coherent");
        let generated_path = state.map_path.clone();
        let generated_args = state.ut99_launch_args.clone();
        state.undo();
        assert_eq!(state.map_path, Some(std::path::PathBuf::from("previous-map.unr")));
        assert_eq!(state.ut99_launch_args, "?game=Previous.Game");
        state.redo();
        assert_eq!(state.map_path, generated_path);
        assert_eq!(state.ut99_launch_args, generated_args);
    }

    let mut state = EditorState::default();
    state.map_wizard_config.auto_run_build_and_audit = false;
    state.map_wizard_config.custom_markers.push(ut99_bsp::WizardCustomMarker {
        id: "isolated_test".into(), label: "IsolatedTestPath".into(), kind: "PathNode".into(),
        class_name: "PathNode".into(), location: glam::Vec3::new(30000.0, 30000.0, 30000.0),
        team: None, teleporter_tag: String::new(), teleporter_url: String::new(),
    });
    let errors = state.generate_wizard_map_verified().expect_err("isolated node must block verified generation");
    assert!(errors.iter().any(|e| e.contains("IsolatedTestPath")), "unexpected errors: {errors:?}");
    assert_eq!(state.console_messages.iter().filter(|m| m.contains("Commencing pre-export audit")).count(), 1);
}

#[test]
fn verified_save_checks_audit_and_stages_native_package() {
    if std::env::var("UT99_NATIVE_SAVE").ok().as_deref() != Some("1") {
        eprintln!("Skipping isolated native save test: set UT99_NATIVE_SAVE=1");
        return;
    }
    let temp_dir = common::TestDir::new();
    let target = temp_dir.path().join("CTF-VerifiedSave.unr");
    let mut state = EditorState::default();
    assert!(state.save_wizard_unr_verified(&target).is_err());
    assert!(!target.exists());

    state.map_wizard_config.game_mode = WizardGameMode::CaptureTheFlag;
    state.map_wizard_config.layout_strategy = WizardLayoutStrategy::ProceduralSeed;
    state.map_wizard_config.seed = 42;
    state.map_wizard_config.map_name = "CTF-VerifiedSave".into();
    state.map_wizard_config.sync_mode_conventions();
    state.generate_wizard_map_verified().expect("audit must pass");
    std::fs::write(&target, b"original map").unwrap();
    let report = state.save_wizard_unr_verified(&target).expect("verified save must pass");
    assert_eq!(report.backend, ut99_editor::compiler::UnrSaveBackend::Native);
    assert!(report.native_reason.is_some());
    assert_eq!(std::fs::read(target.with_extension("unr.bak")).unwrap(), b"original map");
    let loaded = UnrLevel::open(&report.path).expect("saved map must reopen");
    assert!(UnrLevel::validate_package(&loaded.package).is_valid);
    assert!(!std::fs::read_dir(temp_dir.path()).unwrap().any(|entry|
        entry.unwrap().file_name().to_string_lossy().starts_with(".forge99-stage-")));
}

#[test]
fn canyon_team_deathmatch_inventory_markers_are_connected() {
  for hazards in [false, true] {
    let mut state = EditorState::default();
    state.map_wizard_config = MapWizardConfig {
        game_mode: WizardGameMode::TeamDeathMatch,
        layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
        archetype: WizardArchetype::CanyonCrestCitadels,
        room_shape: WizardRoomShape::Rectangular,
        scale: WizardMapScale::Standard,
        toggles: WizardFeatureToggles::full_playtest_ready(),
        auto_run_build_and_audit: false,
        ..Default::default()
    };
    state.map_wizard_config.toggles.include_environmental_hazards = hazards;
    state.generate_map_from_wizard();
    state.build_ai_paths();
    let isolated: Vec<_> = ut99_bsp::LevelDoctor::audit_navigation_graph(&state.nav_graph)
        .into_iter().filter(|i| i.severity == IssueSeverity::Error && i.description.contains("InventorySpot")).collect();
    assert!(isolated.is_empty(), "isolated inventory markers: {isolated:?}");
    assert!(!state.loaded_unr.as_ref().unwrap().actors.iter().any(|a| a.name == "Secret_MegaPowerup"),
        "the deep canyon must not host an inaccessible secret");
  }
}
