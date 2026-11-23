use glam::Vec3;
use ut99_bsp::{build_cube, build_curved_staircase, CsgOperation, NavCoverageRating, PF_FAKE_BACKDROP};
use ut99_editor::editor_state::{EditorMode, EditorState};
use ut99_format::ActorRecord;

#[test]
fn test_skyzone_and_fake_backdrop_visualizer() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Create a room brush with a ceiling flagged as PF_FAKE_BACKDROP
    let mut ceiling_brush = build_cube("SkyCeiling", CsgOperation::Add, Vec3::new(256.0, 256.0, 16.0));
    ceiling_brush.location = Vec3::new(0.0, 0.0, 500.0);
    // Flag the bottom face as Fake Backdrop
    ceiling_brush.polys[0].flags |= PF_FAKE_BACKDROP;
    state.world_brushes.push(ceiling_brush);

    assert!(state.is_brush_fake_backdrop(0), "Brush 0 must be identified as Fake Backdrop");

    // Before SkyZoneInfo actor exists, fallback anchor link should be formed
    let links_before = state.get_skybox_links();
    assert_eq!(links_before.len(), 1, "Should detect 1 backdrop surface link");
    assert_eq!(links_before[0].brush_index, 0);

    // Place a SkyZoneInfo actor in a detached skybox arena
    let mut actor = ActorRecord::default();
    actor.name = "SkyZoneInfo0".to_string();
    actor.class_name = "SkyZoneInfo".to_string();
    actor.location = Vec3::new(10000.0, 10000.0, -5000.0);
    if let Some(ref mut unr) = state.loaded_unr {
        unr.actors.push(actor);
    } else {
        let mut unr = ut99_format::UnrLevel::default();
        unr.actors.push(actor);
        state.loaded_unr = Some(unr);
    }

    let links_after = state.get_skybox_links();
    assert_eq!(links_after.len(), 1);
    assert_eq!(links_after[0].skyzone_name, "SkyZoneInfo0");
    assert_eq!(links_after[0].skyzone_location, Vec3::new(10000.0, 10000.0, -5000.0));
    assert!(links_after[0].backdrop_centroid.z > 400.0);
}

#[test]
fn test_vertex_grid_snap() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    let mut cube = build_cube("TestCube", CsgOperation::Add, Vec3::new(100.0, 100.0, 100.0));
    cube.location = Vec3::new(0.0, 0.0, 0.0);
    state.world_brushes.push(cube);
    state.selected_brush_index = Some(0);
    state.editor_mode = EditorMode::VertexEdit;

    // Perturb the first vertex to an off-grid coordinate: (53.7, 49.2, 51.1)
    state.world_brushes[0].polys[0].vertices[0] = Vec3::new(53.7, 49.2, 51.1);

    // Select vertex 0
    state.selected_vertex_indices = vec![0];
    state.grid_step = 32.0;

    let snapped = state.snap_selected_vertices_to_grid();
    assert!(snapped >= 1, "Should snap at least 1 vertex occurrence");

    let unique = state.get_active_brush_unique_vertices();
    let snapped_v = unique[0];
    assert_eq!(snapped_v.x % 32.0, 0.0, "X must be a multiple of 32");
    assert_eq!(snapped_v.y % 32.0, 0.0, "Y must be a multiple of 32");
    assert_eq!(snapped_v.z % 32.0, 0.0, "Z must be a multiple of 32");

    // Test Undo
    assert!(state.history.can_undo());
    state.undo();
    let unique_restored = state.get_active_brush_unique_vertices();
    assert!((unique_restored[0].x - 53.7).abs() < 0.1, "Undo should restore off-grid vertex");
}

#[test]
fn test_magnetic_vertex_weld() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    let mut b0 = build_cube("Brush0", CsgOperation::Add, Vec3::new(100.0, 100.0, 100.0));
    b0.location = Vec3::new(0.0, 0.0, 0.0);
    state.world_brushes.push(b0);

    let mut b1 = build_cube("Brush1", CsgOperation::Add, Vec3::new(100.0, 100.0, 100.0));
    // Placed adjacent, but vertex is off by 6 uu (within 16 uu tolerance)
    b1.location = Vec3::new(106.0, 0.0, 0.0);
    state.world_brushes.push(b1);

    state.selected_brush_index = Some(1);
    state.editor_mode = EditorMode::VertexEdit;

    let unique = state.get_active_brush_unique_vertices();
    // Select all vertices of Brush 1
    state.selected_vertex_indices = (0..unique.len()).collect();

    let welded = state.weld_selected_vertices_to_nearest(16.0);
    assert!(welded > 0, "Should weld vertices within 16 uu tolerance");

    // Verify distance between welded vertices and brush 0 target is 0.0
    let b0_unique = state.world_brushes[0].polys.iter().flat_map(|p| p.vertices.iter()).map(|v| *v + state.world_brushes[0].location).collect::<Vec<_>>();
    let b1_unique_after = state.get_active_brush_unique_vertices();

    let mut found_zero_distance = false;
    for v1 in &b1_unique_after {
        for v0 in &b0_unique {
            if v1.distance(*v0) < 0.01 {
                found_zero_distance = true;
                break;
            }
        }
    }
    assert!(found_zero_distance, "Welded vertex must match Brush 0 vertex exactly");
}

