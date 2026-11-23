// crates/ut99-editor/tests/test_laser_grid_studio.rs
//
// Integration tests for Interactive Security Laser Grid, Tripwire Alarm & Energy Barrier Studio.

use ut99_bsp::{LaserArchetype, LaserLayoutMode};
use ut99_editor::editor_state::EditorState;
use glam::Vec3;

#[test]
fn test_laser_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is SingleInfraredTripwire
    assert_eq!(
        state.laser_grid_settings.active_config.archetype,
        LaserArchetype::SingleInfraredTripwire
    );

    // Apply CrissCrossSecurityMesh
    state.apply_laser_archetype(LaserArchetype::CrissCrossSecurityMesh);
    assert_eq!(
        state.laser_grid_settings.active_config.archetype,
        LaserArchetype::CrissCrossSecurityMesh
    );
    assert_eq!(state.laser_grid_settings.active_config.beam_count, 6);

    // Apply SweepingPatrolBeam
    state.apply_laser_archetype(LaserArchetype::SweepingPatrolBeam);
    assert_eq!(
        state.laser_grid_settings.active_config.archetype,
        LaserArchetype::SweepingPatrolBeam
    );
    assert!(state.laser_grid_settings.active_config.sweep.is_sweeping);

    // Apply PerimeterCircularGate
    state.apply_laser_archetype(LaserArchetype::PerimeterCircularGate);
    assert_eq!(
        state.laser_grid_settings.active_config.archetype,
        LaserArchetype::PerimeterCircularGate
    );

    // Apply LethalInstagibCurtain
    state.apply_laser_archetype(LaserArchetype::LethalInstagibCurtain);
    assert_eq!(
        state.laser_grid_settings.active_config.archetype,
        LaserArchetype::LethalInstagibCurtain
    );
    assert_eq!(state.laser_grid_settings.active_config.trigger.damage_per_second, 1000.0);

    // Undo returns to PerimeterCircularGate
    state.undo();
    assert_eq!(
        state.laser_grid_settings.active_config.archetype,
        LaserArchetype::PerimeterCircularGate
    );

    // Redo returns to LethalInstagibCurtain
    state.redo();
    assert_eq!(
        state.laser_grid_settings.active_config.archetype,
        LaserArchetype::LethalInstagibCurtain
    );
}

#[test]
fn test_laser_sweep_kinematics_and_time_scrubbing() {
    let mut state = EditorState::default();

    state.apply_laser_archetype(LaserArchetype::SweepingPatrolBeam);
    let cfg = state.laser_grid_settings.active_config.clone();

    // Initial state at t=0
    state.scrub_laser_time(0.0);
    let z0 = state.laser_grid_settings.cached_beams[0].start_point.z;

    // Quarter period: peak upward sweep
    let quarter_t = cfg.sweep.sweep_period_sec * 0.25;
    state.scrub_laser_time(quarter_t);
    let z_peak = state.laser_grid_settings.cached_beams[0].start_point.z;

    assert!((z_peak - z0 - cfg.sweep.sweep_amplitude).abs() < 1e-2);

    // Half period: returns to center base
    state.scrub_laser_time(cfg.sweep.sweep_period_sec * 0.5);
    let z_half = state.laser_grid_settings.cached_beams[0].start_point.z;
    assert!((z_half - z0).abs() < 1e-2);
}

#[test]
fn test_commit_laser_grid_to_world_undo_redo() {
    let mut state = EditorState::default();

    state.apply_laser_archetype(LaserArchetype::CrissCrossSecurityMesh);

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    let (beam_idx, diode_idx, trigger_indices) = state.commit_laser_grid_to_world();

    assert_eq!(beam_idx, initial_world_brushes);
    assert_eq!(diode_idx, initial_world_brushes + 1);
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 2);

    // 6 beams -> 12 crossed quads for ribbons
    let beam_brush = &state.world_brushes[beam_idx];
    assert_eq!(beam_brush.polys.len(), 12);
    assert_eq!(beam_brush.polys[0].flags & 0x00400000, 0x00400000); // PF_UNLIT
    assert_eq!(beam_brush.polys[0].flags & 0x00000004, 0x00000004); // PF_TRANSLUCENT

    // 6 beams -> 12 diode boxes * 6 faces = 72 polys
    let diode_brush = &state.world_brushes[diode_idx];
    assert_eq!(diode_brush.polys.len(), 72);
    assert_eq!(diode_brush.polys[0].flags & 0x00200000, 0x00200000); // PF_SEMISOLID

    // Check trigger actors
    assert_eq!(trigger_indices.len(), 6);
    let total_actors = state.loaded_unr.as_ref().unwrap().actors.len();
    assert_eq!(total_actors, initial_actors + 6);

    let trig_actor = &state.loaded_unr.as_ref().unwrap().actors[trigger_indices[0]];
    assert_eq!(trig_actor.class_name, "Engine.Trigger");
    assert_eq!(trig_actor.event, "Alarm_CorridorMeshBreached");

    // Undo removes both brushes and all 6 trigger actors
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // Redo restores both brushes and all 6 trigger actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors + 6);
}

#[test]
fn test_laser_grid_safety_and_clearance_audit() {
    let mut state = EditorState::default();

    // 1. Unbound alarm event triggers critical hazard
    state.laser_grid_settings.active_config.trigger.alarm_event = String::new();
    state.recalculate_laser_preview();
    let has_unbound = state.laser_grid_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Unbound"));
    assert!(has_unbound, "Must flag critical hazard for empty alarm event");

    // 2. Narrow trigger radius triggers warning
    state.laser_grid_settings.active_config.trigger.alarm_event = "Alarm_Vault".to_string();
    state.laser_grid_settings.active_config.trigger.trigger_radius = 8.0;
    state.recalculate_laser_preview();
    let has_narrow = state.laser_grid_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Narrow"));
    assert!(has_narrow, "Must flag warning hazard for narrow trigger radius");

    // 3. Single beam at mid-height (e.g. 50 uu) triggers crawl bypass warning
    state.laser_grid_settings.active_config.layout_mode = LaserLayoutMode::SingleBeam;
    state.laser_grid_settings.active_config.origin = Vec3::new(0.0, 0.0, 50.0);
    state.recalculate_laser_preview();
    let has_crawl = state.laser_grid_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Crawl Bypass"));
    assert!(has_crawl, "Must flag crawl bypass hazard when beam is suspended at 50 uu");
}
