// crates/ut99-bsp/src/shape_loft.rs
//
// Interactive 2D Shape Loft, Curved Pipe & Architectural Molding Studio
//
// Implements Rotation Minimizing Frames (RMF), Spline Lofts, Hollow Conduits,
// and Classical Unreal Architectural Molding Profiles.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::shape2d::Shape2D;
use crate::CsgOperation;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Classical Unreal Tournament & Unreal Engine architectural molding archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoftProfileArchetype {
    IndustrialRoundPipe,
    HollowTunnelArch,
    GothicVaultRib,
    RomanOgeeMolding,
    ChamferedHexDuct,
    SteppedHandrail,
}

impl LoftProfileArchetype {
    pub fn all() -> &'static [LoftProfileArchetype] {
        &[
            LoftProfileArchetype::IndustrialRoundPipe,
            LoftProfileArchetype::HollowTunnelArch,
            LoftProfileArchetype::GothicVaultRib,
            LoftProfileArchetype::RomanOgeeMolding,
            LoftProfileArchetype::ChamferedHexDuct,
            LoftProfileArchetype::SteppedHandrail,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LoftProfileArchetype::IndustrialRoundPipe => "Industrial Round Pipe",
            LoftProfileArchetype::HollowTunnelArch => "Hollow Tunnel Arch",
            LoftProfileArchetype::GothicVaultRib => "Gothic Vault Rib",
            LoftProfileArchetype::RomanOgeeMolding => "Roman Ogee Cornice",
            LoftProfileArchetype::ChamferedHexDuct => "Chamfered Hex Duct",
            LoftProfileArchetype::SteppedHandrail => "Stepped Bannister / Handrail",
        }
    }

    pub fn to_profile(&self, base_radius: f32) -> Shape2D {
        match self {
            LoftProfileArchetype::IndustrialRoundPipe => Shape2D::new_circle(base_radius, 12),
            LoftProfileArchetype::HollowTunnelArch => Shape2D::new_arch(base_radius * 2.0, base_radius * 1.5, 8),
            LoftProfileArchetype::GothicVaultRib => {
                // Stepped gothic rib with pointed arch top and indented sides
                let r = base_radius;
                let mut pts = Vec::new();
                pts.push(Vec2::new(-r * 0.5, -r));
                pts.push(Vec2::new(r * 0.5, -r));
                pts.push(Vec2::new(r * 0.7, -r * 0.3));
                pts.push(Vec2::new(r * 0.4, 0.0));
                pts.push(Vec2::new(r * 0.5, r * 0.6));
                pts.push(Vec2::new(0.0, r * 1.2)); // pointed crown apex
                pts.push(Vec2::new(-r * 0.5, r * 0.6));
                pts.push(Vec2::new(-r * 0.4, 0.0));
                pts.push(Vec2::new(-r * 0.7, -r * 0.3));
                Shape2D { points: pts, closed: true, ..Default::default() }
            }
            LoftProfileArchetype::RomanOgeeMolding => {
                // Classical S-curve cornice profile
                let r = base_radius;
                let mut pts = Vec::new();
                pts.push(Vec2::new(-r, -r));
                pts.push(Vec2::new(r, -r));
                pts.push(Vec2::new(r, -r * 0.6));
                // Concave dip
                pts.push(Vec2::new(r * 0.5, -r * 0.2));
                pts.push(Vec2::new(r * 0.3, 0.2 * r));
                // Convex swell
                pts.push(Vec2::new(r * 0.8, 0.6 * r));
                pts.push(Vec2::new(r * 0.6, r));
                pts.push(Vec2::new(-r, r));
                Shape2D { points: pts, closed: true, ..Default::default() }
            }
            LoftProfileArchetype::ChamferedHexDuct => {
                let r = base_radius;
                let mut pts = Vec::new();
                let segs = 6;
                for i in 0..segs {
                    let angle = i as f32 * std::f32::consts::TAU / segs as f32;
                    pts.push(Vec2::new(r * angle.cos(), r * angle.sin()));
                }
                Shape2D { points: pts, closed: true, ..Default::default() }
            }
            LoftProfileArchetype::SteppedHandrail => {
                // Rounded top, narrow neck, flared base
                let r = base_radius;
                let mut pts = Vec::new();
                pts.push(Vec2::new(-r * 0.6, -r));
                pts.push(Vec2::new(r * 0.6, -r));
                pts.push(Vec2::new(r * 0.3, -r * 0.2));
                pts.push(Vec2::new(r * 0.5, 0.5 * r));
                pts.push(Vec2::new(0.0, r)); // rounded cap
                pts.push(Vec2::new(-r * 0.5, 0.5 * r));
                pts.push(Vec2::new(-r * 0.3, -r * 0.2));
                Shape2D { points: pts, closed: true, ..Default::default() }
            }
        }
    }
}

