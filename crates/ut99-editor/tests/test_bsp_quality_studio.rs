// crates/ut99-editor/tests/test_bsp_quality_studio.rs
//
// Integration tests for Interactive BSP Tree Quality, Node Cut Minimizer & Sliver Polygon Purging Studio.

use glam::Vec3;
use ut99_bsp::{build_cube, build_cylinder, CsgOperation, Polygon, PF_SEMISOLID};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_bsp_quality_audit_metrics_and_depth_histogram() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Create a subtractive main room
    let room = build_cube("Room_Sub", CsgOperation::Subtract, Vec3::new(1024.0, 1024.0, 512.0));
    state.world_brushes.push(room);

    // 2. Add two decorative pillars inside
    let mut pillar1 = build_cylinder("Pillar_1", CsgOperation::Add, 192.0, 48.0, 8);
    pillar1.location = Vec3::new(-200.0, 0.0, 0.0);
    state.world_brushes.push(pillar1);

    let mut pillar2 = build_cylinder("Pillar_2", CsgOperation::Add, 192.0, 48.0, 8);
    pillar2.location = Vec3::new(200.0, 0.0, 0.0);
    state.world_brushes.push(pillar2);

    // Run BSP quality audit
    state.run_bsp_quality_audit();
    let m = &state.bsp_quality_studio_settings.quality_metrics;

    assert!(m.total_input_brush_polys > 0, "Input brush polygons should be non-zero");
    assert!(m.total_world_bsp_polys >= m.total_input_brush_polys, "World BSP polys should be >= input polys");
    assert!(m.cut_ratio >= 1.0, "Cut ratio should be at least 1.0");
    assert!(m.max_tree_depth > 0, "Max tree depth should be > 0");
    assert!(m.max_tree_depth < 40, "Tree depth should be within safe Unreal Engine limits (< 40)");
    assert!(m.tree_balance_pct > 0.0 && m.tree_balance_pct <= 100.0, "Balance ratio should be 0..100%");

    // Verify histogram contains node depth distributions
    let total_hist_nodes: usize = m.depth_histogram.iter().sum();
    assert_eq!(total_hist_nodes, m.total_nodes, "Histogram sum must match total node count");
}

#[test]
fn test_sliver_detection_and_undo_redo_purging() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    let mut brush = ut99_bsp::Brush::new("SliverHazardBrush", CsgOperation::Add);

    // Healthy quad face
    let healthy_poly = Polygon::new(vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(256.0, 0.0, 0.0),
        Vec3::new(256.0, 256.0, 0.0),
        Vec3::new(0.0, 256.0, 0.0),
    ]).unwrap();
    brush.polys.push(healthy_poly);

    // Micro-sliver triangle (area ~0.25 uu², high aspect ratio)
    let sliver_poly = Polygon::new(vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(50.0, 0.005, 0.0),
    ]).unwrap();
    brush.polys.push(sliver_poly);

    state.world_brushes.push(brush);

    // 1. Audit detects sliver
    state.run_bsp_quality_audit();
    assert_eq!(state.bsp_quality_studio_settings.detected_slivers.len(), 1);
    assert!(state.bsp_quality_studio_settings.detected_slivers[0].area < 1.0);

    // 2. Purge slivers across level
    let purged = state.purge_all_sliver_polygons();
    assert_eq!(purged, 1, "Exactly 1 sliver polygon should have been purged");
    assert_eq!(state.world_brushes[0].polys.len(), 1, "Brush should only have 1 healthy quad left");
    assert_eq!(state.bsp_quality_studio_settings.detected_slivers.len(), 0);

    // 3. Undo restores sliver polygon
    state.undo();
    assert_eq!(state.world_brushes[0].polys.len(), 2, "Undo must restore sliver polygon");
    assert_eq!(state.bsp_quality_studio_settings.detected_slivers.len(), 1);

    // 4. Redo purges sliver polygon again
    state.redo();
    assert_eq!(state.world_brushes[0].polys.len(), 1, "Redo must re-purge sliver polygon");
    assert_eq!(state.bsp_quality_studio_settings.detected_slivers.len(), 0);
}

#[test]
fn test_coplanar_overlap_detection_and_z_fighting() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Brush A: Cube with top face at z = 128
    let mut cube_a = build_cube("PlatformA", CsgOperation::Add, Vec3::new(256.0, 256.0, 128.0));
    cube_a.location = Vec3::new(0.0, 0.0, 64.0);

    // Brush B: Adjacent slab sharing horizontal plane z = 0
    let mut cube_b = build_cube("PlatformB", CsgOperation::Add, Vec3::new(256.0, 256.0, 64.0));
    cube_b.location = Vec3::new(256.0, 0.0, 32.0); // bottom face at z = 0.0

    state.world_brushes.push(cube_a);
    state.world_brushes.push(cube_b);

    state.run_bsp_quality_audit();
    let coplanars = &state.bsp_quality_studio_settings.detected_coplanar;

    assert!(!coplanars.is_empty(), "Should detect coplanar overlap on horizontal plane");
    let match_cop = coplanars.iter().find(|c| c.plane.normal.z.abs() > 0.9);
    assert!(match_cop.is_some(), "Should find horizontal coplanar overlap");
    let cop = match_cop.unwrap();
    assert_ne!(cop.brush_a_index, cop.brush_b_index);
    assert!(cop.approximate_center.z.abs() <= 128.0);
}

#[test]
fn test_brush_cut_offenders_and_semisolid_conversion_undo_redo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Additive cylinder with 10 polygons
    let cylinder = build_cylinder("HighCutColumn", CsgOperation::Add, 256.0, 64.0, 8);
    state.world_brushes.push(cylinder);

    // 2. Audit finds cut offender
    state.run_bsp_quality_audit();
    let offenders = &state.bsp_quality_studio_settings.cut_offenders;
    assert!(!offenders.is_empty(), "High-poly cylinder should be listed as a cut offender");
    assert!(offenders[0].can_be_semisolid, "Additive brush should be eligible for SemiSolid conversion");

    // Check that brush polys do NOT yet have PF_SEMISOLID
    let initial_semi = state.world_brushes[0].polys.iter().all(|p| (p.flags & PF_SEMISOLID) != 0);
    assert!(!initial_semi, "Initial brush must not be SemiSolid");

    // 3. Convert top cut offender to Semi-Solid
    let converted = state.convert_top_cut_offenders_to_semisolid(1);
    assert_eq!(converted, 1, "Should convert 1 high-cut brush to SemiSolid");

    // Verify all polys now have PF_SEMISOLID flag
    let after_semi = state.world_brushes[0].polys.iter().all(|p| (p.flags & PF_SEMISOLID) != 0);
    assert!(after_semi, "All polygons must now have PF_SEMISOLID flag set");

    // 4. Undo restores original solid brush
    state.undo();
    let undone_semi = state.world_brushes[0].polys.iter().all(|p| (p.flags & PF_SEMISOLID) != 0);
    assert!(!undone_semi, "Undo must revert PF_SEMISOLID flag");

    // 5. Redo re-applies SemiSolid conversion
    state.redo();
    let redone_semi = state.world_brushes[0].polys.iter().all(|p| (p.flags & PF_SEMISOLID) != 0);
    assert!(redone_semi, "Redo must re-apply PF_SEMISOLID flag");
}
