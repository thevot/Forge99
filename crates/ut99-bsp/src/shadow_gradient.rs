use glam::Vec3;
use std::f32::consts::PI;
use crate::brush::Brush;
use crate::lighting::ray_intersects_triangle;
use crate::poly::{PF_NOT_SOLID, PF_PORTAL, PF_TRANSLUCENT};

/// Geometric shape and physical dimensions of the light-emitting source
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightSourceShape {
    Point,
    Sphere { radius: f32 },
    Disk { radius: f32, normal: Vec3 },
    Cylinder { radius: f32, height: f32 },
}

impl Default for LightSourceShape {
    fn default() -> Self {
        LightSourceShape::Sphere { radius: 16.0 }
    }
}

/// Detailed profile of the soft shadow penumbra gradient at a surface sample point
#[derive(Debug, Clone, PartialEq)]
pub struct PenumbraGradientProfile {
    pub surface_point: Vec3,
    pub surface_normal: Vec3,
    pub light_position: Vec3,
    pub light_radius: f32,
    pub shape: LightSourceShape,
    pub sample_count: usize,
    pub visible_sample_count: usize,
    /// 0.0 (full umbra, completely occluded) to 1.0 (fully lit, completely unoccluded)
    pub penumbra_factor: f32,
    /// True if the surface point falls strictly in the penumbra boundary (partial occlusion)
    pub is_in_penumbra: bool,
    /// Distance from surface point to the first occluding blocker geometry
    pub first_blocker_dist: Option<f32>,
    /// Physical penumbra transition blur width across the receiver surface in Unreal Units (UU)
    pub penumbra_softness_uu: f32,
}

/// Generates a set of stratified sample positions distributed across the light source volume/surface
pub fn generate_light_source_samples(
    light_pos: Vec3,
    target_pos: Vec3,
    shape: LightSourceShape,
    sample_count: usize,
) -> Vec<Vec3> {
    let count = sample_count.max(1);
    match shape {
        LightSourceShape::Point => vec![light_pos],
        LightSourceShape::Sphere { radius } => {
            if radius <= 1e-4 {
                return vec![light_pos];
            }
            // Fibonacci / Vogel sphere spiral distribution for uniform spherical coverage
            let mut samples = Vec::with_capacity(count);
            let phi = (1.0 + 5.0f32.sqrt()) * 0.5; // Golden ratio
            for i in 0..count {
                let theta = 2.0 * PI * (i as f32) / phi;
                let y = 1.0 - (2.0 * (i as f32) + 1.0) / (count as f32);
                let rad_at_y = (1.0 - y * y).max(0.0).sqrt();
                let x = theta.cos() * rad_at_y;
                let z = theta.sin() * rad_at_y;
                samples.push(light_pos + Vec3::new(x, y, z) * radius);
            }
            samples
        }
        LightSourceShape::Disk { radius, normal } => {
            if radius <= 1e-4 {
                return vec![light_pos];
            }
            let n = if normal.length_squared() > 1e-4 {
                normal.normalize()
            } else {
                (target_pos - light_pos).normalize()
            };

            // Build an orthonormal basis on the disk plane
            let tangent = if n.x.abs() < 0.9 {
                Vec3::X.cross(n).normalize()
            } else {
                Vec3::Y.cross(n).normalize()
            };
            let bitangent = n.cross(tangent).normalize();

            // Concentric / Vogel disk sampling
            let mut samples = Vec::with_capacity(count);
            let golden_angle = PI * (3.0 - 5.0f32.sqrt());
            for i in 0..count {
                let r = ((i as f32 + 0.5) / count as f32).sqrt() * radius;
                let theta = (i as f32) * golden_angle;
                let offset = tangent * (r * theta.cos()) + bitangent * (r * theta.sin());
                samples.push(light_pos + offset);
            }
            samples
        }
        LightSourceShape::Cylinder { radius, height } => {
            if radius <= 1e-4 && height <= 1e-4 {
                return vec![light_pos];
            }
            let mut samples = Vec::with_capacity(count);
            let golden_angle = PI * (3.0 - 5.0f32.sqrt());
            for i in 0..count {
                let t_height = (i as f32 / count as f32 - 0.5) * height;
                let theta = (i as f32) * golden_angle;
                let x = radius * theta.cos();
                let y = radius * theta.sin();
                samples.push(light_pos + Vec3::new(x, y, t_height));
            }
            samples
        }
    }
}

