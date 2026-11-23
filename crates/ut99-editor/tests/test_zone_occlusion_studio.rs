use glam::Vec3;
use ut99_bsp::{
    generate_portal_sheet_brush, AntiPortalVolume, ZoneOcclusionAudit, ZoneOcclusionPreset,
    PF_NOT_SOLID, PF_PORTAL,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_antiportal_shadow_volume_occlusion() {
    let camera_pos = Vec3::new(0.0, 0.0, 0.0);
    // Antiportal located at Y=500, size 128x128x128
    let mut anti = AntiPortalVolume::new(
        1,
        "AP_Shield_01",
        Vec3::new(0.0, 500.0, 0.0),
        Vec3::splat(128.0),
    );

    // 1. Angular shadow cone degrees
    let cone_deg = anti.compute_shadow_cone_degrees(camera_pos);
    assert!(
        cone_deg > 25.0 && cone_deg < 32.0,
        "Shadow cone angle expected ~28.7 deg, got {cone_deg}"
    );

    // 2. Point occlusion:
    // Point directly behind antiportal at Y=1000 -> must be occluded
    assert!(
        anti.occludes_point(camera_pos, Vec3::new(0.0, 1000.0, 0.0)),
        "Point behind antiportal along line of sight must be occluded"
    );

    // Point in front of antiportal at Y=250 -> must NOT be occluded
    assert!(
        !anti.occludes_point(camera_pos, Vec3::new(0.0, 250.0, 0.0)),
        "Point in front of antiportal must not be occluded"
    );

    // Point off to the side at X=400, Y=1000 -> must NOT be occluded
    assert!(
        !anti.occludes_point(camera_pos, Vec3::new(400.0, 1000.0, 0.0)),
        "Point outside shadow frustum must not be occluded"
    );

    // 3. Sphere occlusion:
    // Small target sphere (radius 32) at Y=1000 -> fits inside shadow cone
    assert!(
        anti.occludes_sphere(camera_pos, Vec3::new(0.0, 1000.0, 0.0), 32.0),
        "Small sphere fitting inside shadow cone must be occluded"
    );

    // Large target sphere (radius 400) at Y=1000 -> spills outside shadow cone
    assert!(
        !anti.occludes_sphere(camera_pos, Vec3::new(0.0, 1000.0, 0.0), 400.0),
        "Large sphere exceeding shadow cone must not be occluded"
    );

    // 4. Inactive antiportal:
    anti.is_active = false;
    assert!(
        !anti.occludes_point(camera_pos, Vec3::new(0.0, 1000.0, 0.0)),
        "Deactivated antiportal must never occlude points"
    );
}

#[test]
fn test_generate_portal_sheet_brush() {
    let center = Vec3::new(128.0, 256.0, 64.0);
    let brush = generate_portal_sheet_brush(center, 128.0, 256.0, Vec3::X, "DoorwayPortal");

    assert_eq!(brush.name, "DoorwayPortal");
    assert_eq!(brush.location, center);
    assert_eq!(brush.polys.len(), 2, "Portal sheet must generate 2 polygons (front & back)");

    for poly in &brush.polys {
        assert_eq!(poly.texture, "ZonePortal");
        assert_eq!(
            poly.flags & (PF_PORTAL | PF_NOT_SOLID),
            PF_PORTAL | PF_NOT_SOLID,
            "Polygon must have PF_PORTAL and PF_NOT_SOLID set"
        );
        assert_eq!(poly.vertices.len(), 4, "Portal sheet polygon must have 4 quad vertices");
    }
}

#[test]
fn test_zone_occlusion_presets_and_audit() {
    let presets = ZoneOcclusionPreset::all();
    assert_eq!(presets.len(), 4);

    for &p in presets {
        assert!(!p.display_name().is_empty());
        assert!(!p.description().is_empty());
    }

    let audit = ZoneOcclusionAudit {
        total_zones: 5,
        watertight_zones: 5,
        leaking_zones: 0,
        total_portals: 8,
        active_antiportals: 2,
        occlusion_efficiency_pct: 64.0,
        is_tournament_ready: true,
        summary_verdict: "100% Watertight".to_string(),
    };

    assert!(audit.is_tournament_ready);
    assert_eq!(audit.leaking_zones, 0);
}

#[test]
fn test_editor_portal_sheet_and_antiportal_placement_with_undo_redo() {
    let mut state = EditorState::default();
    state.builder_brush.location = Vec3::new(512.0, 1024.0, 256.0);

    let initial_brush_count = state.world_brushes.len();
    let initial_antiportal_count = state.antiportals.len();

    // 1. Create Zone Portal Sheet
    let brush_idx = state.create_portal_sheet_at_pivot(128.0, 256.0, Vec3::Y);
    assert_eq!(brush_idx, initial_brush_count);
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);

    let portal_brush = &state.world_brushes[brush_idx];
    assert!(portal_brush.name.starts_with("ZonePortalSheet_"));
    assert_eq!(portal_brush.location, Vec3::new(512.0, 1024.0, 256.0));
    assert!(portal_brush.polys.iter().any(|p| (p.flags & PF_PORTAL) != 0));

    // Test Undo portal sheet creation
    assert!(state.history.can_undo());
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brush_count);

    // Test Redo portal sheet creation
    assert!(state.history.can_redo());
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);

    // 2. Add AntiPortal Volume
    let anti_idx = state.add_antiportal_volume_at_pivot(Vec3::new(512.0, 256.0, 512.0));
    assert_eq!(anti_idx, initial_antiportal_count);
    assert_eq!(state.antiportals.len(), initial_antiportal_count + 1);

    let anti = &state.antiportals[anti_idx];
    assert_eq!(anti.location, Vec3::new(512.0, 1024.0, 256.0));
    assert_eq!(anti.size, Vec3::new(512.0, 256.0, 512.0));
    assert!(anti.is_active);

    // Test Undo antiportal addition
    assert!(state.history.can_undo());
    state.undo();
    assert_eq!(state.antiportals.len(), initial_antiportal_count);

    // Test Redo antiportal addition
    assert!(state.history.can_redo());
    state.redo();
    assert_eq!(state.antiportals.len(), initial_antiportal_count + 1);
}

#[test]
fn test_editor_analyze_zone_occlusion_audit() {
    let mut state = EditorState::default();

    // Add a portal sheet brush and an antiportal volume
    state.create_portal_sheet_at_pivot(128.0, 256.0, Vec3::X);
    state.add_antiportal_volume_at_pivot(Vec3::new(256.0, 256.0, 256.0));

    let audit = state.analyze_zone_occlusion();
    assert!(audit.total_zones >= 1);
    assert!(audit.total_portals >= 2);
    assert_eq!(audit.active_antiportals, 1);
    assert!(state.zone_pvs_matrix.is_some());
    assert!(state.zone_occlusion_settings.last_audit_report.is_some());
}
