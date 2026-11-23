// crates/ut99-editor/tests/test_spawn_distribution_studio.rs
//
// Integration tests for Interactive Tournament Spawn Point Balancing Studio.

use glam::Vec3;
use ut99_bsp::{SpawnDistributionArchetype, TournamentTeamId};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_spawn_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is Deck16DuelSpawnRing
    assert_eq!(
        state.spawn_distribution_settings.active_config.archetype,
        SpawnDistributionArchetype::Deck16DuelSpawnRing
    );
    assert_eq!(
        state.spawn_distribution_settings.active_config.team_number,
        TournamentTeamId::FFA
    );
    assert_eq!(
        state.spawn_distribution_settings.active_config.yaw_degrees,
        0.0
    );

    // Apply FaceSymmetricCTFSpawns
    state.apply_spawn_archetype(SpawnDistributionArchetype::FaceSymmetricCTFSpawns);
    assert_eq!(
        state.spawn_distribution_settings.active_config.archetype,
        SpawnDistributionArchetype::FaceSymmetricCTFSpawns
    );
    assert_eq!(
        state.spawn_distribution_settings.active_config.team_number,
        TournamentTeamId::Red
    );
    assert_eq!(
        state.spawn_distribution_settings.active_config.yaw_degrees,
        90.0
    );

    // Apply CampgroundsTDMGrid
    state.apply_spawn_archetype(SpawnDistributionArchetype::CampgroundsTDMGrid);
    assert_eq!(
        state.spawn_distribution_settings.active_config.archetype,
        SpawnDistributionArchetype::CampgroundsTDMGrid
    );
    assert_eq!(
        state.spawn_distribution_settings.active_config.team_number,
        TournamentTeamId::Blue
    );
    assert_eq!(
        state.spawn_distribution_settings.active_config.yaw_degrees,
        180.0
    );

    // Apply PressureTrapPerimeter
    state.apply_spawn_archetype(SpawnDistributionArchetype::PressureTrapPerimeter);
    assert_eq!(
        state.spawn_distribution_settings.active_config.archetype,
        SpawnDistributionArchetype::PressureTrapPerimeter
    );
    assert_eq!(
        state.spawn_distribution_settings.active_config.team_number,
        TournamentTeamId::FFA
    );

    // Apply NoviceLinearSpawnHub
    state.apply_spawn_archetype(SpawnDistributionArchetype::NoviceLinearSpawnHub);
    assert_eq!(
        state.spawn_distribution_settings.active_config.archetype,
        SpawnDistributionArchetype::NoviceLinearSpawnHub
    );

    // Undo back to Pressure
    state.undo();
    assert_eq!(
        state.spawn_distribution_settings.active_config.archetype,
        SpawnDistributionArchetype::PressureTrapPerimeter
    );

    // Redo to Novice
    state.redo();
    assert_eq!(
        state.spawn_distribution_settings.active_config.archetype,
        SpawnDistributionArchetype::NoviceLinearSpawnHub
    );
}

#[test]
fn test_spawn_metrics_and_camping_exposure() {
    let mut state = EditorState::default();

    // Populate level with a PlayerStart and a weapon
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    let level = state.loaded_unr.as_mut().unwrap();

    let mut ps = ut99_format::ActorRecord::default();
    ps.name = "PlayerStart0".to_string();
    ps.class_name = "Engine.PlayerStart".to_string();
    ps.location = Vec3::new(400.0, 0.0, 0.0);
    level.actors.push(ps);

    let mut w = ut99_format::ActorRecord::default();
    w.name = "Shock0".to_string();
    w.class_name = "Botpack.ShockRifle".to_string();
    w.location = Vec3::new(600.0, 0.0, 0.0);
    level.actors.push(w);

    state.recalculate_spawn_preview();
    let metrics = &state.spawn_distribution_settings.cached_metrics;

    assert_eq!(metrics.total_player_starts_in_level, 1);
    assert_eq!(metrics.nearest_spawn_distance_uu, 400.0);
    assert_eq!(metrics.nearest_weapon_distance_uu, 600.0);
    assert_eq!(metrics.unarmed_sprint_time_secs, 1.5);
    assert!(metrics.spawn_camping_exposure_index >= 20.0);
}

#[test]
fn test_spawn_hazard_auditing() {
    let mut state = EditorState::default();

    // Setup an adjacent PlayerStart at distance 80 (< 120 uu)
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();
        let mut ps = ut99_format::ActorRecord::default();
        ps.name = "AdjacentStart".to_string();
        ps.class_name = "Engine.PlayerStart".to_string();
        ps.location = Vec3::new(80.0, 0.0, 0.0);
        level.actors.push(ps);
    }

    state.recalculate_spawn_preview();
    assert!(state.spawn_distribution_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Telefrag")));

    // Test weapon starvation hazard
    {
        let level = state.loaded_unr.as_mut().unwrap();
        let mut w = ut99_format::ActorRecord::default();
        w.name = "DistantWeapon".to_string();
        w.class_name = "Botpack.ShockRifle".to_string();
        w.location = Vec3::new(1200.0, 0.0, 0.0);
        level.actors.push(w);
    }

    state.recalculate_spawn_preview();
    assert!(state.spawn_distribution_settings.cached_hazards.iter().any(|h| h.title.contains("Starvation")));
}

#[test]
fn test_commit_spawn_pad_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Spawn Pad
    state.apply_spawn_archetype(SpawnDistributionArchetype::Deck16DuelSpawnRing);
    let (brush_idx, actor_idx) = state.commit_spawn_pad_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(actor_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    let act = &level.actors[actor_idx.unwrap()];
    assert_eq!(act.class_name, "Engine.PlayerStart");
    assert_eq!(act.group, "Navigation");

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
    assert_eq!(level_redone.actors[0].class_name, "Engine.PlayerStart");
}
