// crates/ut99-editor/tests/test_audit_traps_triggers_and_transits.rs
//
// Master Audit Suite for Industrial Traps, Logic Triggers, Vertical Transit & Zone Fluids:
// - Bio-Rifle Slime Trap & Volumetric Hazard Field Studio
// - Pressure Plate & Proximity Trigger Network Studio
// - Elevator, LiftCenter/LiftExit & Vertical Transit Studio
// - Dynamic Zone Velocity, Atmospheric Convection & Low-G Space Vacuum Studio

use glam::Vec3;
use ut99_bsp::{
    BioTrapArchetype, ElevatorArchetype, TriggerNetworkArchetype, ZoneVelocityArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_cross_system_traps_triggers_elevators_and_fluids_commit() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Bio Trap
    state.bio_trap_settings.active_config = BioTrapArchetype::ReactorVatSpillZone.to_config();
    state.bio_trap_settings.active_config.position = Vec3::new(100.0, 100.0, 0.0);
    let (bio_brush, bio_act) = state.commit_bio_trap_to_world();

    // 2. Commit Trigger Network
    state.trigger_network_settings.active_config = TriggerNetworkArchetype::TrapdoorHiddenPitTrigger.to_config();
    state.trigger_network_settings.active_config.position = Vec3::new(300.0, 100.0, 0.0);
    let (trig_brush, trig_act) = state.commit_trigger_network_to_world();

    // 3. Commit Elevator & Bot Waypoints
    state.elevator_settings.active_config = ElevatorArchetype::StandardHydraulicLift.to_config();
    state.elevator_settings.active_config.position = Vec3::new(600.0, 100.0, 0.0);
    let (elev_brush, center_act, exit_acts) = state.commit_elevator_to_world();

    // 4. Commit Zone Velocity & Portal Sheet
    state.zone_velocity_settings.active_config = ZoneVelocityArchetype::HydroelectricRiverCurrent.to_config();
    state.zone_velocity_settings.active_config.position = Vec3::new(900.0, 100.0, 0.0);
    let (zone_brush, zone_act) = state.commit_zone_velocity_to_world();

    // Verification of brushes: exactly 4 new brushes added atop initial brushes
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(bio_brush, initial_brushes);
    assert_eq!(trig_brush, initial_brushes + 1);
    assert_eq!(elev_brush, initial_brushes + 2);
    assert_eq!(zone_brush, initial_brushes + 3);

    // Verification of actors
    let level = state.loaded_unr.as_ref().unwrap();
    // bio: 1 (BioGel) + trig: 1 (Trigger) + elev: 1 (Center) + 2 (Exits) + zone: 1 (WaterZone) = 6 actors
    assert_eq!(level.actors.len(), 6);
    assert_eq!(bio_act, Some(0));
    assert_eq!(trig_act, Some(1));
    assert_eq!(center_act, Some(2));
    assert_eq!(exit_acts, vec![3, 4]);
    assert_eq!(zone_act, Some(5));

    // Verify class names
    assert_eq!(level.actors[0].class_name, "Botpack.BioGel");
    assert_eq!(level.actors[1].class_name, "Engine.Trigger");
    assert_eq!(level.actors[2].class_name, "Botpack.LiftCenter");
    assert_eq!(level.actors[3].class_name, "Botpack.LiftExit");
    assert_eq!(level.actors[4].class_name, "Botpack.LiftExit");
    assert_eq!(level.actors[5].class_name, "Botpack.WaterZone");
}

#[test]
fn test_cascading_four_stage_multi_undo_redo_purity() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // Stage 1: Bio Trap
    state.bio_trap_settings.active_config = BioTrapArchetype::ReactorVatSpillZone.to_config();
    state.commit_bio_trap_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Stage 2: Trigger Network
    state.trigger_network_settings.active_config = TriggerNetworkArchetype::TrapdoorHiddenPitTrigger.to_config();
    state.commit_trigger_network_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Stage 3: Elevator
    state.elevator_settings.active_config = ElevatorArchetype::StandardHydraulicLift.to_config();
    state.commit_elevator_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 5);

    // Stage 4: Zone Velocity
    state.zone_velocity_settings.active_config = ZoneVelocityArchetype::HydroelectricRiverCurrent.to_config();
    state.commit_zone_velocity_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 6);

    // --- Cascade Undo in Reverse Order ---

    // Undo Stage 4 (Zone Velocity)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 5);

    // Undo Stage 3 (Elevator)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Undo Stage 2 (Trigger Network)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Undo Stage 1 (Bio Trap)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // --- Cascade Redo in Forward Order ---

    // Redo Stage 1 (Bio Trap)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Redo Stage 2 (Trigger Network)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Redo Stage 3 (Elevator)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 5);

    // Redo Stage 4 (Zone Velocity)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 6);
}

#[test]
fn test_integrated_vertical_transit_and_fluid_convection_kinematics() {
    let mut state = EditorState::default();

    // 1. Elevator Kinematics
    state.elevator_settings.active_config.travel_height_uu = 500.0;
    state.elevator_settings.active_config.move_time_secs = 2.0;
    state.recalculate_elevator_preview();
    assert_eq!(state.elevator_settings.cached_metrics.ascent_velocity_uu_per_sec, 250.0);

    // 2. Zone Velocity Kinematics
    state.zone_velocity_settings.active_config.zone_velocity = Vec3::new(0.0, 0.0, 950.0); // Exact 100% buoyancy against gravity
    state.zone_velocity_settings.active_config.zone_length_uu = 1900.0;
    state.recalculate_zone_velocity_preview();
    assert_eq!(state.zone_velocity_settings.cached_metrics.effective_velocity_magnitude, 950.0);
    assert_eq!(state.zone_velocity_settings.cached_metrics.drift_transit_time_secs, 2.0);
    assert_eq!(state.zone_velocity_settings.cached_metrics.buoyancy_rating_pct, 100.0);
}
