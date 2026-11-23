// crates/ut99-editor/tests/test_ventilator_studio.rs
//
// Integration tests for Interactive Environmental Ventilation Fan, Air Current & Wind Tunnel Studio.

use ut99_bsp::{AirFlowDirection, VentilatorArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_ventilator_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is IndustrialCeilingExhaust
    assert_eq!(
        state.ventilator_settings.active_config.archetype,
        VentilatorArchetype::IndustrialCeilingExhaust
    );

    // Apply HighSpeedIntakeTurbine
    state.apply_ventilator_archetype(VentilatorArchetype::HighSpeedIntakeTurbine);
    assert_eq!(
        state.ventilator_settings.active_config.archetype,
        VentilatorArchetype::HighSpeedIntakeTurbine
    );
    assert_eq!(state.ventilator_settings.active_config.geometry.blade_count, 8);
    assert_eq!(state.ventilator_settings.active_config.air_current.flow_direction, AirFlowDirection::Sucking);

    // Apply VerticalUpdraftLiftShaft
    state.apply_ventilator_archetype(VentilatorArchetype::VerticalUpdraftLiftShaft);
    assert_eq!(
        state.ventilator_settings.active_config.archetype,
        VentilatorArchetype::VerticalUpdraftLiftShaft
    );
    assert!(state.ventilator_settings.cached_lift.can_lift_player);

    // Apply BiohazardCoolingBlower
    state.apply_ventilator_archetype(VentilatorArchetype::BiohazardCoolingBlower);
    assert_eq!(
        state.ventilator_settings.active_config.archetype,
        VentilatorArchetype::BiohazardCoolingBlower
    );

    // Apply LethalGrinderVentilator
    state.apply_ventilator_archetype(VentilatorArchetype::LethalGrinderVentilator);
    assert_eq!(
        state.ventilator_settings.active_config.archetype,
        VentilatorArchetype::LethalGrinderVentilator
    );
    assert!(state.ventilator_settings.active_config.lethality.is_lethal_contact);

    // Undo returns to BiohazardCoolingBlower
    state.undo();
    assert_eq!(
        state.ventilator_settings.active_config.archetype,
        VentilatorArchetype::BiohazardCoolingBlower
    );

    // Redo returns to LethalGrinderVentilator
    state.redo();
    assert_eq!(
        state.ventilator_settings.active_config.archetype,
        VentilatorArchetype::LethalGrinderVentilator
    );
}

#[test]
fn test_aerodynamic_lift_and_time_scrubbing() {
    let mut state = EditorState::default();

    state.apply_ventilator_archetype(VentilatorArchetype::VerticalUpdraftLiftShaft);

    // Lift velocity is 1050 uu/s > 950 uu/s (UT99 gravity)
    assert!(state.ventilator_settings.cached_lift.can_lift_player);
    assert!(state.ventilator_settings.cached_lift.equilibrium_hover_height > 0.0);

    // Scrub time and ensure preview updates
    state.scrub_ventilator_time(1.5);
    assert_eq!(state.ventilator_settings.simulated_time_seconds, 1.5);
    assert!(!state.ventilator_settings.cached_streamlines.is_empty());

    // Streamlines must be oriented upwards (flow direction)
    let s0 = &state.ventilator_settings.cached_streamlines[0];
    assert!(s0.velocity.z > 0.0);
}

#[test]
fn test_commit_ventilator_to_world_undo_redo() {
    let mut state = EditorState::default();

    state.apply_ventilator_archetype(VentilatorArchetype::LethalGrinderVentilator);

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    let (fan_idx, house_idx, mover_idx, zone_idx, trig_idx) = state.commit_ventilator_to_world();

    assert_eq!(fan_idx, initial_world_brushes);
    assert_eq!(house_idx, initial_world_brushes + 1);
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 2);

    // Fan brush: Hub (8 sides + 2 caps = 10) + 3 blades * 5 faces = 25 polys
    let fan_brush = &state.world_brushes[fan_idx];
    assert_eq!(fan_brush.polys.len(), 25);
    assert_eq!(fan_brush.polys[0].flags & 0x00200000, 0x00200000); // PF_SEMISOLID

    // Housing brush: Outer ring 12 sides * 4 faces = 48 polys
    let house_brush = &state.world_brushes[house_idx];
    assert_eq!(house_brush.polys.len(), 48);

    // 3 actors created: RotatingMover, ZoneInfo, Trigger (lethal contact)
    assert!(mover_idx.is_some());
    assert!(zone_idx.is_some());
    assert!(trig_idx.is_some());

    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors.len(), initial_actors + 3);

    let mover_actor = &unr.actors[mover_idx.unwrap()];
    assert_eq!(mover_actor.class_name, "Engine.RotatingMover");

    let zone_actor = &unr.actors[zone_idx.unwrap()];
    assert_eq!(zone_actor.class_name, "Engine.ZoneInfo");

    let trig_actor = &unr.actors[trig_idx.unwrap()];
    assert_eq!(trig_actor.class_name, "Engine.Trigger");

    // Undo removes both brushes and all 3 actors
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // Redo restores both brushes and all 3 actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors + 3);
}

#[test]
fn test_ventilator_safety_audit() {
    let mut state = EditorState::default();

    // 1. Excessive air velocity (>2500 uu/s) triggers critical hazard
    state.ventilator_settings.active_config.air_current.zone_velocity_strength = 2800.0;
    state.recalculate_ventilator_preview();
    let has_excess = state.ventilator_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Excessive"));
    assert!(has_excess, "Must flag critical hazard for air velocity > 2500 uu/s");

    // 2. Truncated range (<64 uu) triggers critical hazard
    state.ventilator_settings.active_config.air_current.zone_velocity_strength = 500.0;
    state.ventilator_settings.active_config.air_current.effective_range = 32.0;
    state.recalculate_ventilator_preview();
    let has_trunc = state.ventilator_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Trunkated"));
    assert!(has_trunc, "Must flag critical hazard for range < 64 uu");

    // 3. High-RPM fan (300 RPM) without lethal contact triggers warning
    state.ventilator_settings.active_config.air_current.effective_range = 512.0;
    state.ventilator_settings.active_config.spin_rpm = 300.0;
    state.ventilator_settings.active_config.lethality.is_lethal_contact = false;
    state.recalculate_ventilator_preview();
    let has_unprotected = state.ventilator_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("High-Speed Fan"));
    assert!(has_unprotected, "Must flag warning hazard for high-speed fan without lethal contact");
}
