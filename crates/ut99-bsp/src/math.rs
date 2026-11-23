use glam::Vec3;

pub const THRESH_POINT_ON_PLANE: f32 = 0.001;
pub const THRESH_VECTORS_ARE_NEAR: f32 = 0.0001;

/// Plane equation: Ax + By + Cz - D = 0 (Normal . P - D = 0)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub dist: f32,
}

impl Plane {
    pub fn from_point_and_normal(point: Vec3, normal: Vec3) -> Self {
        let n = normal.normalize();
        let dist = n.dot(point);
        Self { normal: n, dist }
    }

    pub fn from_points(p0: Vec3, p1: Vec3, p2: Vec3) -> Option<Self> {
        let n = (p1 - p0).cross(p2 - p0).normalize();
        if n.is_nan() || n.length_squared() < 0.5 {
            None
        } else {
            Some(Self::from_point_and_normal(p0, n))
        }
    }

    pub fn dist_to_point(&self, p: Vec3) -> f32 {
        self.normal.dot(p) - self.dist
    }

    pub fn classify_point(&self, p: Vec3, epsilon: f32) -> PointClassification {
        let d = self.dist_to_point(p);
        if d > epsilon {
            PointClassification::Front
        } else if d < -epsilon {
            PointClassification::Back
        } else {
            PointClassification::Coplanar
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointClassification {
    Front,
    Back,
    Coplanar,
}
