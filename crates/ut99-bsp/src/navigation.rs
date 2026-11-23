use glam::Vec3;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

pub const R_WALK: u32 = 1;
pub const R_FLY: u32 = 2;
pub const R_SWIM: u32 = 4;
pub const R_JUMP: u32 = 8;
pub const R_DOOR: u32 = 16;
pub const R_SPECIAL: u32 = 32;

/// A Navigation Node in the level's bot path network
#[derive(Debug, Clone, PartialEq)]
pub struct NavNode {
    pub id: usize,
    pub name: String,
    pub location: Vec3,
    pub node_type: String, // "PathNode", "PlayerStart", "InventorySpot", "LiftCenter", "LiftExit"
    pub radius: f32,
}

/// A directed reachability edge between two navigation nodes
#[derive(Debug, Clone, PartialEq)]
pub struct NavReachSpec {
    pub from: usize,
    pub to: usize,
    pub distance: f32,
    pub reach_flags: u32,
}

/// The Navigation Graph for the level
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NavGraph {
    pub nodes: Vec<NavNode>,
    pub reach_specs: Vec<NavReachSpec>,
    pub adjacency: HashMap<usize, Vec<usize>>, // node_id -> indices into reach_specs
}

impl NavGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a navigation node to the graph and returns its index
    pub fn add_node(&mut self, name: impl Into<String>, location: Vec3, node_type: impl Into<String>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(NavNode {
            id,
            name: name.into(),
            location,
            node_type: node_type.into(),
            radius: 32.0,
        });
        id
    }

    /// Adds a directed reachability specification between two nodes
    pub fn add_reach_spec(&mut self, from: usize, to: usize, reach_flags: u32) {
        if from >= self.nodes.len() || to >= self.nodes.len() || from == to {
            return;
        }

        let p1 = self.nodes[from].location;
        let p2 = self.nodes[to].location;
        let distance = p1.distance(p2);

        let spec_idx = self.reach_specs.len();
        self.reach_specs.push(NavReachSpec {
            from,
            to,
            distance,
            reach_flags,
        });

        self.adjacency.entry(from).or_default().push(spec_idx);
    }

    /// Finds the index of the node closest to the given 3D position
    pub fn find_nearest_node(&self, pos: Vec3) -> Option<usize> {
        let mut best_idx = None;
        let mut min_dist_sq = f32::MAX;

        for (idx, node) in self.nodes.iter().enumerate() {
            let dist_sq = node.location.distance_squared(pos);
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                best_idx = Some(idx);
            }
        }

        best_idx
    }

    /// Automatically constructs reachability links between all nodes in the graph
    /// based on maximum distance, walk/jump height limits, and slope constraints.
    pub fn build_paths(&mut self, max_distance: f32, max_step_height: f32, max_drop_height: f32) {
        self.reach_specs.clear();
        self.adjacency.clear();

        let n = self.nodes.len();
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }

                let p1 = self.nodes[i].location;
                let p2 = self.nodes[j].location;
                let dist = p1.distance(p2);

                if dist > max_distance {
                    continue;
                }

                let delta_z = p2.z - p1.z;

                // Reachability classification
                if delta_z > max_step_height {
                    // Cannot jump higher than max_step_height
                    continue;
                }

                if delta_z < -max_drop_height {
                    // Drop is too high for bot without taking fatal falling damage
                    continue;
                }

                let mut flags = R_WALK;
                if delta_z > 20.0 {
                    flags |= R_JUMP;
                }

                let is_lift = self.nodes[i].node_type.contains("Lift") || self.nodes[j].node_type.contains("Lift");
                if is_lift {
                    flags |= R_SPECIAL;
                }

                self.add_reach_spec(i, j, flags);
            }
        }
    }

    /// Finds the optimal shortest path between `start_node` and `target_node` using A* search.
    pub fn find_shortest_path(&self, start_node: usize, target_node: usize) -> Option<Vec<usize>> {
        if start_node >= self.nodes.len() || target_node >= self.nodes.len() {
            return None;
        }
        if start_node == target_node {
            return Some(vec![start_node]);
        }

        #[derive(Copy, Clone, PartialEq)]
        struct State {
            cost: f32,
            node: usize,
        }

        impl Eq for State {}
        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
            }
        }
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let target_pos = self.nodes[target_node].location;
        let mut dist: HashMap<usize, f32> = HashMap::new();
        let mut parent: HashMap<usize, usize> = HashMap::new();
        let mut heap = BinaryHeap::new();

        dist.insert(start_node, 0.0);
        heap.push(State {
            cost: self.nodes[start_node].location.distance(target_pos),
            node: start_node,
        });

        while let Some(State { node, .. }) = heap.pop() {
            if node == target_node {
                // Reconstruct path
                let mut path = vec![target_node];
                let mut curr = target_node;
                while let Some(&prev) = parent.get(&curr) {
                    path.push(prev);
                    curr = prev;
                }
                path.reverse();
                return Some(path);
            }

            let current_g = *dist.get(&node).unwrap_or(&f32::INFINITY);

            if let Some(spec_indices) = self.adjacency.get(&node) {
                for &spec_idx in spec_indices {
                    let spec = &self.reach_specs[spec_idx];
                    let next_node = spec.to;
                    let next_g = current_g + spec.distance;

                    if next_g < *dist.get(&next_node).unwrap_or(&f32::INFINITY) {
                        dist.insert(next_node, next_g);
                        parent.insert(next_node, node);
                        let h = self.nodes[next_node].location.distance(target_pos);
                        heap.push(State {
                            cost: next_g + h,
                            node: next_node,
                        });
                    }
                }
            }
        }

        None
    }
}

/// An interactive Bot Simulator Agent that traverses the navigation graph in real-time
#[derive(Debug, Clone, PartialEq)]
pub struct BotSimulator {
    pub name: String,
    pub location: Vec3,
    pub velocity: Vec3,
    pub speed: f32, // Canonical UT99 running speed ~400 uu/sec
    pub path: Vec<usize>,
    pub current_waypoint_idx: usize,
    pub reached_destination: bool,
    pub is_stuck: bool,
    pub stuck_timer: f32,
}

impl Default for BotSimulator {
    fn default() -> Self {
        Self {
            name: "TestBot_Xan".to_string(),
            location: Vec3::ZERO,
            velocity: Vec3::ZERO,
            speed: 400.0,
            path: Vec::new(),
            current_waypoint_idx: 0,
            reached_destination: false,
            is_stuck: false,
            stuck_timer: 0.0,
        }
    }
}

impl BotSimulator {
    pub fn new(name: impl Into<String>, start_pos: Vec3) -> Self {
        Self {
            name: name.into(),
            location: start_pos,
            ..Default::default()
        }
    }

    /// Sets a new path for the bot to follow
    pub fn set_path(&mut self, path: Vec<usize>, graph: &NavGraph) {
        self.path = path;
        self.current_waypoint_idx = 0;
        self.reached_destination = self.path.is_empty();
        self.is_stuck = false;
        self.stuck_timer = 0.0;

        if let Some(&first_node) = self.path.first() {
            if let Some(node) = graph.nodes.get(first_node) {
                // If close to first node, start heading to second
                if self.location.distance(node.location) < 32.0 && self.path.len() > 1 {
                    self.current_waypoint_idx = 1;
                }
            }
        }
    }

