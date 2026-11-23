// crates/ut99-bsp/src/team_cannon_studio.rs
//
// Interactive Team Cannon & Stationary Automated Turret Defense Studio
// Automated base cannons, tracking speeds, firing rates, damage per shot, TTK analysis,
// spawn camping hazard auditing, and procedural reinforced bunker mount brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Weapon type fired by the automated stationary cannon
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CannonWeaponType {
    MinigunGatling,
    PulseLaserTurret,
    RocketFlakPod,
    SniperRailBattery,
}

impl CannonWeaponType {
    pub fn display_name(&self) -> &'static str {
        match self {
            CannonWeaponType::MinigunGatling => "Heavy Minigun Gatling (High RPM Hitscan)",
            CannonWeaponType::PulseLaserTurret => "Pulse Laser Beam (Sustained Plasma)",
            CannonWeaponType::RocketFlakPod => "Rocket Flak Pod (Explosive Area Denial)",
            CannonWeaponType::SniperRailBattery => "Sniper Rail Battery (Long-Range Anti-Materiel)",
        }
    }

    pub fn default_rpm(&self) -> f32 {
        match self {
            CannonWeaponType::MinigunGatling => 600.0,
            CannonWeaponType::PulseLaserTurret => 400.0,
            CannonWeaponType::RocketFlakPod => 90.0,
            CannonWeaponType::SniperRailBattery => 45.0,
        }
    }

    pub fn default_damage(&self) -> f32 {
        match self {
            CannonWeaponType::MinigunGatling => 9.0,
            CannonWeaponType::PulseLaserTurret => 18.0,
            CannonWeaponType::RocketFlakPod => 75.0,
            CannonWeaponType::SniperRailBattery => 110.0,
        }
    }
}

/// Archetypes of canonical base defense cannons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamCannonArchetype {
    OverlordBeachBunkerCannon,
    HiSpeedRoofMinigunCannon,
    FaceSniperTowerGatling,
    FrigateDeckMountedFlak,
    SubterraneanCeilingTurret,
}

impl TeamCannonArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            TeamCannonArchetype::OverlordBeachBunkerCannon => "Overlord Beach Bunker Heavy Cannon",
            TeamCannonArchetype::HiSpeedRoofMinigunCannon => "HiSpeed Roof Tracking Gatling",
            TeamCannonArchetype::FaceSniperTowerGatling => "Facing Worlds Tower Perimeter Sentinel",
            TeamCannonArchetype::FrigateDeckMountedFlak => "Frigate Naval Deck Flak Pod",
            TeamCannonArchetype::SubterraneanCeilingTurret => "Subterranean Inverted Ceiling Turret",
        }
    }

    pub fn to_config(&self) -> TeamCannonConfig {
        match self {
            TeamCannonArchetype::OverlordBeachBunkerCannon => TeamCannonConfig {
                archetype: *self,
                cannon_type: CannonWeaponType::MinigunGatling,
                team: 0, // Red Defenders
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_degrees: 0.0,
                pitch_degrees: -10.0,
                max_range_uu: 3200.0,
                tracking_speed_deg_per_sec: 90.0,
                fire_rate_rpm: 600.0,
                damage_per_shot: 9.0,
                bunker_mount_radius: 64.0,
                bunker_mount_height: 32.0,
                tag: "BeachCannon_Red".to_string(),
                is_inverted: false,
            },
            TeamCannonArchetype::HiSpeedRoofMinigunCannon => TeamCannonConfig {
                archetype: *self,
                cannon_type: CannonWeaponType::MinigunGatling,
                team: 0,
                position: Vec3::new(0.0, 0.0, 16.0),
                yaw_degrees: 180.0,
                pitch_degrees: 0.0,
                max_range_uu: 2400.0,
                tracking_speed_deg_per_sec: 120.0,
                fire_rate_rpm: 720.0,
                damage_per_shot: 8.0,
                bunker_mount_radius: 48.0,
                bunker_mount_height: 24.0,
                tag: "TrainRoofTurret".to_string(),
                is_inverted: false,
            },
            TeamCannonArchetype::FaceSniperTowerGatling => TeamCannonConfig {
                archetype: *self,
                cannon_type: CannonWeaponType::SniperRailBattery,
                team: 1, // Blue
                position: Vec3::new(0.0, 0.0, 128.0),
                yaw_degrees: 90.0,
                pitch_degrees: -15.0,
                max_range_uu: 4500.0,
                tracking_speed_deg_per_sec: 60.0,
                fire_rate_rpm: 45.0,
                damage_per_shot: 110.0,
                bunker_mount_radius: 72.0,
                bunker_mount_height: 36.0,
                tag: "FaceTowerCannon".to_string(),
                is_inverted: false,
            },
            TeamCannonArchetype::FrigateDeckMountedFlak => TeamCannonConfig {
                archetype: *self,
                cannon_type: CannonWeaponType::RocketFlakPod,
                team: 0,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_degrees: 270.0,
                pitch_degrees: -5.0,
                max_range_uu: 2600.0,
                tracking_speed_deg_per_sec: 80.0,
                fire_rate_rpm: 90.0,
                damage_per_shot: 75.0,
                bunker_mount_radius: 80.0,
                bunker_mount_height: 40.0,
                tag: "DeckFlakCannon".to_string(),
                is_inverted: false,
            },
            TeamCannonArchetype::SubterraneanCeilingTurret => TeamCannonConfig {
                archetype: *self,
                cannon_type: CannonWeaponType::PulseLaserTurret,
                team: 1,
                position: Vec3::new(0.0, 0.0, 256.0),
                yaw_degrees: 0.0,
                pitch_degrees: -45.0,
                max_range_uu: 2000.0,
                tracking_speed_deg_per_sec: 140.0,
                fire_rate_rpm: 400.0,
                damage_per_shot: 18.0,
                bunker_mount_radius: 56.0,
                bunker_mount_height: 28.0,
                tag: "CeilingLaserTurret".to_string(),
                is_inverted: true,
            },
        }
    }
}