/// A spatial control station along the loft path
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoftPathKeyframe {
    pub position: Vec3,
    pub scale: Vec2,
    pub twist_deg: f32,
}

impl LoftPathKeyframe {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            scale: Vec2::ONE,
            twist_deg: 0.0,
        }
    }

    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_twist(mut self, twist_deg: f32) -> Self {
        self.twist_deg = twist_deg;
        self
    }
}

/// Coordinate frame containing tangent, normal, and binormal for sweep alignment
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoftFrame {
    pub position: Vec3,
    pub tangent: Vec3,
    pub normal: Vec3,
    pub binormal: Vec3,
    pub scale: Vec2,
    pub twist_deg: f32,
}

impl LoftFrame {
    pub fn transform_point(&self, p: Vec2) -> Vec3 {
        // Apply twist angle to 2D point before spatial frame transformation
        let rad = self.twist_deg.to_radians();
        let (sin_t, cos_t) = rad.sin_cos();
        let twisted_p = Vec2::new(
            p.x * self.scale.x * cos_t - p.y * self.scale.y * sin_t,
            p.x * self.scale.x * sin_t + p.y * self.scale.y * cos_t,
        );

        self.position + self.normal * twisted_p.x + self.binormal * twisted_p.y
    }
}

/// Configuration settings for the lofting engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoftConfig {
    pub archetype: LoftProfileArchetype,
    pub profile_radius: f32,
    pub tessellation_steps: usize,
    pub is_hollow: bool,
    pub wall_thickness: f32,
    pub cap_start: bool,
    pub cap_end: bool,
    pub texture_name: String,
    pub texture_u_scale: f32,
    pub texture_v_scale: f32,
}

impl Default for LoftConfig {
    fn default() -> Self {
        Self {
            archetype: LoftProfileArchetype::IndustrialRoundPipe,
            profile_radius: 32.0,
            tessellation_steps: 16,
            is_hollow: false,
            wall_thickness: 6.0,
            cap_start: true,
            cap_end: true,
            texture_name: "Indus1.Indus_B01".to_string(),
            texture_u_scale: 1.0,
            texture_v_scale: 1.0,
        }
    }
}

/// Diagnostic warning for geometry hazards along a lofted spline
#[derive(Debug, Clone, PartialEq)]
pub struct LoftHazard {
    pub keyframe_index: usize,
    pub position: Vec3,
    pub message: String,
    pub is_critical: bool, // e.g. self-intersecting bend
}

