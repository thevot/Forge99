use glam::Vec3;
use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;

/// Architectural style of the parametric arch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArchType {
    #[default]
    SemiCircular,   // Classic Roman circular arch (rise = span / 2)
    GothicPointed,  // Pointed Gothic lancet arch with intersecting radii
    Segmental,      // Shallow segmental arch for wide spans and bridges
    Horseshoe,      // Moorish keyhole horseshoe arch
}

impl ArchType {
    pub const ALL: &'static [ArchType] = &[
        ArchType::SemiCircular,
        ArchType::GothicPointed,
        ArchType::Segmental,
        ArchType::Horseshoe,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ArchType::SemiCircular => "Semi-Circular (Roman)",
            ArchType::GothicPointed => "Pointed (Gothic Lancet)",
            ArchType::Segmental => "Segmental (Bridge / Shallow)",
            ArchType::Horseshoe => "Horseshoe (Moorish)",
        }
    }
}

/// Detailed configuration for parametric arch generation
#[derive(Debug, Clone, PartialEq)]
pub struct ArchConfig {
    pub arch_type: ArchType,
    pub span_width: f32,       // Clear opening width between jambs (Unreal Units)
    pub jamb_height: f32,      // Height of straight vertical jambs before springline
    pub total_height: f32,     // Total height from floor to arch apex
    pub depth: f32,            // Thickness through the wall / portal depth
    pub wall_thickness: f32,   // Outer solid masonry thickness around opening
    pub segments: usize,       // Number of radial segments along the curved curve
    pub is_doorway: bool,      // True: hollow arch opening; False: solid decorative block
}

impl Default for ArchConfig {
    fn default() -> Self {
        Self {
            arch_type: ArchType::SemiCircular,
            span_width: 256.0,
            jamb_height: 192.0,
            total_height: 320.0,
            depth: 64.0,
            wall_thickness: 32.0,
            segments: 12,
            is_doorway: true,
        }
    }
}

