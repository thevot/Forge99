use glam::Vec3;
use ut99_bsp::{build_cube, CsgOperation};
use ut99_editor::editor_state::{EditorMover, EditorState};
use ut99_editor::prefab::Prefab;
use ut99_format::unr::actor::ActorRecord;
use ut99_format::unr::level::UnrLevel;

#[test]
fn test_prefab_library_canonical_presets() {
    let presets = Prefab::canonical_presets();
    assert!(presets.len() >= 5, "Expected at least 5 canonical presets, got {}", presets.len());

    let names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"Gothic Arched Pillar"));
    assert!(names.contains(&"Tournament Jump Pad"));
    assert!(names.contains(&"Wall Torch Sconce"));
    assert!(names.contains(&"Health Vial Wall Niche"));
    assert!(names.contains(&"Sci-Fi Sliding Door Assembly"));

    // Verify categories
    let categories: Vec<&str> = presets.iter().map(|p| p.category.as_str()).collect();
    assert!(categories.contains(&"Architecture"));
    assert!(categories.contains(&"Gameplay"));
    assert!(categories.contains(&"Lighting"));
    assert!(categories.contains(&"Pickups"));
    assert!(categories.contains(&"Doors"));

    // Test instantiation of a preset into an EditorState
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());

    let jump_pad_idx = state.prefabs.iter().position(|p| p.name == "Tournament Jump Pad").unwrap();
    let ok = state.instantiate_prefab(jump_pad_idx, Vec3::new(100.0, 200.0, 50.0));
    assert!(ok, "Instantiation must succeed");

    // Must have added at least 1 brush and 1 actor
    assert!(!state.world_brushes.is_empty(), "Prefab should spawn brushes");
    let unr = state.loaded_unr.as_ref().unwrap();
    assert!(!unr.actors.is_empty(), "Prefab should spawn actors");

    let spawned_actor = &unr.actors[unr.actors.len() - 1];
    assert_eq!(spawned_actor.class_name, "SpecialEvent");
    assert_eq!(spawned_actor.location, Vec3::new(100.0, 200.0, 50.0 + 32.0));
}

#[test]
fn test_duplicate_shortcuts_and_history() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());

    // 1. Duplicate Brush
    let cube = build_cube("FloorCube", CsgOperation::Add, Vec3::new(128.0, 128.0, 32.0));
    state.world_brushes.push(cube);
    let orig_idx = state.world_brushes.len() - 1;
    state.selected_brush_index = Some(orig_idx);

    let initial_brush_count = state.world_brushes.len();
    state.duplicate_selected();

    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);
    assert_eq!(state.selected_brush_index, Some(initial_brush_count));
    assert_eq!(state.world_brushes[initial_brush_count].location.x, state.world_brushes[orig_idx].location.x + state.grid_step);

    // Undo duplicate brush
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brush_count);

    // Redo duplicate brush
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);

    // 2. Duplicate Actor
    let mut actor = ActorRecord::default();
    actor.name = "Light0".to_string();
    actor.class_name = "Light".to_string();
    actor.location = Vec3::new(200.0, 100.0, 64.0);
    state.loaded_unr.as_mut().unwrap().actors.push(actor);

    state.selected_brush_index = None;
    state.selected_actor_index = Some(0);

    let initial_actor_count = state.loaded_unr.as_ref().unwrap().actors.len();
    state.duplicate_selected();

    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors.len(), initial_actor_count + 1);
    assert_eq!(state.selected_actor_index, Some(1));
    assert_eq!(unr.actors[1].location.x, 200.0 + state.grid_step);
    assert_eq!(unr.actors[1].name, "Light_1");

    // Undo duplicate actor
    state.undo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actor_count);

    // Redo duplicate actor
    state.redo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actor_count + 1);

    // 3. Duplicate Mover
    let mover_brush = build_cube("DoorBrush", CsgOperation::Add, Vec3::new(64.0, 64.0, 128.0));
    let mut mover = EditorMover::new(mover_brush);
    mover.name = "Mover0".to_string();
    mover.tag = "DoorTag".to_string();
    state.movers.push(mover);

    state.selected_actor_index = None;
    state.selected_mover_index = Some(0);

    let initial_mover_count = state.movers.len();
    state.duplicate_selected();

    assert_eq!(state.movers.len(), initial_mover_count + 1);
    assert_eq!(state.selected_mover_index, Some(1));
    assert_eq!(state.movers[1].brush.location.x, state.movers[0].brush.location.x + state.grid_step);

    // Undo duplicate mover
    state.undo();
    assert_eq!(state.movers.len(), initial_mover_count);
}

