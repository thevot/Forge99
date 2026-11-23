use super::math::{Plane, PointClassification, THRESH_POINT_ON_PLANE};
use glam::Vec3;
// Unreal Engine 1 / UnrealEd 2 Canonical PolyFlags
pub const PF_INVISIBLE: u32 = 0x00000001;
pub const PF_MASKED: u32 = 0x00000002;
pub const PF_TRANSLUCENT: u32 = 0x00000004;
pub const PF_NOT_SOLID: u32 = 0x00000008;
pub const PF_FORCE_VIEW_ZONE: u32 = 0x00000010;
pub const PF_SEMISOLID: u32 = 0x00000020;
pub const PF_MODULATED: u32 = 0x00000040;
pub const PF_FAKE_BACKDROP: u32 = 0x00000080;
pub const PF_TWO_SIDED: u32 = 0x00000100;
pub const PF_AUTO_UPAN: u32 = 0x00000200;
pub const PF_BRIGHT_CORNERS: u32 = 0x00000400;
pub const PF_MIRRORED: u32 = 0x00000800;
pub const PF_DIRTY_SHADOWS: u32 = 0x00001000;
pub const PF_SMALL_WAVY: u32 = 0x00002000;
pub const PF_BIG_WAVY: u32 = 0x00004000;
pub const PF_NO_SMOOTH: u32 = 0x00008000;
pub const PF_AUTO_VPAN: u32 = 0x00010000;
pub const PF_SPECIAL_POLY: u32 = 0x00020000;
pub const PF_LOW_SHADOW_DETAIL: u32 = 0x00040000;
pub const PF_SPECIAL_LIT: u32 = 0x00100000;
pub const PF_LEGACY_SEMISOLID: u32 = 0x00200000;
pub const PF_UNLIT: u32 = 0x00400000;
pub const PF_HIGH_SHADOW_DETAIL: u32 = 0x00800000;
pub const PF_NO_BOUNDS_REJECT: u32 = 0x02000000;
pub const PF_PORTAL: u32 = 0x04000000;
pub const PF_ANTI_PORTAL: u32 = 0x08000000;

#[derive(Debug, Clone, PartialEq)]
pub struct LightmapData {
    pub width: usize,
    pub height: usize,
    pub luxels: Vec<[f32; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    pub vertices: Vec<Vec3>,
    pub plane: Plane,
    pub texture: String,
    pub flags: u32,
    pub base: Vec3,
    pub texture_u: Vec3,
    pub texture_v: Vec3,
    pub pan_u: i16,
    pub pan_v: i16,
    pub vertex_colors: Vec<[f32; 3]>,
    pub lightmap: Option<LightmapData>,
}

impl Polygon {
    pub fn new(vertices: Vec<Vec3>) -> Option<Self> {
        if vertices.len() < 3 {
            return None;
        }
        let plane = Plane::from_points(vertices[0], vertices[1], vertices[2])?;
        Some(Self {
            vertices,
            plane,
            texture: String::new(),
            flags: 0,
            base: Vec3::ZERO,
            texture_u: Vec3::X,
            texture_v: Vec3::Y,
            pan_u: 0,
            pan_v: 0,
            vertex_colors: Vec::new(),
            lightmap: None,
        })
    }

    pub fn with_texture(mut self, texture: impl Into<String>) -> Self {
        self.texture = texture.into();
        self
    }

    pub fn with_flags(mut self, flags: u32) -> Self {
        self.flags = flags;
        self
    }

    pub fn compute_uv(&self, vertex: Vec3) -> [f32; 2] {
        let rel = vertex - self.base;
        let u_dot = rel.dot(self.texture_u) + self.pan_u as f32;
        let v_dot = rel.dot(self.texture_v) + self.pan_v as f32;
        // Canonical Unreal mapping: 128 world units per texture tile when U/V unit vectors
        [u_dot / 128.0, v_dot / 128.0]
    }

    pub fn inverted(&self) -> Self {
        let mut rev = self.vertices.clone();
        rev.reverse();
        let mut rev_colors = self.vertex_colors.clone();
        if !rev_colors.is_empty() {
            rev_colors.reverse();
        }
        Self {
            vertices: rev,
            plane: Plane {
                normal: -self.plane.normal,
                dist: -self.plane.dist,
            },
            texture: self.texture.clone(),
            flags: self.flags,
            base: self.base,
            texture_u: self.texture_u,
            texture_v: self.texture_v,
            pan_u: self.pan_u,
            pan_v: self.pan_v,
            vertex_colors: rev_colors,
            lightmap: self.lightmap.clone(),
        }
    }

