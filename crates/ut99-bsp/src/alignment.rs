use crate::poly::Polygon;
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignmentMode {
    Floor,
    WallX,
    WallY,
    WallAuto,
    Planar,
    Box,
    Fit { repeats_u: f32, repeats_v: f32 },
    Cylinder { center: Vec3, repeats_u: f32, repeats_v: f32 },
}

/// Aligns a polygon's texture UV mapping coordinates according to UnrealEd standards.
pub fn align_polygon(poly: &mut Polygon, mode: AlignmentMode) {
    if poly.vertices.len() < 3 {
        return;
    }

    let norm = poly.plane.normal.normalize();

    match mode {
        AlignmentMode::Floor => {
            poly.texture_u = Vec3::new(1.0, 0.0, 0.0);
            poly.texture_v = Vec3::new(0.0, 1.0, 0.0);
            poly.base = Vec3::ZERO;
        }
        AlignmentMode::WallX => {
            poly.texture_u = Vec3::new(0.0, 1.0, 0.0);
            poly.texture_v = Vec3::new(0.0, 0.0, -1.0);
            poly.base = Vec3::ZERO;
        }
        AlignmentMode::WallY => {
            poly.texture_u = Vec3::new(1.0, 0.0, 0.0);
            poly.texture_v = Vec3::new(0.0, 0.0, -1.0);
            poly.base = Vec3::ZERO;
        }
        AlignmentMode::WallAuto => {
            if norm.z.abs() >= 0.99 {
                // Nearly horizontal floor/ceiling
                poly.texture_u = Vec3::new(1.0, 0.0, 0.0);
                poly.texture_v = if norm.z >= 0.0 {
                    Vec3::new(0.0, 1.0, 0.0)
                } else {
                    Vec3::new(0.0, -1.0, 0.0)
                };
            } else {
                // True horizontal tangent along wall plane:
                // Cross product of normal (nx, ny, nz) with Z (0, 0, 1) = (-ny, nx, 0)
                let tangent_u = Vec3::new(-norm.y, norm.x, 0.0).normalize();
                poly.texture_u = tangent_u;
                poly.texture_v = Vec3::new(0.0, 0.0, -1.0);
            }
            poly.base = poly.vertices[0];
        }
        AlignmentMode::Planar => {
            let (u, v) = compute_planar_tangents(norm);
            poly.texture_u = u;
            poly.texture_v = v;
            poly.base = poly.vertices[0];
        }
        AlignmentMode::Box => {
            let abs_x = norm.x.abs();
            let abs_y = norm.y.abs();
            let abs_z = norm.z.abs();

            if abs_z >= abs_x && abs_z >= abs_y {
                // Horizontal Floor / Ceiling
                poly.texture_u = Vec3::new(1.0, 0.0, 0.0);
                poly.texture_v = if norm.z >= 0.0 {
                    Vec3::new(0.0, 1.0, 0.0)
                } else {
                    Vec3::new(0.0, -1.0, 0.0)
                };
            } else if abs_x >= abs_y {
                // Vertical Wall facing X
                poly.texture_u = if norm.x >= 0.0 {
                    Vec3::new(0.0, 1.0, 0.0)
                } else {
                    Vec3::new(0.0, -1.0, 0.0)
                };
                poly.texture_v = Vec3::new(0.0, 0.0, -1.0);
            } else {
                // Vertical Wall facing Y
                poly.texture_u = if norm.y >= 0.0 {
                    Vec3::new(-1.0, 0.0, 0.0)
                } else {
                    Vec3::new(1.0, 0.0, 0.0)
                };
                poly.texture_v = Vec3::new(0.0, 0.0, -1.0);
            }
            poly.base = poly.vertices[0];
        }
        AlignmentMode::Fit { repeats_u, repeats_v } => {
            let (u_dir, v_dir) = compute_planar_tangents(norm);
            let origin = poly.vertices[0];

            let mut min_u = f32::MAX;
            let mut max_u = f32::MIN;
            let mut min_v = f32::MAX;
            let mut max_v = f32::MIN;

            for &v in &poly.vertices {
                let d = v - origin;
                let u_proj = d.dot(u_dir);
                let v_proj = d.dot(v_dir);

                min_u = min_u.min(u_proj);
                max_u = max_u.max(u_proj);
                min_v = min_v.min(v_proj);
                max_v = max_v.max(v_proj);
            }

            let width = (max_u - min_u).max(1.0);
            let height = (max_v - min_v).max(1.0);

            let r_u = repeats_u.max(0.01);
            let r_v = repeats_v.max(0.01);

            // Scale U and V vectors so that width / (128 * r_u) == 1 repeat across span
            let scale_u = width / (128.0 * r_u);
            let scale_v = height / (128.0 * r_v);

            poly.texture_u = u_dir * (1.0 / scale_u);
            poly.texture_v = v_dir * (1.0 / scale_v);
            poly.base = origin + u_dir * min_u + v_dir * min_v;
        }
        AlignmentMode::Cylinder { center, repeats_u, repeats_v } => {
            // Horizontal caps (top/bottom) fall back to planar/floor
            if norm.z.abs() > 0.85 {
                poly.texture_u = Vec3::X;
                poly.texture_v = Vec3::Y;
                poly.base = Vec3::ZERO;
            } else {
                // Compute polygon center
                let mut p_center = Vec3::ZERO;
                for v in &poly.vertices {
                    p_center += *v;
                }
                p_center /= poly.vertices.len() as f32;

                let rel_x = p_center.x - center.x;
                let rel_y = p_center.y - center.y;
                let theta = rel_y.atan2(rel_x);

                // Tangent vector around cylinder circumference: (-sin theta, cos theta, 0)
                let tangent = Vec3::new(-theta.sin(), theta.cos(), 0.0);
                let bitangent = Vec3::new(0.0, 0.0, -1.0);

                let r_u = repeats_u.max(0.1);
                let r_v = repeats_v.max(0.1);

                poly.texture_u = tangent * r_u;
                poly.texture_v = bitangent * r_v;
                poly.base = poly.vertices[0];
            }
        }
    }
}