#[test]
fn test_snap_selected_to_floor_physics_raycast() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());

    // Create a floor box centered at (0, 0, 0) with size (512, 512, 32).
    // Bounds: Z from -16.0 to +16.0 (top surface at Z = 16.0).
    let floor_brush = build_cube("Floor", CsgOperation::Add, Vec3::new(512.0, 512.0, 32.0));
    state.world_brushes.push(floor_brush);

    // Create a floating light actor above the floor
    let mut actor = ActorRecord::default();
    actor.name = "FloatingLight".to_string();
    actor.class_name = "Light".to_string();
    actor.location = Vec3::new(0.0, 0.0, 200.0);
    state.loaded_unr.as_mut().unwrap().actors.push(actor);

    // Select actor and snap to floor
    state.selected_actor_index = Some(0);
    let snapped = state.snap_selected_to_floor();
    assert!(snapped, "Snap to floor must succeed when floor geometry exists below actor");

    let unr = state.loaded_unr.as_ref().unwrap();
    // Actor origin rests at floor_z + 16.0 offset = 16.0 + 16.0 = 32.0
    assert_eq!(unr.actors[0].location.z, 32.0, "Actor Z should snap to floor surface + clearance");

    // Test Undo snap
    state.undo();
    let unr_after_undo = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr_after_undo.actors[0].location.z, 200.0, "Undo should restore pre-snap actor Z");

    // Test snap on Brush
    let mut floating_box = build_cube("Crate", CsgOperation::Add, Vec3::new(64.0, 64.0, 64.0));
    floating_box.location = Vec3::new(0.0, 0.0, 300.0);
    state.world_brushes.push(floating_box);

    let box_idx = state.world_brushes.len() - 1;
    state.selected_actor_index = None;
    state.selected_brush_index = Some(box_idx);

    let brush_snapped = state.snap_selected_to_floor();
    assert!(brush_snapped, "Brush snap to floor must succeed");

    // Box has height 64 (extent +/-32 from location.z).
    // Bottom at floor_z = 16.0 means center Z = 16.0 + 32.0 = 48.0
    let (min_b, _) = state.world_brushes[box_idx].world_bounds();
    assert!((min_b.z - 16.0).abs() < 0.01, "Brush bottom min_z should match floor surface at 16.0");

    // Test snapping when completely off the floor (should return false)
    state.world_brushes[box_idx].location = Vec3::new(2000.0, 2000.0, 300.0);
    let no_floor_snapped = state.snap_selected_to_floor();
    assert!(!no_floor_snapped, "Snap must fail gracefully when no floor is beneath");
}

#[test]
fn test_tag_event_wiring_visualizer() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());

    // Trigger actor
    let mut trigger = ActorRecord::default();
    trigger.name = "Trigger0".to_string();
    trigger.class_name = "Trigger".to_string();
    trigger.location = Vec3::new(0.0, 0.0, 0.0);
    trigger.event = "OpenSecretDoor".to_string();
    state.loaded_unr.as_mut().unwrap().actors.push(trigger);

    // Mover with matching Tag
    let door_brush = build_cube("Door", CsgOperation::Add, Vec3::new(32.0, 128.0, 128.0));
    let mut door_mover = EditorMover::new(door_brush);
    door_mover.name = "SecretDoorMover".to_string();
    door_mover.tag = "opensecretdoor".to_string(); // test case-insensitivity
    door_mover.brush.location = Vec3::new(300.0, 0.0, 0.0);
    state.movers.push(door_mover);

    let links = state.get_event_tag_links();
    assert_eq!(links.len(), 1, "Must find exactly one trigger-to-mover link");
    assert_eq!(links[0].0, Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(links[0].1, Vec3::new(300.0, 0.0, 0.0));
    assert_eq!(links[0].2, "OpenSecretDoor");
}

#[test]
fn test_save_selection_as_prefab() {
    let mut state = EditorState::default();
    let cube = build_cube("CustomArch", CsgOperation::Add, Vec3::new(64.0, 64.0, 128.0));
    state.world_brushes.push(cube);
    let sel_idx = state.world_brushes.len() - 1;
    state.selected_brush_index = Some(sel_idx);

    let initial_count = state.prefabs.len();
    let res = state.save_selection_as_prefab("My Arch", "Architecture", "Custom user arch");
    assert!(res.is_ok(), "Saving valid selection as prefab should succeed");
    assert_eq!(state.prefabs.len(), initial_count + 1);

    let custom_idx = state.prefabs.len() - 1;
    let custom_prefab = &state.prefabs[custom_idx];
    assert_eq!(custom_prefab.name, "My Arch");
    assert_eq!(custom_prefab.category, "Architecture");
    assert_eq!(custom_prefab.brushes.len(), 1);

    // Instantiate custom prefab
    let initial_brush_count = state.world_brushes.len();
    let inst_ok = state.instantiate_prefab(custom_idx, Vec3::new(500.0, 500.0, 0.0));
    assert!(inst_ok);
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);
    assert_eq!(state.world_brushes[initial_brush_count].location, Vec3::new(500.0, 500.0, 0.0));
}
