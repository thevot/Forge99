// crates/ut99-editor/src/culling.rs
//! High-Performance Dynamic View Frustum Culling Engine (03)
//!
//! Provides Gribb-Hartmann analytical extraction of camera frustum planes
//! from the combined View-Projection matrix with sub-microsecond AABB and sphere
//! intersection tests to cull off-screen brushes, actors, and lights.

use glam::{Mat4, Vec3, Vec4};

/// An infinite half-space plane defined by unit normal vector `N` and signed offset `distance`.
/// A point `P` satisfies `normal.dot(P) + distance >= 0` if it is inside the half-space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrustumPlane {
    pub normal: Vec3,
    pub distance: f32,
}

impl FrustumPlane {
    /// Creates a normalized frustum plane from raw algebraic coefficients `(a, b, c, d)`.
    #[inline]
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        let len = (a * a + b * b + c * c).sqrt();
        if len > 1e-12 {
            let inv = 1.0 / len;
            Self {
                normal: Vec3::new(a * inv, b * inv, c * inv),
                distance: d * inv,
            }
        } else {
            // Degenerate row: treat as always-inside so no geometry is spuriously culled.
            Self {
                normal: Vec3::ZERO,
                distance: f32::INFINITY,
            }
        }
    }

    /// Computes the signed distance from a 3D point to the plane.
    #[inline]
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

/// A 6-sided camera view frustum (Left, Right, Bottom, Top, Near, Far).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum {
    pub planes: [FrustumPlane; 6],
}

impl Frustum {
    /// Extracts the 6 frustum planes from a combined 4x4 View-Projection matrix
    /// using the standard Gribb-Hartmann analytical algorithm.
    pub fn from_view_projection(vp: &Mat4) -> Self {
        // glam Mat4 storage is column-major: cols[col][row]
        // Rows of the matrix:
        let r0 = Vec4::new(vp.x_axis.x, vp.y_axis.x, vp.z_axis.x, vp.w_axis.x);
        let r1 = Vec4::new(vp.x_axis.y, vp.y_axis.y, vp.z_axis.y, vp.w_axis.y);
        let r2 = Vec4::new(vp.x_axis.z, vp.y_axis.z, vp.z_axis.z, vp.w_axis.z);
        let r3 = Vec4::new(vp.x_axis.w, vp.y_axis.w, vp.z_axis.w, vp.w_axis.w);

        // 1. Left Plane:   row3 + row0
        let p_left = r3 + r0;
        // 2. Right Plane:  row3 - row0
        let p_right = r3 - r0;
        // 3. Bottom Plane: row3 + row1
        let p_bottom = r3 + r1;
        // 4. Top Plane:    row3 - row1
        let p_top = r3 - r1;
        // 5. Near Plane:   row3 + row2 (or row2 for 0..1 clip space)
        let p_near = r2;
        // 6. Far Plane:    row3 - row2
        let p_far = r3 - r2;

        Self {
            planes: [
                FrustumPlane::new(p_left.x, p_left.y, p_left.z, p_left.w),
                FrustumPlane::new(p_right.x, p_right.y, p_right.z, p_right.w),
                FrustumPlane::new(p_bottom.x, p_bottom.y, p_bottom.z, p_bottom.w),
                FrustumPlane::new(p_top.x, p_top.y, p_top.z, p_top.w),
                FrustumPlane::new(p_near.x, p_near.y, p_near.z, p_near.w),
                FrustumPlane::new(p_far.x, p_far.y, p_far.z, p_far.w),
            ],
        }
    }

