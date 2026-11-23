// crates/ut99-bsp/src/zone_velocity_studio.rs
//
// Interactive Dynamic Zone Velocity, Atmospheric Convection & Low-G Space Vacuum Studio
// Zone velocity fields, fluid currents, upward wind updrafts, zero-G vacuum drift,
// buoyancy balance, pain zone damage over time, and procedural zone portal sheet brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Fluid medium and environmental zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneMediumType {
    AtmosphericAir,
    SubmergedWater,
    CorrosiveAcidSlime,
    MoltenLava,
    DeepSpaceVacuum,
}

impl ZoneMediumType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ZoneMediumType::AtmosphericAir => "Atmospheric Air Convection",
            ZoneMediumType::SubmergedWater => "Submerged Water Current",
            ZoneMediumType::CorrosiveAcidSlime => "Corrosive Acid Slime Flume",
            ZoneMediumType::MoltenLava => "Molten Lava Stream",
            ZoneMediumType::DeepSpaceVacuum => "Deep Space Vacuum Zero-G",
        }
    }

    pub fn default_friction(&self) -> f32 {
        match self {
            ZoneMediumType::AtmosphericAir => 1.0,
            ZoneMediumType::SubmergedWater => 2.5,
            ZoneMediumType::CorrosiveAcidSlime => 3.5,
            ZoneMediumType::MoltenLava => 5.0,
            ZoneMediumType::DeepSpaceVacuum => 0.1,
        }
    }
}

/// Archetypes of canonical UT99 zone velocity dynamics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneVelocityArchetype {
    HydroelectricRiverCurrent,
    IndustrialUpdraftWindTunnel,
    OuterSpaceVacuumDrift,
    ToxicSlimeGutterFlume,
    HighGravCentrifugeVortex,
}

impl ZoneVelocityArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            ZoneVelocityArchetype::HydroelectricRiverCurrent => "Hydroelectric River Current (Hydro16 / OceanFloor)",
            ZoneVelocityArchetype::IndustrialUpdraftWindTunnel => "Industrial Updraft Wind Tunnel (Shrapnel Shaft)",
            ZoneVelocityArchetype::OuterSpaceVacuumDrift => "Outer Space Vacuum Drift (Phobos Zero-G Air)",
            ZoneVelocityArchetype::ToxicSlimeGutterFlume => "Toxic Slime Gutter Flume (Deck16 Acid Conduit)",
            ZoneVelocityArchetype::HighGravCentrifugeVortex => "High-Grav Centrifuge Vortex (Pressure Core)",
        }
    }

    pub fn to_config(&self) -> ZoneVelocityConfig {
        match self {
            ZoneVelocityArchetype::HydroelectricRiverCurrent => ZoneVelocityConfig {
                archetype: *self,
                medium: ZoneMediumType::SubmergedWater,
                position: Vec3::new(0.0, 0.0, 0.0),
                zone_velocity: Vec3::new(400.0, 0.0, 0.0),
                zone_length_uu: 1200.0,
                fluid_friction: 2.5,
                terminal_velocity: 1200.0,
                is_zero_gravity: false,
                is_pain_zone: false,
                damage_per_sec: 0.0,
                damage_type: "Drowned".to_string(),
                portal_width_uu: 256.0,
                portal_height_uu: 192.0,
                zone_tag: "RiverZone0".to_string(),
            },
            ZoneVelocityArchetype::IndustrialUpdraftWindTunnel => ZoneVelocityConfig {
                archetype: *self,
                medium: ZoneMediumType::AtmosphericAir,
                position: Vec3::new(0.0, 0.0, 0.0),
                zone_velocity: Vec3::new(0.0, 0.0, 850.0),
                zone_length_uu: 800.0,
                fluid_friction: 0.8,
                terminal_velocity: 2500.0,
                is_zero_gravity: false,
                is_pain_zone: false,
                damage_per_sec: 0.0,
                damage_type: "None".to_string(),
                portal_width_uu: 192.0,
                portal_height_uu: 192.0,
                zone_tag: "UpdraftTunnel".to_string(),
            },
            ZoneVelocityArchetype::OuterSpaceVacuumDrift => ZoneVelocityConfig {
                archetype: *self,
                medium: ZoneMediumType::DeepSpaceVacuum,
                position: Vec3::new(0.0, 0.0, 0.0),
                zone_velocity: Vec3::new(60.0, 60.0, 0.0),
                zone_length_uu: 1500.0,
                fluid_friction: 0.1,
                terminal_velocity: 3000.0,
                is_zero_gravity: true,
                is_pain_zone: false,
                damage_per_sec: 0.0,
                damage_type: "Asphyxiated".to_string(),
                portal_width_uu: 384.0,
                portal_height_uu: 256.0,
                zone_tag: "VacuumChamber".to_string(),
            },
            ZoneVelocityArchetype::ToxicSlimeGutterFlume => ZoneVelocityConfig {
                archetype: *self,
                medium: ZoneMediumType::CorrosiveAcidSlime,
                position: Vec3::new(0.0, 0.0, 0.0),
                zone_velocity: Vec3::new(0.0, -300.0, 0.0),
                zone_length_uu: 900.0,
                fluid_friction: 3.5,
                terminal_velocity: 1000.0,
                is_zero_gravity: false,
                is_pain_zone: true,
                damage_per_sec: 20.0,
                damage_type: "Corroded".to_string(),
                portal_width_uu: 200.0,
                portal_height_uu: 160.0,
                zone_tag: "SlimeFlume".to_string(),
            },
            ZoneVelocityArchetype::HighGravCentrifugeVortex => ZoneVelocityConfig {
                archetype: *self,
                medium: ZoneMediumType::AtmosphericAir,
                position: Vec3::new(0.0, 0.0, 0.0),
                zone_velocity: Vec3::new(0.0, 0.0, -700.0),
                zone_length_uu: 600.0,
                fluid_friction: 1.2,
                terminal_velocity: 3500.0,
                is_zero_gravity: false,
                is_pain_zone: false,
                damage_per_sec: 0.0,
                damage_type: "None".to_string(),
                portal_width_uu: 256.0,
                portal_height_uu: 256.0,
                zone_tag: "CentrifugeCore".to_string(),
            },
        }
    }
}

