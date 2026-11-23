// crates/ut99-bsp/src/hazard_barrel_studio.rs
//
// Interactive Explosive Barrel, Toxic Drum & Hazard Pyrotechnics Studio
// Explosive chaining radius, toxic leakage fields, damage accumulation thresholds,
// impulse momentum transfer, and procedural faceted barrel brush with reinforcing rings.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Archetypes of canonical hazard drums and pyrotechnic containers in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarrelHazardType {
    VolatileNapalmFuelDrum,
    CorrosiveSludgeAcidCanister,
    ReinforcedIndustrialKeg,
    PyrotechnicGasCylinder,
    DepletedUraniumWasteCask,
}

impl BarrelHazardType {
    pub fn display_name(&self) -> &'static str {
        match self {
            BarrelHazardType::VolatileNapalmFuelDrum => "Volatile Napalm Fuel Drum (High Blast Damage)",
            BarrelHazardType::CorrosiveSludgeAcidCanister => "Corrosive Sludge Acid Canister (Persistent Pool)",
            BarrelHazardType::ReinforcedIndustrialKeg => "Reinforced Industrial Keg (Heavy Shrapnel Blast)",
            BarrelHazardType::PyrotechnicGasCylinder => "Pyrotechnic Gas Cylinder (Propulsion Jet & Burst)",
            BarrelHazardType::DepletedUraniumWasteCask => "Depleted Uranium Waste Cask (Radioactive Field)",
        }
    }

    pub fn to_config(&self) -> HazardBarrelConfig {
        match self {
            BarrelHazardType::VolatileNapalmFuelDrum => HazardBarrelConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                health: 20.0,
                blast_damage: 160.0,
                blast_radius_uu: 320.0,
                momentum_transfer: 650.0,
                is_toxic_spill: false,
                toxic_dps: 0.0,
                toxic_duration_secs: 0.0,
                chain_detonation_delay_secs: 0.15,
                barrel_radius_uu: 24.0,
                barrel_height_uu: 64.0,
                tag: "FuelDrum0".to_string(),
                event_on_explode: "NapalmExplosion".to_string(),
            },
            BarrelHazardType::CorrosiveSludgeAcidCanister => HazardBarrelConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                health: 15.0,
                blast_damage: 80.0,
                blast_radius_uu: 240.0,
                momentum_transfer: 300.0,
                is_toxic_spill: true,
                toxic_dps: 15.0,
                toxic_duration_secs: 8.0,
                chain_detonation_delay_secs: 0.25,
                barrel_radius_uu: 20.0,
                barrel_height_uu: 56.0,
                tag: "SludgeCanister0".to_string(),
                event_on_explode: "AcidSpill".to_string(),
            },
            BarrelHazardType::ReinforcedIndustrialKeg => HazardBarrelConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                health: 100.0,
                blast_damage: 220.0,
                blast_radius_uu: 420.0,
                momentum_transfer: 900.0,
                is_toxic_spill: false,
                toxic_dps: 0.0,
                toxic_duration_secs: 0.0,
                chain_detonation_delay_secs: 0.10,
                barrel_radius_uu: 30.0,
                barrel_height_uu: 72.0,
                tag: "HeavyKeg0".to_string(),
                event_on_explode: "IndustrialBurst".to_string(),
            },
            BarrelHazardType::PyrotechnicGasCylinder => HazardBarrelConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                health: 10.0,
                blast_damage: 120.0,
                blast_radius_uu: 280.0,
                momentum_transfer: 1200.0,
                is_toxic_spill: false,
                toxic_dps: 0.0,
                toxic_duration_secs: 0.0,
                chain_detonation_delay_secs: 0.30,
                barrel_radius_uu: 16.0,
                barrel_height_uu: 80.0,
                tag: "GasCylinder0".to_string(),
                event_on_explode: "GasRocketJet".to_string(),
            },
            BarrelHazardType::DepletedUraniumWasteCask => HazardBarrelConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                health: 50.0,
                blast_damage: 180.0,
                blast_radius_uu: 360.0,
                momentum_transfer: 500.0,
                is_toxic_spill: true,
                toxic_dps: 25.0,
                toxic_duration_secs: 12.0,
                chain_detonation_delay_secs: 0.20,
                barrel_radius_uu: 28.0,
                barrel_height_uu: 60.0,
                tag: "UraniumCask0".to_string(),
                event_on_explode: "RadiationBreach".to_string(),
            },
        }
    }
}

/// Configuration settings for an explosive hazard barrel
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardBarrelConfig {
    pub archetype: BarrelHazardType,
    pub position: Vec3,
    pub health: f32,
    pub blast_damage: f32,
    pub blast_radius_uu: f32,
    pub momentum_transfer: f32,
    pub is_toxic_spill: bool,
    pub toxic_dps: f32,
    pub toxic_duration_secs: f32,
    pub chain_detonation_delay_secs: f32,
    pub barrel_radius_uu: f32,
    pub barrel_height_uu: f32,
    pub tag: String,
    pub event_on_explode: String,
}

