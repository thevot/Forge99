// crates/ut99-editor/tests/test_piston_studio.rs
//
// Integration tests for Interactive Industrial Steam Piston, Hydraulic Compactor & Trap Hazard Studio

use glam::Vec3;
use ut99_bsp::{
    evaluate_piston_metrics, PistonArchetype, PistonConfig, PistonEncroachAction, PistonOrientation,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_piston_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    let initial_config = state.piston_settings.active_config.clone();
    assert_eq!(
        initial_config.archetype,
        PistonArchetype::PressureChamberVerticalCrusher
    );

    // Apply HorizontalBulkheadRam
    state.apply_piston_archetype(PistonArchetype::HorizontalBulkheadRam);
    assert_eq!(
        state.piston_settings.active_config.archetype,
        PistonArchetype::HorizontalBulkheadRam
    );
    assert_eq!(
        state.piston_settings.active_config.orientation,
        PistonOrientation::HorizontalX
    );
    assert_eq!(state.piston_settings.active_config.stroke_length_uu, 256.0);

    // Undo -> Back to initial
    state.undo();
    assert_eq!(
        state.piston_settings.active_config.archetype,
        PistonArchetype::PressureChamberVerticalCrusher
    );
    assert_eq!(
        state.piston_settings.active_config.orientation,
        PistonOrientation::VerticalDown
    );

    // Redo -> Back to HorizontalBulkheadRam
    state.redo();
    assert_eq!(
        state.piston_settings.active_config.archetype,
        PistonArchetype::HorizontalBulkheadRam
    );
}

#[test]
fn test_piston_metrics_and_compression_force() {
    let mut cfg = PistonConfig::default();
    cfg.stroke_length_uu = 400.0;
    cfg.stroke_velocity_uu_s = 200.0;
    cfg.wait_time_s = 1.0;
    cfg.head_width_uu = 200.0;
    cfg.head_depth_uu = 200.0;
    cfg.head_thickness_uu = 50.0;
    cfg.squish_damage = 500;

    let metrics = evaluate_piston_metrics(&cfg, &[]);
    // travel time = 400 / 200 = 2.0s
    assert!((metrics.one_way_travel_time_s - 2.0).abs() < 1e-4);
    // full cycle = 2.0*2 + 1.0*2 = 6.0s
    assert!((metrics.full_cycle_period_s - 6.0).abs() < 1e-4);
    // area = 200 * 200 = 40,000 sq uu
    assert_eq!(metrics.compactor_surface_area_uu2, 40000.0);
    // volume = 40,000 * 50 = 2,000,000 cu uu
    assert_eq!(metrics.head_volume_uu3, 2000000.0);
    // compression impulse = (2,000,000 / 10,000) * 200 = 200 * 200 = 40,000
    assert_eq!(metrics.compression_impulse_force, 40000.0);
    assert!(metrics.is_instant_kill);
}

#[test]
fn test_piston_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // Trigger lethal squish without warning + lockout
    state.piston_settings.active_config.squish_damage = 1000;
    state.piston_settings.active_config.has_abort_button = false;
    state.piston_settings.active_config.has_warning_siren = false;
    state.piston_settings.active_config.stroke_length_uu = 384.0;
    state.piston_settings.active_config.wait_time_s = 0.1;
    state.recalculate_piston_preview();

    let hazards = &state.piston_settings.cached_hazards;
    assert!(hazards.iter().any(|h| h.title.contains("Lockout")));
    assert!(hazards.iter().any(|h| h.title.contains("Lethal Squish")));
    assert!(hazards.iter().any(|h| h.title.contains("Zero Wait Time")));
}

#[test]
fn test_commit_piston_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brush_count = state.world_brushes.len();

    state.piston_settings.active_config.position = Vec3::new(200.0, 300.0, 400.0);
    state.piston_settings.active_config.encroachment = PistonEncroachAction::CrushWhenEncroach;

    let (brush_idx, mover_idx) = state.commit_piston_to_world();
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);
    assert_eq!(brush_idx, initial_brush_count);
    assert!(mover_idx.is_some());

    // Check mover actor in level
    let level = state.loaded_unr.as_ref().unwrap();
    let mover = &level.actors[mover_idx.unwrap()];
    assert_eq!(mover.class_name, "Engine.Mover");
    assert_eq!(mover.location, Vec3::new(200.0, 300.0, 400.0));
    assert_eq!(mover.group, "PistonTraps");

    // Undo -> Piston brush and actor removed
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brush_count);
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 0);

    // Redo -> Restored
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Engine.Mover");
}
