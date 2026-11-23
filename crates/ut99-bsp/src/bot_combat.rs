use glam::Vec3;
use crate::brush::Brush;
use crate::navigation::NavGraph;

/// Tactical cover rating for a position during a combat encounter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CombatCoverRating {
    FullCover,      // Completely occluded by solid brush geometry
    PartialCover,   // Partial visibility / low obstacle present
    #[default]
    Exposed,        // Direct, unobstructed line-of-sight from enemy positions
}

/// A strategic vantage point or sniper perch in the tournament arena
#[derive(Debug, Clone, PartialEq)]
pub struct CombatVantagePoint {
    pub node_index: usize,
    pub location: Vec3,
    pub elevation_advantage_uu: f32,
    pub visible_enemies_count: usize,
    pub sniper_threat_score: f32, // 0.0 (safe) .. 1.0 (dominant sightline)
    pub cover_rating: CombatCoverRating,
}

/// A line-of-sight engagement vector between two combat navigation points
#[derive(Debug, Clone, PartialEq)]
pub struct CombatSightline {
    pub from_node: usize,
    pub to_node: usize,
    pub from_pos: Vec3,
    pub to_pos: Vec3,
    pub distance_uu: f32,
    pub is_direct_los: bool,
    pub recommended_weapon: &'static str,
}

/// Comprehensive tactical combat analysis report
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CombatCoverageReport {
    pub vantage_points: Vec<CombatVantagePoint>,
    pub sightlines: Vec<CombatSightline>,
    pub high_ground_perches_count: usize,
    pub exposed_death_zones_count: usize,
    pub longest_sightline_uu: f32,
}

/// Analyzes mutual visibility, elevation advantage, and tactical sightlines across navigation nodes
pub fn analyze_combat_sightlines(nav_graph: &NavGraph, brushes: &[Brush]) -> CombatCoverageReport {
    let mut report = CombatCoverageReport::default();
    let n = nav_graph.nodes.len();

    let mut visible_targets_per_node = vec![0usize; n];
    let mut max_sightline = 0.0f32;

    // 1. Evaluate pairwise lines-of-sight between nodes
    for i in 0..n {
        let p_i = nav_graph.nodes[i].location + Vec3::new(0.0, 0.0, 48.0); // Eye level (~48 uu above feet)

        for j in (i + 1)..n {
            let p_j = nav_graph.nodes[j].location + Vec3::new(0.0, 0.0, 48.0);
            let dist = p_i.distance(p_j);

            // Maximum realistic combat engagement range in UT99 is ~3500 uu
            if dist > 3500.0 {
                continue;
            }

            // Raycast test against world brush bounding boxes
            let mut occluded = false;
            let ray_dir = (p_j - p_i).normalize();

            for brush in brushes {
                let (min, max) = brush.world_bounds();
                // Expanded AABB check for ray intersection
                if ray_intersects_aabb(p_i, ray_dir, dist, min, max) {
                    occluded = true;
                    break;
                }
            }

            let weapon = if dist > 1800.0 {
                "SniperRifle"
            } else if dist > 800.0 {
                "ShockRifle"
            } else if dist > 400.0 {
                "RocketLauncher"
            } else {
                "FlakCannon"
            };

            if !occluded {
                visible_targets_per_node[i] += 1;
                visible_targets_per_node[j] += 1;

                if dist > max_sightline {
                    max_sightline = dist;
                }

                report.sightlines.push(CombatSightline {
                    from_node: i,
                    to_node: j,
                    from_pos: p_i,
                    to_pos: p_j,
                    distance_uu: dist,
                    is_direct_los: true,
                    recommended_weapon: weapon,
                });
            }
        }
    }

    report.longest_sightline_uu = max_sightline;

    // 2. Classify vantage points and elevation advantage
    for i in 0..n {
        let loc = nav_graph.nodes[i].location;
        let mut elevation_diff = 0.0;
        for j in 0..n {
            if i != j {
                let diff = loc.z - nav_graph.nodes[j].location.z;
                if diff > elevation_diff {
                    elevation_diff = diff;
                }
            }
        }

        let vis_count = visible_targets_per_node[i];
        let threat_score = (vis_count as f32 / (n.max(1) as f32) + (elevation_diff / 800.0).clamp(0.0, 0.5)).clamp(0.0, 1.0);

        let cover = if vis_count == 0 {
            CombatCoverRating::FullCover
        } else if vis_count <= 2 {
            CombatCoverRating::PartialCover
        } else {
            CombatCoverRating::Exposed
        };

        if elevation_diff >= 120.0 && vis_count > 0 {
            report.high_ground_perches_count += 1;
        }
        if cover == CombatCoverRating::Exposed {
            report.exposed_death_zones_count += 1;
        }

        report.vantage_points.push(CombatVantagePoint {
            node_index: i,
            location: loc,
            elevation_advantage_uu: elevation_diff,
            visible_enemies_count: vis_count,
            sniper_threat_score: threat_score,
            cover_rating: cover,
        });
    }

    report
}

/// Fast slab method for ray-AABB intersection test
fn ray_intersects_aabb(origin: Vec3, dir: Vec3, max_dist: f32, min: Vec3, max: Vec3) -> bool {
    let inv_d = Vec3::new(
        if dir.x.abs() > 1e-6 { 1.0 / dir.x } else { 1e6 },
        if dir.y.abs() > 1e-6 { 1.0 / dir.y } else { 1e6 },
        if dir.z.abs() > 1e-6 { 1.0 / dir.z } else { 1e6 },
    );

    let t0 = (min - origin) * inv_d;
    let t1 = (max - origin) * inv_d;

    let tmin = t0.min(t1).max_element();
    let tmax = t0.max(t1).min_element();

    tmax >= tmin && tmax > 0.0 && tmin < max_dist
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::build_cube;
    use crate::CsgOperation;

    #[test]
    fn test_bot_combat_sightlines_open() {
        let mut graph = NavGraph::new();
        graph.add_node("SniperPerch", Vec3::new(0.0, 0.0, 300.0), "PathNode");
        graph.add_node("GroundTarget", Vec3::new(1200.0, 0.0, 0.0), "PathNode");

        let brushes = vec![];
        let report = analyze_combat_sightlines(&graph, &brushes);

        assert_eq!(report.sightlines.len(), 1);
        assert!(report.sightlines[0].is_direct_los);
        assert_eq!(report.sightlines[0].recommended_weapon, "ShockRifle");
        assert_eq!(report.high_ground_perches_count, 1);
    }

    #[test]
    fn test_bot_combat_sightlines_occluded() {
        let mut graph = NavGraph::new();
        graph.add_node("RoomA", Vec3::new(-500.0, 0.0, 0.0), "PathNode");
        graph.add_node("RoomB", Vec3::new(500.0, 0.0, 0.0), "PathNode");

        // Solid wall placed in the center between RoomA and RoomB
        let wall = build_cube("CenterWall", CsgOperation::Add, Vec3::new(64.0, 400.0, 400.0));
        let brushes = vec![wall];

        let report = analyze_combat_sightlines(&graph, &brushes);
        assert_eq!(report.sightlines.len(), 0, "Wall must block direct sightline");
    }
}