/// Evaluates a sequence of 3D control stations and computes smooth Rotation Minimizing Frames (RMF)
pub fn generate_rmf_frames(keyframes: &[LoftPathKeyframe], total_samples: usize) -> Vec<LoftFrame> {
    if keyframes.len() < 2 || total_samples < 2 {
        return Vec::new();
    }

    // 1. Generate interpolated centerline curve positions via Catmull-Rom spline
    let mut curve_pts = Vec::with_capacity(total_samples);
    let mut scales = Vec::with_capacity(total_samples);
    let mut twists = Vec::with_capacity(total_samples);

    let n = keyframes.len();
    for i in 0..total_samples {
        let t_global = i as f32 / (total_samples - 1) as f32;
        let seg_f = t_global * (n - 1) as f32;
        let seg_idx = (seg_f.floor() as usize).min(n - 2);
        let t = seg_f - seg_idx as f32;

        let p0 = if seg_idx > 0 { keyframes[seg_idx - 1].position } else { keyframes[0].position * 2.0 - keyframes[1].position };
        let p1 = keyframes[seg_idx].position;
        let p2 = keyframes[seg_idx + 1].position;
        let p3 = if seg_idx + 2 < n { keyframes[seg_idx + 2].position } else { p2 * 2.0 - p1 };

        let pos = catmull_rom(p0, p1, p2, p3, t);
        let scale = keyframes[seg_idx].scale.lerp(keyframes[seg_idx + 1].scale, t);
        let twist = keyframes[seg_idx].twist_deg + (keyframes[seg_idx + 1].twist_deg - keyframes[seg_idx].twist_deg) * t;

        curve_pts.push(pos);
        scales.push(scale);
        twists.push(twist);
    }

    // 2. Compute Rotation Minimizing Frames (RMF) using the Double Reflection method
    let mut frames = Vec::with_capacity(total_samples);

    // Initial frame
    let tangent_0 = (curve_pts[1] - curve_pts[0]).normalize_or_zero();
    let initial_up = if tangent_0.z.abs() < 0.9 { Vec3::Z } else { Vec3::Y };
    let normal_0 = tangent_0.cross(initial_up).normalize_or_zero();
    let binormal_0 = tangent_0.cross(normal_0).normalize_or_zero();

    frames.push(LoftFrame {
        position: curve_pts[0],
        tangent: tangent_0,
        normal: normal_0,
        binormal: binormal_0,
        scale: scales[0],
        twist_deg: twists[0],
    });

    for i in 0..(total_samples - 1) {
        let x_i = curve_pts[i];
        let x_next = curve_pts[i + 1];
        let v1 = x_next - x_i;
        let c1 = v1.length_squared();

        let r_i = frames[i].normal;
        let t_i = frames[i].tangent;

        // Reflection 1: across plane bisecting x_i and x_next
        let r_i_reflected = if c1 > 1e-6 {
            r_i - v1 * (2.0 / c1 * v1.dot(r_i))
        } else {
            r_i
        };
        let t_i_reflected = if c1 > 1e-6 {
            t_i - v1 * (2.0 / c1 * v1.dot(t_i))
        } else {
            t_i
        };

        // Tangent at next sample
        let t_next = if i + 2 < total_samples {
            (curve_pts[i + 2] - x_next).normalize_or_zero()
        } else {
            (x_next - x_i).normalize_or_zero()
        };

        // Reflection 2: across plane bisecting t_i_reflected and t_next
        let v2 = t_next - t_i_reflected;
        let c2 = v2.length_squared();

        let r_next = if c2 > 1e-6 {
            (r_i_reflected - v2 * (2.0 / c2 * v2.dot(r_i_reflected))).normalize_or_zero()
        } else {
            r_i_reflected.normalize_or_zero()
        };

        let s_next = t_next.cross(r_next).normalize_or_zero();

        frames.push(LoftFrame {
            position: x_next,
            tangent: t_next,
            normal: r_next,
            binormal: s_next,
            scale: scales[i + 1],
            twist_deg: twists[i + 1],
        });
    }

    frames
}

