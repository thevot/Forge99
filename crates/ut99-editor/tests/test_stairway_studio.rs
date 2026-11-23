// crates/ut99-editor/tests/test_stairway_studio.rs
//
// Integration tests for Interactive Curved Architectural Stairway, Spiral Ramp & Smooth Collision Hull Studio.

use ut99_bsp::{CsgOperation, StairArchetype, StairHazardSeverity, PF_INVISIBLE, PF_SEMISOLID};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_stair_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is LinearStraight
    assert_eq!(
        state.stairway_studio_settings.active_config.archetype,
        StairArchetype::LinearStraight
    );
    assert!(state.stairway_studio_settings.cached_stair_brush.is_some());
    assert!(state.stairway_studio_settings.cached_ramp_brush.is_some());

    // Apply Curved90QuarterTurn
    state.apply_stair_archetype(StairArchetype::Curved90QuarterTurn);
    assert_eq!(
        state.stairway_studio_settings.active_config.archetype,
        StairArchetype::Curved90QuarterTurn
    );
    assert_eq!(state.stairway_studio_settings.active_config.sweep_angle_deg, 90.0);

    // Apply Curved180HalfTurn
    state.apply_stair_archetype(StairArchetype::Curved180HalfTurn);
    assert_eq!(
        state.stairway_studio_settings.active_config.archetype,
        StairArchetype::Curved180HalfTurn
    );
    assert_eq!(state.stairway_studio_settings.active_config.sweep_angle_deg, 180.0);

    // Apply SpiralTower
    state.apply_stair_archetype(StairArchetype::SpiralTower);
    assert_eq!(
        state.stairway_studio_settings.active_config.archetype,
        StairArchetype::SpiralTower
    );
    assert_eq!(state.stairway_studio_settings.active_config.sweep_angle_deg, 360.0);

    // Apply SwitchbackLanding
    state.apply_stair_archetype(StairArchetype::SwitchbackLanding);
    assert_eq!(
        state.stairway_studio_settings.active_config.archetype,
        StairArchetype::SwitchbackLanding
    );

    // Undo returns to SpiralTower
    state.undo();
    assert_eq!(
        state.stairway_studio_settings.active_config.archetype,
        StairArchetype::SpiralTower
    );

    // Redo returns to SwitchbackLanding
    state.redo();
    assert_eq!(
        state.stairway_studio_settings.active_config.archetype,
        StairArchetype::SwitchbackLanding
    );
}

#[test]
fn test_smooth_collision_wedge_ramp_generation() {
    let mut state = EditorState::default();

    // 1. Linear straight stairs smooth ramp
    state.apply_stair_archetype(StairArchetype::LinearStraight);
    state.stairway_studio_settings.active_config.step_count = 10;
    state.stairway_studio_settings.active_config.step_height = 16.0;
    state.stairway_studio_settings.active_config.step_length = 32.0;
    state.stairway_studio_settings.active_config.add_smooth_collision_hull = true;
    state.recalculate_stair_geometry();

    let ramp_brush = state.stairway_studio_settings.cached_ramp_brush.as_ref().expect("Ramp must be present");
    assert_eq!(ramp_brush.polys.len(), 5, "Linear collision ramp is a 5-sided triangular wedge prism");
    for poly in &ramp_brush.polys {
        assert_ne!(poly.flags & PF_INVISIBLE, 0, "Ramp face must be flagged PF_INVISIBLE");
        assert_ne!(poly.flags & PF_SEMISOLID, 0, "Ramp face must be flagged PF_SEMISOLID by default");
    }

    // 2. Spiral stairs smooth helical ramp
    state.apply_stair_archetype(StairArchetype::SpiralTower);
    state.stairway_studio_settings.active_config.step_count = 16;
    state.stairway_studio_settings.active_config.add_smooth_collision_hull = true;
    state.recalculate_stair_geometry();

    let spiral_ramp = state.stairway_studio_settings.cached_ramp_brush.as_ref().expect("Spiral ramp must be present");
    assert!(!spiral_ramp.polys.is_empty(), "Spiral ramp must have polygons");
    for poly in &spiral_ramp.polys {
        assert_ne!(poly.flags & PF_INVISIBLE, 0, "Spiral ramp face must be flagged PF_INVISIBLE");
    }
}

#[test]
fn test_ergonomics_and_max_step_height_safety_audit() {
    let mut state = EditorState::default();

    // 1. Standard competitive dimensions: 16 uu riser, 32 uu tread -> 0 critical hazards
    state.stairway_studio_settings.active_config.step_height = 16.0;
    state.stairway_studio_settings.active_config.step_length = 32.0;
    state.stairway_studio_settings.active_config.step_width = 192.0;
    state.recalculate_stair_geometry();

    let critical_hazards = state.stairway_studio_settings.cached_hazards.iter()
        .filter(|h| h.severity == StairHazardSeverity::Critical)
        .count();
    assert_eq!(critical_hazards, 0, "Safe stair must have 0 critical hazards");

    // 2. Violate UT99 MaxStepHeight: 28 uu riser (> 25.0 uu)
    state.stairway_studio_settings.active_config.step_height = 28.0;
    state.recalculate_stair_geometry();

    let max_step_violation = state.stairway_studio_settings.cached_hazards.iter().find(|h| {
        h.severity == StairHazardSeverity::Critical && h.title.contains("MaxStepHeight")
    });
    assert!(max_step_violation.is_some(), "Audit must catch risers exceeding UT99 MaxStepHeight (25.0 uu)");

    // 3. Steep slope angle: riser 24 uu, tread 12 uu -> slope > 60 degrees (> 45 deg warning)
    state.stairway_studio_settings.active_config.step_height = 24.0;
    state.stairway_studio_settings.active_config.step_length = 12.0;
    state.recalculate_stair_geometry();

    let steep_slope = state.stairway_studio_settings.cached_hazards.iter().find(|h| {
        h.title.contains("Slope")
    });
    assert!(steep_slope.is_some(), "Audit must warn about slopes steeper than 45 degrees");
}

#[test]
fn test_bot_navigation_centerline_and_world_brush_commit() {
    let mut state = EditorState::default();
    state.apply_stair_archetype(StairArchetype::LinearStraight);
    state.stairway_studio_settings.active_config.step_count = 8;
    state.recalculate_stair_geometry();

    // Verify bot waypoints
    let waypoints = &state.stairway_studio_settings.cached_waypoints;
    assert!(waypoints.len() >= 3, "Linear stair has entry base, mid flight, and top exit waypoints");
    assert!(waypoints.iter().all(|wp| wp.reachspec_valid), "All waypoints should be reachspec valid");

    // Commit to Active Red Builder Brush
    let old_builder_polys = state.builder_brush.polys.len();
    state.commit_stair_to_builder_brush();
    assert_eq!(state.builder_brush.csg_oper, CsgOperation::Active);
    assert_eq!(state.builder_brush.name, "ActiveBuilder");

    // Undo restores builder brush
    state.undo();
    assert_eq!(state.builder_brush.polys.len(), old_builder_polys);

    // Commit Stairs + Smooth Ramp to World
    let initial_world_brushes = state.world_brushes.len();
    let (stair_idx, ramp_idx) = state.commit_stair_and_ramp_to_world(CsgOperation::Add);
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 2);
    assert_eq!(stair_idx, initial_world_brushes);
    assert_eq!(ramp_idx, Some(initial_world_brushes + 1));
    assert!(state.map_is_dirty);

    // Undo removes both world brushes
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);

    // Redo re-inserts both world brushes
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 2);
}
