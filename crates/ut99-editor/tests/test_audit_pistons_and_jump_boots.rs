// crates/ut99-editor/tests/test_audit_pistons_and_jump_boots.rs
//
// Master Cross-System Audit Suite:
// - Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio
// - Interactive Glass Skylight, Observation Dome & Structural Truss Studio
// - Interactive Ventilation Duct, Crawlspace Shaft & Infiltration Grate Studio
// - Interactive Industrial Steam Piston, Hydraulic Compactor & Trap Hazard Studio
// - Interactive Jump Boots Cache, Anti-Gravity Booster & High-Vault Studio

use glam::Vec3;
use ut99_bsp::{
    JumpBootsArchetype, PistonArchetype, SkylightArchetype, SupplyLockerArchetype, VentDuctArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_cross_system_pistons_and_jump_boots_commit() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Supply Locker
    state.supply_locker_settings.active_config = SupplyLockerArchetype::LiandriStandardHeavyArmory.to_config();
    state.supply_locker_settings.active_config.position = Vec3::new(100.0, 100.0, 0.0);
    let (locker_brush, locker_act) = state.commit_supply_locker_to_world();

    // 2. Commit Skylight
    state.skylight_settings.active_config = SkylightArchetype::CurvedObservationDome.to_config();
    state.skylight_settings.active_config.position = Vec3::new(400.0, 100.0, 200.0);
    let (skylight_brush, skylight_act) = state.commit_skylight_to_world();

    // 3. Commit Vent Duct
    state.vent_duct_settings.active_config = VentDuctArchetype::AssaultOverlordAirVentilationBypass.to_config();
    state.vent_duct_settings.active_config.position = Vec3::new(700.0, 100.0, 0.0);
    let (duct_brush, duct_act) = state.commit_vent_duct_to_world();

    // 4. Commit Piston
    state.piston_settings.active_config = PistonArchetype::PressureChamberVerticalCrusher.to_config();
    state.piston_settings.active_config.position = Vec3::new(1000.0, 100.0, 384.0);
    let (piston_brush, piston_act) = state.commit_piston_to_world();

    // 5. Commit Jump Boots
    state.jump_boots_settings.active_config = JumpBootsArchetype::PhobosLowGravityVault.to_config();
    state.jump_boots_settings.active_config.position = Vec3::new(1300.0, 100.0, 0.0);
    let (boots_brush, boots_act) = state.commit_jump_boots_to_world();

    // Verification of brushes: exactly 5 new brushes added atop initial brushes
    assert_eq!(state.world_brushes.len(), initial_brushes + 5);
    assert_eq!(locker_brush, initial_brushes);
    assert_eq!(skylight_brush, initial_brushes + 1);
    assert_eq!(duct_brush, initial_brushes + 2);
    assert_eq!(piston_brush, initial_brushes + 3);
    assert_eq!(boots_brush, initial_brushes + 4);

    // Verification of actors
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 5);
    assert_eq!(locker_act, Some(0));
    assert_eq!(skylight_act, Some(1));
    assert_eq!(duct_act, Some(2));
    assert_eq!(piston_act, Some(3));
    assert_eq!(boots_act, Some(4));

    // Verify class names
    assert_eq!(level.actors[0].class_name, "Botpack.UT_Eightball");
    assert_eq!(level.actors[1].class_name, "Engine.Light");
    assert_eq!(level.actors[2].class_name, "Engine.Mover");
    assert_eq!(level.actors[3].class_name, "Engine.Mover");
    assert_eq!(level.actors[4].class_name, "Botpack.UT_JumpBoots");
}

#[test]
fn test_cascading_five_stage_multi_undo_redo_purity() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // Stage 1: Supply Locker
    state.supply_locker_settings.active_config = SupplyLockerArchetype::LiandriStandardHeavyArmory.to_config();
    state.commit_supply_locker_to_world();

    // Stage 2: Skylight
    state.skylight_settings.active_config = SkylightArchetype::CurvedObservationDome.to_config();
    state.commit_skylight_to_world();

    // Stage 3: Vent Duct
    state.vent_duct_settings.active_config = VentDuctArchetype::AssaultOverlordAirVentilationBypass.to_config();
    state.commit_vent_duct_to_world();

    // Stage 4: Piston
    state.piston_settings.active_config = PistonArchetype::PressureChamberVerticalCrusher.to_config();
    state.commit_piston_to_world();

    // Stage 5: Jump Boots
    state.jump_boots_settings.active_config = JumpBootsArchetype::PhobosLowGravityVault.to_config();
    state.commit_jump_boots_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 5);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 5);

    // Cascading Undo 5 times
    state.undo(); // removes Jump Boots
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 4);

    state.undo(); // removes Piston
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);

    state.undo(); // removes Vent Duct
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    state.undo(); // removes Skylight
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    state.undo(); // removes Supply Locker
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // Cascading Redo 5 times
    state.redo(); // restores Supply Locker
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    state.redo(); // restores Skylight
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    state.redo(); // restores Vent Duct
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);

    state.redo(); // restores Piston
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 4);

    state.redo(); // restores Jump Boots
    assert_eq!(state.world_brushes.len(), initial_brushes + 5);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 5);
}