    /// Classifies this polygon relative to a plane.
    pub fn classify(&self, plane: &Plane) -> PolygonClassification {
        let mut front = 0;
        let mut back = 0;
        let mut coplanar = 0;

        for &v in &self.vertices {
            match plane.classify_point(v, THRESH_POINT_ON_PLANE) {
                PointClassification::Front => front += 1,
                PointClassification::Back => back += 1,
                PointClassification::Coplanar => coplanar += 1,
            }
        }

        if coplanar == self.vertices.len() {
            // Check if normals are aligned
            if self.plane.normal.dot(plane.normal) > 0.0 {
                PolygonClassification::CoplanarFront
            } else {
                PolygonClassification::CoplanarBack
            }
        } else if front > 0 && back > 0 {
            PolygonClassification::Spanning
        } else if front > 0 {
            PolygonClassification::Front
        } else {
            PolygonClassification::Back
        }
    }

    /// Splits this polygon by a plane into a front piece and a back piece.
    pub fn split(&self, plane: &Plane) -> SplitResult {
        match self.classify(plane) {
            PolygonClassification::Front | PolygonClassification::CoplanarFront => {
                SplitResult::Front(self.clone())
            }
            PolygonClassification::Back | PolygonClassification::CoplanarBack => {
                SplitResult::Back(self.clone())
            }
            PolygonClassification::Spanning => {
                let mut front_verts = Vec::new();
                let mut back_verts = Vec::new();
                let n = self.vertices.len();
                let has_colors = self.vertex_colors.len() == n;
                let mut front_colors = if has_colors { Vec::new() } else { Vec::new() };
                let mut back_colors = if has_colors { Vec::new() } else { Vec::new() };

                for i in 0..n {
                    let v_curr = self.vertices[i];
                    let v_next = self.vertices[(i + 1) % n];

                    let class_curr = plane.classify_point(v_curr, THRESH_POINT_ON_PLANE);
                    let class_next = plane.classify_point(v_next, THRESH_POINT_ON_PLANE);

                    match class_curr {
                        PointClassification::Front => {
                            front_verts.push(v_curr);
                            if has_colors {
                                front_colors.push(self.vertex_colors[i]);
                            }
                        }
                        PointClassification::Back => {
                            back_verts.push(v_curr);
                            if has_colors {
                                back_colors.push(self.vertex_colors[i]);
                            }
                        }
                        PointClassification::Coplanar => {
                            front_verts.push(v_curr);
                            back_verts.push(v_curr);
                            if has_colors {
                                front_colors.push(self.vertex_colors[i]);
                                back_colors.push(self.vertex_colors[i]);
                            }
                        }
                    }

                    // If edge crosses the plane, compute intersection in 64-bit IEEE 754 precision (DVec3)
                    // using canonical vertex ordering so shared opposite edges (A->B vs B->A) yield bit-identical vertices.
                    if (class_curr == PointClassification::Front && class_next == PointClassification::Back)
                        || (class_curr == PointClassification::Back && class_next == PointClassification::Front)
                    {
                        let n_d = glam::DVec3::new(
                            plane.normal.x as f64,
                            plane.normal.y as f64,
                            plane.normal.z as f64,
                        );
                        let dist_d = plane.dist as f64;
                        let (p0, p1) = if (v_curr.x.to_bits(), v_curr.y.to_bits(), v_curr.z.to_bits())
                            <= (v_next.x.to_bits(), v_next.y.to_bits(), v_next.z.to_bits())
                        {
                            (
                                glam::DVec3::new(v_curr.x as f64, v_curr.y as f64, v_curr.z as f64),
                                glam::DVec3::new(v_next.x as f64, v_next.y as f64, v_next.z as f64),
                            )
                        } else {
                            (
                                glam::DVec3::new(v_next.x as f64, v_next.y as f64, v_next.z as f64),
                                glam::DVec3::new(v_curr.x as f64, v_curr.y as f64, v_curr.z as f64),
                            )
                        };
                        let d0 = n_d.dot(p0) - dist_d;
                        let d1 = n_d.dot(p1) - dist_d;
                        let denom = d0 - d1;
                        let t_canon = if denom.abs() > 1e-12 {
                            (d0 / denom).clamp(0.0, 1.0)
                        } else {
                            0.5
                        };
                        let p_hit = p0 + (p1 - p0) * t_canon;
                        let v_intersect = Vec3::new(p_hit.x as f32, p_hit.y as f32, p_hit.z as f32);

                        let d_curr = plane.dist_to_point(v_curr);
                        let d_next = plane.dist_to_point(v_next);
                        let t = if (d_curr - d_next).abs() > 1e-6 {
                            (d_curr / (d_curr - d_next)).clamp(0.0, 1.0)
                        } else {
                            0.5
                        };

                        front_verts.push(v_intersect);
                        back_verts.push(v_intersect);
                        if has_colors {
                            let c0 = self.vertex_colors[i];
                            let c1 = self.vertex_colors[(i + 1) % n];
                            let c_interp = [
                                c0[0] + (c1[0] - c0[0]) * t,
                                c0[1] + (c1[1] - c0[1]) * t,
                                c0[2] + (c1[2] - c0[2]) * t,
                            ];
                            front_colors.push(c_interp);
                            back_colors.push(c_interp);
                        }
                    }
                }

                let front_poly = Polygon {
                    vertices: front_verts,
                    plane: self.plane,
                    texture: self.texture.clone(),
                    flags: self.flags,
                    base: self.base,
                    texture_u: self.texture_u,
                    texture_v: self.texture_v,
                    pan_u: self.pan_u,
                    pan_v: self.pan_v,
                    vertex_colors: front_colors,
                    lightmap: self.lightmap.clone(),
                };

                let back_poly = Polygon {
                    vertices: back_verts,
                    plane: self.plane,
                    texture: self.texture.clone(),
                    flags: self.flags,
                    base: self.base,
                    texture_u: self.texture_u,
                    texture_v: self.texture_v,
                    pan_u: self.pan_u,
                    pan_v: self.pan_v,
                    vertex_colors: back_colors,
                    lightmap: self.lightmap.clone(),
                };

                SplitResult::Spanning {
                    front: front_poly,
                    back: back_poly,
                }
            }
        }
    }

