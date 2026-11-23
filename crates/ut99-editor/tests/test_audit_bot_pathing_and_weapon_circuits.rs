// crates/ut99-editor/tests/test_audit_bot_pathing_and_weapon_circuits.rs
//
// Master Audit Suite for (Interactive Bot AI Tactical JumpSpot & Translocator Route Studio)
// and (Interactive Tournament Weapon Placement & Ammo Circuit Studio).

use glam::Vec3;
use ut99_bsp::{BotJumpArchetype, TournamentWeaponType, WeaponCircuitArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_cross_system_multi_commit_bot_jump_and_weapon_circuit() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Tactical Jump Route (Deck16 Slime Bridge Jump)
    state.bot_jump_routes_settings.active_config = BotJumpArchetype::Deck16SlimeBridgeJump.to_config();
    let (plate_idx, jump_actors) = state.commit_jump_route_to_world();
    assert_eq!(plate_idx, initial_brushes);
    assert!(jump_actors.len() >= 2);

    // 2. Commit Tournament Weapon Station (Flak Cannon Altar)
    state.weapon_circuit_settings.active_config = WeaponCircuitArchetype::GothicCloseQuartersCarnage.to_config();
    state.weapon_circuit_settings.active_config.position = Vec3::new(400.0, 200.0, 0.0);
    let (altar_idx, weapon_actors) = state.commit_weapon_station_to_world();
    assert_eq!(altar_idx, initial_brushes + 1);
    assert_eq!(weapon_actors.len(), 3); // Flak + 2 Ammo

    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), jump_actors.len() + weapon_actors.len());

    // Verify actors coexist properly
    assert!(level.actors.iter().any(|a| a.class_name == "Botpack.JumpSpot"));
    assert!(level.actors.iter().any(|a| a.class_name == "Botpack.UT_FlakCannon"));
    assert!(level.actors.iter().any(|a| a.class_name == "Botpack.FlakShellAmmo"));
}

#[test]
fn test_cascading_multi_undo_redo_route_and_circuit() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // Setup state without pushing archetype config changes
    state.bot_jump_routes_settings.active_config.archetype = BotJumpArchetype::FaceSniperTowerClimb;
    state.bot_jump_routes_settings.active_config.b_always_translocate = true;
    let (_, jump_actors) = state.commit_jump_route_to_world(); // Push 1

    state.weapon_circuit_settings.active_config.archetype = WeaponCircuitArchetype::CampgroundsHeavyTDM;
    state.weapon_circuit_settings.active_config.position = Vec3::new(600.0, 0.0, 0.0);
    let (_, weap_actors) = state.commit_weapon_station_to_world(); // Push 2

    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    let total_actors = jump_actors.len() + weap_actors.len();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), total_actors);

    // Undo 1: Revert Weapon Station
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), jump_actors.len());

    // Undo 2: Revert Jump Route
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // Redo 1: Restore Jump Route
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), jump_actors.len());

    // Redo 2: Restore Weapon Station
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), total_actors);
}

#[test]
fn test_arena_integration_bot_reachability_and_weapon_telemetry() {
    let mut state = EditorState::default();

    // 1. Setup perch Jump Route with destination at (500, 0, 200)
    state.bot_jump_routes_settings.active_config.start_pos = Vec3::new(0.0, 0.0, 0.0);
    state.bot_jump_routes_settings.active_config.target_pos = Vec3::new(500.0, 0.0, 200.0);
    state.recalculate_jump_route_preview();
    assert!(state.bot_jump_routes_settings.cached_solution.is_reachable);

    // 2. Place Sniper Rifle at the perch
    state.weapon_circuit_settings.active_config.position = Vec3::new(500.0, 0.0, 200.0);
    state.weapon_circuit_settings.active_config.primary_weapon = TournamentWeaponType::SniperRifle;
    state.recalculate_weapon_circuit_preview();

    assert_eq!(state.weapon_circuit_settings.cached_metrics.total_weapons_in_level, 1);
    assert!(state.weapon_circuit_settings.cached_metrics.total_max_burst_damage_potential >= 150.0);
}
