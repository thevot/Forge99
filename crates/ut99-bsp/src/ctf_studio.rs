// crates/ut99-bsp/src/ctf_studio.rs
//
// Interactive Capture The Flag (CTF) Flag Base & Transit Routes Studio
// Flag base placement, flag-to-flag distance metrics, sprint/translocator transit times,
// route symmetry auditing, defense radius visualization, and procedural stepped flag pedestal brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// CTF Team identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CTFTeam {
    Red,
    Blue,
    Green,
    Gold,
}

impl CTFTeam {
    pub fn display_name(&self) -> &'static str {
        match self {
            CTFTeam::Red => "Red Team (0)",
            CTFTeam::Blue => "Blue Team (1)",
            CTFTeam::Green => "Green Team (2)",
            CTFTeam::Gold => "Gold Team (3)",
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            CTFTeam::Red => 0,
            CTFTeam::Blue => 1,
            CTFTeam::Green => 2,
            CTFTeam::Gold => 3,
        }
    }

    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => CTFTeam::Red,
            1 => CTFTeam::Blue,
            2 => CTFTeam::Green,
            _ => CTFTeam::Gold,
        }
    }

    pub fn actor_class_name(&self) -> &'static str {
        match self {
            CTFTeam::Red => "Botpack.RedFlagBase",
            CTFTeam::Blue => "Botpack.BlueFlagBase",
            CTFTeam::Green => "Botpack.GreenFlagBase",
            CTFTeam::Gold => "Botpack.GoldFlagBase",
        }
    }
}

/// Archetypes of canonical CTF bases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CtfBaseArchetype {
    FaceMonolithBase,
    NovemberSubmarineDeck,
    CoretCryoChamber,
    GauntletFortressDais,
    DrearyIndustrialGutter,
}

impl CtfBaseArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            CtfBaseArchetype::FaceMonolithBase => "Facing Worlds Monolith (Long-Range Sniper Vista)",
            CtfBaseArchetype::NovemberSubmarineDeck => "November Submarine Deck (Naval Hull Dock)",
            CtfBaseArchetype::CoretCryoChamber => "Coret Facility Cryo-Chamber (Airlock Chokepoints)",
            CtfBaseArchetype::GauntletFortressDais => "The Gauntlet Fortress Dais (Tiered Ramp Ramparts)",
            CtfBaseArchetype::DrearyIndustrialGutter => "Dreary Acid Gutter (Subterranean Tight Quarters)",
        }
    }

    pub fn to_config(&self) -> CtfBaseConfig {
        match self {
            CtfBaseArchetype::FaceMonolithBase => CtfBaseConfig {
                archetype: *self,
                team: CTFTeam::Red,
                position: Vec3::new(0.0, 0.0, 0.0),
                defense_radius_uu: 1200.0,
                flag_return_timeout_secs: 25.0,
                pedestal_radius: 64.0,
                pedestal_height: 24.0,
                tag: "RedFlagBase".to_string(),
                defense_point_count: 4,
            },
            CtfBaseArchetype::NovemberSubmarineDeck => CtfBaseConfig {
                archetype: *self,
                team: CTFTeam::Blue,
                position: Vec3::new(0.0, 0.0, 16.0),
                defense_radius_uu: 900.0,
                flag_return_timeout_secs: 25.0,
                pedestal_radius: 56.0,
                pedestal_height: 20.0,
                tag: "BlueFlagBase".to_string(),
                defense_point_count: 3,
            },
            CtfBaseArchetype::CoretCryoChamber => CtfBaseConfig {
                archetype: *self,
                team: CTFTeam::Red,
                position: Vec3::new(0.0, 0.0, 0.0),
                defense_radius_uu: 800.0,
                flag_return_timeout_secs: 30.0,
                pedestal_radius: 72.0,
                pedestal_height: 28.0,
                tag: "RedCryoBase".to_string(),
                defense_point_count: 4,
            },
            CtfBaseArchetype::GauntletFortressDais => CtfBaseConfig {
                archetype: *self,
                team: CTFTeam::Blue,
                position: Vec3::new(0.0, 0.0, 32.0),
                defense_radius_uu: 1100.0,
                flag_return_timeout_secs: 25.0,
                pedestal_radius: 80.0,
                pedestal_height: 32.0,
                tag: "BlueFortressBase".to_string(),
                defense_point_count: 5,
            },
            CtfBaseArchetype::DrearyIndustrialGutter => CtfBaseConfig {
                archetype: *self,
                team: CTFTeam::Red,
                position: Vec3::new(0.0, 0.0, -16.0),
                defense_radius_uu: 650.0,
                flag_return_timeout_secs: 20.0,
                pedestal_radius: 48.0,
                pedestal_height: 16.0,
                tag: "RedGutterBase".to_string(),
                defense_point_count: 3,
            },
        }
    }
}

