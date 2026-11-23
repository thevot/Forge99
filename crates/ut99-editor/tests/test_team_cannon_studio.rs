// crates/ut99-editor/tests/test_team_cannon_studio.rs
//
// Integration tests for Interactive Team Cannon & Automated Turret Defense Studio.

use glam::Vec3;
use ut99_bsp::{CannonWeaponType, TeamCannonArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_team_cannon_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is OverlordBeachBunkerCannon
    assert_eq!(
        state.team_cannon_settings.active_config.archetype,
        TeamCannonArchetype::OverlordBeachBunkerCannon
    );
    assert_eq!(state.team_cannon_settings.active_config.team, 0);
    assert_eq!(
        state.team_cannon_settings.active_config.cannon_type,
        CannonWeaponType::MinigunGatling
    );

    // Apply HiSpeedRoofMinigunCannon
    state.apply_team_cannon_archetype(TeamCannonArchetype::HiSpeedRoofMinigunCannon);
    assert_eq!(
        state.team_cannon_settings.active_config.archetype,
        TeamCannonArchetype::HiSpeedRoofMinigunCannon
    );

    // Apply FaceSniperTowerGatling
    state.apply_team_cannon_archetype(TeamCannonArchetype::FaceSniperTowerGatling);
    assert_eq!(
        state.team_cannon_settings.active_config.archetype,
        TeamCannonArchetype::FaceSniperTowerGatling
    );
    assert_eq!(
        state.team_cannon_settings.active_config.cannon_type,
        CannonWeaponType::SniperRailBattery
    );

    // Apply FrigateDeckMountedFlak
    state.apply_team_cannon_archetype(TeamCannonArchetype::FrigateDeckMountedFlak);
    assert_eq!(
        state.team_cannon_settings.active_config.archetype,
        TeamCannonArchetype::FrigateDeckMountedFlak
    );

    // Apply SubterraneanCeilingTurret
    state.apply_team_cannon_archetype(TeamCannonArchetype::SubterraneanCeilingTurret);
    assert_eq!(
        state.team_cannon_settings.active_config.archetype,
        TeamCannonArchetype::SubterraneanCeilingTurret
    );
    assert!(state.team_cannon_settings.active_config.is_inverted);

    // Undo back to Frigate
    state.undo();
    assert_eq!(
        state.team_cannon_settings.active_config.archetype,
        TeamCannonArchetype::FrigateDeckMountedFlak
    );
    assert!(!state.team_cannon_settings.active_config.is_inverted);

    // Redo to Subterranean
    state.redo();
    assert_eq!(
        state.team_cannon_settings.active_config.archetype,
        TeamCannonArchetype::SubterraneanCeilingTurret
    );
    assert!(state.team_cannon_settings.active_config.is_inverted);
}

#[test]
fn test_team_cannon_metrics_and_ballistics() {
    let mut state = EditorState::default();

    state.apply_team_cannon_archetype(TeamCannonArchetype::OverlordBeachBunkerCannon);
    state.recalculate_team_cannon_preview();

    let metrics = &state.team_cannon_settings.cached_metrics;
    assert_eq!(metrics.dps, 90.0);
    assert!((metrics.time_to_kill_standard_player_secs - 1.111).abs() < 0.01);
    assert_eq!(metrics.target_acquisition_latency_secs, 2.0); // 180 / 90
}

#[test]
fn test_team_cannon_safety_auditing() {
    let mut state = EditorState::default();

    state.team_cannon_settings.active_config.team = 0; // Red
    state.team_cannon_settings.active_config.position = Vec3::new(0.0, 0.0, 0.0);
    state.team_cannon_settings.active_config.max_range_uu = 3200.0;

    // Place an opposing PlayerStart in firing range
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        let mut p_start = ut99_format::ActorRecord::default();
        p_start.name = "PlayerStart_Blue_0".to_string();
        p_start.class_name = "Engine.PlayerStart".to_string();
        p_start.location = Vec3::new(600.0, 0.0, 0.0);
        level.actors.push(p_start);
    }

    state.recalculate_team_cannon_preview();
    assert!(state.team_cannon_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Spawn Camp")));
}

#[test]
fn test_commit_team_cannon_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Cannon
    state.apply_team_cannon_archetype(TeamCannonArchetype::OverlordBeachBunkerCannon);
    let (brush_idx, actor_idx) = state.commit_team_cannon_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(actor_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    let act = &level.actors[actor_idx.unwrap()];
    assert_eq!(act.class_name, "Botpack.TeamCannon");
    assert_eq!(act.group, "Defense");
    assert_eq!(act.tag, "BeachCannon_Red");

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
    assert_eq!(level_redone.actors[0].class_name, "Botpack.TeamCannon");
}
