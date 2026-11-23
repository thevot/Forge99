use glam::Vec3;
use crate::brush::Brush;
use crate::poly::Polygon;
use crate::PF_PORTAL;

/// Information about a detected zone volume leak where fluid or zone effects escape into world space.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneLeak {
    pub zone_name: String,
    pub zone_location: Vec3,
    pub leak_location: Vec3,
    pub leak_distance: f32,
    pub escape_direction: Vec3,
    pub description: String,
}

/// Boundary status and seal audit for a specific ZoneInfo or WaterZone actor
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneSealReport {
    pub zone_name: String,
    pub class_name: String,
    pub location: Vec3,
    pub is_watertight: bool,
    pub portal_surfaces_count: usize,
    pub bounding_volume_uu: f32,
    pub detected_leaks: Vec<ZoneLeak>,
    pub recommended_action: String,
}

/// Configuration options for the leak detection algorithm
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneLeakConfig {
    pub ray_samples: usize,
    pub max_zone_radius_uu: f32,
    pub step_size_uu: f32,
}

impl Default for ZoneLeakConfig {
    fn default() -> Self {
        Self {
            ray_samples: 32,
            max_zone_radius_uu: 3000.0,
            step_size_uu: 64.0,
        }
    }
}

/// Evaluates whether a zone origin point is safely contained by surrounding geometry and portals,
/// or if it leaks out into open/unbounded space using precomputed brush world AABBs.
pub fn test_zone_watertightness_with_bounds(
    zone_name: &str,
    class_name: &str,
    zone_loc: Vec3,
    brush_bounds: &[(Vec3, Vec3)],
    portal_polys: &[Polygon],
    config: &ZoneLeakConfig,
) -> ZoneSealReport {
    let mut leaks = Vec::new();
    let mut portal_count = 0;

    // 1. Count portal surfaces within the zone's operational sphere
    for poly in portal_polys {
        if (poly.flags & PF_PORTAL) != 0 {
            let center = poly.vertices.iter().copied().fold(Vec3::ZERO, |a, b| a + b)
                / (poly.vertices.len().max(1) as f32);
            if center.distance(zone_loc) <= config.max_zone_radius_uu {
                portal_count += 1;
            }
        }
    }

    let reach = Vec3::splat(config.max_zone_radius_uu + config.step_size_uu);
    let q_min = zone_loc - reach;
    let q_max = zone_loc + reach;
    let local_bounds: Vec<(Vec3, Vec3)> = brush_bounds
        .iter()
        .copied()
        .filter(|&(b_min, b_max)| {
            b_max.x >= q_min.x
                && b_min.x <= q_max.x
                && b_max.y >= q_min.y
                && b_min.y <= q_max.y
                && b_max.z >= q_min.z
                && b_min.z <= q_max.z
        })
        .collect();

    // 2. Fibonacci sphere sampling to test containment in all 3D spherical directions
    let num_samples = config.ray_samples.clamp(8, 128);
    let phi = std::f32::consts::PI * (3.0 - (5.0_f32).sqrt()); // Golden angle in radians

    for i in 0..num_samples {
        let y = 1.0 - (i as f32 / (num_samples - 1).max(1) as f32) * 2.0;
        let radius = (1.0 - y * y).max(0.0).sqrt();
        let theta = phi * (i as f32);

        let dir = Vec3::new(theta.cos() * radius, y, theta.sin() * radius).normalize();

        // Trace ray outward from zone_loc
        let mut hit_solid = false;
        let mut hit_portal = false;
        let mut current_pos = zone_loc;
        let mut traveled = 0.0;

        while traveled < config.max_zone_radius_uu {
            current_pos += dir * config.step_size_uu;
            traveled += config.step_size_uu;

            // Check portal interception
            for poly in portal_polys {
                if (poly.flags & PF_PORTAL) != 0 && poly.plane.dist_to_point(current_pos).abs() < config.step_size_uu {
                    hit_portal = true;
                    break;
                }
            }
            if hit_portal {
                break;
            }

            // Check solid brush boundaries
            for &(min, max) in &local_bounds {
                if current_pos.x >= min.x && current_pos.x <= max.x
                    && current_pos.y >= min.y && current_pos.y <= max.y
                    && current_pos.z >= min.z && current_pos.z <= max.z
                {
                    hit_solid = true;
                    break;
                }
            }
            if hit_solid {
                break;
            }
        }

        // If the ray traveled to max radius without meeting either a portal seal or a solid boundary,
        // it has escaped through an unsealed geometry boundary
        if !hit_portal && !hit_solid && traveled >= config.max_zone_radius_uu {
            // Register unique leak
            let leak_pos = zone_loc + dir * (config.max_zone_radius_uu * 0.75);
            let already_reported = leaks.iter().any(|l: &ZoneLeak| l.leak_location.distance(leak_pos) < 250.0);
            if !already_reported {
                leaks.push(ZoneLeak {
                    zone_name: zone_name.to_string(),
                    zone_location: zone_loc,
                    leak_location: leak_pos,
                    leak_distance: traveled,
                    escape_direction: dir,
                    description: format!(
                        "Uncontained fluid/air leak along ray ({:.2}, {:.2}, {:.2}) escaping beyond {:.0} uu",
                        dir.x, dir.y, dir.z, traveled
                    ),
                });
            }
        }
    }

    let is_watertight = leaks.is_empty();
    let action = if is_watertight {
        "Zone is completely sealed. No boundary leakage detected.".to_string()
    } else {
        format!(
            "CRITICAL: {} unsealed openings detected. Place a 2D Sheet with PF_PORTAL across doorways or close brush gaps.",
            leaks.len()
        )
    };

    ZoneSealReport {
        zone_name: zone_name.to_string(),
        class_name: class_name.to_string(),
        location: zone_loc,
        is_watertight,
        portal_surfaces_count: portal_count,
        bounding_volume_uu: (config.max_zone_radius_uu * 2.0).powi(3),
        detected_leaks: leaks,
        recommended_action: action,
    }
}