#[test]
fn test_edge_split() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    let mut cube = build_cube("SplitCube", CsgOperation::Add, Vec3::new(100.0, 100.0, 100.0));
    cube.location = Vec3::new(0.0, 0.0, 0.0);
    state.world_brushes.push(cube);
    state.selected_brush_index = Some(0);
    state.editor_mode = EditorMode::VertexEdit;

    let unique = state.get_active_brush_unique_vertices();
    assert_eq!(unique.len(), 8, "Cube must have 8 unique vertices");

    // Find two vertices that form an actual polygon edge
    let mut edge_pair = None;
    for poly in &state.world_brushes[0].polys {
        let n = poly.vertices.len();
        for i in 0..n {
            let v0 = poly.vertices[i];
            let v1 = poly.vertices[(i + 1) % n];
            let idx0 = unique.iter().position(|u| u.distance(v0) < 0.01).unwrap();
            let idx1 = unique.iter().position(|u| u.distance(v1) < 0.01).unwrap();
            edge_pair = Some((idx0, idx1, (v0 + v1) * 0.5));
            break;
        }
        if edge_pair.is_some() {
            break;
        }
    }

    let (i0, i1, expected_midpoint) = edge_pair.expect("Cube must have at least one edge");
    state.selected_vertex_indices = vec![i0, i1];

    let success = state.split_selected_edge();
    assert!(success, "Edge split must succeed for valid edge");

    let unique_after = state.get_active_brush_unique_vertices();
    assert_eq!(unique_after.len(), 9, "Cube should now have 9 unique vertices after edge split");
    assert!(unique_after.iter().any(|u| u.distance(expected_midpoint) < 0.01), "New vertex must match midpoint");

    // Non-edge pair should fail
    state.selected_vertex_indices = vec![0, 0]; // Invalid pair (same vertex)
    assert!(!state.split_selected_edge());
}

#[test]
fn test_bot_navigation_coverage_heatmap() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Create a large arena room floor: 1000 x 1000 uu
    let mut floor_brush = build_cube("ArenaFloor", CsgOperation::Subtract, Vec3::new(1000.0, 1000.0, 20.0));
    floor_brush.location = Vec3::new(0.0, 0.0, 0.0);
    state.world_brushes.push(floor_brush);

    // Place a single PathNode at (0, 0, 10)
    state.nav_graph.add_node("PathNode0", Vec3::new(0.0, 0.0, 10.0), "PathNode");

    let report = state.compute_nav_coverage();
    assert!(report.total_walkable_area_points > 0, "Must have sampled walkable floor points");
    assert!(report.optimal_count > 0, "Points near center node must be Optimal (<400 uu)");
    assert!(report.samples.iter().any(|s| s.rating == NavCoverageRating::Optimal));

    // Add a distant second room floor at (2500, 0, 0) without any path nodes
    let mut far_floor = build_cube("FarFloor", CsgOperation::Subtract, Vec3::new(200.0, 200.0, 20.0));
    far_floor.location = Vec3::new(2500.0, 0.0, 0.0);
    state.world_brushes.push(far_floor);

    let report2 = state.compute_nav_coverage();
    assert!(report2.dead_zone_count > 0, "Far room without nodes must produce DeadZone samples (>800 uu)");
    assert!(report2.coverage_percentage < 100.0, "Coverage percentage must reflect unnavigable dead zones");
    assert!(report2.samples.iter().any(|s| s.rating == NavCoverageRating::DeadZone));
}

#[test]
fn test_texture_auto_fit_and_continuous_strip() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Auto-fit test
    let mut wall_brush = build_cube("WideWall", CsgOperation::Add, Vec3::new(512.0, 32.0, 256.0));
    wall_brush.location = Vec3::new(0.0, 0.0, 0.0);
    state.world_brushes.push(wall_brush);
    state.selected_brush_index = Some(0);

    state.fit_selected_brush_texture(1.0, 1.0);
    let front_poly = &state.world_brushes[0].polys[0];
    assert!(front_poly.texture_u.length() > 0.0, "Texture U must be valid");
    assert!(front_poly.texture_v.length() > 0.0, "Texture V must be valid");

    // 2. Continuous strip alignment test on curved staircase
    let mut curved_stairs = build_curved_staircase(
        "CurvedStripTest",
        CsgOperation::Add,
        8,      // steps
        16.0,   // step_height
        128.0,  // inner_radius
        256.0,  // step_width
        90.0,   // total_angle
        false,  // clockwise
    );
    curved_stairs.location = Vec3::new(200.0, 200.0, 0.0);
    state.world_brushes.push(curved_stairs);
    state.selected_brush_index = Some(1);

    state.align_selected_brush_continuous_strip(1.0, 1.0);
    let stair_brush = &state.world_brushes[1];
    let wall_polys: Vec<_> = stair_brush.polys.iter().filter(|p| p.plane.normal.z.abs() < 0.85).collect();
    assert!(!wall_polys.is_empty(), "Curved stairs must have lateral wall polygons");
    for wp in wall_polys {
        assert!(wp.texture_u.length() > 0.0, "Wall polygon must have non-zero continuous U tangent");
        assert!(wp.texture_v.length() > 0.0, "Wall polygon must have non-zero V bitangent");
    }
}
