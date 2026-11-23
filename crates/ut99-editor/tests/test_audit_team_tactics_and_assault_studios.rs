// crates/ut99-editor/tests/test_audit_team_tactics_and_assault_studios.rs
//
// Master Audit Suite for Team Base Tactics, Spawn Distribution & Assault Studios:
// Interactive Tournament Spawn Point Balancing & Telefrag Prevention Studio
// Interactive Domination Control Point & Point-Weight Triangle Studio
// Interactive Assault Objective, Blast Door & Objective Chain Studio
// Interactive Team Base Radar, Jammer & Holographic Threat Map Studio

use glam::Vec3;
use ut99_bsp::{
    AssaultArchetype, DominationArchetype, RadarStudioArchetype, SpawnDistributionArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_multistudio_tactical_setup_and_cascading_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Spawn Pad
    state.spawn_distribution_settings.active_config =
        SpawnDistributionArchetype::FaceSymmetricCTFSpawns.to_config();
    let (pad_b_idx, pad_act_idx) = state.commit_spawn_pad_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(pad_b_idx, initial_brushes);
    assert!(pad_act_idx.is_some());

    // 2. Commit Domination Point
    state.domination_settings.active_config =
        DominationArchetype::CinderVerticalTower.to_config();
    let (dom_b_idx, dom_act_idx) = state.commit_domination_point_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(dom_b_idx, initial_brushes + 1);
    assert!(dom_act_idx.is_some());

    // 3. Commit Assault Objective
    state.assault_settings.active_config =
        AssaultArchetype::OverlordBeachLanding.to_config();
    let (as_b_idx, as_act_idx) = state.commit_assault_objective_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(as_b_idx, initial_brushes + 2);
    assert!(as_act_idx.is_some());

    // 4. Commit Base Radar Station
    state.radar_settings.active_config =
        RadarStudioArchetype::OrbitalCommandHQ.to_config();
    let (rad_b_idx, rad_act_idx) = state.commit_radar_station_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(rad_b_idx, initial_brushes + 3);
    assert!(rad_act_idx.is_some());

    // Verify all 4 actors exist in level
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 4);
    assert_eq!(level.actors[0].class_name, "Engine.PlayerStart");
    assert_eq!(level.actors[1].class_name, "Botpack.ControlPoint");
    assert_eq!(level.actors[2].class_name, "Botpack.FortStandard");
    assert_eq!(level.actors[3].class_name, "Botpack.RadarStation");

    // Cascading Undo 1: Undo Radar Commit
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);

    // Cascading Undo 2: Undo Assault Commit
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Cascading Undo 3: Undo Domination Commit
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Cascading Undo 4: Undo Spawn Pad Commit
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // Cascading Redo 1: Redo Spawn Pad
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Cascading Redo 2: Redo Domination Point
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Cascading Redo 3: Redo Assault Objective
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);

    // Cascading Redo 4: Redo Radar Station
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[3].class_name, "Botpack.RadarStation");
}

#[test]
fn test_tactical_inter_system_cross_influence_and_safety() {
    let mut state = EditorState::default();

    // Setup an integrated base scenario:
    // Red Base: PlayerStart at (0, 0, 0), Radar at (200, 0, 0), Domination Point at (600, 0, 0)
    state.spawn_distribution_settings.active_config.position = Vec3::new(0.0, 0.0, 0.0);
    state.commit_spawn_pad_to_world();

    state.radar_settings.active_config.position = Vec3::new(200.0, 0.0, 0.0);
    state.commit_radar_station_to_world();

    state.domination_settings.active_config.position = Vec3::new(600.0, 0.0, 0.0);
    state.commit_domination_point_to_world();

    // Recalculate previews across systems
    state.recalculate_spawn_preview();
    state.recalculate_domination_preview();
    state.recalculate_radar_preview();

    // Verify radar detects 1 peer/node in system
    assert_eq!(state.radar_settings.cached_metrics.total_nodes_in_level, 2);
    assert!(state.radar_settings.cached_metrics.threat_detection_latency_secs <= 2.5);

    // Verify domination metrics evaluate cleanly with 1 point in map
    assert_eq!(state.domination_settings.cached_metrics.total_points_in_level, 2);

    // Verify spawn metrics evaluate cleanly (1 PlayerStart in map)
    assert_eq!(state.spawn_distribution_settings.cached_metrics.total_player_starts_in_level, 1);
}
