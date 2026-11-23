// crates/ut99-bsp/src/domination_studio.rs
//
// Interactive Domination Control Point & Point-Weight Triangle Studio
// Triangle geometry metrics (AB, BC, CA distances, travel times, triangle asymmetry ratio),
// vertical elevation disparity, point capture radius, scoring cadence telemetry,
// bot defense weighting, and procedural stepped capture pedestal brush generation.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical UT99 Domination Control Point Identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DominationPointId {
    PointA,
    PointB,
    PointC,
}

impl DominationPointId {
    pub fn display_name(&self) -> &'static str {
        match self {
            DominationPointId::PointA => "Control Point A (Alpha)",
            DominationPointId::PointB => "Control Point B (Beta)",
            DominationPointId::PointC => "Control Point C (Gamma)",
        }
    }

    pub fn letter(&self) -> &'static str {
        match self {
            DominationPointId::PointA => "A",
            DominationPointId::PointB => "B",
            DominationPointId::PointC => "C",
        }
    }
}

/// Archetypes of canonical Domination map point configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DominationArchetype {
    CrypticTriadBalance,
    CinderVerticalTower,
    GearsIndustrialHub,
    LethalCorridorSprint,
    SesmarTombPyramid,
}

impl DominationArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            DominationArchetype::CrypticTriadBalance => "DOM-Cryptic Equilateral Triad (Balanced 3-Point Layout)",
            DominationArchetype::CinderVerticalTower => "DOM-Cinder Vertical Disparity (High Spire vs Lava Basin)",
            DominationArchetype::GearsIndustrialHub => "DOM-Gears Central Furnace Hub (High-Risk Center Point)",
            DominationArchetype::LethalCorridorSprint => "DOM-Lethal Defense Gauntlet (Linear Chokepoint Sprint)",
            DominationArchetype::SesmarTombPyramid => "DOM-Sesmar Sacred Burial Chamber (Flanking Antechambers)",
        }
    }

    pub fn to_config(&self) -> DominationConfig {
        match self {
            DominationArchetype::CrypticTriadBalance => DominationConfig {
                archetype: *self,
                active_point: DominationPointId::PointA,
                position: Vec3::new(0.0, 0.0, 0.0),
                capture_radius_uu: 64.0,
                score_cadence_secs: 5.0,
                defense_priority: 1.0,
                pedestal_radius: 56.0,
                pedestal_height: 12.0,
                tag: "ControlPoint_A".to_string(),
            },
            DominationArchetype::CinderVerticalTower => DominationConfig {
                archetype: *self,
                active_point: DominationPointId::PointB,
                position: Vec3::new(450.0, 300.0, 240.0),
                capture_radius_uu: 60.0,
                score_cadence_secs: 5.0,
                defense_priority: 1.2,
                pedestal_radius: 50.0,
                pedestal_height: 14.0,
                tag: "CinderHighSpirePoint".to_string(),
            },
            DominationArchetype::GearsIndustrialHub => DominationConfig {
                archetype: *self,
                active_point: DominationPointId::PointC,
                position: Vec3::new(0.0, 0.0, -120.0),
                capture_radius_uu: 72.0,
                score_cadence_secs: 4.0,
                defense_priority: 1.5,
                pedestal_radius: 64.0,
                pedestal_height: 10.0,
                tag: "GearsCentralFurnace".to_string(),
            },
            DominationArchetype::LethalCorridorSprint => DominationConfig {
                archetype: *self,
                active_point: DominationPointId::PointA,
                position: Vec3::new(-600.0, 0.0, 0.0),
                capture_radius_uu: 56.0,
                score_cadence_secs: 5.0,
                defense_priority: 0.9,
                pedestal_radius: 48.0,
                pedestal_height: 12.0,
                tag: "LethalWestCorridor".to_string(),
            },
            DominationArchetype::SesmarTombPyramid => DominationConfig {
                archetype: *self,
                active_point: DominationPointId::PointB,
                position: Vec3::new(0.0, 500.0, 80.0),
                capture_radius_uu: 64.0,
                score_cadence_secs: 5.0,
                defense_priority: 1.1,
                pedestal_radius: 56.0,
                pedestal_height: 16.0,
                tag: "SesmarTombAltar".to_string(),
            },
        }
    }
}

