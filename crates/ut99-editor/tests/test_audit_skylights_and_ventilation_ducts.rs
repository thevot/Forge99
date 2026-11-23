// crates/ut99-editor/tests/test_audit_skylights_and_ventilation_ducts.rs
//
// Master Audit Suite for Glazed Skylights & Infiltration Ventilation Ducts:
// - Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio
// - Interactive Glass Skylight, Observation Dome & Structural Truss Studio
// - Interactive Ventilation Duct, Crawlspace Shaft & Infiltration Grate Studio

use glam::Vec3;
use ut99_bsp::{
    SkylightArchetype, SupplyLockerArchetype, VentDuctArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_cross_system_skylights_vent_ducts_and_supply_lockers_commit() {
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

    // Verification of brushes: exactly 3 new brushes added atop initial brushes
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(locker_brush, initial_brushes);
    assert_eq!(skylight_brush, initial_brushes + 1);
    assert_eq!(duct_brush, initial_brushes + 2);

    // Verification of actors
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 3);
    assert_eq!(locker_act, Some(0));
    assert_eq!(skylight_act, Some(1));
    assert_eq!(duct_act, Some(2));

    // Verify class names
    assert_eq!(level.actors[0].class_name, "Botpack.UT_Eightball");
    assert_eq!(level.actors[1].class_name, "Engine.Light");
    assert_eq!(level.actors[2].class_name, "Engine.Mover");
}

#[test]
fn test_cascading_three_stage_multi_undo_redo_purity() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // Stage 1: Supply Locker
    state.supply_locker_settings.active_config = SupplyLockerArchetype::LiandriStandardHeavyArmory.to_config();
    state.commit_supply_locker_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Stage 2: Skylight
    state.skylight_settings.active_config = SkylightArchetype::CurvedObservationDome.to_config();
    state.commit_skylight_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Stage 3: Vent Duct
    state.vent_duct_settings.active_config = VentDuctArchetype::AssaultOverlordAirVentilationBypass.to_config();
    state.commit_vent_duct_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);

    // --- Cascade Undo in Reverse Order ---

    // Undo Stage 3 (Vent Duct)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Undo Stage 2 (Skylight)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Undo Stage 1 (Supply Locker)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // --- Cascade Redo in Forward Order ---

    // Redo Stage 1 (Supply Locker)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Redo Stage 2 (Skylight)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Redo Stage 3 (Vent Duct)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);
}

#[test]
fn test_skylight_solar_and_vent_duct_kinematics() {
    let mut state = EditorState::default();

    // 1. Skylight Solar Transmittance Rating
    state.skylight_settings.active_config.glass_transmittance_pct = 85.0;
    state.skylight_settings.active_config.sunlight_brightness = 220;
    state.recalculate_skylight_preview();
    assert_eq!(state.skylight_settings.cached_metrics.lux_solar_transmittance_rating, 0.85 * 220.0);

    // 2. Vent Duct Clearance & Convection
    state.vent_duct_settings.active_config.duct_width_uu = 64.0;
    state.vent_duct_settings.active_config.duct_height_uu = 56.0;
    state.vent_duct_settings.active_config.wall_thickness_uu = 8.0;
    state.vent_duct_settings.active_config.air_draft_velocity = 150.0;
    state.recalculate_vent_duct_preview();
    assert_eq!(state.vent_duct_settings.cached_metrics.internal_clearance_cross_section_sq_uu, 48.0 * 40.0);
    assert_eq!(state.vent_duct_settings.cached_metrics.aerodynamic_convection_force, 225.0);
}
