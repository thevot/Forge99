// crates/ut99-editor/tests/test_domination_studio.rs
//
// Integration tests for Interactive Domination Control Point Studio.

use glam::Vec3;
use ut99_bsp::{DominationArchetype, DominationPointId};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_domination_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is CrypticTriadBalance
    assert_eq!(
        state.domination_settings.active_config.archetype,
        DominationArchetype::CrypticTriadBalance
    );
    assert_eq!(
        state.domination_settings.active_config.active_point,
        DominationPointId::PointA
    );
    assert_eq!(
        state.domination_settings.active_config.score_cadence_secs,
        5.0
    );

    // Apply CinderVerticalTower
    state.apply_domination_archetype(DominationArchetype::CinderVerticalTower);
    assert_eq!(
        state.domination_settings.active_config.archetype,
        DominationArchetype::CinderVerticalTower
    );
    assert_eq!(
        state.domination_settings.active_config.active_point,
        DominationPointId::PointB
    );
    assert_eq!(
        state.domination_settings.active_config.position.z,
        240.0
    );

    // Apply GearsIndustrialHub
    state.apply_domination_archetype(DominationArchetype::GearsIndustrialHub);
    assert_eq!(
        state.domination_settings.active_config.archetype,
        DominationArchetype::GearsIndustrialHub
    );
    assert_eq!(
        state.domination_settings.active_config.active_point,
        DominationPointId::PointC
    );

    // Apply LethalCorridorSprint
    state.apply_domination_archetype(DominationArchetype::LethalCorridorSprint);
    assert_eq!(
        state.domination_settings.active_config.archetype,
        DominationArchetype::LethalCorridorSprint
    );

    // Apply SesmarTombPyramid
    state.apply_domination_archetype(DominationArchetype::SesmarTombPyramid);
    assert_eq!(
        state.domination_settings.active_config.archetype,
        DominationArchetype::SesmarTombPyramid
    );

    // Undo back to Lethal
    state.undo();
    assert_eq!(
        state.domination_settings.active_config.archetype,
        DominationArchetype::LethalCorridorSprint
    );

    // Redo to Sesmar
    state.redo();
    assert_eq!(
        state.domination_settings.active_config.archetype,
        DominationArchetype::SesmarTombPyramid
    );
}

#[test]
fn test_domination_metrics_and_triangle_telemetry() {
    let mut state = EditorState::default();

    // Populate level with 2 existing ControlPoints
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        let mut cp1 = ut99_format::ActorRecord::default();
        cp1.name = "ControlPoint_B".to_string();
        cp1.class_name = "Botpack.ControlPoint".to_string();
        cp1.location = Vec3::new(800.0, 0.0, 0.0);
        level.actors.push(cp1);

        let mut cp2 = ut99_format::ActorRecord::default();
        cp2.name = "ControlPoint_C".to_string();
        cp2.class_name = "Botpack.ControlPoint".to_string();
        cp2.location = Vec3::new(0.0, 800.0, 0.0);
        level.actors.push(cp2);
    }

    state.recalculate_domination_preview();
    let metrics = &state.domination_settings.cached_metrics;

    assert_eq!(metrics.total_points_in_level, 3);
    assert_eq!(metrics.dist_to_point_b, 800.0);
    assert_eq!(metrics.dist_to_point_c, 800.0);
    assert_eq!(metrics.sprint_time_to_point_b_secs, 2.0);
    assert_eq!(metrics.triangle_asymmetry_ratio, 0.0);
    assert_eq!(metrics.points_per_minute, 12.0);
}

#[test]
fn test_domination_hazard_auditing() {
    let mut state = EditorState::default();

    // Setup an adjacent ControlPoint at distance 350 (< 500 uu)
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();
        let mut cp = ut99_format::ActorRecord::default();
        cp.name = "SpamPoint".to_string();
        cp.class_name = "Botpack.ControlPoint".to_string();
        cp.location = Vec3::new(350.0, 0.0, 0.0);
        level.actors.push(cp);
    }

    state.recalculate_domination_preview();
    assert!(state.domination_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Proximity Spam")));

    // Test scoring cadence warning
    state.domination_settings.active_config.score_cadence_secs = 2.0; // < 3.0s
    state.recalculate_domination_preview();
    assert!(state.domination_settings.cached_hazards.iter().any(|h| h.title.contains("Rapid Scoring")));
}

#[test]
fn test_commit_domination_point_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit ControlPoint
    state.apply_domination_archetype(DominationArchetype::CrypticTriadBalance);
    let (brush_idx, actor_idx) = state.commit_domination_point_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(actor_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    let act = &level.actors[actor_idx.unwrap()];
    assert_eq!(act.class_name, "Botpack.ControlPoint");
    assert_eq!(act.group, "Domination");

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
    assert_eq!(level_redone.actors[0].class_name, "Botpack.ControlPoint");
}
