use crate::brush::Brush;
use crate::math::Plane;
use crate::poly::{Polygon, SplitResult};
use glam::Vec3;

/// Clips a brush against a cutting plane.
/// If `keep_front` is true, keeps the side in front of the plane (normal direction);
/// otherwise keeps the side in back of the plane.
/// If the brush is cut, a capping polygon is added to seal the open cut surface.
pub fn clip_brush_by_plane(brush: &Brush, plane: &Plane, keep_front: bool) -> Option<Brush> {
    let mut kept_polys = Vec::new();
    let mut cut_points = Vec::new();

    for poly in &brush.polys {
        match poly.split(plane) {
            SplitResult::Front(p) => {
                if keep_front {
                    kept_polys.push(p);
                }
            }
            SplitResult::Back(p) => {
                if !keep_front {
                    kept_polys.push(p);
                }
            }
            SplitResult::Spanning { front, back } => {
                // Collect intersection points on the plane
                for v in &front.vertices {
                    if plane.dist_to_point(*v).abs() < 0.01 {
                        cut_points.push(*v);
                    }
                }
                if keep_front {
                    kept_polys.push(front);
                } else {
                    kept_polys.push(back);
                }
            }
        }
    }

    if kept_polys.is_empty() {
        return None;
    }

    // If there were spanning cuts, create a cap polygon
    if let Some(cap) = build_cap_polygon(&cut_points, plane, keep_front) {
        kept_polys.push(cap);
    }

    let mut result = brush.clone();
    result.polys = kept_polys;
    Some(result)
}

/// Splits a brush into two separate brushes along a plane: (FrontBrush, BackBrush).
pub fn split_brush_by_plane(brush: &Brush, plane: &Plane) -> (Option<Brush>, Option<Brush>) {
    let front = clip_brush_by_plane(brush, plane, true);
    let back = clip_brush_by_plane(brush, plane, false);
    (front, back)
}

/// Builds a planar cap polygon from a cloud of intersection points lying on the plane.
fn build_cap_polygon(points: &[Vec3], plane: &Plane, is_front_side: bool) -> Option<Polygon> {
    if points.len() < 3 {
        return None;
    }

    // 1. Remove duplicate points
    let mut unique_points: Vec<Vec3> = Vec::new();
    for &p in points {
        if !unique_points.iter().any(|u| u.distance_squared(p) < 0.01) {
            unique_points.push(p);
        }
    }

    if unique_points.len() < 3 {
        return None;
    }

    // 2. Compute center of points
    let mut center = Vec3::ZERO;
    for &p in &unique_points {
        center += p;
    }
    center /= unique_points.len() as f32;

    // 3. Setup orthonormal basis on the cutting plane
    let norm = plane.normal.normalize();
    // Cap normal faces outward from the kept piece:
    // Front piece: points are >= plane.dist, outward at cut is -norm
    // Back piece: points are <= plane.dist, outward at cut is +norm
    let cap_normal = if is_front_side { -norm } else { norm };

    let u_axis = if norm.x.abs() < 0.9 {
        norm.cross(Vec3::X).normalize()
    } else {
        norm.cross(Vec3::Y).normalize()
    };
    let v_axis = norm.cross(u_axis).normalize();

    // 4. Sort points in angular order around the cap normal
    unique_points.sort_by(|a, b| {
        let da = *a - center;
        let db = *b - center;
        let angle_a = da.dot(v_axis).atan2(da.dot(u_axis));
        let angle_b = db.dot(v_axis).atan2(db.dot(u_axis));
        angle_a.partial_cmp(&angle_b).unwrap_or(std::cmp::Ordering::Equal)
    });

    // 5. Ensure winding matches cap normal (CCW)
    let p0 = unique_points[0];
    let p1 = unique_points[1];
    let p2 = unique_points[2];
    let computed_normal = (p1 - p0).cross(p2 - p0);
    if computed_normal.dot(cap_normal) < 0.0 {
        unique_points.reverse();
    }

    Polygon::new(unique_points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::build_cube;
    use crate::CsgOperation;

    #[test]
    fn test_clip_cube_by_horizontal_plane() {
        let cube = build_cube("TestCube", CsgOperation::Active, Vec3::splat(256.0));
        // Cutting plane at Z = 0 with normal Vec3::Z
        let plane = Plane::from_point_and_normal(Vec3::ZERO, Vec3::Z);

        let (front, back) = split_brush_by_plane(&cube, &plane);
        assert!(front.is_some(), "Front piece should exist");
        assert!(back.is_some(), "Back piece should exist");

        let f = front.unwrap();
        let b = back.unwrap();

        // Both parts should have polygons (original cut sides + cap)
        assert!(f.polys.len() >= 6);
        assert!(b.polys.len() >= 6);
    }
}
