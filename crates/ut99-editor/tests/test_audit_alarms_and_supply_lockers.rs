// crates/ut99-editor/tests/test_audit_alarms_and_supply_lockers.rs
//
// Master Audit Suite for Emergency Alarms & Tournament Supply Lockers:
// - Interactive Toxic Slime Acid Basin, Lava Flow & Caustic Pool Studio
// - Interactive Alarm Siren Beacon, Emergency Strobe & Warning Klaxon Studio
// - Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio

use glam::Vec3;
use ut99_bsp::{
    AlarmBeaconArchetype, FluidBasinArchetype, SupplyLockerArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_cross_system_alarms_supply_lockers_and_fluid_basins_commit() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Fluid Basin
    state.fluid_basin_settings.active_config = FluidBasinArchetype::CorrosiveDeckSlimeVat.to_config();
    state.fluid_basin_settings.active_config.position = Vec3::new(100.0, 100.0, 0.0);
    let (basin_brush, basin_act) = state.commit_fluid_basin_to_world();

    // 2. Commit Alarm Beacon
    state.alarm_beacon_settings.active_config = AlarmBeaconArchetype::NuclearReactorBreachStrobe.to_config();
    state.alarm_beacon_settings.active_config.position = Vec3::new(400.0, 100.0, 0.0);
    let (alarm_brush, alarm_act) = state.commit_alarm_beacon_to_world();

    // 3. Commit Supply Locker
    state.supply_locker_settings.active_config = SupplyLockerArchetype::LiandriStandardHeavyArmory.to_config();
    state.supply_locker_settings.active_config.position = Vec3::new(700.0, 100.0, 0.0);
    let (locker_brush, locker_act) = state.commit_supply_locker_to_world();

    // Verification of brushes: exactly 3 new brushes added atop initial brushes
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(basin_brush, initial_brushes);
    assert_eq!(alarm_brush, initial_brushes + 1);
    assert_eq!(locker_brush, initial_brushes + 2);

    // Verification of actors
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 3);
    assert_eq!(basin_act, Some(0));
    assert_eq!(alarm_act, Some(1));
    assert_eq!(locker_act, Some(2));

    // Verify class names
    assert_eq!(level.actors[0].class_name, "Botpack.PainZone");
    assert_eq!(level.actors[1].class_name, "Engine.Light");
    assert_eq!(level.actors[2].class_name, "Botpack.UT_Eightball");
}

#[test]
fn test_cascading_three_stage_multi_undo_redo_purity() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // Stage 1: Fluid Basin
    state.fluid_basin_settings.active_config = FluidBasinArchetype::CorrosiveDeckSlimeVat.to_config();
    state.commit_fluid_basin_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Stage 2: Alarm Beacon
    state.alarm_beacon_settings.active_config = AlarmBeaconArchetype::NuclearReactorBreachStrobe.to_config();
    state.commit_alarm_beacon_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Stage 3: Supply Locker
    state.supply_locker_settings.active_config = SupplyLockerArchetype::LiandriStandardHeavyArmory.to_config();
    state.commit_supply_locker_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);

    // --- Cascade Undo in Reverse Order ---

    // Undo Stage 3 (Supply Locker)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Undo Stage 2 (Alarm Beacon)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Undo Stage 1 (Fluid Basin)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // --- Cascade Redo in Forward Order ---

    // Redo Stage 1 (Fluid Basin)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Redo Stage 2 (Alarm Beacon)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Redo Stage 3 (Supply Locker)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);
}

#[test]
fn test_alarm_strobe_and_locker_firepower_kinematics() {
    let mut state = EditorState::default();

    // 1. Alarm Beacon Acoustics & Strobe Period
    state.alarm_beacon_settings.active_config.sound_volume_db = 90.0;
    state.alarm_beacon_settings.active_config.strobe_frequency_hz = 6.0;
    state.recalculate_alarm_beacon_preview();
    assert_eq!(state.alarm_beacon_settings.cached_metrics.acoustic_audible_radius_uu, 1800.0);
    assert_eq!(state.alarm_beacon_settings.cached_metrics.flash_cycle_duration_secs, 1.0 / 6.0);

    // 2. Supply Locker Contained Firepower
    state.supply_locker_settings.active_config.primary_weapon_class = "Botpack.UT_Eightball".to_string(); // 50 pts
    state.supply_locker_settings.active_config.ammo_pack_count = 2; // 2 * 15 = 30 pts
    state.recalculate_supply_locker_preview();
    assert_eq!(state.supply_locker_settings.cached_metrics.total_contained_firepower_rating, 80.0);
}
