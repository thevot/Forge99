// crates/ut99-editor/tests/test_destructible_studio.rs
//
// Integration tests for Interactive Destructible Architecture, Breakable Glass & Fracture Studio.

use ut99_bsp::{
    CsgOperation, DestructibleArchetype, DestructibleHazardSeverity, FracturePatternType,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_archetype_transitions_and_shard_generation() {
    let mut state = EditorState::default();

    // Default archetype is BreakableWindowGlass
    assert_eq!(
        state.destructible_studio_settings.selected_archetype,
        DestructibleArchetype::BreakableWindowGlass
    );
    assert_eq!(
        state.destructible_studio_settings.selected_pattern,
        FracturePatternType::RadialSpiderweb
    );
    assert_eq!(state.destructible_studio_settings.cached_shards.len(), 16);

    // Verify all shards have valid convex polygons with positive area
    for shard in &state.destructible_studio_settings.cached_shards {
        assert!(shard.local_vertices.len() >= 3);
        assert!(shard.area > 0.0);
    }

    // Switch to ExplodingStoneWall
    state.apply_destructible_archetype(DestructibleArchetype::ExplodingStoneWall);
    assert_eq!(
        state.destructible_studio_settings.selected_archetype,
        DestructibleArchetype::ExplodingStoneWall
    );
    assert_eq!(
        state.destructible_studio_settings.selected_pattern,
        FracturePatternType::CellularVoronoi
    );
    assert_eq!(state.destructible_studio_settings.active_config.health, 120.0);
    assert_eq!(state.destructible_studio_settings.cached_shards.len(), 24);

    // Switch to ReinforcedBlastDoor
    state.apply_destructible_archetype(DestructibleArchetype::ReinforcedBlastDoor);
    assert_eq!(
        state.destructible_studio_settings.selected_archetype,
        DestructibleArchetype::ReinforcedBlastDoor
    );
    assert_eq!(state.destructible_studio_settings.active_config.health, 300.0);
    assert_eq!(state.destructible_studio_settings.cached_shards.len(), 12);

    // Test manual pattern override and shard regeneration
    state.destructible_studio_settings.selected_pattern = FracturePatternType::DiagonalShatter;
    state.destructible_studio_settings.active_config.shard_count = 32;
    state.regenerate_destructible_shards();
    assert_eq!(state.destructible_studio_settings.cached_shards.len(), 32);
}

#[test]
fn test_destructible_shatter_simulation_and_floor_bouncing() {
    let mut state = EditorState::default();

    assert!(!state.destructible_studio_settings.simulation.is_shattered);
    assert_eq!(
        state.destructible_studio_settings.simulation.particles.len(),
        0
    );

    // Trigger shatter blast
    state.trigger_destructible_shatter();
    assert!(state.destructible_studio_settings.simulation.is_shattered);
    assert_eq!(
        state.destructible_studio_settings.simulation.particles.len(),
        state.destructible_studio_settings.cached_shards.len()
    );

    // Verify initial particle ejection velocities
    for p in &state.destructible_studio_settings.simulation.particles {
        assert!(p.velocity.length_squared() > 100.0);
        assert!(!p.is_at_rest);
    }

    // Step the physics simulation forward
    let initial_pos = state.destructible_studio_settings.simulation.particles[0].position;
    for _ in 0..10 {
        state.step_destructible_simulation(0.05);
    }

    let stepped_pos = state.destructible_studio_settings.simulation.particles[0].position;
    assert_ne!(initial_pos, stepped_pos);
    assert!(state.destructible_studio_settings.simulation.total_sim_time > 0.49);

    // Step far into future to verify bouncing and resting
    for _ in 0..200 {
        state.step_destructible_simulation(0.1);
    }

    // Particles must respect floor plane
    let floor_z = state.destructible_studio_settings.simulation.floor_z;
    for p in &state.destructible_studio_settings.simulation.particles {
        assert!(p.position.z >= floor_z - 1.0);
    }
}

#[test]
fn test_generate_barrier_and_undo_redo_invariance() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(ut99_format::UnrLevel::default());

    let initial_brush_count = state.world_brushes.len();
    let initial_actor_count = state.loaded_unr.as_ref().unwrap().actors.len();

    // Generate destructible barrier
    let brush_idx = state.generate_destructible_barrier();
    assert_eq!(brush_idx, initial_brush_count);

    // Check newly added brush in world_brushes
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);
    let brush = &state.world_brushes[brush_idx];
    assert_eq!(brush.csg_oper, CsgOperation::Active);

    // Check newly added actors in level
    let unr = state.loaded_unr.as_ref().unwrap();
    assert!(unr.actors.len() > initial_actor_count);

    let has_breaking_glass = unr.actors.iter().any(|a| a.class_name == "BreakingGlass");
    let has_path_node = unr.actors.iter().any(|a| a.class_name == "PathNode");
    assert!(has_breaking_glass);
    assert!(has_path_node);

    let total_actors_after_generate = unr.actors.len();

    // Test Undo: must remove both brush and actors cleanly
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brush_count);
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors.len(),
        initial_actor_count
    );

    // Test Redo: must restore both brush and actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors.len(),
        total_actors_after_generate
    );
    assert!(state
        .loaded_unr
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .any(|a| a.class_name == "BreakingGlass"));
}

#[test]
fn test_hazard_audit_and_validation() {
    let mut state = EditorState::default();

    // Default configuration must pass audit without critical errors
    state.audit_destructible_hazards();
    let initial_hazards = &state.destructible_studio_settings.cached_hazards;
    let has_critical = initial_hazards
        .iter()
        .any(|h| h.severity == DestructibleHazardSeverity::Critical);
    assert!(!has_critical);

    // Introduce invalid values: zero health & empty event tag
    state.destructible_studio_settings.active_config.health = 0.0;
    state.destructible_studio_settings.active_config.event_on_break = String::new();
    state.audit_destructible_hazards();

    let new_hazards = &state.destructible_studio_settings.cached_hazards;
    let zero_health_hazard = new_hazards
        .iter()
        .find(|h| h.title.contains("Zero or Negative Health"));
    assert!(zero_health_hazard.is_some());
    assert_eq!(
        zero_health_hazard.unwrap().severity,
        DestructibleHazardSeverity::Critical
    );

    let missing_event_hazard = new_hazards
        .iter()
        .find(|h| h.title.contains("Unnotified Breach Event"));
    assert!(missing_event_hazard.is_some());
    assert_eq!(
        missing_event_hazard.unwrap().severity,
        DestructibleHazardSeverity::Warning
    );
}
