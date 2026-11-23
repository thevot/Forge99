// crates/ut99-editor/tests/test_hazard_chamber_studio.rs
//
// Integration tests for Environmental Hazard Chamber, Pressure Airlock & Gas Decompression Studio.

use glam::Vec3;
use ut99_bsp::{ChamberDoorStyle, HazardChamberArchetype, HazardDamageType};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_hazard_chamber_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is HighVacuumDecompressionChamber
    assert_eq!(
        state.hazard_chamber_settings.active_config.archetype,
        HazardChamberArchetype::HighVacuumDecompressionChamber
    );
    assert_eq!(state.hazard_chamber_settings.active_config.damage_type, HazardDamageType::Asphyxiated);
    assert!(state.hazard_chamber_settings.active_config.b_kill_zone);

    // Apply BiohazardToxicGasIncinerator
    state.apply_hazard_chamber_archetype(HazardChamberArchetype::BiohazardToxicGasIncinerator);
    assert_eq!(
        state.hazard_chamber_settings.active_config.archetype,
        HazardChamberArchetype::BiohazardToxicGasIncinerator
    );
    assert_eq!(state.hazard_chamber_settings.active_config.door_style, ChamberDoorStyle::VerticalBlastDrop);
    assert_eq!(state.hazard_chamber_settings.active_config.damage_type, HazardDamageType::Corroded);

    // Apply SubmergedHydrostaticFloodLock
    state.apply_hazard_chamber_archetype(HazardChamberArchetype::SubmergedHydrostaticFloodLock);
    assert_eq!(
        state.hazard_chamber_settings.active_config.archetype,
        HazardChamberArchetype::SubmergedHydrostaticFloodLock
    );
    assert_eq!(state.hazard_chamber_settings.active_config.damage_type, HazardDamageType::Drowned);

    // Apply CryogenicFreezeVault
    state.apply_hazard_chamber_archetype(HazardChamberArchetype::CryogenicFreezeVault);
    assert_eq!(
        state.hazard_chamber_settings.active_config.archetype,
        HazardChamberArchetype::CryogenicFreezeVault
    );
    assert_eq!(state.hazard_chamber_settings.active_config.damage_type, HazardDamageType::Frozen);

    // Apply HighVoltageArcFurnace
    state.apply_hazard_chamber_archetype(HazardChamberArchetype::HighVoltageArcFurnace);
    assert_eq!(
        state.hazard_chamber_settings.active_config.archetype,
        HazardChamberArchetype::HighVoltageArcFurnace
    );
    assert_eq!(state.hazard_chamber_settings.active_config.damage_type, HazardDamageType::Electrocuted);

    // Undo returns to CryogenicFreezeVault
    state.undo();
    assert_eq!(
        state.hazard_chamber_settings.active_config.archetype,
        HazardChamberArchetype::CryogenicFreezeVault
    );

    // Redo returns to HighVoltageArcFurnace
    state.redo();
    assert_eq!(
        state.hazard_chamber_settings.active_config.archetype,
        HazardChamberArchetype::HighVoltageArcFurnace
    );
}

#[test]
fn test_chamber_metrics_and_time_scrubbing() {
    let mut state = EditorState::default();

    state.apply_hazard_chamber_archetype(HazardChamberArchetype::HighVacuumDecompressionChamber);

    let m = &state.hazard_chamber_settings.cached_metrics;
    assert_eq!(m.chamber_volume_uu3, 512.0 * 512.0 * 256.0);
    assert!(m.is_instant_death);
    assert!(m.total_cycle_time_secs >= 7.0);

    // Scrub to 0.5s -> Door delay stage
    state.scrub_hazard_chamber_time(0.5);
    assert_eq!(state.hazard_chamber_settings.simulated_time_seconds, 0.5);

    // Scrub to 3.0s -> Sealed venting stage
    state.scrub_hazard_chamber_time(3.0);
    assert_eq!(state.hazard_chamber_settings.simulated_time_seconds, 3.0);
}

#[test]
fn test_commit_hazard_chamber_to_world_undo_redo() {
    let mut state = EditorState::default();

    state.apply_hazard_chamber_archetype(HazardChamberArchetype::HighVacuumDecompressionChamber);

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    let (chamber_idx, door_l_idx, door_r_idx, win_idx, con_idx, zone_idx, trig_idx, waypoints) =
        state.commit_hazard_chamber_to_world();

    // 5 Brushes committed: Chamber + DoorLeft + DoorRight + WindowGlass + Console
    assert_eq!(chamber_idx, initial_world_brushes);
    assert_eq!(door_l_idx, initial_world_brushes + 1);
    assert_eq!(door_r_idx, initial_world_brushes + 2);
    assert_eq!(win_idx, Some(initial_world_brushes + 3));
    assert_eq!(con_idx, initial_world_brushes + 4);
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 5);

    // 5 Actors committed: ZoneInfo + Trigger + 3 PathNodes
    assert!(zone_idx.is_some());
    assert!(trig_idx.is_some());
    assert_eq!(waypoints.len(), 3);

    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors.len(), initial_actors + 5);

    let zone_actor = &unr.actors[zone_idx.unwrap()];
    assert_eq!(zone_actor.class_name, "Engine.ZoneInfo");

    let trig_actor = &unr.actors[trig_idx.unwrap()];
    assert_eq!(trig_actor.class_name, "Engine.Trigger");
    assert_eq!(trig_actor.event, "VentCycle_Trigger");

    // Undo removes all 5 brushes and all 5 actors
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // Redo restores all 5 brushes and all 5 actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 5);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors + 5);
}

#[test]
fn test_hazard_chamber_safety_audit() {
    let mut state = EditorState::default();

    // 1. Zero damage & non-killzone flags critical hazard
    state.hazard_chamber_settings.active_config.damage_per_sec = 0.0;
    state.hazard_chamber_settings.active_config.b_kill_zone = false;
    state.recalculate_hazard_chamber_preview();
    let has_zero_dmg = state.hazard_chamber_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Zero Hazard"));
    assert!(has_zero_dmg, "Must flag critical hazard for zero damage");

    // 2. Console button inside chamber flags critical hazard
    state.hazard_chamber_settings.active_config.damage_per_sec = 100.0;
    state.hazard_chamber_settings.active_config.console_offset = Vec3::new(50.0, 50.0, 0.0); // Inside 512x512 room!
    state.recalculate_hazard_chamber_preview();
    let has_trapped_con = state.hazard_chamber_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Console Activator Inside"));
    assert!(has_trapped_con, "Must flag critical hazard for console inside chamber");

    // 3. Empty event name flags critical hazard
    state.hazard_chamber_settings.active_config.console_offset = Vec3::new(320.0, 0.0, 0.0);
    state.hazard_chamber_settings.active_config.event_name = "".to_string();
    state.recalculate_hazard_chamber_preview();
    let has_unbound_evt = state.hazard_chamber_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Unbound Vent"));
    assert!(has_unbound_evt, "Must flag critical hazard for empty event name");
}