/// Evaluates the soft penumbra gradient at a surface point against triangulated occluders
pub fn evaluate_shadow_penumbra_profile(
    surface_point: Vec3,
    surface_normal: Vec3,
    light_pos: Vec3,
    shape: LightSourceShape,
    occluders: &[[Vec3; 3]],
    sample_count: usize,
) -> PenumbraGradientProfile {
    let source_radius = match shape {
        LightSourceShape::Point => 0.0,
        LightSourceShape::Sphere { radius } => radius,
        LightSourceShape::Disk { radius, .. } => radius,
        LightSourceShape::Cylinder { radius, .. } => radius,
    };

    let samples = generate_light_source_samples(light_pos, surface_point, shape, sample_count);
    let total_samples = samples.len();

    let ray_start = surface_point + surface_normal.normalize() * 0.2;
    let dist_to_light = surface_point.distance(light_pos);

    let mut visible_count = 0;
    let mut min_blocker_dist: Option<f32> = None;

    for sample_pos in &samples {
        let to_sample = *sample_pos - ray_start;
        let sample_dist = to_sample.length();
        if sample_dist < 1e-4 {
            visible_count += 1;
            continue;
        }

        let dir = to_sample / sample_dist;
        let mut hit_blocker = false;

        for tri in occluders {
            if let Some(t) = ray_intersects_triangle(ray_start, dir, tri[0], tri[1], tri[2]) {
                if t > 0.05 && t < sample_dist - 0.1 {
                    hit_blocker = true;
                    let current_min = min_blocker_dist.unwrap_or(f32::MAX);
                    if t < current_min {
                        min_blocker_dist = Some(t);
                    }
                    break;
                }
            }
        }

        if !hit_blocker {
            visible_count += 1;
        }
    }

    let factor = visible_count as f32 / total_samples.max(1) as f32;
    let is_penumbra = visible_count > 0 && visible_count < total_samples;

    // Similar triangle optics formula for soft penumbra width:
    // W = 2.0 * R_source * (d_blocker / (d_light - d_blocker))
    let softness_uu = if let Some(d_blocker) = min_blocker_dist {
        let d_remaining = dist_to_light - d_blocker;
        if d_remaining > 1.0 && source_radius > 0.0 {
            (2.0 * source_radius * (d_blocker / d_remaining)).clamp(0.0, 500.0)
        } else {
            0.0
        }
    } else {
        0.0
    };

    PenumbraGradientProfile {
        surface_point,
        surface_normal,
        light_position: light_pos,
        light_radius: source_radius,
        shape,
        sample_count: total_samples,
        visible_sample_count: visible_count,
        penumbra_factor: factor,
        is_in_penumbra: is_penumbra,
        first_blocker_dist: min_blocker_dist,
        penumbra_softness_uu: softness_uu,
    }
}

/// Evaluates soft penumbra gradient using solid world brushes as occluders
pub fn evaluate_brush_shadow_gradient(
    surface_point: Vec3,
    surface_normal: Vec3,
    light_pos: Vec3,
    shape: LightSourceShape,
    brushes: &[Brush],
    sample_count: usize,
) -> PenumbraGradientProfile {
    let mut occluders = Vec::new();
    for brush in brushes {
        let world_polys = brush.to_world_polys();
        for poly in world_polys {
            if (poly.flags & (PF_NOT_SOLID | PF_PORTAL | PF_TRANSLUCENT)) != 0 {
                continue;
            }
            if poly.vertices.len() >= 3 {
                let v0 = poly.vertices[0];
                for i in 1..(poly.vertices.len() - 1) {
                    occluders.push([v0, poly.vertices[i], poly.vertices[i + 1]]);
                }
            }
        }
    }

    evaluate_shadow_penumbra_profile(
        surface_point,
        surface_normal,
        light_pos,
        shape,
        &occluders,
        sample_count,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::build_cube;
    use crate::CsgOperation;

    #[test]
    fn test_unoccluded_shadow_gradient() {
        let light_pos = Vec3::new(0.0, 0.0, 200.0);
        let surface_pos = Vec3::new(0.0, 0.0, 0.0);
        let surface_norm = Vec3::Z;

        let occluders: Vec<[Vec3; 3]> = vec![];
        let profile = evaluate_shadow_penumbra_profile(
            surface_pos,
            surface_norm,
            light_pos,
            LightSourceShape::Sphere { radius: 32.0 },
            &occluders,
            16,
        );

        assert_eq!(profile.visible_sample_count, 16);
        assert!((profile.penumbra_factor - 1.0).abs() < 1e-4);
        assert!(!profile.is_in_penumbra);
        assert_eq!(profile.first_blocker_dist, None);
    }

    #[test]
    fn test_fully_occluded_shadow_gradient() {
        let light_pos = Vec3::new(0.0, 0.0, 200.0);
        let surface_pos = Vec3::new(0.0, 0.0, 0.0);
        let surface_norm = Vec3::Z;

        // Big wall at Z = 100 blocking everything
        let blocker = build_cube("BigBlocker", CsgOperation::Add, Vec3::new(500.0, 500.0, 20.0));
        let mut blocker_brush = blocker;
        blocker_brush.location = Vec3::new(0.0, 0.0, 100.0);

        let profile = evaluate_brush_shadow_gradient(
            surface_pos,
            surface_norm,
            light_pos,
            LightSourceShape::Sphere { radius: 16.0 },
            &[blocker_brush],
            16,
        );

        assert_eq!(profile.visible_sample_count, 0);
        assert_eq!(profile.penumbra_factor, 0.0);
        assert!(!profile.is_in_penumbra);
        assert!(profile.first_blocker_dist.is_some());
    }

    #[test]
    fn test_partial_penumbra_gradient() {
        let light_pos = Vec3::new(0.0, 0.0, 200.0);
        let surface_pos = Vec3::new(0.0, 0.0, 0.0);
        let surface_norm = Vec3::Z;

        // Place a thin edge occluder slicing across half the light disk at Z = 100
        // Half-space blocker: x from 0 to 500
        let tri1 = [
            Vec3::new(0.0, -200.0, 100.0),
            Vec3::new(500.0, -200.0, 100.0),
            Vec3::new(500.0, 200.0, 100.0),
        ];
        let tri2 = [
            Vec3::new(0.0, -200.0, 100.0),
            Vec3::new(500.0, 200.0, 100.0),
            Vec3::new(0.0, 200.0, 100.0),
        ];
        let occluders = vec![tri1, tri2];

        let profile = evaluate_shadow_penumbra_profile(
            surface_pos,
            surface_norm,
            light_pos,
            LightSourceShape::Disk {
                radius: 40.0,
                normal: Vec3::Z,
            },
            &occluders,
            32,
        );

        // Should have some samples blocked, some visible
        assert!(profile.visible_sample_count > 0);
        assert!(profile.visible_sample_count < 32);
        assert!(profile.is_in_penumbra);
        assert!(profile.penumbra_factor > 0.1 && profile.penumbra_factor < 0.9);
        assert!(profile.penumbra_softness_uu > 0.0);
    }
}
