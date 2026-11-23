// crates/ut99-bsp/src/spawn_distribution.rs
//
// Interactive Tournament Spawn Point Balancing & Telefrag Prevention Studio
// Team distribution, collision hull clearances, telefrag prevention, spawn camping exposure
// analytics, unarmed sprint time calculations, and procedural directional spawn pad generation.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical UT99 Team Identifiers for PlayerStarts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentTeamId {
    Red = 0,
    Blue = 1,
    Green = 2,
    Gold = 3,
    FFA = 255,
}

impl TournamentTeamId {
    pub fn display_name(&self) -> &'static str {
        match self {
            TournamentTeamId::Red => "Team Red (ID 0)",
            TournamentTeamId::Blue => "Team Blue (ID 1)",
            TournamentTeamId::Green => "Team Green (ID 2)",
            TournamentTeamId::Gold => "Team Gold (ID 3)",
            TournamentTeamId::FFA => "Free-For-All / Deathmatch (ID 255)",
        }
    }

    pub fn team_color_rgb(&self) -> [f32; 4] {
        match self {
            TournamentTeamId::Red => [0.95, 0.20, 0.20, 0.95],
            TournamentTeamId::Blue => [0.20, 0.50, 1.00, 0.95],
            TournamentTeamId::Green => [0.20, 0.90, 0.30, 0.95],
            TournamentTeamId::Gold => [1.00, 0.85, 0.15, 0.95],
            TournamentTeamId::FFA => [0.90, 0.90, 0.90, 0.95],
        }
    }
}

/// Archetypes of strategic tournament spawn setups
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpawnDistributionArchetype {
    Deck16DuelSpawnRing,
    FaceSymmetricCTFSpawns,
    CampgroundsTDMGrid,
    PressureTrapPerimeter,
    NoviceLinearSpawnHub,
}

impl SpawnDistributionArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            SpawnDistributionArchetype::Deck16DuelSpawnRing => "Deck16][ Duel Spawn Ring (4 Balanced Spawns)",
            SpawnDistributionArchetype::FaceSymmetricCTFSpawns => "Facing Worlds Symmetric CTF Spawns (Base Defense)",
            SpawnDistributionArchetype::CampgroundsTDMGrid => "Campgrounds TDM Dispersed Grid (8 Spawns)",
            SpawnDistributionArchetype::PressureTrapPerimeter => "Pressure Hazard Perimeter (Risk/Reward Spawns)",
            SpawnDistributionArchetype::NoviceLinearSpawnHub => "Novice Linear Training Spawns (Direct Path)",
        }
    }

    pub fn to_config(&self) -> SpawnDistributionConfig {
        match self {
            SpawnDistributionArchetype::Deck16DuelSpawnRing => SpawnDistributionConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_degrees: 0.0,
                team_number: TournamentTeamId::FFA,
                b_single_player_start: true,
                b_coop_start: false,
                b_enabled: true,
                pad_radius: 36.0,
                pad_height: 6.0,
                tag: "Deck16DuelSpawn".to_string(),
            },
            SpawnDistributionArchetype::FaceSymmetricCTFSpawns => SpawnDistributionConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_degrees: 90.0,
                team_number: TournamentTeamId::Red,
                b_single_player_start: false,
                b_coop_start: false,
                b_enabled: true,
                pad_radius: 40.0,
                pad_height: 8.0,
                tag: "FaceRedBaseSpawn".to_string(),
            },
            SpawnDistributionArchetype::CampgroundsTDMGrid => SpawnDistributionConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_degrees: 180.0,
                team_number: TournamentTeamId::Blue,
                b_single_player_start: false,
                b_coop_start: false,
                b_enabled: true,
                pad_radius: 38.0,
                pad_height: 6.0,
                tag: "CampgroundsTDMSpawn".to_string(),
            },
            SpawnDistributionArchetype::PressureTrapPerimeter => SpawnDistributionConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_degrees: 45.0,
                team_number: TournamentTeamId::FFA,
                b_single_player_start: true,
                b_coop_start: false,
                b_enabled: true,
                pad_radius: 34.0,
                pad_height: 6.0,
                tag: "PressurePerimeterSpawn".to_string(),
            },
            SpawnDistributionArchetype::NoviceLinearSpawnHub => SpawnDistributionConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_degrees: 0.0,
                team_number: TournamentTeamId::FFA,
                b_single_player_start: true,
                b_coop_start: true,
                b_enabled: true,
                pad_radius: 36.0,
                pad_height: 6.0,
                tag: "NoviceLinearSpawn".to_string(),
            },
        }
    }
}