/// Evaluates whether a zone origin point is safely contained by surrounding geometry and portals,
/// or if it leaks out into open/unbounded space.
pub fn test_zone_watertightness(
    zone_name: &str,
    class_name: &str,
    zone_loc: Vec3,
    brushes: &[Brush],
    portal_polys: &[Polygon],
    config: &ZoneLeakConfig,
) -> ZoneSealReport {
    let brush_bounds: Vec<(Vec3, Vec3)> = brushes.iter().map(|b| b.world_bounds()).collect();
    test_zone_watertightness_with_bounds(
        zone_name,
        class_name,
        zone_loc,
        &brush_bounds,
        portal_polys,
        config,
    )
}

/// A 3D shortest-path trace between two `ZoneInfo` actors that leaked into the same BSP zone (`BL-P3-03`).
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneLeakPath {
    pub zone_a_name: String,
    pub zone_b_name: String,
    pub leak_gap_point: Vec3,
    pub waypoints: Vec<Vec3>,
}

/// Result of the 3D BSP Zone Flood-Fill across `PF_PORTAL` surfaces (`BL-P2-02` & `BL-P3-03`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ZoneFloodFillResult {
    /// For each post-CSG polygon index, `(front_zone_id, back_zone_id)` in `0..64`.
    pub poly_zones: Vec<(usize, usize)>,
    /// Assigned zone ID (`1..63`) for each `ZoneInfo` actor name. If two zones leak together, they collapse to the same ID.
    pub actor_zone_ids: std::collections::HashMap<String, usize>,
    /// Number of distinct sealed zones discovered.
    pub distinct_zone_count: usize,
    /// 3D leak paths between `ZoneInfo` actors whose volumes are not hermetically separated by `PF_PORTAL` sheets.
    pub leak_paths: Vec<ZoneLeakPath>,
    /// 64-entry zone connectivity bitmask across `PF_PORTAL` sheets.
    pub connectivity_mask: Vec<u64>,
}

/// Checks if `pt` lies inside the 3D convex polygon `poly` (projected onto its plane) with `margin` tolerance.
/// Positive `margin` expands the polygon slightly; negative `margin` shrinks it.
fn point_in_convex_poly_3d(pt: Vec3, poly: &Polygon, margin: f32) -> bool {
    let n = poly.vertices.len();
    if n < 3 {
        return false;
    }
    for i in 0..n {
        let v0 = poly.vertices[i];
        let v1 = poly.vertices[(i + 1) % n];
        let edge = v1 - v0;
        let len = edge.length();
        if len < 1e-4 {
            continue;
        }
        let inward = poly.plane.normal.cross(edge / len);
        let signed_dist = (pt - v0).dot(inward);
        if signed_dist < -margin {
            return false;
        }
    }
    true
}

