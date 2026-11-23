use glam::Vec3;
use ut99_bsp::{
    audit_level_music, simulate_crossfade, MusicEventData, MusicIssueSeverity,
    MusicTrackPreset, MusicTransition,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_music_crossfade_simulation_curves() {
    let transitions = [
        MusicTransition::None,
        MusicTransition::Instant,
        MusicTransition::Fade,
        MusicTransition::FastFade,
        MusicTransition::SlowFade,
        MusicTransition::Chorus,
    ];

    for &trans in &transitions {
        let dur = trans.fade_duration_secs();

        // Test at start (elapsed = 0.0)
        let (out_start, in_start) = simulate_crossfade(1.0, 1.0, trans, 0.0);
        assert!(out_start >= 0.0 && out_start <= 1.0);
        assert!(in_start >= 0.0 && in_start <= 1.0);

        // Test at midpoint (elapsed = dur / 2.0)
        let (out_mid, in_mid) = simulate_crossfade(1.0, 1.0, trans, dur * 0.5);
        assert!(out_mid >= 0.0 && out_mid <= 1.0);
        assert!(in_mid >= 0.0 && in_mid <= 1.0);

        // Test at completion (elapsed >= dur)
        let (out_end, in_end) = simulate_crossfade(1.0, 1.0, trans, dur + 1.0);
        assert!(out_end >= 0.0 && out_end <= 1.0);
        assert!(in_end >= 0.0 && in_end <= 1.0);

        // Instant transition checks
        if trans == MusicTransition::Instant {
            let (out_inst, in_inst) = simulate_crossfade(1.0, 1.0, trans, 0.0);
            assert_eq!(out_inst, 0.0);
            assert_eq!(in_inst, 1.0);
        }

        // Fade transition checks
        if trans == MusicTransition::Fade {
            assert!((out_start - 1.0).abs() < 1e-4);
            assert!((in_start - 0.0).abs() < 1e-4);
            assert!((out_end - 0.0).abs() < 1e-4);
            assert!((in_end - 1.0).abs() < 1e-4);
        }
    }
}

#[test]
fn test_music_track_presets_and_tracker_module_info() {
    let presets = MusicTrackPreset::all();
    assert_eq!(presets.len(), 6, "Must provide 6 canonical tracker music presets");

    for &preset in presets {
        assert!(!preset.display_name().is_empty());
        assert!(!preset.description().is_empty());

        let module = preset.to_tracker_info();
        assert!(!module.package.is_empty());
        assert!(!module.title.is_empty());
        assert!(!module.author.is_empty());
        assert!(!module.format.is_empty());
        assert!(module.channels >= 8);
        assert!(module.default_bpm >= 100 && module.default_bpm <= 180);
        assert!(module.sections.len() >= 2);

        for sec in &module.sections {
            assert!(!sec.name.is_empty());
            assert!(!sec.mood.is_empty());
            assert!(sec.bpm >= 100);
        }
    }
}

#[test]
fn test_music_audit_validation() {
    let events = vec![
        MusicEventData {
            index: 0,
            name: "MusicEvent0".to_string(),
            tag: "CombatZoneTrigger".to_string(),
            location: Vec3::new(100.0, 200.0, 0.0),
            song: "Botpack99.umx".to_string(),
            song_section: 1,
            transition: MusicTransition::Fade,
            affect_all_players: true,
            once_only: false,
            silence: false,
        },
        MusicEventData {
            index: 1,
            name: "MusicEvent1".to_string(),
            tag: "BossEntrance".to_string(),
            location: Vec3::new(1000.0, 500.0, 100.0),
            song: "Botpack99.umx".to_string(),
            song_section: 2,
            transition: MusicTransition::SlowFade,
            affect_all_players: false,
            once_only: true,
            silence: false,
        },
    ];

    // Perfect audit case
    let audit = audit_level_music("Botpack99.umx", 0, &events);
    assert_eq!(audit.music_events_count, 2);
    assert!(audit.is_valid);
    assert_eq!(audit.unique_songs_referenced.len(), 1);

    // Audit with unassigned level song (warns about silent map)
    let unassigned_audit = audit_level_music("", 0, &events);
    assert!(unassigned_audit.issues.iter().any(|i| i.severity == MusicIssueSeverity::Warning));

    // Audit with critical issue (MusicEvent has empty song and silence = false)
    let mut bad_events = events.clone();
    bad_events[0].song = "".to_string();
    bad_events[0].silence = false;
    let critical_audit = audit_level_music("Botpack99.umx", 0, &bad_events);
    assert!(!critical_audit.is_valid);
    assert!(critical_audit.issues.iter().any(|i| i.severity == MusicIssueSeverity::Critical));
}

#[test]
fn test_editor_music_studio_placement_and_undo_redo() {
    let mut state = EditorState::default();
    state.builder_brush.location = Vec3::new(256.0, 512.0, 128.0);

    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    // 1. Place MusicEvent at Pivot
    let actor_idx = state.place_music_event_at_pivot(
        "Run.umx",
        1,
        MusicTransition::FastFade,
    );

    let unr = state.loaded_unr.as_ref().expect("Level must exist");
    assert_eq!(unr.actors.len(), initial_actors + 1);
    let placed = &unr.actors[actor_idx];
    assert_eq!(placed.class_name, "MusicEvent");
    assert_eq!(placed.location, Vec3::new(256.0, 512.0, 128.0));

    // Test Undo placement
    assert!(state.history.can_undo());
    state.undo();
    let unr_after_undo = state.loaded_unr.as_ref().expect("Level must exist");
    assert_eq!(unr_after_undo.actors.len(), initial_actors);

    // Test Redo placement
    assert!(state.history.can_redo());
    state.redo();
    let unr_after_redo = state.loaded_unr.as_ref().expect("Level must exist");
    assert_eq!(unr_after_redo.actors.len(), initial_actors + 1);

    // 2. Test apply_level_music_track with Undo/Redo
    let prev_song = state.level_song.clone();
    state.apply_level_music_track("Colossus.umx", 2);
    assert_eq!(state.level_song, "Colossus.umx");
    assert_eq!(state.level_song_section, 2);

    assert!(state.history.can_undo());
    state.undo();
    assert_eq!(state.level_song, prev_song);
    assert_eq!(state.level_song_section, 0);

    assert!(state.history.can_redo());
    state.redo();
    assert_eq!(state.level_song, "Colossus.umx");
    assert_eq!(state.level_song_section, 2);

    // 3. Test collect_music_events and audit_music_setup
    let collected = state.collect_music_events();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].song, "Run.umx");
    assert_eq!(collected[0].song_section, 1);
    assert_eq!(collected[0].transition, MusicTransition::FastFade);

    let audit = state.audit_music_setup();
    assert_eq!(audit.music_events_count, 1);
    assert!(state.music_studio_settings.last_audit_report.is_some());
}
