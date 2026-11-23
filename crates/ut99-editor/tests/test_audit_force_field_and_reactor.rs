// crates/ut99-editor/tests/test_audit_force_field_and_reactor.rs
//
// Master Cross-System Audit Suite:
// - Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio
// - Interactive Glass Skylight, Observation Dome & Structural Truss Studio
// - Interactive Ventilation Duct, Crawlspace Shaft & Infiltration Grate Studio
// - Interactive Industrial Steam Piston, Hydraulic Compactor & Trap Hazard Studio
// - Interactive Jump Boots Cache, Anti-Gravity Booster & High-Vault Studio
// - Interactive Force Field, Energy Barrier & Power Core Overload Studio
// - Interactive Rotating Gyroscope, Reactor Core & Kinetic Ring Studio

use glam::Vec3;
use ut99_bsp::{
    ForceFieldArchetype, JumpBootsArchetype, PistonArchetype, ReactorCoreArchetype,
    SkylightArchetype, SupplyLockerArchetype, VentDuctArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_cross_system_force_field_and_reactor_commit() {
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

    // 6. Commit Force Field: 1 field brush + 2 pylon brushes = 3 brushes
    state.force_field_settings.active_config = ForceFieldArchetype::AssaultBulkheadForceField.to_config();
    state.force_field_settings.active_config.position = Vec3::new(1600.0, 100.0, 0.0);
    state.force_field_settings.active_config.has_emitter_pylons = true;
    let (field_brush, pylon_brushes, force_act) = state.commit_force_field_to_world();

    // 7. Commit Reactor Core: 1 core brush + 2 ring brushes = 3 brushes
    state.reactor_core_settings.active_config = ReactorCoreArchetype::LiandriCentrifugalCore.to_config();
    state.reactor_core_settings.active_config.position = Vec3::new(2000.0, 100.0, 0.0);
    let (core_brush, ring_brushes, reactor_act) = state.commit_reactor_core_to_world();

    // Total brushes added:
    // Supply Locker (1) + Skylight (1) + Vent Duct (1) + Piston (1) + Jump Boots (1) + Force Field (3) + Reactor Core (3) = 11 brushes
    assert_eq!(state.world_brushes.len(), initial_brushes + 11);
    assert_eq!(locker_brush, initial_brushes);
    assert_eq!(skylight_brush, initial_brushes + 1);
    assert_eq!(duct_brush, initial_brushes + 2);
    assert_eq!(piston_brush, initial_brushes + 3);
    assert_eq!(boots_brush, initial_brushes + 4);
    assert_eq!(field_brush, initial_brushes + 5);
    assert_eq!(pylon_brushes.len(), 2);
    assert_eq!(core_brush, initial_brushes + 8);
    assert_eq!(ring_brushes.len(), 2);

    // Total actors: 7 actors
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 7);
    assert_eq!(locker_act, Some(0));
    assert_eq!(skylight_act, Some(1));
    assert_eq!(duct_act, Some(2));
    assert_eq!(piston_act, Some(3));
    assert_eq!(boots_act, Some(4));
    assert_eq!(force_act, Some(5));
    assert_eq!(reactor_act, Some(6));

    // Verify class names
    assert_eq!(level.actors[0].class_name, "Botpack.UT_Eightball");
    assert_eq!(level.actors[1].class_name, "Engine.Light");
    assert_eq!(level.actors[2].class_name, "Engine.Mover");
    assert_eq!(level.actors[3].class_name, "Engine.Mover");
    assert_eq!(level.actors[4].class_name, "Botpack.UT_JumpBoots");
    assert_eq!(level.actors[5].class_name, "Engine.Trigger");
    assert_eq!(level.actors[6].class_name, "Engine.RotatingMover");
}

#[test]
fn test_cross_system_seven_stage_cascading_undo_redo_purity() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit all 7 setups
    state.commit_supply_locker_to_world();
    state.commit_skylight_to_world();
    state.commit_vent_duct_to_world();
    state.commit_piston_to_world();
    state.commit_jump_boots_to_world();
    state.commit_force_field_to_world();
    state.commit_reactor_core_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 11);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 7);

    // Undo 7: Undo Reactor Core
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 8);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 6);

    // Undo 6: Undo Force Field
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 5);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 5);

    // Undo 5: Undo Jump Boots
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 4);

    // Undo 4: Undo Piston
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);

    // Undo 3: Undo Vent Duct
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Undo 2: Undo Skylight
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Undo 1: Undo Supply Locker -> Back to initial state
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // Redo 1 through 7
    for _ in 0..7 {
        state.redo();
    }

    assert_eq!(state.world_brushes.len(), initial_brushes + 11);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 7);
}