    /// Tests whether an Axis-Aligned Bounding Box (AABB) intersects or is contained within the frustum.
    /// Returns `false` if the AABB is completely outside any of the 6 planes.
    #[inline]
    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            // Find the positive vertex (the corner furthest along the normal direction)
            let p = Vec3::new(
                if plane.normal.x >= 0.0 { max.x } else { min.x },
                if plane.normal.y >= 0.0 { max.y } else { min.y },
                if plane.normal.z >= 0.0 { max.z } else { min.z },
            );
            if plane.distance_to_point(p) < 0.0 {
                return false; // Entire AABB is outside this plane
            }
        }
        true
    }

    /// Tests whether a bounding sphere with center `center` and radius `radius`
    /// intersects or is contained within the frustum.
    #[inline]
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(center) < -radius {
                return false; // Entire sphere is outside this plane
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::{Camera, ViewportType};

    #[test]
    fn frustum_from_vp_covers_perspective_and_max_zoom_orthographic_planes() {
        let mut cam = Camera::default();
        cam.position = Vec3::ZERO;
        cam.yaw = 0.0;
        cam.pitch = 0.0;
        cam.ortho_pos_top = glam::Vec2::ZERO;
        cam.ortho_pos_front = glam::Vec2::ZERO;
        cam.ortho_pos_side = glam::Vec2::ZERO;
        cam.set_ortho_zoom(ViewportType::Top, 100_000.0);
        cam.set_ortho_zoom(ViewportType::Front, 100_000.0);
        cam.set_ortho_zoom(ViewportType::Side, 100_000.0);

        let aspect = 16.0 / 9.0;

        // Perspective: +X forward, +Y right, +Z up
        let persp = Frustum::from_view_projection(
            &cam.view_projection_matrix_for_type(ViewportType::Perspective, aspect),
        );
        assert!(persp.intersects_aabb(Vec3::new(400.0, -32.0, -32.0), Vec3::new(600.0, 32.0, 32.0)));
        assert!(persp.intersects_aabb(Vec3::new(-16.0, -16.0, -16.0), Vec3::new(32.0, 16.0, 16.0)));
        for (outside_min, outside_max) in [
            (Vec3::new(-50.0, -8.0, -8.0), Vec3::new(-10.0, 8.0, 8.0)),         // behind near
            (Vec3::new(60_000.0, -8.0, -8.0), Vec3::new(60_100.0, 8.0, 8.0)),    // beyond far
            (Vec3::new(400.0, -2_000.0, -8.0), Vec3::new(500.0, -1_900.0, 8.0)), // left (-Y)
            (Vec3::new(400.0, 1_900.0, -8.0), Vec3::new(500.0, 2_000.0, 8.0)),   // right (+Y)
            (Vec3::new(400.0, -8.0, 2_000.0), Vec3::new(500.0, 8.0, 2_100.0)),   // top (+Z)
            (Vec3::new(400.0, -8.0, -2_100.0), Vec3::new(500.0, 8.0, -2_000.0)), // bottom (-Z)
        ] {
            assert!(
                !persp.intersects_aabb(outside_min, outside_max),
                "perspective should cull {outside_min:?}..{outside_max:?}"
            );
        }

        // Top ortho at max zoom 100_000: half_h = 100_000, half_w = 177_777.8, z in [-40_000, 39_999]
        let top = Frustum::from_view_projection(
            &cam.view_projection_matrix_for_type(ViewportType::Top, aspect),
        );
        assert!(top.intersects_aabb(Vec3::splat(-64.0), Vec3::splat(64.0)));
        assert!(top.intersects_aabb(Vec3::new(170_000.0, 90_000.0, -10.0), Vec3::new(175_000.0, 95_000.0, 10.0)));
        for (outside_min, outside_max) in [
            (Vec3::new(-190_000.0, -10.0, -10.0), Vec3::new(-185_000.0, 10.0, 10.0)), // -X
            (Vec3::new(185_000.0, -10.0, -10.0), Vec3::new(190_000.0, 10.0, 10.0)),   // +X
            (Vec3::new(-10.0, -110_000.0, -10.0), Vec3::new(10.0, -105_000.0, 10.0)), // -Y
            (Vec3::new(-10.0, 105_000.0, -10.0), Vec3::new(10.0, 110_000.0, 10.0)),   // +Y
            (Vec3::new(-10.0, -10.0, 41_000.0), Vec3::new(10.0, 10.0, 42_000.0)),     // near (+Z)
            (Vec3::new(-10.0, -10.0, -42_000.0), Vec3::new(10.0, 10.0, -41_000.0)),   // far (-Z)
        ] {
            assert!(
                !top.intersects_aabb(outside_min, outside_max),
                "top ortho should cull {outside_min:?}..{outside_max:?}"
            );
        }

        // Front ortho at max zoom 100_000: y in [-39_999, 40_000]
        let front = Frustum::from_view_projection(
            &cam.view_projection_matrix_for_type(ViewportType::Front, aspect),
        );
        assert!(front.intersects_aabb(Vec3::splat(-64.0), Vec3::splat(64.0)));
        for (outside_min, outside_max) in [
            (Vec3::new(-190_000.0, -10.0, -10.0), Vec3::new(-185_000.0, 10.0, 10.0)),
            (Vec3::new(185_000.0, -10.0, -10.0), Vec3::new(190_000.0, 10.0, 10.0)),
            (Vec3::new(-10.0, -10.0, -110_000.0), Vec3::new(10.0, 10.0, -105_000.0)),
            (Vec3::new(-10.0, -10.0, 105_000.0), Vec3::new(10.0, 10.0, 110_000.0)),
            (Vec3::new(-10.0, -42_000.0, -10.0), Vec3::new(10.0, -41_000.0, 10.0)),
            (Vec3::new(-10.0, 41_000.0, -10.0), Vec3::new(10.0, 42_000.0, 10.0)),
        ] {
            assert!(
                !front.intersects_aabb(outside_min, outside_max),
                "front ortho should cull {outside_min:?}..{outside_max:?}"
            );
        }

        // Side ortho at max zoom 100_000: x in [-39_999, 40_000]
        let side = Frustum::from_view_projection(
            &cam.view_projection_matrix_for_type(ViewportType::Side, aspect),
        );
        assert!(side.intersects_aabb(Vec3::splat(-64.0), Vec3::splat(64.0)));
        for (outside_min, outside_max) in [
            (Vec3::new(-10.0, -190_000.0, -10.0), Vec3::new(10.0, -185_000.0, 10.0)),
            (Vec3::new(-10.0, 185_000.0, -10.0), Vec3::new(10.0, 190_000.0, 10.0)),
            (Vec3::new(-10.0, -10.0, -110_000.0), Vec3::new(10.0, 10.0, -105_000.0)),
            (Vec3::new(-10.0, -10.0, 105_000.0), Vec3::new(10.0, 10.0, 110_000.0)),
            (Vec3::new(-42_000.0, -10.0, -10.0), Vec3::new(-41_000.0, 10.0, 10.0)),
            (Vec3::new(41_000.0, -10.0, -10.0), Vec3::new(42_000.0, 10.0, 10.0)),
        ] {
            assert!(
                !side.intersects_aabb(outside_min, outside_max),
                "side ortho should cull {outside_min:?}..{outside_max:?}"
            );
        }
    }
}
