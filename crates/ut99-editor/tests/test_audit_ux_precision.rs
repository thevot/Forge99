use glam::Vec3;
use ut99_editor::camera::{Camera, ViewportType, ViewportLayout};
use ut99_editor::editor_state::EditorState;
use ut99_editor::history::Action;
use ut99_editor::raycast::{Ray, GizmoAxis, GizmoMode};

#[test]
fn test_orthographic_camera_viewports() {
    let mut camera = Camera::new(Vec3::new(256.0, 512.0, 128.0), 1.6);
    camera.ortho_zoom = 2048.0;

    let aspect = 1.6;

    // 1. Perspective Viewport
    let vp_matrix_persp = camera.view_projection_matrix_for_type(ViewportType::Perspective, aspect);
    assert!(!vp_matrix_persp.is_nan());

    // 2. Top Viewport [X-Y Plane]
    let vp_matrix_top = camera.view_projection_matrix_for_type(ViewportType::Top, aspect);
    assert!(!vp_matrix_top.is_nan());
    let view_top = camera.view_matrix_for_type(ViewportType::Top);
    let proj_top = camera.projection_matrix_for_type(ViewportType::Top, aspect);
    assert!(!view_top.is_nan());
    assert!(!proj_top.is_nan());

    // 3. Front Viewport [X-Z Plane]
    let vp_matrix_front = camera.view_projection_matrix_for_type(ViewportType::Front, aspect);
    assert!(!vp_matrix_front.is_nan());

    // 4. Side Viewport [Y-Z Plane]
    let vp_matrix_side = camera.view_projection_matrix_for_type(ViewportType::Side, aspect);
    assert!(!vp_matrix_side.is_nan());

    // In orthographic projection, row 3 is [0, 0, 0, 1] instead of [0, 0, -1, 0] of perspective
    let col3 = proj_top.col(3);
    assert_eq!(col3.w, 1.0, "Orthographic projection matrix must have w_col == 1.0");
}

#[test]
fn test_gizmo_rotation_and_scale_hit_testing() {
    let center = Vec3::new(0.0, 0.0, 100.0);

    // Create a ray aiming towards the top of the Z-ring (in the X-Y plane at radius 70.0)
    // Ring for Z-axis lies in plane z = 100.0
    let ray_z = Ray::new(
        Vec3::new(70.0, 0.0, 200.0),
        Vec3::new(0.0, 0.0, -1.0),
    );

    let hit_axis = ray_z.hit_test_rotation_gizmo(center, 70.0, 12.0);
    assert_eq!(hit_axis, GizmoAxis::Z, "Ray should hit the Z rotation ring");

    // Test angle calculation: at (70.0, 0.0, 100.0), offset from center is (70, 0, 0)
    // In Z plane, u_axis is X, v_axis is Y => atan2(0, 70) = 0.0
    let angle_0 = ray_z.get_rotation_angle(center, GizmoAxis::Z).expect("Angle must be calculable");
    assert!((angle_0 - 0.0).abs() < 1e-4, "Angle at (70, 0) should be 0 radians");

    // Ray aiming at (0.0, 70.0, 200.0) => atan2(70, 0) = PI / 2 (90 degrees)
    let ray_90 = Ray::new(
        Vec3::new(0.0, 70.0, 200.0),
        Vec3::new(0.0, 0.0, -1.0),
    );
    let angle_90 = ray_90.get_rotation_angle(center, GizmoAxis::Z).expect("Angle must be calculable");
    assert!((angle_90.to_degrees() - 90.0).abs() < 1e-3, "Angle at (0, 70) should be 90 degrees");

    // Test axis projection for translation/scale
    let ray_proj = Ray::new(
        Vec3::new(10.0, -50.0, 100.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let dist = ray_proj.project_onto_axis(center, Vec3::X).expect("Projection should succeed");
    assert!((dist - 10.0).abs() < 1e-4, "Distance along X should be 10.0");
}

#[test]
fn test_brush_rotation_undo_redo() {
    let mut state = EditorState::default();

    let initial_count = state.world_brushes.len();
    // Add a cube brush
    state.set_builder_cube(Vec3::splat(256.0));
    state.csg_add();
    assert_eq!(state.world_brushes.len(), initial_count + 1);

    let brush_idx = initial_count;
    let initial_rot = state.world_brushes[brush_idx].rotation;
    assert_eq!(initial_rot, Vec3::ZERO);

    // Apply rotation (e.g. 45 degrees Pitch, 90 degrees Yaw)
    let new_rot = Vec3::new(45.0, 90.0, 0.0);
    state.world_brushes[brush_idx].rotation = new_rot;
    state.history.push(Action::RotateBrush {
        brush_index: brush_idx,
        old_rotation: initial_rot,
        new_rotation: new_rot,
    });

    assert_eq!(state.world_brushes[brush_idx].rotation, new_rot);

    // Undo rotation
    state.undo();
    assert_eq!(state.world_brushes[brush_idx].rotation, initial_rot, "Undo should restore initial rotation");

    // Redo rotation
    state.redo();
    assert_eq!(state.world_brushes[brush_idx].rotation, new_rot, "Redo should restore new rotation");
}

#[test]
fn test_editor_state_viewport_and_ui_scale_defaults() {
    let state = EditorState::default();

    // Check defaults
    assert_eq!(state.viewport_layout, ViewportLayout::Single);
    assert_eq!(state.active_viewport_type, ViewportType::Perspective);
    assert_eq!(state.gizmo_mode, GizmoMode::Translate);
    assert_eq!(state.ui_scale, 1.0);
    assert_eq!(state.rotation_snap_deg, 15.0);
    assert!(state.snap_to_rotation);
}