    /// Simulates one time step `dt` (in seconds) of bot movement
    pub fn update(&mut self, dt: f32, graph: &NavGraph) {
        if self.reached_destination || self.path.is_empty() {
            self.velocity = Vec3::ZERO;
            return;
        }

        if self.current_waypoint_idx >= self.path.len() {
            self.reached_destination = true;
            self.velocity = Vec3::ZERO;
            return;
        }

        let target_node_idx = self.path[self.current_waypoint_idx];
        let mut target_node = match graph.nodes.get(target_node_idx) {
            Some(n) => n,
            None => {
                self.is_stuck = true;
                return;
            }
        };

        let mut diff = target_node.location - self.location;
        let mut dist = diff.length();

        // Reached current waypoint?
        if dist <= 24.0 {
            self.current_waypoint_idx += 1;
            self.stuck_timer = 0.0;
            if self.current_waypoint_idx >= self.path.len() {
                self.reached_destination = true;
                self.velocity = Vec3::ZERO;
                return;
            }

            // Immediately switch steering target to next waypoint to prevent reversing
            target_node = match graph.nodes.get(self.path[self.current_waypoint_idx]) {
                Some(n) => n,
                None => {
                    self.is_stuck = true;
                    return;
                }
            };
            diff = target_node.location - self.location;
            dist = diff.length();
        }

        // Steer towards target node
        let dir = if dist > 1e-3 { diff / dist } else { Vec3::ZERO };
        let step_dist = self.speed * dt;
        let move_dist = step_dist.min(dist);
        self.velocity = dir * self.speed;

        let prev_loc = self.location;
        self.location += dir * move_dist;

        // Stuck detection: if moved less than 1.0 unit in 1.5 seconds while moving
        if self.location.distance(prev_loc) < 0.1 * dt {
            self.stuck_timer += dt;
            if self.stuck_timer > 1.5 {
                self.is_stuck = true;
            }
        } else {
            self.stuck_timer = 0.0;
        }
    }
}

/// Classification of navigation coverage quality at a given point in the arena
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavCoverageRating {
    /// Optimal coverage (< 400 uu): Bot can immediately acquire path without evasive maneuvers
    Optimal,
    /// Reachable coverage (400..800 uu): Bot requires dodge/jump or sustained sightline
    Reachable,
    /// Dead Zone (> 800 uu or no nodes): High risk of bot stalling, looping, or roaming failure
    DeadZone,
}

/// A spatial sample point evaluated for bot path coverage
#[derive(Debug, Clone, PartialEq)]
pub struct NavSamplePoint {
    pub position: Vec3,
    pub distance_to_nearest_node: f32,
    pub nearest_node_id: Option<usize>,
    pub rating: NavCoverageRating,
}

/// Comprehensive report of bot path network arena coverage
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NavCoverageReport {
    pub total_walkable_area_points: usize,
    pub optimal_count: usize,
    pub reachable_count: usize,
    pub dead_zone_count: usize,
    pub coverage_percentage: f32,
    pub samples: Vec<NavSamplePoint>,
}