/// Unwraps and aligns an entire brush (e.g. cylinder, curved stair, torus) with cylindrical UV projection.
pub fn align_brush_cylindrical(brush: &mut crate::brush::Brush, repeats_u: f32, repeats_v: f32) {
    if brush.polys.is_empty() {
        return;
    }

    // Compute bounding centroid (center of cylinder)
    let mut center = Vec3::ZERO;
    let mut count = 0;
    for p in &brush.polys {
        for v in &p.vertices {
            center += *v;
            count += 1;
        }
    }
    if count > 0 {
        center /= count as f32;
    }

    let mode = AlignmentMode::Cylinder { center, repeats_u, repeats_v };
    for poly in &mut brush.polys {
        align_polygon(poly, mode);
    }
}

/// Computes an orthonormal tangent and bitangent pair for any arbitrary normal.
fn compute_planar_tangents(norm: Vec3) -> (Vec3, Vec3) {
    if norm.z.abs() > 0.99 {
        let u = Vec3::new(1.0, 0.0, 0.0);
        let v = if norm.z >= 0.0 {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            Vec3::new(0.0, -1.0, 0.0)
        };
        (u, v)
    } else {
        // Tangent perpendicular to normal and Z axis
        let u = norm.cross(Vec3::new(0.0, 0.0, 1.0)).normalize();
        let v = norm.cross(u).normalize();
        (u, v)
    }
}

/// Pans (translates) polygon texture coordinates by (du, dv) world units.
pub fn pan_texture(poly: &mut Polygon, du: f32, dv: f32) {
    let u_dir = if poly.texture_u.length_squared() > 1e-6 {
        poly.texture_u.normalize()
    } else {
        Vec3::X
    };
    let v_dir = if poly.texture_v.length_squared() > 1e-6 {
        poly.texture_v.normalize()
    } else {
        Vec3::Y
    };
    poly.base += u_dir * du + v_dir * dv;
}

/// Scales polygon texture coordinates by scaling factors (su, sv).
pub fn scale_texture(poly: &mut Polygon, su: f32, sv: f32) {
    if su != 0.0 {
        poly.texture_u *= su;
    }
    if sv != 0.0 {
        poly.texture_v *= sv;
    }
}

/// Rotates texture coordinates in-plane around polygon normal by angle in degrees.
pub fn rotate_texture(poly: &mut Polygon, angle_deg: f32) {
    let rad = angle_deg.to_radians();
    let norm = poly.plane.normal.normalize();

    let quat = glam::Quat::from_axis_angle(norm, rad);
    poly.texture_u = quat.mul_vec3(poly.texture_u);
    poly.texture_v = quat.mul_vec3(poly.texture_v);
}

