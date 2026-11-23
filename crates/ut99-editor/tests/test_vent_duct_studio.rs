// crates/ut99-editor/tests/test_vent_duct_studio.rs
//
// Integration tests for Interactive Ventilation Duct, Crawlspace Shaft & Infiltration Grate Studio.

use ut99_bsp::VentDuctArchetype;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_vent_duct_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is AssaultOverlordAirVentilationBypass
    assert_eq!(
        state.vent_duct_settings.active_config.archetype,
        VentDuctArchetype::AssaultOverlordAirVentilationBypass
    );
    assert_eq!(state.vent_duct_settings.active_config.duct_length_uu, 384.0);
    assert_eq!(state.vent_duct_settings.active_config.air_draft_velocity, 150.0);

    // Apply SubterraneanMaintenanceDrainCrawlway
    state.apply_vent_duct_archetype(VentDuctArchetype::SubterraneanMaintenanceDrainCrawlway);
    assert_eq!(
        state.vent_duct_settings.active_config.archetype,
        VentDuctArchetype::SubterraneanMaintenanceDrainCrawlway
    );
    assert_eq!(state.vent_duct_settings.active_config.duct_length_uu, 512.0);
    assert!(!state.vent_duct_settings.active_config.has_exhaust_grate);

    // Apply HighTechTurbineCoolingChute
    state.apply_vent_duct_archetype(VentDuctArchetype::HighTechTurbineCoolingChute);
    assert_eq!(
        state.vent_duct_settings.active_config.archetype,
        VentDuctArchetype::HighTechTurbineCoolingChute
    );
    assert_eq!(state.vent_duct_settings.active_config.air_draft_velocity, 300.0);

    // Undo back to SubterraneanMaintenanceDrainCrawlway
    state.undo();
    assert_eq!(
        state.vent_duct_settings.active_config.archetype,
        VentDuctArchetype::SubterraneanMaintenanceDrainCrawlway
    );

    // Redo to HighTechTurbineCoolingChute
    state.redo();
    assert_eq!(
        state.vent_duct_settings.active_config.archetype,
        VentDuctArchetype::HighTechTurbineCoolingChute
    );
}

#[test]
fn test_vent_duct_metrics_and_clearance() {
    let mut state = EditorState::default();

    state.apply_vent_duct_archetype(VentDuctArchetype::AssaultOverlordAirVentilationBypass);
    state.recalculate_vent_duct_preview();

    let metrics = &state.vent_duct_settings.cached_metrics;
    assert_eq!(metrics.internal_clearance_cross_section_sq_uu, 48.0 * 40.0);
    assert_eq!(metrics.aerodynamic_convection_force, 150.0 * 1.5);
}

#[test]
fn test_vent_duct_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Insufficient player crouch clearance (< 40 uu internal height)
    state.vent_duct_settings.active_config.duct_height_uu = 40.0;
    state.vent_duct_settings.active_config.wall_thickness_uu = 8.0; // 40 - 16 = 24 uu < 40 uu
    state.recalculate_vent_duct_preview();
    assert!(state.vent_duct_settings.cached_hazards.iter().any(|h| h.title.contains("Insufficient Player Crouch Clearance")));

    // 2. Trapped dead-end duct
    state.vent_duct_settings.active_config.duct_height_uu = 56.0;
    state.vent_duct_settings.active_config.has_intake_grate = true;
    state.vent_duct_settings.active_config.has_exhaust_grate = true;
    state.vent_duct_settings.active_config.grate_breakable = false;
    state.vent_duct_settings.active_config.event = "".to_string();
    state.recalculate_vent_duct_preview();
    assert!(state.vent_duct_settings.cached_hazards.iter().any(|h| h.title.contains("Trapped Dead-End Ventilation Duct")));
}

#[test]
fn test_commit_vent_duct_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Vent Duct Brush & Mover Actor to World
    state.apply_vent_duct_archetype(VentDuctArchetype::AssaultOverlordAirVentilationBypass);
    let (brush_idx, mover_idx) = state.commit_vent_duct_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(mover_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Engine.Mover");

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
    assert_eq!(level_redone.actors[0].class_name, "Engine.Mover");
}
