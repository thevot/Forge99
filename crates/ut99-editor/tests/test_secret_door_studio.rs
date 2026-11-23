// crates/ut99-editor/tests/test_secret_door_studio.rs
//
// Integration tests for Interactive Secret Door, Concealed Passage & Keyhole Mover Studio.

use ut99_bsp::SecretDoorArchetype;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_secret_door_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is FlushWallSlidingPanel
    assert_eq!(
        state.secret_door_settings.active_config.archetype,
        SecretDoorArchetype::FlushWallSlidingPanel
    );
    assert_eq!(state.secret_door_settings.active_config.door_width_uu, 64.0);
    assert_eq!(state.secret_door_settings.active_config.stay_open_time_secs, 4.0);

    // Apply RotatingPivotingBookshelf
    state.apply_secret_door_archetype(SecretDoorArchetype::RotatingPivotingBookshelf);
    assert_eq!(
        state.secret_door_settings.active_config.archetype,
        SecretDoorArchetype::RotatingPivotingBookshelf
    );
    assert_eq!(state.secret_door_settings.active_config.door_width_uu, 72.0);

    // Apply SunkenFloorTrapdoor
    state.apply_secret_door_archetype(SecretDoorArchetype::SunkenFloorTrapdoor);
    assert_eq!(
        state.secret_door_settings.active_config.archetype,
        SecretDoorArchetype::SunkenFloorTrapdoor
    );
    assert_eq!(state.secret_door_settings.active_config.travel_displacement.z, -128.0);

    // Undo back to RotatingPivotingBookshelf
    state.undo();
    assert_eq!(
        state.secret_door_settings.active_config.archetype,
        SecretDoorArchetype::RotatingPivotingBookshelf
    );

    // Redo to SunkenFloorTrapdoor
    state.redo();
    assert_eq!(
        state.secret_door_settings.active_config.archetype,
        SecretDoorArchetype::SunkenFloorTrapdoor
    );
}

#[test]
fn test_secret_door_metrics_and_passage_clearance() {
    let mut state = EditorState::default();

    state.apply_secret_door_archetype(SecretDoorArchetype::FlushWallSlidingPanel);
    state.recalculate_secret_door_preview();

    let metrics = &state.secret_door_settings.cached_metrics;
    assert_eq!(metrics.passage_cross_section_sq_uu, 64.0 * 128.0);
    assert_eq!(metrics.stealth_rating_pct, 65.0);
}

#[test]
fn test_secret_door_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Missing Internal Exit Trigger (Entrapment Hazard)
    state.secret_door_settings.active_config.has_internal_exit_trigger = false;
    state.recalculate_secret_door_preview();
    assert!(state.secret_door_settings.cached_hazards.iter().any(|h| h.title.contains("Inescapable Room Entrapment")));

    // 2. Lethal Crush Encroachment
    state.secret_door_settings.active_config.encroach_behavior = "CrushWhenEncroached".to_string();
    state.recalculate_secret_door_preview();
    assert!(state.secret_door_settings.cached_hazards.iter().any(|h| h.title.contains("Lethal Crush Encroachment")));
}

#[test]
fn test_commit_secret_door_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Secret Door & Trigger to World
    state.apply_secret_door_archetype(SecretDoorArchetype::FlushWallSlidingPanel);
    let (brush_idx, trigger_idx) = state.commit_secret_door_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(trigger_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Engine.Trigger");

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
    assert_eq!(level_redone.actors[0].class_name, "Engine.Trigger");
}
