use glam::Vec3;
use ut99_bsp::{
    simulate_translocator_flight, TranslocatorPreset, TranslocatorTossConfig,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_translocator_presets_coverage() {
    assert_eq!(TranslocatorPreset::ALL.len(), 6, "Must provide 6 canonical translocator toss presets");

    for &preset in &TranslocatorPreset::ALL {
        assert!(!preset.display_name().is_empty());
        let cfg = preset.to_config();
        assert!(cfg.toss_speed >= 400.0 && cfg.toss_speed <= 1800.0);
        assert!(cfg.gravity < -500.0);
        assert!(cfg.elasticity > 0.0 && cfg.elasticity < 1.0);
        assert!(cfg.friction > 0.0 && cfg.friction <= 1.0);
        assert_eq!(cfg.disc_radius, 6.0);
    }
}

#[test]
fn test_translocator_flight_trajectory_and_timing() {
    let cfg = TranslocatorPreset::MaxDistanceArc45.to_config();
    let start = Vec3::new(0.0, 0.0, 60.0);

    let sim = simulate_translocator_flight(start, &cfg, &[]);

    assert!(!sim.points.is_empty(), "Trajectory must produce sampled flight points");
    assert!(sim.total_flight_time > 0.0);
    assert!(sim.final_beacon_location.is_some());

    let final_pos = sim.final_beacon_location.unwrap();
    assert_eq!(final_pos.z, 0.0, "Disc must settle on the floor z = 0");
    assert!(final_pos.x > 800.0, "45-degree toss at 1200 uu/s must travel far along X");

    // Check transit timings
    assert!(sim.walking_travel_time > 0.0);
    assert!(sim.teleport_transit_time > sim.total_flight_time, "Transit time includes 0.2s switch latency");
    assert!(sim.time_saved_seconds > 0.0, "Max distance arc translocator toss must be faster than walking");
}

#[test]
fn test_translocator_bounce_damping_and_normal_reflection() {
    let mut cfg = TranslocatorTossConfig::default();
    cfg.toss_speed = 1000.0;
    cfg.pitch_deg = -30.0; // Angled down towards floor
    cfg.elasticity = 0.40;
    cfg.max_bounces = 4;

    let sim = simulate_translocator_flight(Vec3::new(0.0, 0.0, 200.0), &cfg, &[]);

    assert!(sim.bounce_count > 0, "Downward toss must bounce on the floor");
    assert!(sim.bounce_count <= cfg.max_bounces);

    let bounces: Vec<_> = sim.points.iter().filter(|p| p.is_bounce).collect();
    assert!(!bounces.is_empty());
    assert!(bounces[0].surface_normal.is_some());
    assert_eq!(bounces[0].surface_normal.unwrap(), Vec3::new(0.0, 0.0, 1.0));
}

#[test]
fn test_editor_translocator_probe_placement_and_undo_redo() {
    let mut state = EditorState::default();
    state.builder_brush.location = Vec3::new(250.0, 350.0, 120.0);
    state.translocator_studio_settings.selected_preset = TranslocatorPreset::HighLedgePerchLob;
    state.translocator_studio_settings.pitch_deg = 70.0;
    state.translocator_studio_settings.yaw_deg = 45.0;

    // 1. Place Probe
    let probe_idx = state.place_translocator_probe_at_pivot();
    assert_eq!(state.translocator_studio_settings.probes.len(), 1);
    let probe = &state.translocator_studio_settings.probes[probe_idx];
    assert_eq!(probe.preset, TranslocatorPreset::HighLedgePerchLob);
    assert_eq!(probe.pitch_deg, 70.0);
    assert_eq!(probe.yaw_deg, 45.0);

    // 2. Active simulation recalculated
    assert!(state.translocator_studio_settings.active_simulation.is_some());

    // 3. Test Undo
    assert!(state.history.can_undo());
    state.undo();
    assert_eq!(state.translocator_studio_settings.probes.len(), 0, "Undo must remove placed translocator probe");

    // 4. Test Redo
    assert!(state.history.can_redo());
    state.redo();
    assert_eq!(state.translocator_studio_settings.probes.len(), 1, "Redo must restore placed translocator probe");
    assert_eq!(state.translocator_studio_settings.probes[0].preset, TranslocatorPreset::HighLedgePerchLob);
}