/// Configuration settings for Spawn Point Balancing & Telefrag Prevention Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnDistributionConfig {
    pub archetype: SpawnDistributionArchetype,
    pub position: Vec3,
    pub yaw_degrees: f32,
    pub team_number: TournamentTeamId,
    pub b_single_player_start: bool,
    pub b_coop_start: bool,
    pub b_enabled: bool,
    pub pad_radius: f32,
    pub pad_height: f32,
    pub tag: String,
}

impl Default for SpawnDistributionConfig {
    fn default() -> Self {
        SpawnDistributionArchetype::Deck16DuelSpawnRing.to_config()
    }
}

/// Representation of an existing PlayerStart or Actor in the level
#[derive(Debug, Clone, PartialEq)]
pub struct LevelPlayerStartRecord {
    pub name: String,
    pub position: Vec3,
    pub team_number: u8,
    pub is_player_start: bool,
    pub is_weapon: bool,
}

/// Analytical metrics for tournament spawn balancing
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnDistributionMetrics {
    pub total_player_starts_in_level: usize,
    pub same_team_spawns: usize,
    pub opposing_team_spawns: usize,
    pub nearest_spawn_distance_uu: f32,
    pub nearest_weapon_distance_uu: f32,
    pub unarmed_sprint_time_secs: f32,
    pub spawn_camping_exposure_index: f32, // 0 to 100
}

/// Safety and balance hazards for tournament spawn points
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates telemetry and spawn equity across the level
pub fn evaluate_spawn_metrics(
    config: &SpawnDistributionConfig,
    spawns: &[LevelPlayerStartRecord],
) -> SpawnDistributionMetrics {
    let mut total_starts = 0;
    let mut same_team = 0;
    let mut opposing_team = 0;
    let mut min_dist_spawn = f32::MAX;
    let mut min_dist_weapon = f32::MAX;

    for s in spawns {
        if s.is_player_start {
            total_starts += 1;
            let dist = s.position.distance(config.position);
            if dist < min_dist_spawn {
                min_dist_spawn = dist;
            }
            if s.team_number == config.team_number as u8 {
                same_team += 1;
            } else {
                opposing_team += 1;
            }
        }
        if s.is_weapon {
            let dist = s.position.distance(config.position);
            if dist < min_dist_weapon {
                min_dist_weapon = dist;
            }
        }
    }

    let nearest_weap = if min_dist_weapon < f32::MAX { min_dist_weapon } else { 800.0 };
    let unarmed_sprint = nearest_weap / 400.0; // standard UT99 sprint speed: 400 uu/s

    // Spawn Camping Exposure Index: Higher if far from weapons or isolated
    let mut camping_index = 20.0_f32;
    if nearest_weap > 600.0 {
        camping_index += 30.0;
    }
    if min_dist_spawn < 250.0 && min_dist_spawn > 0.0 {
        camping_index += 25.0;
    }
    camping_index = camping_index.clamp(5.0, 95.0);

    SpawnDistributionMetrics {
        total_player_starts_in_level: total_starts,
        same_team_spawns: same_team,
        opposing_team_spawns: opposing_team,
        nearest_spawn_distance_uu: if min_dist_spawn < f32::MAX { min_dist_spawn } else { 9999.0 },
        nearest_weapon_distance_uu: nearest_weap,
        unarmed_sprint_time_secs: unarmed_sprint,
        spawn_camping_exposure_index: camping_index,
    }
}

