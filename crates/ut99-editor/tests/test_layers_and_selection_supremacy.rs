// ==============================================================================
// Test Suite: Layers & Viewport Selection Supremacy (01 & 09)
// ==============================================================================

use ut99_editor::editor_state::EditorState;
use ut99_editor::raycast::Ray;
use ut99_editor::transform_session::gizmo_pivot;
use glam::Vec3;

#[test]
fn test_empty_selection_has_no_gizmo_and_builder_is_not_stuck() {
    let mut state = EditorState::default();
    state.new_map();

    // 1. Initially after new_map, nothing is selected
    assert_eq!(state.selected_brush_index, None);
    assert_eq!(state.selected_actor_index, None);
    assert_eq!(state.selected_mover_index, None);
    assert_eq!(state.selected_static_mesh_index, None);
    assert!(!state.is_builder_selected);
    assert_eq!(state.total_selected_count(), 0);

    // 2. Crucial: Gizmo pivot must be None! Zero phantom gizmo!
    assert_eq!(gizmo_pivot(&state), None);

    // 3. Explicitly selecting Red Builder Brush enables gizmo at its location
    state.is_builder_selected = true;
    assert_eq!(gizmo_pivot(&state), Some(state.builder_brush.location));

    // 4. Deselect all clears builder selection and removes gizmo completely
    state.deselect_all();
    assert!(!state.is_builder_selected);
    assert_eq!(gizmo_pivot(&state), None);
}

#[test]
fn test_single_click_clears_multi_selection_and_no_stuck_objects() {
    let mut state = EditorState::default();
    state.new_map();

    // Setup multi-selection with actors 0 and 1
    state.multi_selected_actors.insert(0);
    state.multi_selected_actors.insert(1);
    state.multi_selected_brushes.insert(0);
    assert!(state.has_multi_selection());

    // Ray directed at PlayerStart0 (location: 0, 0, 48)
    let ray = Ray::new(Vec3::new(200.0, 0.0, 48.0), Vec3::new(-1.0, 0.0, 0.0));
    let hit = state.select_at_ray_with_modifier(ray, false);
    assert!(hit);

    // Single click must have wiped all multi-selections!
    assert_eq!(state.multi_selected_actors.len(), 0);
    assert_eq!(state.multi_selected_brushes.len(), 0);
    assert_eq!(state.selected_actor_index, Some(0));
    assert_eq!(state.selected_brush_index, None);
    assert!(!state.is_builder_selected);
    assert!(!state.has_multi_selection());

    // Gizmo pivot is cleanly at PlayerStart0
    assert_eq!(gizmo_pivot(&state), Some(Vec3::new(0.0, 0.0, 48.0)));
}

#[test]
fn test_playerstart_hit_testing_at_half_height() {
    let mut state = EditorState::default();
    state.new_map();

    // PlayerStart is at (0, 0, 48), cylinder height is 72, center at (0, 0, 84).
    // Test ray shooting horizontally at torso height Z = 80.
    let torso_ray = Ray::new(Vec3::new(150.0, 0.0, 80.0), Vec3::new(-1.0, 0.0, 0.0));
    let hit = state.select_at_ray_with_modifier(torso_ray, false);
    assert!(hit);
    assert_eq!(state.selected_actor_index, Some(0));
    assert_eq!(state.selected_brush_index, None);
}

#[test]
fn test_visibility_hide_and_show_for_all_types() {
    let mut state = EditorState::default();
    state.new_map();

    // Brush visibility
    assert!(state.is_brush_visible(0));
    assert!(!state.toggle_brush_visibility(0)); // returns new visibility: false
    assert!(!state.is_brush_visible(0));
    assert!(state.toggle_brush_visibility(0)); // returns new visibility: true
    assert!(state.is_brush_visible(0));

    // Actor visibility
    assert!(state.is_actor_visible(0));
    assert!(!state.toggle_actor_visibility(0));
    assert!(!state.is_actor_visible(0));
    assert!(state.toggle_actor_visibility(0));
    assert!(state.is_actor_visible(0));

    // Builder brush visibility
    assert!(state.is_builder_visible());
    assert!(!state.toggle_builder_visibility());
    assert!(!state.is_builder_visible());
    assert!(state.toggle_builder_visibility());
    assert!(state.is_builder_visible());

    // Hide all / Show all
    state.hide_all_elements();
    assert!(!state.is_brush_visible(0));
    assert!(!state.is_actor_visible(0));
    assert!(!state.is_builder_visible());

    state.show_all_elements();
    assert!(state.is_brush_visible(0));
    assert!(state.is_actor_visible(0));
    assert!(state.is_builder_visible());
}

#[test]
fn test_hidden_and_locked_objects_are_immune_to_raycast_picking() {
    let mut state = EditorState::default();
    state.new_map();

    let ray = Ray::new(Vec3::new(150.0, 0.0, 48.0), Vec3::new(-1.0, 0.0, 0.0));

    // When visible and unlocked, ray selects Actor 0
    assert!(state.select_at_ray_with_modifier(ray, false));
    assert_eq!(state.selected_actor_index, Some(0));

    state.deselect_all();

    // Hide Actor 0 -> Raycast must skip it and hit the background brush!
    state.set_actor_visibility(0, false);
    assert!(state.select_at_ray_with_modifier(ray, false));
    assert_ne!(state.selected_actor_index, Some(0)); // skipped!

    state.deselect_all();
    state.set_actor_visibility(0, true);

    // Lock Actor 0 -> Raycast must skip locked objects!
    state.toggle_actor_lock(0);
    assert!(state.is_actor_locked(0));
    assert!(state.select_at_ray_with_modifier(ray, false));
    assert_ne!(state.selected_actor_index, Some(0)); // locked actor not picked!

    // Unlock Actor 0 -> Can be picked again
    state.toggle_actor_lock(0);
    assert!(!state.is_actor_locked(0));
    assert!(state.select_at_ray_with_modifier(ray, false));
    assert_eq!(state.selected_actor_index, Some(0));
}

#[test]
fn test_duplicate_selected_batch_includes_all_elements() {
    let mut state = EditorState::default();
    state.new_map();

    let initial_actors = state.loaded_unr.as_ref().unwrap().actors.len();
    let initial_brushes = state.world_brushes.len();

    // Select Actor 0 and Brush 0
    state.multi_selected_actors.insert(0);
    state.multi_selected_brushes.insert(0);

    state.duplicate_selected();

    let new_actors = state.loaded_unr.as_ref().unwrap().actors.len();
    let new_brushes = state.world_brushes.len();

    assert_eq!(new_actors, initial_actors + 1);
    assert_eq!(new_brushes, initial_brushes + 1);
}