/// Configuration settings for CTF Flag Base
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CtfBaseConfig {
    pub archetype: CtfBaseArchetype,
    pub team: CTFTeam,
    pub position: Vec3,
    pub defense_radius_uu: f32,
    pub flag_return_timeout_secs: f32,
    pub pedestal_radius: f32,
    pub pedestal_height: f32,
    pub tag: String,
    pub defense_point_count: usize,
}

impl Default for CtfBaseConfig {
    fn default() -> Self {
        CtfBaseArchetype::FaceMonolithBase.to_config()
    }
}

/// Representation of an existing Flag Base in the level
#[derive(Debug, Clone, PartialEq)]
pub struct LevelFlagBaseRecord {
    pub name: String,
    pub team: CTFTeam,
    pub position: Vec3,
}

/// Telemetry metrics for CTF base and routes
#[derive(Debug, Clone, PartialEq)]
pub struct CtfBaseMetrics {
    pub flag_to_flag_distance_uu: f32,
    pub runner_transit_time_secs: f32,
    pub translocator_transit_time_secs: f32,
    pub total_flag_bases_in_level: usize,
    pub defense_coverage_area_sq_uu: f32,
}

/// Safety diagnostics and competitive balance warnings
#[derive(Debug, Clone, PartialEq)]
pub struct CtfBaseHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates CTF transit and flag-to-flag distance metrics
pub fn evaluate_ctf_base_metrics(
    config: &CtfBaseConfig,
    peers: &[LevelFlagBaseRecord],
) -> CtfBaseMetrics {
    let mut min_opp_dist = 9999.0_f32;
    for p in peers {
        if p.team != config.team {
            let d = p.position.distance(config.position);
            if d < min_opp_dist {
                min_opp_dist = d;
            }
        }
    }

    let runner_time = if min_opp_dist < 9000.0 {
        min_opp_dist / 400.0 // Standard running speed = 400 uu/s
    } else {
        0.0
    };

    let translocator_time = if min_opp_dist < 9000.0 {
        min_opp_dist / 800.0 // Effective translocator speed = 800 uu/s
    } else {
        0.0
    };

    let defense_area = std::f32::consts::PI * config.defense_radius_uu.powi(2);

    CtfBaseMetrics {
        flag_to_flag_distance_uu: min_opp_dist,
        runner_transit_time_secs: runner_time,
        translocator_transit_time_secs: translocator_time,
        total_flag_bases_in_level: peers.len() + 1,
        defense_coverage_area_sq_uu: defense_area,
    }
}

