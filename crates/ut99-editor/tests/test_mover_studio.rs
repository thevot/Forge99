//! Automated Integration Tests for Interactive Mover Encroachment, Door Sequencing & Crushing Trap Studio

use glam::Vec3;
use ut99_bsp::{MoverEncroachType, MoverTemplateArchetype, MoverTriggerType};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_mover_templates_and_timeline_scrubbing() {
    let mut state = EditorState::default();

    // Verify initial default template (Sliding Door)
    assert_eq!(state.mover_studio_settings.selected_template, MoverTemplateArchetype::AutomaticSlidingDoor);
    assert_eq!(state.mover_studio_settings.active_config.move_time, 1.5);
    assert_eq!(state.mover_studio_settings.active_config.stay_open_time, 3.0);
    assert_eq!(state.mover_studio_settings.active_config.encroach_type, MoverEncroachType::ReturnWhenEncroach);

    // 1. Apply VerticalBlastBulkhead template
    state.apply_mover_template(MoverTemplateArchetype::VerticalBlastBulkhead);
    assert_eq!(state.mover_studio_settings.selected_template, MoverTemplateArchetype::VerticalBlastBulkhead);
    assert_eq!(state.mover_studio_settings.active_config.move_time, 2.2);
    assert_eq!(state.mover_studio_settings.active_config.encroach_type, MoverEncroachType::StopWhenEncroach);
    assert_eq!(state.mover_studio_settings.active_config.trigger_type, MoverTriggerType::TriggerOpenTimed);

    // 2. Advance simulation
    state.mover_studio_settings.playback_time = 0.0;
    state.step_mover_studio_simulation(1.0);
    assert!((state.mover_studio_settings.playback_time - 1.0).abs() < 1e-2);

    // 3. Scrub timeline
    state.scrub_mover_timeline(2.5);
    assert!((state.mover_studio_settings.playback_time - 2.5).abs() < 1e-2);

    // 4. Test Undo template application
    state.undo();
    assert_eq!(state.mover_studio_settings.selected_template, MoverTemplateArchetype::AutomaticSlidingDoor);
    assert_eq!(state.mover_studio_settings.active_config.move_time, 1.5);

    // 5. Test Redo
    state.redo();
    assert_eq!(state.mover_studio_settings.selected_template, MoverTemplateArchetype::VerticalBlastBulkhead);
    assert_eq!(state.mover_studio_settings.active_config.move_time, 2.2);
}

#[test]
fn test_mover_hazard_audit_and_pinch_detection() {
    let mut state = EditorState::default();

    // Set dangerously small MoveTime and empty tag with trigger mode
    state.mover_studio_settings.active_config.move_time = 0.03;
    state.mover_studio_settings.active_config.trigger_type = MoverTriggerType::TriggerOpenTimed;
    state.mover_studio_settings.active_config.mover_tag = "  ".to_string();

    state.audit_active_mover_hazards();

    let hazards = &state.mover_studio_settings.cached_hazards;
    assert!(hazards.iter().any(|h| h.hazard_type == "SubsecondGlitchSpeed"));
    assert!(hazards.iter().any(|h| h.hazard_type == "UncallableMoverNoTag"));
}

#[test]
fn test_bot_encroachment_simulation_and_crushing() {
    let mut state = EditorState::default();

    // Apply lethal crushing trap template
    state.apply_mover_template(MoverTemplateArchetype::HydraulicCrusherTrap);
    assert_eq!(state.mover_studio_settings.active_config.encroach_type, MoverEncroachType::CrushWhenEncroach);
    assert!(state.mover_studio_settings.active_config.crush_damage >= 500.0);

    // Place test bot directly in the downward travel path
    state.mover_studio_settings.bot_test_position = Vec3::new(0.0, 0.0, -100.0);
    state.simulate_active_mover_bot_encroachment();

    let sim = state.mover_studio_settings.cached_encroach_sim.as_ref().expect("Bot in path must register encroachment");
    assert!(sim.damage_dealt >= 500.0, "Crusher trap must deal lethal crushing damage");
    assert!(sim.mover_reaction.contains("Crushing"), "Reaction must be crushing");
}

#[test]
fn test_proximity_trigger_wiring_and_undo_redo_invariance() {
    let mut state = EditorState::default();
    state.mover_studio_settings.active_config.mover_name = "BlastGateA".to_string();
    state.mover_studio_settings.active_config.mover_tag = "GateA_Event".to_string();
    state.mover_studio_settings.trigger_approach_distance = 120.0;
    state.mover_studio_settings.trigger_radius = 80.0;

    let initial_actor_count = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    // 1. Wire proximity trigger
    let ok = state.wire_proximity_trigger_for_active_mover();
    assert!(ok);

    let actor_count_after = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap();
    assert_eq!(actor_count_after, initial_actor_count + 1);

    let added_trigger = &state.loaded_unr.as_ref().unwrap().actors[initial_actor_count];
    assert_eq!(added_trigger.class_name, "Trigger");
    assert_eq!(added_trigger.name, "Trigger_BlastGateA");
    assert_eq!(added_trigger.event, "GateA_Event");
    assert_eq!(added_trigger.location, Vec3::new(120.0, 0.0, 0.0));

    // 2. Test Undo
    state.undo();
    let count_after_undo = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap();
    assert_eq!(count_after_undo, initial_actor_count, "Undo must remove generated Trigger actor");

    // 3. Test Redo
    state.redo();
    let count_after_redo = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap();
    assert_eq!(count_after_redo, initial_actor_count + 1, "Redo must restore generated Trigger actor");
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[initial_actor_count].name, "Trigger_BlastGateA");
}
