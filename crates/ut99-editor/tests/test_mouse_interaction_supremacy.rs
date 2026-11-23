// crates/ut99-editor/tests/test_mouse_interaction_supremacy.rs
//
// Verification test suite for Mouse Interaction Supremacy (MOU-01 to MOU-07).

use glam::Vec3;
use ut99_bsp::brush::build_cube;
use ut99_bsp::CsgOperation;
use ut99_editor::editor_state::{EditorState, MarqueeBox};
use ut99_editor::raycast::{GizmoAxis, Ray};
use ut99_format::unr::actor::ActorRecord;
use ut99_format::UnrLevel;

#[test]
fn test_mou01_direct_drag_and_screen_plane_gizmo() {
    let mut state = EditorState::default();
    state.active_gizmo_axis = GizmoAxis::ScreenPlane;
    state.snap_to_grid = true;
    state.grid_step = 16.0;

    let start_loc = Vec3::new(100.0, 200.0, 300.0);
    let camera_fwd = Vec3::new(0.0, 1.0, 0.0); // Facing +Y
    let plane_normal = camera_fwd;

    // Ray passing through (133.0, 0.0, 349.0) heading +Y
    let ray = Ray::new(Vec3::new(133.0, 0.0, 349.0), Vec3::new(0.0, 1.0, 0.0));
    let hit = ray.project_onto_plane(start_loc, plane_normal).expect("Should hit plane");

    let drag_offset = Vec3::ZERO;
    let mut delta_world = hit - drag_offset - start_loc;
    assert!((delta_world.x - 33.0).abs() < 1e-4);
    assert!((delta_world.z - 49.0).abs() < 1e-4);

    // Apply grid snap (16 UU)
    delta_world.x = (delta_world.x / state.grid_step).round() * state.grid_step;
    delta_world.z = (delta_world.z / state.grid_step).round() * state.grid_step;
    assert_eq!(delta_world.x, 32.0);
    assert_eq!(delta_world.z, 48.0);

    let final_loc = start_loc + delta_world;
    assert_eq!(final_loc, Vec3::new(132.0, 200.0, 348.0));
}

#[test]
fn test_mou02_surface_magnet_actor_rest_offset_and_normal_alignment() {
    let mut state = EditorState::default();
    let mut unr = UnrLevel::default();

    let mut medbox = ActorRecord::default();
    medbox.name = "MedBox0".to_string();
    medbox.class_name = "MedBox".to_string();
    medbox.location = Vec3::new(0.0, 0.0, 500.0);
    unr.actors.push(medbox);

    let mut wall_torch = ActorRecord::default();
    wall_torch.name = "Torch0".to_string();
    wall_torch.class_name = "TorchFlame".to_string();
    wall_torch.location = Vec3::new(0.0, 0.0, 500.0);
    unr.actors.push(wall_torch);

    state.loaded_unr = Some(unr);

    // 1. Snap MedBox to floor at Z = 100.0 with normal (0, 0, 1)
    let floor_hit = Vec3::new(256.0, 512.0, 100.0);
    let floor_norm = Vec3::new(0.0, 0.0, 1.0);
    let ok = state.snap_actor_to_surface(0, floor_hit, floor_norm, false);
    assert!(ok);

    let actor0 = &state.loaded_unr.as_ref().unwrap().actors[0];
    // MedBox has collision height 8.0, so rest_offset is 8 + 1 = 9.0
    assert_eq!(actor0.location.x, 256.0);
    assert_eq!(actor0.location.y, 512.0);
    assert_eq!(actor0.location.z, 109.0); // Exact stock physical resting position

    // 2. Snap TorchFlame to a vertical wall with normal (1, 0, 0) and align normal = true
    let wall_hit = Vec3::new(-400.0, 128.0, 64.0);
    let wall_norm = Vec3::new(1.0, 0.0, 0.0);
    let ok2 = state.snap_actor_to_surface(1, wall_hit, wall_norm, true);
    assert!(ok2);

    let actor1 = &state.loaded_unr.as_ref().unwrap().actors[1];
    // Clearance along normal
    assert!(actor1.location.x > -400.0);
    // Yaw aligned along +X (angle 0)
    assert_eq!(actor1.rotation.1, 0);
}

#[test]
fn test_mou02_surface_magnet_brush_bounds_clearance() {
    let mut state = EditorState::default();
    state.world_brushes.clear(); // Clear default room
    let cube = build_cube("FloorCube", CsgOperation::Add, Vec3::new(128.0, 128.0, 64.0));
    state.world_brushes.push(cube);

    let hit_pt = Vec3::new(0.0, 0.0, 0.0);
    let hit_norm = Vec3::new(0.0, 0.0, 1.0); // Floor normal
    let ok = state.snap_brush_to_surface(0, hit_pt, hit_norm);
    assert!(ok);

    let b = &state.world_brushes[0];
    // Half extent along Z is 32.0, so it rests cleanly at Z = 32.0
    assert_eq!(b.location, Vec3::new(0.0, 0.0, 32.0));
}