/// Configuration settings for automated Team Cannon
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamCannonConfig {
    pub archetype: TeamCannonArchetype,
    pub cannon_type: CannonWeaponType,
    pub team: u8,
    pub position: Vec3,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub max_range_uu: f32,
    pub tracking_speed_deg_per_sec: f32,
    pub fire_rate_rpm: f32,
    pub damage_per_shot: f32,
    pub bunker_mount_radius: f32,
    pub bunker_mount_height: f32,
    pub tag: String,
    pub is_inverted: bool,
}

impl Default for TeamCannonConfig {
    fn default() -> Self {
        TeamCannonArchetype::OverlordBeachBunkerCannon.to_config()
    }
}

/// Representation of an existing cannon or spawn in the level
#[derive(Debug, Clone, PartialEq)]
pub struct LevelCombatActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
    pub team: u8,
}

/// Telemetry metrics for Team Cannon
#[derive(Debug, Clone, PartialEq)]
pub struct TeamCannonMetrics {
    pub dps: f32,
    pub time_to_kill_standard_player_secs: f32,
    pub target_acquisition_latency_secs: f32,
    pub effective_defense_volume_cu_uu: f32,
    pub total_cannons_in_level: usize,
}

/// Safety diagnostics and balance alerts
#[derive(Debug, Clone, PartialEq)]
pub struct TeamCannonHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates lethal DPS, TTK, and defense sphere volume
pub fn evaluate_team_cannon_metrics(
    config: &TeamCannonConfig,
    peers: &[LevelCombatActorRecord],
) -> TeamCannonMetrics {
    let dps = (config.fire_rate_rpm / 60.0) * config.damage_per_shot;
    let ttk = if dps > 0.0 { 100.0 / dps } else { 99.0 };

    let acq_latency = if config.tracking_speed_deg_per_sec > 0.0 {
        180.0 / config.tracking_speed_deg_per_sec
    } else {
        99.0
    };

    // Hemispherical volume: (2/3) * pi * r^3
    let volume = (2.0 / 3.0) * std::f32::consts::PI * config.max_range_uu.powi(3);

    let mut cannons = 1;
    for p in peers {
        if p.class_name.to_lowercase().contains("cannon") || p.class_name.to_lowercase().contains("turret") {
            cannons += 1;
        }
    }

    TeamCannonMetrics {
        dps,
        time_to_kill_standard_player_secs: ttk,
        target_acquisition_latency_secs: acq_latency,
        effective_defense_volume_cu_uu: volume,
        total_cannons_in_level: cannons,
    }
}