/// Generates a complete 3D Brush by lofting a 2D Shape along RMF coordinate frames
pub fn generate_loft_brush(
    profile: &Shape2D,
    keyframes: &[LoftPathKeyframe],
    config: &LoftConfig,
) -> Brush {
    let mut brush = Brush::new("Loft_Pipe", CsgOperation::Add);

    if profile.points.len() < 3 || keyframes.len() < 2 {
        return brush;
    }

    let frames = generate_rmf_frames(keyframes, config.tessellation_steps.max(2));
    if frames.is_empty() {
        return brush;
    }

    let ring_count = frames.len();
    let pts_per_ring = profile.points.len();

    // 1. Build rings of 3D vertices
    let mut rings: Vec<Vec<Vec3>> = Vec::with_capacity(ring_count);
    for frame in &frames {
        let mut ring_pts = Vec::with_capacity(pts_per_ring);
        for &p in &profile.points {
            ring_pts.push(frame.transform_point(p));
        }
        rings.push(ring_pts);
    }

    // 2. Generate side quad polygons along length of pipe
    for r in 0..(ring_count - 1) {
        let ring_a = &rings[r];
        let ring_b = &rings[r + 1];

        for i in 0..pts_per_ring {
            let next_i = (i + 1) % pts_per_ring;

            let quad = vec![
                ring_a[i],
                ring_b[i],
                ring_b[next_i],
                ring_a[next_i],
            ];

            if let Some(mut poly) = Polygon::new(quad) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
        }
    }

    // 3. Optional hollow inner wall generation (if is_hollow is enabled)
    if config.is_hollow && config.wall_thickness > 0.5 {
        let mut inner_rings: Vec<Vec<Vec3>> = Vec::with_capacity(ring_count);

        for frame in &frames {
            let mut ring_pts = Vec::with_capacity(pts_per_ring);
            for &p in &profile.points {
                // Inward offset by wall thickness
                let len = p.length();
                let inner_p = if len > config.wall_thickness {
                    p * ((len - config.wall_thickness) / len)
                } else {
                    p * 0.2
                };
                ring_pts.push(frame.transform_point(inner_p));
            }
            inner_rings.push(ring_pts);
        }

        // Inner walls (facing inwards)
        for r in 0..(ring_count - 1) {
            let ring_a = &inner_rings[r];
            let ring_b = &inner_rings[r + 1];

            for i in 0..pts_per_ring {
                let next_i = (i + 1) % pts_per_ring;

                // Reversed winding for interior surface normal
                let quad = vec![
                    ring_a[next_i],
                    ring_b[next_i],
                    ring_b[i],
                    ring_a[i],
                ];

                if let Some(mut poly) = Polygon::new(quad) {
                    poly.texture = config.texture_name.clone();
                    brush.polys.push(poly);
                }
            }
        }

        // Rim caps at both ends linking outer and inner walls
        if config.cap_start {
            let outer_start = &rings[0];
            let inner_start = &inner_rings[0];
            for i in 0..pts_per_ring {
                let next_i = (i + 1) % pts_per_ring;
                let quad = vec![
                    outer_start[next_i],
                    inner_start[next_i],
                    inner_start[i],
                    outer_start[i],
                ];
                if let Some(mut poly) = Polygon::new(quad) {
                    poly.texture = config.texture_name.clone();
                    brush.polys.push(poly);
                }
            }
        }

        if config.cap_end {
            let last_r = ring_count - 1;
            let outer_end = &rings[last_r];
            let inner_end = &inner_rings[last_r];
            for i in 0..pts_per_ring {
                let next_i = (i + 1) % pts_per_ring;
                let quad = vec![
                    outer_end[i],
                    inner_end[i],
                    inner_end[next_i],
                    outer_end[next_i],
                ];
                if let Some(mut poly) = Polygon::new(quad) {
                    poly.texture = config.texture_name.clone();
                    brush.polys.push(poly);
                }
            }
        }
    } else {
        // Solid end caps
        if config.cap_start {
            let mut cap_verts = rings[0].clone();
            cap_verts.reverse(); // Face pointing backwards out of start
            if let Some(mut poly) = Polygon::new(cap_verts) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
        }

        if config.cap_end {
            let cap_verts = rings[ring_count - 1].clone();
            if let Some(mut poly) = Polygon::new(cap_verts) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
        }
    }

    brush
}

