// crates/ut99-editor/tests/test_force_field_studio.rs
//
// Integration tests for Interactive Force Field, Energy Barrier & Power Core Overload Studio

use glam::Vec3;
use ut99_bsp::{
    audit_force_field_safety, evaluate_force_field_metrics, ForceFieldArchetype, ForceFieldConfig,
    ForceFieldPermeability,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_force_field_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    let initial_config = state.force_field_settings.active_config.clone();
    assert_eq!(
        initial_config.archetype,
        ForceFieldArchetype::AssaultBulkheadForceField
    );

    // Apply MothershipEnergyShield
    state.apply_force_field_archetype(ForceFieldArchetype::MothershipEnergyShield);
    assert_eq!(
        state.force_field_settings.active_config.archetype,
        ForceFieldArchetype::MothershipEnergyShield
    );
    assert_eq!(
        state.force_field_settings.active_config.voltage_kv,
        120.0
    );

    // Undo -> Back to initial
    state.undo();
    assert_eq!(
        state.force_field_settings.active_config.archetype,
        ForceFieldArchetype::AssaultBulkheadForceField
    );

    // Redo -> Back to MothershipEnergyShield
    state.redo();
    assert_eq!(
        state.force_field_settings.active_config.archetype,
        ForceFieldArchetype::MothershipEnergyShield
    );
}

#[test]
fn test_force_field_metrics_and_permeability() {
    let mut cfg = ForceFieldConfig::default();
    cfg.width_uu = 200.0;
    cfg.height_uu = 150.0;
    cfg.voltage_kv = 100.0;
    cfg.permeability = ForceFieldPermeability::ShockComboPassThrough;
    cfg.pylon_radius_uu = 20.0;
    cfg.has_emitter_pylons = true;

    let metrics = evaluate_force_field_metrics(&cfg, &[]);
    assert_eq!(metrics.surface_area_uu2, 30000.0);
    assert_eq!(metrics.deflection_efficiency_pct, 70.0);
    assert_eq!(metrics.pylon_clearance_width_uu, 160.0); // 200 - (20 * 2)
    assert!(metrics.power_consumption_kw > 0.0);
    assert!(metrics.is_safe_for_assault);
}

#[test]
fn test_force_field_safety_audits() {
    let mut cfg = ForceFieldArchetype::AssaultBulkheadForceField.to_config();
    cfg.permeability = ForceFieldPermeability::BlocksAll;
    cfg.generator_linked = false; // Block all without generator
    cfg.width_uu = 60.0;
    cfg.pylon_radius_uu = 20.0; // 60 - 40 = 20 uu clearance (< 48 uu minimum)

    let hazards = audit_force_field_safety(&cfg, &[]);
    assert!(hazards.iter().any(|h| h.title == "Permanent Impassable Barrier Hazard" && h.is_critical));
    assert!(hazards.iter().any(|h| h.title == "Corridor Chokepoint Encroachment" && h.is_critical));
}

#[test]
fn test_force_field_world_commit_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brush_count = state.world_brushes.len();

    let mut cfg = ForceFieldArchetype::AssaultBulkheadForceField.to_config();
    cfg.position = Vec3::new(512.0, 256.0, 0.0);
    cfg.has_emitter_pylons = true;
    state.force_field_settings.active_config = cfg;

    let (field_idx, pylon_indices, gen_idx) = state.commit_force_field_to_world();

    // 1 field brush + 2 pylon brushes = 3 new brushes
    assert_eq!(state.world_brushes.len(), initial_brush_count + 3);
    assert_eq!(field_idx, initial_brush_count);
    assert_eq!(pylon_indices.len(), 2);
    assert_eq!(gen_idx, Some(0));

    // Verify actor
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Engine.Trigger");

    // Test Undo
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brush_count);
    let level_after_undo = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_after_undo.actors.len(), 0);

    // Test Redo
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brush_count + 3);
    let level_after_redo = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_after_redo.actors.len(), 1);
}