/// Returns `Some(hit_point)` if the 3D segment `a -> b` crosses `poly` within `margin`.
fn segment_hits_poly(a: Vec3, b: Vec3, poly: &Polygon, margin: f32) -> Option<Vec3> {
    let da = poly.plane.dist_to_point(a);
    let db = poly.plane.dist_to_point(b);
    if (da > 0.05 && db > 0.05) || (da < -0.05 && db < -0.05) {
        return None;
    }
    let denom = da - db;
    let hit = if denom.abs() > 1e-5 {
        let t = (da / denom).clamp(0.0, 1.0);
        a + (b - a) * t
    } else {
        (a + b) * 0.5
    };
    if point_in_convex_poly_3d(hit, poly, margin) {
        Some(hit)
    } else {
        None
    }
}

/// Returns true if a 3D point `pt` is inside carved playable air space (not inside solid rock).
fn is_point_in_playable_air(pt: Vec3, brushes: &[Brush]) -> bool {
    if brushes.is_empty() {
        return false;
    }
    let first_is_sub = brushes
        .iter()
        .find(|b| {
            b.polys
                .iter()
                .all(|p| (p.flags & (PF_PORTAL | crate::poly::PF_NOT_SOLID)) == 0)
        })
        .map(|b| b.csg_oper == crate::CsgOperation::Subtract)
        .unwrap_or(false);
    let mut is_solid = first_is_sub;

    for b in brushes {
        let is_non_solid = b
            .polys
            .iter()
            .any(|p| (p.flags & (PF_PORTAL | crate::poly::PF_NOT_SOLID)) != 0);
        if is_non_solid || b.polys.len() < 4 {
            continue;
        }
        let wpolys = b.to_world_polys();
        let inside_brush = wpolys.iter().all(|p| p.plane.dist_to_point(pt) <= 0.1);
        if inside_brush {
            match b.csg_oper {
                crate::CsgOperation::Subtract => is_solid = false,
                crate::CsgOperation::Add => is_solid = true,
                _ => {}
            }
        }
    }
    !is_solid
}

