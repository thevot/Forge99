use glam::Vec3;
use ut99_bsp::{
    build_cube, build_mesh_lod_chain, generate_collision_hull, CollisionHullType, CsgOperation,
    PolygonBudgetRating,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_polygon_decimation_and_lod_chain_generation() {
    let cube = build_cube("ComplexMesh", CsgOperation::Add, Vec3::splat(256.0));
    let original_count = cube.polys.len();
    assert_eq!(original_count, 6);

    let chain = build_mesh_lod_chain("ComplexMesh", &cube.polys);
    assert_eq!(chain.lod0_polys.len(), original_count);
    assert!(chain.lod1_polys.len() <= chain.lod0_polys.len());
    assert!(chain.lod2_polys.len() <= chain.lod1_polys.len());
    assert!(chain.lod3_polys.len() <= chain.lod2_polys.len());
    assert!(!chain.lod3_polys.is_empty(), "LOD3 should retain at least basic bounding geometry");
}

#[test]
fn test_convex_collision_hull_synthesis() {
    let cube = build_cube("TargetMesh", CsgOperation::Add, Vec3::splat(256.0));

    // 1. AABB Hull (6 Planes)
    let aabb_hull = generate_collision_hull("TargetMesh_AABB", &cube.polys, CollisionHullType::AabbBox, 4.0)
        .expect("AABB hull must be generated");
    assert_eq!(aabb_hull.polys.len(), 6);

    // 2. KDOP-10 Hull (10 Planes: 2 caps + 8 side walls)
    let kdop10_hull = generate_collision_hull("TargetMesh_KDOP10", &cube.polys, CollisionHullType::Kdop10, 2.0)
        .expect("KDOP10 hull must be generated");
    assert_eq!(kdop10_hull.polys.len(), 10, "KDOP-10 must produce exactly 10 convex boundary faces");

    // 3. KDOP-18 Hull
    let kdop18_hull = generate_collision_hull("TargetMesh_KDOP18", &cube.polys, CollisionHullType::Kdop18, 2.0)
        .expect("KDOP18 hull must be generated");
    assert!(!kdop18_hull.polys.is_empty());
}

#[test]
fn test_polygon_budget_rating_evaluator() {
    assert_eq!(PolygonBudgetRating::from_poly_count(150), PolygonBudgetRating::Optimal);
    assert_eq!(PolygonBudgetRating::from_poly_count(650), PolygonBudgetRating::Acceptable);
    assert_eq!(PolygonBudgetRating::from_poly_count(1800), PolygonBudgetRating::HeavyBudget);
    assert_eq!(PolygonBudgetRating::from_poly_count(3200), PolygonBudgetRating::CriticalHazard);
}

#[test]
fn test_lod_collision_bake_and_undo_redo_workflow() {
    let mut state = EditorState::default();

    // Setup an active world brush
    let cube = build_cube("TestProp", CsgOperation::Add, Vec3::splat(200.0));
    state.world_brushes.push(cube);
    state.selected_brush_index = Some(0);

    // 1. Bake Collision Hull to World Brush
    let initial_brush_count = state.world_brushes.len();
    let baked_idx = state.bake_collision_hull_as_brush(CollisionHullType::Kdop10, 4.0)
        .expect("Collision hull bake must succeed");
    assert_eq!(baked_idx, initial_brush_count);
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);

    // 2. Undo Collision Hull Bake
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brush_count, "Undo must remove collision hull brush");

    // 3. Redo Collision Hull Bake
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1, "Redo must restore collision hull brush");

    // 4. Test LOD Application to Selected Brush
    state.selected_brush_index = Some(0);
    let original_poly_len = state.world_brushes[0].polys.len();
    let lod_success = state.apply_lod_to_selected_brush(2);
    assert!(lod_success);

    // 5. Undo LOD change
    state.undo();
    assert_eq!(state.world_brushes[0].polys.len(), original_poly_len, "Undo must restore original brush polygons");
}
