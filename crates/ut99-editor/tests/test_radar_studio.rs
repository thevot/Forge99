// crates/ut99-editor/tests/test_radar_studio.rs
//
// Integration tests for Interactive Team Base Radar & Jammer Studio.

use glam::Vec3;
use ut99_bsp::{RadarScanMode, RadarStudioArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_radar_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is OrbitalCommandHQ
    assert_eq!(
        state.radar_settings.active_config.archetype,
        RadarStudioArchetype::OrbitalCommandHQ
    );
    assert_eq!(
        state.radar_settings.active_config.scan_mode,
        RadarScanMode::OmnidirectionalSweep
    );
    assert_eq!(
        state.radar_settings.active_config.detection_radius_uu,
        3200.0
    );
    assert!(!state.radar_settings.active_config.is_jammer);

    // Apply StealthInfiltrationOutpost
    state.apply_radar_archetype(RadarStudioArchetype::StealthInfiltrationOutpost);
    assert_eq!(
        state.radar_settings.active_config.archetype,
        RadarStudioArchetype::StealthInfiltrationOutpost
    );
    assert!(state.radar_settings.active_config.is_jammer);
    assert_eq!(
        state.radar_settings.active_config.team,
        1 // Blue
    );

    // Apply FortressPerimeterSentinel
    state.apply_radar_archetype(RadarStudioArchetype::FortressPerimeterSentinel);
    assert_eq!(
        state.radar_settings.active_config.archetype,
        RadarStudioArchetype::FortressPerimeterSentinel
    );
    assert_eq!(
        state.radar_settings.active_config.scan_mode,
        RadarScanMode::DirectionalSectorCone
    );

    // Apply SubterraneanBunkerGrid
    state.apply_radar_archetype(RadarStudioArchetype::SubterraneanBunkerGrid);
    assert_eq!(
        state.radar_settings.active_config.archetype,
        RadarStudioArchetype::SubterraneanBunkerGrid
    );

    // Apply MobileAPCReconVehicle
    state.apply_radar_archetype(RadarStudioArchetype::MobileAPCReconVehicle);
    assert_eq!(
        state.radar_settings.active_config.archetype,
        RadarStudioArchetype::MobileAPCReconVehicle
    );

    // Undo back to Subterranean
    state.undo();
    assert_eq!(
        state.radar_settings.active_config.archetype,
        RadarStudioArchetype::SubterraneanBunkerGrid
    );

    // Redo to MobileAPC
    state.redo();
    assert_eq!(
        state.radar_settings.active_config.archetype,
        RadarStudioArchetype::MobileAPCReconVehicle
    );
}

#[test]
fn test_radar_metrics_and_telemetry() {
    let mut state = EditorState::default();

    // Populate level with an existing radar station
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        let mut rad = ut99_format::ActorRecord::default();
        rad.name = "RadarStation_1".to_string();
        rad.class_name = "Botpack.RadarStation".to_string();
        rad.location = Vec3::new(1500.0, 0.0, 0.0);
        level.actors.push(rad);
    }

    state.recalculate_radar_preview();
    let metrics = &state.radar_settings.cached_metrics;

    assert_eq!(metrics.total_nodes_in_level, 2);
    assert_eq!(metrics.distance_to_closest_peer_uu, 1500.0);
    assert_eq!(metrics.threat_detection_latency_secs, 1.0); // 30 / 30 = 1.0s
    assert!(metrics.effective_coverage_area_sq_uu > 10_000_000.0);
}

#[test]
fn test_radar_hazard_auditing() {
    let mut state = EditorState::default();

    // Configure jammer on Red team near a Red radar station
    state.radar_settings.active_config.is_jammer = true;
    state.radar_settings.active_config.team = 0;
    state.radar_settings.active_config.detection_radius_uu = 2000.0;
    state.radar_settings.active_config.position = Vec3::new(0.0, 0.0, 0.0);

    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        let mut rad = ut99_format::ActorRecord::default();
        rad.name = "RedCommandRadar".to_string();
        rad.class_name = "Botpack.RadarStation".to_string();
        rad.location = Vec3::new(800.0, 0.0, 0.0); // Within 2000 uu!
        level.actors.push(rad);
    }

    state.recalculate_radar_preview();
    assert!(state.radar_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Friendly Radar Jamming")));
}

#[test]
fn test_commit_radar_station_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Radar Station
    state.apply_radar_archetype(RadarStudioArchetype::OrbitalCommandHQ);
    let (brush_idx, actor_idx) = state.commit_radar_station_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(actor_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    let act = &level.actors[actor_idx.unwrap()];
    assert_eq!(act.class_name, "Botpack.RadarStation");
    assert_eq!(act.group, "Radar");
    assert_eq!(act.tag, "Red_CommandRadar");

    // 2. Undo Commit
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    let level_undone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_undone.actors.len(), 0);

    // 3. Redo Commit
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    let level_redone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_redone.actors.len(), 1);
    assert_eq!(level_redone.actors[0].class_name, "Botpack.RadarStation");
}
