use glam::Vec3;
use ut99_bsp::{
    build_cube, CsgOperation, CtfSymmetryConfig, CtfSymmetryMode,
    LightSourceShape, PF_PORTAL,
};
use ut99_editor::editor_state::EditorState;

/// Audit Test Suite for Fase 15 (..040)
/// Zero-warning, 100% industrial test verification

#[test]
fn test_audit_pro037_bot_combat_and_sightlines() {
    let mut state = EditorState::default();

    // Clear initial demo room and setup high sniper perch and low ground targets
    state.world_brushes.clear();

    state.nav_graph.add_node("SniperTower", Vec3::new(0.0, 0.0, 400.0), "PathNode");
    state.nav_graph.add_node("CourtyardMid", Vec3::new(1000.0, 0.0, 0.0), "PathNode");
    state.nav_graph.add_node("CourtyardFar", Vec3::new(2200.0, 0.0, 0.0), "PathNode");

    // Add a solid wall occluding the far target from the courtyard
    let mut wall = build_cube("OccluderWall", CsgOperation::Add, Vec3::new(64.0, 400.0, 600.0));
    wall.location = Vec3::new(1600.0, 0.0, 100.0);
    state.world_brushes.push(wall);

    let report = state.compute_combat_sightlines();

    assert!(state.combat_coverage_report.is_some());
    // Sightline from tower to CourtyardMid should be clear
    let direct_mid = report.sightlines.iter().find(|s| {
        (s.from_node == 0 && s.to_node == 1) || (s.from_node == 1 && s.to_node == 0)
    });
    assert!(direct_mid.is_some(), "Tower should have clear line of sight to CourtyardMid");

    // Sightline from CourtyardMid to CourtyardFar should be blocked by OccluderWall
    let occluded_far = report.sightlines.iter().find(|s| {
        (s.from_node == 1 && s.to_node == 2) || (s.from_node == 2 && s.to_node == 1)
    });
    assert!(occluded_far.is_none(), "Wall must occlude line of sight between mid and far courtyard");

    // Sniper tower has significant elevation advantage
    assert!(report.high_ground_perches_count >= 1);
    let tower_vantage = report.vantage_points.iter().find(|v| v.node_index == 0).unwrap();
    assert!(tower_vantage.elevation_advantage_uu >= 400.0);
}

#[test]
fn test_audit_pro038_zone_pvs_cell_occlusion() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Create a brush representing a Zone Portal sheet
    let mut portal_brush = build_cube("ZonePortal0", CsgOperation::Add, Vec3::new(16.0, 256.0, 256.0));
    portal_brush.location = Vec3::new(500.0, 0.0, 0.0);
    for p in &mut portal_brush.polys {
        p.flags |= PF_PORTAL;
    }
    state.world_brushes.push(portal_brush);

    let matrix = state.compute_zone_pvs();

    assert!(state.zone_pvs_matrix.is_some());
    assert!(matrix.zone_count >= 2);
    // Zone 0 and Zone 1 are connected by portal
    assert!(matrix.is_visible(0, 0));
    assert!(matrix.is_visible(0, 1));
    assert!(matrix.is_visible(1, 0));
}

#[test]
fn test_audit_pro039_soft_penumbra_shadow_profiler() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Place an occluder brush partially blocking a light source above
    let mut occluder = build_cube("OccluderSlab", CsgOperation::Add, Vec3::new(400.0, 400.0, 32.0));
    occluder.location = Vec3::new(200.0, 0.0, 150.0); // Blocks x >= 0, leaving x < 0 unoccluded
    state.world_brushes.push(occluder);

    let surface_pt = Vec3::new(0.0, 0.0, 0.0);
    let surface_norm = Vec3::Z;
    let light_pos = Vec3::new(0.0, 0.0, 300.0);

    let profile = ut99_bsp::evaluate_brush_shadow_gradient(
        surface_pt,
        surface_norm,
        light_pos,
        LightSourceShape::Sphere { radius: 120.0 },
        &state.world_brushes,
        32,
    );

    assert!(profile.sample_count == 32);
    assert!(profile.visible_sample_count > 0);
    assert!(profile.visible_sample_count < 32);
    assert!(profile.is_in_penumbra);
    assert!(profile.penumbra_factor > 0.0 && profile.penumbra_factor < 1.0);
    assert!(profile.first_blocker_dist.is_some());
    assert!(profile.penumbra_softness_uu > 0.0);
}

#[test]
fn test_audit_pro040_symmetrical_ctf_base_generator() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Create a Red Base brush
    let mut red_base = build_cube("Red_FlagHall", CsgOperation::Add, Vec3::new(128.0, 512.0, 256.0));
    red_base.location = Vec3::new(800.0, 400.0, 0.0);
    for p in &mut red_base.polys {
        p.texture = "cf_redbrick".to_string();
    }
    state.world_brushes.push(red_base);

    // 2. Generate Blue Base with Rotational 180 symmetry around origin
    let rot_config = CtfSymmetryConfig {
        mode: CtfSymmetryMode::Rotational180,
        center: Vec3::ZERO,
        remap_textures: true,
        prefix_filter: None,
    };
    let rot_res = state.apply_symmetrical_ctf_base(&rot_config);

    assert_eq!(rot_res.transformed_brush_count, 1);
    assert!(rot_res.remapped_texture_polys_count > 0);
    assert_eq!(state.world_brushes.len(), 2);

    let blue_rot = &state.world_brushes[1];
    assert_eq!(blue_rot.name, "Blue_FlagHall");
    assert!((blue_rot.location - Vec3::new(-800.0, -400.0, 0.0)).length() < 1e-3);
    assert_eq!(blue_rot.polys[0].texture, "cf_bluebrick");

    // 3. Test MirrorX mode
    let mirror_config = CtfSymmetryConfig {
        mode: CtfSymmetryMode::MirrorX,
        center: Vec3::ZERO,
        remap_textures: true,
        prefix_filter: Some("Red_".to_string()),
    };
    let mirror_res = ut99_bsp::generate_symmetrical_ctf_base(&state.world_brushes[0..1], &mirror_config);

    assert_eq!(mirror_res.transformed_brush_count, 1);
    assert_eq!(mirror_res.inverted_winding_polys_count, 6); // 6 cube faces inverted to preserve outward normal
    let blue_mirror = &mirror_res.generated_brushes[0];
    assert_eq!(blue_mirror.name, "Blue_FlagHall");
    assert!((blue_mirror.location - Vec3::new(-800.0, 400.0, 0.0)).length() < 1e-3);
}
