use super::bvh::{AabbBvh, BvhEntry};
use super::math::Plane;
use super::poly::{Polygon, SplitResult};
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BspOptimizationLevel {
    Lame,
    Good,
    Optimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BspBuilderConfig {
    pub balance_factor: u32, // 0..100, default: 15 (UnrealEd 2.0 canonical balance vs cuts)
    pub portal_bias: u32,    // 0..100, default: 70
    pub optimization: BspOptimizationLevel,
    pub build_geometry: bool,
    pub build_lighting: bool,
    pub build_paths: bool,
    pub build_decals: bool,
    pub only_visible_brushes: bool,
}

impl Default for BspBuilderConfig {
    fn default() -> Self {
        Self {
            balance_factor: 15,
            portal_bias: 70,
            optimization: BspOptimizationLevel::Good,
            build_geometry: true,
            build_lighting: true,
            build_paths: true,
            build_decals: true,
            only_visible_brushes: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BspBuildReport {
    pub node_count: usize,
    pub poly_count: usize,
    pub coplanar_count: usize,
    pub max_depth: usize,
    pub cut_count: usize,
    pub balance_ratio_pct: f32,
    pub elapsed_ms: u64,
}

impl BspBuildReport {
    /// Returns the classic UnrealEd 2.0 `Nodes : Polys` ratio (`node_count / poly_count`).
    /// Canonical UnrealEd 2.0 target is `<= 2.00` (e.g. `1.50:1` to `2.00:1`).
    pub fn nodes_to_polys_ratio(&self) -> f32 {
        if self.poly_count == 0 {
            0.0
        } else {
            self.node_count as f32 / self.poly_count as f32
        }
    }

    /// Returns the inverse `Polys : Nodes` ratio (`poly_count / node_count`).
    pub fn polys_to_nodes_ratio(&self) -> f32 {
        if self.node_count == 0 {
            0.0
        } else {
            self.poly_count as f32 / self.node_count as f32
        }
    }

    /// Returns true if the `Nodes : Polys` ratio is within the canonical UnrealEd 2.0 optimal threshold (`<= 2.0`).
    pub fn is_optimal_ratio(&self) -> bool {
        self.nodes_to_polys_ratio() <= 2.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BspNode {
    pub plane: Plane,
    pub coplanar: Vec<Polygon>,
    pub front: Option<Box<BspNode>>,
    pub back: Option<Box<BspNode>>,
}

fn pick_best_splitting_plane(polys: &[Polygon], config: &BspBuilderConfig) -> Plane {
    if polys.len() <= 1 {
        return polys[0].plane;
    }

    let alpha = (config.balance_factor as f32 / 100.0).clamp(0.0, 1.0);
    let portal_weight = (config.portal_bias as f32 / 100.0).clamp(0.0, 1.0);
    let sample_limit = match config.optimization {
        BspOptimizationLevel::Lame => 4,
        BspOptimizationLevel::Good => 16,
        BspOptimizationLevel::Optimal => polys.len().min(64),
    };

    let step = (polys.len() / sample_limit).max(1);
    let mut candidate_indices: Vec<usize> = (0..polys.len()).step_by(step).take(sample_limit).collect();
    if portal_weight > 0.0 {
        for (idx, p) in polys.iter().enumerate() {
            if (p.flags & crate::poly::PF_PORTAL) != 0 && !candidate_indices.contains(&idx) {
                candidate_indices.push(idx);
            }
        }
    }

    let mut best_plane = polys[0].plane;
    let mut best_score = f32::MAX;

    for i in candidate_indices {
        let candidate_plane = polys[i].plane;
        let is_portal = (polys[i].flags & crate::poly::PF_PORTAL) != 0;
        let mut front_count = 0;
        let mut back_count = 0;
        let mut cut_count = 0;

        for p in polys {
            match p.split(&candidate_plane) {
                SplitResult::Front(ref f) => {
                    let is_coplanar = (f.plane.normal.dot(candidate_plane.normal) - 1.0).abs() < 0.001
                        && (f.plane.dist - candidate_plane.dist).abs() < 0.001;
                    if !is_coplanar {
                        front_count += 1;
                    }
                }
                SplitResult::Back(_) => back_count += 1,
                SplitResult::Spanning { .. } => cut_count += 1,
            }
        }

        let diff = (front_count as f32 - back_count as f32).abs();
        let mut score = alpha * diff + (1.0 - alpha) * (cut_count as f32) * 8.0;
        if is_portal && portal_weight > 0.0 {
            score = score * (1.0 - 0.85 * portal_weight) - 100.0 * portal_weight;
        }

        if score < best_score {
            best_score = score;
            best_plane = candidate_plane;
        }
    }

    best_plane
}

fn build_bsp_recursive(
    polys: Vec<Polygon>,
    config: &BspBuilderConfig,
    total_cuts: &mut usize,
) -> Option<BspNode> {
    if polys.is_empty() {
        return None;
    }

    let plane = pick_best_splitting_plane(&polys, config);
    let mut coplanar = Vec::new();
    let mut front_list = Vec::new();
    let mut back_list = Vec::new();

    for poly in polys {
        match poly.split(&plane) {
            SplitResult::Front(p) => {
                // Check if it's coplanar with the node's plane
                if (p.plane.normal.dot(plane.normal) - 1.0).abs() < 0.001
                    && (p.plane.dist - plane.dist).abs() < 0.001
                {
                    coplanar.push(p);
                } else {
                    front_list.push(p);
                }
            }
            SplitResult::Back(p) => back_list.push(p),
            SplitResult::Spanning { front, back } => {
                *total_cuts += 1;
                front_list.push(front);
                back_list.push(back);
            }
        }
    }

    let front = if !front_list.is_empty() {
        build_bsp_recursive(front_list, config, total_cuts).map(Box::new)
    } else {
        None
    };

    let back = if !back_list.is_empty() {
        build_bsp_recursive(back_list, config, total_cuts).map(Box::new)
    } else {
        None
    };

    Some(BspNode {
        plane,
        coplanar,
        front,
        back,
    })
}

/// Inserts non-structural detail polygons (`PF_SEMISOLID` / `PF_NOT_SOLID`) into an existing
/// structural BSP tree without splitting any structural polygons already placed in the tree.
fn insert_detail_polys_into_bsp(
    node_opt: &mut Option<BspNode>,
    detail_polys: Vec<Polygon>,
    config: &BspBuilderConfig,
    detail_cuts: &mut usize,
) {
    if detail_polys.is_empty() {
        return;
    }
    let Some(node) = node_opt.as_mut() else {
        *node_opt = build_bsp_recursive(detail_polys, config, detail_cuts);
        return;
    };

    let mut front_detail = Vec::new();
    let mut back_detail = Vec::new();

    for poly in detail_polys {
        match poly.split(&node.plane) {
            SplitResult::Front(p) => {
                if (p.plane.normal.dot(node.plane.normal) - 1.0).abs() < 0.001
                    && (p.plane.dist - node.plane.dist).abs() < 0.001
                {
                    node.coplanar.push(p);
                } else {
                    front_detail.push(p);
                }
            }
            SplitResult::Back(p) => back_detail.push(p),
            SplitResult::Spanning { front, back } => {
                *detail_cuts += 1;
                front_detail.push(front);
                back_detail.push(back);
            }
        }
    }

    if !front_detail.is_empty() {
        let mut front_child = node.front.take().map(|b| *b);
        insert_detail_polys_into_bsp(&mut front_child, front_detail, config, detail_cuts);
        node.front = front_child.map(Box::new);
    }

    if !back_detail.is_empty() {
        let mut back_child = node.back.take().map(|b| *b);
        insert_detail_polys_into_bsp(&mut back_child, back_detail, config, detail_cuts);
        node.back = back_child.map(Box::new);
    }
}

fn build_two_phase_bsp(
    polys: Vec<Polygon>,
    config: &BspBuilderConfig,
) -> (Option<BspNode>, usize) {
    let mut structural_polys = Vec::new();
    let mut detail_polys = Vec::new();
    for p in polys {
        let is_portal = (p.flags & crate::poly::PF_PORTAL) != 0;
        let is_detail = !is_portal
            && (p.flags & (crate::poly::PF_SEMISOLID | crate::poly::PF_LEGACY_SEMISOLID | crate::poly::PF_NOT_SOLID)) != 0;
        if is_detail {
            detail_polys.push(p);
        } else {
            structural_polys.push(p);
        }
    }

    let mut structural_cuts = 0;
    let mut root = build_bsp_recursive(structural_polys, config, &mut structural_cuts);
    let mut _detail_cuts = 0;
    insert_detail_polys_into_bsp(&mut root, detail_polys, config, &mut _detail_cuts);
    (root, structural_cuts)
}

impl BspNode {
    pub fn new(polys: Vec<Polygon>) -> Option<Self> {
        build_two_phase_bsp(polys, &BspBuilderConfig::default()).0
    }

    pub fn build_with_config(polys: Vec<Polygon>, config: &BspBuilderConfig) -> (Option<Self>, usize) {
        build_two_phase_bsp(polys, config)
    }

    pub fn depth(&self) -> usize {
        let f_depth = self.front.as_ref().map(|f| f.depth()).unwrap_or(0);
        let b_depth = self.back.as_ref().map(|b| b.depth()).unwrap_or(0);
        1 + f_depth.max(b_depth)
    }

    pub fn node_count(&self) -> usize {
        let f_cnt = self.front.as_ref().map(|f| f.node_count()).unwrap_or(0);
        let b_cnt = self.back.as_ref().map(|b| b.node_count()).unwrap_or(0);
        1 + f_cnt + b_cnt
    }

    pub fn coplanar_count(&self) -> usize {
        let f_cnt = self.front.as_ref().map(|f| f.coplanar_count()).unwrap_or(0);
        let b_cnt = self.back.as_ref().map(|b| b.coplanar_count()).unwrap_or(0);
        self.coplanar.len() + f_cnt + b_cnt
    }

    /// Recursively collects all polygons contained in this BSP tree.
    pub fn collect_polygons(&self) -> Vec<Polygon> {
        let mut result = self.coplanar.clone();
        if let Some(ref f) = self.front {
            result.extend(f.collect_polygons());
        }
        if let Some(ref b) = self.back {
            result.extend(b.collect_polygons());
        }
        result
    }

    /// Inverts this BSP tree (reversing all planes and normals, swapping front and back).
    pub fn invert(&mut self) {
        self.plane.normal = -self.plane.normal;
        self.plane.dist = -self.plane.dist;

        for p in &mut self.coplanar {
            *p = p.inverted();
        }

        std::mem::swap(&mut self.front, &mut self.back);

        if let Some(ref mut f) = self.front {
            f.invert();
        }
        if let Some(ref mut b) = self.back {
            b.invert();
        }
    }

    /// Clips a list of polygons against this BSP tree, keeping only parts outside the tree solid.
    pub fn clip_polygons(&self, polys: &[Polygon]) -> Vec<Polygon> {
        let mut front_list = Vec::new();
        let mut back_list = Vec::new();

        for poly in polys {
            match poly.split(&self.plane) {
                SplitResult::Front(p) => front_list.push(p),
                SplitResult::Back(p) => back_list.push(p),
                SplitResult::Spanning { front, back } => {
                    front_list.push(front);
                    back_list.push(back);
                }
            }
        }

        let mut result = Vec::new();

        if let Some(ref f) = self.front {
            result.extend(f.clip_polygons(&front_list));
        } else {
            result.extend(front_list);
        }

        if let Some(ref b) = self.back {
            result.extend(b.clip_polygons(&back_list));
        }

        result
    }

    /// Clips a list of polygons against this BSP tree, keeping only parts INSIDE the tree solid.
    pub fn clip_polygons_inside(&self, polys: &[Polygon]) -> Vec<Polygon> {
        let mut inverted = self.clone();
        inverted.invert();
        inverted.clip_polygons(polys)
    }

    /// Clips this tree's polygons against another BSP tree (retaining only parts outside `other`).
    pub fn clip_to(&mut self, other: &BspNode) {
        self.coplanar = other.clip_polygons(&self.coplanar);
        if let Some(ref mut f) = self.front {
            f.clip_to(other);
        }
        if let Some(ref mut b) = self.back {
            b.clip_to(other);
        }
    }
}

#[derive(Clone)]
struct PreparedStructuralBrush {
    brush_idx: usize,
    csg_oper: crate::CsgOperation,
    world_polys: Vec<Polygon>,
    aabb: (glam::Vec3, glam::Vec3),
    is_convex: bool,
}

fn poly_aabb(poly: &Polygon) -> (glam::Vec3, glam::Vec3) {
    let mut min = glam::Vec3::splat(f32::MAX);
    let mut max = glam::Vec3::splat(f32::MIN);
    for &v in &poly.vertices {
        min = min.min(v);
        max = max.max(v);
    }
    (min, max)
}

fn poly_area_3d(vertices: &[glam::Vec3]) -> f32 {
    if vertices.len() < 3 {
        return 0.0;
    }
    let p0 = vertices[0];
    let mut sum = glam::Vec3::ZERO;
    for i in 1..(vertices.len() - 1) {
        sum += (vertices[i] - p0).cross(vertices[i + 1] - p0);
    }
    sum.length() * 0.5
}

fn point_in_structural_brush(pt: glam::Vec3, b: &PreparedStructuralBrush) -> bool {
    if pt.x < b.aabb.0.x - 0.05
        || pt.x > b.aabb.1.x + 0.05
        || pt.y < b.aabb.0.y - 0.05
        || pt.y > b.aabb.1.y + 0.05
        || pt.z < b.aabb.0.z - 0.05
        || pt.z > b.aabb.1.z + 0.05
    {
        return false;
    }
    if b.world_polys.is_empty() {
        return false;
    }
    if b.is_convex {
        b.world_polys
            .iter()
            .all(|p| p.plane.dist_to_point(pt) <= 0.05)
    } else {
        // Jordan ray-casting parity test for concave polyhedra
        let ray_dir = glam::Vec3::new(0.57735026, 0.51449576, 0.63402235).normalize();
        let mut hits = 0usize;
        for poly in &b.world_polys {
            if poly.vertices.len() < 3 {
                continue;
            }
            let v0 = poly.vertices[0];
            for i in 1..(poly.vertices.len() - 1) {
                if let Some(t) = crate::lighting::ray_intersects_triangle(
                    pt,
                    ray_dir,
                    v0,
                    poly.vertices[i],
                    poly.vertices[i + 1],
                ) {
                    if t > 0.02 {
                        hits += 1;
                    }
                }
            }
        }
        hits % 2 == 1
    }
}

fn is_point_solid_at(
    pt: glam::Vec3,
    structural: &[PreparedStructuralBrush],
    universe_starts_solid: bool,
) -> bool {
    let mut solid = universe_starts_solid;
    for b in structural {
        if point_in_structural_brush(pt, b) {
            solid = b.csg_oper != crate::CsgOperation::Subtract;
        }
    }
    solid
}

fn partition_poly_by_brush(poly: Polygon, b: &PreparedStructuralBrush) -> Vec<Polygon> {
    let (p_min, p_max) = poly_aabb(&poly);
    let tol = 0.5;
    if p_max.x < b.aabb.0.x - tol
        || p_min.x > b.aabb.1.x + tol
        || p_max.y < b.aabb.0.y - tol
        || p_min.y > b.aabb.1.y + tol
        || p_max.z < b.aabb.0.z - tol
        || p_min.z > b.aabb.1.z + tol
    {
        return vec![poly];
    }

    if b.is_convex {
        let mut outside = Vec::new();
        let mut current = Some(poly);

        for face in &b.world_polys {
            let Some(curr) = current.take() else {
                break;
            };
            let dot = curr.plane.normal.dot(face.plane.normal);
            let is_coplanar = dot.abs() > 0.999
                && (curr.plane.dist - face.plane.dist * dot.signum()).abs() < 0.25;
            if is_coplanar {
                current = Some(curr);
                continue;
            }
            match curr.split(&face.plane) {
                SplitResult::Front(f) => {
                    outside.push(f);
                    break;
                }
                SplitResult::Back(bk) => {
                    current = Some(bk);
                }
                SplitResult::Spanning { front, back } => {
                    outside.push(front);
                    current = Some(back);
                }
            }
        }

        if let Some(rem) = current {
            outside.push(rem);
        }
        outside
    } else {
        let mut fragments = vec![poly];
        for face in &b.world_polys {
            let mut next_frags = Vec::with_capacity(fragments.len());
            for frag in fragments {
                let dot = frag.plane.normal.dot(face.plane.normal);
                let is_coplanar = dot.abs() > 0.999
                    && (frag.plane.dist - face.plane.dist * dot.signum()).abs() < 0.25;
                if is_coplanar {
                    next_frags.push(frag);
                    continue;
                }
                match frag.split(&face.plane) {
                    SplitResult::Front(f) => next_frags.push(f),
                    SplitResult::Back(bk) => next_frags.push(bk),
                    SplitResult::Spanning { front, back } => {
                        next_frags.push(front);
                        next_frags.push(back);
                    }
                }
            }
            fragments = next_frags;
        }
        fragments
    }
}

/// Default number of worker threads reserved for parallel CSG evaluation (`(cores - 2).clamp(1, 6)`).
pub fn default_csg_worker_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .saturating_sub(2)
        .clamp(1, 6)
}

fn csg_thread_pool(threads: usize) -> Option<&'static rayon::ThreadPool> {
    static POOL_2: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    static POOL_3: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    static POOL_4: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    static POOL_5: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    static POOL_6: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();

    let n = threads.clamp(2, 6);
    let slot = match n {
        2 => &POOL_2,
        3 => &POOL_3,
        4 => &POOL_4,
        5 => &POOL_5,
        _ => &POOL_6,
    };
    slot.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .thread_name(move |i| format!("forge99-csg{n}-{i}"))
            .build()
            .ok()
    })
    .as_ref()
}

#[inline]
fn is_cancelled(cancel: Option<&AtomicBool>) -> bool {
    cancel.is_some_and(|c| c.load(Ordering::Relaxed))
}

#[inline]
fn classify_brush_role(brush: &crate::brush::Brush) -> (bool, bool) {
    let is_non_solid = brush.polys.len() <= 2
        || brush
            .polys
            .iter()
            .all(|p| (p.flags & (crate::PF_NOT_SOLID | crate::PF_PORTAL)) != 0);
    let is_semi_solid = !is_non_solid
        && brush.csg_oper == crate::CsgOperation::Add
        && brush
            .polys
            .iter()
            .any(|p| (p.flags & (crate::PF_SEMISOLID | crate::PF_LEGACY_SEMISOLID)) != 0);
    (is_non_solid, is_semi_solid)
}

fn brush_world_polys_and_aabb(
    brush: &crate::brush::Brush,
) -> (Vec<Polygon>, (glam::Vec3, glam::Vec3)) {
    if brush.polys.is_empty() {
        return (Vec::new(), (brush.location, brush.location));
    }
    let model_matrix = brush.model_matrix();
    let mut min = glam::Vec3::splat(f32::MAX);
    let mut max = glam::Vec3::splat(f32::MIN);
    let mut world_polys = Vec::with_capacity(brush.polys.len());

    for poly in &brush.polys {
        let mut transformed_verts = Vec::with_capacity(poly.vertices.len());
        for &v in &poly.vertices {
            let tv = model_matrix.transform_point3(v);
            min = min.min(tv);
            max = max.max(tv);
            transformed_verts.push(tv);
        }
        if let Some(mut new_poly) = Polygon::new(transformed_verts) {
            new_poly.texture = poly.texture.clone();
            new_poly.flags = poly.flags;
            new_poly.base = model_matrix.transform_point3(poly.base);
            new_poly.texture_u = model_matrix.transform_vector3(poly.texture_u);
            new_poly.texture_v = model_matrix.transform_vector3(poly.texture_v);
            new_poly.pan_u = poly.pan_u;
            new_poly.pan_v = poly.pan_v;
            new_poly.vertex_colors = poly.vertex_colors.clone();
            new_poly.lightmap = poly.lightmap.clone();
            world_polys.push(new_poly);
        }
    }

    (world_polys, (min, max))
}

#[inline]
fn is_point_solid_at_bvh(
    pt: glam::Vec3,
    structural: &[PreparedStructuralBrush],
    bvh: &AabbBvh,
    max_pos_exclusive: usize,
    universe_starts_solid: bool,
) -> bool {
    match bvh.find_highest_pos_containing_point(pt, 0.051, max_pos_exclusive as u32, |pos| {
        point_in_structural_brush(pt, &structural[pos as usize])
    }) {
        Some(pos) => structural[pos as usize].csg_oper != crate::CsgOperation::Subtract,
        None => universe_starts_solid,
    }
}

fn is_overridden_by_later_bvh(
    centroid: glam::Vec3,
    n: glam::Vec3,
    frag_dist: f32,
    s_pos: usize,
    structural: &[PreparedStructuralBrush],
    bvh: &AabbBvh,
) -> bool {
    bvh.any_pos_above_containing_point(centroid, 0.051, s_pos as u32, |later_pos| {
        let later = &structural[later_pos as usize];
        if !point_in_structural_brush(centroid, later) {
            return false;
        }
        let later_sub = later.csg_oper == crate::CsgOperation::Subtract;
        for lp in &later.world_polys {
            let eff_normal = if later_sub {
                -lp.plane.normal
            } else {
                lp.plane.normal
            };
            let eff_dist = if later_sub {
                -lp.plane.dist
            } else {
                lp.plane.dist
            };
            if eff_normal.dot(n) > 0.999 && (eff_dist - frag_dist).abs() < 0.25 {
                return true;
            }
        }
        false
    })
}

fn eval_structural_brush(
    s_pos: usize,
    structural: &[PreparedStructuralBrush],
    bvh: &AabbBvh,
    universe_starts_solid: bool,
    cancel: Option<&AtomicBool>,
) -> Option<Vec<(usize, Polygon)>> {
    if is_cancelled(cancel) {
        return None;
    }
    let sb = &structural[s_pos];
    let is_sub = sb.csg_oper == crate::CsgOperation::Subtract;
    let mut fragments: Vec<Polygon> = sb
        .world_polys
        .iter()
        .map(|p| if is_sub { p.inverted() } else { p.clone() })
        .collect();

    let mut candidates = Vec::new();
    bvh.query_aabb_into(
        sb.aabb.0,
        sb.aabb.1,
        0.501,
        structural.len() as u32,
        &mut candidates,
    );
    // Partition order must match sequential structural brush order.
    candidates.sort_unstable();

    let tol = 0.5;
    for other_pos_u32 in candidates {
        let other_pos = other_pos_u32 as usize;
        if other_pos == s_pos {
            continue;
        }
        if is_cancelled(cancel) {
            return None;
        }
        let other = &structural[other_pos];
        if sb.aabb.1.x < other.aabb.0.x - tol
            || sb.aabb.0.x > other.aabb.1.x + tol
            || sb.aabb.1.y < other.aabb.0.y - tol
            || sb.aabb.0.y > other.aabb.1.y + tol
            || sb.aabb.1.z < other.aabb.0.z - tol
            || sb.aabb.0.z > other.aabb.1.z + tol
        {
            continue;
        }
        let mut next_frags = Vec::with_capacity(fragments.len());
        for frag in fragments {
            next_frags.extend(partition_poly_by_brush(frag, other));
        }
        fragments = next_frags;
    }

    let mut out = Vec::new();
    for (frag_idx, frag) in fragments.into_iter().enumerate() {
        if (frag_idx & 31) == 0 && is_cancelled(cancel) {
            return None;
        }
        if frag.vertices.len() < 3 || poly_area_3d(&frag.vertices) < 0.05 {
            continue;
        }
        let centroid =
            frag.vertices.iter().copied().sum::<glam::Vec3>() / (frag.vertices.len() as f32);
        let n = frag.plane.normal;
        let front_pt = centroid + n * 0.25;
        let back_pt = centroid - n * 0.25;

        let front_is_air = !is_point_solid_at_bvh(
            front_pt,
            structural,
            bvh,
            structural.len(),
            universe_starts_solid,
        );
        let back_is_solid = is_point_solid_at_bvh(
            back_pt,
            structural,
            bvh,
            structural.len(),
            universe_starts_solid,
        );

        if front_is_air
            && back_is_solid
            && !is_overridden_by_later_bvh(centroid, n, frag.plane.dist, s_pos, structural, bvh)
        {
            out.push((sb.brush_idx, frag));
        }
    }

    Some(out)
}

enum NonStructuralRole {
    NonSolid,
    SemiSolid,
}

fn eval_non_structural_brush(
    idx: usize,
    role: &NonStructuralRole,
    brush: &crate::brush::Brush,
    structural: &[PreparedStructuralBrush],
    bvh: &AabbBvh,
    cancel: Option<&AtomicBool>,
) -> Option<Vec<(usize, Polygon)>> {
    if is_cancelled(cancel) {
        return None;
    }
    match role {
        NonStructuralRole::NonSolid => Some(
            brush
                .to_world_polys()
                .into_iter()
                .map(|poly| (idx, poly))
                .collect(),
        ),
        NonStructuralRole::SemiSolid => {
            let prior_pos = structural.partition_point(|s| s.brush_idx < idx);
            let prior_starts_solid = structural
                .first()
                .filter(|_| prior_pos > 0)
                .map(|b| b.csg_oper == crate::CsgOperation::Subtract)
                .unwrap_or(false);
            let (mut fragments, aabb) = brush_world_polys_and_aabb(brush);
            let mut candidates = Vec::new();
            bvh.query_aabb_into(aabb.0, aabb.1, 0.501, prior_pos as u32, &mut candidates);
            candidates.sort_unstable();

            let tol = 0.5;
            for other_pos_u32 in candidates {
                if is_cancelled(cancel) {
                    return None;
                }
                let other = &structural[other_pos_u32 as usize];
                if aabb.1.x < other.aabb.0.x - tol
                    || aabb.0.x > other.aabb.1.x + tol
                    || aabb.1.y < other.aabb.0.y - tol
                    || aabb.0.y > other.aabb.1.y + tol
                    || aabb.1.z < other.aabb.0.z - tol
                    || aabb.0.z > other.aabb.1.z + tol
                {
                    continue;
                }
                let mut next_frags = Vec::with_capacity(fragments.len());
                for frag in fragments {
                    next_frags.extend(partition_poly_by_brush(frag, other));
                }
                fragments = next_frags;
            }

            let mut out = Vec::new();
            for (frag_idx, frag) in fragments.into_iter().enumerate() {
                if (frag_idx & 31) == 0 && is_cancelled(cancel) {
                    return None;
                }
                if frag.vertices.len() < 3 || poly_area_3d(&frag.vertices) < 0.05 {
                    continue;
                }
                let centroid = frag.vertices.iter().copied().sum::<glam::Vec3>()
                    / (frag.vertices.len() as f32);
                let front_pt = centroid + frag.plane.normal * 0.25;
                if !is_point_solid_at_bvh(front_pt, structural, bvh, prior_pos, prior_starts_solid)
                {
                    out.push((idx, frag));
                }
            }
            Some(out)
        }
    }
}

/// Evaluates sequential Boolean CSG operations (`Subtract`, `Add`, `Semi-Solid`, `Non-Solid`)
/// across all world brushes in order, returning `(brush_index, Polygon)` for every visible
/// world-space surface fragment.
pub fn evaluate_csg_brushes_indexed(brushes: &[crate::brush::Brush]) -> Vec<(usize, Polygon)> {
    evaluate_csg_brushes_indexed_with_options(brushes, default_csg_worker_threads(), None)
        .unwrap_or_default()
}

/// Evaluates sequential Boolean CSG operations with cooperative cancellation via `cancel`.
/// Returns `None` if `cancel` is observed set before completion.
pub fn evaluate_csg_brushes_indexed_cancellable(
    brushes: &[crate::brush::Brush],
    cancel: &AtomicBool,
) -> Option<Vec<(usize, Polygon)>> {
    evaluate_csg_brushes_indexed_with_options(brushes, default_csg_worker_threads(), Some(cancel))
}

/// Evaluates sequential Boolean CSG operations using an explicit worker thread budget and
/// optional cooperative cancellation flag while preserving deterministic polygon ordering.
pub fn evaluate_csg_brushes_indexed_with_options(
    brushes: &[crate::brush::Brush],
    max_threads: usize,
    cancel: Option<&AtomicBool>,
) -> Option<Vec<(usize, Polygon)>> {
    if is_cancelled(cancel) {
        return None;
    }
    if brushes.is_empty() {
        return Some(Vec::new());
    }

    let mut structural = Vec::new();
    let mut non_structural: Vec<(usize, NonStructuralRole)> = Vec::new();
    for (idx, brush) in brushes.iter().enumerate() {
        if (idx & 31) == 0 && is_cancelled(cancel) {
            return None;
        }
        if brush.polys.is_empty() {
            continue;
        }
        let (is_non_solid, is_semi_solid) = classify_brush_role(brush);
        if is_non_solid {
            non_structural.push((idx, NonStructuralRole::NonSolid));
        } else if is_semi_solid {
            non_structural.push((idx, NonStructuralRole::SemiSolid));
        } else {
            let (world_polys, aabb) = brush_world_polys_and_aabb(brush);
            if world_polys.is_empty() {
                continue;
            }
            let is_convex = world_polys.iter().all(|f| {
                world_polys
                    .iter()
                    .all(|other| other.vertices.iter().all(|&v| f.plane.dist_to_point(v) <= 0.5))
            });
            structural.push(PreparedStructuralBrush {
                brush_idx: idx,
                csg_oper: brush.csg_oper,
                world_polys,
                aabb,
                is_convex,
            });
        }
    }

    if is_cancelled(cancel) {
        return None;
    }

    // In Unreal Engine 1, if the first structural brush is Subtract, the universe starts as Solid;
    // if the first structural brush is Add (e.g. an additive-only test map), the universe starts as Air.
    let universe_starts_solid = structural
        .first()
        .map(|b| b.csg_oper == crate::CsgOperation::Subtract)
        .unwrap_or(false);

    let bvh_entries: Vec<BvhEntry> = structural
        .iter()
        .enumerate()
        .map(|(s_pos, sb)| BvhEntry {
            pos: s_pos as u32,
            min: sb.aabb.0,
            max: sb.aabb.1,
        })
        .collect();
    let bvh = AabbBvh::build(&bvh_entries);

    let pool = if max_threads > 1 && brushes.len() >= 16 {
        csg_thread_pool(max_threads)
    } else {
        None
    };

    let (structural_batches, non_structural_batches): (
        Option<Vec<Vec<(usize, Polygon)>>>,
        Option<Vec<Vec<(usize, Polygon)>>>,
    ) = if let Some(pool) = pool {
        pool.install(|| {
            let sb = (0..structural.len())
                .into_par_iter()
                .map(|s_pos| {
                    eval_structural_brush(s_pos, &structural, &bvh, universe_starts_solid, cancel)
                })
                .collect::<Option<Vec<_>>>();
            let sb = match sb {
                Some(v) => Some(v),
                None => return (None, None),
            };
            let nsb = non_structural
                .par_iter()
                .map(|(idx, role)| {
                    eval_non_structural_brush(
                        *idx,
                        role,
                        &brushes[*idx],
                        &structural,
                        &bvh,
                        cancel,
                    )
                })
                .collect::<Option<Vec<_>>>();
            (sb, nsb)
        })
    } else {
        let mut sb = Vec::with_capacity(structural.len());
        for s_pos in 0..structural.len() {
            sb.push(eval_structural_brush(
                s_pos,
                &structural,
                &bvh,
                universe_starts_solid,
                cancel,
            )?);
        }
        let mut nsb = Vec::with_capacity(non_structural.len());
        for (idx, role) in &non_structural {
            nsb.push(eval_non_structural_brush(
                *idx,
                role,
                &brushes[*idx],
                &structural,
                &bvh,
                cancel,
            )?);
        }
        (Some(sb), Some(nsb))
    };

    let structural_batches = structural_batches?;
    let non_structural_batches = non_structural_batches?;

    let total: usize = structural_batches.iter().map(Vec::len).sum::<usize>()
        + non_structural_batches.iter().map(Vec::len).sum::<usize>();
    let mut result = Vec::with_capacity(total);
    for batch in structural_batches {
        result.extend(batch);
    }
    for batch in non_structural_batches {
        result.extend(batch);
    }

    Some(result)
}

/// Evaluates sequential Boolean CSG operations across all brushes and returns the resulting world polygons.
pub fn evaluate_csg_brushes(brushes: &[crate::brush::Brush]) -> Vec<Polygon> {
    evaluate_csg_brushes_indexed(brushes)
        .into_iter()
        .map(|(_, p)| p)
        .collect()
}

/// Computes the UnrealEd 2 `CSG Intersect` (`keep_solid = true`) or `CSG Deintersect` (`keep_solid = false`)
/// of `builder` against `world_brushes`, returning the resulting world-space polygons with outward normals.
pub fn csg_boolean_with_world(
    builder: &crate::brush::Brush,
    world_brushes: &[crate::brush::Brush],
    keep_solid: bool,
) -> Vec<Polygon> {
    let builder_world = builder.to_world_polys();
    if builder_world.is_empty() || world_brushes.is_empty() {
        return Vec::new();
    }
    let builder_aabb = builder.world_bounds();
    let builder_is_convex = builder_world.iter().all(|f| {
        builder_world
            .iter()
            .all(|other| other.vertices.iter().all(|&v| f.plane.dist_to_point(v) <= 0.5))
    });
    let prep_builder = PreparedStructuralBrush {
        brush_idx: usize::MAX,
        csg_oper: crate::CsgOperation::Active,
        world_polys: builder_world.clone(),
        aabb: builder_aabb,
        is_convex: builder_is_convex,
    };

    let mut structural = Vec::new();
    for (idx, brush) in world_brushes.iter().enumerate() {
        if brush.polys.is_empty() {
            continue;
        }
        let is_non_solid = brush.polys.len() <= 2
            || brush
                .polys
                .iter()
                .all(|p| (p.flags & (crate::PF_NOT_SOLID | crate::PF_PORTAL)) != 0);
        let is_semi_solid = !is_non_solid
            && brush.csg_oper == crate::CsgOperation::Add
            && brush.polys.iter().any(|p| (p.flags & (crate::PF_SEMISOLID | crate::PF_LEGACY_SEMISOLID)) != 0);
        if !is_non_solid && !is_semi_solid {
            let wp = brush.to_world_polys();
            if wp.is_empty() {
                continue;
            }
            let aabb = brush.world_bounds();
            let is_convex = wp.iter().all(|f| {
                wp.iter()
                    .all(|other| other.vertices.iter().all(|&v| f.plane.dist_to_point(v) <= 0.5))
            });
            structural.push(PreparedStructuralBrush {
                brush_idx: idx,
                csg_oper: brush.csg_oper,
                world_polys: wp,
                aabb,
                is_convex,
            });
        }
    }

    if structural.is_empty() {
        return Vec::new();
    }

    let universe_starts_solid = structural
        .first()
        .map(|b| b.csg_oper == crate::CsgOperation::Subtract)
        .unwrap_or(false);

    // 1. Partition builder's own faces against overlapping structural brushes
    let mut builder_frags = builder_world.clone();
    for sb in &structural {
        let tol = 0.5;
        if builder_aabb.1.x < sb.aabb.0.x - tol
            || builder_aabb.0.x > sb.aabb.1.x + tol
            || builder_aabb.1.y < sb.aabb.0.y - tol
            || builder_aabb.0.y > sb.aabb.1.y + tol
            || builder_aabb.1.z < sb.aabb.0.z - tol
            || builder_aabb.0.z > sb.aabb.1.z + tol
        {
            continue;
        }
        let mut next = Vec::with_capacity(builder_frags.len());
        for frag in builder_frags {
            next.extend(partition_poly_by_brush(frag, sb));
        }
        builder_frags = next;
    }

    let mut out_polys = Vec::new();
    for frag in builder_frags {
        if frag.vertices.len() < 3 || poly_area_3d(&frag.vertices) < 0.05 {
            continue;
        }
        let centroid = frag.vertices.iter().copied().sum::<glam::Vec3>()
            / (frag.vertices.len() as f32);
        let inside_builder_pt = centroid - frag.plane.normal * 0.25;
        let is_solid = is_point_solid_at(inside_builder_pt, &structural, universe_starts_solid);
        if is_solid == keep_solid {
            out_polys.push(frag);
        }
    }

    // 2. Partition world CSG surfaces against builder and keep pieces strictly inside builder
    let world_csg_polys = evaluate_csg_brushes(world_brushes);
    for wpoly in world_csg_polys {
        for frag in partition_poly_by_brush(wpoly, &prep_builder) {
            if frag.vertices.len() < 3 || poly_area_3d(&frag.vertices) < 0.05 {
                continue;
            }
            let centroid = frag.vertices.iter().copied().sum::<glam::Vec3>()
                / (frag.vertices.len() as f32);
            if !point_in_structural_brush(centroid, &prep_builder) {
                continue;
            }
            // Skip if coplanar with any builder face
            let is_coplanar = builder_world.iter().any(|bp| {
                let dot = bp.plane.normal.dot(frag.plane.normal);
                dot.abs() > 0.999 && (bp.plane.dist - frag.plane.dist * dot.signum()).abs() < 0.25
            });
            if !is_coplanar {
                if keep_solid {
                    out_polys.push(frag);
                } else {
                    out_polys.push(frag.inverted());
                }
            }
        }
    }

    out_polys
}

pub fn build_bsp_from_brushes(
    brushes: &[crate::brush::Brush],
    config: &BspBuilderConfig,
) -> (Option<BspNode>, BspBuildReport) {
    let start_time = std::time::Instant::now();
    let all_polys = evaluate_csg_brushes(brushes);

    let poly_count = all_polys.len();
    let (root, cut_count) = build_two_phase_bsp(all_polys, config);

    let (node_count, coplanar_count, max_depth) = if let Some(ref r) = root {
        (r.node_count(), r.coplanar_count(), r.depth())
    } else {
        (0, 0, 0)
    };

    let elapsed_ms = start_time.elapsed().as_millis() as u64;

    let balance_ratio_pct = if node_count > 0 && max_depth > 0 {
        let optimal_depth = (node_count as f32).log2().ceil().max(1.0);
        let ratio = (optimal_depth / max_depth as f32).clamp(0.0, 1.0);
        (ratio * 100.0).round()
    } else {
        100.0
    };

    let report = BspBuildReport {
        node_count,
        poly_count,
        coplanar_count,
        max_depth,
        cut_count,
        balance_ratio_pct,
        elapsed_ms,
    };

    (root, report)
}

/// Diagnostic and repair summary produced by the Assisted Geometry Auto-Repair tool ("BSP Doctor", `BL-P3-02`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BspDoctorReport {
    pub snapped_vertices_count: usize,
    pub welded_vertices_count: usize,
    pub triangulated_non_planar_polys_count: usize,
    pub removed_degenerate_polys_count: usize,
}

/// Inspects and repairs floating-point micro-drift, coincident vertices, degenerate edges/faces,
/// and non-planar >= 4-vertex polygons across a slice of brushes (`BL-P2-01` & `BL-P3-02`).
pub fn repair_brushes_geometry(
    brushes: &mut [crate::brush::Brush],
    grid_step: f32,
    snap_epsilon: f32,
    planarity_epsilon: f32,
) -> BspDoctorReport {
    let mut report = BspDoctorReport::default();
    let step = if grid_step >= 1.0 { grid_step } else { 1.0 };

    let snap_scalar = |v: f32, snapped_cnt: &mut usize| -> f32 {
        let nearest_int = v.round();
        if (v - nearest_int).abs() > 1e-6 && (v - nearest_int).abs() <= snap_epsilon {
            *snapped_cnt += 1;
            return nearest_int;
        }
        let nearest_grid = (v / step).round() * step;
        if (v - nearest_grid).abs() > 1e-6 && (v - nearest_grid).abs() <= snap_epsilon {
            *snapped_cnt += 1;
            return nearest_grid;
        }
        v
    };

    for brush in brushes.iter_mut() {
        brush.location.x = snap_scalar(brush.location.x, &mut report.snapped_vertices_count);
        brush.location.y = snap_scalar(brush.location.y, &mut report.snapped_vertices_count);
        brush.location.z = snap_scalar(brush.location.z, &mut report.snapped_vertices_count);

        let old_polys = std::mem::take(&mut brush.polys);
        let mut repaired_polys = Vec::with_capacity(old_polys.len());

        for mut poly in old_polys {
            // 1. Snap micro-drifted vertex coordinates
            for v in &mut poly.vertices {
                v.x = snap_scalar(v.x, &mut report.snapped_vertices_count);
                v.y = snap_scalar(v.y, &mut report.snapped_vertices_count);
                v.z = snap_scalar(v.z, &mut report.snapped_vertices_count);
            }

            // 2. Weld consecutive coincident vertices (distance < 0.05 UU)
            let mut dedup_verts: Vec<glam::Vec3> = Vec::with_capacity(poly.vertices.len());
            for v in poly.vertices {
                if let Some(&prev) = dedup_verts.last() {
                    if prev.distance(v) < 0.05 {
                        report.welded_vertices_count += 1;
                        continue;
                    }
                }
                dedup_verts.push(v);
            }
            if dedup_verts.len() >= 2
                && dedup_verts
                    .first()
                    .unwrap()
                    .distance(*dedup_verts.last().unwrap())
                    < 0.05
            {
                dedup_verts.pop();
                report.welded_vertices_count += 1;
            }
            poly.vertices = dedup_verts;

            // 3. Remove degenerate polygons (< 3 vertices or near-zero area)
            if poly.vertices.len() < 3 || poly_area_3d(&poly.vertices) < 0.05 {
                report.removed_degenerate_polys_count += 1;
                continue;
            }

            // 4. Refresh base plane from first 3 non-collinear vertices
            if let Some(plane) = crate::math::Plane::from_points(
                poly.vertices[0],
                poly.vertices[1],
                poly.vertices[2],
            ) {
                poly.plane = plane;
            } else {
                report.removed_degenerate_polys_count += 1;
                continue;
            }

            // 5. Auto-triangulate non-planar >= 4-vertex polygons
            if !poly.is_planar(planarity_epsilon) {
                report.triangulated_non_planar_polys_count += 1;
                let tris = poly.triangulate_if_non_planar(planarity_epsilon);
                repaired_polys.extend(tris);
            } else {
                repaired_polys.push(poly);
            }
        }

        brush.polys = repaired_polys;
    }

    report
}

#[cfg(test)]
mod csg_tests {
    use super::*;
    use crate::brush::build_cube;
    use crate::CsgOperation;
    use glam::Vec3;

    #[test]
    fn test_csg_subtract_carves_add_brush() {
        // Start with a 256x256x256 Add block, then punch a 64x300x64 Subtract tunnel through Y
        let add_cube = build_cube("Wall", CsgOperation::Add, Vec3::new(256.0, 64.0, 256.0));
        let sub_door = build_cube("Door", CsgOperation::Subtract, Vec3::new(64.0, 64.0, 128.0));

        let polys = evaluate_csg_brushes(&[add_cube, sub_door]);
        assert!(
            polys.len() > 6,
            "Carved wall must have split faces and inner doorway jambs, got {}",
            polys.len()
        );
        for p in &polys {
            let c = p.vertices.iter().copied().sum::<Vec3>() / (p.vertices.len() as f32);
            let blocks_doorway_center = c.x.abs() < 16.0 && c.z.abs() < 16.0 && p.plane.normal.y.abs() > 0.9;
            assert!(
                !blocks_doorway_center,
                "Center of carved doorway at {:?} must be open (not blocked by a cap polygon)",
                c
            );
        }
    }

    #[test]
    fn test_csg_two_subtract_rooms_open_connection() {
        let mut room1 = build_cube("Room1", CsgOperation::Subtract, Vec3::new(512.0, 512.0, 256.0));
        room1.location = Vec3::new(-256.0, 0.0, 0.0); // X from -512 to 0
        let mut room2 = build_cube("Room2", CsgOperation::Subtract, Vec3::new(512.0, 512.0, 256.0));
        room2.location = Vec3::new(256.0, 0.0, 0.0); // X from 0 to +512 (touching at X=0)

        let polys = evaluate_csg_brushes(&[room1, room2]);
        assert_eq!(
            polys.len(),
            10,
            "Two identical touching Subtract rooms should have their shared wall removed (10 outer faces)"
        );
    }

    #[test]
    fn test_csg_intersect_and_deintersect_inside_subtract_room() {
        let room = build_cube("Room0", CsgOperation::Subtract, Vec3::new(1024.0, 1024.0, 512.0));
        let crate_add = build_cube("Crate", CsgOperation::Add, Vec3::new(128.0, 128.0, 128.0));
        let builder = build_cube("Builder", CsgOperation::Active, Vec3::new(256.0, 256.0, 256.0));

        // Intersect captures only the 128x128x128 solid Crate inside the 256x256x256 Builder
        let intersected = csg_boolean_with_world(&builder, &[room.clone(), crate_add.clone()], true);
        assert_eq!(intersected.len(), 6, "CSG Intersect around a single Add crate inside a Subtract room must yield the 6 crate faces");

        // Deintersect captures the 256x256x256 Air volume with the 128x128x128 Crate carved out
        let deintersected = csg_boolean_with_world(&builder, &[room, crate_add], false);
        assert_eq!(deintersected.len(), 12, "CSG Deintersect around an Add crate must yield 6 outer faces + 6 inner cavity faces");
    }

    #[test]
    fn test_semisolid_does_not_split_structural_bsp() {
        let room = build_cube("Room0", CsgOperation::Subtract, Vec3::new(512.0, 512.0, 256.0));
        let mut pillar = crate::brush::build_cylinder("Pillar", CsgOperation::Add, 192.0, 48.0, 16);
        for p in &mut pillar.polys {
            p.flags |= crate::poly::PF_SEMISOLID;
        }

        let cfg = BspBuilderConfig::default();
        let (_, report) = build_bsp_from_brushes(&[room, pillar], &cfg);
        assert_eq!(
            report.cut_count, 0,
            "Semi-solid pillar inside a convex subtracted room must cause 0 structural BSP cuts, got {}",
            report.cut_count
        );
    }

    #[test]
    fn test_portal_bias_prioritizes_portal_plane() {
        let mut room1 = build_cube("Room1", CsgOperation::Subtract, Vec3::new(512.0, 512.0, 256.0));
        room1.location = Vec3::new(-256.0, 0.0, 0.0);
        let mut room2 = build_cube("Room2", CsgOperation::Subtract, Vec3::new(512.0, 512.0, 256.0));
        room2.location = Vec3::new(256.0, 0.0, 0.0);
        let mut portal = crate::brush::build_sheet("Portal0", CsgOperation::Add, 512.0, 256.0);
        for p in &mut portal.polys {
            p.flags |= crate::poly::PF_PORTAL | crate::poly::PF_NOT_SOLID;
        }

        let cfg = BspBuilderConfig {
            portal_bias: 100,
            ..Default::default()
        };
        let (root_opt, _) = build_bsp_from_brushes(&[room1, room2, portal], &cfg);
        let root = root_opt.expect("BSP root should exist");
        assert!(
            root.coplanar.iter().any(|p| (p.flags & crate::poly::PF_PORTAL) != 0),
            "With portal_bias=100, the root BSP splitting plane must be the ZonePortal plane"
        );
    }

    fn evaluate_csg_brushes_indexed_reference(
        brushes: &[crate::brush::Brush],
    ) -> Vec<(usize, Polygon)> {
        if brushes.is_empty() {
            return Vec::new();
        }
        let mut structural = Vec::new();
        for (idx, brush) in brushes.iter().enumerate() {
            if brush.polys.is_empty() {
                continue;
            }
            let (is_non_solid, is_semi_solid) = classify_brush_role(brush);
            if !is_non_solid && !is_semi_solid {
                let world_polys = brush.to_world_polys();
                if world_polys.is_empty() {
                    continue;
                }
                let aabb = brush.world_bounds();
                let is_convex = world_polys.iter().all(|f| {
                    world_polys
                        .iter()
                        .all(|other| other.vertices.iter().all(|&v| f.plane.dist_to_point(v) <= 0.5))
                });
                structural.push(PreparedStructuralBrush {
                    brush_idx: idx,
                    csg_oper: brush.csg_oper,
                    world_polys,
                    aabb,
                    is_convex,
                });
            }
        }
        let universe_starts_solid = structural
            .first()
            .map(|b| b.csg_oper == crate::CsgOperation::Subtract)
            .unwrap_or(false);
        let mut result = Vec::new();
        for (s_pos, sb) in structural.iter().enumerate() {
            let is_sub = sb.csg_oper == crate::CsgOperation::Subtract;
            let mut fragments: Vec<Polygon> = sb
                .world_polys
                .iter()
                .map(|p| if is_sub { p.inverted() } else { p.clone() })
                .collect();
            for (other_pos, other) in structural.iter().enumerate() {
                if other_pos == s_pos {
                    continue;
                }
                let tol = 0.5;
                if sb.aabb.1.x < other.aabb.0.x - tol
                    || sb.aabb.0.x > other.aabb.1.x + tol
                    || sb.aabb.1.y < other.aabb.0.y - tol
                    || sb.aabb.0.y > other.aabb.1.y + tol
                    || sb.aabb.1.z < other.aabb.0.z - tol
                    || sb.aabb.0.z > other.aabb.1.z + tol
                {
                    continue;
                }
                let mut next_frags = Vec::with_capacity(fragments.len());
                for frag in fragments {
                    next_frags.extend(partition_poly_by_brush(frag, other));
                }
                fragments = next_frags;
            }
            for frag in fragments {
                if frag.vertices.len() < 3 || poly_area_3d(&frag.vertices) < 0.05 {
                    continue;
                }
                let centroid =
                    frag.vertices.iter().copied().sum::<Vec3>() / (frag.vertices.len() as f32);
                let n = frag.plane.normal;
                let front_pt = centroid + n * 0.25;
                let back_pt = centroid - n * 0.25;
                let front_is_air = !is_point_solid_at(front_pt, &structural, universe_starts_solid);
                let back_is_solid = is_point_solid_at(back_pt, &structural, universe_starts_solid);
                if front_is_air && back_is_solid {
                    let mut overridden = false;
                    for later in &structural[(s_pos + 1)..] {
                        if !point_in_structural_brush(centroid, later) {
                            continue;
                        }
                        let later_sub = later.csg_oper == crate::CsgOperation::Subtract;
                        for lp in &later.world_polys {
                            let eff_n = if later_sub { -lp.plane.normal } else { lp.plane.normal };
                            let eff_d = if later_sub { -lp.plane.dist } else { lp.plane.dist };
                            if eff_n.dot(n) > 0.999 && (eff_d - frag.plane.dist).abs() < 0.25 {
                                overridden = true;
                                break;
                            }
                        }
                        if overridden {
                            break;
                        }
                    }
                    if !overridden {
                        result.push((sb.brush_idx, frag));
                    }
                }
            }
        }
        for (idx, brush) in brushes.iter().enumerate() {
            if brush.polys.is_empty() {
                continue;
            }
            let (is_non_solid, is_semi_solid) = classify_brush_role(brush);
            if is_non_solid {
                for poly in brush.to_world_polys() {
                    result.push((idx, poly));
                }
            } else if is_semi_solid {
                let prior_pos = structural.partition_point(|s| s.brush_idx < idx);
                let prior_structural = &structural[..prior_pos];
                let prior_starts_solid = prior_structural
                    .first()
                    .map(|b| b.csg_oper == crate::CsgOperation::Subtract)
                    .unwrap_or(false);
                let aabb = brush.world_bounds();
                let mut fragments = brush.to_world_polys();
                for other in prior_structural {
                    let tol = 0.5;
                    if aabb.1.x < other.aabb.0.x - tol
                        || aabb.0.x > other.aabb.1.x + tol
                        || aabb.1.y < other.aabb.0.y - tol
                        || aabb.0.y > other.aabb.1.y + tol
                        || aabb.1.z < other.aabb.0.z - tol
                        || aabb.0.z > other.aabb.1.z + tol
                    {
                        continue;
                    }
                    let mut next_frags = Vec::with_capacity(fragments.len());
                    for frag in fragments {
                        next_frags.extend(partition_poly_by_brush(frag, other));
                    }
                    fragments = next_frags;
                }
                for frag in fragments {
                    if frag.vertices.len() < 3 || poly_area_3d(&frag.vertices) < 0.05 {
                        continue;
                    }
                    let centroid =
                        frag.vertices.iter().copied().sum::<Vec3>() / (frag.vertices.len() as f32);
                    let front_pt = centroid + frag.plane.normal * 0.25;
                    if !is_point_solid_at(front_pt, prior_structural, prior_starts_solid) {
                        result.push((idx, frag));
                    }
                }
            }
        }
        result
    }

    fn build_rich_csg_test_scene() -> Vec<crate::brush::Brush> {
        let mut brushes = Vec::new();
        // Touching Subtract rooms with coplanar shared faces and doorway cutouts
        let mut r0 = build_cube("SubRoom0", CsgOperation::Subtract, Vec3::new(1024.0, 1024.0, 512.0));
        r0.location = Vec3::ZERO;
        brushes.push(r0);

        let mut r1 = build_cube("SubRoom1", CsgOperation::Subtract, Vec3::new(1024.0, 1024.0, 512.0));
        r1.location = Vec3::new(1024.0, 0.0, 0.0);
        brushes.push(r1);

        // Add walls carved by a later Subtract doorway, plus coplanar overlapping structural Add
        let mut wall = build_cube("AddWall", CsgOperation::Add, Vec3::new(64.0, 512.0, 256.0));
        wall.location = Vec3::new(256.0, 0.0, -128.0);
        brushes.push(wall);

        let mut door = build_cube("DoorCut", CsgOperation::Subtract, Vec3::new(128.0, 96.0, 160.0));
        door.location = Vec3::new(256.0, 0.0, -176.0);
        brushes.push(door);

        let mut overlap = build_cube("CoplanarOverlap", CsgOperation::Add, Vec3::new(64.0, 256.0, 256.0));
        overlap.location = Vec3::new(256.0, 128.0, -128.0);
        for p in &mut overlap.polys {
            p.texture = "CustomTex".to_string();
        }
        brushes.push(overlap);

        // Concave brush built from stairs
        let mut stairs = crate::brush::build_staircase("ConcaveStairs", CsgOperation::Add, 3, 16.0, 32.0, 96.0);
        stairs.location = Vec3::new(-256.0, -200.0, -256.0);
        brushes.push(stairs);

        // SemiSolid and NonSolid sheet portal
        let mut semi = crate::brush::build_cylinder("SemiPillar", CsgOperation::Add, 192.0, 32.0, 8);
        semi.location = Vec3::new(128.0, 180.0, 0.0);
        for p in &mut semi.polys {
            p.flags |= crate::poly::PF_SEMISOLID;
        }
        brushes.push(semi);

        let mut portal = crate::brush::build_sheet("ZoneSheet", CsgOperation::Add, 512.0, 256.0);
        portal.location = Vec3::new(512.0, 0.0, 0.0);
        for p in &mut portal.polys {
            p.flags |= crate::poly::PF_PORTAL | crate::poly::PF_NOT_SOLID;
        }
        brushes.push(portal);

        // Populate 40+ structural Add/Subtract/SemiSolid brushes across multiple spatial clusters
        for i in 0..42 {
            let x = (i % 6) as f32 * 380.0 - 900.0;
            let y = (i / 6) as f32 * 320.0 - 900.0;
            if i % 3 == 0 {
                let mut sub = build_cube(format!("ClusterSub_{i}"), CsgOperation::Subtract, Vec3::new(256.0, 256.0, 192.0));
                sub.location = Vec3::new(x, y, 0.0);
                brushes.push(sub);
            } else {
                let mut add = build_cube(format!("ClusterAdd_{i}"), CsgOperation::Add, Vec3::new(96.0, 96.0, 128.0));
                add.location = Vec3::new(x + 32.0, y, -32.0);
                if i % 5 == 0 {
                    for p in &mut add.polys {
                        p.flags |= crate::poly::PF_SEMISOLID;
                    }
                }
                brushes.push(add);
            }
        }
        brushes
    }

    #[test]
    fn test_csg_bvh_and_parallel_match_legacy_reference() {
        let brushes = build_rich_csg_test_scene();
        let expected = evaluate_csg_brushes_indexed_reference(&brushes);
        assert!(expected.len() > 20, "expected non-trivial CSG output");

        for &threads in &[1usize, 2, default_csg_worker_threads()] {
            let actual = evaluate_csg_brushes_indexed_with_options(&brushes, threads, None)
                .expect("uncancelled evaluation must succeed");
            assert_eq!(actual.len(), expected.len(), "poly count mismatch at threads={threads}");
            for (k, ((eb, ep), (ab, ap))) in expected.iter().zip(actual.iter()).enumerate() {
                assert_eq!(ab, eb, "brush index mismatch at poly {k} threads={threads}");
                assert_eq!(ap.vertices, ep.vertices, "vertex mismatch at poly {k} threads={threads}");
                assert_eq!(ap.plane, ep.plane, "plane mismatch at poly {k} threads={threads}");
                assert_eq!(ap.flags, ep.flags, "flags mismatch at poly {k} threads={threads}");
                assert_eq!(ap.texture, ep.texture, "texture mismatch at poly {k} threads={threads}");
            }
        }
    }

    #[test]
    fn test_csg_cancellable_aborts_and_completes() {
        let brushes = build_rich_csg_test_scene();
        let cancel_true = AtomicBool::new(true);
        assert!(
            evaluate_csg_brushes_indexed_cancellable(&brushes, &cancel_true).is_none(),
            "pre-cancelled CSG evaluation must return None"
        );

        let cancel_false = AtomicBool::new(false);
        let uncancelled = evaluate_csg_brushes_indexed_cancellable(&brushes, &cancel_false)
            .expect("uncancelled CSG must return Some");
        assert_eq!(uncancelled, evaluate_csg_brushes_indexed(&brushes));

        // Larger synthetic set to exercise mid-flight cooperative abort across threads
        let mut large = build_rich_csg_test_scene();
        for i in 0..80 {
            let mut b = build_cube(format!("MidCancel_{i}"), CsgOperation::Add, Vec3::new(160.0, 160.0, 160.0));
            b.location = Vec3::new((i % 8) as f32 * 96.0, (i / 8) as f32 * 96.0, 0.0);
            large.push(b);
        }
        let flag = std::sync::Arc::new(AtomicBool::new(false));
        let flag_setter = flag.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1));
            flag_setter.store(true, Ordering::Relaxed);
        });
        let res = evaluate_csg_brushes_indexed_cancellable(&large, &flag);
        handle.join().unwrap();
        // Either aborted mid-flight (None) or completed before wake (Some); both must match if Some.
        if let Some(polys) = res {
            assert_eq!(polys, evaluate_csg_brushes_indexed(&large));
        }
    }
}

