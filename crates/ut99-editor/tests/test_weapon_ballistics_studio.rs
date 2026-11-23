use glam::Vec3;
use ut99_bsp::{
    audit_combat_sightline, simulate_weapon_trajectory, Brush, CsgOperation, Polygon,
    WeaponCombatPreset,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_weapon_presets_and_projectiles() {
    let presets = WeaponCombatPreset::all();
    assert_eq!(presets.len(), 6, "Must provide 6 canonical combat weapon presets");

    for &p in presets {
        assert!(!p.display_name().is_empty());
        assert!(!p.description().is_empty());

        let proj = p.to_projectile();
        assert!(!proj.name.is_empty());
        assert!(proj.initial_speed >= 800.0);
        assert!(proj.damage_amount > 0.0);
        assert!(proj.damage_radius > 0.0);
    }
}

#[test]
fn test_trajectory_simulation_and_ricochets() {
    // 1. Eightball Grenade bouncing off a solid floor
    let grenade = WeaponCombatPreset::EightballGrenadeArc.to_projectile();
    let floor_poly = Polygon::new(vec![
        Vec3::new(-500.0, -500.0, 0.0),
        Vec3::new(500.0, -500.0, 0.0),
        Vec3::new(500.0, 500.0, 0.0),
        Vec3::new(-500.0, 500.0, 0.0),
    ]).expect("Valid polygon");

    let mut floor_brush = Brush::new("Floor", CsgOperation::Add);
    floor_brush.polys.push(floor_poly);

    let traj = simulate_weapon_trajectory(
        Vec3::new(0.0, 0.0, 100.0),
        Vec3::new(0.0, 1.0, -0.5),
        &grenade,
        &[floor_brush],
        2.0,
        0.02,
    );

    assert!(traj.len() > 5);
    let has_bounce = traj.iter().any(|pt| pt.is_bounce);
    assert!(has_bounce, "Grenade launched downward towards solid floor must bounce");

    // 2. Rocket linear propulsion without gravity
    let rocket = WeaponCombatPreset::RocketVolleyBarrage.to_projectile();
    let rocket_traj = simulate_weapon_trajectory(
        Vec3::new(0.0, 0.0, 100.0),
        Vec3::new(0.0, 1.0, 0.0),
        &rocket,
        &[],
        1.0,
        0.05,
    );

    assert!(rocket_traj.len() > 5);
    for pt in &rocket_traj {
        assert!((pt.position.z - 100.0).abs() < 1e-3, "Rocket with 0 gravity must stay exactly at Z=100");
    }
}

#[test]
fn test_combat_sightline_audit() {
    let muzzle = Vec3::new(0.0, 0.0, 50.0);
    let target = Vec3::new(0.0, 800.0, 50.0);

    // 1. Clear shot with no brushes
    let clear = audit_combat_sightline(muzzle, target, &[]);
    assert!(clear.is_unobstructed);
    assert_eq!(clear.hit_distance, 800.0);

    // 2. Obstructed shot with pillar at Y=400
    let pillar_poly = Polygon::new(vec![
        Vec3::new(-100.0, 400.0, -100.0),
        Vec3::new(100.0, 400.0, -100.0),
        Vec3::new(100.0, 400.0, 200.0),
        Vec3::new(-100.0, 400.0, 200.0),
    ]).expect("Valid polygon");

    let mut pillar = Brush::new("BunkerPillar", CsgOperation::Add);
    pillar.polys.push(pillar_poly);

    let blocked = audit_combat_sightline(muzzle, target, &[pillar]);
    assert!(!blocked.is_unobstructed);
    assert!((blocked.hit_distance - 400.0).abs() < 1e-2);
    assert_eq!(blocked.first_blocking_brush, Some("BunkerPillar".to_string()));
}

#[test]
fn test_editor_ballistics_studio_placement_and_undo_redo() {
    let mut state = EditorState::default();
    state.builder_brush.location = Vec3::new(256.0, 512.0, 64.0);

    let initial_probes = state.ballistics_studio_settings.probes.len();

    // 1. Place Ballistic Probe at Pivot
    let probe_idx = state.place_ballistic_probe_at_pivot(WeaponCombatPreset::SniperSightlineLongshot);
    assert_eq!(state.ballistics_studio_settings.probes.len(), initial_probes + 1);

    let probe = &state.ballistics_studio_settings.probes[probe_idx];
    assert_eq!(probe.location, Vec3::new(256.0, 512.0, 64.0));
    assert_eq!(probe.preset, WeaponCombatPreset::SniperSightlineLongshot);
    assert!(!state.ballistics_studio_settings.active_trajectory.is_empty());

    // Test Undo placement
    assert!(state.history.can_undo());
    state.undo();
    assert_eq!(state.ballistics_studio_settings.probes.len(), initial_probes);

    // Test Redo placement
    assert!(state.history.can_redo());
    state.redo();
    assert_eq!(state.ballistics_studio_settings.probes.len(), initial_probes + 1);
    assert_eq!(state.ballistics_studio_settings.probes[probe_idx].preset, WeaponCombatPreset::SniperSightlineLongshot);

    // 2. Trajectory recalculation
    state.ballistics_studio_settings.pitch_deg = 45.0;
    state.ballistics_studio_settings.selected_preset = WeaponCombatPreset::RocketVolleyBarrage;
    state.ballistics_studio_settings.preview_projectile = WeaponCombatPreset::RocketVolleyBarrage.to_projectile();
    state.calculate_active_weapon_trajectory();

    assert!(!state.ballistics_studio_settings.active_trajectory.is_empty());
    let traj = &state.ballistics_studio_settings.active_trajectory;
    // With 45 deg pitch, Z should ascend
    assert!(traj.last().unwrap().position.z > traj[0].position.z);
}
