// crates/ut99-editor/tests/test_supply_locker_studio.rs
//
// Integration tests for Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio.

use ut99_bsp::SupplyLockerArchetype;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_supply_locker_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is LiandriStandardHeavyArmory
    assert_eq!(
        state.supply_locker_settings.active_config.archetype,
        SupplyLockerArchetype::LiandriStandardHeavyArmory
    );
    assert_eq!(state.supply_locker_settings.active_config.locker_width_uu, 96.0);
    assert_eq!(state.supply_locker_settings.active_config.primary_weapon_class, "Botpack.UT_Eightball");

    // Apply ShockCoreEnergyDepot
    state.apply_supply_locker_archetype(SupplyLockerArchetype::ShockCoreEnergyDepot);
    assert_eq!(
        state.supply_locker_settings.active_config.archetype,
        SupplyLockerArchetype::ShockCoreEnergyDepot
    );
    assert_eq!(state.supply_locker_settings.active_config.primary_weapon_class, "Botpack.ShockRifle");
    assert_eq!(state.supply_locker_settings.active_config.ammo_pack_count, 3);

    // Apply BioHazardToxicCache
    state.apply_supply_locker_archetype(SupplyLockerArchetype::BioHazardToxicCache);
    assert_eq!(
        state.supply_locker_settings.active_config.archetype,
        SupplyLockerArchetype::BioHazardToxicCache
    );
    assert_eq!(state.supply_locker_settings.active_config.primary_weapon_class, "Botpack.UT_BioRifle");

    // Undo back to ShockCoreEnergyDepot
    state.undo();
    assert_eq!(
        state.supply_locker_settings.active_config.archetype,
        SupplyLockerArchetype::ShockCoreEnergyDepot
    );

    // Redo to BioHazardToxicCache
    state.redo();
    assert_eq!(
        state.supply_locker_settings.active_config.archetype,
        SupplyLockerArchetype::BioHazardToxicCache
    );
}

#[test]
fn test_supply_locker_metrics_and_firepower() {
    let mut state = EditorState::default();

    state.apply_supply_locker_archetype(SupplyLockerArchetype::LiandriStandardHeavyArmory);
    state.recalculate_supply_locker_preview();

    let metrics = &state.supply_locker_settings.cached_metrics;
    assert_eq!(metrics.shelf_usable_area_sq_uu, 96.0 * 32.0 * 3.0);
    assert_eq!(metrics.total_contained_firepower_rating, 50.0 + 2.0 * 15.0); // 80.0
}

#[test]
fn test_supply_locker_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Depleted Empty Locker
    state.supply_locker_settings.active_config.primary_weapon_class = "".to_string();
    state.supply_locker_settings.active_config.ammo_pack_count = 0;
    state.recalculate_supply_locker_preview();
    assert!(state.supply_locker_settings.cached_hazards.iter().any(|h| h.title.contains("Depleted Empty Supply Locker")));

    // 2. Shallow Locker Depth (< 16 uu)
    state.supply_locker_settings.active_config.locker_depth_uu = 12.0;
    state.recalculate_supply_locker_preview();
    assert!(state.supply_locker_settings.cached_hazards.iter().any(|h| h.title.contains("Shallow Locker Depth")));
}

#[test]
fn test_commit_supply_locker_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Locker Brush & Weapon Actor to World
    state.apply_supply_locker_archetype(SupplyLockerArchetype::LiandriStandardHeavyArmory);
    let (brush_idx, weapon_idx) = state.commit_supply_locker_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(weapon_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Botpack.UT_Eightball");

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
    assert_eq!(level_redone.actors[0].class_name, "Botpack.UT_Eightball");
}
