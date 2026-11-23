use glam::Vec3;
use crate::brush::Brush;
use crate::poly::Polygon;

/// Symmetrical reflection mode for CTF arenas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CtfSymmetryMode {
    #[default]
    Rotational180, // 180-degree rotational symmetry around center pivot (Z axis)
    MirrorX,       // Planar mirror symmetry across X = center.x
    MirrorY,       // Planar mirror symmetry across Y = center.y
}

/// Configuration for symmetrical base duplication
#[derive(Debug, Clone, PartialEq)]
pub struct CtfSymmetryConfig {
    pub mode: CtfSymmetryMode,
    pub center: Vec3,
    pub remap_textures: bool,
    pub prefix_filter: Option<String>,
}

impl Default for CtfSymmetryConfig {
    fn default() -> Self {
        Self {
            mode: CtfSymmetryMode::Rotational180,
            center: Vec3::ZERO,
            remap_textures: true,
            prefix_filter: None,
        }
    }
}

/// Statistics and output brushes from symmetrical CTF generation
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CtfSymmetryResult {
    pub generated_brushes: Vec<Brush>,
    pub transformed_brush_count: usize,
    pub remapped_texture_polys_count: usize,
    pub inverted_winding_polys_count: usize,
}

/// Replaces Red team identifiers with Blue team identifiers and vice versa
pub fn remap_ctf_name(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    let lower = name.to_lowercase();
    if lower.contains("red") {
        name.replace("RED", "BLUE")
            .replace("Red", "Blue")
            .replace("red", "blue")
    } else if lower.contains("blue") {
        name.replace("BLUE", "RED")
            .replace("Blue", "Red")
            .replace("blue", "red")
    } else {
        format!("{}_Symm", name)
    }
}

/// Transforms a world-space point according to symmetry mode and pivot center
pub fn transform_point_symmetry(p: Vec3, mode: CtfSymmetryMode, center: Vec3) -> Vec3 {
    match mode {
        CtfSymmetryMode::Rotational180 => {
            let dx = p.x - center.x;
            let dy = p.y - center.y;
            Vec3::new(center.x - dx, center.y - dy, p.z)
        }
        CtfSymmetryMode::MirrorX => {
            let dx = p.x - center.x;
            Vec3::new(center.x - dx, p.y, p.z)
        }
        CtfSymmetryMode::MirrorY => {
            let dy = p.y - center.y;
            Vec3::new(p.x, center.y - dy, p.z)
        }
    }
}

/// Transforms a directional vector according to symmetry mode
pub fn transform_vector_symmetry(v: Vec3, mode: CtfSymmetryMode) -> Vec3 {
    match mode {
        CtfSymmetryMode::Rotational180 => Vec3::new(-v.x, -v.y, v.z),
        CtfSymmetryMode::MirrorX => Vec3::new(-v.x, v.y, v.z),
        CtfSymmetryMode::MirrorY => Vec3::new(v.x, -v.y, v.z),
    }
}

/// Duplicates and transforms a single brush across the symmetry axis
pub fn mirror_or_rotate_brush(
    brush: &Brush,
    mode: CtfSymmetryMode,
    center: Vec3,
    remap_textures: bool,
) -> (Brush, usize, usize) {
    let new_name = remap_ctf_name(&brush.name);
    let new_location = transform_point_symmetry(brush.location, mode, center);

    let mut new_brush = Brush::new(new_name, brush.csg_oper);
    new_brush.location = new_location;
    new_brush.rotation = Vec3::ZERO;
    new_brush.scale = Vec3::ONE;

    let mut remapped_tex_count = 0;
    let mut inverted_winding_count = 0;

    let world_polys = brush.to_world_polys();
    for poly in world_polys {
        let mut transformed_verts: Vec<Vec3> = poly
            .vertices
            .iter()
            .map(|&v| {
                let world_v = transform_point_symmetry(v, mode, center);
                world_v - new_location
            })
            .collect();

        // For planar mirror reflections (determinant = -1), invert vertex winding
        let should_reverse = matches!(mode, CtfSymmetryMode::MirrorX | CtfSymmetryMode::MirrorY);
        if should_reverse {
            transformed_verts.reverse();
            inverted_winding_count += 1;
        }

        if let Some(mut new_poly) = Polygon::new(transformed_verts) {
            new_poly.flags = poly.flags;

            let world_base = transform_point_symmetry(poly.base, mode, center);
            new_poly.base = world_base - new_location;
            new_poly.texture_u = transform_vector_symmetry(poly.texture_u, mode);
            new_poly.texture_v = transform_vector_symmetry(poly.texture_v, mode);

            if remap_textures && !poly.texture.is_empty() {
                let remapped = remap_ctf_name(&poly.texture);
                if remapped != poly.texture {
                    remapped_tex_count += 1;
                }
                new_poly.texture = remapped;
            } else {
                new_poly.texture = poly.texture.clone();
            }

            new_brush.polys.push(new_poly);
        }
    }

    (new_brush, remapped_tex_count, inverted_winding_count)
}

