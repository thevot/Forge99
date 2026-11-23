// crates/ut99-bsp/src/bsp_quality.rs
//
// Analytical BSP Tree Quality Profiler, Sliver Polygon Purger,
// Coplanar Z-Fighting Detector & Cut Minimization Analyzer for Unreal Tournament 99.

use super::bsp::BspNode;
use super::brush::Brush;
use super::math::Plane;
use super::CsgOperation;
use glam::Vec3;

/// Core quality diagnostics for a compiled BSP tree
#[derive(Debug, Clone, PartialEq)]
pub struct BspQualityMetrics {
    pub total_input_brush_polys: usize,
    pub total_world_bsp_polys: usize,
    pub cut_ratio: f32, // world_polys / input_polys
    pub max_tree_depth: usize,
    pub average_tree_depth: f32,
    pub depth_histogram: [usize; 64],
    pub tree_balance_pct: f32, // 100% = perfectly balanced binary tree
    pub total_nodes: usize,
    pub total_leaves: usize,
}

impl Default for BspQualityMetrics {
    fn default() -> Self {
        Self {
            total_input_brush_polys: 0,
            total_world_bsp_polys: 0,
            cut_ratio: 1.0,
            max_tree_depth: 0,
            average_tree_depth: 0.0,
            depth_histogram: [0; 64],
            tree_balance_pct: 100.0,
            total_nodes: 0,
            total_leaves: 0,
        }
    }
}

/// A micro-triangle or degenerate sliver polygon produced by aggressive CSG splits
#[derive(Debug, Clone, PartialEq)]
pub struct SliverPolygon {
    pub brush_index: usize,
    pub poly_index: usize,
    pub area: f32,
    pub perimeter: f32,
    pub aspect_ratio: f32, // longest_edge / shortest_altitude
    pub centroid: Vec3,
    pub vertices: Vec<Vec3>,
}

/// Two brush faces sharing the exact same geometric plane, risking z-fighting and redundant splits
#[derive(Debug, Clone, PartialEq)]
pub struct CoplanarOverlap {
    pub brush_a_index: usize,
    pub brush_b_index: usize,
    pub poly_a_index: usize,
    pub poly_b_index: usize,
    pub plane: Plane,
    pub approximate_center: Vec3,
}

/// Analysis of which brushes cause the highest amount of planar world BSP cuts
#[derive(Debug, Clone, PartialEq)]
pub struct BrushCutOffender {
    pub brush_index: usize,
    pub brush_name: String,
    pub csg_oper: CsgOperation,
    pub poly_count: usize,
    pub estimated_cuts_induced: usize,
    pub can_be_semisolid: bool,
}

/// Analyzes an existing BSP node hierarchy and computes tree depth, balance, and node counts
pub fn analyze_bsp_tree_quality(root: Option<&BspNode>, input_brushes: &[Brush]) -> BspQualityMetrics {
    let mut metrics = BspQualityMetrics::default();

    let mut total_brush_polys = 0;
    for b in input_brushes {
        total_brush_polys += b.polys.len();
    }
    metrics.total_input_brush_polys = total_brush_polys;

    if let Some(node) = root {
        let mut leaf_depths = Vec::new();
        traverse_node_depths(node, 0, &mut metrics, &mut leaf_depths);

        if !leaf_depths.is_empty() {
            let sum_depth: usize = leaf_depths.iter().sum();
            metrics.average_tree_depth = sum_depth as f32 / leaf_depths.len() as f32;
            metrics.total_leaves = leaf_depths.len();

            // Optimal depth for N leaves in a balanced binary tree is log2(N)
            let optimal_depth = (leaf_depths.len() as f32).log2().max(1.0);
            let balance = (optimal_depth / metrics.average_tree_depth.max(1.0)).min(1.0) * 100.0;
            metrics.tree_balance_pct = balance;
        }
    } else {
        // Fallback: estimate from input brushes if BSP has not yet been built
        metrics.total_world_bsp_polys = total_brush_polys;
        metrics.cut_ratio = 1.0;
    }

    if metrics.total_input_brush_polys > 0 {
        metrics.cut_ratio = (metrics.total_world_bsp_polys as f32 / metrics.total_input_brush_polys as f32).max(1.0);
    }

    metrics
}

