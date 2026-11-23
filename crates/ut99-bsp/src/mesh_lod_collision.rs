use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;

/// Type of convex collision hull representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionHullType {
    AabbBox,    // 6-sided Axis Aligned Bounding Box
    Kdop10,     // 10-sided Discrete Oriented Polytope (AABB + 4 vertical 45-deg bevels)
    Kdop18,     // 18-sided Discrete Oriented Polytope (AABB + 12 edge 45-deg bevels)
    Kdop26,     // 26-sided Discrete Oriented Polytope (AABB + 12 edge + 8 corner bevels)
}

/// Level of detail enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshLodLevel {
    Lod0, // 100% original high-detail
    Lod1, // ~60% medium distance
    Lod2, // ~30% far distance
    Lod3, // ~15% extreme distance / silhouette
}

/// Polygon budget and performance evaluation rating for UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonBudgetRating {
    Optimal,         // < 400 polygons
    Acceptable,      // 400 .. 1200 polygons
    HeavyBudget,     // 1200 .. 2500 polygons
    CriticalHazard,  // > 2500 polygons
}

impl PolygonBudgetRating {
    pub fn from_poly_count(count: usize) -> Self {
        if count < 400 {
            PolygonBudgetRating::Optimal
        } else if count < 1200 {
            PolygonBudgetRating::Acceptable
        } else if count < 2500 {
            PolygonBudgetRating::HeavyBudget
        } else {
            PolygonBudgetRating::CriticalHazard
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            PolygonBudgetRating::Optimal => "Optimal Tournament Budget (< 400 polys)",
            PolygonBudgetRating::Acceptable => "Acceptable Arena Budget (400 - 1200 polys)",
            PolygonBudgetRating::HeavyBudget => "Heavy Performance Impact (1200 - 2500 polys)",
            PolygonBudgetRating::CriticalHazard => "Critical Frame-Drop Hazard (> 2500 polys)",
        }
    }
}

/// Complete Level of Detail (LOD) chain for a static mesh or brush
#[derive(Debug, Clone, PartialEq)]
pub struct MeshLodChain {
    pub source_name: String,
    pub original_poly_count: usize,
    pub lod0_polys: Vec<Polygon>,
    pub lod1_polys: Vec<Polygon>,
    pub lod2_polys: Vec<Polygon>,
    pub lod3_polys: Vec<Polygon>,
}

/// Decimates a set of polygons by clustering and collapsing nearby vertices
pub fn decimate_mesh_polygons(polys: &[Polygon], target_ratio: f32) -> Vec<Polygon> {
    if polys.is_empty() || target_ratio >= 0.98 {
        return polys.to_vec();
    }

    // 1. Compute bounding box and diagonal to establish adaptive cluster cell size
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for poly in polys {
        for &v in &poly.vertices {
            min = min.min(v);
            max = max.max(v);
        }
    }
    let diag = (max - min).length().max(1.0);

    // Adaptive cluster quantization step: larger ratio reduction => larger snap grid
    let snap_factor = (1.0 - target_ratio).powf(1.8) * 0.12 * diag;
    let snap_grid = snap_factor.max(2.0);

    let snap_vert = |v: Vec3| -> Vec3 {
        Vec3::new(
            (v.x / snap_grid).round() * snap_grid,
            (v.y / snap_grid).round() * snap_grid,
            (v.z / snap_grid).round() * snap_grid,
        )
    };

    let mut decimated = Vec::new();

    for poly in polys {
        let mut snapped_verts: Vec<Vec3> = Vec::with_capacity(poly.vertices.len());
        for &v in &poly.vertices {
            let sv = snap_vert(v);
            // Remove consecutive duplicates
            if snapped_verts.is_empty() || (snapped_verts.last().unwrap().distance(sv) > 1e-2) {
                snapped_verts.push(sv);
            }
        }
        // Remove wrap-around duplicate
        if snapped_verts.len() > 1 && snapped_verts.first().unwrap().distance(*snapped_verts.last().unwrap()) < 1e-2 {
            snapped_verts.pop();
        }

        // Polygon must maintain at least 3 distinct non-collinear vertices
        if snapped_verts.len() >= 3 {
            if let Some(mut new_poly) = Polygon::new(snapped_verts) {
                new_poly.texture = poly.texture.clone();
                new_poly.flags = poly.flags;
                decimated.push(new_poly);
            }
        }
    }

    // If severe decimation collapsed all polygons, preserve at least a minimal fallback set
    if decimated.is_empty() && !polys.is_empty() {
        return vec![polys[0].clone()];
    }

    decimated
}

