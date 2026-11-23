// crates/ut99-editor/tests/test_pipe_conduit_studio.rs
//
// Integration tests for Industrial Pipe Network, Steam Valve & Conduit Junction Studio.

use ut99_bsp::PipeArchetype;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_pipe_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is HighPressureSteamMain
    assert_eq!(
        state.pipe_conduit_settings.active_config.archetype,
        PipeArchetype::HighPressureSteamMain
    );
    assert!(state.pipe_conduit_settings.active_config.steam_hazard_enabled);
    assert_eq!(state.pipe_conduit_settings.active_config.pipe_outer_radius, 32.0);

    // Apply BiohazardCoolantConduit
    state.apply_pipe_archetype(PipeArchetype::BiohazardCoolantConduit);
    assert_eq!(
        state.pipe_conduit_settings.active_config.archetype,
        PipeArchetype::BiohazardCoolantConduit
    );
    assert_eq!(state.pipe_conduit_settings.active_config.pipe_outer_radius, 24.0);

    // Apply SubterraneanSewerAqueduct
    state.apply_pipe_archetype(PipeArchetype::SubterraneanSewerAqueduct);
    assert_eq!(
        state.pipe_conduit_settings.active_config.archetype,
        PipeArchetype::SubterraneanSewerAqueduct
    );
    assert_eq!(state.pipe_conduit_settings.active_config.pipe_outer_radius, 48.0);
    assert!(!state.pipe_conduit_settings.active_config.has_elbow_bend);

    // Apply CryogenicFuelTransferLine
    state.apply_pipe_archetype(PipeArchetype::CryogenicFuelTransferLine);
    assert_eq!(
        state.pipe_conduit_settings.active_config.archetype,
        PipeArchetype::CryogenicFuelTransferLine
    );

    // Apply HydraulicControlBundle
    state.apply_pipe_archetype(PipeArchetype::HydraulicControlBundle);
    assert_eq!(
        state.pipe_conduit_settings.active_config.archetype,
        PipeArchetype::HydraulicControlBundle
    );

    // Undo returns to CryogenicFuelTransferLine
    state.undo();
    assert_eq!(
        state.pipe_conduit_settings.active_config.archetype,
        PipeArchetype::CryogenicFuelTransferLine
    );

    // Redo returns to HydraulicControlBundle
    state.redo();
    assert_eq!(
        state.pipe_conduit_settings.active_config.archetype,
        PipeArchetype::HydraulicControlBundle
    );
}

#[test]
fn test_pipe_metrics_and_valve_scrubbing() {
    let mut state = EditorState::default();

    state.apply_pipe_archetype(PipeArchetype::HighPressureSteamMain);

    let m_closed = &state.pipe_conduit_settings.cached_metrics;
    assert!(m_closed.total_pipe_length_uu > 512.0);
    assert!(m_closed.total_fluid_volume_liters > 10.0);
    assert_eq!(state.pipe_conduit_settings.active_config.valve_open_fraction, 0.0);

    // Scrub valve to 50%
    state.scrub_pipe_valve_position(0.5);
    assert_eq!(state.pipe_conduit_settings.active_config.valve_open_fraction, 0.5);
    let m_half = &state.pipe_conduit_settings.cached_metrics;
    assert!(m_half.flow_velocity_uu_s >= 200.0);

    // Scrub valve to 100%
    state.scrub_pipe_valve_position(1.0);
    assert_eq!(state.pipe_conduit_settings.active_config.valve_open_fraction, 1.0);
    let m_full = &state.pipe_conduit_settings.cached_metrics;
    assert!(m_full.flow_velocity_uu_s >= 400.0);
}

#[test]
fn test_commit_pipe_conduit_to_world_undo_redo() {
    let mut state = EditorState::default();

    state.apply_pipe_archetype(PipeArchetype::HighPressureSteamMain);

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    let (straight_idx, elbow_idx, flange_indices, collar_idx, valve_body_idx, valve_wheel_idx, mover_idx, trig_idx) =
        state.commit_pipe_conduit_to_world();

    // Check brushes
    assert_eq!(straight_idx, initial_world_brushes);
    assert!(elbow_idx.is_some());
    assert!(!flange_indices.is_empty());
    assert!(collar_idx.is_some());
    assert!(valve_body_idx.is_some());
    assert!(valve_wheel_idx.is_some());

    let total_new_brushes = 1 + (if elbow_idx.is_some() { 1 } else { 0 }) + flange_indices.len() + 1 + 1 + 1;
    assert_eq!(state.world_brushes.len(), initial_world_brushes + total_new_brushes);

    // Check actors: 1 Mover (handwheel) + 1 Trigger (steam vent)
    assert!(mover_idx.is_some());
    assert!(trig_idx.is_some());
    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors.len(), initial_actors + 2);

    let mover_actor = &unr.actors[mover_idx.unwrap()];
    assert_eq!(mover_actor.class_name, "Engine.Mover");

    let trig_actor = &unr.actors[trig_idx.unwrap()];
    assert_eq!(trig_actor.class_name, "Engine.Trigger");
    assert_eq!(trig_actor.event, "SteamValve_Open");

    // Undo removes all brushes and actors
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // Redo restores all brushes and actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + total_new_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors + 2);
}

#[test]
fn test_pipe_safety_audit() {
    let mut state = EditorState::default();

    // 1. Severe bend pinch flags critical hazard
    state.pipe_conduit_settings.active_config.has_elbow_bend = true;
    state.pipe_conduit_settings.active_config.elbow_radius_uu = 15.0; // Pipe radius is 32 -> diameter 64!
    state.recalculate_pipe_preview();
    let has_pinch = state.pipe_conduit_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Severe Elbow Bend Pinch"));
    assert!(has_pinch, "Must flag critical hazard for elbow bend pinch");

    // 2. Degenerate facets flags critical hazard
    state.pipe_conduit_settings.active_config.elbow_radius_uu = 96.0;
    state.pipe_conduit_settings.active_config.cylinder_facets = 4;
    state.recalculate_pipe_preview();
    let has_degen = state.pipe_conduit_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Degenerate Cylinder Facets"));
    assert!(has_degen, "Must flag critical hazard for facets < 6");

    // 3. Steam hazard with 0 damage flags warning
    state.pipe_conduit_settings.active_config.cylinder_facets = 8;
    state.pipe_conduit_settings.active_config.steam_hazard_enabled = true;
    state.pipe_conduit_settings.active_config.scald_damage_per_sec = 0.0;
    state.recalculate_pipe_preview();
    let has_harmless = state.pipe_conduit_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Harmless Steam Relief Vent"));
    assert!(has_harmless, "Must flag warning for 0 scald damage");

    // 4. Empty event name flags warning
    state.pipe_conduit_settings.active_config.event_name = "".to_string();
    state.recalculate_pipe_preview();
    let has_unbound = state.pipe_conduit_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Unbound Valve Trigger Event"));
    assert!(has_unbound, "Must flag warning for empty event name");
}
