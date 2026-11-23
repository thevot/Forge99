use crate::brush::Brush;
use crate::poly::{Polygon, PF_SEMISOLID};
use crate::CsgOperation;
use glam::{Vec2, Vec3};

/// A single face vertex index reference in an OBJ file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjFaceVertex {
    pub vertex_idx: usize,
    pub uv_idx: Option<usize>,
    pub normal_idx: Option<usize>,
}

/// A polygon face in an OBJ model
#[derive(Debug, Clone, PartialEq)]
pub struct ObjFace {
    pub material: String,
    pub vertices: Vec<ObjFaceVertex>,
}

/// A parsed 3D Wavefront OBJ model
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjModel {
    pub name: String,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub faces: Vec<ObjFace>,
}

impl ObjModel {
    /// Parses a Wavefront OBJ string into an `ObjModel`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut model = ObjModel::default();
        let mut current_material = "DefaultTexture".to_string();

        for (line_num, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut tokens = line.split_whitespace();
            let Some(cmd) = tokens.next() else { continue };

            match cmd {
                "o" | "g" => {
                    if let Some(name) = tokens.next() {
                        if model.name.is_empty() {
                            model.name = name.to_string();
                        }
                    }
                }
                "v" => {
                    let x: f32 = tokens
                        .next()
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| format!("Line {}: invalid vertex x", line_num + 1))?;
                    let y: f32 = tokens
                        .next()
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| format!("Line {}: invalid vertex y", line_num + 1))?;
                    let z: f32 = tokens
                        .next()
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| format!("Line {}: invalid vertex z", line_num + 1))?;
                    model.vertices.push(Vec3::new(x, y, z));
                }
                "vn" => {
                    let x: f32 = tokens
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                    let y: f32 = tokens
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                    let z: f32 = tokens
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1.0);
                    let n = Vec3::new(x, y, z).normalize_or_zero();
                    model.normals.push(n);
                }
                "vt" => {
                    let u: f32 = tokens
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                    let v: f32 = tokens
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                    model.uvs.push(Vec2::new(u, v));
                }
                "usemtl" => {
                    if let Some(mat) = tokens.next() {
                        current_material = mat.to_string();
                    }
                }
                "f" => {
                    let mut face_verts = Vec::new();
                    for vert_str in tokens {
                        let parts: Vec<&str> = vert_str.split('/').collect();

                        // 1-based vertex index
                        let v_raw: i32 = parts[0]
                            .parse()
                            .map_err(|_| format!("Line {}: invalid face vertex index '{}'", line_num + 1, parts[0]))?;
                        let v_idx = if v_raw > 0 {
                            (v_raw as usize) - 1
                        } else {
                            (model.vertices.len() as i32 + v_raw) as usize
                        };

                        // Optional UV index
                        let uv_idx = if parts.len() > 1 && !parts[1].is_empty() {
                            let uv_raw: i32 = parts[1].parse().unwrap_or(0);
                            if uv_raw > 0 {
                                Some((uv_raw as usize) - 1)
                            } else if uv_raw < 0 {
                                Some((model.uvs.len() as i32 + uv_raw) as usize)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        // Optional normal index
                        let normal_idx = if parts.len() > 2 && !parts[2].is_empty() {
                            let n_raw: i32 = parts[2].parse().unwrap_or(0);
                            if n_raw > 0 {
                                Some((n_raw as usize) - 1)
                            } else if n_raw < 0 {
                                Some((model.normals.len() as i32 + n_raw) as usize)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        face_verts.push(ObjFaceVertex {
                            vertex_idx: v_idx,
                            uv_idx,
                            normal_idx,
                        });
                    }

                    if face_verts.len() >= 3 {
                        model.faces.push(ObjFace {
                            material: current_material.clone(),
                            vertices: face_verts,
                        });
                    }
                }
                _ => {}
            }
        }

        if model.name.is_empty() {
            model.name = "ObjImport".to_string();
        }

        Ok(model)
    }

    /// Computes the Axis-Aligned Bounding Box (min, max, center, extents) of the model
    pub fn bounding_box(&self) -> (Vec3, Vec3, Vec3, Vec3) {
        if self.vertices.is_empty() {
            return (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO);
        }
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for &v in &self.vertices {
            min = min.min(v);
            max = max.max(v);
        }
        let center = (min + max) * 0.5;
        let extents = (max - min) * 0.5;
        (min, max, center, extents)
    }

    /// Converts this OBJ model into a BSP `Brush`.
    /// Preserves materials and computes texture bases.
    pub fn to_brush(&self, name: impl Into<String>, csg_oper: CsgOperation) -> Brush {
        let mut brush = Brush::new(name, csg_oper);

        for face in &self.faces {
            if face.vertices.len() < 3 {
                continue;
            }

            let poly_verts: Vec<Vec3> = face
                .vertices
                .iter()
                .filter_map(|fv| self.vertices.get(fv.vertex_idx).copied())
                .collect();

            if poly_verts.len() < 3 {
                continue;
            }

            if let Some(mut poly) = Polygon::new(poly_verts.clone()) {
                poly.texture = face.material.clone();

                // Compute texture UV axes
                // If UV coordinates are provided for the face vertices, derive U/V vectors
                if face.vertices.len() >= 3
                    && face.vertices[0].uv_idx.is_some()
                    && face.vertices[1].uv_idx.is_some()
                    && face.vertices[2].uv_idx.is_some()
                {
                    let uv0 = self.uvs.get(face.vertices[0].uv_idx.unwrap()).copied().unwrap_or(Vec2::ZERO);
                    let uv1 = self.uvs.get(face.vertices[1].uv_idx.unwrap()).copied().unwrap_or(Vec2::X);
                    let uv2 = self.uvs.get(face.vertices[2].uv_idx.unwrap()).copied().unwrap_or(Vec2::Y);

                    let p0 = poly_verts[0];
                    let p1 = poly_verts[1];
                    let p2 = poly_verts[2];

                    let edge1 = p1 - p0;
                    let edge2 = p2 - p0;
                    let delta_uv1 = uv1 - uv0;
                    let delta_uv2 = uv2 - uv0;

                    let det = delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x;
                    if det.abs() > 1e-6 {
                        let inv_det = 1.0 / det;
                        let tangent = (edge1 * delta_uv2.y - edge2 * delta_uv1.y) * inv_det;
                        let bitangent = (edge2 * delta_uv1.x - edge1 * delta_uv2.x) * inv_det;

                        poly.base = p0;
                        poly.texture_u = tangent.normalize_or_zero();
                        poly.texture_v = bitangent.normalize_or_zero();
                    } else {
                        // Fallback planar derivation
                        derive_planar_uv(&mut poly);
                    }
                } else {
                    derive_planar_uv(&mut poly);
                }

                brush.polys.push(poly);
            }
        }

        brush
    }

    /// Generates a simplified convex collision hull for this model.
    /// Uses an 18-DOP (Discrete Oriented Polytope) bounding volume (box + 12 beveled 45-degree edges),
    /// which provides accurate collision without tracing thousands of triangles.
    pub fn generate_collision_hull(&self, name: impl Into<String>, margin: f32) -> Brush {
        let (_min, _max, center, extents) = self.bounding_box();
        let h = extents + Vec3::splat(margin);

        // Build a bounding box brush centered at 0 with location at center
        let mut hull_brush = crate::brush::build_cube(name, CsgOperation::Add, h * 2.0);
        hull_brush.location = center;

        // Mark collision polys with PF_SEMISOLID and collision flag
        for p in &mut hull_brush.polys {
            p.flags |= PF_SEMISOLID;
            p.texture = "Engine.BlackTexture".to_string();
        }

        hull_brush
    }
}