fn traverse_node_depths(
    node: &BspNode,
    current_depth: usize,
    metrics: &mut BspQualityMetrics,
    leaf_depths: &mut Vec<usize>,
) {
    metrics.total_nodes += 1;
    metrics.total_world_bsp_polys += node.coplanar.len();

    let clamped_depth = current_depth.min(63);
    metrics.depth_histogram[clamped_depth] += 1;
    metrics.max_tree_depth = metrics.max_tree_depth.max(current_depth);

    let is_leaf = node.front.is_none() && node.back.is_none();
    if is_leaf {
        leaf_depths.push(current_depth);
    }

    if let Some(ref front) = node.front {
        traverse_node_depths(front, current_depth + 1, metrics, leaf_depths);
    }
    if let Some(ref back) = node.back {
        traverse_node_depths(back, current_depth + 1, metrics, leaf_depths);
    }
}

/// Detects sliver triangles (micro-polygons with area < min_area or extreme aspect ratio)
pub fn detect_sliver_polygons(
    brushes: &[Brush],
    min_area: f32,
    max_aspect_ratio: f32,
) -> Vec<SliverPolygon> {
    let mut slivers = Vec::new();

    for (b_idx, brush) in brushes.iter().enumerate() {
        for (p_idx, poly) in brush.polys.iter().enumerate() {
            if poly.vertices.len() < 3 {
                continue;
            }

            let area = calculate_poly_area(&poly.vertices);
            let perimeter = calculate_poly_perimeter(&poly.vertices);

            // Compute edge lengths
            let mut max_edge = 0.0_f32;
            for i in 0..poly.vertices.len() {
                let v0 = poly.vertices[i];
                let v1 = poly.vertices[(i + 1) % poly.vertices.len()];
                let len = (v1 - v0).length();
                if len > max_edge {
                    max_edge = len;
                }
            }

            // Approximate altitude = (2 * area) / max_edge
            let altitude = if max_edge > 0.001 { (2.0 * area) / max_edge } else { 0.0 };
            let aspect_ratio = if altitude > 0.001 { max_edge / altitude } else { 999.0 };

            if area < min_area || aspect_ratio > max_aspect_ratio {
                let mut sum = Vec3::ZERO;
                for v in &poly.vertices {
                    sum += *v;
                }
                let centroid = sum / poly.vertices.len() as f32;

                slivers.push(SliverPolygon {
                    brush_index: b_idx,
                    poly_index: p_idx,
                    area,
                    perimeter,
                    aspect_ratio,
                    centroid: centroid + brush.location,
                    vertices: poly.vertices.clone(),
                });
            }
        }
    }

    slivers
}