/// Configuration settings for Zone Velocity and Fluid Dynamics Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneVelocityConfig {
    pub archetype: ZoneVelocityArchetype,
    pub medium: ZoneMediumType,
    pub position: Vec3,
    pub zone_velocity: Vec3,
    pub zone_length_uu: f32,
    pub fluid_friction: f32,
    pub terminal_velocity: f32,
    pub is_zero_gravity: bool,
    pub is_pain_zone: bool,
    pub damage_per_sec: f32,
    pub damage_type: String,
    pub portal_width_uu: f32,
    pub portal_height_uu: f32,
    pub zone_tag: String,
}

impl Default for ZoneVelocityConfig {
    fn default() -> Self {
        ZoneVelocityArchetype::HydroelectricRiverCurrent.to_config()
    }
}

/// Representation of an existing ZoneInfo actor in the level
#[derive(Debug, Clone, PartialEq)]
pub struct LevelZoneContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub tag: String,
    pub position: Vec3,
}

/// Telemetry metrics for Zone Velocity
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneVelocityMetrics {
    pub effective_velocity_magnitude: f32,
    pub drift_transit_time_secs: f32,
    pub buoyancy_rating_pct: f32,
    pub terminal_speed_uu_per_sec: f32,
    pub total_velocity_zones_in_level: usize,
}

/// Safety diagnostics and fluid dynamics alerts
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneVelocityHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates velocity magnitude, drift transit time, buoyancy rating, and terminal speed
pub fn evaluate_zone_velocity_metrics(
    config: &ZoneVelocityConfig,
    peers: &[LevelZoneContextActorRecord],
) -> ZoneVelocityMetrics {
    let speed = config.zone_velocity.length();

    let transit_time = if speed > 0.0 {
        config.zone_length_uu / speed
    } else {
        99.0
    };

    // Standard UT99 gravity is 950 uu/s^2. Buoyancy is positive Z velocity relative to 950.
    let buoyancy = (config.zone_velocity.z / 950.0).clamp(-1.0, 2.0) * 100.0;

    let mut zones_count = 1;
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("zoneinfo") || cls_lower.contains("waterzone") || cls_lower.contains("lavazone") {
            zones_count += 1;
        }
    }

    ZoneVelocityMetrics {
        effective_velocity_magnitude: speed,
        drift_transit_time_secs: transit_time,
        buoyancy_rating_pct: buoyancy,
        terminal_speed_uu_per_sec: config.terminal_velocity,
        total_velocity_zones_in_level: zones_count,
    }
}