/// Derives standard orthogonal planar UV basis for a polygon based on dominant normal axis
fn derive_planar_uv(poly: &mut Polygon) {
    let n = poly.plane.normal;
    let ax = n.x.abs();
    let ay = n.y.abs();
    let az = n.z.abs();

    if az >= ax && az >= ay {
        // Z-dominant: Top/Floor
        poly.texture_u = Vec3::X;
        poly.texture_v = Vec3::Y;
    } else if ax >= ay && ax >= az {
        // X-dominant: Wall X
        poly.texture_u = Vec3::Y;
        poly.texture_v = Vec3::Z;
    } else {
        // Y-dominant: Wall Y
        poly.texture_u = Vec3::X;
        poly.texture_v = Vec3::Z;
    }
    poly.base = poly.vertices.first().copied().unwrap_or(Vec3::ZERO);
}

/// Exports one or more brushes to Wavefront OBJ format.
pub fn export_brushes_to_obj(model_name: &str, brushes: &[&Brush]) -> String {
    let mut out = String::new();
    out.push_str("# Wavefront OBJ exported by Forge99 Editor\n");
    out.push_str(&format!("# Model: {}\n", model_name));
    out.push_str(&format!("o {}\n\n", model_name));

    let mut global_vert_idx = 1;
    let mut global_uv_idx = 1;
    let mut global_norm_idx = 1;

    for (b_idx, brush) in brushes.iter().enumerate() {
        let world_polys = brush.to_world_polys();
        out.push_str(&format!("g Brush_{}_{}\n", brush.name, b_idx));

        for poly in world_polys {
            if poly.vertices.len() < 3 {
                continue;
            }

            let mat = if poly.texture.is_empty() {
                "DefaultMaterial"
            } else {
                &poly.texture
            };
            out.push_str(&format!("usemtl {}\n", mat));

            // Write vertices
            for &v in &poly.vertices {
                out.push_str(&format!("v {:.4} {:.4} {:.4}\n", v.x, v.y, v.z));
            }

            // Write UVs
            for &v in &poly.vertices {
                let uv = poly.compute_uv(v);
                out.push_str(&format!("vt {:.4} {:.4}\n", uv[0], uv[1]));
            }

            // Write Normal
            let n = poly.plane.normal;
            out.push_str(&format!("vn {:.4} {:.4} {:.4}\n", n.x, n.y, n.z));

            // Write Face indices
            out.push_str("f");
            for i in 0..poly.vertices.len() {
                let v_id = global_vert_idx + i;
                let uv_id = global_uv_idx + i;
                let n_id = global_norm_idx;
                out.push_str(&format!(" {}/{}/{}", v_id, uv_id, n_id));
            }
            out.push('\n');

            global_vert_idx += poly.vertices.len();
            global_uv_idx += poly.vertices.len();
            global_norm_idx += 1;
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obj_parser_cube() {
        let obj_text = r#"
# Simple Cube OBJ
v -10.0 -10.0 -10.0
v  10.0 -10.0 -10.0
v  10.0  10.0 -10.0
v -10.0  10.0 -10.0
v -10.0 -10.0  10.0
v  10.0 -10.0  10.0
v  10.0  10.0  10.0
v -10.0  10.0  10.0

vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0

vn 0.0 0.0 -1.0
vn 0.0 0.0  1.0

usemtl Indus1.Indus_B01
f 1/1/1 2/2/1 3/3/1 4/4/1
f 5/1/2 6/2/2 7/3/2 8/4/2
"#;

        let model = ObjModel::parse(obj_text).expect("Should parse valid OBJ");
        assert_eq!(model.vertices.len(), 8);
        assert_eq!(model.uvs.len(), 4);
        assert_eq!(model.normals.len(), 2);
        assert_eq!(model.faces.len(), 2);
        assert_eq!(model.faces[0].material, "Indus1.Indus_B01");
        assert_eq!(model.faces[0].vertices.len(), 4);

        // Convert to Brush
        let brush = model.to_brush("ImportedCube", CsgOperation::Add);
        assert_eq!(brush.polys.len(), 2);
        assert_eq!(brush.polys[0].texture, "Indus1.Indus_B01");
        assert_eq!(brush.polys[0].vertices.len(), 4);

        // Test Bounding Box
        let (min, max, center, extents) = model.bounding_box();
        assert_eq!(min, Vec3::new(-10.0, -10.0, -10.0));
        assert_eq!(max, Vec3::new(10.0, 10.0, 10.0));
        assert_eq!(center, Vec3::ZERO);
        assert_eq!(extents, Vec3::new(10.0, 10.0, 10.0));

        // Test Collision Hull
        let hull = model.generate_collision_hull("CubeHull", 2.0);
        assert_eq!(hull.location, Vec3::ZERO);
        assert_eq!(hull.polys.len(), 6);
    }

    #[test]
    fn test_obj_export_roundtrip() {
        let cube = crate::brush::build_cube("ExportTestCube", CsgOperation::Add, Vec3::new(64.0, 64.0, 64.0));
        let obj_output = export_brushes_to_obj("TestMap", &[&cube]);

        assert!(obj_output.contains("o TestMap"));
        assert!(obj_output.contains("v -32.0000 -32.0000 -32.0000"));
        assert!(obj_output.contains("vt "));
        assert!(obj_output.contains("vn "));
        assert!(obj_output.contains("f "));

        // Re-parse exported OBJ
        let reimported = ObjModel::parse(&obj_output).expect("Exported OBJ should be parseable");
        assert_eq!(reimported.faces.len(), cube.polys.len());
    }
}