/// Configuration settings for Domination Control Point Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DominationConfig {
    pub archetype: DominationArchetype,
    pub active_point: DominationPointId,
    pub position: Vec3,
    pub capture_radius_uu: f32,
    pub score_cadence_secs: f32,
    pub defense_priority: f32,
    pub pedestal_radius: f32,
    pub pedestal_height: f32,
    pub tag: String,
}

impl Default for DominationConfig {
    fn default() -> Self {
        DominationArchetype::CrypticTriadBalance.to_config()
    }
}

/// Representation of an existing ControlPoint in the level
#[derive(Debug, Clone, PartialEq)]
pub struct LevelControlPointRecord {
    pub name: String,
    pub point_id: DominationPointId,
    pub position: Vec3,
    pub score_cadence_secs: f32,
}

/// Analytical metrics for Domination triangle balance
#[derive(Debug, Clone, PartialEq)]
pub struct DominationMetrics {
    pub total_points_in_level: usize,
    pub dist_to_point_b: f32,
    pub dist_to_point_c: f32,
    pub sprint_time_to_point_b_secs: f32,
    pub sprint_time_to_point_c_secs: f32,
    pub triangle_asymmetry_ratio: f32, // 0.0 to 100.0%
    pub vertical_elevation_disparity_uu: f32,
    pub points_per_minute: f32,
}

/// Balance and safety hazards for Domination setups
#[derive(Debug, Clone, PartialEq)]
pub struct DominationHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates Domination triangle geometry and scoring telemetry
pub fn evaluate_domination_metrics(
    config: &DominationConfig,
    points: &[LevelControlPointRecord],
) -> DominationMetrics {
    let mut total_pts = 1; // including current config
    let mut dist_b = 9999.0_f32;
    let mut dist_c = 9999.0_f32;
    let mut z_min = config.position.z;
    let mut z_max = config.position.z;

    let mut dist_list = Vec::new();

    for pt in points {
        total_pts += 1;
        let d = pt.position.distance(config.position);
        dist_list.push(d);
        if d < dist_b {
            dist_c = dist_b;
            dist_b = d;
        } else if d < dist_c {
            dist_c = d;
        }
        if pt.position.z < z_min {
            z_min = pt.position.z;
        }
        if pt.position.z > z_max {
            z_max = pt.position.z;
        }
    }

    let sprint_b = if dist_b < 9000.0 { dist_b / 400.0 } else { 0.0 };
    let sprint_c = if dist_c < 9000.0 { dist_c / 400.0 } else { 0.0 };

    let mut asymmetry = 0.0_f32;
    if dist_b < 9000.0 && dist_c < 9000.0 {
        let diff = (dist_b - dist_c).abs();
        let mean = (dist_b + dist_c) * 0.5;
        if mean > 1.0 {
            asymmetry = (diff / mean) * 100.0;
        }
    }

    let ppm = 60.0 / config.score_cadence_secs.max(1.0);

    DominationMetrics {
        total_points_in_level: total_pts,
        dist_to_point_b: dist_b,
        dist_to_point_c: dist_c,
        sprint_time_to_point_b_secs: sprint_b,
        sprint_time_to_point_c_secs: sprint_c,
        triangle_asymmetry_ratio: asymmetry,
        vertical_elevation_disparity_uu: (z_max - z_min).abs(),
        points_per_minute: ppm,
    }
}

