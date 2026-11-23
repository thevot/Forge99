// crates/ut99-editor/tests/test_audit_industrial_and_scifi_studios.rs
//
// Audit Test Suite for Industrial & Sci-Fi Environmental Studios (.. ):
// - Conveyor Belt, Treadmill & Surface Velocity Studio
// - Automated Defense Turret & Sentry Cannon Studio
// - Ambient Soundscape, Spatial Audio Attenuation & Acoustic Reverb Studio
// - Environmental Hazard Chamber & Pressure Airlock Studio
// - Industrial Pipe Network & Steam Valve Studio
// - Holographic Signage, Sci-Fi Billboard & Volumetric Display Studio
//
// Zero-warning, 100% industrial test verification.

use ut99_bsp::{
    ConveyorArchetype, HazardChamberArchetype, HologramArchetype, PipeArchetype,
    SoundscapeArchetype, TurretArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_audit_pro188_to_pro198_cross_system_commit_and_multi_undo_redo() {
    let mut state = EditorState::default();

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    // 1. Commit Conveyor Belt
    state.conveyor_settings.active_config.archetype = ConveyorArchetype::MiningOreConveyor;
    state.conveyor_settings.active_config.deck.length = 512.0;
    state.commit_conveyor_to_world();

    let brushes_after_conveyor = state.world_brushes.len();
    let actors_after_conveyor = state.loaded_unr.as_ref().unwrap().actors.len();
    assert!(brushes_after_conveyor >= initial_world_brushes + 3); // Belt, Rollers, Frame
    assert!(actors_after_conveyor >= initial_actors + 3); // ZoneInfo + 2 PathNodes

    // 2. Commit Defense Turret
    state.turret_settings.active_config.archetype = TurretArchetype::MinigunCeilingSentry;
    state.commit_turret_to_world();

    let brushes_after_turret = state.world_brushes.len();
    let actors_after_turret = state.loaded_unr.as_ref().unwrap().actors.len();
    assert!(brushes_after_turret >= brushes_after_conveyor + 2); // Mount + Shield
    assert!(actors_after_turret >= actors_after_conveyor + 5); // TeamCannon + 4 evasion waypoints

    // 3. Commit Soundscape
    state.soundscape_settings.active_config.archetype = SoundscapeArchetype::IndustrialGeneratorDrone;
    state.commit_soundscape_to_world();

    let brushes_after_soundscape = state.world_brushes.len();
    let actors_after_soundscape = state.loaded_unr.as_ref().unwrap().actors.len();
    assert!(brushes_after_soundscape >= brushes_after_turret + 1); // Acoustic volume brush
    assert!(actors_after_soundscape >= actors_after_turret + 2); // AmbientSound + ZoneInfo

    // 4. Commit Hazard Chamber
    state.hazard_chamber_settings.active_config.archetype = HazardChamberArchetype::HighVacuumDecompressionChamber;
    state.commit_hazard_chamber_to_world();

    let brushes_after_chamber = state.world_brushes.len();
    let actors_after_chamber = state.loaded_unr.as_ref().unwrap().actors.len();
    assert!(brushes_after_chamber >= brushes_after_soundscape + 5); // Enclosure, 2 Doors, Window, Console
    assert!(actors_after_chamber >= actors_after_soundscape + 5); // ZoneInfo, Trigger + 3 PathNodes

    // 5. Commit Pipe Conduit Network
    state.pipe_conduit_settings.active_config.archetype = PipeArchetype::HighPressureSteamMain;
    state.commit_pipe_conduit_to_world();

    let brushes_after_pipe = state.world_brushes.len();
    let actors_after_pipe = state.loaded_unr.as_ref().unwrap().actors.len();
    assert!(brushes_after_pipe >= brushes_after_chamber + 3); // Main conduit, elbows, flanges
    assert!(actors_after_pipe >= actors_after_chamber + 2); // Mover + Trigger

    // 6. Commit Hologram Display
    state.hologram_settings.active_config.archetype = HologramArchetype::LiandriCorporationEmblem;
    state.commit_hologram_to_world();

    let brushes_after_holo = state.world_brushes.len();
    let actors_after_holo = state.loaded_unr.as_ref().unwrap().actors.len();
    assert!(brushes_after_holo >= brushes_after_pipe + 3); // Display, Projector base, Beam cone
    assert!(actors_after_holo >= actors_after_pipe + 2); // Mover + Light

    // Total accumulated entities
    let total_brushes = state.world_brushes.len();
    let total_actors = state.loaded_unr.as_ref().unwrap().actors.len();
    assert!(total_brushes >= initial_world_brushes + 17);
    assert!(total_actors >= initial_actors + 19);

    // ================= Cascading Undo 6 times =================
    // Undo Hologram
    state.undo();
    assert_eq!(state.world_brushes.len(), brushes_after_pipe);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), actors_after_pipe);

    // Undo Pipe Conduit
    state.undo();
    assert_eq!(state.world_brushes.len(), brushes_after_chamber);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), actors_after_chamber);

    // Undo Hazard Chamber
    state.undo();
    assert_eq!(state.world_brushes.len(), brushes_after_soundscape);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), actors_after_soundscape);

    // Undo Soundscape
    state.undo();
    assert_eq!(state.world_brushes.len(), brushes_after_turret);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), actors_after_turret);

    // Undo Turret
    state.undo();
    assert_eq!(state.world_brushes.len(), brushes_after_conveyor);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), actors_after_conveyor);

    // Undo Conveyor
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // ================= Cascading Redo 6 times =================
    for _ in 0..6 {
        state.redo();
    }
    assert_eq!(state.world_brushes.len(), total_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), total_actors);
}

