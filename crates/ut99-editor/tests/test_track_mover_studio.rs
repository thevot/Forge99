// crates/ut99-editor/tests/test_track_mover_studio.rs
//
// Integration tests for Interactive Spline Rail, Kinetic Track Mover & Rollercoaster Studio.

use ut99_bsp::{sample_track_spline, TrackArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_track_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is IndustrialMinecartRail
    assert_eq!(
        state.track_studio_settings.active_config.archetype,
        TrackArchetype::IndustrialMinecartRail
    );

    // Apply HighSpeedBulletTrain
    state.apply_track_archetype(TrackArchetype::HighSpeedBulletTrain);
    assert_eq!(
        state.track_studio_settings.active_config.archetype,
        TrackArchetype::HighSpeedBulletTrain
    );
    assert_eq!(state.track_studio_settings.active_config.gauge_width, 80.0);

    // Apply SuspendedGondolaCable
    state.apply_track_archetype(TrackArchetype::SuspendedGondolaCable);
    assert_eq!(
        state.track_studio_settings.active_config.archetype,
        TrackArchetype::SuspendedGondolaCable
    );

    // Apply RollercoasterLoopThrill
    state.apply_track_archetype(TrackArchetype::RollercoasterLoopThrill);
    assert_eq!(
        state.track_studio_settings.active_config.archetype,
        TrackArchetype::RollercoasterLoopThrill
    );

    // Apply SubterraneanSubwayMetro
    state.apply_track_archetype(TrackArchetype::SubterraneanSubwayMetro);
    assert_eq!(
        state.track_studio_settings.active_config.archetype,
        TrackArchetype::SubterraneanSubwayMetro
    );

    // Undo returns to RollercoasterLoopThrill
    state.undo();
    assert_eq!(
        state.track_studio_settings.active_config.archetype,
        TrackArchetype::RollercoasterLoopThrill
    );

    // Redo returns to SubterraneanSubwayMetro
    state.redo();
    assert_eq!(
        state.track_studio_settings.active_config.archetype,
        TrackArchetype::SubterraneanSubwayMetro
    );
}

#[test]
fn test_spline_sampling_and_physical_banking() {
    let mut state = EditorState::default();

    state.apply_track_archetype(TrackArchetype::RollercoasterLoopThrill);
    let cfg = &state.track_studio_settings.active_config;

    let sample_0 = sample_track_spline(cfg, 0.0);
    let sample_50 = sample_track_spline(cfg, 0.5);
    let sample_100 = sample_track_spline(cfg, 1.0);

    assert_eq!(sample_0.position, cfg.points[0].position);
    assert!(sample_50.position.length() > 0.0);
    assert_eq!(sample_100.position, cfg.points.last().unwrap().position);

    // Tangent and normal vectors must be unit length
    assert!((sample_50.tangent.length() - 1.0).abs() < 0.05);
    assert!((sample_50.normal.length() - 1.0).abs() < 0.05);

    // Bank angle must stay within configured max bank limit
    assert!(sample_50.bank_angle_deg.abs() <= cfg.max_bank_angle_deg);

    // Test timeline scrubbing
    state.scrub_track_timeline(0.75);
    assert_eq!(state.track_studio_settings.timeline_t, 0.75);
}

#[test]
fn test_commit_track_setup_with_brushes_mover_and_waypoints_undo_redo() {
    let mut state = EditorState::default();

    state.apply_track_archetype(TrackArchetype::HighSpeedBulletTrain);
    state.track_studio_settings.active_config.keyframe_count = 6;
    state.recalculate_track_preview();

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    let (rails_idx, ties_idx, mover_idx, waypoint_indices) = state.commit_track_setup();

    assert_eq!(rails_idx, initial_world_brushes);
    assert_eq!(ties_idx, initial_world_brushes + 1);
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 2);

    assert!(mover_idx.is_some());
    assert_eq!(waypoint_indices.len(), 3, "2 LiftExits + 1 LiftCenter");

    let total_actors = state.loaded_unr.as_ref().unwrap().actors.len();
    assert_eq!(total_actors, initial_actors + 4);

    let mover_actor = &state.loaded_unr.as_ref().unwrap().actors[mover_idx.unwrap()];
    assert_eq!(mover_actor.class_name, "Engine.Mover");
    assert_eq!(mover_actor.group, "TrackMovers");

    // Undo removes all committed brushes and actors
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // Redo restores all committed brushes and actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors + 4);
}

#[test]
fn test_track_kinetic_safety_audit() {
    let mut state = EditorState::default();

    // 1. Audit catches excessive keyframes (>8)
    state.track_studio_settings.active_config.keyframe_count = 12;
    state.recalculate_track_preview();
    let has_crit = state.track_studio_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains(">8"));
    assert!(has_crit, "Audit must flag critical hazard for keyframe count > 8");

    // 2. High speed sharp corner produces lateral G-force warning
    state.apply_track_archetype(TrackArchetype::RollercoasterLoopThrill);
    state.track_studio_settings.active_config.default_speed = 3000.0;
    state.recalculate_track_preview();
    let has_g_warn = state.track_studio_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Lateral"));
    assert!(has_g_warn, "Audit must warn about extreme lateral G-forces");
}