impl Default for HazardBarrelConfig {
    fn default() -> Self {
        BarrelHazardType::VolatileNapalmFuelDrum.to_config()
    }
}

/// Representation of an existing barrel or player start in the level
#[derive(Debug, Clone, PartialEq)]
pub struct LevelBarrelContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub tag: String,
    pub position: Vec3,
}

/// Telemetry metrics for an explosive barrel
#[derive(Debug, Clone, PartialEq)]
pub struct HazardBarrelMetrics {
    pub lethal_kill_radius_uu: f32,
    pub chain_reaction_count: usize,
    pub toxic_hazard_footprint_sq_uu: f32,
    pub impulse_ejection_velocity: f32,
    pub total_barrels_in_level: usize,
}

/// Safety diagnostics for barrel hazards
#[derive(Debug, Clone, PartialEq)]
pub struct HazardBarrelSafetyAlert {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates blast physics, lethal kill radius, chain reaction count, and impulse force
pub fn evaluate_hazard_barrel_metrics(
    config: &HazardBarrelConfig,
    peers: &[LevelBarrelContextActorRecord],
) -> HazardBarrelMetrics {
    // In UT99 splash damage falls off linearly from center:
    // Damage(r) = BlastDamage * (1.0 - r / BlastRadius)
    // Lethal radius is where Damage(r) >= 100 HP (base player health)
    let lethal_radius = if config.blast_damage > 100.0 && config.blast_radius_uu > 0.0 {
        config.blast_radius_uu * (1.0 - (100.0 / config.blast_damage))
    } else {
        0.0
    };

    // Calculate how many adjacent barrels fall within blast radius to form a chain reaction
    let mut chain_count = 0;
    let mut barrels_in_level = 1;
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("barrel") || cls_lower.contains("keg") || cls_lower.contains("canister") {
            barrels_in_level += 1;
            let dist = (p.position - config.position).length();
            if dist > 1.0 && dist <= config.blast_radius_uu {
                chain_count += 1;
            }
        }
    }

    let toxic_footprint = if config.is_toxic_spill {
        std::f32::consts::PI * config.blast_radius_uu * 0.75 * config.blast_radius_uu * 0.75
    } else {
        0.0
    };

    // Impulse velocity imparted to player (v = momentum / mass, where mass ~ 100 UT units)
    let impulse_vel = config.momentum_transfer / 100.0 * 250.0;

    HazardBarrelMetrics {
        lethal_kill_radius_uu: lethal_radius,
        chain_reaction_count: chain_count,
        toxic_hazard_footprint_sq_uu: toxic_footprint,
        impulse_ejection_velocity: impulse_vel,
        total_barrels_in_level: barrels_in_level,
    }
}

/// Audits safety for spawn-camping hazards, dense cluster lag, and runaway cascades
pub fn audit_hazard_barrel_safety(
    config: &HazardBarrelConfig,
    peers: &[LevelBarrelContextActorRecord],
) -> Vec<HazardBarrelSafetyAlert> {
    let mut alerts = Vec::new();

    // 1. Proximity to PlayerStart (Instant Spawn-Kill Hazard)
    for p in peers {
        if p.class_name.eq_ignore_ascii_case("Engine.PlayerStart") {
            let dist = (p.position - config.position).length();
            if dist <= config.blast_radius_uu {
                alerts.push(HazardBarrelSafetyAlert {
                    title: "Hazardous Spawn-Kill Proximity".to_string(),
                    description: format!(
                        "Barrel is located {:.0} uu from PlayerStart '{}', which is within its blast radius ({:.0} uu). Spawning players risk instant lethal death!",
                        dist, p.name, config.blast_radius_uu
                    ),
                    is_critical: true,
                });
            }
        }
    }

    // 2. Excessive Cluster Density (> 5 barrels within 150 uu)
    let mut close_neighbors = 0;
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("barrel") || cls_lower.contains("keg") || cls_lower.contains("canister") {
            let dist = (p.position - config.position).length();
            if dist > 1.0 && dist <= 150.0 {
                close_neighbors += 1;
            }
        }
    }
    if close_neighbors >= 5 {
        alerts.push(HazardBarrelSafetyAlert {
            title: "Excessive Barrel Cluster Density".to_string(),
            description: format!(
                "Found {} explosive barrels clustered within 150 uu. Simultaneous detonation may cause server frame-rate hitching and physics simulation spikes.",
                close_neighbors + 1
            ),
            is_critical: false,
        });
    }

    // 3. Extreme Blast Radius Warning (> 500 uu)
    if config.blast_radius_uu > 500.0 {
        alerts.push(HazardBarrelSafetyAlert {
            title: "Catastrophic Blast Radius".to_string(),
            description: format!(
                "Blast radius of {:.0} uu will clear an entire arena sector or room. Ensure adequate cover walls are present to prevent unintended cross-room splash.",
                config.blast_radius_uu
            ),
            is_critical: false,
        });
    }

    // 4. Lethal Lingering Toxic Spill
    if config.is_toxic_spill && config.toxic_dps >= 20.0 && config.toxic_duration_secs >= 10.0 {
        alerts.push(HazardBarrelSafetyAlert {
            title: "Lethal Area Denial Contamination".to_string(),
            description: format!(
                "Toxic spill deals {:.0} DPS for {:.1}s (total possible damage: {:.0} HP). Corridors contaminated by this barrel will be impassable without invulnerability.",
                config.toxic_dps, config.toxic_duration_secs, config.toxic_dps * config.toxic_duration_secs
            ),
            is_critical: false,
        });
    }

    alerts
}

