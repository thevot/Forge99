use glam::Vec3;
use ut99_bsp::Shape2D;
use ut99_editor::editor_state::{EditorMode, EditorState};

#[test]
fn test_vertex_editing_operations() {
    let mut state = EditorState::default();
    state.editor_mode = EditorMode::VertexEdit;

    // Get unique vertices of the default active cube
    let initial_verts = state.get_active_brush_unique_vertices();
    assert_eq!(initial_verts.len(), 8, "Cube has 8 unique vertices");

    // Select vertex #0
    state.selected_vertex_indices = vec![0];
    let v0_initial = initial_verts[0];

    // Move vertex #0 by +64 units on X
    let delta = Vec3::new(64.0, 0.0, 0.0);
    state.move_selected_vertices(delta);

    let updated_verts = state.get_active_brush_unique_vertices();
    let expected_v0 = v0_initial + delta;
    assert!(
        updated_verts.iter().any(|v| v.distance(expected_v0) < 0.1),
        "Vertex position must update by delta"
    );

    // Verify all polygons of the brush have valid plane equations
    for poly in &state.builder_brush.polys {
        assert!(poly.plane.normal.length_squared() > 0.5, "Plane normal must remain valid");
    }
}

#[test]
fn test_vertex_snap_welding() {
    let mut state = EditorState::default();
    state.editor_mode = EditorMode::VertexEdit;

    // Default room is centered at (0, 0, 256) size (1024, 1024, 512), corner at (512, 512, 512)
    // Place builder cube (size 256) so its top corner is slightly offset from the room corner
    state.builder_brush.location = Vec3::new(390.0, 390.0, 390.0);

    let verts = state.get_active_brush_unique_vertices();
    assert!(!verts.is_empty());

    // Select the corner vertex closest to (512, 512, 512)
    let room_corner = Vec3::new(512.0, 512.0, 512.0);
    let closest_idx = verts
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance(room_corner)
                .partial_cmp(&b.distance(room_corner))
                .unwrap()
        })
        .map(|(i, _)| i)
        .unwrap();

    state.selected_vertex_indices = vec![closest_idx];

    // Snap to nearest room vertex within 32 units (offset is ~6 units)
    let snapped = state.snap_selected_vertices_to_nearest(32.0);
    assert!(snapped, "Vertex should find and snap to nearest room corner");

}

#[test]
fn test_brush_clipping_tool() {
    let mut state = EditorState::default();
    state.editor_mode = EditorMode::BrushClip;

    let initial_poly_count = state.builder_brush.polys.len();
    assert_eq!(initial_poly_count, 6, "Initial cube has 6 faces");

    // Place 2 clip markers across the builder cube center at Z = 0
    state.add_clip_point(Vec3::new(-200.0, 0.0, 0.0));
    state.add_clip_point(Vec3::new(200.0, 0.0, 0.0));
    assert_eq!(state.clip_points.len(), 2);

    let plane = state.get_clipping_plane();
    assert!(plane.is_some(), "2 markers must form a valid vertical cutting plane");

    // Execute Clip Brush: should slice the cube in half and cap the open face
    state.execute_clip_brush();
    assert!(
        !state.builder_brush.polys.is_empty(),
        "Clipped brush must preserve valid polygons"
    );
    assert!(
        state.builder_brush.polys.len() >= 4,
        "Clipped cube must have remaining faces and cap"
    );

    // Test Flip Normal
    let normal_before = state.clip_normal_flipped;
    state.flip_clip_normal();
    assert_eq!(state.clip_normal_flipped, !normal_before);

    // Test Clear Clip Points
    state.clear_clip_points();
    assert!(state.clip_points.is_empty());
}

#[test]
fn test_brush_split_into_two_solids() {
    let mut state = EditorState::default();
    state.editor_mode = EditorMode::BrushClip;

    // Slicing across with 3 points
    state.add_clip_point(Vec3::new(-100.0, -100.0, 0.0));
    state.add_clip_point(Vec3::new(100.0, -100.0, 0.0));
    state.add_clip_point(Vec3::new(0.0, 100.0, 0.0));

    let initial_world_brushes = state.world_brushes.len();

    // Execute split: keeps one piece in builder and places the other piece as a new world brush
    state.execute_split_brush();
    assert_eq!(
        state.world_brushes.len(),
        initial_world_brushes + 1,
        "Splitting builder adds the other solid half to the world"
    );
    assert!(!state.builder_brush.polys.is_empty());
}

#[test]
fn test_2d_shape_editor_extrude_and_revolve() {
    let mut state = EditorState::default();

    // 1. Extrude custom 2D Arch
    state.shape2d = Shape2D::new_arch(256.0, 384.0, 8);
    state.shape_editor_depth = 128.0;
    state.apply_shape2d_extrude();

    assert!(
        state.builder_brush.polys.len() >= 10,
        "Extruded arch brush must have top, bottom and side quads"
    );

    // 2. Revolve 2D Profile (Pillar / Lathe)
    state.shape2d = Shape2D::new_circle(64.0, 12);
    state.shape_editor_revolve_angle = 360.0;
    state.shape_editor_revolve_segs = 12;
    state.apply_shape2d_revolve();

    assert!(
        state.builder_brush.polys.len() >= 12,
        "Revolved brush must produce solid cylindrical ring faces"
    );

    // 3. Bevel Extrusion
    state.shape2d = Shape2D::new_box(200.0, 200.0);
    state.shape_editor_bevel_depth = 24.0;
    state.shape_editor_bevel_width = 16.0;
    state.apply_shape2d_bevel();
    assert!(
        state.builder_brush.polys.len() > 6,
        "Beveled box must have beveled facets"
    );

    // 4. JSON Serialization & Deserialization
    let json = state.shape2d.to_json().expect("Must serialize to JSON");
    let loaded: Shape2D = Shape2D::from_json(&json).expect("Must deserialize from JSON");
    assert_eq!(loaded.points.len(), state.shape2d.points.len());
}