/// Audits telefrag hazards, weapon starvation and team clumping
pub fn audit_spawn_safety(
    config: &SpawnDistributionConfig,
    spawns: &[LevelPlayerStartRecord],
) -> Vec<SpawnHazard> {
    let mut hazards = Vec::new();

    // 1. Critical Telefrag Risk: Another spawn point < 120 uu
    for s in spawns {
        if s.is_player_start {
            let dist = s.position.distance(config.position);
            if dist < 120.0 && dist > 1.0 {
                hazards.push(SpawnHazard {
                    title: "Critical Telefrag Hazard".to_string(),
                    description: format!(
                        "Spawn point is only {:.1} uu from '{}'. Players spawning simultaneously will instantly telefrag each other (min safe clearance: 120 uu).",
                        dist, s.name
                    ),
                    is_critical: true,
                });
                break;
            }
        }
    }

    // 2. Unarmed Starvation Risk: Nearest weapon > 800 uu (> 2.0s unarmed sprint)
    let mut min_weap_dist = f32::MAX;
    for s in spawns {
        if s.is_weapon {
            let dist = s.position.distance(config.position);
            if dist < min_weap_dist {
                min_weap_dist = dist;
            }
        }
    }
    if min_weap_dist > 800.0 && min_weap_dist < f32::MAX {
        hazards.push(SpawnHazard {
            title: "Unarmed Spawn Starvation Risk".to_string(),
            description: format!(
                "Nearest weapon is {:.0} uu away ({:.1}s unarmed run). Freshly spawned players are vulnerable to spawn camping.",
                min_weap_dist, min_weap_dist / 400.0
            ),
            is_critical: false,
        });
    }

    // 3. Team Clumping / Asymmetry Warning for CTF / TDM
    if config.team_number != TournamentTeamId::FFA {
        let mut same_team_count = 0;
        for s in spawns {
            if s.is_player_start && s.team_number == config.team_number as u8 {
                same_team_count += 1;
            }
        }
        if same_team_count > 6 {
            hazards.push(SpawnHazard {
                title: "Excessive Team Spawn Density".to_string(),
                description: format!(
                    "{} spawns already assigned to {}. Distribute spawn points into multiple defense corridors.",
                    same_team_count, config.team_number.display_name()
                ),
                is_critical: false,
            });
        }
    }

    hazards
}

/// Generates an octagonal team-tinted directional spawn pad brush
pub fn generate_spawn_pad_brush(config: &SpawnDistributionConfig) -> Brush {
    let mut brush = Brush::new("SpawnPad".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let r = config.pad_radius;
    let h = config.pad_height;
    let num_sides = 8;

    let yaw_rad = config.yaw_degrees.to_radians();

    let mut top_ring = Vec::new();
    let mut bot_ring = Vec::new();

    for i in 0..num_sides {
        let angle = yaw_rad + std::f32::consts::TAU * (i as f32) / (num_sides as f32);
        let x = r * angle.cos();
        let y = r * angle.sin();
        top_ring.push(Vec3::new(x, y, h));
        bot_ring.push(Vec3::new(x, y, 0.0));
    }

    // Top polygon
    if let Some(poly) = Polygon::new(top_ring.clone()) {
        brush.polys.push(poly);
    }

    // Bottom polygon (reversed winding)
    let mut bot_rev = bot_ring.clone();
    bot_rev.reverse();
    if let Some(poly) = Polygon::new(bot_rev) {
        brush.polys.push(poly);
    }

    // Side quad polygons
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
    fn test_spawn_archetypes_and_defaults() {
        let cfg = SpawnDistributionArchetype::Deck16DuelSpawnRing.to_config();
        assert_eq!(cfg.team_number, TournamentTeamId::FFA);
        assert_eq!(cfg.yaw_degrees, 0.0);
        assert!(cfg.b_single_player_start);
        assert!(cfg.b_enabled);
    }

    #[test]
    fn test_spawn_metrics_evaluation() {
        let cfg = SpawnDistributionConfig::default();
        let spawns = vec![
            LevelPlayerStartRecord {
                name: "Start0".to_string(),
                position: Vec3::new(300.0, 0.0, 0.0),
                team_number: 255,
                is_player_start: true,
                is_weapon: false,
            },
            LevelPlayerStartRecord {
                name: "Weapon0".to_string(),
                position: Vec3::new(400.0, 0.0, 0.0),
                team_number: 0,
                is_player_start: false,
                is_weapon: true,
            },
        ];

        let metrics = evaluate_spawn_metrics(&cfg, &spawns);
        assert_eq!(metrics.total_player_starts_in_level, 1);
        assert_eq!(metrics.nearest_spawn_distance_uu, 300.0);
        assert_eq!(metrics.nearest_weapon_distance_uu, 400.0);
        assert_eq!(metrics.unarmed_sprint_time_secs, 1.0);
    }

    #[test]
    fn test_spawn_safety_audit_telefrag_detection() {
        let cfg = SpawnDistributionConfig::default();
        let spawns = vec![LevelPlayerStartRecord {
            name: "AdjacentStart".to_string(),
            position: Vec3::new(80.0, 0.0, 0.0), // < 120 uu! Telefrag risk
            team_number: 255,
            is_player_start: true,
            is_weapon: false,
        }];

        let hazards = audit_spawn_safety(&cfg, &spawns);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Telefrag")));
    }

    #[test]
    fn test_generate_spawn_pad_brush() {
        let cfg = SpawnDistributionConfig::default();
        let brush = generate_spawn_pad_brush(&cfg);

        assert_eq!(brush.name, "SpawnPad");
        assert_eq!(brush.polys.len(), 10); // 8 sides + 2 caps
    }
}
