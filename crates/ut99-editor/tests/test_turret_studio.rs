// crates/ut99-editor/tests/test_turret_studio.rs
//
// Integration tests for Automated Defense Turret, Sentry Cannon & Emplacement Studio.

use ut99_bsp::{TurretArchetype, TurretMountType, TurretTeamOwnership, TurretWeaponType};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_turret_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is CoastalDefenseBattery
    assert_eq!(
        state.turret_settings.active_config.archetype,
        TurretArchetype::CoastalDefenseBattery
    );
    assert_eq!(state.turret_settings.active_config.tracking.tracking_range, 3000.0);
    assert_eq!(state.turret_settings.active_config.team, TurretTeamOwnership::RedTeam);

    // Apply MinigunCeilingSentry
    state.apply_turret_archetype(TurretArchetype::MinigunCeilingSentry);
    assert_eq!(
        state.turret_settings.active_config.archetype,
        TurretArchetype::MinigunCeilingSentry
    );
    assert_eq!(state.turret_settings.active_config.mount.mount_type, TurretMountType::CeilingSuspended);
    assert_eq!(state.turret_settings.active_config.combat.weapon_type, TurretWeaponType::MinigunBullet);

    // Apply PerimeterRocketPod
    state.apply_turret_archetype(TurretArchetype::PerimeterRocketPod);
    assert_eq!(
        state.turret_settings.active_config.archetype,
        TurretArchetype::PerimeterRocketPod
    );
    assert_eq!(state.turret_settings.active_config.mount.barrel_count, 2);

    // Apply ShockBeamTurret
    state.apply_turret_archetype(TurretArchetype::ShockBeamTurret);
    assert_eq!(
        state.turret_settings.active_config.archetype,
        TurretArchetype::ShockBeamTurret
    );
    assert_eq!(state.turret_settings.active_config.combat.weapon_type, TurretWeaponType::ShockBeam);

    // Apply SniperFlakBunker
    state.apply_turret_archetype(TurretArchetype::SniperFlakBunker);
    assert_eq!(
        state.turret_settings.active_config.archetype,
        TurretArchetype::SniperFlakBunker
    );
    assert_eq!(state.turret_settings.active_config.combat.health, 400.0);

    // Undo returns to ShockBeamTurret
    state.undo();
    assert_eq!(
        state.turret_settings.active_config.archetype,
        TurretArchetype::ShockBeamTurret
    );

    // Redo returns to SniperFlakBunker
    state.redo();
    assert_eq!(
        state.turret_settings.active_config.archetype,
        TurretArchetype::SniperFlakBunker
    );
}

#[test]
fn test_turret_patrol_kinematics_and_time_scrubbing() {
    let mut state = EditorState::default();

    state.apply_turret_archetype(TurretArchetype::MinigunCeilingSentry);

    let m0 = state.turret_settings.cached_metrics.clone();
    assert!(m0.aiming_direction.length() > 0.99);
    assert_eq!(m0.swept_yaw_arc_deg, 360.0);

    // Scrub time and verify timeline updates
    state.scrub_turret_time(1.5);
    assert_eq!(state.turret_settings.simulated_time_seconds, 1.5);
    let m1 = &state.turret_settings.cached_metrics;
    assert!(m1.aiming_direction.length() > 0.99);
}

#[test]
fn test_commit_turret_to_world_undo_redo() {
    let mut state = EditorState::default();

    state.apply_turret_archetype(TurretArchetype::CoastalDefenseBattery);

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    let (mount_idx, shield_idx, turret_idx, waypoint_indices) = state.commit_turret_to_world();

    assert_eq!(mount_idx, initial_world_brushes);
    assert_eq!(shield_idx, initial_world_brushes + 1);
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 2);

    // Mount brush: 10 polys, PF_SEMISOLID (0x200000)
    let mount_b = &state.world_brushes[mount_idx];
    assert_eq!(mount_b.polys.len(), 10);
    assert_eq!(mount_b.polys[0].flags & 0x00200000, 0x00200000);

    // Shield brush: 6 polys, PF_SEMISOLID (0x200000)
    let shield_b = &state.world_brushes[shield_idx];
    assert_eq!(shield_b.polys.len(), 6);
    assert_eq!(shield_b.polys[0].flags & 0x00200000, 0x00200000);

    // Actors created: TeamCannon + 4 PathNodes = 5 actors
    assert!(turret_idx.is_some());
    assert_eq!(waypoint_indices.len(), 4);

    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors.len(), initial_actors + 5);

    let turret_act = &unr.actors[turret_idx.unwrap()];
    assert_eq!(turret_act.class_name, "Botpack.TeamCannon");

    for &w_idx in &waypoint_indices {
        let node = &unr.actors[w_idx];
        assert_eq!(node.class_name, "Engine.PathNode");
    }

    // Undo removes both brushes and all 5 actors
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // Redo restores both brushes and all 5 actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors + 5);
}

#[test]
fn test_turret_safety_audit() {
    let mut state = EditorState::default();

    // 1. Invalid elevation (pitch_max <= pitch_min) flags critical hazard
    state.turret_settings.active_config.tracking.pitch_min_deg = 20.0;
    state.turret_settings.active_config.tracking.pitch_max_deg = -10.0;
    state.recalculate_turret_preview();
    let has_pitch = state.turret_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Invalid Elevation"));
    assert!(has_pitch, "Must flag critical hazard for inverted pitch limits");

    // 2. Excessive tracking range (>4000 uu) flags critical hazard
    state.turret_settings.active_config.tracking.pitch_min_deg = -10.0;
    state.turret_settings.active_config.tracking.pitch_max_deg = 45.0;
    state.turret_settings.active_config.tracking.tracking_range = 4500.0;
    state.recalculate_turret_preview();
    let has_range = state.turret_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Excessive Tracking"));
    assert!(has_range, "Must flag critical hazard for range > 4000 uu");

    // 3. Punishing DPS (>300 HP/s) flags warning
    state.turret_settings.active_config.tracking.tracking_range = 2500.0;
    state.turret_settings.active_config.combat.damage_per_shot = 100.0;
    state.turret_settings.active_config.combat.fire_rate_hz = 5.0; // DPS = 500
    state.recalculate_turret_preview();
    let has_dps = state.turret_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Punishing Turret DPS"));
    assert!(has_dps, "Must flag warning for DPS > 300 HP/s");
}
