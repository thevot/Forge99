use glam::{Vec2, Vec3};
use ut99_bsp::{
    CsgOperation, TerrainGrid, TerrainPreset, TerrainSculptMode, TerrainSlopeCategory,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_terrain_grid_creation_and_sculpt_stroke() {
    let mut grid = TerrainGrid::new(8, 8, 128.0, 0.0);
    assert_eq!(grid.heights.len(), 9 * 9);

    let center_pos = grid.vertex_pos(4, 4);
    assert_eq!(center_pos.x, 0.0);
    assert_eq!(center_pos.y, 0.0);
    assert_eq!(center_pos.z, 0.0);

    // Sculpt Raise at center
    grid.sculpt(Vec2::ZERO, 256.0, 64.0, TerrainSculptMode::Raise, 0.0);

    let center_h = grid.get_height(4, 4);
    assert!((center_h - 64.0).abs() < 1e-2, "Center must be elevated to ~64.0 uu (got {})", center_h);

    // Far corner must remain at 0.0
    let corner_h = grid.get_height(0, 0);
    assert_eq!(corner_h, 0.0, "Corner outside brush radius must remain untouched");
}

#[test]
fn test_terrain_smooth_and_plateau_flattening() {
    let mut grid = TerrainGrid::new(8, 8, 128.0, 0.0);

    // Create a sharp single spike at center
    grid.set_height(4, 4, 200.0);

    // Smooth stroke
    grid.sculpt(Vec2::ZERO, 300.0, 0.8, TerrainSculptMode::Smooth, 0.0);
    let smoothed_peak = grid.get_height(4, 4);
    assert!(smoothed_peak < 200.0, "Smooth must diffuse and lower sharp peak (got {})", smoothed_peak);
    let neighbor_h = grid.get_height(4, 5);
    assert!(neighbor_h > 0.0, "Smooth must elevate adjacent neighbor (got {})", neighbor_h);

    // Flatten to plateau at Z = 75.0
    grid.sculpt(Vec2::ZERO, 300.0, 1.0, TerrainSculptMode::Flatten, 75.0);
    let plateau_h = grid.get_height(4, 4);
    assert!((plateau_h - 75.0).abs() < 5.0, "Flatten must converge center toward target 75.0 uu (got {})", plateau_h);
}

#[test]
fn test_terrain_presets_and_slope_evaluation() {
    let mut grid = TerrainGrid::new(12, 12, 128.0, 0.0);
    grid.apply_preset(TerrainPreset::ArenaCraterBasin);

    let (min_z, max_z) = grid.min_max_elevation();
    assert!(min_z < -50.0, "Crater basin must have depressed floor (got min_z: {})", min_z);
    assert!(max_z >= 0.0, "Crater rim must reach ground or higher (got max_z: {})", max_z);

    // Evaluate flat horizontal triangle
    let v0 = Vec3::new(0.0, 0.0, 0.0);
    let v1 = Vec3::new(100.0, 0.0, 0.0);
    let v2 = Vec3::new(0.0, 100.0, 0.0);
    let (slope_deg, cat) = grid.evaluate_triangle_slope(v0, v1, v2);
    assert!(slope_deg < 1.0);
    assert_eq!(cat, TerrainSlopeCategory::FlatWalkable);

    // Evaluate steep wall triangle (slope ~ 90 deg)
    let s0 = Vec3::new(0.0, 0.0, 0.0);
    let s1 = Vec3::new(0.0, 100.0, 0.0);
    let s2 = Vec3::new(0.0, 0.0, 100.0);
    let (wall_slope, wall_cat) = grid.evaluate_triangle_slope(s0, s1, s2);
    assert!(wall_slope > 80.0);
    assert_eq!(wall_cat, TerrainSlopeCategory::SteepCliff);
}

#[test]
fn test_terrain_bake_to_world_and_undo_redo() {
    let mut state = EditorState::default();

    // 1. Apply preset and generate polygons
    state.apply_terrain_preset(TerrainPreset::HighlandsFortressPlateau);
    state.synthesize_terrain_to_builder();

    let expected_triangles = state.terrain_settings.grid.rows * state.terrain_settings.grid.cols * 2;
    assert!(state.builder_brush.polys.len() >= expected_triangles, "Must contain all terrain surface triangles");

    // 2. Bake to World Brush
    let initial_count = state.world_brushes.len();
    let baked_idx = state.bake_terrain_to_world(CsgOperation::Add);
    assert_eq!(baked_idx, initial_count);
    assert_eq!(state.world_brushes.len(), initial_count + 1);

    // 3. Undo and Redo verification
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_count, "Undo must remove terrain brush");

    state.redo();
    assert_eq!(state.world_brushes.len(), initial_count + 1, "Redo must restore terrain brush");
}