/// Audits safety for spawn camping lockouts, instant lethality, and sluggish tracking
pub fn audit_team_cannon_safety(
    config: &TeamCannonConfig,
    peers: &[LevelCombatActorRecord],
) -> Vec<TeamCannonHazard> {
    let mut hazards = Vec::new();

    // 1. Spawn Camping Hazard: Cannon range covers opposing player starts
    for p in peers {
        let lower = p.class_name.to_lowercase();
        if lower.contains("playerstart") && p.team != config.team {
            let dist = p.position.distance(config.position);
            if dist < config.max_range_uu * 0.4 {
                hazards.push(TeamCannonHazard {
                    title: "Critical Spawn Camp Lockout".to_string(),
                    description: format!(
                        "Cannon at {:.0} uu directly targets opposing PlayerStart '{}'. Freshly spawned players will be immediately obliterated!",
                        dist, p.name
                    ),
                    is_critical: true,
                });
            }
        }
    }

    // 2. Unfair Instagib TTK (<0.6s)
    let dps = (config.fire_rate_rpm / 60.0) * config.damage_per_shot;
    if dps > 180.0 {
        let ttk = 100.0 / dps;
        hazards.push(TeamCannonHazard {
            title: "Excessive Lethality / Zero Reaction Time".to_string(),
            description: format!(
                "Cannon deals {:.1} DPS (TTK: {:.2}s). Attackers have virtually zero reaction time to reach cover.",
                dps, ttk
            ),
            is_critical: false,
        });
    }

    // 3. Sluggish Tracking Blind Spot (<45 deg/s)
    if config.tracking_speed_deg_per_sec < 45.0 {
        hazards.push(TeamCannonHazard {
            title: "Sluggish Tracking Speed".to_string(),
            description: format!(
                "Tracking speed of {:.0}°/s is too slow. Strafe-jumping players can easily circle-strafe the turret unharmed.",
                config.tracking_speed_deg_per_sec
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural octagonal reinforced bunker mount brush
pub fn generate_team_cannon_bunker_brush(config: &TeamCannonConfig) -> Brush {
    let mut brush = Brush::new("TeamCannonBunkerMount".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let sides = 8;
    let base_r = config.bunker_mount_radius;
    let h = config.bunker_mount_height;
    let z_mult = if config.is_inverted { -1.0 } else { 1.0 };

    let mut lower_verts = Vec::new();
    let mut upper_verts = Vec::new();

    for i in 0..sides {
        let angle = std::f32::consts::TAU * (i as f32) / (sides as f32);
        let x = base_r * angle.cos();
        let y = base_r * angle.sin();
        lower_verts.push(Vec3::new(x, y, 0.0));
        upper_verts.push(Vec3::new(x * 0.85, y * 0.85, h * z_mult));
    }

    // 1. Bottom Cap
    if let Some(poly) = Polygon::new(lower_verts.clone()) {
        brush.polys.push(poly);
    }

    // 2. Sides
    for i in 0..sides {
        let next = (i + 1) % sides;
        if let Some(poly) = Polygon::new(vec![
            lower_verts[i],
            lower_verts[next],
            upper_verts[next],
            upper_verts[i],
        ]) {
            brush.polys.push(poly);
        }
    }

    // 3. Top Cap
    let mut top_rev = upper_verts.clone();
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
    fn test_cannon_archetypes_and_defaults() {
        let cfg = TeamCannonArchetype::OverlordBeachBunkerCannon.to_config();
        assert_eq!(cfg.team, 0);
        assert_eq!(cfg.cannon_type, CannonWeaponType::MinigunGatling);
        assert_eq!(cfg.fire_rate_rpm, 600.0);
        assert_eq!(cfg.damage_per_shot, 9.0);
        assert!(!cfg.is_inverted);
    }

    #[test]
    fn test_cannon_metrics_evaluation() {
        let cfg = TeamCannonConfig::default();
        let peers = vec![LevelCombatActorRecord {
            name: "OtherCannon".to_string(),
            class_name: "Botpack.TeamCannon".to_string(),
            position: Vec3::new(1200.0, 0.0, 0.0),
            team: 0,
        }];

        let metrics = evaluate_team_cannon_metrics(&cfg, &peers);
        assert_eq!(metrics.total_cannons_in_level, 2);
        assert_eq!(metrics.dps, 90.0); // (600 / 60) * 9 = 90
        assert_eq!(metrics.time_to_kill_standard_player_secs, 100.0 / 90.0);
        assert_eq!(metrics.target_acquisition_latency_secs, 180.0 / 90.0); // 2.0s
    }

    #[test]
    fn test_cannon_hazard_auditing() {
        let cfg = TeamCannonConfig::default(); // max_range = 3200 uu
        let peers = vec![LevelCombatActorRecord {
            name: "PlayerStart_Blue".to_string(),
            class_name: "Engine.PlayerStart".to_string(),
            position: Vec3::new(800.0, 0.0, 0.0), // 800 < 3200 * 0.4 (1280 uu)!
            team: 1,
        }];

        let hazards = audit_team_cannon_safety(&cfg, &peers);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Spawn Camp")));
    }

    #[test]
    fn test_generate_team_cannon_bunker_brush() {
        let cfg = TeamCannonConfig::default();
        let brush = generate_team_cannon_bunker_brush(&cfg);

        assert_eq!(brush.name, "TeamCannonBunkerMount");
        // 1 bottom cap + 8 sides + 1 top cap = 10 polys
        assert_eq!(brush.polys.len(), 10);
    }
}
