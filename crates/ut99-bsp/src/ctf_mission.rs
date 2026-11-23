// crates/ut99-bsp/src/ctf_mission.rs
//
// Procedural Capture The Flag (CTF-) Mission & Polar Route Solver.
// Reverse-engineered from forensic study of all 26 official UT99 CTF maps:
// - Automatic selection of opposed polar FlagBase rooms maximizing topological separation
// - Multi-lane route analysis with balanced AlternatePath networks (Team 0 & Team 1)
// - Strategic Bot Defense hierarchy (Goal Guard, Chokepoint Sentries, High Sniper Overwatch)
// - Dedicated Sniper Perches with SniperRifle + RifleAmmo pickups on high vantage decks
// - Symmetrical contested center mega-powerup (UT_DamageAmplifier / UT_ShieldBelt)
// - Red / Blue / Midfield Chromatic Zoning parameters

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Spatial representation of a room or corridor node for CTF mission planning
#[derive(Debug, Clone, PartialEq)]
pub struct CtfNode {
    pub name: String,
    pub center: Vec3,
    pub size: Vec3,
    pub is_corridor: bool,
    pub team_owner: Option<u8>,
}

impl CtfNode {
    pub fn floor_z(&self) -> f32 {
        self.center.z - self.size.z * 0.5
    }

    pub fn ceiling_z(&self) -> f32 {
        self.center.z + self.size.z * 0.5
    }
}

/// Architectural dais style for FlagBase pedestals
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CtfDaisStyle {
    #[default]
    SteppedTeamPedestal,
    AltarShrine,
    TechBunkerAlcove,
    PillarCanopy,
}

impl CtfDaisStyle {
    pub fn all() -> &'static [CtfDaisStyle] {
        &[
            Self::SteppedTeamPedestal,
            Self::AltarShrine,
            Self::TechBunkerAlcove,
            Self::PillarCanopy,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::SteppedTeamPedestal => "Stepped Octagonal Pedestal (2-Tier Trim)",
            Self::AltarShrine => "Altar Shrine (Pillars & Colored Torchlight)",
            Self::TechBunkerAlcove => "Tech Bunker Alcove (Armored Wall Recess)",
            Self::PillarCanopy => "Pillar Canopy (4 Columns & Overwatch Deck)",
        }
    }
}

/// Tactical route strategy for bot flag transport
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CtfRouteStrategy {
    #[default]
    MultiLaneDirectAndFlanks,
    ParallelTrenches,
    VerticalThreeTier,
}

impl CtfRouteStrategy {
    pub fn all() -> &'static [CtfRouteStrategy] {
        &[
            Self::MultiLaneDirectAndFlanks,
            Self::ParallelTrenches,
            Self::VerticalThreeTier,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::MultiLaneDirectAndFlanks => "Multi-Lane (Main Concourse + Flanking Tunnels)",
            Self::ParallelTrenches => "Parallel Trenches (Twin Direct Lanes + Midfield Bunker)",
            Self::VerticalThreeTier => "3-Tier Vertical (Ground Catwalks + High Bridge + Sump)",
        }
    }
}

/// A bot navigation marker directing flag carriers along specific paths
#[derive(Debug, Clone, PartialEq)]
pub struct CtfAlternatePath {
    pub team: u8,
    pub weight: f32,
    pub return_only: bool,
    pub location: Vec3,
}

/// Tactical defense waypoint for bots guarding base and perimeter
#[derive(Debug, Clone, PartialEq)]
pub struct CtfDefensePoint {
    pub name: String,
    pub team: u8,
    pub priority: u8, // 1 = Sniper/Overwatch, 2 = Chokepoint/Door, 3 = Goal Guard
    pub is_sniping: bool,
    pub location: Vec3,
    pub yaw: i32,
}

/// Elevated sniping vantage overlooking the midfield
#[derive(Debug, Clone, PartialEq)]
pub struct CtfSniperPerch {
    pub team: u8,
    pub perch_location: Vec3,
    pub defense_point_loc: Vec3,
    pub weapon_pickup_loc: Vec3,
    pub yaw: i32,
}

/// Complete planned layout for a Capture The Flag arena
#[derive(Debug, Clone, PartialEq)]
pub struct CtfMissionPlan {
    pub red_base_node_index: usize,
    pub blue_base_node_index: usize,
    pub red_flag_loc: Vec3,
    pub blue_flag_loc: Vec3,
    pub flag_to_flag_distance: f32,
    pub dais_style: CtfDaisStyle,
    pub alternate_paths: Vec<CtfAlternatePath>,
    pub defense_points: Vec<CtfDefensePoint>,
    pub sniper_perches: Vec<CtfSniperPerch>,
    pub center_powerup_loc: Option<Vec3>,
    pub center_powerup_class: String,
    pub forward_powerup_red: Option<Vec3>,
    pub forward_powerup_blue: Option<Vec3>,
}