/// Generates a procedural 8-sided faceted cylindrical barrel brush in CSG with top and bottom rims
pub fn generate_hazard_barrel_brush(config: &HazardBarrelConfig) -> Brush {
    let mut brush = Brush::new("Hazard_Barrel".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let sides = 8;
    let r = config.barrel_radius_uu;
    let h = config.barrel_height_uu;
    let _half_h = h * 0.5;

    // Generate circular vertices for bottom and top
    let mut bottom_verts = Vec::with_capacity(sides);
    let mut top_verts = Vec::with_capacity(sides);

    for i in 0..sides {
        let angle = (i as f32) / (sides as f32) * std::f32::consts::TAU;
        let x = angle.cos() * r;
        let y = angle.sin() * r;
        bottom_verts.push(Vec3::new(x, y, 0.0));
        top_verts.push(Vec3::new(x, y, h));
    }

    // Side Faces
    for i in 0..sides {
        let next = (i + 1) % sides;
        let p0 = bottom_verts[i];
        let p1 = bottom_verts[next];
        let p2 = top_verts[next];
        let p3 = top_verts[i];

        if let Some(mut poly) = Polygon::new(vec![p0, p1, p2, p3]) {
            poly.texture = "Indus1.Indus_B01".to_string();
            brush.polys.push(poly);
        }
    }

    // Bottom Face (facing down)
    let mut bot_rev = bottom_verts.clone();
    bot_rev.reverse();
    if let Some(mut poly) = Polygon::new(bot_rev) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    // Top Face (facing up)
    if let Some(mut poly) = Polygon::new(top_verts) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hazard_barrel_archetypes_and_defaults() {
        let cfg = BarrelHazardType::VolatileNapalmFuelDrum.to_config();
        assert_eq!(cfg.blast_damage, 160.0);
        assert_eq!(cfg.blast_radius_uu, 320.0);
        assert!(!cfg.is_toxic_spill);

        let acid = BarrelHazardType::CorrosiveSludgeAcidCanister.to_config();
        assert!(acid.is_toxic_spill);
        assert_eq!(acid.toxic_dps, 15.0);
    }

    #[test]
    fn test_hazard_barrel_metrics_evaluation() {
        let cfg = HazardBarrelConfig::default(); // 160 dmg, 320 radius
        let peers = vec![
            LevelBarrelContextActorRecord {
                name: "Barrel1".to_string(),
                class_name: "Botpack.UT_SteelBarrel".to_string(),
                tag: "FuelDrum0".to_string(),
                position: Vec3::new(200.0, 0.0, 0.0), // within 320 uu
            },
            LevelBarrelContextActorRecord {
                name: "Barrel2".to_string(),
                class_name: "Botpack.UT_SteelBarrel".to_string(),
                tag: "FuelDrum0".to_string(),
                position: Vec3::new(600.0, 0.0, 0.0), // outside 320 uu
            },
        ];

        let metrics = evaluate_hazard_barrel_metrics(&cfg, &peers);
        // Lethal radius: 320 * (1.0 - 100/160) = 320 * 0.375 = 120 uu
        assert_eq!(metrics.lethal_kill_radius_uu, 120.0);
        assert_eq!(metrics.chain_reaction_count, 1);
        assert_eq!(metrics.total_barrels_in_level, 3);
    }

    #[test]
    fn test_hazard_barrel_safety_alert_spawn_kill() {
        let cfg = HazardBarrelConfig::default(); // radius 320 uu
        let peers = vec![
            LevelBarrelContextActorRecord {
                name: "PlayerStart0".to_string(),
                class_name: "Engine.PlayerStart".to_string(),
                tag: "".to_string(),
                position: Vec3::new(100.0, 0.0, 0.0), // 100 uu < 320 uu
            }
        ];

        let alerts = audit_hazard_barrel_safety(&cfg, &peers);
        assert!(alerts.iter().any(|a| a.is_critical && a.title.contains("Hazardous Spawn-Kill Proximity")));
    }

    #[test]
    fn test_generate_hazard_barrel_brush() {
        let cfg = HazardBarrelConfig::default();
        let brush = generate_hazard_barrel_brush(&cfg);

        assert_eq!(brush.name, "Hazard_Barrel");
        // 8 sides + top + bottom = 10 polygons
        assert_eq!(brush.polys.len(), 10);
    }
}
