// crates/ut99-editor/tests/test_weapon_circuit_studio.rs
//
// Integration tests for Interactive Tournament Weapon Placement & Ammo Circuit Studio.

use ut99_bsp::{TournamentWeaponType, WeaponCircuitArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_weapon_circuit_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is Deck16DuelMasterCircuit
    assert_eq!(
        state.weapon_circuit_settings.active_config.archetype,
        WeaponCircuitArchetype::Deck16DuelMasterCircuit
    );
    assert_eq!(
        state.weapon_circuit_settings.active_config.primary_weapon,
        TournamentWeaponType::ShockRifle
    );
    assert_eq!(
        state.weapon_circuit_settings.active_config.flanking_ammo_count,
        2
    );

    // Apply CampgroundsHeavyTDM
    state.apply_weapon_circuit_archetype(WeaponCircuitArchetype::CampgroundsHeavyTDM);
    assert_eq!(
        state.weapon_circuit_settings.active_config.archetype,
        WeaponCircuitArchetype::CampgroundsHeavyTDM
    );
    assert_eq!(
        state.weapon_circuit_settings.active_config.primary_weapon,
        TournamentWeaponType::RocketLauncher
    );
    assert!(state.weapon_circuit_settings.active_config.b_weapon_stay);

    // Apply FaceLongRangeAssault
    state.apply_weapon_circuit_archetype(WeaponCircuitArchetype::FaceLongRangeAssault);
    assert_eq!(
        state.weapon_circuit_settings.active_config.archetype,
        WeaponCircuitArchetype::FaceLongRangeAssault
    );
    assert_eq!(
        state.weapon_circuit_settings.active_config.primary_weapon,
        TournamentWeaponType::SniperRifle
    );

    // Apply GothicCloseQuartersCarnage
    state.apply_weapon_circuit_archetype(WeaponCircuitArchetype::GothicCloseQuartersCarnage);
    assert_eq!(
        state.weapon_circuit_settings.active_config.archetype,
        WeaponCircuitArchetype::GothicCloseQuartersCarnage
    );
    assert_eq!(
        state.weapon_circuit_settings.active_config.primary_weapon,
        TournamentWeaponType::FlakCannon
    );

    // Apply MinimalistDuel1v1
    state.apply_weapon_circuit_archetype(WeaponCircuitArchetype::MinimalistDuel1v1);
    assert_eq!(
        state.weapon_circuit_settings.active_config.archetype,
        WeaponCircuitArchetype::MinimalistDuel1v1
    );

    // Undo back to Gothic
    state.undo();
    assert_eq!(
        state.weapon_circuit_settings.active_config.archetype,
        WeaponCircuitArchetype::GothicCloseQuartersCarnage
    );

    // Redo to MinimalistDuel1v1
    state.redo();
    assert_eq!(
        state.weapon_circuit_settings.active_config.archetype,
        WeaponCircuitArchetype::MinimalistDuel1v1
    );
}

#[test]
fn test_weapon_circuit_metrics_and_sprint_matrix() {
    let mut state = EditorState::default();

    state.apply_weapon_circuit_archetype(WeaponCircuitArchetype::Deck16DuelMasterCircuit);
    let metrics = &state.weapon_circuit_settings.cached_metrics;

    assert_eq!(metrics.total_weapons_in_level, 1);
    assert_eq!(metrics.total_ammo_boxes_in_level, 2);
    assert_eq!(metrics.ammo_pairing_coverage_pct, 100.0);
    assert!(metrics.total_max_burst_damage_potential >= 200.0);
}

#[test]
fn test_weapon_circuit_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Initial Deck16 has no critical hazards
    state.recalculate_weapon_circuit_preview();
    assert!(!state.weapon_circuit_settings.cached_hazards.iter().any(|h| h.is_critical));

    // 2. Set Redeemer -> triggers devastating weapon advisory
    state.weapon_circuit_settings.active_config.primary_weapon = TournamentWeaponType::Redeemer;
    state.recalculate_weapon_circuit_preview();
    assert!(state.weapon_circuit_settings.cached_hazards.iter().any(|h| h.title.contains("Nuclear Devastation")));

    // 3. Set flanking ammo to 0 -> triggers orphaned weapon warning
    state.weapon_circuit_settings.active_config.flanking_ammo_count = 0;
    state.recalculate_weapon_circuit_preview();
    assert!(state.weapon_circuit_settings.cached_hazards.iter().any(|h| h.title.contains("Orphaned Weapon")));
}

#[test]
fn test_commit_weapon_station_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Shock Rifle with 2 ammo boxes
    state.apply_weapon_circuit_archetype(WeaponCircuitArchetype::Deck16DuelMasterCircuit);
    let (brush_idx, actor_indices) = state.commit_weapon_station_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert_eq!(actor_indices.len(), 3); // 1 Weapon + 2 Ammo

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors[actor_indices[0]].class_name, "Botpack.ShockRifle");
    assert_eq!(level.actors[actor_indices[1]].class_name, "Botpack.ShockCore");
    assert_eq!(level.actors[actor_indices[2]].class_name, "Botpack.ShockCore");

    // Undo commit
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    let level_undone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_undone.actors.len(), 0);

    // Redo commit
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    let level_redone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_redone.actors.len(), 3);

    // 2. Test Dual Enforcers akimbo commit (adds 2 weapon actors + ammo)
    let mut state2 = EditorState::default();
    state2.weapon_circuit_settings.active_config.primary_weapon = TournamentWeaponType::DualEnforcers;
    state2.weapon_circuit_settings.active_config.flanking_ammo_count = 1;
    let (_, actors2) = state2.commit_weapon_station_to_world();
    assert_eq!(actors2.len(), 3); // 2 Enforcers + 1 EClip
    let lvl2 = state2.loaded_unr.as_ref().unwrap();
    assert_eq!(lvl2.actors[actors2[0]].class_name, "Botpack.Enforcer");
    assert_eq!(lvl2.actors[actors2[1]].class_name, "Botpack.Enforcer");
    assert_eq!(lvl2.actors[actors2[2]].class_name, "Botpack.EClip");
}