/// Audits Domination setup for proximity spam, extreme asymmetry and broken scoring
pub fn audit_domination_safety(
    config: &DominationConfig,
    points: &[LevelControlPointRecord],
) -> Vec<DominationHazard> {
    let mut hazards = Vec::new();

    // 1. Critical Proximity Spam Hazard: Another point < 500 uu
    for pt in points {
        let dist = pt.position.distance(config.position);
        if dist < 500.0 && dist > 1.0 {
            hazards.push(DominationHazard {
                title: "Critical Control Point Proximity Spam".to_string(),
                description: format!(
                    "Control Point is only {:.1} uu from '{}'. Minimum tournament distance is 500 uu to prevent rapid cap spam and spawn defense locks.",
                    dist, pt.name
                ),
                is_critical: true,
            });
            break;
        }
    }

    // 2. High Triangle Asymmetry Warning: Asymmetry > 35%
    let metrics = evaluate_domination_metrics(config, points);
    if metrics.triangle_asymmetry_ratio > 35.0 {
        hazards.push(DominationHazard {
            title: "Severe Domination Triangle Asymmetry".to_string(),
            description: format!(
                "Travel time asymmetry between control points is {:.1}%. Unequal path distances often lead to degenerate 2-point camping dominance.",
                metrics.triangle_asymmetry_ratio
            ),
            is_critical: false,
        });
    }

    // 3. Cadence Warning: Score cadence < 3.0s
    if config.score_cadence_secs < 3.0 {
        hazards.push(DominationHazard {
            title: "Excessively Rapid Scoring Cadence".to_string(),
            description: format!(
                "Scoring interval of {:.1}s generates {:.0} points/minute. Tournament standard is 5.0s per point award.",
                config.score_cadence_secs, metrics.points_per_minute
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a stepped octagonal Domination capture pedestal brush
pub fn generate_domination_pedestal_brush(config: &DominationConfig) -> Brush {
    let mut brush = Brush::new("DominationPedestal".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let r_base = config.pedestal_radius;
    let r_top = config.pedestal_radius * 0.85;
    let h = config.pedestal_height;
    let num_sides = 8;

    let mut top_ring = Vec::new();
    let mut bot_ring = Vec::new();

    for i in 0..num_sides {
        let angle = std::f32::consts::TAU * (i as f32) / (num_sides as f32);
        let x_top = r_top * angle.cos();
        let y_top = r_top * angle.sin();
        let x_bot = r_base * angle.cos();
        let y_bot = r_base * angle.sin();

        top_ring.push(Vec3::new(x_top, y_top, h));
        bot_ring.push(Vec3::new(x_bot, y_bot, 0.0));
    }

    // Top cap
    if let Some(poly) = Polygon::new(top_ring.clone()) {
        brush.polys.push(poly);
    }

    // Bottom cap (reversed winding)
    let mut bot_rev = bot_ring.clone();
    bot_rev.reverse();
    if let Some(poly) = Polygon::new(bot_rev) {
        brush.polys.push(poly);
    }

    // Side quads
    for i in 0..num_sides {
        let next = (i + 1) % num_sides;
        let quad = vec![bot_ring[i], bot_ring[next], top_ring[next], top_ring[i]];
        if let Some(poly) = Polygon::new(quad) {
            brush.polys.push(poly);
        }
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domination_archetypes_and_defaults() {
        let cfg = DominationArchetype::CrypticTriadBalance.to_config();
        assert_eq!(cfg.active_point, DominationPointId::PointA);
        assert_eq!(cfg.score_cadence_secs, 5.0);
        assert_eq!(cfg.capture_radius_uu, 64.0);
        assert!(cfg.pedestal_radius >= 40.0);
    }

    #[test]
    fn test_domination_metrics_evaluation() {
        let cfg = DominationConfig::default();
        let points = vec![
            LevelControlPointRecord {
                name: "PointB".to_string(),
                point_id: DominationPointId::PointB,
                position: Vec3::new(800.0, 0.0, 0.0),
                score_cadence_secs: 5.0,
            },
            LevelControlPointRecord {
                name: "PointC".to_string(),
                point_id: DominationPointId::PointC,
                position: Vec3::new(0.0, 800.0, 0.0),
                score_cadence_secs: 5.0,
            },
        ];

        let metrics = evaluate_domination_metrics(&cfg, &points);
        assert_eq!(metrics.total_points_in_level, 3);
        assert_eq!(metrics.dist_to_point_b, 800.0);
        assert_eq!(metrics.dist_to_point_c, 800.0);
        assert_eq!(metrics.sprint_time_to_point_b_secs, 2.0); // 800 / 400 = 2.0s
        assert_eq!(metrics.triangle_asymmetry_ratio, 0.0); // perfectly symmetric
        assert_eq!(metrics.points_per_minute, 12.0); // 60 / 5 = 12
    }

    #[test]
    fn test_domination_hazard_auditing() {
        let cfg = DominationConfig::default();
        let points = vec![LevelControlPointRecord {
            name: "AdjacentPoint".to_string(),
            point_id: DominationPointId::PointB,
            position: Vec3::new(300.0, 0.0, 0.0), // < 500 uu! Proximity spam
            score_cadence_secs: 5.0,
        }];

        let hazards = audit_domination_safety(&cfg, &points);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Proximity Spam")));
    }

    #[test]
    fn test_generate_domination_pedestal_brush() {
        let cfg = DominationConfig::default();
        let brush = generate_domination_pedestal_brush(&cfg);

        assert_eq!(brush.name, "DominationPedestal");
        assert_eq!(brush.polys.len(), 10); // 8 sides + 2 caps
    }
}
