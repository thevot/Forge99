use glam::Vec3;
use ut99_bsp::math_eval::evaluate_expression;
use ut99_editor::editor_state::{BuilderShape, EditorState};

#[test]
fn test_math_evaluator() {
    assert_eq!(evaluate_expression("=512-128"), Ok(384.0));
    assert_eq!(evaluate_expression("=256*2"), Ok(512.0));
    assert_eq!(evaluate_expression("1024/4"), Ok(256.0));
    assert_eq!(evaluate_expression("=2^8"), Ok(256.0));
    assert_eq!(evaluate_expression("sqrt(65536)"), Ok(256.0));
    assert_eq!(evaluate_expression("=(128 + 64) * 2"), Ok(384.0));
}

#[test]
fn test_all_12_builder_shapes() {
    let mut state = EditorState::default();

    // 1. Cube
    state.set_builder_cube(Vec3::new(256.0, 256.0, 256.0));
    assert_eq!(state.builder_shape, BuilderShape::Cube);
    assert_eq!(state.builder_brush.polys.len(), 6);

    // 2. Cylinder
    state.set_builder_cylinder(256.0, 128.0, 8);
    assert_eq!(state.builder_shape, BuilderShape::Cylinder);
    assert!(state.builder_brush.polys.len() >= 8);

    // 3. Cone
    state.set_builder_cone(256.0, 128.0, 8);
    assert_eq!(state.builder_shape, BuilderShape::Cone);
    assert!(state.builder_brush.polys.len() >= 8);

    // 4. Sheet
    state.set_builder_sheet(256.0, 256.0);
    assert_eq!(state.builder_shape, BuilderShape::Sheet);
    assert_eq!(state.builder_brush.polys.len(), 2); // 2-sided sheet

    // 5. LinearStairs
    state.set_builder_stairs(8, 16.0, 128.0, 32.0);
    assert_eq!(state.builder_shape, BuilderShape::Stairs);
    assert!(state.builder_brush.polys.len() > 10);

    // 6. CurvedStairs
    state.set_builder_curved_stairs(8, 16.0, 128.0, 128.0, 90.0, false);
    assert_eq!(state.builder_shape, BuilderShape::CurvedStairs);
    assert!(state.builder_brush.polys.len() > 10);

    // 7. SpiralStairs
    state.set_builder_spiral_stairs(8, 16.0, 64.0, 128.0, 360.0, false);
    assert_eq!(state.builder_shape, BuilderShape::SpiralStairs);
    assert!(state.builder_brush.polys.len() > 10);

    // 8. Sphere
    state.set_builder_sphere(128.0, 8, 8);
    assert_eq!(state.builder_shape, BuilderShape::Sphere);
    assert!(state.builder_brush.polys.len() >= 32);

    // 9. Tetrahedron
    state.set_builder_tetrahedron(256.0);
    assert_eq!(state.builder_shape, BuilderShape::Tetrahedron);
    assert_eq!(state.builder_brush.polys.len(), 4);

    // 10. Torus
    state.set_builder_torus(256.0, 64.0, 8, 8);
    assert_eq!(state.builder_shape, BuilderShape::Torus);
    assert!(state.builder_brush.polys.len() >= 32);

    // 11. Volumetric
    state.set_builder_volumetric(256.0, 256.0, 2);
    assert_eq!(state.builder_shape, BuilderShape::Volumetric);
    assert!(state.builder_brush.polys.len() >= 2);

    // 12. Terrain
    state.set_builder_terrain(4, 4, 64.0, 32.0, 1234);
    assert_eq!(state.builder_shape, BuilderShape::Terrain);
    assert!(state.builder_brush.polys.len() >= 2);
}

#[test]
fn test_brush_ordering_and_csg_ops() {
    let mut state = EditorState::default();
    // Default state starts with 1 subtract brush (the initial room)
    assert_eq!(state.world_brushes.len(), 1);

    // Add Cube to world
    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 2);

    // Add Cylinder to world
    state.set_builder_cylinder(128.0, 64.0, 8);
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 3);

    // Select the last brush (index 2)
    state.selected_brush_index = Some(2);

    // Move up (toward index 0)
    state.move_selected_brush_up();
    assert_eq!(state.selected_brush_index, Some(1));

    // Send to first
    state.send_selected_brush_to_first();
    assert_eq!(state.selected_brush_index, Some(0));

    // Move down
    state.move_selected_brush_down();
    assert_eq!(state.selected_brush_index, Some(1));

    // Send to last
    state.send_selected_brush_to_last();
    assert_eq!(state.selected_brush_index, Some(2));
}

#[test]
fn test_csg_intersect_and_deintersect() {
    let mut state = EditorState::default();
    // Initial room is 1024x1024x512 centered at (0, 256, 0)
    assert_eq!(state.world_brushes.len(), 1);

    // Builder brush Cube 256x256x256 placed inside the room
    state.set_builder_cube(Vec3::new(256.0, 256.0, 256.0));
    state.builder_brush.location = Vec3::new(0.0, 256.0, 0.0);

    // Intersect: builder brush is inside the carved subtract room,
    // so intersecting it keeps its interior volume.
    state.csg_intersect();
    assert!(
        !state.builder_brush.polys.is_empty(),
        "Intersect inside room should preserve polygons"
    );

    // Test Deintersect
    state.set_builder_cube(Vec3::new(256.0, 256.0, 256.0));
    state.builder_brush.location = Vec3::new(0.0, 256.0, 0.0);
    state.csg_deintersect();
    println!(
        "Deintersect poly count: {}",
        state.builder_brush.polys.len()
    );
}
