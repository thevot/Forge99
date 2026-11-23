// crates/ut99-editor/tests/test_audit_competitive_arena_studios.rs
//
// Master Audit Suite for Competitive Arena Studios:
// - Interactive Bot AI Tactical JumpSpot & Translocator Route Studio
// - Interactive Tournament Weapon Placement & Ammo Circuit Studio
// - Interactive Tournament Powerup Control & Relic Altar Studio
// - Interactive Tournament Spawn Point Balancing & Telefrag Prevention Studio

use glam::Vec3;
use ut99_bsp::{
    BotJumpArchetype, PowerupArchetype, SpawnDistributionArchetype, WeaponCircuitArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_audit_arena_cross_system_commit() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // Stage 1: Commit Bot Jump Route
    state.apply_jump_route_archetype(BotJumpArchetype::FaceSniperTowerClimb);
    let (jump_brush_idx, jump_actor_indices) = state.commit_jump_route_to_world();
    assert_eq!(jump_brush_idx, initial_brushes);
    assert_eq!(jump_actor_indices.len(), 3); // JumpPad trigger + 2 JumpSpot / Lift waypoints

    // Stage 2: Commit Weapon Station
    state.apply_weapon_circuit_archetype(WeaponCircuitArchetype::Deck16DuelMasterCircuit);
    let (weapon_brush_idx, weapon_actor_indices) = state.commit_weapon_station_to_world();
    assert_eq!(weapon_brush_idx, initial_brushes + 1);
    assert!(!weapon_actor_indices.is_empty()); // Weapon + Ammo boxes

    // Stage 3: Commit Powerup Altar
    state.apply_powerup_archetype(PowerupArchetype::Deck16HighLedgeUDamage);
    let (powerup_brush_idx, powerup_actor_idx) = state.commit_powerup_to_world();
    assert_eq!(powerup_brush_idx, initial_brushes + 2);
    assert!(powerup_actor_idx.is_some());

    // Stage 4: Commit Spawn Point
    state.apply_spawn_archetype(SpawnDistributionArchetype::Deck16DuelSpawnRing);
    let (spawn_brush_idx, spawn_actor_idx) = state.commit_spawn_pad_to_world();
    assert_eq!(spawn_brush_idx, initial_brushes + 3);
    assert!(spawn_actor_idx.is_some());

    // Cumulative world checks
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);

    let level = state.loaded_unr.as_ref().unwrap();
    let total_actors = level.actors.len();
    assert_eq!(
        total_actors,
        jump_actor_indices.len() + weapon_actor_indices.len() + 1 + 1
    );

    // Verify presence of classes
    assert!(level.actors.iter().any(|a| a.class_name == "Botpack.JumpSpot"));
    assert!(level.actors.iter().any(|a| a.class_name == "Botpack.ShockRifle"));
    assert!(level.actors.iter().any(|a| a.class_name == "Botpack.UDamage"));
    assert!(level.actors.iter().any(|a| a.class_name == "Engine.PlayerStart"));
}

#[test]
fn test_audit_arena_cascading_multi_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // Directly assign configs to ensure exactly 1 commit action per studio on the undo stack
    state.bot_jump_routes_settings.active_config = BotJumpArchetype::FaceSniperTowerClimb.to_config();
    state.commit_jump_route_to_world();

    state.weapon_circuit_settings.active_config = WeaponCircuitArchetype::Deck16DuelMasterCircuit.to_config();
    state.commit_weapon_station_to_world();

    state.powerup_control_settings.active_config = PowerupArchetype::Deck16HighLedgeUDamage.to_config();
    state.commit_powerup_to_world();

    state.spawn_distribution_settings.active_config = SpawnDistributionArchetype::Deck16DuelSpawnRing.to_config();
    state.commit_spawn_pad_to_world();

    let committed_brushes = state.world_brushes.len();
    assert_eq!(committed_brushes, initial_brushes + 4);
    let committed_actors = state.loaded_unr.as_ref().unwrap().actors.len();

    // Cascading Undo 1: Undo Spawn Point
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);

    // Cascading Undo 2: Undo Powerup Altar
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);

    // Cascading Undo 3: Undo Weapon Station
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);

    // Cascading Undo 4: Undo Jump Route
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // Cascading Redo 1: Redo Jump Route
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);

    // Cascading Redo 2: Redo Weapon Station
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);

    // Cascading Redo 3: Redo Powerup Altar
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);

    // Cascading Redo 4: Redo Spawn Point
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), committed_actors);
}

#[test]
fn test_audit_arena_cross_system_spatial_and_safety_interference() {
    let mut state = EditorState::default();

    // Configure a Powerup Altar (UDamage) at position (0, 0, 0)
    state.powerup_control_settings.active_config = PowerupArchetype::Deck16HighLedgeUDamage.to_config();
    state.powerup_control_settings.active_config.position = Vec3::new(0.0, 0.0, 0.0);
    state.commit_powerup_to_world();

    // Place a Weapon Station at (250, 0, 0) - very close!
    state.weapon_circuit_settings.active_config = WeaponCircuitArchetype::Deck16DuelMasterCircuit.to_config();
    state.weapon_circuit_settings.active_config.position = Vec3::new(250.0, 0.0, 0.0);
    state.recalculate_weapon_circuit_preview();

    // Place a PlayerStart at (50, 0, 0) - adjacent to powerup (< 120 uu)
    state.spawn_distribution_settings.active_config = SpawnDistributionArchetype::Deck16DuelSpawnRing.to_config();
    state.spawn_distribution_settings.active_config.position = Vec3::new(50.0, 0.0, 0.0);
    state.recalculate_spawn_preview();

    // Verify that the Powerup Studio sees the weapon at 250 uu
    state.recalculate_powerup_preview();
    let p_metrics = &state.powerup_control_settings.cached_metrics;
    assert_eq!(p_metrics.nearest_weapon_distance_uu, 9999.0); // No weapon committed yet

    // Commit Weapon Station
    state.commit_weapon_station_to_world();
    state.recalculate_powerup_preview();
    let p_metrics2 = &state.powerup_control_settings.cached_metrics;
    assert!(p_metrics2.nearest_weapon_distance_uu <= 250.0 && p_metrics2.nearest_weapon_distance_uu >= 200.0);

    // Verify Spawn point safety sees the adjacent weapon
    state.recalculate_spawn_preview();
    let sp_metrics = &state.spawn_distribution_settings.cached_metrics;
    assert!(sp_metrics.nearest_weapon_distance_uu <= 250.0);
    assert!(sp_metrics.unarmed_sprint_time_secs < 1.0);
}