/// Performs a 3D BSP Zone Flood-Fill across `PF_PORTAL` surfaces (`BL-P2-02`) and traces the shortest
/// 3D path (`ZoneLeakPath`, `BL-P3-03`) through any unsealed doorway or 1-UU portal gap.
pub fn compute_bsp_zone_flood_fill(
    brushes: &[Brush],
    zone_actors: &[(String, Vec3)],
) -> ZoneFloodFillResult {
    use std::collections::{HashMap, VecDeque};

    let csg_polys = crate::bsp::evaluate_csg_brushes(brushes);
    let mut solid_polys = Vec::new();
    let mut portal_polys = Vec::new();

    for p in &csg_polys {
        if (p.flags & PF_PORTAL) != 0 {
            portal_polys.push(p.clone());
        } else if (p.flags & crate::poly::PF_NOT_SOLID) == 0 {
            solid_polys.push(p.clone());
        }
    }
    for b in brushes {
        for wp in b.to_world_polys() {
            if (wp.flags & PF_PORTAL) != 0 {
                portal_polys.push(wp);
            }
        }
    }

    // 1. Build 3D navigation/flood-fill nodes:
    //    - Nodes 0..zone_actors.len(): exact ZoneInfo actor positions
    //    - Interior centroids of carved rooms/brushes
    //    - Gap-probe points around every PF_PORTAL edge (detects even a 1 UU gap!)
    let mut nodes: Vec<Vec3> = Vec::new();
    for (_, loc) in zone_actors {
        nodes.push(*loc);
    }
    for b in brushes {
        if b.csg_oper == crate::CsgOperation::Subtract {
            let (min, max) = b.world_bounds();
            let center = (min + max) * 0.5;
            if is_point_in_playable_air(center, brushes) {
                nodes.push(center);
            }
        }
    }

    // Probe around every portal edge for unsealed perimeter gaps (0.5 UU, 2.0 UU, 16.0 UU outside edge)
    let mut gap_probe_indices: Vec<usize> = Vec::new();
    for portal in &portal_polys {
        let n = portal.vertices.len();
        if n < 3 {
            continue;
        }
        let p_norm = portal.plane.normal;
        for i in 0..n {
            let v0 = portal.vertices[i];
            let v1 = portal.vertices[(i + 1) % n];
            let edge_dir = (v1 - v0).normalize_or_zero();
            if edge_dir == Vec3::ZERO {
                continue;
            }
            let outward = edge_dir.cross(p_norm).normalize_or_zero();
            let mid = (v0 + v1) * 0.5;
            for delta in [0.5_f32, 2.0, 12.0] {
                let gap_pt = mid + outward * delta;
                // Only add if gap_pt is outside the portal polygon AND inside playable air!
                if !point_in_convex_poly_3d(gap_pt, portal, 0.0)
                    && is_point_in_playable_air(gap_pt, brushes)
                {
                    let side_a = gap_pt - p_norm * 16.0;
                    let side_b = gap_pt + p_norm * 16.0;
                    if is_point_in_playable_air(side_a, brushes)
                        && is_point_in_playable_air(side_b, brushes)
                    {
                        nodes.push(side_a);
                        let gap_idx = nodes.len();
                        nodes.push(gap_pt);
                        gap_probe_indices.push(gap_idx);
                        nodes.push(side_b);
                    }
                }
            }
        }
    }

    // Helper: can flood-fill transition directly between `a` and `b` without hitting a solid wall or a portal?
    let can_traverse_unsealed = |a: Vec3, b: Vec3| -> bool {
        if (a - b).length_squared() < 1e-4 {
            return true;
        }
        for sp in &solid_polys {
            if segment_hits_poly(a, b, sp, 0.25).is_some() {
                return false;
            }
        }
        for pp in &portal_polys {
            if segment_hits_poly(a, b, pp, 0.0).is_some() {
                return false;
            }
        }
        true
    };

    let n_nodes = nodes.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n_nodes];
    for i in 0..n_nodes {
        for j in (i + 1)..n_nodes {
            if can_traverse_unsealed(nodes[i], nodes[j]) {
                adj[i].push(j);
                adj[j].push(i);
            }
        }
    }

    // 2. BFS Connected Components from ZoneInfo seeds
    let mut node_zone = vec![0usize; n_nodes];
    let mut actor_zone_ids = HashMap::new();
    let mut leak_paths = Vec::new();
    let mut next_zone_id = 1usize;

    for z_idx in 0..zone_actors.len() {
        if node_zone[z_idx] != 0 {
            // Already reached from an earlier ZoneInfo -> LEAK between earlier ZoneInfo and this one!
            let assigned = node_zone[z_idx];
            actor_zone_ids.insert(zone_actors[z_idx].0.clone(), assigned);
            continue;
        }
        let z_id = next_zone_id.min(63);
        next_zone_id += 1;
        node_zone[z_idx] = z_id;
        actor_zone_ids.insert(zone_actors[z_idx].0.clone(), z_id);

        let mut parent: HashMap<usize, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        queue.push_back(z_idx);

        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                if node_zone[v] == 0 {
                    node_zone[v] = z_id;
                    parent.insert(v, u);
                    queue.push_back(v);

                    // If `v` is another ZoneInfo seed (`v < zone_actors.len()`), record the 3D leak path!
                    if v < zone_actors.len() && v != z_idx {
                        let mut path_indices = vec![v];
                        let mut cur = v;
                        while let Some(&p) = parent.get(&cur) {
                            path_indices.push(p);
                            if p == z_idx {
                                break;
                            }
                            cur = p;
                        }
                        path_indices.reverse();
                        let waypoints: Vec<Vec3> =
                            path_indices.iter().map(|&idx| nodes[idx]).collect();
                        let leak_gap_point = path_indices
                            .iter()
                            .find(|&&idx| gap_probe_indices.contains(&idx))
                            .map(|&idx| nodes[idx])
                            .unwrap_or_else(|| (zone_actors[z_idx].1 + zone_actors[v].1) * 0.5);
                        leak_paths.push(ZoneLeakPath {
                            zone_a_name: zone_actors[z_idx].0.clone(),
                            zone_b_name: zone_actors[v].0.clone(),
                            leak_gap_point,
                            waypoints,
                        });
                    }
                }
            }
        }
    }

    let mut distinct_zones: Vec<usize> = actor_zone_ids.values().copied().collect();
    distinct_zones.sort_unstable();
    distinct_zones.dedup();
    let distinct_zone_count = if distinct_zones.is_empty() {
        1
    } else {
        distinct_zones.len()
    };

    // 3. Assign (front_zone, back_zone) to each post-CSG polygon and compute portal connectivity
    let mut connectivity_mask = vec![0u64; 64];
    for z in 0..64 {
        connectivity_mask[z] = 1u64 << z;
    }

    let resolve_point_zone = |pt: Vec3| -> usize {
        for (i, &n_pos) in nodes.iter().enumerate() {
            if node_zone[i] > 0 && can_traverse_unsealed(pt, n_pos) {
                return node_zone[i];
            }
        }
        // Fallback nearest seeded node
        let mut best_z = 1usize;
        let mut best_d = f32::MAX;
        for (i, &n_pos) in nodes.iter().enumerate() {
            if node_zone[i] > 0 {
                let d = pt.distance_squared(n_pos);
                if d < best_d {
                    best_d = d;
                    best_z = node_zone[i];
                }
            }
        }
        best_z
    };

    let mut poly_zones = Vec::with_capacity(csg_polys.len());
    for p in &csg_polys {
        let centroid = if p.vertices.is_empty() {
            Vec3::ZERO
        } else {
            p.vertices.iter().copied().sum::<Vec3>() / (p.vertices.len() as f32)
        };
        let front_pt = centroid + p.plane.normal * 8.0;
        let back_pt = centroid - p.plane.normal * 8.0;
        let f_z = resolve_point_zone(front_pt);
        let b_z = if (p.flags & PF_PORTAL) != 0 {
            let bz = resolve_point_zone(back_pt);
            if f_z < 64 && bz < 64 {
                connectivity_mask[f_z] |= 1u64 << bz;
                connectivity_mask[bz] |= 1u64 << f_z;
            }
            bz
        } else {
            f_z
        };
        poly_zones.push((f_z, b_z));
    }

    ZoneFloodFillResult {
        poly_zones,
        actor_zone_ids,
        distinct_zone_count,
        leak_paths,
        connectivity_mask,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::{build_cube, build_sheet};
    use crate::CsgOperation;

    #[test]
    fn test_sealed_zone_inside_box() {
        let room_brush = build_cube("Room", CsgOperation::Add, Vec3::new(1000.0, 1000.0, 1000.0));
        let brushes = vec![room_brush];
        let portal_polys = vec![];
        let config = ZoneLeakConfig {
            ray_samples: 16,
            max_zone_radius_uu: 400.0,
            step_size_uu: 32.0,
        };

        let report = test_zone_watertightness(
            "WaterZone_0",
            "WaterZone",
            Vec3::new(0.0, 0.0, 0.0),
            &brushes,
            &portal_polys,
            &config,
        );

        assert!(report.is_watertight);
        assert!(report.detected_leaks.is_empty());
    }

    #[test]
    fn test_unsealed_zone_leak_detection() {
        // Open world, no brushes or portals
        let brushes = vec![];
        let portal_polys = vec![];
        let config = ZoneLeakConfig {
            ray_samples: 16,
            max_zone_radius_uu: 500.0,
            step_size_uu: 50.0,
        };

        let report = test_zone_watertightness(
            "LavaZone_1",
            "LavaZone",
            Vec3::new(100.0, 200.0, 50.0),
            &brushes,
            &portal_polys,
            &config,
        );

        assert!(!report.is_watertight);
        assert!(!report.detected_leaks.is_empty());
        assert!(report.recommended_action.contains("CRITICAL"));
    }

    #[test]
    fn test_zone_portal_seal_and_1uu_leak_detection() {
        // Two touching Subtract rooms along X: Room1 [-512..0], Room2 [0..512], width Y=256, height Z=256
        let mut room1 = build_cube("Room1", CsgOperation::Subtract, Vec3::new(512.0, 256.0, 256.0));
        room1.location = Vec3::new(-256.0, 0.0, 0.0);
        let mut room2 = build_cube("Room2", CsgOperation::Subtract, Vec3::new(512.0, 256.0, 256.0));
        room2.location = Vec3::new(256.0, 0.0, 0.0);

        // Build a vertical portal sheet at X = 0 that hermetically seals Y in [-128..128], Z in [-128..128]
        let mut sealed_portal = build_sheet("PortalSeal", CsgOperation::Add, 256.0, 256.0);
        // Rotate sheet so its normal points along +X (EulerRot::YXZ uses rotation.y around Y axis)
        sealed_portal.rotation = Vec3::new(0.0, 90.0, 0.0);
        for p in &mut sealed_portal.polys {
            p.flags |= PF_PORTAL | crate::poly::PF_NOT_SOLID;
        }

        let zones = vec![
            ("ZoneInfo0".to_string(), Vec3::new(-256.0, 0.0, 0.0)),
            ("ZoneInfo1".to_string(), Vec3::new(256.0, 0.0, 0.0)),
        ];

        let sealed_res = compute_bsp_zone_flood_fill(
            &[room1.clone(), room2.clone(), sealed_portal],
            &zones,
        );
        assert_eq!(
            sealed_res.distinct_zone_count, 2,
            "Hermetically sealed ZonePortal must separate Room1 and Room2 into 2 distinct zones"
        );
        assert!(
            sealed_res.leak_paths.is_empty(),
            "Hermetically sealed ZonePortal must have 0 leak paths"
        );
        assert_ne!(
            sealed_res.actor_zone_ids["ZoneInfo0"],
            sealed_res.actor_zone_ids["ZoneInfo1"]
        );

        // Now shrink the portal width by 2 UU (254.0 instead of 256.0 -> 1 UU gap on each side!)
        let mut leaky_portal = build_sheet("PortalLeaky", CsgOperation::Add, 254.0, 256.0);
        leaky_portal.rotation = Vec3::new(0.0, 90.0, 0.0);
        for p in &mut leaky_portal.polys {
            p.flags |= PF_PORTAL | crate::poly::PF_NOT_SOLID;
        }

        let leaky_res =
            compute_bsp_zone_flood_fill(&[room1, room2, leaky_portal], &zones);
        assert_eq!(
            leaky_res.distinct_zone_count, 1,
            "A 1-UU gap around the ZonePortal must cause the two zones to merge into 1 zone"
        );
        assert_eq!(
            leaky_res.actor_zone_ids["ZoneInfo0"],
            leaky_res.actor_zone_ids["ZoneInfo1"]
        );
        assert!(
            !leaky_res.leak_paths.is_empty(),
            "Must produce a 3D ZoneLeakPath tracing through the 1-UU gap"
        );
        assert!(
            leaky_res.leak_paths[0].waypoints.len() >= 3,
            "3D leak path must include start ZoneInfo, gap waypoint, and end ZoneInfo"
        );
    }

    #[test]
    fn test_zone_watertightness_with_bounds_matches_sealed_and_leaky_cases() {
        let mut far_brush = build_cube("FarNoise", CsgOperation::Add, Vec3::splat(128.0));
        far_brush.location = Vec3::new(20_000.0, 20_000.0, 0.0);
        let cfg = ZoneLeakConfig::default();

        // Sealed enclosure around origin
        let sealed = vec![build_cube("SealedRoom", CsgOperation::Subtract, Vec3::splat(512.0)), far_brush.clone()];
        let sealed_bounds: Vec<_> = sealed.iter().map(|b| b.world_bounds()).collect();
        let rep_sealed_a = test_zone_watertightness("Z0", "ZoneInfo", Vec3::ZERO, &sealed, &[], &cfg);
        let rep_sealed_b = test_zone_watertightness_with_bounds("Z0", "ZoneInfo", Vec3::ZERO, &sealed_bounds, &[], &cfg);
        assert_eq!(rep_sealed_a, rep_sealed_b);
        assert!(rep_sealed_b.is_watertight);

        // Open/leaky scene (only one floor slab + far out-of-range brush)
        let mut floor = build_cube("Floor", CsgOperation::Add, Vec3::new(512.0, 512.0, 32.0));
        floor.location = Vec3::new(0.0, 0.0, -128.0);
        let leaky = vec![floor, far_brush];
        let leaky_bounds: Vec<_> = leaky.iter().map(|b| b.world_bounds()).collect();
        let rep_leaky_a = test_zone_watertightness("Z1", "ZoneInfo", Vec3::ZERO, &leaky, &[], &cfg);
        let rep_leaky_b = test_zone_watertightness_with_bounds("Z1", "ZoneInfo", Vec3::ZERO, &leaky_bounds, &[], &cfg);
        assert_eq!(rep_leaky_a, rep_leaky_b);
        assert!(!rep_leaky_b.is_watertight);
    }
}

