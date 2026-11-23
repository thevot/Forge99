use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoMode {
    #[default]
    Translate,
    Rotate,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoAxis {
    #[default]
    None,
    X,
    Y,
    Z,
    PlaneXY,
    PlaneXZ,
    PlaneYZ,
    ScreenPlane,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    pub fn from_screen(
        mouse_x: f32,
        mouse_y: f32,
        width: f32,
        height: f32,
        inv_view_proj: Mat4,
    ) -> Self {
        let ndc_x = (2.0 * mouse_x / width) - 1.0;
        let ndc_y = 1.0 - (2.0 * mouse_y / height);

        let near_h = inv_view_proj.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
        let far_h = inv_view_proj.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));

        let dir = (far_h - near_h).normalize();
        Self::new(near_h, dir)
    }

    /// Intersects ray with a sphere at `center` with `radius`.
    pub fn intersect_sphere(&self, center: Vec3, radius: f32) -> Option<f32> {
        let oc = self.origin - center;
        let b = oc.dot(self.direction);
        let c = oc.dot(oc) - radius * radius;
        let discriminant = b * b - c;

        if discriminant < 0.0 {
            None
        } else {
            let t = -b - discriminant.sqrt();
            if t > 0.0 {
                Some(t)
            } else {
                let t2 = -b + discriminant.sqrt();
                if t2 > 0.0 {
                    Some(t2)
                } else {
                    None
                }
            }
        }
    }

    /// Intersects ray with an axis-aligned bounding box (NaN-free for axis-aligned and flat bounds).
    pub fn intersect_aabb(&self, min: Vec3, max: Vec3) -> Option<f32> {
        let mut near = f32::NEG_INFINITY;
        let mut far = f32::INFINITY;
        let orig = self.origin.to_array();
        let dir = self.direction.to_array();
        let bmin = min.to_array();
        let bmax = max.to_array();

        for i in 0..3 {
            if dir[i].abs() < 1e-12 {
                if orig[i] < bmin[i] || orig[i] > bmax[i] {
                    return None;
                }
            } else {
                let inv = 1.0 / dir[i];
                let mut t0 = (bmin[i] - orig[i]) * inv;
                let mut t1 = (bmax[i] - orig[i]) * inv;
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                near = near.max(t0);
                far = far.min(t1);
                if near > far {
                    return None;
                }
            }
        }

        if far > 0.0 {
            Some(near.max(0.0))
        } else {
            None
        }
    }

    /// Möller–Trumbore ray-triangle intersection algorithm.
    pub fn intersect_triangle(&self, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
        const EPSILON: f32 = 1e-7;
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let h = self.direction.cross(edge2);
        let a = edge1.dot(h);

        if a.abs() < EPSILON {
            return None;
        }

        let f = 1.0 / a;
        let s = self.origin - v0;
        let u = f * s.dot(h);

        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * self.direction.dot(q);

        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);
        if t > EPSILON {
            Some(t)
        } else {
            None
        }
    }

    /// Closest distance between this ray and a 3D line segment (p0 to p1).
    pub fn distance_to_segment(&self, p0: Vec3, p1: Vec3) -> f32 {
        let u = p1 - p0;
        let v = self.direction;
        let w = p0 - self.origin;

        let a = u.dot(u);
        let b = u.dot(v);
        let c = v.dot(v);
        let d = u.dot(w);
        let e = v.dot(w);

        let denom = a * c - b * b;
        let sc = if denom < 1e-6 {
            0.0
        } else {
            ((b * e - c * d) / denom).clamp(0.0, 1.0)
        };

        let tc = (b * sc + e) / c;
        let closest_point_ray = self.origin + v * tc.max(0.0);
        let closest_point_seg = p0 + u * sc;

        closest_point_ray.distance(closest_point_seg)
    }

    /// Intersects ray with an infinite plane defined by a point `plane_point` and normal `plane_normal`.
    /// Returns the 3D world position of intersection if the ray hits the plane in front of its origin.
    pub fn project_onto_plane(&self, plane_point: Vec3, plane_normal: Vec3) -> Option<Vec3> {
        let n = plane_normal.normalize();
        let denom = self.direction.dot(n);
        if denom.abs() < 1e-5 {
            return None; // Ray is parallel to plane
        }
        let t = (plane_point - self.origin).dot(n) / denom;
        if t <= 0.0 {
            return None; // Behind camera
        }
        Some(self.origin + self.direction * t)
    }

    /// Tests which axis or planar handle of an oriented 3D transform gizmo at `center` with rotation `rot` is intersected.
    pub fn hit_test_gizmo_oriented(&self, center: Vec3, axis_length: f32, hit_radius: f32, rot: glam::Mat3) -> GizmoAxis {
        let inv_rot = rot.transpose();
        let local_origin = inv_rot * (self.origin - center);
        let local_dir = (inv_rot * self.direction).normalize();
        let local_ray = Ray { origin: local_origin, direction: local_dir };
        local_ray.hit_test_gizmo(Vec3::ZERO, axis_length, hit_radius)
    }

    /// Tests which rotation ring (X, Y, or Z) of an oriented gizmo is intersected.
    pub fn hit_test_rotation_gizmo_oriented(&self, center: Vec3, ring_radius: f32, hit_threshold: f32, rot: glam::Mat3) -> GizmoAxis {
        let inv_rot = rot.transpose();
        let local_origin = inv_rot * (self.origin - center);
        let local_dir = (inv_rot * self.direction).normalize();
        let local_ray = Ray { origin: local_origin, direction: local_dir };
        local_ray.hit_test_rotation_gizmo(Vec3::ZERO, ring_radius, hit_threshold)
    }

    /// Tests which axis or planar handle of the 3D transform gizmo at `center` is intersected by the ray.
    pub fn hit_test_gizmo(&self, center: Vec3, axis_length: f32, hit_radius: f32) -> GizmoAxis {
        // 0. Check central handle (ScreenPlane) first
        let center_radius = (axis_length * 0.18).max(hit_radius * 0.85);
        if self.intersect_sphere(center, center_radius).is_some() {
            return GizmoAxis::ScreenPlane;
        }

        // 1. Check planar square handles first (situated between axis pairs)
        let plane_offset = axis_length * 0.20;
        let plane_size = axis_length * 0.28;

        // Plane XY (Normal Z: ground plane)
        if let Some(hit) = self.project_onto_plane(center, Vec3::Z) {
            let dx = hit.x - center.x;
            let dy = hit.y - center.y;
            if dx >= plane_offset && dx <= (plane_offset + plane_size)
                && dy >= plane_offset && dy <= (plane_offset + plane_size)
            {
                return GizmoAxis::PlaneXY;
            }
        }

        // Plane XZ (Normal Y: front plane)
        if let Some(hit) = self.project_onto_plane(center, Vec3::Y) {
            let dx = hit.x - center.x;
            let dz = hit.z - center.z;
            if dx >= plane_offset && dx <= (plane_offset + plane_size)
                && dz >= plane_offset && dz <= (plane_offset + plane_size)
            {
                return GizmoAxis::PlaneXZ;
            }
        }

        // Plane YZ (Normal X: side plane)
        if let Some(hit) = self.project_onto_plane(center, Vec3::X) {
            let dy = hit.y - center.y;
            let dz = hit.z - center.z;
            if dy >= plane_offset && dy <= (plane_offset + plane_size)
                && dz >= plane_offset && dz <= (plane_offset + plane_size)
            {
                return GizmoAxis::PlaneYZ;
            }
        }

        // 2. Check linear 1D axes (X, Y, Z)
        let p_x = center + Vec3::new(axis_length, 0.0, 0.0);
        let p_y = center + Vec3::new(0.0, axis_length, 0.0);
        let p_z = center + Vec3::new(0.0, 0.0, axis_length);

        let d_x = self.distance_to_segment(center, p_x);
        let d_y = self.distance_to_segment(center, p_y);
        let d_z = self.distance_to_segment(center, p_z);

        let min_d = d_x.min(d_y).min(d_z);
        if min_d > hit_radius {
            return GizmoAxis::None;
        }

        if (d_x - min_d).abs() < 1e-5 {
            GizmoAxis::X
        } else if (d_y - min_d).abs() < 1e-5 {
            GizmoAxis::Y
        } else {
            GizmoAxis::Z
        }
    }

    /// Tests which rotation ring (X, Y, or Z) of radius `ring_radius` is intersected by the ray.
    pub fn hit_test_rotation_gizmo(
        &self,
        center: Vec3,
        ring_radius: f32,
        hit_threshold: f32,
    ) -> GizmoAxis {
        let axes = [
            (GizmoAxis::X, Vec3::X),
            (GizmoAxis::Y, Vec3::Y),
            (GizmoAxis::Z, Vec3::Z),
        ];

        let mut best_axis = GizmoAxis::None;
        let mut min_diff = hit_threshold;

        for (axis, normal) in axes {
            let denom = self.direction.dot(normal);
            if denom.abs() < 1e-5 {
                continue;
            }
            let t = (center - self.origin).dot(normal) / denom;
            if t <= 0.0 {
                continue;
            }
            let hit_pt = self.origin + self.direction * t;
            let dist = (hit_pt - center).length();
            let diff = (dist - ring_radius).abs();
            if diff < min_diff {
                min_diff = diff;
                best_axis = axis;
            }
        }

        best_axis
    }

    /// Like [`Ray::get_rotation_angle`], but measured in the gizmo's own (possibly rotated) frame `rot`.
    /// The returned angle is a right-handed rotation about the world axis `rot * axis`.
    pub fn get_rotation_angle_oriented(&self, center: Vec3, axis: GizmoAxis, rot: glam::Mat3) -> Option<f32> {
        let inv_rot = rot.transpose();
        let local_ray = Ray {
            origin: inv_rot * (self.origin - center),
            direction: (inv_rot * self.direction).normalize(),
        };
        local_ray.get_rotation_angle(Vec3::ZERO, axis)
    }

    /// Computes the angle in radians around `axis` (X, Y, or Z) of the ray intersection with the gizmo plane.
    pub fn get_rotation_angle(&self, center: Vec3, axis: GizmoAxis) -> Option<f32> {
        let (normal, u_axis, v_axis) = match axis {
            GizmoAxis::X => (Vec3::X, Vec3::Y, Vec3::Z),
            GizmoAxis::Y => (Vec3::Y, Vec3::Z, Vec3::X),
            GizmoAxis::Z => (Vec3::Z, Vec3::X, Vec3::Y),
            _ => return None,
        };

        let denom = self.direction.dot(normal);
        if denom.abs() < 1e-5 {
            return None;
        }
        let t = (center - self.origin).dot(normal) / denom;
        if t <= 0.0 {
            return None;
        }
        let hit = self.origin + self.direction * t;
        let offset = hit - center;
        let u = offset.dot(u_axis);
        let v = offset.dot(v_axis);
        Some(v.atan2(u))
    }

    /// Projects ray onto a 3D line passing through `center` in direction `axis_dir`,
    /// returning the signed distance `s` along `axis_dir` where the ray is closest.
    pub fn project_onto_axis(&self, center: Vec3, axis_dir: Vec3) -> Option<f32> {
        let d = axis_dir.normalize();
        let r = self.direction;
        let b = r.dot(d);
        let denom = 1.0 - b * b;
        if denom.abs() < 1e-4 {
            return None; // Ray is parallel to the axis
        }
        let w = self.origin - center;
        let e = w.dot(d);
        let d_val = w.dot(r);
        let s = (e - b * d_val) / denom;
        Some(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrustumPlane {
    pub normal: Vec3,
    pub distance: f32,
}

impl FrustumPlane {
    pub fn new(point: Vec3, normal: Vec3) -> Self {
        let n = normal.normalize();
        Self {
            normal: n,
            distance: -n.dot(point),
        }
    }

    pub fn distance_to_point(&self, p: Vec3) -> f32 {
        self.normal.dot(p) + self.distance
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frustum3D {
    pub planes: [FrustumPlane; 6], // Left, Right, Top, Bottom, Near, Far
}

impl Frustum3D {
    pub fn from_screen_rect(
        x0: f32, y0: f32, x1: f32, y1: f32,
        width: f32, height: f32,
        inv_view_proj: glam::Mat4,
        cam_pos: Vec3,
    ) -> Option<Self> {
        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);

        if width <= 0.0 || height <= 0.0 || (max_x - min_x) < 3.0 || (max_y - min_y) < 3.0 {
            return None;
        }

        let r_tl = Ray::from_screen(min_x, min_y, width, height, inv_view_proj);
        let r_tr = Ray::from_screen(max_x, min_y, width, height, inv_view_proj);
        let r_bl = Ray::from_screen(min_x, max_y, width, height, inv_view_proj);
        let r_br = Ray::from_screen(max_x, max_y, width, height, inv_view_proj);

        let fwd = (r_tl.direction + r_tr.direction + r_bl.direction + r_br.direction).normalize_or_zero();
        if fwd.length_squared() < 0.5 {
            return None;
        }
        let inward = |raw: Vec3| -> Option<Vec3> {
            let n = raw.normalize_or_zero();
            if n.length_squared() < 0.5 {
                return None;
            }
            Some(if n.dot(fwd) < 0.0 { -n } else { n })
        };

        let n_left = inward(r_tl.direction.cross(r_bl.direction))?;
        let n_right = inward(r_br.direction.cross(r_tr.direction))?;
        let n_top = inward(r_tr.direction.cross(r_tl.direction))?;
        let n_bottom = inward(r_bl.direction.cross(r_br.direction))?;

        let near_plane = FrustumPlane::new(cam_pos + fwd * 2.0, fwd);
        let far_plane = FrustumPlane::new(cam_pos + fwd * 65536.0, -fwd);

        Some(Self {
            planes: [
                FrustumPlane::new(cam_pos, n_left),
                FrustumPlane::new(cam_pos, n_right),
                FrustumPlane::new(cam_pos, n_top),
                FrustumPlane::new(cam_pos, n_bottom),
                near_plane,
                far_plane,
            ],
        })
    }

    /// Tests if an axis-aligned bounding box intersects or is contained within the frustum.
    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            let mut p = min;
            if plane.normal.x > 0.0 { p.x = max.x; }
            if plane.normal.y > 0.0 { p.y = max.y; }
            if plane.normal.z > 0.0 { p.z = max.z; }

            if plane.distance_to_point(p) < 0.0 {
                return false;
            }
        }
        true
    }

    /// Tests if a sphere intersects or is contained within the frustum.
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(center) < -radius {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gizmo_hit_x() {
        let center = Vec3::new(100.0, 100.0, 100.0);
        let ray = Ray::new(Vec3::new(140.0, 0.0, 100.0), Vec3::new(0.0, 1.0, 0.0));
        let axis = ray.hit_test_gizmo(center, 80.0, 16.0);
        assert_eq!(axis, GizmoAxis::X);
    }

    #[test]
    fn test_gizmo_hit_planexy() {
        let center = Vec3::ZERO;
        let axis_len = 80.0;
        // PlaneXY tile is between offset (80 * 0.20 = 16) and (16 + 22.4 = 38.4) on X and Y, with Z = 0
        let ray = Ray::new(Vec3::new(25.0, 25.0, 100.0), Vec3::new(0.0, 0.0, -1.0));
        let axis = ray.hit_test_gizmo(center, axis_len, 16.0);
        assert_eq!(axis, GizmoAxis::PlaneXY);
    }

    #[test]
    fn test_gizmo_hit_planexz() {
        let center = Vec3::ZERO;
        let axis_len = 80.0;
        // PlaneXZ tile is between offset (16) and (38.4) on X and Z, with Y = 0
        let ray = Ray::new(Vec3::new(25.0, 100.0, 25.0), Vec3::new(0.0, -1.0, 0.0));
        let axis = ray.hit_test_gizmo(center, axis_len, 16.0);
        assert_eq!(axis, GizmoAxis::PlaneXZ);
    }

    #[test]
    fn test_rotation_gizmo_hit_z() {
        let center = Vec3::new(0.0, 0.0, 0.0);
        // Ray heading down towards the XY plane (normal Z) at (64, 0, 50) heading -Z
        let ray = Ray::new(Vec3::new(64.0, 0.0, 50.0), Vec3::new(0.0, 0.0, -1.0));
        let axis = ray.hit_test_rotation_gizmo(center, 64.0, 8.0);
        assert_eq!(axis, GizmoAxis::Z);

        let angle = ray.get_rotation_angle(center, GizmoAxis::Z);
        assert!(angle.is_some());
        let a = angle.unwrap();
        // Point is (64, 0) so angle along u (X) is ~0 radians
        assert!(a.abs() < 1e-3);
    }

    #[test]
    fn test_project_onto_axis() {
        let center = Vec3::ZERO;
        let ray = Ray::new(Vec3::new(50.0, -200.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        let s = ray.project_onto_axis(center, Vec3::X).expect("Should intersect axis");
        assert!((s - 50.0).abs() < 1e-3);
    }

    #[test]
    fn test_project_onto_plane() {
        let plane_point = Vec3::new(0.0, 0.0, 100.0);
        let plane_normal = Vec3::Z;
        let ray = Ray::new(Vec3::new(10.0, 20.0, 0.0), Vec3::new(0.0, 0.0, 1.0));
        let hit = ray.project_onto_plane(plane_point, plane_normal).expect("Should intersect plane");
        assert_eq!(hit, Vec3::new(10.0, 20.0, 100.0));
    }

    #[test]
    fn test_screen_plane_gizmo_hit() {
        let center = Vec3::new(50.0, 50.0, 50.0);
        // Ray aimed directly at the origin of the gizmo
        let ray = Ray::new(Vec3::new(50.0, -100.0, 50.0), Vec3::new(0.0, 1.0, 0.0));
        let hit = ray.hit_test_gizmo(center, 80.0, 16.0);
        assert_eq!(hit, GizmoAxis::ScreenPlane);
    }

    #[test]
    fn test_frustum_3d_aabb_and_sphere_intersection() {
        let cam_pos = Vec3::new(0.0, 0.0, 0.0);
        let planes = [
            FrustumPlane::new(cam_pos, Vec3::X),
            FrustumPlane::new(Vec3::new(200.0, 0.0, 0.0), -Vec3::X),
            FrustumPlane::new(cam_pos, Vec3::Y),
            FrustumPlane::new(Vec3::new(0.0, 200.0, 0.0), -Vec3::Y),
            FrustumPlane::new(cam_pos, Vec3::Z),
            FrustumPlane::new(Vec3::new(0.0, 0.0, 200.0), -Vec3::Z),
        ];
        let frustum = Frustum3D { planes };

        // Point inside
        assert!(frustum.intersects_sphere(Vec3::new(50.0, 50.0, 50.0), 10.0));
        assert!(frustum.intersects_aabb(Vec3::new(40.0, 40.0, 40.0), Vec3::new(60.0, 60.0, 60.0)));

        // Point far outside
        assert!(!frustum.intersects_sphere(Vec3::new(500.0, 50.0, 50.0), 10.0));
        assert!(!frustum.intersects_aabb(Vec3::new(400.0, 40.0, 40.0), Vec3::new(600.0, 60.0, 60.0)));
    }

    #[test]
    fn test_intersect_aabb_orthographic_faces_edges_flat_sheets_and_interior() {
        let min = Vec3::new(-64.0, -32.0, -16.0);
        let max = Vec3::new(64.0, 32.0, 16.0);

        // Orthographic rays grazing exact min/max faces, edges, and corners without 0 * inf = NaN
        for &(x, y) in &[
            (-64.0, -32.0),
            (64.0, 32.0),
            (-64.0, 0.0),
            (0.0, 32.0),
            (0.0, 0.0),
        ] {
            let ray = Ray::new(Vec3::new(x, y, 200.0), Vec3::new(0.0, 0.0, -1.0));
            let t = ray.intersect_aabb(min, max).expect("exact -Z boundary hit");
            assert!((t - 184.0).abs() < 1e-4, "unexpected t={t} at ({x},{y})");
        }

        let ray_pos_x = Ray::new(Vec3::new(-200.0, -32.0, 16.0), Vec3::X);
        assert!((ray_pos_x.intersect_aabb(min, max).unwrap() - 136.0).abs() < 1e-4);

        let ray_pos_y = Ray::new(Vec3::new(64.0, -100.0, -16.0), Vec3::Y);
        assert!((ray_pos_y.intersect_aabb(min, max).unwrap() - 68.0).abs() < 1e-4);

        // Flat zero-thickness Sheet brush (min.z == max.z)
        let sheet_min = Vec3::new(-64.0, -64.0, 12.0);
        let sheet_max = Vec3::new(64.0, 64.0, 12.0);
        let top_ray = Ray::new(Vec3::new(0.0, 0.0, 100.0), Vec3::NEG_Z);
        assert!((top_ray.intersect_aabb(sheet_min, sheet_max).unwrap() - 88.0).abs() < 1e-4);

        // Origin strictly inside AABB returns 0.0
        let inside_ray = Ray::new(Vec3::ZERO, Vec3::new(1.0, 2.0, -3.0));
        assert_eq!(inside_ray.intersect_aabb(min, max), Some(0.0));

        // AABB completely behind ray origin returns None
        let behind_ray = Ray::new(Vec3::new(0.0, 0.0, 200.0), Vec3::Z);
        assert_eq!(behind_ray.intersect_aabb(min, max), None);

        // Lateral misses return None
        let miss_ray = Ray::new(Vec3::new(64.1, 0.0, 200.0), Vec3::NEG_Z);
        assert_eq!(miss_ray.intersect_aabb(min, max), None);
        let miss_diag = Ray::new(Vec3::new(200.0, 200.0, 200.0), Vec3::new(-1.0, 0.2, -1.0));
        assert_eq!(miss_diag.intersect_aabb(min, max), None);

        // Diagonal hit
        let hit_diag = Ray::new(Vec3::new(-128.0, -128.0, -128.0), Vec3::ONE);
        assert!(hit_diag.intersect_aabb(min, max).is_some());
    }

    #[test]
    fn test_frustum_3d_from_screen_rect_centered_corner_rotated_and_reversed() {
        use crate::camera::{Camera, ViewportType};

        let mut cam = Camera::default();
        cam.position = Vec3::ZERO;
        cam.yaw = 0.0;
        cam.pitch = 0.0;
        let (w, h) = (1440.0, 900.0);
        let vp = cam.view_projection_matrix_for_type(ViewportType::Perspective, w / h);
        let inv_vp = vp.inverse();

        // Centered marquee around screen center (720, 450)
        let center_frustum = Frustum3D::from_screen_rect(200.0, 200.0, 1240.0, 700.0, w, h, inv_vp, cam.position)
            .expect("valid centered frustum");
        assert!(center_frustum.intersects_sphere(Vec3::new(300.0, 0.0, 0.0), 8.0));
        assert!(!center_frustum.intersects_sphere(Vec3::new(300.0, 5000.0, 0.0), 8.0));
        assert!(!center_frustum.intersects_sphere(Vec3::new(-200.0, 0.0, 0.0), 8.0));

        // Reversed drag order matches forward drag order
        let rev_frustum = Frustum3D::from_screen_rect(1240.0, 700.0, 200.0, 200.0, w, h, inv_vp, cam.position)
            .expect("valid reversed frustum");
        assert_eq!(center_frustum, rev_frustum);

        // Off-center marquee in top-right quadrant (+Y right, +Z top)
        let corner_frustum = Frustum3D::from_screen_rect(900.0, 50.0, 1400.0, 350.0, w, h, inv_vp, cam.position)
            .expect("valid off-center frustum");
        let target_pt = Vec3::new(500.0, 250.0, 180.0);
        assert!(corner_frustum.intersects_sphere(target_pt, 8.0));
        assert!(!corner_frustum.intersects_sphere(Vec3::new(500.0, 0.0, 0.0), 8.0));
        assert!(!corner_frustum.intersects_sphere(Vec3::new(500.0, -300.0, -200.0), 8.0));

        // Rotated camera
        cam.yaw = 1.2;
        cam.pitch = -0.35;
        cam.position = Vec3::new(-120.0, 240.0, 160.0);
        let rot_vp = cam.view_projection_matrix_for_type(ViewportType::Perspective, w / h);
        let rot_frustum = Frustum3D::from_screen_rect(360.0, 200.0, 1080.0, 700.0, w, h, rot_vp.inverse(), cam.position)
            .expect("valid rotated frustum");
        let fwd_pt = cam.position + cam.forward() * 400.0;
        let back_pt = cam.position - cam.forward() * 100.0;
        assert!(rot_frustum.intersects_sphere(fwd_pt, 8.0));
        assert!(!rot_frustum.intersects_sphere(back_pt, 8.0));
    }
}