/// Detects coplanar overlapping faces across different brushes that risk z-buffer fighting
pub fn detect_coplanar_overlaps(brushes: &[Brush], dist_tolerance: f32) -> Vec<CoplanarOverlap> {
    let mut overlaps = Vec::new();

    for i in 0..brushes.len() {
        for j in (i + 1)..brushes.len() {
            let brush_a = &brushes[i];
            let brush_b = &brushes[j];

            for (p_a_idx, poly_a) in brush_a.polys.iter().enumerate() {
                let world_plane_a = Plane::from_point_and_normal(
                    poly_a.vertices.first().copied().unwrap_or(Vec3::ZERO) + brush_a.location,
                    poly_a.plane.normal,
                );

                for (p_b_idx, poly_b) in brush_b.polys.iter().enumerate() {
                    let world_plane_b = Plane::from_point_and_normal(
                        poly_b.vertices.first().copied().unwrap_or(Vec3::ZERO) + brush_b.location,
                        poly_b.plane.normal,
                    );

                    // Must have identical or opposite normal (within 0.01)
                    let dot = world_plane_a.normal.dot(world_plane_b.normal);
                    if dot.abs() > 0.99 {
                        // Check distance of brush_b point to plane_a
                        let pt_b = poly_b.vertices.first().copied().unwrap_or(Vec3::ZERO) + brush_b.location;
                        let dist = world_plane_a.dist_to_point(pt_b).abs();

                        if dist <= dist_tolerance {
                            // Bounding box overlap test
                            let center_a = poly_center(&poly_a.vertices) + brush_a.location;
                            let center_b = poly_center(&poly_b.vertices) + brush_b.location;
                            let dist_centers = (center_a - center_b).length();

                            if dist_centers <= 512.0 {
                                overlaps.push(CoplanarOverlap {
                                    brush_a_index: i,
                                    brush_b_index: j,
                                    poly_a_index: p_a_idx,
                                    poly_b_index: p_b_idx,
                                    plane: world_plane_a,
                                    approximate_center: (center_a + center_b) * 0.5,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    overlaps
}

/// Evaluates each brush's contribution to planar world cuts and identifies candidates for SemiSolid conversion
pub fn analyze_brush_cut_offenders(brushes: &[Brush]) -> Vec<BrushCutOffender> {
    let mut offenders = Vec::new();

    for (idx, brush) in brushes.iter().enumerate() {
        // High polygon count or non-axial rotated brushes induce significant BSP splits
        let is_axial = is_brush_axis_aligned(brush);
        let poly_count = brush.polys.len();

        // Estimated cuts induced across neighboring geometry
        let base_cuts = poly_count * if is_axial { 2 } else { 5 };
        let can_be_semisolid = brush.csg_oper == CsgOperation::Add && poly_count >= 8;

        offenders.push(BrushCutOffender {
            brush_index: idx,
            brush_name: brush.name.clone(),
            csg_oper: brush.csg_oper,
            poly_count,
            estimated_cuts_induced: base_cuts,
            can_be_semisolid,
        });
    }

    // Sort descending by cuts induced
    offenders.sort_by(|a, b| b.estimated_cuts_induced.cmp(&a.estimated_cuts_induced));
    offenders
}

/// Purges degenerate sliver polygons from a brush by removing polygons whose area is below min_area
pub fn purge_sliver_triangles(brush: &mut Brush, min_area: f32) -> usize {
    let initial_count = brush.polys.len();
    brush.polys.retain(|poly| {
        if poly.vertices.len() < 3 {
            return false;
        }
        let area = calculate_poly_area(&poly.vertices);
        area >= min_area
    });
    initial_count - brush.polys.len()
}

fn calculate_poly_area(vertices: &[Vec3]) -> f32 {
    if vertices.len() < 3 {
        return 0.0;
    }
    let mut total = Vec3::ZERO;
    let v0 = vertices[0];
    for i in 1..vertices.len() - 1 {
        let v1 = vertices[i];
        let v2 = vertices[i + 1];
        total += (v1 - v0).cross(v2 - v0);
    }
    total.length() * 0.5
}

fn calculate_poly_perimeter(vertices: &[Vec3]) -> f32 {
    if vertices.len() < 2 {
        return 0.0;
    }
    let mut perim = 0.0;
    for i in 0..vertices.len() {
        let v0 = vertices[i];
        let v1 = vertices[(i + 1) % vertices.len()];
        perim += (v1 - v0).length();
    }
    perim
}

fn poly_center(vertices: &[Vec3]) -> Vec3 {
    if vertices.is_empty() {
        return Vec3::ZERO;
    }
    let mut sum = Vec3::ZERO;
    for v in vertices {
        sum += *v;
    }
    sum / vertices.len() as f32
}

fn is_brush_axis_aligned(brush: &Brush) -> bool {
    let r = brush.rotation;
    let is_zero_rot = (r.x % 90.0).abs() < 0.01 && (r.y % 90.0).abs() < 0.01 && (r.z % 90.0).abs() < 0.01;
    is_zero_rot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::Polygon;

    #[test]
    fn test_sliver_polygon_detection_and_purging() {
        let mut brush = Brush::new("TestBrush", CsgOperation::Add);

        // 1. Normal healthy quad (area = 256 * 256 = 65536)
        let normal_poly = Polygon::new(vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(256.0, 0.0, 0.0),
            Vec3::new(256.0, 256.0, 0.0),
            Vec3::new(0.0, 256.0, 0.0),
        ]).unwrap();
        brush.polys.push(normal_poly);

        // 2. Micro sliver triangle (area < 0.5)
        let sliver_poly = Polygon::new(vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(50.0, 0.005, 0.0),
        ]).unwrap();
        brush.polys.push(sliver_poly);

        assert_eq!(brush.polys.len(), 2);

        let slivers = detect_sliver_polygons(&[brush.clone()], 2.0, 40.0);
        assert_eq!(slivers.len(), 1);
        assert!(slivers[0].area < 1.0);

        // Purge sliver
        let purged = purge_sliver_triangles(&mut brush, 2.0);
        assert_eq!(purged, 1);
        assert_eq!(brush.polys.len(), 1);
    }

    #[test]
    fn test_coplanar_overlap_detection() {
        let mut brush_a = Brush::new("FloorA", CsgOperation::Add);
        brush_a.location = Vec3::new(0.0, 0.0, 0.0);
        brush_a.polys.push(Polygon::new(vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(256.0, 0.0, 0.0),
            Vec3::new(256.0, 256.0, 0.0),
            Vec3::new(0.0, 256.0, 0.0),
        ]).unwrap());

        let mut brush_b = Brush::new("FloorB", CsgOperation::Add);
        brush_b.location = Vec3::new(128.0, 128.0, 0.0); // Exact same Z plane
        brush_b.polys.push(Polygon::new(vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(256.0, 0.0, 0.0),
            Vec3::new(256.0, 256.0, 0.0),
            Vec3::new(0.0, 256.0, 0.0),
        ]).unwrap());

        let overlaps = detect_coplanar_overlaps(&[brush_a, brush_b], 0.1);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].brush_a_index, 0);
        assert_eq!(overlaps[0].brush_b_index, 1);
    }

    #[test]
    fn test_bsp_quality_metrics_evaluation() {
        let mut node = BspNode {
            plane: Plane { normal: Vec3::Z, dist: 0.0 },
            coplanar: vec![
                Polygon::new(vec![Vec3::ZERO, Vec3::X * 100.0, Vec3::Y * 100.0]).unwrap(),
            ],
            front: None,
            back: None,
        };

        // Add back child
        node.back = Some(Box::new(BspNode {
            plane: Plane { normal: Vec3::X, dist: 50.0 },
            coplanar: vec![
                Polygon::new(vec![Vec3::ZERO, Vec3::Y * 50.0, Vec3::Z * 50.0]).unwrap(),
            ],
            front: None,
            back: None,
        }));

        let metrics = analyze_bsp_tree_quality(Some(&node), &[]);
        assert_eq!(metrics.total_nodes, 2);
        assert_eq!(metrics.total_world_bsp_polys, 2);
        assert!(metrics.max_tree_depth >= 1);
    }

    #[test]
    fn test_brush_cut_offenders_ranking() {
        let mut b1 = Brush::new("SimpleCube", CsgOperation::Add);
        b1.polys = vec![Polygon::new(vec![Vec3::ZERO, Vec3::X, Vec3::Y]).unwrap(); 6];

        let mut b2 = Brush::new("ComplexPillar", CsgOperation::Add);
        b2.polys = vec![Polygon::new(vec![Vec3::ZERO, Vec3::X, Vec3::Y]).unwrap(); 24];
        b2.rotation = Vec3::new(15.0, 30.0, 0.0); // Non-axial!

        let offenders = analyze_brush_cut_offenders(&[b1, b2]);
        assert_eq!(offenders.len(), 2);
        // Complex non-axial pillar must be ranked #1 offender
        assert_eq!(offenders[0].brush_name, "ComplexPillar");
        assert!(offenders[0].can_be_semisolid);
    }
}
