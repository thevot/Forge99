use glam::Vec3;
use ut99_bsp::alignment::{align_polygon, pan_texture, rotate_texture, scale_texture, AlignmentMode};
use ut99_bsp::lighting::{
    evaluate_light_style, is_occluded, ut99_light_to_rgb, LightType, SceneLight,
};
use ut99_bsp::poly::Polygon;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_ut99_light_color_conversion() {
    // In UT99: Saturation 255 is pure desaturated white
    let white = ut99_light_to_rgb(0, 255, 255);
    assert!((white.x - 1.0).abs() < 1e-3);
    assert!((white.y - 1.0).abs() < 1e-3);
    assert!((white.z - 1.0).abs() < 1e-3);

    // Saturation 0 is pure chromatic color
    // Hue 0 is pure Red
    let red = ut99_light_to_rgb(0, 0, 255);
    assert!((red.x - 1.0).abs() < 1e-3);
    assert!(red.y < 1e-3);
    assert!(red.z < 1e-3);

    // Hue 85 is Green (85/255 * 360 ≈ 120°)
    let green = ut99_light_to_rgb(85, 0, 255);
    assert!(green.x < 0.05);
    assert!(green.y > 0.95);
    assert!(green.z < 0.05);

    // Half brightness
    let dim_red = ut99_light_to_rgb(0, 0, 128);
    assert!((dim_red.x - 0.50).abs() < 0.02);
}

#[test]
fn test_light_styles_evaluation() {
    let mut light = SceneLight {
        period: 32, // 1 second cycle
        ..Default::default()
    };

    // 1. Steady
    light.light_type = LightType::Steady as u8;
    assert_eq!(evaluate_light_style(&light, 0.0), 1.0);
    assert_eq!(evaluate_light_style(&light, 0.5), 1.0);

    // 2. Blink
    light.light_type = LightType::Blink as u8;
    assert_eq!(evaluate_light_style(&light, 0.25), 1.0);
    assert_eq!(evaluate_light_style(&light, 0.75), 0.0);

    // 3. Pulse (Cosine oscillation)
    light.light_type = LightType::Pulse as u8;
    let p0 = evaluate_light_style(&light, 0.0);
    let p_half = evaluate_light_style(&light, 0.5);
    assert!((p0 - 1.0).abs() < 1e-3);
    assert!(p_half < 0.05);

    // 4. Flicker
    light.light_type = LightType::Flicker as u8;
    let f1 = evaluate_light_style(&light, 0.1);
    let f2 = evaluate_light_style(&light, 0.6);
    assert!(f1 >= 0.35 && f1 <= 1.0);
    assert!(f2 >= 0.35 && f2 <= 1.0);
}

#[test]
fn test_raytraced_shadow_occlusion() {
    // Place a vertical occluding wall at X = 0 (from Y = -100 to 100, Z = -100 to 100)
    let wall_tri1 = [
        Vec3::new(0.0, -100.0, -100.0),
        Vec3::new(0.0, 100.0, -100.0),
        Vec3::new(0.0, 100.0, 100.0),
    ];
    let wall_tri2 = [
        Vec3::new(0.0, -100.0, -100.0),
        Vec3::new(0.0, 100.0, 100.0),
        Vec3::new(0.0, -100.0, 100.0),
    ];
    let occluders = vec![wall_tri1, wall_tri2];

    let light_pos = Vec3::new(100.0, 0.0, 0.0);

    // Point on same side (X = 50): Line of sight is clear
    let clear_pt = Vec3::new(50.0, 0.0, 0.0);
    assert!(!is_occluded(clear_pt, light_pos, Vec3::new(1.0, 0.0, 0.0), &occluders));

    // Point behind the wall (X = -50): Occluded by the wall
    let shadowed_pt = Vec3::new(-50.0, 0.0, 0.0);
    assert!(is_occluded(shadowed_pt, light_pos, Vec3::new(-1.0, 0.0, 0.0), &occluders));
}

#[test]
fn test_surface_alignment_modes() {
    // 1. Horizontal Floor Quad
    let floor_verts = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(256.0, 0.0, 0.0),
        Vec3::new(256.0, 256.0, 0.0),
        Vec3::new(0.0, 256.0, 0.0),
    ];
    let mut poly = Polygon::new(floor_verts).expect("valid polygon");

    // Align to Floor
    align_polygon(&mut poly, AlignmentMode::Floor);
    assert_eq!(poly.texture_u, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(poly.texture_v, Vec3::new(0.0, 1.0, 0.0));

    let uv0 = poly.compute_uv(Vec3::new(0.0, 0.0, 0.0));
    let uv1 = poly.compute_uv(Vec3::new(256.0, 256.0, 0.0));
    assert_eq!(uv0, [0.0, 0.0]);
    assert_eq!(uv1, [2.0, 2.0]); // 256 / 128 = 2.0 tiles

    // Align to Wall X
    align_polygon(&mut poly, AlignmentMode::WallX);
    assert_eq!(poly.texture_u, Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(poly.texture_v, Vec3::new(0.0, 0.0, -1.0));

    // Pan Texture
    let orig_base = poly.base;
    pan_texture(&mut poly, 32.0, 64.0);
    assert_ne!(poly.base, orig_base);

    // Scale Texture
    scale_texture(&mut poly, 2.0, 2.0);
    assert_eq!(poly.texture_u, Vec3::new(0.0, 2.0, 0.0));

    // Rotate Texture
    let len_before = poly.texture_u.length();
    rotate_texture(&mut poly, 45.0);
    let len_after = poly.texture_u.length();
    assert!((len_before - len_after).abs() < 1e-4);
}

#[test]
fn test_multithreaded_lighting_compiler() {
    let mut state = EditorState::default();

    // The default room has subtractive brushes
    assert!(!state.world_brushes.is_empty());

    // Build lighting across all brushes using Rayon
    state.build_lighting();

    let stats = state.last_lighting_stats.expect("Lighting stats should be populated");
    assert!(stats.total_polygons > 0);
    assert!(stats.total_luxels > 0);
    assert!(stats.shadow_rays_cast > 0);

    // Verify all brush polygons have baked vertex lighting
    for brush in &state.world_brushes {
        for poly in &brush.polys {
            assert_eq!(poly.vertex_colors.len(), poly.vertices.len());
            assert!(poly.lightmap.is_some());
            let lm = poly.lightmap.as_ref().unwrap();
            assert!(lm.width > 0 && lm.height > 0);
            assert_eq!(lm.luxels.len(), lm.width * lm.height);
        }
    }
}