    /// Returns true if all vertices of this polygon lie within `epsilon` of its plane.
    pub fn is_planar(&self, epsilon: f32) -> bool {
        if self.vertices.len() <= 3 {
            return true;
        }
        self.vertices
            .iter()
            .all(|&v| self.plane.dist_to_point(v).abs() <= epsilon)
    }

    /// If this polygon has >= 4 vertices and deviates from planarity by more than `epsilon`,
    /// splits it into a fan of planar triangles preserving texture UVs and flags.
    pub fn triangulate_if_non_planar(&self, epsilon: f32) -> Vec<Polygon> {
        if self.is_planar(epsilon) || self.vertices.len() <= 3 {
            return vec![self.clone()];
        }
        let mut tris = Vec::with_capacity(self.vertices.len() - 2);
        let v0 = self.vertices[0];
        for i in 1..(self.vertices.len() - 1) {
            let v1 = self.vertices[i];
            let v2 = self.vertices[i + 1];
            if let Some(plane) = Plane::from_points(v0, v1, v2) {
                let tri_colors = if self.vertex_colors.len() == self.vertices.len() {
                    vec![
                        self.vertex_colors[0],
                        self.vertex_colors[i],
                        self.vertex_colors[i + 1],
                    ]
                } else {
                    Vec::new()
                };
                tris.push(Polygon {
                    vertices: vec![v0, v1, v2],
                    plane,
                    texture: self.texture.clone(),
                    flags: self.flags,
                    base: self.base,
                    texture_u: self.texture_u,
                    texture_v: self.texture_v,
                    pan_u: self.pan_u,
                    pan_v: self.pan_v,
                    vertex_colors: tri_colors,
                    lightmap: None,
                });
            }
        }
        if tris.is_empty() {
            vec![self.clone()]
        } else {
            tris
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonClassification {
    Front,
    Back,
    CoplanarFront,
    CoplanarBack,
    Spanning,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SplitResult {
    Front(Polygon),
    Back(Polygon),
    Spanning { front: Polygon, back: Polygon },
}
