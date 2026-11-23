use glam::Vec3;
use ut99_bsp::{build_cube, build_cylinder, CsgOperation, PF_INVISIBLE, PF_SEMISOLID};
use ut99_editor::editor_state::{EditorState, OutlinerCategory};
use ut99_format::unr::actor::ActorRecord;

#[test]
fn test_cylindrical_uv_unwrapping() {
    let mut state = EditorState::default();

    // Create a cylinder brush in state
    let cylinder = build_cylinder("TestCylinder", CsgOperation::Add, 256.0, 128.0, 8);
    state.world_brushes.push(cylinder);

    let cyl_idx = state.world_brushes.len() - 1;
    state.selected_brush_index = Some(cyl_idx);

    // polys[0] and polys[1] are caps; polys[2..] are lateral faces
    let side_idx = 2;
    let old_u = state.world_brushes[cyl_idx].polys[side_idx].texture_u;

    // Apply cylindrical unwrap with 2x repeat U, 1x repeat V
    state.align_selected_brush_cylinder(2.0, 1.0);

    let new_brush = &state.world_brushes[cyl_idx];
    let mut side_polys_checked = 0;

    for poly in &new_brush.polys {
        // Lateral faces have normal with near-zero z
        if poly.plane.normal.z.abs() < 0.2 {
            side_polys_checked += 1;
            assert!(
                poly.texture_u.length() > 0.0,
                "Cylinder lateral face texture_u must be non-zero"
            );
            assert!(
                poly.texture_v.length() > 0.0,
                "Cylinder lateral face texture_v must be non-zero"
            );
        }
    }

    assert!(side_polys_checked >= 8, "Expected at least 8 lateral cylinder faces");

    let unwrapped_u = state.world_brushes[cyl_idx].polys[side_idx].texture_u;
    assert_ne!(
        unwrapped_u, old_u,
        "Lateral face texture_u should change after cylindrical unwrap"
    );

    // Test Undo reverts geometry UVs
    state.undo();
    assert_eq!(
        state.world_brushes[cyl_idx].polys[side_idx].texture_u,
        old_u,
        "Undo should revert cylindrical UV alignment"
    );

    // Test Redo
    state.redo();
    assert_eq!(
        state.world_brushes[cyl_idx].polys[side_idx].texture_u,
        unwrapped_u,
        "Redo should re-apply cylindrical UV alignment"
    );
}

#[test]
fn test_static_mesh_collision_hull_generation() {
    let mut state = EditorState::default();

    // Create a mock static mesh
    let mut mesh = build_cube("MyStaticMesh", CsgOperation::Add, Vec3::new(100.0, 150.0, 200.0));
    mesh.location = Vec3::new(500.0, 250.0, 64.0);
    state.static_meshes.push(mesh);
    state.selected_static_mesh_index = Some(0);

    let initial_brush_count = state.world_brushes.len();

    // Generate collision hull
    state.generate_collision_hull_for_selected_static_mesh();

    assert_eq!(
        state.world_brushes.len(),
        initial_brush_count + 1,
        "Collision hull should be added to world brushes"
    );

    let hull = state.world_brushes.last().expect("Hull must exist");
    assert_eq!(hull.name, "MyStaticMesh_CollisionHull");
    assert_eq!(hull.csg_oper, CsgOperation::Add);

    // Verify PF_INVISIBLE and PF_SEMISOLID flags
    for p in &hull.polys {
        assert_ne!(
            p.flags & PF_INVISIBLE,
            0,
            "Hull polygon must have PF_INVISIBLE flag"
        );
        assert_ne!(
            p.flags & PF_SEMISOLID,
            0,
            "Hull polygon must have PF_SEMISOLID flag"
        );
    }

    // Verify bounds encompass mesh + margin
    let (h_min, h_max) = hull.world_bounds();
    let width = h_max.x - h_min.x;
    let depth = h_max.y - h_min.y;
    let height = h_max.z - h_min.z;

    // Cube was (100, 150, 200), with margin 2.0 on each side -> (104, 154, 204)
    assert!((width - 104.0).abs() < 0.1, "Width should include margin: got {}", width);
    assert!((depth - 154.0).abs() < 0.1, "Depth should include margin: got {}", depth);
    assert!((height - 204.0).abs() < 0.1, "Height should include margin: got {}", height);
}

#[test]
fn test_outliner_filter_and_categories() {
    let mut state = EditorState::default();

    // Populate state with various actors
    let mut level = ut99_format::UnrLevel::default();

    let mut light = ActorRecord::default();
    light.name = "MainLight".to_string();
    light.class_name = "Light".to_string();
    light.location = Vec3::new(0.0, 0.0, 128.0);
    level.actors.push(light);

    let mut node = ActorRecord::default();
    node.name = "PathNode_A".to_string();
    node.class_name = "PathNode".to_string();
    node.location = Vec3::new(100.0, 0.0, 0.0);
    level.actors.push(node);

    let mut weapon = ActorRecord::default();
    weapon.name = "ShockRiflePickup".to_string();
    weapon.class_name = "ShockRifle".to_string();
    weapon.location = Vec3::new(200.0, 0.0, 0.0);
    level.actors.push(weapon);

    state.loaded_unr = Some(level);

    // Verify default category is All
    assert_eq!(state.outliner_category, OutlinerCategory::All);
    assert_eq!(state.outliner_filter, "");

    // Test camera focus on selection
    state.selected_actor_index = Some(0);
    let original_cam = state.camera.position;
    state.focus_camera_on_selection();
    assert_ne!(
        state.camera.position, original_cam,
        "Camera position should change when framing selected actor"
    );
}