/// Audits CTF flag base setup for missing opposing bases, instant-cap proximity, and subsecond return
pub fn audit_ctf_base_safety(
    config: &CtfBaseConfig,
    peers: &[LevelFlagBaseRecord],
) -> Vec<CtfBaseHazard> {
    let mut hazards = Vec::new();

    // 1. Missing Opposing Flag Base Warning
    let has_opposing = peers.iter().any(|p| p.team != config.team);
    if !has_opposing {
        hazards.push(CtfBaseHazard {
            title: "Missing Opposing Team Flag Base".to_string(),
            description: "No opposing team FlagBase was found in the level. CTF matches require both Red and Blue bases to function.".to_string(),
            is_critical: false,
        });
    }

    // 2. Instant-Cap Proximity Hazard (<800 uu)
    for p in peers {
        if p.team != config.team {
            let dist = p.position.distance(config.position);
            if dist < 800.0 {
                hazards.push(CtfBaseHazard {
                    title: "Critical Instant-Cap Flag Proximity".to_string(),
                    description: format!(
                        "Flag base is only {:.0} uu from opposing base '{}'. Runners can capture flags in under 2 seconds!",
                        dist, p.name
                    ),
                    is_critical: true,
                });
            }
        }
    }

    // 3. Duplicate Team Flag Base Hazard
    for p in peers {
        if p.team == config.team {
            hazards.push(CtfBaseHazard {
                title: "Duplicate Same-Team Flag Base".to_string(),
                description: format!(
                    "Level already contains '{}' for {:?}. Multiple bases for the same team will cause bot AI confusion.",
                    p.name, config.team
                ),
                is_critical: false,
            });
        }
    }

    // 4. Excessively Short Flag Return Timeout (<15s)
    if config.flag_return_timeout_secs < 15.0 {
        hazards.push(CtfBaseHazard {
            title: "Overly Aggressive Flag Auto-Return".to_string(),
            description: format!(
                "Return timeout of {:.1}s is too short. Defenders will not have time to recover dropped flags.",
                config.flag_return_timeout_secs
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural octagonal stepped flag base pedestal brush
pub fn generate_ctf_flag_base_brush(config: &CtfBaseConfig) -> Brush {
    let mut brush = Brush::new("CTFFlagBasePedestal".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let sides = 8;
    let base_r = config.pedestal_radius;
    let step_h = config.pedestal_height * 0.5;

    // Lower tier: Octagonal prism from 0 to step_h
    let mut bot_tier_lower = Vec::new();
    let mut bot_tier_upper = Vec::new();
    for i in 0..sides {
        let angle = std::f32::consts::TAU * (i as f32) / (sides as f32);
        let x = base_r * angle.cos();
        let y = base_r * angle.sin();
        bot_tier_lower.push(Vec3::new(x, y, 0.0));
        bot_tier_upper.push(Vec3::new(x, y, step_h));
    }

    // Upper tier: Octagonal prism from step_h to pedestal_height (smaller radius)
    let top_r = base_r * 0.8;
    let mut top_tier_upper = Vec::new();
    for i in 0..sides {
        let angle = std::f32::consts::TAU * (i as f32) / (sides as f32);
        let x = top_r * angle.cos();
        let y = top_r * angle.sin();
        top_tier_upper.push(Vec3::new(x, y, config.pedestal_height));
    }

    // 1. Lower tier bottom cap
    if let Some(poly) = Polygon::new(bot_tier_lower.clone()) {
        brush.polys.push(poly);
    }

    // 2. Lower tier sides
    for i in 0..sides {
        let next = (i + 1) % sides;
        if let Some(poly) = Polygon::new(vec![
            bot_tier_lower[i],
            bot_tier_lower[next],
            bot_tier_upper[next],
            bot_tier_upper[i],
        ]) {
            brush.polys.push(poly);
        }
    }

    // 3. Upper tier sides
    for i in 0..sides {
        let next = (i + 1) % sides;
        if let Some(poly) = Polygon::new(vec![
            bot_tier_upper[i],
            bot_tier_upper[next],
            top_tier_upper[next],
            top_tier_upper[i],
        ]) {
            brush.polys.push(poly);
        }
    }

    // 4. Top cap
    let mut top_rev = top_tier_upper.clone();
    top_rev.reverse();
    if let Some(poly) = Polygon::new(top_rev) {
        brush.polys.push(poly);
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctf_archetypes_and_defaults() {
        let cfg = CtfBaseArchetype::FaceMonolithBase.to_config();
        assert_eq!(cfg.team, CTFTeam::Red);
        assert_eq!(cfg.defense_radius_uu, 1200.0);
        assert_eq!(cfg.flag_return_timeout_secs, 25.0);
        assert_eq!(cfg.pedestal_radius, 64.0);
    }

    #[test]
    fn test_ctf_base_metrics_evaluation() {
        let cfg = CtfBaseConfig::default();
        let peers = vec![LevelFlagBaseRecord {
            name: "BlueBase".to_string(),
            team: CTFTeam::Blue,
            position: Vec3::new(3200.0, 0.0, 0.0),
        }];

        let metrics = evaluate_ctf_base_metrics(&cfg, &peers);
        assert_eq!(metrics.total_flag_bases_in_level, 2);
        assert_eq!(metrics.flag_to_flag_distance_uu, 3200.0);
        assert_eq!(metrics.runner_transit_time_secs, 8.0); // 3200 / 400 = 8.0s
        assert_eq!(metrics.translocator_transit_time_secs, 4.0); // 3200 / 800 = 4.0s
    }

    #[test]
    fn test_ctf_base_hazard_auditing() {
        let cfg = CtfBaseConfig::default(); // Red, pos=(0,0,0)
        let peers = vec![LevelFlagBaseRecord {
            name: "BlueBase".to_string(),
            team: CTFTeam::Blue,
            position: Vec3::new(500.0, 0.0, 0.0), // < 800 uu!
        }];

        let hazards = audit_ctf_base_safety(&cfg, &peers);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Instant-Cap")));
    }

    #[test]
    fn test_generate_ctf_flag_base_brush() {
        let cfg = CtfBaseConfig::default();
        let brush = generate_ctf_flag_base_brush(&cfg);

        assert_eq!(brush.name, "CTFFlagBasePedestal");
        // 1 bottom cap + 8 lower sides + 8 upper sides + 1 top cap = 18 polys
        assert_eq!(brush.polys.len(), 18);
    }
}