/// Audits a loft trajectory for self-intersecting bends, tight radii and extreme aspect ratios
pub fn audit_loft_hazards(
    keyframes: &[LoftPathKeyframe],
    profile_radius: f32,
) -> Vec<LoftHazard> {
    let mut hazards = Vec::new();

    if keyframes.len() < 3 {
        return hazards;
    }

    for i in 1..(keyframes.len() - 1) {
        let prev = keyframes[i - 1].position;
        let curr = keyframes[i].position;
        let next = keyframes[i + 1].position;

        let v1 = (curr - prev).normalize_or_zero();
        let v2 = (next - curr).normalize_or_zero();

        let dot = v1.dot(v2).clamp(-1.0, 1.0);
        let angle_deg = dot.acos().to_degrees();

        // If turn angle > 75 degrees over short span, risk of self-intersection
        let min_segment_len = (curr - prev).length().min((next - curr).length());
        let local_radius = keyframes[i].scale.max_element() * profile_radius;

        if min_segment_len < local_radius * 1.5 && angle_deg > 60.0 {
            hazards.push(LoftHazard {
                keyframe_index: i,
                position: curr,
                message: format!(
                    "Sharp turn bend ({:.0}°) with short segment length ({:.0} uu < {:.0} uu) creates self-intersection geometry artifacts.",
                    angle_deg, min_segment_len, local_radius * 1.5
                ),
                is_critical: true,
            });
        }
    }

    hazards
}

fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;

    0.5 * (
        (2.0 * p1) +
        (-p0 + p2) * t +
        (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 +
        (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archetypes_and_profile_generation() {
        let archs = LoftProfileArchetype::all();
        assert_eq!(archs.len(), 6);

        for &arch in archs {
            let profile = arch.to_profile(32.0);
            assert!(profile.points.len() >= 3, "Profile must have at least 3 points");
            assert!(profile.closed, "Profile must be closed");
        }
    }

    #[test]
    fn test_rmf_frame_generation_straight_and_curved() {
        let keyframes = vec![
            LoftPathKeyframe::new(Vec3::new(0.0, 0.0, 0.0)),
            LoftPathKeyframe::new(Vec3::new(0.0, 0.0, 256.0)),
            LoftPathKeyframe::new(Vec3::new(256.0, 0.0, 512.0)),
        ];

        let frames = generate_rmf_frames(&keyframes, 10);
        assert_eq!(frames.len(), 10);

        for f in &frames {
            // Tangent and normal must be strictly orthogonal
            assert!(f.tangent.dot(f.normal).abs() < 0.001);
            assert!(f.normal.dot(f.binormal).abs() < 0.001);
            assert!(f.tangent.dot(f.binormal).abs() < 0.001);
        }
    }

    #[test]
    fn test_generate_solid_and_hollow_loft_brushes() {
        let profile = LoftProfileArchetype::IndustrialRoundPipe.to_profile(32.0);
        let keyframes = vec![
            LoftPathKeyframe::new(Vec3::new(0.0, 0.0, 0.0)),
            LoftPathKeyframe::new(Vec3::new(0.0, 0.0, 256.0)),
        ];
        let mut cfg = LoftConfig::default();
        cfg.tessellation_steps = 4;

        // Solid brush
        cfg.is_hollow = false;
        let solid_brush = generate_loft_brush(&profile, &keyframes, &cfg);
        assert!(solid_brush.polys.len() > 10);

        // Hollow brush
        cfg.is_hollow = true;
        cfg.wall_thickness = 4.0;
        let hollow_brush = generate_loft_brush(&profile, &keyframes, &cfg);
        assert!(hollow_brush.polys.len() > solid_brush.polys.len(), "Hollow brush must generate inner walls and rims");
    }

    #[test]
    fn test_audit_loft_hazards_detection() {
        // Path with a violent 90 degree turn over a tiny distance (20 uu)
        let keyframes = vec![
            LoftPathKeyframe::new(Vec3::new(0.0, 0.0, 0.0)),
            LoftPathKeyframe::new(Vec3::new(0.0, 0.0, 20.0)),
            LoftPathKeyframe::new(Vec3::new(20.0, 0.0, 20.0)),
        ];

        let hazards = audit_loft_hazards(&keyframes, 32.0);
        assert!(!hazards.is_empty(), "Audit must detect sharp bend pinch hazard");
        assert!(hazards[0].is_critical);
    }
}