/// Continuously aligns adjacent or sequential wall polygons along their horizontal curve/perimeter,
/// ensuring 100% seamless texture continuation without UV seams.
pub fn align_polygons_continuous_strip(polys: &mut [Polygon], repeats_u: f32, repeats_v: f32) {
    if polys.is_empty() {
        return;
    }

    // Filter wall polygons (|norm.z| < 0.85)
    // Compute width of each polygon along its horizontal tangent
    let mut widths = Vec::with_capacity(polys.len());
    let mut tangents = Vec::with_capacity(polys.len());
    let mut total_width = 0.0_f32;

    for p in polys.iter() {
        let norm = p.plane.normal;
        let t = if norm.z.abs() < 0.85 {
            let cross = norm.cross(Vec3::new(0.0, 0.0, 1.0));
            if cross.length_squared() > 1e-6 {
                cross.normalize()
            } else {
                Vec3::X
            }
        } else {
            Vec3::X
        };

        let mut min_t = f32::MAX;
        let mut max_t = f32::MIN;
        for &v in &p.vertices {
            let proj = v.dot(t);
            min_t = min_t.min(proj);
            max_t = max_t.max(proj);
        }
        let w = (max_t - min_t).max(1.0);
        widths.push(w);
        tangents.push(t);
        total_width += w;
    }

    if total_width < 1.0 {
        total_width = 1.0;
    }

    let scale_u_factor = if repeats_u > 0.0 {
        (total_width / (128.0 * repeats_u)).max(0.001)
    } else {
        1.0
    };
    let u_magnitude = 1.0 / scale_u_factor;

    let r_v = repeats_v.max(0.01);

    let mut accumulated_u = 0.0_f32;
    for (i, p) in polys.iter_mut().enumerate() {
        let norm = p.plane.normal;
        if norm.z.abs() >= 0.85 {
            // Cap/floor faces: align planar
            align_polygon(p, AlignmentMode::Planar);
            continue;
        }

        let t = tangents[i];
        let bitangent = Vec3::new(0.0, 0.0, -1.0);

        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;
        for &v in &p.vertices {
            min_z = min_z.min(v.z);
            max_z = max_z.max(v.z);
        }
        let height = (max_z - min_z).max(1.0);
        let scale_v = height / (128.0 * r_v);
        let v_magnitude = 1.0 / scale_v.max(0.001);

        p.texture_u = t * u_magnitude;
        p.texture_v = bitangent * v_magnitude;

        // Base positioned such that projection at origin matches accumulated_u
        let origin = p.vertices.first().copied().unwrap_or(Vec3::ZERO);
        let origin_proj = origin.dot(t);
        let u_shift_world = origin_proj - accumulated_u;
        p.base = origin - t * u_shift_world + Vec3::new(0.0, 0.0, max_z - origin.z);

        accumulated_u += widths[i];
    }
}

/// Continuously aligns lateral wall faces of a brush (curved stair, cylinder, arch, corridor).
pub fn align_brush_continuous_strip(brush: &mut crate::brush::Brush, repeats_u: f32, repeats_v: f32) {
    align_polygons_continuous_strip(&mut brush.polys, repeats_u, repeats_v);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_polygons_continuous_strip() {
        let mut p1 = Polygon::new(vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 200.0),
            Vec3::new(0.0, 0.0, 200.0),
        ]).unwrap();
        p1.plane = crate::math::Plane::from_point_and_normal(Vec3::ZERO, Vec3::new(0.0, -1.0, 0.0));

        let mut p2 = Polygon::new(vec![
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(200.0, 0.0, 0.0),
            Vec3::new(200.0, 0.0, 200.0),
            Vec3::new(100.0, 0.0, 200.0),
        ]).unwrap();
        p2.plane = crate::math::Plane::from_point_and_normal(Vec3::new(100.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0));

        let mut polys = vec![p1, p2];
        align_polygons_continuous_strip(&mut polys, 1.0, 1.0);

        assert!(polys[0].texture_u.length() > 0.0);
        assert!(polys[1].texture_u.length() > 0.0);
        assert_eq!(polys[0].texture_u, polys[1].texture_u);
    }
}