#[test]
fn test_audit_industrial_and_scifi_safety_evaluators() {
    let mut state = EditorState::default();

    // 1. Conveyor audit
    state.conveyor_settings.active_config.kinematics.speed_uu_per_sec = 1500.0;
    state.recalculate_conveyor_preview();
    assert!(state.conveyor_settings.cached_hazards.iter().any(|h| h.is_critical));

    // 2. Turret audit
    state.turret_settings.active_config.tracking.pitch_min_deg = 20.0;
    state.turret_settings.active_config.tracking.pitch_max_deg = -10.0;
    state.recalculate_turret_preview();
    assert!(state.turret_settings.cached_hazards.iter().any(|h| h.is_critical));

    // 3. Soundscape audit
    state.soundscape_settings.active_config.emitter.sound_volume = 0;
    state.recalculate_soundscape_preview();
    assert!(state.soundscape_settings.cached_hazards.iter().any(|h| h.is_critical));

    // 4. Hazard Chamber audit
    state.hazard_chamber_settings.active_config.damage_per_sec = 0.0;
    state.hazard_chamber_settings.active_config.b_kill_zone = false;
    state.recalculate_hazard_chamber_preview();
    assert!(state.hazard_chamber_settings.cached_hazards.iter().any(|h| h.is_critical));

    // 5. Pipe Conduit audit
    state.pipe_conduit_settings.active_config.has_elbow_bend = true;
    state.pipe_conduit_settings.active_config.elbow_radius_uu = 10.0;
    state.recalculate_pipe_preview();
    assert!(state.pipe_conduit_settings.cached_hazards.iter().any(|h| h.is_critical));

    // 6. Hologram audit
    state.hologram_settings.active_config.display_width = 16.0;
    state.recalculate_hologram_preview();
    assert!(state.hologram_settings.cached_hazards.iter().any(|h| h.is_critical));
}

#[test]
fn test_audit_industrial_physics_and_kinematic_evaluations() {
    let mut state = EditorState::default();

    // Conveyor 1:1 synchronization pan rate
    state.apply_conveyor_archetype(ConveyorArchetype::HighSpeedPackageChute);
    let sync = &state.conveyor_settings.cached_metrics;
    assert_eq!(sync.linear_velocity.length(), 512.0);
    assert_eq!(sync.texture_pan_rate_pixels_sec, 512.0);

    // Turret aiming direction and telemetry
    state.apply_turret_archetype(TurretArchetype::CoastalDefenseBattery);
    let tracking = &state.turret_settings.cached_metrics;
    assert!(tracking.swept_yaw_arc_deg > 0.0);

    // Soundscape listener probe attenuation
    state.apply_soundscape_archetype(SoundscapeArchetype::CathedralGothicVault);
    state.soundscape_settings.probe_listener_distance_uu = 200.0;
    state.recalculate_soundscape_preview();
    let probe = &state.soundscape_settings.cached_metrics;
    assert!(probe.outer_radius_uu > 0.0);
    assert!(probe.volume_fraction > 0.0);

    // Hologram wave and kinematics
    state.apply_hologram_archetype(HologramArchetype::TournamentLiveScoreboard);
    state.hologram_settings.simulated_time_seconds = 2.5;
    state.recalculate_hologram_preview();
    let kin = &state.hologram_settings.cached_metrics;
    assert!(kin.surface_area_sq_uu > 0.0);
    assert!(kin.effective_luminosity > 0.0);
}