/// Audits safety for excessive velocity ejection, irreversible updrafts, and lethal pain traps
pub fn audit_zone_velocity_safety(
    config: &ZoneVelocityConfig,
    _peers: &[LevelZoneContextActorRecord],
) -> Vec<ZoneVelocityHazard> {
    let mut hazards = Vec::new();
    let speed = config.zone_velocity.length();

    // 1. Excessive Velocity (> 1800 uu/s)
    if speed > 1800.0 {
        hazards.push(ZoneVelocityHazard {
            title: "Excessive Zone Velocity".to_string(),
            description: format!(
                "Zone velocity magnitude of {:.0} uu/s is violently high. Players may clip through world BSP geometry or get stuck in walls.",
                speed
            ),
            is_critical: true,
        });
    }

    // 2. Irreversible Updraft Trap (Z > 1200 uu/s without pain or zero-G)
    if config.zone_velocity.z > 1200.0 && !config.is_zero_gravity {
        hazards.push(ZoneVelocityHazard {
            title: "Irreversible Ceiling Updraft".to_string(),
            description: format!(
                "Upward velocity of +{:.0} uu/s exceeds terminal fall speed. Players will be permanently pinned to the ceiling with no descent possible.",
                config.zone_velocity.z
            ),
            is_critical: false,
        });
    }

    // 3. Lethal Pain Trap where transit time exceeds survival window (100 HP / DPS)
    if config.is_pain_zone && config.damage_per_sec > 0.0 {
        let transit_time = if speed > 0.0 { config.zone_length_uu / speed } else { 99.0 };
        let max_survive_time = 100.0 / config.damage_per_sec;
        if transit_time > max_survive_time {
            hazards.push(ZoneVelocityHazard {
                title: "Lethal Pain Trap (Guaranteed Death)".to_string(),
                description: format!(
                    "Transit through the zone takes {:.1}s, but player health is depleted in {:.1}s ({:.0} DPS). Passage without invulnerability is impossible!",
                    transit_time, max_survive_time, config.damage_per_sec
                ),
                is_critical: true,
            });
        }
    }

    // 4. Zero Friction Ice Skidding Hazard
    if config.fluid_friction < 0.05 && speed > 100.0 {
        hazards.push(ZoneVelocityHazard {
            title: "Zero-Friction Ice Skidding".to_string(),
            description: format!(
                "Fluid friction of {:.2} provides virtually zero traction. Players will slide indefinitely with uncontrollable momentum.",
                config.fluid_friction
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural rectangular zone portal sheet brush in CSG
pub fn generate_zone_portal_sheet_brush(config: &ZoneVelocityConfig) -> Brush {
    let mut brush = Brush::new("Zone_Portal_Sheet".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let hw = config.portal_width_uu * 0.5;
    let hh = config.portal_height_uu * 0.5;

    // A 2-sided planar sheet spanning the portal opening in X-Z
    let p0 = Vec3::new(-hw, 0.0, -hh);
    let p1 = Vec3::new(hw, 0.0, -hh);
    let p2 = Vec3::new(hw, 0.0, hh);
    let p3 = Vec3::new(-hw, 0.0, hh);

    // Front Face (PF_PORTAL | PF_NOT_SOLID | PF_TWO_SIDED = 0x04000000 | 0x00000008 | 0x00000100)
    if let Some(mut poly) = Polygon::new(vec![p0, p1, p2, p3]) {
        poly.flags = 0x04000108;
        poly.texture = "Engine.BlackTexture".to_string();
        brush.polys.push(poly);
    }

    // Back Face
    if let Some(mut poly) = Polygon::new(vec![p3, p2, p1, p0]) {
        poly.flags = 0x04000108;
        poly.texture = "Engine.BlackTexture".to_string();
        brush.polys.push(poly);
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zone_velocity_archetypes_and_defaults() {
        let cfg = ZoneVelocityArchetype::HydroelectricRiverCurrent.to_config();
        assert_eq!(cfg.medium, ZoneMediumType::SubmergedWater);
        assert_eq!(cfg.zone_velocity, Vec3::new(400.0, 0.0, 0.0));
        assert_eq!(cfg.fluid_friction, 2.5);
    }

    #[test]
    fn test_zone_velocity_metrics_evaluation() {
        let cfg = ZoneVelocityConfig::default();
        let peers = vec![
            LevelZoneContextActorRecord {
                name: "ZoneInfo0".to_string(),
                class_name: "Engine.ZoneInfo".to_string(),
                tag: "RiverZone0".to_string(),
                position: Vec3::ZERO,
            }
        ];

        let metrics = evaluate_zone_velocity_metrics(&cfg, &peers);
        assert_eq!(metrics.effective_velocity_magnitude, 400.0);
        assert_eq!(metrics.drift_transit_time_secs, 1200.0 / 400.0);
        assert_eq!(metrics.total_velocity_zones_in_level, 2);
    }

    #[test]
    fn test_zone_velocity_safety_hazard_auditing() {
        let mut cfg = ZoneVelocityConfig::default();
        cfg.zone_velocity = Vec3::new(2500.0, 0.0, 0.0); // Extreme velocity
        cfg.fluid_friction = 0.01; // Ice friction

        let hazards = audit_zone_velocity_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Excessive Zone Velocity")));
        assert!(hazards.iter().any(|h| h.title.contains("Zero-Friction Ice Skidding")));
    }

    #[test]
    fn test_generate_zone_portal_sheet_brush() {
        let cfg = ZoneVelocityConfig::default();
        let brush = generate_zone_portal_sheet_brush(&cfg);

        assert_eq!(brush.name, "Zone_Portal_Sheet");
        assert_eq!(brush.polys.len(), 2);
        assert_eq!(brush.polys[0].flags & 0x04000000, 0x04000000); // PF_PORTAL flag
    }
}