/// Generates a complete symmetrical opposite base for CTF maps
pub fn generate_symmetrical_ctf_base(
    source_brushes: &[Brush],
    config: &CtfSymmetryConfig,
) -> CtfSymmetryResult {
    let mut result = CtfSymmetryResult::default();

    for brush in source_brushes {
        // Filter by prefix if requested
        if let Some(ref prefix) = config.prefix_filter {
            if !brush.name.starts_with(prefix) {
                continue;
            }
        }

        let (new_brush, remapped_tex, inverted_wind) = mirror_or_rotate_brush(
            brush,
            config.mode,
            config.center,
            config.remap_textures,
        );

        result.generated_brushes.push(new_brush);
        result.transformed_brush_count += 1;
        result.remapped_texture_polys_count += remapped_tex;
        result.inverted_winding_polys_count += inverted_wind;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::build_cube;
    use crate::CsgOperation;

    #[test]
    fn test_ctf_rotational_symmetry_180() {
        let mut red_wall = build_cube("Red_BaseWall", CsgOperation::Add, Vec3::new(64.0, 256.0, 128.0));
        red_wall.location = Vec3::new(500.0, 300.0, 0.0);
        for p in &mut red_wall.polys {
            p.texture = "cf_redbrick".to_string();
        }

        let config = CtfSymmetryConfig {
            mode: CtfSymmetryMode::Rotational180,
            center: Vec3::ZERO,
            remap_textures: true,
            prefix_filter: None,
        };

        let result = generate_symmetrical_ctf_base(&[red_wall], &config);
        assert_eq!(result.transformed_brush_count, 1);
        let blue_wall = &result.generated_brushes[0];

        assert_eq!(blue_wall.name, "Blue_BaseWall");
        // Rotated 180 around origin: (500, 300, 0) -> (-500, -300, 0)
        assert!((blue_wall.location - Vec3::new(-500.0, -300.0, 0.0)).length() < 1e-4);
        assert_eq!(result.inverted_winding_polys_count, 0); // No winding inversion for 180-deg rotation
        assert!(result.remapped_texture_polys_count > 0);
        assert_eq!(blue_wall.polys[0].texture, "cf_bluebrick");
    }

    #[test]
    fn test_ctf_mirror_symmetry_x() {
        let mut red_pillar = build_cube("Red_Pillar", CsgOperation::Add, Vec3::new(32.0, 32.0, 256.0));
        red_pillar.location = Vec3::new(200.0, 100.0, 50.0);

        let config = CtfSymmetryConfig {
            mode: CtfSymmetryMode::MirrorX,
            center: Vec3::ZERO,
            remap_textures: true,
            prefix_filter: None,
        };

        let result = generate_symmetrical_ctf_base(&[red_pillar], &config);
        assert_eq!(result.transformed_brush_count, 1);
        let blue_pillar = &result.generated_brushes[0];

        assert_eq!(blue_pillar.name, "Blue_Pillar");
        // Mirrored across X=0: (200, 100, 50) -> (-200, 100, 50)
        assert!((blue_pillar.location - Vec3::new(-200.0, 100.0, 50.0)).length() < 1e-4);
        // All 6 faces inverted their winding order to maintain outward normals
        assert_eq!(result.inverted_winding_polys_count, 6);
        assert_eq!(blue_pillar.polys.len(), 6);
    }
}