impl NavGraph {
    /// Evaluates navigation coverage across a collection of walkable floor sample coordinates.
    pub fn analyze_coverage(&self, walkable_points: &[Vec3]) -> NavCoverageReport {
        if self.nodes.is_empty() || walkable_points.is_empty() {
            let samples: Vec<NavSamplePoint> = walkable_points
                .iter()
                .map(|&p| NavSamplePoint {
                    position: p,
                    distance_to_nearest_node: f32::INFINITY,
                    nearest_node_id: None,
                    rating: NavCoverageRating::DeadZone,
                })
                .collect();
            let total = samples.len();
            return NavCoverageReport {
                total_walkable_area_points: total,
                optimal_count: 0,
                reachable_count: 0,
                dead_zone_count: total,
                coverage_percentage: 0.0,
                samples,
            };
        }

        let mut optimal_count = 0;
        let mut reachable_count = 0;
        let mut dead_zone_count = 0;
        let mut samples = Vec::with_capacity(walkable_points.len());

        for &pt in walkable_points {
            let mut min_dist = f32::INFINITY;
            let mut nearest_id = None;

            for node in &self.nodes {
                let dist = pt.distance(node.location);
                if dist < min_dist {
                    min_dist = dist;
                    nearest_id = Some(node.id);
                }
            }

            let rating = if min_dist < 400.0 {
                optimal_count += 1;
                NavCoverageRating::Optimal
            } else if min_dist <= 800.0 {
                reachable_count += 1;
                NavCoverageRating::Reachable
            } else {
                dead_zone_count += 1;
                NavCoverageRating::DeadZone
            };

            samples.push(NavSamplePoint {
                position: pt,
                distance_to_nearest_node: min_dist,
                nearest_node_id: nearest_id,
                rating,
            });
        }

        let total = walkable_points.len();
        let covered = optimal_count + reachable_count;
        let coverage_percentage = if total > 0 {
            (covered as f32 / total as f32) * 100.0
        } else {
            100.0
        };

        NavCoverageReport {
            total_walkable_area_points: total,
            optimal_count,
            reachable_count,
            dead_zone_count,
            coverage_percentage,
            samples,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_graph_path_building() {
        let mut graph = NavGraph::new();

        // 3 linear nodes: Node0 -> Node1 -> Node2
        let n0 = graph.add_node("PathNode0", Vec3::new(0.0, 0.0, 0.0), "PathNode");
        let n1 = graph.add_node("PathNode1", Vec3::new(200.0, 0.0, 10.0), "PathNode");
        let n2 = graph.add_node("PathNode2", Vec3::new(400.0, 0.0, 20.0), "PathNode");
        // Isolated far node:
        let _far = graph.add_node("FarNode", Vec3::new(2000.0, 0.0, 0.0), "PathNode");

        graph.build_paths(300.0, 60.0, 300.0);

        assert!(!graph.reach_specs.is_empty());
        // Verify n0 connects to n1, n1 connects to n2
        assert!(graph.adjacency.get(&n0).is_some());
        assert!(graph.adjacency.get(&n1).is_some());

        // Test A* pathfinding
        let path = graph.find_shortest_path(n0, n2).expect("Should find path from n0 to n2");
        assert_eq!(path, vec![n0, n1, n2]);
    }

    #[test]
    fn test_bot_simulator_movement() {
        let mut graph = NavGraph::new();
        let n0 = graph.add_node("Start", Vec3::new(0.0, 0.0, 0.0), "PlayerStart");
        let _n1 = graph.add_node("Mid", Vec3::new(100.0, 0.0, 0.0), "PathNode");
        let n2 = graph.add_node("Goal", Vec3::new(200.0, 0.0, 0.0), "InventorySpot");

        graph.build_paths(150.0, 50.0, 200.0);
        let path = graph.find_shortest_path(n0, n2).unwrap();

        let mut bot = BotSimulator::new("TestBot", Vec3::new(0.0, 0.0, 0.0));
        bot.set_path(path, &graph);

        assert!(!bot.reached_destination);

        // Step simulation for 0.5s at 400 uu/sec -> moves 200 uu
        bot.update(0.3, &graph);
        assert!(bot.location.x > 50.0);

        bot.update(0.3, &graph);
        bot.update(0.3, &graph);

        assert!(bot.reached_destination, "Bot should reach destination");
        assert!((bot.location.x - 200.0).abs() <= 24.0);
    }

    #[test]
    fn test_nav_coverage_analyzer() {
        let mut graph = NavGraph::new();
        graph.add_node("PathNode0", Vec3::new(0.0, 0.0, 0.0), "PathNode");
        graph.add_node("PathNode1", Vec3::new(600.0, 0.0, 0.0), "PathNode");

        let walkable_points = vec![
            Vec3::new(50.0, 0.0, 0.0),   // dist = 50.0 -> Optimal (< 400)
            Vec3::new(500.0, 0.0, 0.0),  // dist = 100.0 from Node1 -> Optimal (< 400)
            Vec3::new(300.0, 0.0, 0.0),  // dist = 300.0 from both -> Optimal (< 400)
            Vec3::new(0.0, 500.0, 0.0),  // dist = 500.0 from Node0 -> Reachable (400..800)
            Vec3::new(2000.0, 0.0, 0.0), // dist = 1400.0 from Node1 -> DeadZone (> 800)
        ];

        let report = graph.analyze_coverage(&walkable_points);
        assert_eq!(report.total_walkable_area_points, 5);
        assert_eq!(report.optimal_count, 3);
        assert_eq!(report.reachable_count, 1);
        assert_eq!(report.dead_zone_count, 1);
        assert_eq!(report.coverage_percentage, 80.0);
        assert_eq!(report.samples[0].rating, NavCoverageRating::Optimal);
        assert_eq!(report.samples[3].rating, NavCoverageRating::Reachable);
        assert_eq!(report.samples[4].rating, NavCoverageRating::DeadZone);
    }

    #[test]
    fn test_auto_build_navigation_reachspecs() {
        let mut graph = NavGraph::new();
        // 3 linear nodes within walking distance
        let n0 = graph.add_node("NodeA", Vec3::new(0.0, 0.0, 0.0), "PathNode");
        let n1 = graph.add_node("NodeB", Vec3::new(400.0, 0.0, 20.0), "PathNode");
        // Jumpable ledge (+80 uu up, horiz 200 uu)
        let n2 = graph.add_node("NodeC", Vec3::new(600.0, 0.0, 100.0), "InventorySpot");
        // Isolated node 5000 uu away
        let _n3 = graph.add_node("NodeOrphan", Vec3::new(5000.0, 5000.0, 0.0), "PathNode");

        let report = auto_build_navigation_reachspecs(&mut graph, 1000.0, 54.0, 96.0);
        assert_eq!(report.total_nodes, 4);
        assert!(report.created_reachspecs >= 4);
        assert!(report.walk_reachspecs > 0);
        assert!(report.jump_reachspecs > 0);
        assert_eq!(report.isolated_nodes, vec!["NodeOrphan".to_string()]);

        // Verify shortest path works using auto-built reachspecs
        let path = graph.find_shortest_path(n0, n2);
        assert!(path.is_some());
        assert_eq!(path.unwrap(), vec![n0, n1, n2]);
    }
}

/// Diagnostic report on the reachspecs constructed by the auto-pathing builder
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutoPathReport {
    pub total_nodes: usize,
    pub created_reachspecs: usize,
    pub walk_reachspecs: usize,
    pub jump_reachspecs: usize,
    pub fly_reachspecs: usize,
    pub isolated_nodes: Vec<String>,
    pub max_step_height_uu: f32,
    pub max_link_distance_uu: f32,
}

/// Automatically computes and links ReachSpecs between nodes in the NavGraph
/// based on canonical UT99 locomotion parameters:
/// - Max Walk Step Height: 54.0 uu (R_WALK)
/// - Max Jump Height: 96.0 uu (R_JUMP)
/// - Max Link Distance: 1200.0 uu
pub fn auto_build_navigation_reachspecs(
    nav_graph: &mut NavGraph,
    max_distance_uu: f32,
    max_step_height: f32,
    max_jump_height: f32,
) -> AutoPathReport {
    auto_build_navigation_reachspecs_with_brushes(
        nav_graph,
        &[],
        max_distance_uu,
        max_step_height,
        max_jump_height,
    )
}

/// Checks whether a world-space point lies strictly inside a convex world brush (independent of inward/outward normal winding).
pub fn point_inside_brush(point: Vec3, brush: &crate::Brush) -> bool {
    if brush.polys.len() < 4 {
        return false;
    }
    let (min_b, max_b) = brush.world_bounds();
    if point.x < min_b.x - 0.1
        || point.x > max_b.x + 0.1
        || point.y < min_b.y - 0.1
        || point.y > max_b.y + 0.1
        || point.z < min_b.z - 0.1
        || point.z > max_b.z + 0.1
    {
        return false;
    }

    let world_polys = brush.to_world_polys();
    if world_polys.is_empty() {
        return false;
    }

    let mut sum = Vec3::ZERO;
    let mut cnt = 0usize;
    for p in &world_polys {
        for &v in &p.vertices {
            sum += v;
            cnt += 1;
        }
    }
    let center = if cnt > 0 {
        sum / (cnt as f32)
    } else {
        (min_b + max_b) * 0.5
    };

    for poly in &world_polys {
        if poly.vertices.is_empty() {
            continue;
        }
        let v0 = poly.vertices[0];
        let n = poly.plane.normal;
        let center_side = n.dot(center - v0);
        let point_side = n.dot(point - v0);
        if center_side * point_side < -1e-2 {
            return false;
        }
    }
    true
}

const NAV_EXCLUDED_BRUSH_NAMES: [&str; 8] = [
    "TransitRamp",
    "TransitStair",
    "HazardPitEgress",
    "HazardBridge",
    "CatwalkStairs",
    "CatwalkDeck",
    "CtfSniperDeck",
    "HubSniperBalcony",
];

#[inline]
fn is_nav_excluded_brush_name(name: &str) -> bool {
    NAV_EXCLUDED_BRUSH_NAMES.iter().any(|tag| name.contains(tag))
}

struct NavAddRayBrush {
    brush_idx: usize,
    triangles: Vec<[Vec3; 3]>,
}

struct NavVolBrush {
    csg_oper: crate::CsgOperation,
    aabb: (Vec3, Vec3),
    planes: Vec<(Vec3, Vec3, f32)>,
    affects_los_air: bool,
    affects_chasm_air: bool,
}

/// Precomputed BVH index over world brushes for repeated navigation LoS and air-containment queries.
pub struct NavBrushIndex {
    add_ray_brushes: Vec<NavAddRayBrush>,
    add_ray_bvh: crate::bvh::AabbBvh,
    vol_brushes: Vec<NavVolBrush>,
    vol_bvh: crate::bvh::AabbBvh,
    sub_vol_bvh: crate::bvh::AabbBvh,
    brush_to_vol: Vec<u32>,
}

impl NavBrushIndex {
    /// Builds spatial indices for all world brushes without mutating or reallocating per query.
    pub fn build(brushes: &[crate::Brush]) -> Self {
        let mut add_ray_brushes = Vec::new();
        let mut add_ray_entries = Vec::new();
        let mut vol_brushes = Vec::new();
        let mut vol_entries = Vec::new();
        let mut sub_entries = Vec::new();
        let mut brush_to_vol = vec![u32::MAX; brushes.len()];

        let mut world_polys_verts: Vec<(Vec<Vec3>, Vec3)> = Vec::new();

        for (b_idx, brush) in brushes.iter().enumerate() {
            if brush.polys.is_empty() {
                continue;
            }
            let model_matrix = brush.model_matrix();
            let track_vol = brush.polys.len() >= 4;
            let track_add_ray = brush.csg_oper == crate::CsgOperation::Add
                && !is_nav_excluded_brush_name(&brush.name);

            if !track_vol && !track_add_ray {
                continue;
            }

            let mut ray_tris = Vec::new();
            let mut ray_min = Vec3::splat(f32::MAX);
            let mut ray_max = Vec3::splat(f32::MIN);

            let mut vol_min = Vec3::splat(f32::MAX);
            let mut vol_max = Vec3::splat(f32::MIN);
            let mut vert_sum = Vec3::ZERO;
            let mut vert_cnt = 0usize;
            world_polys_verts.clear();

            for poly in &brush.polys {
                let mut tverts = Vec::with_capacity(poly.vertices.len());
                for &v in &poly.vertices {
                    let tv = model_matrix.transform_point3(v);
                    if track_vol {
                        vol_min = vol_min.min(tv);
                        vol_max = vol_max.max(tv);
                    }
                    tverts.push(tv);
                }

                // Match Brush::to_world_polys / Polygon::new plane validity checks.
                if tverts.len() < 3 {
                    continue;
                }
                let Some(plane) = crate::math::Plane::from_points(tverts[0], tverts[1], tverts[2]) else {
                    continue;
                };

                if track_add_ray
                    && (poly.flags
                        & (crate::poly::PF_NOT_SOLID
                            | crate::poly::PF_PORTAL
                            | crate::poly::PF_TRANSLUCENT
                            | crate::poly::PF_INVISIBLE))
                        == 0
                {
                    let v0 = tverts[0];
                    for k in 1..(tverts.len() - 1) {
                        let tri = [v0, tverts[k], tverts[k + 1]];
                        for &tv in &tri {
                            ray_min = ray_min.min(tv);
                            ray_max = ray_max.max(tv);
                        }
                        ray_tris.push(tri);
                    }
                }

                if track_vol {
                    for &tv in &tverts {
                        vert_sum += tv;
                        vert_cnt += 1;
                    }
                    world_polys_verts.push((tverts, plane.normal));
                }
            }

            if !ray_tris.is_empty() {
                let ray_pos = add_ray_brushes.len() as u32;
                add_ray_brushes.push(NavAddRayBrush {
                    brush_idx: b_idx,
                    triangles: ray_tris,
                });
                add_ray_entries.push(crate::bvh::BvhEntry {
                    pos: ray_pos,
                    min: ray_min,
                    max: ray_max,
                });
            }

            if track_vol && !world_polys_verts.is_empty() {
                let is_sub = brush.csg_oper == crate::CsgOperation::Subtract;
                let affects_los_air = is_sub
                    || (brush.csg_oper == crate::CsgOperation::Add
                        && !is_nav_excluded_brush_name(&brush.name)
                        && !brush.polys.iter().all(|p| {
                            (p.flags
                                & (crate::poly::PF_NOT_SOLID
                                    | crate::poly::PF_PORTAL
                                    | crate::poly::PF_INVISIBLE))
                                != 0
                        }));
                let affects_chasm_air = is_sub
                    || (brush.csg_oper == crate::CsgOperation::Add
                        && !brush.polys.iter().all(|p| {
                            (p.flags & (crate::poly::PF_NOT_SOLID | crate::poly::PF_PORTAL)) != 0
                        }));

                if affects_los_air || affects_chasm_air {
                    let center = if vert_cnt > 0 {
                        vert_sum / (vert_cnt as f32)
                    } else {
                        (vol_min + vol_max) * 0.5
                    };
                    let mut planes = Vec::with_capacity(world_polys_verts.len());
                    for (tverts, n) in &world_polys_verts {
                        if let Some(&v0) = tverts.first() {
                            planes.push((v0, *n, n.dot(center - v0)));
                        }
                    }
                    let vol_idx = vol_brushes.len() as u32;
                    brush_to_vol[b_idx] = vol_idx;
                    vol_brushes.push(NavVolBrush {
                        csg_oper: brush.csg_oper,
                        aabb: (vol_min, vol_max),
                        planes,
                        affects_los_air,
                        affects_chasm_air,
                    });
                    vol_entries.push(crate::bvh::BvhEntry {
                        pos: b_idx as u32,
                        min: vol_min,
                        max: vol_max,
                    });
                    if is_sub {
                        // 1-based brush index allows pos > 0 for all-subtract checks and
                        // pos > (b_idx + 1) for strictly-later carve checks.
                        sub_entries.push(crate::bvh::BvhEntry {
                            pos: (b_idx + 1) as u32,
                            min: vol_min,
                            max: vol_max,
                        });
                    }
                }
            }
        }

        Self {
            add_ray_brushes,
            add_ray_bvh: crate::bvh::AabbBvh::build(&add_ray_entries),
            vol_brushes,
            vol_bvh: crate::bvh::AabbBvh::build(&vol_entries),
            sub_vol_bvh: crate::bvh::AabbBvh::build(&sub_entries),
            brush_to_vol,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.add_ray_brushes.is_empty() && self.vol_brushes.is_empty()
    }

    #[inline]
    fn vol_at_brush(&self, brush_idx: u32) -> &NavVolBrush {
        let vol_idx = self.brush_to_vol[brush_idx as usize] as usize;
        &self.vol_brushes[vol_idx]
    }

    #[inline]
    fn point_inside_vol(pt: Vec3, vol: &NavVolBrush) -> bool {
        if pt.x < vol.aabb.0.x - 0.1
            || pt.x > vol.aabb.1.x + 0.1
            || pt.y < vol.aabb.0.y - 0.1
            || pt.y > vol.aabb.1.y + 0.1
            || pt.z < vol.aabb.0.z - 0.1
            || pt.z > vol.aabb.1.z + 0.1
        {
            return false;
        }
        for &(v0, n, center_side) in &vol.planes {
            let point_side = n.dot(pt - v0);
            if center_side * point_side < -1e-2 {
                return false;
            }
        }
        true
    }

    #[inline]
    fn is_carved_by_later_subtract(&self, pt: Vec3, add_brush_idx: usize) -> bool {
        self.sub_vol_bvh.any_pos_above_containing_point(
            pt,
            0.101,
            (add_brush_idx + 1) as u32,
            |pos_1based| Self::point_inside_vol(pt, self.vol_at_brush(pos_1based - 1)),
        )
    }

    #[inline]
    fn any_subtract_contains(&self, pt: Vec3) -> bool {
        self.sub_vol_bvh
            .any_pos_above_containing_point(pt, 0.101, 0, |pos_1based| {
                Self::point_inside_vol(pt, self.vol_at_brush(pos_1based - 1))
            })
    }

    /// Evaluates whether a navigation link between `from` and `to` is blocked by solid CSG geometry.
    pub fn is_nav_link_blocked(&self, from: Vec3, to: Vec3) -> bool {
        if self.is_empty() {
            return false;
        }
        let seg = to - from;
        let dist = seg.length();
        if dist < 1.0 {
            return false;
        }
        let dir = seg / dist;

        // 1. Check ray intersection against solid Add brush polygons not carved by a later Subtract.
        let hit_solid_add = self.add_ray_bvh.any_segment_hit(from, dir, dist, 0.01, |ray_idx| {
            let add_brush = &self.add_ray_brushes[ray_idx as usize];
            for tri in &add_brush.triangles {
                if let Some(t) =
                    crate::lighting::ray_intersects_triangle(from, dir, tri[0], tri[1], tri[2])
                {
                    if t > 1.0 && t < dist - 1.0 {
                        let hit_pt = from + dir * t;
                        if !self.is_carved_by_later_subtract(hit_pt, add_brush.brush_idx) {
                            return true;
                        }
                    }
                }
            }
            false
        });
        if hit_solid_add {
            return true;
        }

        // 2. If subtractive geometry exists and both endpoints are inside Subtract space,
        // verify interior samples remain inside carved air.
        if !self.sub_vol_bvh.is_empty()
            && self.any_subtract_contains(from)
            && self.any_subtract_contains(to)
        {
            const STEPS: usize = 16;
            for s in 1..STEPS {
                let alpha = s as f32 / STEPS as f32;
                let sample_pt = from.lerp(to, alpha);
                let top = self.vol_bvh.find_highest_pos_containing_point(
                    sample_pt,
                    0.101,
                    u32::MAX,
                    |b_idx| {
                        let vol = self.vol_at_brush(b_idx);
                        vol.affects_los_air && Self::point_inside_vol(sample_pt, vol)
                    },
                );
                let is_air = top.is_some_and(|b_idx| {
                    self.vol_at_brush(b_idx).csg_oper == crate::CsgOperation::Subtract
                });
                if !is_air {
                    return true;
                }
            }
        }

        false
    }

    #[inline]
    pub fn is_point_in_carved_air(&self, pt: Vec3) -> bool {
        if self.vol_bvh.is_empty() {
            return false;
        }
        let top = self
            .vol_bvh
            .find_highest_pos_containing_point(pt, 0.101, u32::MAX, |b_idx| {
                let vol = self.vol_at_brush(b_idx);
                vol.affects_chasm_air && Self::point_inside_vol(pt, vol)
            });
        top.is_some_and(|b_idx| self.vol_at_brush(b_idx).csg_oper == crate::CsgOperation::Subtract)
    }
}

/// Evaluates whether a navigation link between `from` and `to` is blocked by solid CSG geometry
/// (additive/semisolid walls or void rock between disconnected subtracted rooms).
pub fn is_nav_link_blocked_by_brushes(from: Vec3, to: Vec3, brushes: &[crate::Brush]) -> bool {
    if brushes.is_empty() {
        return false;
    }

    let seg = to - from;
    let dist = seg.length();
    if dist < 1.0 {
        return false;
    }
    let dir = seg / dist;

    // 1. Check ray intersection against solid Add brush polygons
    // that are not subsequently carved away by a later Subtract brush.
    for (b_idx, brush) in brushes.iter().enumerate() {
        if brush.csg_oper != crate::CsgOperation::Add {
            continue;
        }
        if is_nav_excluded_brush_name(&brush.name) {
            continue;
        }
        let world_polys = brush.to_world_polys();
        for poly in &world_polys {
            if (poly.flags
                & (crate::poly::PF_NOT_SOLID
                    | crate::poly::PF_PORTAL
                    | crate::poly::PF_TRANSLUCENT
                    | crate::poly::PF_INVISIBLE))
                != 0
            {
                continue;
            }
            if poly.vertices.len() >= 3 {
                let v0 = poly.vertices[0];
                for k in 1..(poly.vertices.len() - 1) {
                    if let Some(t) = crate::lighting::ray_intersects_triangle(
                        from,
                        dir,
                        v0,
                        poly.vertices[k],
                        poly.vertices[k + 1],
                    ) {
                        if t > 1.0 && t < dist - 1.0 {
                            let hit_pt = from + dir * t;
                            // Check if a subsequent Subtract brush carves out this hit point (e.g. a doorway)
                            let carved_later = brushes[(b_idx + 1)..]
                                .iter()
                                .any(|later| {
                                    later.csg_oper == crate::CsgOperation::Subtract
                                        && point_inside_brush(hit_pt, later)
                                });
                            if !carved_later {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. If the level uses Subtractive CSG rooms, verify that interior sample points
    // along the path remain inside carved air (and not inside solid rock between disconnected rooms).
    let has_subtract = brushes
        .iter()
        .any(|b| b.csg_oper == crate::CsgOperation::Subtract);
    if has_subtract {
        // Only enforce void-rock sampling if both endpoints are inside Subtract space
        let endpoints_in_sub = brushes.iter().any(|b| {
            b.csg_oper == crate::CsgOperation::Subtract && point_inside_brush(from, b)
        }) && brushes.iter().any(|b| {
            b.csg_oper == crate::CsgOperation::Subtract && point_inside_brush(to, b)
        });

        if endpoints_in_sub {
            let steps = 16;
            for s in 1..steps {
                let alpha = s as f32 / steps as f32;
                let sample_pt = from.lerp(to, alpha);
                let mut is_air = false;
                for brush in brushes {
                    match brush.csg_oper {
                        crate::CsgOperation::Subtract => {
                            if point_inside_brush(sample_pt, brush) {
                                is_air = true;
                            }
                        }
                        crate::CsgOperation::Add => {
                            if is_nav_excluded_brush_name(&brush.name) {
                                continue;
                            }
                            let is_non_solid = brush.polys.iter().all(|p| {
                                (p.flags
                                    & (crate::poly::PF_NOT_SOLID
                                        | crate::poly::PF_PORTAL
                                        | crate::poly::PF_INVISIBLE))
                                    != 0
                            });
                            if !is_non_solid && point_inside_brush(sample_pt, brush) {
                                is_air = false;
                            }
                        }
                        _ => {}
                    }
                }
                if !is_air {
                    return true;
                }
            }
        }
    }

    false
}

/// Collision-aware automatic ReachSpec builder using a prebuilt `NavBrushIndex`.
pub fn auto_build_navigation_reachspecs_with_index(
    nav_graph: &mut NavGraph,
    brush_index: &NavBrushIndex,
    max_distance_uu: f32,
    max_step_height: f32,
    max_jump_height: f32,
) -> AutoPathReport {
    let mut report = AutoPathReport {
        total_nodes: nav_graph.nodes.len(),
        max_step_height_uu: max_step_height,
        max_link_distance_uu: max_distance_uu,
        ..Default::default()
    };

    // Clear existing reach specs and rebuild fresh topology
    nav_graph.reach_specs.clear();
    nav_graph.adjacency.clear();

    struct NodeMeta {
        location: Vec3,
        is_lift_center: bool,
        is_lift_exit: bool,
        is_teleporter: bool,
        is_pathnode: bool,
        is_foot: bool,
        is_head: bool,
        has_floor: bool,
    }

    let floor_probe_depth = 200.0;
    let check_chasm = !brush_index.is_empty();
    let metas: Vec<NodeMeta> = nav_graph
        .nodes
        .iter()
        .map(|node| {
            let lower = node.node_type.to_ascii_lowercase();
            let has_floor = if check_chasm {
                !brush_index.is_point_in_carved_air(Vec3::new(
                    node.location.x,
                    node.location.y,
                    node.location.z - floor_probe_depth,
                ))
            } else {
                false
            };
            NodeMeta {
                location: node.location,
                is_lift_center: lower.contains("liftcenter"),
                is_lift_exit: lower.contains("liftexit"),
                is_teleporter: lower.contains("teleporter"),
                is_pathnode: lower.contains("pathnode"),
                is_foot: node.name.starts_with("PathNode_Foot_"),
                is_head: node.name.starts_with("PathNode_Head_"),
                has_floor,
            }
        })
        .collect();

    let n = metas.len();
    for i in 0..n {
        let mut connected_count = 0;
        let mi = &metas[i];
        let p_i = mi.location;

        for j in 0..n {
            if i == j {
                continue;
            }
            let mj = &metas[j];
            let p_j = mj.location;
            let dist = p_i.distance(p_j);

            let delta_z = p_j.z - p_i.z;
            let horiz_dist = Vec3::new(p_i.x - p_j.x, p_i.y - p_j.y, 0.0).length();

            let is_lift_link =
                (mi.is_lift_center && mj.is_lift_exit) || (mi.is_lift_exit && mj.is_lift_center);
            let is_tele_link = mi.is_teleporter && mj.is_teleporter;
            let is_ramp_link = (mi.is_pathnode && mj.is_pathnode)
                && ((mi.is_foot && mj.is_head) || (mi.is_head && mj.is_foot));

            if !is_lift_link && !is_tele_link && !is_ramp_link && dist > max_distance_uu {
                continue;
            }

            // Check vertical locomotion feasibility
            let (mut can_reach, reach_flags) = if is_tele_link {
                (true, R_SPECIAL)
            } else if is_lift_link && horiz_dist <= 300.0 && delta_z.abs() <= 2048.0 {
                (true, R_SPECIAL)
            } else if is_ramp_link {
                (true, R_WALK)
            } else if delta_z.abs() <= max_step_height {
                // Flat walk / gentle ramp
                (true, R_WALK)
            } else if delta_z > 0.0 && delta_z <= max_jump_height && horiz_dist <= 350.0 {
                // Jumping up onto ledge
                (true, R_JUMP | R_WALK)
            } else if delta_z < 0.0 && delta_z.abs() <= 400.0 {
                // Dropping down off ledge
                (true, R_WALK)
            } else {
                (false, 0)
            };

            if !can_reach {
                continue;
            }

            // Check for an unbridged deep chasm beneath flat R_WALK links when brushes are present
            if reach_flags == R_WALK
                && delta_z.abs() <= max_step_height
                && check_chasm
                && mi.has_floor
                && mj.has_floor
            {
                let mid = p_i.lerp(p_j, 0.5);
                let mid_is_deep_pit = brush_index.is_point_in_carved_air(Vec3::new(
                    mid.x,
                    mid.y,
                    mid.z - floor_probe_depth,
                ));
                if mid_is_deep_pit {
                    can_reach = false;
                }
            }

            if !can_reach {
                continue;
            }

            let path_blocked = if is_tele_link || brush_index.is_empty() {
                false
            } else if is_lift_link {
                // Elevator movement is L-shaped: vertical along LiftCenter X/Y to top Z, then horizontal to LiftExit
                let (lc_pos, le_pos) = if mi.is_lift_center {
                    (p_i, p_j)
                } else {
                    (p_j, p_i)
                };
                let shaft_top = Vec3::new(lc_pos.x, lc_pos.y, lc_pos.z.max(le_pos.z));
                brush_index.is_nav_link_blocked(lc_pos, shaft_top)
                    || brush_index.is_nav_link_blocked(shaft_top, le_pos)
            } else {
                brush_index.is_nav_link_blocked(p_i, p_j)
            };

            if !path_blocked {
                nav_graph.add_reach_spec(i, j, reach_flags);
                connected_count += 1;
            }
        }

        if connected_count == 0 {
            report.isolated_nodes.push(nav_graph.nodes[i].name.clone());
        }
    }

    // Post-processing: Prune redundant transitive edges (shortcuts) and cap each node to UE1 Paths[16]
    prune_redundant_and_cap_paths16(nav_graph);

    report.created_reachspecs = nav_graph.reach_specs.len();
    report.walk_reachspecs = nav_graph
        .reach_specs
        .iter()
        .filter(|s| (s.reach_flags & (R_JUMP | R_FLY | R_SPECIAL)) == 0)
        .count();
    report.jump_reachspecs = nav_graph
        .reach_specs
        .iter()
        .filter(|s| (s.reach_flags & R_JUMP) != 0)
        .count();
    report.fly_reachspecs = nav_graph
        .reach_specs
        .iter()
        .filter(|s| (s.reach_flags & R_FLY) != 0)
        .count();

    report
}

/// Collision-aware automatic ReachSpec builder that checks CSG line-of-sight against `brushes`,
/// supports vertical `LiftCenter <-> LiftExit` (`R_SPECIAL`) connections, and prevents `R_WALK` across deep chasms.
pub fn auto_build_navigation_reachspecs_with_brushes(
    nav_graph: &mut NavGraph,
    brushes: &[crate::Brush],
    max_distance_uu: f32,
    max_step_height: f32,
    max_jump_height: f32,
) -> AutoPathReport {
    let brush_index = NavBrushIndex::build(brushes);
    auto_build_navigation_reachspecs_with_index(
        nav_graph,
        &brush_index,
        max_distance_uu,
        max_step_height,
        max_jump_height,
    )
}

/// Prunes redundant geometric shortcut ReachSpecs (transitive reduction) and enforces the
/// Unreal Engine 1 canonical maximum of 16 outgoing ReachSpecs (`NavigationPoint::Paths[16]`).
/// Returns the number of pruned ReachSpecs.
pub fn prune_redundant_and_cap_paths16(nav_graph: &mut NavGraph) -> usize {
    let initial_count = nav_graph.reach_specs.len();
    if initial_count == 0 {
        return 0;
    }

    let node_count = nav_graph.nodes.len();

    // 1. Build adjacency map for fast lookups: from -> Vec<(spec_idx, to, dist, flags)>
    let mut outgoing: Vec<Vec<(usize, usize, f32, u32)>> = vec![Vec::new(); node_count];
    for (idx, spec) in nav_graph.reach_specs.iter().enumerate() {
        if spec.from < node_count && spec.to < node_count {
            outgoing[spec.from].push((idx, spec.to, spec.distance, spec.reach_flags));
        }
    }

    // Fast direct reachability matrix: direct_edge[u][w] = (dist, flags)
    let mut direct_dist: HashMap<(usize, usize), (f32, u32)> = HashMap::with_capacity(initial_count);
    for spec in &nav_graph.reach_specs {
        direct_dist.insert((spec.from, spec.to), (spec.distance, spec.reach_flags));
    }

    let mut keep_spec = vec![true; initial_count];

    // 2. Transitive shortcut reduction (Triangle Pruning)
    // If u -> w can be bridged via intermediate node v (u -> v and v -> w),
    // and d(u, v) + d(v, w) <= 1.22 * d(u, w), then direct u -> w is redundant.
    // Exclude R_SPECIAL (lifts, teleporters) and R_JUMP from pruning.
    for u in 0..node_count {
        for &(idx_uw, w, dist_uw, flags_uw) in &outgoing[u] {
            if (flags_uw & (R_SPECIAL | R_JUMP)) != 0 {
                continue;
            }

            // Look for intermediate v
            for &(_idx_uv, v, dist_uv, flags_uv) in &outgoing[u] {
                if v == u || v == w {
                    continue;
                }
                if dist_uv >= dist_uw {
                    continue; // intermediate must be closer than destination
                }

                if let Some(&(dist_vw, flags_vw)) = direct_dist.get(&(v, w)) {
                    if dist_vw >= dist_uw {
                        continue;
                    }
                    // Both intermediate legs must be traversable without special actions
                    let can_traverse_via_v = (flags_uv & (R_SPECIAL | R_JUMP)) == 0
                        && (flags_vw & (R_SPECIAL | R_JUMP)) == 0;
                    let to_is_inv = nav_graph.nodes.get(w).map_or(false, |n| {
                        n.node_type.to_ascii_lowercase().contains("inventory")
                    });
                    if can_traverse_via_v && !to_is_inv && (dist_uv + dist_vw) <= 1.22 * dist_uw {
                        keep_spec[idx_uw] = false;
                        break;
                    }
                }
            }
        }
    }

    // Track active incoming counts so Paths[16] capping never isolates a node
    let mut incoming_counts = vec![0usize; node_count];
    for (idx, spec) in nav_graph.reach_specs.iter().enumerate() {
        if keep_spec[idx] && spec.to < node_count {
            incoming_counts[spec.to] += 1;
        }
    }

    // 3. Enforce UE1 Paths[16] maximum outgoing edges per NavigationPoint
    let is_structural = |node_idx: usize| -> bool {
        if let Some(n) = nav_graph.nodes.get(node_idx) {
            let t = n.node_type.to_ascii_lowercase();
            t.contains("pathnode")
                || t.contains("playerstart")
                || t.contains("lift")
                || t.contains("teleporter")
                || t.contains("flagbase")
                || t.contains("controlpoint")
                || t.contains("fortstandard")
        } else {
            false
        }
    };

    for u in 0..node_count {
        // Collect currently active outgoing edges from u
        let mut active: Vec<(usize, usize, f32, u32)> = outgoing[u]
            .iter()
            .copied()
            .filter(|&(idx, _, _, _)| keep_spec[idx])
            .collect();

        if active.len() > 16 {
            // Sort by priority so we preserve the most critical 16 outgoing navigation links:
            // 1) R_SPECIAL (lifts, teleporters)
            // 2) R_JUMP
            // 3) Structural NavigationPoints (PathNodes, PlayerStarts, FlagBases)
            // 4) Shortest distance ascending
            active.sort_by(|&(_, to_a, dist_a, flags_a), &(_, to_b, dist_b, flags_b)| {
                let spec_a = (flags_a & R_SPECIAL) != 0;
                let spec_b = (flags_b & R_SPECIAL) != 0;
                if spec_a != spec_b {
                    return spec_b.cmp(&spec_a);
                }

                let jump_a = (flags_a & R_JUMP) != 0;
                let jump_b = (flags_b & R_JUMP) != 0;
                if jump_a != jump_b {
                    return jump_b.cmp(&jump_a);
                }

                let struct_a = is_structural(to_a);
                let struct_b = is_structural(to_b);
                if struct_a != struct_b {
                    return struct_b.cmp(&struct_a);
                }

                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            });

            // Keep top 16, prune excess (first pruning specs whose destination has alternative paths)
            let mut excess = active.len().saturating_sub(16);
            for &(idx, to, _, _) in &active[16..] {
                if incoming_counts[to] > 1 {
                    keep_spec[idx] = false;
                    incoming_counts[to] -= 1;
                    excess = excess.saturating_sub(1);
                }
            }
            // If still exceeding 16 paths, enforce UE1 Paths[16] hard array limit
            if excess > 0 {
                for &(idx, to, _, _) in active[16..].iter().rev() {
                    if excess == 0 {
                        break;
                    }
                    if keep_spec[idx] {
                        keep_spec[idx] = false;
                        incoming_counts[to] = incoming_counts[to].saturating_sub(1);
                        excess -= 1;
                    }
                }
            }
        }
    }

    // 4. Compact ReachSpecs array and rebuild adjacency
    let old_specs = std::mem::take(&mut nav_graph.reach_specs);
    nav_graph.adjacency.clear();

    for (old_idx, spec) in old_specs.into_iter().enumerate() {
        if keep_spec[old_idx] {
            let new_idx = nav_graph.reach_specs.len();
            nav_graph.adjacency.entry(spec.from).or_default().push(new_idx);
            nav_graph.reach_specs.push(spec);
        }
    }

    initial_count.saturating_sub(nav_graph.reach_specs.len())
}

/// 3D Parabolic Trajectory Arc for UT_JumpPad / Kicker actors
#[derive(Debug, Clone, PartialEq)]
pub struct JumpPadTrajectoryArc {
    pub source_name: String,
    pub source_location: Vec3,
    pub kick_velocity: Vec3,
    pub target_node_name: Option<String>,
    pub target_location: Option<Vec3>,
    pub trajectory_points: Vec<Vec3>,
    pub estimated_landing_pos: Vec3,
    pub landing_distance_error: f32,
    pub flight_time_secs: f32,
    pub apex_height_z: f32,
    pub has_ceiling_hazard: bool,
    pub has_overshoot_hazard: bool,
}

/// Simulates ballistic projectile flight for a JumpPad or Kicker pad under UT99 gravity
pub fn simulate_jumppad_trajectory(
    source_name: &str,
    source_location: Vec3,
    kick_velocity: Vec3,
    target_node_name: Option<String>,
    target_location: Option<Vec3>,
    gravity_z: f32,
) -> JumpPadTrajectoryArc {
    let grav = if gravity_z.abs() < 1e-3 { -950.0 } else { gravity_z }; // Canonical UT99 gravity

    // Determine flight duration until landing
    let target_z = target_location.map(|t| t.z).unwrap_or(source_location.z);
    let delta_z = target_z - source_location.z;

    // Solve: 0.5 * grav * t^2 + kick_velocity.z * t - delta_z = 0
    let a = 0.5 * grav;
    let b = kick_velocity.z;
    let c = -delta_z;
    let discriminant = b * b - 4.0 * a * c;

    let flight_time = if discriminant >= 0.0 {
        let t1 = (-b + discriminant.sqrt()) / (2.0 * a);
        let t2 = (-b - discriminant.sqrt()) / (2.0 * a);
        let max_t = t1.max(t2);
        if max_t > 0.05 { max_t } else { 1.2 }
    } else {
        // Fallback: flight time to return to ground level
        let t = -2.0 * b / a;
        if t > 0.05 { t } else { 1.2 }
    }.clamp(0.1, 4.0);

    // Apex height calculation
    let t_apex = (-b / (2.0 * a)).clamp(0.0, flight_time);
    let apex_z = source_location.z + b * t_apex + 0.5 * grav * t_apex * t_apex;

    // Sample trajectory curve
    let segments = 24;
    let mut trajectory_points = Vec::with_capacity(segments + 1);
    for s in 0..=segments {
        let t = (s as f32 / segments as f32) * flight_time;
        let pos = Vec3::new(
            source_location.x + kick_velocity.x * t,
            source_location.y + kick_velocity.y * t,
            source_location.z + kick_velocity.z * t + 0.5 * grav * t * t,
        );
        trajectory_points.push(pos);
    }

    let estimated_landing_pos = *trajectory_points.last().unwrap();
    let landing_error = target_location
        .map(|tgt| estimated_landing_pos.distance(tgt))
        .unwrap_or(0.0);

    let overshoot = landing_error > 96.0;
    let ceiling_hazard = apex_z - source_location.z > 500.0;

    JumpPadTrajectoryArc {
        source_name: source_name.to_string(),
        source_location,
        kick_velocity,
        target_node_name,
        target_location,
        trajectory_points,
        estimated_landing_pos,
        landing_distance_error: landing_error,
        flight_time_secs: flight_time,
        apex_height_z: apex_z,
        has_ceiling_hazard: ceiling_hazard,
        has_overshoot_hazard: overshoot,
    }
}

/// Sychronization and validation report for LiftCenter and LiftExit pathways
#[derive(Debug, Clone, PartialEq)]
pub struct LiftSyncReport {
    pub lift_center_name: String,
    pub lift_tag: String,
    pub associated_mover_name: Option<String>,
    pub associated_exits: Vec<String>,
    pub mover_stroke_height: Option<f32>,
    pub exits_vertical_span: f32,
    pub is_synchronized: bool,
    pub issues: Vec<String>,
}

/// Tactical Bot Cover classification rating against enemy sightlines
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticalCoverRating {
    SolidCover,   // High concealment behind pillars or interior walls
    PartialCover, // Medium concealment, partially shielded
    OpenHazard,   // Openly exposed to line-of-fire from multiple high vantage perches
}

impl TacticalCoverRating {
    pub fn label(&self) -> &'static str {
        match self {
            TacticalCoverRating::SolidCover => "Solid Shielded Cover",
            TacticalCoverRating::PartialCover => "Partial Obstacle Cover",
            TacticalCoverRating::OpenHazard => "Exposed Crossfire Hazard",
        }
    }
}

/// Tactical evaluation of a navigation node for AI bot combat positioning
#[derive(Debug, Clone, PartialEq)]
pub struct TacticalCoverEntry {
    pub node_id: usize,
    pub node_name: String,
    pub location: Vec3,
    pub cover_rating: TacticalCoverRating,
    pub visible_nodes_count: usize,
    pub is_high_vantage_perch: bool,
    pub strategic_item_bonus: bool,
}

#[cfg(test)]
mod lift_and_chasm_tests {
    use super::*;
    use crate::brush::build_cube;
    use crate::CsgOperation;

    #[test]
    fn test_lift_shaft_vertical_reachspecs_and_chasm_blocking() {
        let mut graph = NavGraph::new();
        let exit_low = graph.add_node("LiftExit0", Vec3::new(0.0, 0.0, 0.0), "LiftExit");
        let lift_center = graph.add_node("LiftCenter0", Vec3::new(64.0, 0.0, 0.0), "LiftCenter");
        let exit_high = graph.add_node("LiftExit1", Vec3::new(64.0, 64.0, 512.0), "LiftExit");

        let report = auto_build_navigation_reachspecs_with_brushes(&mut graph, &[], 800.0, 45.0, 90.0);
        assert!(report.created_reachspecs >= 4);
        let has_up_lift = graph
            .reach_specs
            .iter()
            .any(|rs| rs.from == lift_center && rs.to == exit_high && (rs.reach_flags & R_SPECIAL) != 0);
        let has_down_lift = graph
            .reach_specs
            .iter()
            .any(|rs| rs.from == exit_high && rs.to == lift_center && (rs.reach_flags & R_SPECIAL) != 0);
        let _ = exit_low;
        assert!(has_up_lift && has_down_lift);

        // Chasm test: two PathNodes in a room separated by a deep Subtract pit
        let mut room = build_cube("Room", CsgOperation::Subtract, Vec3::new(1024.0, 512.0, 256.0));
        room.location = Vec3::ZERO;
        let mut pit = build_cube("DeepPit", CsgOperation::Subtract, Vec3::new(256.0, 512.0, 512.0));
        pit.location = Vec3::new(0.0, 0.0, -256.0);

        let mut chasm_graph = NavGraph::new();
        chasm_graph.add_node("PathNode0", Vec3::new(-300.0, 0.0, -80.0), "PathNode");
        chasm_graph.add_node("PathNode1", Vec3::new(300.0, 0.0, -80.0), "PathNode");

        let chasm_report = auto_build_navigation_reachspecs_with_brushes(
            &mut chasm_graph,
            &[room, pit],
            800.0,
            45.0,
            90.0,
        );
        assert_eq!(
            chasm_report.created_reachspecs, 0,
            "Flat R_WALK link across an unbridged deep pit must be blocked"
        );
    }

    #[test]
    fn test_prune_redundant_and_cap_paths16() {
        let mut graph = NavGraph::new();

        // 1. Triangle pruning: A (0,0) -> B (200,0) -> C (400,0) in a line
        let na = graph.add_node("NodeA", Vec3::new(0.0, 0.0, 0.0), "PathNode");
        let nb = graph.add_node("NodeB", Vec3::new(200.0, 0.0, 0.0), "PathNode");
        let nc = graph.add_node("NodeC", Vec3::new(400.0, 0.0, 0.0), "PathNode");

        // Direct edges
        graph.add_reach_spec(na, nb, R_WALK);
        graph.add_reach_spec(nb, nc, R_WALK);
        graph.add_reach_spec(na, nc, R_WALK); // Redundant long chord: d(A,B) + d(B,C) = 400 == d(A,C)

        let pruned = prune_redundant_and_cap_paths16(&mut graph);
        assert_eq!(pruned, 1, "Expected direct chord na -> nc to be pruned");
        assert_eq!(graph.reach_specs.len(), 2);
        assert!(graph.reach_specs.iter().any(|s| s.from == na && s.to == nb));
        assert!(graph.reach_specs.iter().any(|s| s.from == nb && s.to == nc));
        assert!(!graph.reach_specs.iter().any(|s| s.from == na && s.to == nc));

        // 2. Paths[16] cap test: Node Hub with 25 target nodes
        let mut hub_graph = NavGraph::new();
        let hub = hub_graph.add_node("Hub", Vec3::ZERO, "PathNode");

        // Add 1 LiftCenter (R_SPECIAL), 1 JumpSpot (R_JUMP), 10 PathNodes, and 13 InventorySpots
        let lift = hub_graph.add_node("LiftTarget", Vec3::new(100.0, 0.0, 0.0), "LiftExit");
        hub_graph.add_reach_spec(hub, lift, R_SPECIAL);

        let jump = hub_graph.add_node("JumpTarget", Vec3::new(150.0, 0.0, 0.0), "PathNode");
        hub_graph.add_reach_spec(hub, jump, R_JUMP);

        for i in 0..10 {
            let p = hub_graph.add_node(format!("Path_{i}"), Vec3::new(200.0 + i as f32 * 50.0, 100.0, 0.0), "PathNode");
            hub_graph.add_reach_spec(hub, p, R_WALK);
        }
        for i in 0..13 {
            let inv = hub_graph.add_node(format!("Item_{i}"), Vec3::new(-200.0 - i as f32 * 50.0, -100.0, 0.0), "InventorySpot");
            hub_graph.add_reach_spec(hub, inv, R_WALK);
        }

        assert_eq!(hub_graph.reach_specs.len(), 25);
        let cap_pruned = prune_redundant_and_cap_paths16(&mut hub_graph);
        assert_eq!(cap_pruned, 9, "Expected 9 excess specs to be capped to 16");
        assert_eq!(hub_graph.reach_specs.len(), 16);

        // Verify high-priority specs are kept
        assert!(hub_graph.reach_specs.iter().any(|s| s.from == hub && s.to == lift && (s.reach_flags & R_SPECIAL) != 0));
        assert!(hub_graph.reach_specs.iter().any(|s| s.from == hub && s.to == jump && (s.reach_flags & R_JUMP) != 0));
    }

    #[test]
    fn test_nav_brush_index_matches_legacy_predicates() {
        let mut brushes = Vec::new();

        // Disconnected subtracted rooms
        let mut room_a = build_cube("SubRoomA", CsgOperation::Subtract, Vec3::new(512.0, 512.0, 256.0));
        room_a.location = Vec3::new(-400.0, 0.0, 0.0);
        brushes.push(room_a);

        let mut room_b = build_cube("SubRoomB", CsgOperation::Subtract, Vec3::new(512.0, 512.0, 256.0));
        room_b.location = Vec3::new(400.0, 0.0, 0.0);
        brushes.push(room_b);

        // Solid additive dividing wall inside room_a, plus a doorway subtract cut after it
        let mut wall = build_cube("DividingWall", CsgOperation::Add, Vec3::new(32.0, 512.0, 256.0));
        wall.location = Vec3::new(-400.0, 0.0, 0.0);
        brushes.push(wall);

        let mut door = build_cube("DoorwayCut", CsgOperation::Subtract, Vec3::new(96.0, 96.0, 160.0));
        door.location = Vec3::new(-400.0, 0.0, -48.0);
        brushes.push(door);

        // Excluded named additive ramp brush and a 1-poly sheet solid Add brush + non-solid portal
        let mut ramp = build_cube("TransitRamp_01", CsgOperation::Add, Vec3::new(128.0, 64.0, 64.0));
        ramp.location = Vec3::new(400.0, 0.0, -64.0);
        brushes.push(ramp);

        let mut thin_sheet = crate::brush::build_sheet("ThinSolidSheet", CsgOperation::Add, 128.0, 128.0);
        thin_sheet.location = Vec3::new(400.0, 140.0, 0.0);
        brushes.push(thin_sheet);

        let mut nonsolid = build_cube("NonSolidBlock", CsgOperation::Add, Vec3::new(64.0, 64.0, 64.0));
        nonsolid.location = Vec3::new(400.0, -140.0, 0.0);
        for p in &mut nonsolid.polys {
            p.flags |= crate::poly::PF_NOT_SOLID;
        }
        brushes.push(nonsolid);

        let index = NavBrushIndex::build(&brushes);

        let pairs = [
            // Through doorway cut inside room_a -> unblocked
            (Vec3::new(-500.0, 0.0, -48.0), Vec3::new(-300.0, 0.0, -48.0)),
            // Through solid wall away from doorway -> blocked
            (Vec3::new(-500.0, 180.0, -48.0), Vec3::new(-300.0, 180.0, -48.0)),
            // Across void rock between disconnected room_a and room_b -> blocked
            (Vec3::new(-300.0, 0.0, 0.0), Vec3::new(300.0, 0.0, 0.0)),
            // Across TransitRamp (excluded by name) inside room_b -> unblocked
            (Vec3::new(300.0, 0.0, -64.0), Vec3::new(500.0, 0.0, -64.0)),
            // Across 1-poly solid sheet inside room_b -> blocked
            (Vec3::new(320.0, 140.0, 0.0), Vec3::new(480.0, 140.0, 0.0)),
            // Across non-solid Add block inside room_b -> unblocked
            (Vec3::new(320.0, -140.0, 0.0), Vec3::new(480.0, -140.0, 0.0)),
            // Pure vertical lift segment -> axis-aligned dir (0, 0, 1)
            (Vec3::new(-400.0, 0.0, -80.0), Vec3::new(-400.0, 0.0, 80.0)),
        ];

        for &(from, to) in &pairs {
            assert_eq!(
                index.is_nav_link_blocked(from, to),
                is_nav_link_blocked_by_brushes(from, to, &brushes),
                "NavBrushIndex mismatch on segment {from:?} -> {to:?}"
            );
        }

        let mut g_legacy = NavGraph::new();
        let mut g_index = NavGraph::new();
        for g in [&mut g_legacy, &mut g_index] {
            g.add_node("P0", Vec3::new(-500.0, 0.0, -48.0), "PathNode");
            g.add_node("P1", Vec3::new(-300.0, 0.0, -48.0), "PathNode");
            g.add_node("P2", Vec3::new(-500.0, 180.0, -48.0), "PathNode");
            g.add_node("P3", Vec3::new(-300.0, 180.0, -48.0), "PathNode");
            g.add_node("P4", Vec3::new(300.0, 0.0, -48.0), "PathNode");
            g.add_node("P5", Vec3::new(500.0, 0.0, -48.0), "PathNode");
        }
        let mut g_legacy2 = g_legacy.clone();
        // Also verify full graph equivalence
        let rep_idx = auto_build_navigation_reachspecs_with_index(&mut g_index, &index, 850.0, 54.0, 180.0);
        let rep_wrap = auto_build_navigation_reachspecs_with_brushes(&mut g_legacy2, &brushes, 850.0, 54.0, 180.0);
        assert_eq!(rep_idx, rep_wrap);
        assert_eq!(g_index.reach_specs, g_legacy2.reach_specs);
    }
}