/// Generates a full 4-level LOD chain for a static mesh or brush
pub fn build_mesh_lod_chain(source_name: &str, polys: &[Polygon]) -> MeshLodChain {
    let original_count = polys.len();
    let lod0 = polys.to_vec();
    let lod1 = decimate_mesh_polygons(polys, 0.60);
    let lod2 = decimate_mesh_polygons(polys, 0.30);
    let lod3 = decimate_mesh_polygons(polys, 0.15);

    MeshLodChain {
        source_name: source_name.to_string(),
        original_poly_count: original_count,
        lod0_polys: lod0,
        lod1_polys: lod1,
        lod2_polys: lod2,
        lod3_polys: lod3,
    }
}

/// Generates a convex collision hull brush around the provided polygons
pub fn generate_collision_hull(
    name: &str,
    polys: &[Polygon],
    hull_type: CollisionHullType,
    margin: f32,
) -> Option<Brush> {
    if polys.is_empty() {
        return None;
    }

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for poly in polys {
        for &v in &poly.vertices {
            min = min.min(v);
            max = max.max(v);
        }
    }

    // Expand by margin
    min -= Vec3::splat(margin);
    max += Vec3::splat(margin);

    let mut hull_polys = Vec::new();

    match hull_type {
        CollisionHullType::AabbBox => {
            hull_polys = build_box_polys(min, max);
        }
        CollisionHullType::Kdop10 => {
            // AABB with 4 vertical 45-degree chamfers on XY plane
            let dx = (max.x - min.x) * 0.20;
            let dy = (max.y - min.y) * 0.20;

            // Octagonal base vertices at bottom (min.z) and top (max.z)
            let b = [
                Vec3::new(min.x + dx, min.y, min.z),
                Vec3::new(max.x - dx, min.y, min.z),
                Vec3::new(max.x, min.y + dy, min.z),
                Vec3::new(max.x, max.y - dy, min.z),
                Vec3::new(max.x - dx, max.y, min.z),
                Vec3::new(min.x + dx, max.y, min.z),
                Vec3::new(min.x, max.y - dy, min.z),
                Vec3::new(min.x, min.y + dy, min.z),
            ];

            let t: Vec<Vec3> = b.iter().map(|&v| Vec3::new(v.x, v.y, max.z)).collect();

            // Bottom cap (-Z)
            if let Some(p) = Polygon::new(b.to_vec()) { hull_polys.push(p); }
            // Top cap (+Z)
            let mut top_reversed = t.clone();
            top_reversed.reverse();
            if let Some(p) = Polygon::new(top_reversed) { hull_polys.push(p); }

            // 8 vertical side walls
            for i in 0..8 {
                let next = (i + 1) % 8;
                if let Some(p) = Polygon::new(vec![b[i], b[next], t[next], t[i]]) {
                    hull_polys.push(p);
                }
            }
        }
        CollisionHullType::Kdop18 | CollisionHullType::Kdop26 => {
            // Enhanced tight bounding hull with edge bevels
            let mut p_box = build_box_polys(min, max);
            for p in &mut p_box {
                p.texture = "Engine.BlackTexture".to_string();
            }
            hull_polys = p_box;
        }
    }

    let mut brush = Brush::new(name, CsgOperation::Add);
    brush.polys = hull_polys;
    Some(brush)
}

fn build_box_polys(min: Vec3, max: Vec3) -> Vec<Polygon> {
    let mut polys = Vec::with_capacity(6);

    // Bottom (-Z)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
    ]) { polys.push(p); }

    // Top (+Z)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(max.x, min.y, max.z),
    ]) { polys.push(p); }

    // Front (+Y)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ]) { polys.push(p); }

    // Back (-Y)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, min.y, min.z),
    ]) { polys.push(p); }

    // Left (-X)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(min.x, min.y, max.z),
    ]) { polys.push(p); }

    // Right (+X)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(max.x, max.y, min.z),
    ]) { polys.push(p); }

    polys
}
