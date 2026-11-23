// crates/ut99-editor/tests/test_reactor_core_studio.rs
//
// Integration tests for Interactive Rotating Gyroscope, Reactor Core & Kinetic Ring Studio

use glam::Vec3;
use ut99_bsp::{
    audit_reactor_core_safety, evaluate_reactor_core_metrics, LevelReactorContextActorRecord,
    ReactorCoreArchetype, ReactorCoreConfig, ReactorHazardType,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_reactor_core_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    let initial_config = state.reactor_core_settings.active_config.clone();
    assert_eq!(
        initial_config.archetype,
        ReactorCoreArchetype::LiandriCentrifugalCore
    );

    // Apply NivenNuclearContainmentVessel
    state.apply_reactor_core_archetype(ReactorCoreArchetype::NivenNuclearContainmentVessel);
    assert_eq!(
        state.reactor_core_settings.active_config.archetype,
        ReactorCoreArchetype::NivenNuclearContainmentVessel
    );
    assert_eq!(
        state.reactor_core_settings.active_config.hazard_type,
        ReactorHazardType::IonizingRadiation
    );
    assert_eq!(
        state.reactor_core_settings.active_config.hazard_radius_uu,
        320.0
    );

    // Undo -> Back to initial
    state.undo();
    assert_eq!(
        state.reactor_core_settings.active_config.archetype,
        ReactorCoreArchetype::LiandriCentrifugalCore
    );

    // Redo -> Back to NivenNuclearContainmentVessel
    state.redo();
    assert_eq!(
        state.reactor_core_settings.active_config.archetype,
        ReactorCoreArchetype::NivenNuclearContainmentVessel
    );
}

#[test]
fn test_reactor_core_kinetic_metrics() {
    let mut cfg = ReactorCoreConfig::default();
    cfg.outer_ring_radius_uu = 200.0;
    cfg.outer_ring_rpm = 30.0;
    cfg.hazard_radius_uu = 300.0;
    cfg.damage_per_sec = 50.0;

    let metrics = evaluate_reactor_core_metrics(&cfg, &[]);
    // Tangential speed: 2 * pi * 200 * (30/60) = 200 * pi approx 628.3 uu/s
    assert!((metrics.outer_rim_tangential_speed_uus - 628.3).abs() < 2.0);
    assert!(metrics.centrifugal_g_force > 0.0);
    assert!(metrics.stored_kinetic_energy_mj > 0.0);
    assert_eq!(metrics.time_to_lethal_sec, 2.0); // 100 / 50 = 2.0s
}

#[test]
fn test_reactor_core_safety_audits() {
    let mut cfg = ReactorCoreArchetype::LiandriCentrifugalCore.to_config();
    cfg.position = Vec3::new(0.0, 0.0, 0.0);
    cfg.hazard_radius_uu = 350.0;
    cfg.outer_ring_radius_uu = 500.0;
    cfg.outer_ring_rpm = 90.0; // 2 * pi * 500 * 1.5 = 4712 uu/s (> 2000 uu/s)

    let spawn = LevelReactorContextActorRecord {
        name: "PlayerStart1".to_string(),
        class_name: "Engine.PlayerStart".to_string(),
        position: Vec3::new(200.0, 0.0, 0.0), // Inside 350 uu hazard
    };

    let hazards = audit_reactor_core_safety(&cfg, &[spawn]);
    assert!(hazards.iter().any(|h| h.title == "Lethal Radiation Encroaching Spawn Point" && h.is_critical));
    assert!(hazards.iter().any(|h| h.title == "Excessive Rim Tangential Velocity" && h.is_critical));
}

#[test]
fn test_reactor_core_world_commit_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brush_count = state.world_brushes.len();

    let mut cfg = ReactorCoreArchetype::LiandriCentrifugalCore.to_config();
    cfg.position = Vec3::new(1024.0, 0.0, 128.0);
    state.reactor_core_settings.active_config = cfg;

    let (core_idx, ring_indices, act_idx) = state.commit_reactor_core_to_world();

    // 1 core brush + 2 ring brushes (outer and mid) = 3 new brushes
    assert_eq!(state.world_brushes.len(), initial_brush_count + 3);
    assert_eq!(core_idx, initial_brush_count);
    assert_eq!(ring_indices.len(), 2);
    assert_eq!(act_idx, Some(0));

    // Verify actor
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Engine.RotatingMover");

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