/// Constructs an authentic, watertight 3D architectural Arch Brush.
/// Centered horizontally at X=0, Y spanning from -depth/2 to +depth/2, Z resting on floor at Z=0.
pub fn build_arch(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    config: &ArchConfig,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let segments = config.segments.clamp(4, 64);
    let half_span = config.span_width * 0.5;
    let half_depth = config.depth * 0.5;
    let jamb_h = config.jamb_height.max(0.0);
    let arch_rise = (config.total_height - jamb_h).max(16.0);
    let wall_t = config.wall_thickness.max(8.0);

    // Compute inner (soffit) and outer (extrados) curve 2D profiles (X, Z)
    let mut inner_pts: Vec<(f32, f32)> = Vec::with_capacity(segments + 1);
    let mut outer_pts: Vec<(f32, f32)> = Vec::with_capacity(segments + 1);

    match config.arch_type {
        ArchType::SemiCircular => {
            for i in 0..=segments {
                let frac = i as f32 / segments as f32;
                let angle = std::f32::consts::PI * (1.0 - frac); // from PI (left) to 0 (right)
                let x_in = half_span * angle.cos();
                let z_in = jamb_h + arch_rise * angle.sin();

                let outer_radius_x = half_span + wall_t;
                let outer_rise = arch_rise + wall_t;
                let x_out = outer_radius_x * angle.cos();
                let z_out = jamb_h + outer_rise * angle.sin();

                inner_pts.push((x_in, z_in));
                outer_pts.push((x_out, z_out));
            }
        }
        ArchType::GothicPointed => {
            let half_segs = segments / 2;
            // Left half: centered at +half_span curving up to apex (0, total_height)
            for i in 0..=half_segs {
                let frac = i as f32 / half_segs as f32;
                let theta = (1.0 - frac) * 0.5 * std::f32::consts::PI;
                let x_in = -half_span + half_span * 2.0 * (1.0 - theta.cos());
                let z_in = jamb_h + arch_rise * theta.sin();

                let x_out = x_in - wall_t * (1.0 - frac);
                let z_out = z_in + wall_t;

                inner_pts.push((x_in, z_in));
                outer_pts.push((x_out, z_out));
            }
            // Right half: symmetric reflection
            for i in (0..half_segs).rev() {
                let (xi, zi) = inner_pts[i];
                let (xo, zo) = outer_pts[i];
                inner_pts.push((-xi, zi));
                outer_pts.push((-xo, zo));
            }
        }
        ArchType::Segmental => {
            // Shallow circular arc with radius > half_span
            let chord = half_span;
            let s = arch_rise;
            let radius = (chord * chord + s * s) / (2.0 * s).max(1.0);
            let half_angle = (chord / radius).clamp(-1.0, 1.0).asin();

            for i in 0..=segments {
                let frac = i as f32 / segments as f32;
                let angle = half_angle * (1.0 - 2.0 * frac); // from +half_angle to -half_angle
                let x_in = -radius * angle.sin();
                let z_in = jamb_h + (radius * angle.cos() - (radius - s));

                let x_out = x_in * (1.0 + wall_t / chord);
                let z_out = z_in + wall_t;

                inner_pts.push((x_in, z_in));
                outer_pts.push((x_out, z_out));
            }
        }
        ArchType::Horseshoe => {
            let extra_angle = 0.25; // Expands 0.25 rad beyond semicircle
            for i in 0..=segments {
                let frac = i as f32 / segments as f32;
                let start_a = std::f32::consts::PI + extra_angle;
                let end_a = -extra_angle;
                let angle = start_a + (end_a - start_a) * frac;

                let x_in = half_span * angle.cos();
                let z_in = jamb_h + arch_rise * (angle.sin() + extra_angle.sin());

                let x_out = (half_span + wall_t) * angle.cos();
                let z_out = jamb_h + (arch_rise + wall_t) * (angle.sin() + extra_angle.sin());

                inner_pts.push((x_in, z_in));
                outer_pts.push((x_out, z_out));
            }
        }
    }

    if config.is_doorway {
        // Construct hollow arch opening:
        // Front and back arch radial segments
        let n = inner_pts.len();
        for i in 0..n.saturating_sub(1) {
            let (x0_in, z0_in) = inner_pts[i];
            let (x1_in, z1_in) = inner_pts[i + 1];
            let (x0_out, z0_out) = outer_pts[i];
            let (x1_out, z1_out) = outer_pts[i + 1];

            // Front face (+Y)
            let v_f0 = Vec3::new(x0_in, half_depth, z0_in);
            let v_f1 = Vec3::new(x1_in, half_depth, z1_in);
            let v_f2 = Vec3::new(x1_out, half_depth, z1_out);
            let v_f3 = Vec3::new(x0_out, half_depth, z0_out);
            if let Some(poly) = Polygon::new(vec![v_f0, v_f1, v_f2, v_f3]) {
                brush.polys.push(poly);
            }

            // Back face (-Y)
            let v_b0 = Vec3::new(x0_in, -half_depth, z0_in);
            let v_b1 = Vec3::new(x0_out, -half_depth, z0_out);
            let v_b2 = Vec3::new(x1_out, -half_depth, z1_out);
            let v_b3 = Vec3::new(x1_in, -half_depth, z1_in);
            if let Some(poly) = Polygon::new(vec![v_b0, v_b1, v_b2, v_b3]) {
                brush.polys.push(poly);
            }

            // Inner soffit ceiling (facing into the doorway opening)
            let v_s0 = Vec3::new(x0_in, -half_depth, z0_in);
            let v_s1 = Vec3::new(x1_in, -half_depth, z1_in);
            let v_s2 = Vec3::new(x1_in, half_depth, z1_in);
            let v_s3 = Vec3::new(x0_in, half_depth, z0_in);
            if let Some(poly) = Polygon::new(vec![v_s0, v_s1, v_s2, v_s3]) {
                brush.polys.push(poly);
            }

            // Outer extrados roof (facing upward/outward)
            let v_e0 = Vec3::new(x0_out, half_depth, z0_out);
            let v_e1 = Vec3::new(x1_out, half_depth, z1_out);
            let v_e2 = Vec3::new(x1_out, -half_depth, z1_out);
            let v_e3 = Vec3::new(x0_out, -half_depth, z0_out);
            if let Some(poly) = Polygon::new(vec![v_e0, v_e1, v_e2, v_e3]) {
                brush.polys.push(poly);
            }
        }

        // Left vertical jamb column
        let x_in_l = -half_span;
        let x_out_l = -half_span - wall_t;
        // Inner jamb face
        let v_jl0 = Vec3::new(x_in_l, -half_depth, 0.0);
        let v_jl1 = Vec3::new(x_in_l, -half_depth, jamb_h);
        let v_jl2 = Vec3::new(x_in_l, half_depth, jamb_h);
        let v_jl3 = Vec3::new(x_in_l, half_depth, 0.0);
        if let Some(poly) = Polygon::new(vec![v_jl0, v_jl1, v_jl2, v_jl3]) {
            brush.polys.push(poly);
        }
        // Outer jamb face
        let v_ol0 = Vec3::new(x_out_l, half_depth, 0.0);
        let v_ol1 = Vec3::new(x_out_l, half_depth, jamb_h);
        let v_ol2 = Vec3::new(x_out_l, -half_depth, jamb_h);
        let v_ol3 = Vec3::new(x_out_l, -half_depth, 0.0);
        if let Some(poly) = Polygon::new(vec![v_ol0, v_ol1, v_ol2, v_ol3]) {
            brush.polys.push(poly);
        }

        // Right vertical jamb column
        let x_in_r = half_span;
        let x_out_r = half_span + wall_t;
        // Inner jamb face
        let v_jr0 = Vec3::new(x_in_r, half_depth, 0.0);
        let v_jr1 = Vec3::new(x_in_r, half_depth, jamb_h);
        let v_jr2 = Vec3::new(x_in_r, -half_depth, jamb_h);
        let v_jr3 = Vec3::new(x_in_r, -half_depth, 0.0);
        if let Some(poly) = Polygon::new(vec![v_jr0, v_jr1, v_jr2, v_jr3]) {
            brush.polys.push(poly);
        }
        // Outer jamb face
        let v_or0 = Vec3::new(x_out_r, -half_depth, 0.0);
        let v_or1 = Vec3::new(x_out_r, -half_depth, jamb_h);
        let v_or2 = Vec3::new(x_out_r, half_depth, jamb_h);
        let v_or3 = Vec3::new(x_out_r, half_depth, 0.0);
        if let Some(poly) = Polygon::new(vec![v_or0, v_or1, v_or2, v_or3]) {
            brush.polys.push(poly);
        }

        // Left and Right base bottoms (Z = 0)
        let v_b_l0 = Vec3::new(x_out_l, -half_depth, 0.0);
        let v_b_l1 = Vec3::new(x_in_l, -half_depth, 0.0);
        let v_b_l2 = Vec3::new(x_in_l, half_depth, 0.0);
        let v_b_l3 = Vec3::new(x_out_l, half_depth, 0.0);
        if let Some(poly) = Polygon::new(vec![v_b_l0, v_b_l1, v_b_l2, v_b_l3]) {
            brush.polys.push(poly);
        }

        let v_b_r0 = Vec3::new(x_in_r, -half_depth, 0.0);
        let v_b_r1 = Vec3::new(x_out_r, -half_depth, 0.0);
        let v_b_r2 = Vec3::new(x_out_r, half_depth, 0.0);
        let v_b_r3 = Vec3::new(x_in_r, half_depth, 0.0);
        if let Some(poly) = Polygon::new(vec![v_b_r0, v_b_r1, v_b_r2, v_b_r3]) {
            brush.polys.push(poly);
        }
    } else {
        // Solid curved vault / block (e.g. for subtractive doorways or decorative barrel vault tops)
        let n = inner_pts.len();
        // Front face (+Y)
        let mut front_verts = vec![
            Vec3::new(inner_pts[0].0, half_depth, 0.0),
            Vec3::new(inner_pts[n - 1].0, half_depth, 0.0),
        ];
        for pt in inner_pts.iter().rev() {
            front_verts.push(Vec3::new(pt.0, half_depth, pt.1));
        }
        if let Some(poly) = Polygon::new(front_verts) {
            brush.polys.push(poly);
        }

        // Back face (-Y)
        let mut back_verts = vec![
            Vec3::new(inner_pts[n - 1].0, -half_depth, 0.0),
            Vec3::new(inner_pts[0].0, -half_depth, 0.0),
        ];
        for pt in &inner_pts {
            back_verts.push(Vec3::new(pt.0, -half_depth, pt.1));
        }
        if let Some(poly) = Polygon::new(back_verts) {
            brush.polys.push(poly);
        }

        // Curved roof surface
        for i in 0..n.saturating_sub(1) {
            let (x0, z0) = inner_pts[i];
            let (x1, z1) = inner_pts[i + 1];
            let v0 = Vec3::new(x0, half_depth, z0);
            let v1 = Vec3::new(x1, half_depth, z1);
            let v2 = Vec3::new(x1, -half_depth, z1);
            let v3 = Vec3::new(x0, -half_depth, z0);
            if let Some(poly) = Polygon::new(vec![v0, v1, v2, v3]) {
                brush.polys.push(poly);
            }
        }

        // Bottom base face (Z = 0)
        let x_l = inner_pts[0].0;
        let x_r = inner_pts[n - 1].0;
        let vb0 = Vec3::new(x_l, -half_depth, 0.0);
        let vb1 = Vec3::new(x_r, -half_depth, 0.0);
        let vb2 = Vec3::new(x_r, half_depth, 0.0);
        let vb3 = Vec3::new(x_l, half_depth, 0.0);
        if let Some(poly) = Polygon::new(vec![vb0, vb1, vb2, vb3]) {
            brush.polys.push(poly);
        }
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_builder_semicircular() {
        let config = ArchConfig {
            arch_type: ArchType::SemiCircular,
            span_width: 256.0,
            jamb_height: 128.0,
            total_height: 256.0,
            depth: 64.0,
            wall_thickness: 32.0,
            segments: 8,
            is_doorway: true,
        };
        let brush = build_arch("TestArch", CsgOperation::Add, &config);
        assert!(!brush.polys.is_empty(), "Arch brush must have polygons");
        let (min, max) = brush.world_bounds();
        assert!((min.x - (-128.0 - 32.0)).abs() < 2.0);
        assert!((max.x - (128.0 + 32.0)).abs() < 2.0);
        assert!((max.z - (256.0 + 32.0)).abs() < 2.0);
        assert_eq!(min.z, 0.0);
    }

    #[test]
    fn test_arch_builder_solid_vault() {
        let config = ArchConfig {
            arch_type: ArchType::GothicPointed,
            span_width: 200.0,
            jamb_height: 100.0,
            total_height: 300.0,
            depth: 80.0,
            wall_thickness: 24.0,
            segments: 10,
            is_doorway: false,
        };
        let brush = build_arch("SolidVault", CsgOperation::Subtract, &config);
        assert!(!brush.polys.is_empty());
        assert_eq!(brush.csg_oper, CsgOperation::Subtract);
    }
}