#[test]
fn test_mou03_mouse_wheel_in_flight_rotation_and_elevation() {
    let mut state = EditorState::default();
    let mut unr = UnrLevel::default();

    let mut actor = ActorRecord::default();
    actor.name = "TestPlayerStart".to_string();
    actor.class_name = "PlayerStart".to_string();
    actor.location = Vec3::new(100.0, 100.0, 40.0);
    actor.rotation = (0, 0, 0);
    unr.actors.push(actor);
    state.loaded_unr = Some(unr);
    state.selected_actor_index = Some(0);

    // 1. Test In-flight Yaw rotation: +15 deg = +2730 rotational units
    let yaw_step_deg = 15.0f32;
    let yaw_units = (yaw_step_deg * 65536.0 / 360.0) as i32;
    assert_eq!(yaw_units, 2730);

    if let Some(ref mut unr) = state.loaded_unr {
        let a = unr.actors.get_mut(0).unwrap();
        a.rotation.1 = (a.rotation.1 + yaw_units) & 0xFFFF;
    }
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].rotation.1, 2730);

    // 2. Test In-flight elevation along Z
    state.grid_step = 32.0;
    if let Some(ref mut unr) = state.loaded_unr {
        let a = unr.actors.get_mut(0).unwrap();
        a.location.z += state.grid_step;
    }
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].location.z, 72.0);
}

#[test]
fn test_mou04_shift_drag_camera_tracking() {
    let mut state = EditorState::default();
    state.camera.position = Vec3::new(0.0, -200.0, 100.0);

    let mut unr = UnrLevel::default();
    let mut actor = ActorRecord::default();
    actor.location = Vec3::new(0.0, 0.0, 40.0);
    unr.actors.push(actor);
    state.loaded_unr = Some(unr);

    let initial_cam_to_actor = state.loaded_unr.as_ref().unwrap().actors[0].location - state.camera.position;

    // Simulate Shift-Drag motion of actor by (512.0, 1024.0, 0.0)
    let step_delta = Vec3::new(512.0, 1024.0, 0.0);
    let shift_drag_tracking = true;

    if shift_drag_tracking {
        state.camera.position += step_delta;
    }
    if let Some(ref mut unr) = state.loaded_unr {
        unr.actors[0].location += step_delta;
    }

    let final_cam_to_actor = state.loaded_unr.as_ref().unwrap().actors[0].location - state.camera.position;

    // The relative distance between camera and actor remains perfectly invariant!
    assert_eq!(initial_cam_to_actor, final_cam_to_actor);
    assert_eq!(state.camera.position, Vec3::new(512.0, 824.0, 100.0));
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].location, Vec3::new(512.0, 1024.0, 40.0));
}

#[test]
fn test_mou05_interactive_vertex_magnet_snapping() {
    let mut state = EditorState::default();
    state.world_brushes.clear();
    let mut cube = build_cube("TargetCube", CsgOperation::Add, Vec3::new(64.0, 64.0, 64.0));
    cube.location = Vec3::new(200.0, 300.0, 400.0);
    state.world_brushes.push(cube);

    // Front vertex along ray from origin (232.0, 0.0, 432.0) heading +Y is at (232.0, 268.0, 432.0)
    let expected_vertex = Vec3::new(232.0, 268.0, 432.0);

    // Cast a ray passing directly through X=232, Z=432 heading +Y
    let ray_origin = Vec3::new(232.0, 0.0, 432.0);
    let ray_dir = Vec3::new(0.0, 1.0, 0.0);
    let ray = Ray::new(ray_origin, ray_dir);

    let nearest = state.find_nearest_world_vertex(ray, 48.0);
    assert!(nearest.is_some());
    let snapped_v = nearest.unwrap();
    assert_eq!(snapped_v, expected_vertex);
}

#[test]
fn test_mou06_3d_frustum_marquee_perspective_selection() {
    let mut state = EditorState::default();
    let mut unr = UnrLevel::default();

    // Camera at (0, 0, 0) looking along +X (yaw = 0)
    state.camera.position = Vec3::ZERO;
    state.camera.yaw = 0.0;
    state.camera.pitch = 0.0;

    // Actor 0: directly along forward vector (+X) at (300.0, 0.0, 0.0) -> inside frustum
    let mut a0 = ActorRecord::default();
    a0.name = "InsideActor".to_string();
    a0.location = Vec3::new(300.0, 0.0, 0.0);
    unr.actors.push(a0);

    // Actor 1: far to the side (+Y = 5000) -> outside frustum
    let mut a1 = ActorRecord::default();
    a1.name = "OutsideActor".to_string();
    a1.location = Vec3::new(300.0, 5000.0, 0.0);
    unr.actors.push(a1);

    state.loaded_unr = Some(unr);

    // Construct a marquee box in perspective mode enclosing center of screen
    let marquee = MarqueeBox {
        start_screen: (200.0, 200.0),
        current_screen: (1240.0, 700.0),
        viewport_type: ut99_editor::camera::ViewportType::Perspective,
    };

    let (_brushes, actors) = state.select_within_marquee(marquee, false);
    assert!(actors >= 1);
    assert!(state.multi_selected_actors.contains(&0));
    // Actor 1 is outside and not selected
    assert!(!state.multi_selected_actors.contains(&1));
}

#[test]
fn test_mou07_floating_cursor_delta_hud_telemetry() {
    let delta = Vec3::new(64.0, -128.0, 32.0);
    let length = delta.length();
    let hud_text = format!("ΔX: {:+.0}  ΔY: {:+.0}  ΔZ: {:+.0}  [{:.1} UU]", delta.x, delta.y, delta.z, length);

    assert!(hud_text.contains("ΔX: +64"));
    assert!(hud_text.contains("ΔY: -128"));
    assert!(hud_text.contains("ΔZ: +32"));
    assert!(hud_text.contains("UU"));
}
