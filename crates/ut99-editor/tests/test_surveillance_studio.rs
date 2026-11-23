// crates/ut99-editor/tests/test_surveillance_studio.rs
//
// Integration tests for Interactive Security CCTV Camera, Live Video Monitor & Surveillance Network Studio.

use ut99_bsp::{Brush, CsgOperation, Polygon, SurveillanceArchetype};
use ut99_editor::editor_state::EditorState;
use glam::Vec3;

#[test]
fn test_surveillance_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is PerimeterTurretSecurity
    assert_eq!(
        state.surveillance_settings.active_config.archetype,
        SurveillanceArchetype::PerimeterTurretSecurity
    );

    // Apply VaultDomeCCTV
    state.apply_surveillance_archetype(SurveillanceArchetype::VaultDomeCCTV);
    assert_eq!(
        state.surveillance_settings.active_config.archetype,
        SurveillanceArchetype::VaultDomeCCTV
    );
    assert_eq!(state.surveillance_settings.active_config.camera_mount.fov_deg, 95.0);

    // Apply IndustrialFacilityMonitor
    state.apply_surveillance_archetype(SurveillanceArchetype::IndustrialFacilityMonitor);
    assert_eq!(
        state.surveillance_settings.active_config.archetype,
        SurveillanceArchetype::IndustrialFacilityMonitor
    );

    // Apply CorridorCheckpointScanner
    state.apply_surveillance_archetype(SurveillanceArchetype::CorridorCheckpointScanner);
    assert_eq!(
        state.surveillance_settings.active_config.archetype,
        SurveillanceArchetype::CorridorCheckpointScanner
    );

    // Apply TacticalDroneRecon
    state.apply_surveillance_archetype(SurveillanceArchetype::TacticalDroneRecon);
    assert_eq!(
        state.surveillance_settings.active_config.archetype,
        SurveillanceArchetype::TacticalDroneRecon
    );
    assert!(state.surveillance_settings.active_config.patrol.is_continuous_spin);

    // Undo returns to CorridorCheckpointScanner
    state.undo();
    assert_eq!(
        state.surveillance_settings.active_config.archetype,
        SurveillanceArchetype::CorridorCheckpointScanner
    );

    // Redo returns to TacticalDroneRecon
    state.redo();
    assert_eq!(
        state.surveillance_settings.active_config.archetype,
        SurveillanceArchetype::TacticalDroneRecon
    );
}

#[test]
fn test_patrol_kinematics_and_time_scrubbing() {
    let mut state = EditorState::default();

    state.apply_surveillance_archetype(SurveillanceArchetype::PerimeterTurretSecurity);
    let cfg = state.surveillance_settings.active_config.clone();

    // Initial state at t=0
    state.scrub_surveillance_time(0.0);
    let f0 = state.surveillance_settings.cached_frustum.clone().unwrap();

    // Scrub to quarter period (peak pan)
    let quarter_t = cfg.patrol.sweep_period_seconds * 0.25;
    state.scrub_surveillance_time(quarter_t);
    let f_peak = state.surveillance_settings.cached_frustum.clone().unwrap();

    // Direction must have rotated away from base yaw
    assert!(f0.forward.dot(f_peak.forward) < 0.999);

    // Scrub back to full period (returns to base orientation)
    state.scrub_surveillance_time(cfg.patrol.sweep_period_seconds);
    let f_full = state.surveillance_settings.cached_frustum.clone().unwrap();
    assert!((f0.forward.dot(f_full.forward) - 1.0).abs() < 1e-3);
}

#[test]
fn test_commit_surveillance_setup_to_world_undo_redo() {
    let mut state = EditorState::default();

    state.apply_surveillance_archetype(SurveillanceArchetype::VaultDomeCCTV);

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    let (monitor_idx, cam_idx, scr_idx) = state.commit_surveillance_setup_to_world();

    assert_eq!(monitor_idx, initial_world_brushes);
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 1);

    // Check monitor brush has PF_UNLIT front face
    let mon_brush = &state.world_brushes[monitor_idx];
    assert_eq!(mon_brush.polys.len(), 6);
    let has_unlit = mon_brush.polys.iter().any(|p| p.flags & 0x00400000 != 0);
    assert!(has_unlit, "Monitor screen polygon must carry PF_UNLIT flag");

    assert!(cam_idx.is_some());
    assert!(scr_idx.is_some());

    let total_actors = state.loaded_unr.as_ref().unwrap().actors.len();
    assert_eq!(total_actors, initial_actors + 2);

    let cam_actor = &state.loaded_unr.as_ref().unwrap().actors[cam_idx.unwrap()];
    assert_eq!(cam_actor.class_name, "Engine.Camera");
    assert_eq!(cam_actor.tag, "SecCam_VaultDome");

    let scr_actor = &state.loaded_unr.as_ref().unwrap().actors[scr_idx.unwrap()];
    assert_eq!(scr_actor.class_name, "Screen.Screen");
    assert_eq!(scr_actor.event, "Screen_VaultCCTV");

    // Undo removes monitor brush and both actors
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // Redo restores monitor brush and both actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors + 2);
}

#[test]
fn test_surveillance_sightline_occlusion_and_safety_audit() {
    let mut state = EditorState::default();

    // 1. Unbound event tags trigger critical hazard
    state.surveillance_settings.active_config.camera_tag = String::new();
    state.recalculate_surveillance_preview();
    let has_unbound = state.surveillance_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Unbound"));
    assert!(has_unbound, "Must flag critical hazard for unbound camera tag");

    // 2. Camera placed right inside a solid world brush triggers obstruction hazard
    let mut solid_box = Brush::new("WallObstruction", CsgOperation::Add);
    let v0 = Vec3::new(500.0, 500.0, 250.0);
    let v1 = Vec3::new(520.0, 500.0, 250.0);
    let v2 = Vec3::new(520.0, 520.0, 250.0);
    if let Some(poly) = Polygon::new(vec![v0, v1, v2]) {
        solid_box.polys.push(poly);
    }
    state.world_brushes.push(solid_box);

    state.surveillance_settings.active_config.camera_tag = "SecCam_01".to_string();
    state.surveillance_settings.active_config.camera_mount.position = Vec3::new(510.0, 510.0, 250.0);
    state.recalculate_surveillance_preview();

    let has_obstruct = state.surveillance_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Obstructed"));
    assert!(has_obstruct, "Must flag critical obstruction hazard when camera touches solid geometry");
}