/// Computes the complete deterministic CTF mission plan from arena nodes.
pub fn plan_ctf_mission(
    nodes: &[CtfNode],
    dais_style: CtfDaisStyle,
    include_snipers: bool,
    contested_center_powerup: bool,
    seed: u64,
) -> CtfMissionPlan {
    // 1. Identify Red Base (Team 0) and Blue Base (Team 1)
    let non_corridor_rooms: Vec<(usize, &CtfNode)> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| !n.is_corridor)
        .collect();

    let candidate_rooms: Vec<(usize, &CtfNode)> = if non_corridor_rooms.len() >= 2 {
        non_corridor_rooms
    } else {
        nodes.iter().enumerate().collect()
    };

    // Check if any rooms already have explicit team owners
    let red_explicit = candidate_rooms.iter().find(|(_, n)| n.team_owner == Some(0));
    let blue_explicit = candidate_rooms.iter().find(|(_, n)| n.team_owner == Some(1));

    let (red_idx, blue_idx) = if let (Some(&(r_idx, _)), Some(&(b_idx, _))) = (red_explicit, blue_explicit) {
        (r_idx, b_idx)
    } else {
        // Find the pair of rooms that maximizes Euclidean distance
        let mut best_pair = (candidate_rooms[0].0, candidate_rooms.last().map(|c| c.0).unwrap_or(0));
        let mut max_dist = 0.0f32;

        for i in 0..candidate_rooms.len() {
            for j in (i + 1)..candidate_rooms.len() {
                let dist = candidate_rooms[i].1.center.distance(candidate_rooms[j].1.center);
                if dist > max_dist {
                    max_dist = dist;
                    best_pair = (candidate_rooms[i].0, candidate_rooms[j].0);
                }
            }
        }

        // Symmetrically assign Red to lower X (or lower Y if X is tied) and Blue to opposite
        let node_a = &nodes[best_pair.0];
        let node_b = &nodes[best_pair.1];

        if node_a.center.x < node_b.center.x || (node_a.center.x == node_b.center.x && node_a.center.y < node_b.center.y) {
            (best_pair.0, best_pair.1)
        } else {
            (best_pair.1, best_pair.0)
        }
    };

    let red_node = &nodes[red_idx];
    let blue_node = &nodes[blue_idx];

    // Compute Flag locations on top of dais
    let dais_height = match dais_style {
        CtfDaisStyle::SteppedTeamPedestal => 24.0,
        CtfDaisStyle::AltarShrine => 24.0,
        CtfDaisStyle::TechBunkerAlcove => 16.0,
        CtfDaisStyle::PillarCanopy => 12.0,
    };

    // FlagBase has CollisionHeight = 60.0. To rest flush on the dais floor with 0 UU
    // vertical penetration, Location.z must be room_fz + dais_height + 60.0.
    let red_flag_loc = Vec3::new(red_node.center.x, red_node.center.y, red_node.floor_z() + dais_height + 60.0);
    let blue_flag_loc = Vec3::new(blue_node.center.x, blue_node.center.y, blue_node.floor_z() + dais_height + 60.0);
    let flag_to_flag_distance = red_flag_loc.distance(blue_flag_loc);

    // Vector pointing from Red Base toward Blue Base
    let to_blue = (blue_flag_loc - red_flag_loc).normalize_or_zero();
    let red_yaw = ((to_blue.y.atan2(to_blue.x) * 65536.0 / (2.0 * std::f32::consts::PI)) as i32) & 65535;
    let blue_yaw = (red_yaw + 32768) & 65535;

    // 2. Tactical Defense Points (Goal Guard + Chokepoints)
    let mut defense_points = Vec::new();

    // Red Base DefensePoints
    // A) Goal Guard: standing close to flag facing enemy approach
    defense_points.push(CtfDefensePoint {
        name: "DefensePoint_Red_Goal".to_string(),
        team: 0,
        priority: 3,
        is_sniping: false,
        location: Vec3::new(
            red_flag_loc.x + to_blue.x * 128.0,
            red_flag_loc.y + to_blue.y * 128.0,
            red_node.floor_z() + 48.0,
        ),
        yaw: red_yaw,
    });

    // B) Two Chokepoint Sentries at room exit flanks
    let perp_vector = Vec3::new(-to_blue.y, to_blue.x, 0.0);
    let exit_dist = (red_node.size.x.min(red_node.size.y) * 0.35).clamp(160.0, 320.0);
    let flank_spread = (red_node.size.x.min(red_node.size.y) * 0.28).clamp(128.0, 256.0);

    defense_points.push(CtfDefensePoint {
        name: "DefensePoint_Red_ChokeL".to_string(),
        team: 0,
        priority: 2,
        is_sniping: false,
        location: Vec3::new(
            red_flag_loc.x + to_blue.x * exit_dist + perp_vector.x * flank_spread,
            red_flag_loc.y + to_blue.y * exit_dist + perp_vector.y * flank_spread,
            red_node.floor_z() + 48.0,
        ),
        yaw: red_yaw,
    });
    defense_points.push(CtfDefensePoint {
        name: "DefensePoint_Red_ChokeR".to_string(),
        team: 0,
        priority: 2,
        is_sniping: false,
        location: Vec3::new(
            red_flag_loc.x + to_blue.x * exit_dist - perp_vector.x * flank_spread,
            red_flag_loc.y + to_blue.y * exit_dist - perp_vector.y * flank_spread,
            red_node.floor_z() + 48.0,
        ),
        yaw: red_yaw,
    });

    // Blue Base DefensePoints
    // A) Goal Guard
    defense_points.push(CtfDefensePoint {
        name: "DefensePoint_Blue_Goal".to_string(),
        team: 1,
        priority: 3,
        is_sniping: false,
        location: Vec3::new(
            blue_flag_loc.x - to_blue.x * 128.0,
            blue_flag_loc.y - to_blue.y * 128.0,
            blue_node.floor_z() + 48.0,
        ),
        yaw: blue_yaw,
    });

    // B) Two Chokepoint Sentries
    let blue_exit_dist = (blue_node.size.x.min(blue_node.size.y) * 0.35).clamp(160.0, 320.0);
    let blue_flank_spread = (blue_node.size.x.min(blue_node.size.y) * 0.28).clamp(128.0, 256.0);

    defense_points.push(CtfDefensePoint {
        name: "DefensePoint_Blue_ChokeL".to_string(),
        team: 1,
        priority: 2,
        is_sniping: false,
        location: Vec3::new(
            blue_flag_loc.x - to_blue.x * blue_exit_dist + perp_vector.x * blue_flank_spread,
            blue_flag_loc.y - to_blue.y * blue_exit_dist + perp_vector.y * blue_flank_spread,
            blue_node.floor_z() + 48.0,
        ),
        yaw: blue_yaw,
    });
    defense_points.push(CtfDefensePoint {
        name: "DefensePoint_Blue_ChokeR".to_string(),
        team: 1,
        priority: 2,
        is_sniping: false,
        location: Vec3::new(
            blue_flag_loc.x - to_blue.x * blue_exit_dist - perp_vector.x * blue_flank_spread,
            blue_flag_loc.y - to_blue.y * blue_exit_dist - perp_vector.y * blue_flank_spread,
            blue_node.floor_z() + 48.0,
        ),
        yaw: blue_yaw,
    });

    // 3. Sniper Perches & Rifles
    let mut sniper_perches = Vec::new();
    if include_snipers {
        // Red Sniper Overwatch: prioritize dedicated sniper roost if one exists
        let red_roost = nodes.iter().find(|n| {
            !n.is_corridor
                && (n.team_owner == Some(0) || n.name.to_ascii_lowercase().contains("red"))
                && (n.name.to_ascii_lowercase().contains("roost") || n.name.to_ascii_lowercase().contains("sniper"))
        });

        let red_snip_loc = if let Some(roost) = red_roost {
            Vec3::new(roost.center.x, roost.center.y, roost.floor_z() + 32.0)
        } else {
            let red_snip_z = red_node.floor_z() + 48.0;
            Vec3::new(
                red_flag_loc.x - to_blue.x * (red_node.size.x * 0.28).clamp(96.0, 220.0),
                red_flag_loc.y - to_blue.y * (red_node.size.y * 0.28).clamp(96.0, 220.0),
                red_snip_z,
            )
        };
        sniper_perches.push(CtfSniperPerch {
            team: 0,
            perch_location: red_snip_loc,
            defense_point_loc: red_snip_loc + Vec3::new(0.0, 0.0, 16.0),
            weapon_pickup_loc: red_snip_loc + perp_vector * 48.0,
            yaw: red_yaw,
        });
        defense_points.push(CtfDefensePoint {
            name: "DefensePoint_Red_Sniper".to_string(),
            team: 0,
            priority: 1,
            is_sniping: true,
            location: red_snip_loc + Vec3::new(0.0, 0.0, 16.0),
            yaw: red_yaw,
        });

        // Blue Sniper Overwatch: prioritize dedicated sniper roost if one exists
        let blue_roost = nodes.iter().find(|n| {
            !n.is_corridor
                && (n.team_owner == Some(1) || n.name.to_ascii_lowercase().contains("blue"))
                && (n.name.to_ascii_lowercase().contains("roost") || n.name.to_ascii_lowercase().contains("sniper"))
        });

        let blue_snip_loc = if let Some(roost) = blue_roost {
            Vec3::new(roost.center.x, roost.center.y, roost.floor_z() + 32.0)
        } else {
            let blue_snip_z = blue_node.floor_z() + 48.0;
            Vec3::new(
                blue_flag_loc.x + to_blue.x * (blue_node.size.x * 0.28).clamp(96.0, 220.0),
                blue_flag_loc.y + to_blue.y * (blue_node.size.y * 0.28).clamp(96.0, 220.0),
                blue_snip_z,
            )
        };
        sniper_perches.push(CtfSniperPerch {
            team: 1,
            perch_location: blue_snip_loc,
            defense_point_loc: blue_snip_loc + Vec3::new(0.0, 0.0, 16.0),
            weapon_pickup_loc: blue_snip_loc - perp_vector * 48.0,
            yaw: blue_yaw,
        });
        defense_points.push(CtfDefensePoint {
            name: "DefensePoint_Blue_Sniper".to_string(),
            team: 1,
            priority: 1,
            is_sniping: true,
            location: blue_snip_loc + Vec3::new(0.0, 0.0, 16.0),
            yaw: blue_yaw,
        });
    }

    // 4. Multi-Lane AlternatePath Network
    let mut alternate_paths = Vec::new();
    let corridors: Vec<(usize, &CtfNode)> = nodes.iter().enumerate().filter(|(_, n)| n.is_corridor).collect();

    if !corridors.is_empty() {
        for (_, c) in &corridors {
            let fz = c.floor_z() + 48.0;
            // Distinguish lateral flank corridors vs central conduits
            let is_flank = c.center.y.abs() > 192.0 || c.center.x.abs() > 192.0;
            let is_deep = c.center.z < (red_flag_loc.z.min(blue_flag_loc.z) - 64.0);

            let (weight, return_only) = if is_deep {
                // Subterranean bypass: high weight return route
                (2.4, true)
            } else if is_flank {
                // Flanking corridor: favored alternative path
                (1.8, false)
            } else {
                // Central primary conduit
                (1.0, false)
            };

            // Every valid route needs AlternatePath for BOTH teams
            alternate_paths.push(CtfAlternatePath {
                team: 0,
                weight,
                return_only,
                location: Vec3::new(c.center.x, c.center.y, fz),
            });
            alternate_paths.push(CtfAlternatePath {
                team: 1,
                weight,
                return_only,
                location: Vec3::new(c.center.x, c.center.y, fz),
            });
        }
    } else {
        // Fallback for maps without corridors: place waypoint markers at mid-points
        let mid_pt = (red_flag_loc + blue_flag_loc) * 0.5;
        let perp = perp_vector * 384.0;
        let fz = red_node.floor_z().max(blue_node.floor_z()) + 48.0;

        for (team, ret_only, w) in [(0, false, 1.0), (1, false, 1.0), (0, true, 2.0), (1, true, 2.0)] {
            let offset = if ret_only { perp } else { -perp };
            alternate_paths.push(CtfAlternatePath {
                team,
                weight: w,
                return_only: ret_only,
                location: Vec3::new(mid_pt.x + offset.x, mid_pt.y + offset.y, fz),
            });
        }
    }

    // 5. Contested Center Mega-Powerup & Forward Base Armor
    let (center_powerup_loc, center_powerup_class) = if contested_center_powerup {
        let mid_pt = (red_flag_loc + blue_flag_loc) * 0.5;
        // Find node closest to mid_pt
        let nearest_node = nodes.iter().min_by(|a, b| {
            let da = a.center.distance(mid_pt);
            let db = b.center.distance(mid_pt);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });

        let p_loc = if let Some(n) = nearest_node {
            Vec3::new(mid_pt.x, mid_pt.y, n.floor_z() + 36.0)
        } else {
            Vec3::new(mid_pt.x, mid_pt.y, red_node.floor_z() + 36.0)
        };

        let p_class = if (seed % 2) == 0 {
            "UDamage".to_string()
        } else {
            "UT_ShieldBelt".to_string()
        };

        (Some(p_loc), p_class)
    } else {
        (None, String::new())
    };

    // Forward armor pickups symmetrically in each base
    let forward_dist = (red_node.size.x.min(red_node.size.y) * 0.32).clamp(128.0, 256.0);
    let forward_powerup_red = Some(Vec3::new(
        red_flag_loc.x + to_blue.x * forward_dist,
        red_flag_loc.y + to_blue.y * forward_dist,
        red_node.floor_z() + 32.0,
    ));
    let forward_powerup_blue = Some(Vec3::new(
        blue_flag_loc.x - to_blue.x * forward_dist,
        blue_flag_loc.y - to_blue.y * forward_dist,
        blue_node.floor_z() + 32.0,
    ));

    CtfMissionPlan {
        red_base_node_index: red_idx,
        blue_base_node_index: blue_idx,
        red_flag_loc,
        blue_flag_loc,
        flag_to_flag_distance,
        dais_style,
        alternate_paths,
        defense_points,
        sniper_perches,
        center_powerup_loc,
        center_powerup_class,
        forward_powerup_red,
        forward_powerup_blue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctf_polar_solver_finds_maximum_distance() {
        let nodes = vec![
            CtfNode {
                name: "WestFlag".to_string(),
                center: Vec3::new(-3000.0, 0.0, 0.0),
                size: Vec3::new(1024.0, 1024.0, 512.0),
                is_corridor: false,
                team_owner: None,
            },
            CtfNode {
                name: "MidCrux".to_string(),
                center: Vec3::new(0.0, 0.0, 0.0),
                size: Vec3::new(1024.0, 1024.0, 512.0),
                is_corridor: false,
                team_owner: None,
            },
            CtfNode {
                name: "EastFlag".to_string(),
                center: Vec3::new(3000.0, 0.0, 0.0),
                size: Vec3::new(1024.0, 1024.0, 512.0),
                is_corridor: false,
                team_owner: None,
            },
        ];

        let plan = plan_ctf_mission(&nodes, CtfDaisStyle::SteppedTeamPedestal, true, true, 42);

        // Nodes 0 and 2 are furthest apart (distance 6000 UU)
        assert_eq!(plan.red_base_node_index, 0);
        assert_eq!(plan.blue_base_node_index, 2);
        assert!(plan.flag_to_flag_distance >= 5900.0);
        assert!(plan.center_powerup_loc.is_some());
    }

    #[test]
    fn test_ctf_defense_points_and_alternate_paths_symmetry() {
        let nodes = vec![
            CtfNode {
                name: "RedBase".to_string(),
                center: Vec3::new(-2000.0, 0.0, 0.0),
                size: Vec3::new(896.0, 896.0, 480.0),
                is_corridor: false,
                team_owner: Some(0),
            },
            CtfNode {
                name: "BlueBase".to_string(),
                center: Vec3::new(2000.0, 0.0, 0.0),
                size: Vec3::new(896.0, 896.0, 480.0),
                is_corridor: false,
                team_owner: Some(1),
            },
            CtfNode {
                name: "NorthBridge".to_string(),
                center: Vec3::new(0.0, 512.0, 0.0),
                size: Vec3::new(3072.0, 384.0, 384.0),
                is_corridor: true,
                team_owner: None,
            },
            CtfNode {
                name: "SouthTunnel".to_string(),
                center: Vec3::new(0.0, -512.0, -128.0),
                size: Vec3::new(3072.0, 384.0, 384.0),
                is_corridor: true,
                team_owner: None,
            },
        ];

        let plan = plan_ctf_mission(&nodes, CtfDaisStyle::AltarShrine, true, true, 101);

        // Verify DefensePoints exist for both teams
        let red_dp: Vec<_> = plan.defense_points.iter().filter(|d| d.team == 0).collect();
        let blue_dp: Vec<_> = plan.defense_points.iter().filter(|d| d.team == 1).collect();
        assert_eq!(red_dp.len(), blue_dp.len(), "DefensePoint counts must be symmetric");
        assert!(red_dp.len() >= 4, "Must have Goal, 2 Chokepoints and Sniper");

        // Verify AlternatePaths exist for both teams
        let red_ap: Vec<_> = plan.alternate_paths.iter().filter(|a| a.team == 0).collect();
        let blue_ap: Vec<_> = plan.alternate_paths.iter().filter(|a| a.team == 1).collect();
        assert_eq!(red_ap.len(), blue_ap.len(), "AlternatePath counts must be symmetric");
        assert!(red_ap.len() >= 2, "Must have multiple route lanes");

        // Verify Snipers
        assert_eq!(plan.sniper_perches.len(), 2, "Must have 1 Red and 1 Blue sniper perch");
    }
}
