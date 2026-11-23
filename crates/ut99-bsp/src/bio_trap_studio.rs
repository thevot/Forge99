// crates/ut99-bsp/src/bio_trap_studio.rs
//
// Interactive Bio-Rifle Slime Trap & Volumetric Hazard Field Studio
// Secondary fire charged gel glob traps, surface adhesion normal reflection, toxic slime coverage,
// detonation timers, environmental hazards, and procedural drainage grate brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Mode of toxic gel dispersion and surface attachment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BioDispersionMode {
    ConcentratedGlob,
    DispersedMinefield,
    WallAdhesiveWeb,
    CorrosivePool,
}

impl BioDispersionMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            BioDispersionMode::ConcentratedGlob => "Concentrated Bio-Glob (Max Burst Lethality)",
            BioDispersionMode::DispersedMinefield => "Dispersed Minefield (Multi-Droplet Perimeter)",
            BioDispersionMode::WallAdhesiveWeb => "Wall Adhesive Web (Vertical & Ceiling Trap)",
            BioDispersionMode::CorrosivePool => "Corrosive Pool (Persistent Lingering Caustic Acid)",
        }
    }

    pub fn default_splash_radius(&self) -> f32 {
        match self {
            BioDispersionMode::ConcentratedGlob => 240.0,
            BioDispersionMode::DispersedMinefield => 380.0,
            BioDispersionMode::WallAdhesiveWeb => 180.0,
            BioDispersionMode::CorrosivePool => 320.0,
        }
    }
}

/// Archetypes of canonical Bio-Rifle traps and environmental hazards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BioTrapArchetype {
    ChokepointDoorwayAmbushTrap,
    FlagPedestalSurroundMinefield,
    ElevatorShaftToxicWeb,
    CorridorAreaDenialVolley,
    ReactorVatSpillZone,
}

impl BioTrapArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            BioTrapArchetype::ChokepointDoorwayAmbushTrap => "Chokepoint Doorway Ambush Trap (10x Glob)",
            BioTrapArchetype::FlagPedestalSurroundMinefield => "Flag Pedestal Surround Minefield (Perimeter Denier)",
            BioTrapArchetype::ElevatorShaftToxicWeb => "Elevator Shaft Toxic Web (Lift Ambush)",
            BioTrapArchetype::CorridorAreaDenialVolley => "Corridor Area Denial Volley (Floor Carpet)",
            BioTrapArchetype::ReactorVatSpillZone => "Reactor Vat Spill Basin (Industrial Slime)",
        }
    }

    pub fn to_config(&self) -> BioTrapConfig {
        match self {
            BioTrapArchetype::ChokepointDoorwayAmbushTrap => BioTrapConfig {
                archetype: *self,
                dispersion_mode: BioDispersionMode::ConcentratedGlob,
                position: Vec3::new(0.0, 0.0, 0.0),
                surface_normal: Vec3::new(0.0, 0.0, 1.0),
                glob_charge_count: 10,
                splash_radius_uu: 240.0,
                detonation_timeout_secs: 15.0,
                burst_damage_hp: 220.0,
                continuous_dps: 15.0,
                grate_radius: 48.0,
                grate_height: 16.0,
                tag: "DoorAmbush_Slime".to_string(),
            },
            BioTrapArchetype::FlagPedestalSurroundMinefield => BioTrapConfig {
                archetype: *self,
                dispersion_mode: BioDispersionMode::DispersedMinefield,
                position: Vec3::new(0.0, 0.0, 0.0),
                surface_normal: Vec3::new(0.0, 0.0, 1.0),
                glob_charge_count: 8,
                splash_radius_uu: 360.0,
                detonation_timeout_secs: 20.0,
                burst_damage_hp: 160.0,
                continuous_dps: 20.0,
                grate_radius: 64.0,
                grate_height: 16.0,
                tag: "FlagPerimeter_Slime".to_string(),
            },
            BioTrapArchetype::ElevatorShaftToxicWeb => BioTrapConfig {
                archetype: *self,
                dispersion_mode: BioDispersionMode::WallAdhesiveWeb,
                position: Vec3::new(0.0, 0.0, 128.0),
                surface_normal: Vec3::new(1.0, 0.0, 0.0), // Wall mounted
                glob_charge_count: 6,
                splash_radius_uu: 180.0,
                detonation_timeout_secs: 12.0,
                burst_damage_hp: 120.0,
                continuous_dps: 10.0,
                grate_radius: 36.0,
                grate_height: 12.0,
                tag: "LiftShaft_Slime".to_string(),
            },
            BioTrapArchetype::CorridorAreaDenialVolley => BioTrapConfig {
                archetype: *self,
                dispersion_mode: BioDispersionMode::DispersedMinefield,
                position: Vec3::new(0.0, 0.0, 0.0),
                surface_normal: Vec3::new(0.0, 0.0, 1.0),
                glob_charge_count: 9,
                splash_radius_uu: 320.0,
                detonation_timeout_secs: 18.0,
                burst_damage_hp: 180.0,
                continuous_dps: 25.0,
                grate_radius: 56.0,
                grate_height: 16.0,
                tag: "CorridorVolley_Slime".to_string(),
            },
            BioTrapArchetype::ReactorVatSpillZone => BioTrapConfig {
                archetype: *self,
                dispersion_mode: BioDispersionMode::CorrosivePool,
                position: Vec3::new(0.0, 0.0, 0.0),
                surface_normal: Vec3::new(0.0, 0.0, 1.0),
                glob_charge_count: 10,
                splash_radius_uu: 420.0,
                detonation_timeout_secs: 30.0,
                burst_damage_hp: 100.0,
                continuous_dps: 45.0, // High corrosive acid DPS
                grate_radius: 80.0,
                grate_height: 20.0,
                tag: "ReactorSpill_Acid".to_string(),
            },
        }
    }
}

/// Configuration settings for Bio-Rifle Slime Trap & Hazard
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BioTrapConfig {
    pub archetype: BioTrapArchetype,
    pub dispersion_mode: BioDispersionMode,
    pub position: Vec3,
    pub surface_normal: Vec3,
    pub glob_charge_count: u8,
    pub splash_radius_uu: f32,
    pub detonation_timeout_secs: f32,
    pub burst_damage_hp: f32,
    pub continuous_dps: f32,
    pub grate_radius: f32,
    pub grate_height: f32,
    pub tag: String,
}

impl Default for BioTrapConfig {
    fn default() -> Self {
        BioTrapArchetype::ChokepointDoorwayAmbushTrap.to_config()
    }
}

/// Representation of an existing actor in the map for hazard proximity auditing
#[derive(Debug, Clone, PartialEq)]
pub struct LevelBioContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Telemetry metrics for Bio-Trap lethal capability
#[derive(Debug, Clone, PartialEq)]
pub struct BioTrapMetrics {
    pub total_burst_damage_hp: f32,
    pub hazard_footprint_area_sq_uu: f32,
    pub time_to_lethal_kill_secs: f32,
    pub total_traps_in_level: usize,
}

/// Safety diagnostics and balance alerts
#[derive(Debug, Clone, PartialEq)]
pub struct BioTrapHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates lethal burst damage, toxic footprint area, and kill latency
pub fn evaluate_bio_trap_metrics(
    config: &BioTrapConfig,
    peers: &[LevelBioContextActorRecord],
) -> BioTrapMetrics {
    let burst = config.burst_damage_hp * (config.glob_charge_count as f32 / 10.0).max(0.2);
    let area = std::f32::consts::PI * config.splash_radius_uu.powi(2);

    let dps_total = burst + config.continuous_dps;
    let ttk = if dps_total > 0.0 {
        (100.0 / dps_total).min(10.0)
    } else {
        10.0
    };

    let mut traps = 1;
    for p in peers {
        let lower = p.class_name.to_lowercase();
        if lower.contains("bio") || lower.contains("slime") || lower.contains("gel") {
            traps += 1;
        }
    }

    BioTrapMetrics {
        total_burst_damage_hp: burst,
        hazard_footprint_area_sq_uu: area,
        time_to_lethal_kill_secs: ttk,
        total_traps_in_level: traps,
    }
}

/// Audits safety for friendly spawn contamination, impassable chokepoints, and low ceiling burst
pub fn audit_bio_trap_safety(
    config: &BioTrapConfig,
    peers: &[LevelBioContextActorRecord],
) -> Vec<BioTrapHazard> {
    let mut hazards = Vec::new();

    // 1. Spawn Contamination: Slime hazard too close to player spawn
    for p in peers {
        let lower = p.class_name.to_lowercase();
        if lower.contains("playerstart") {
            let dist = p.position.distance(config.position);
            if dist < config.splash_radius_uu + 80.0 {
                hazards.push(BioTrapHazard {
                    title: "Spawn Point Contamination".to_string(),
                    description: format!(
                        "Slime trap at distance {:.0} uu overlaps PlayerStart '{}'. Freshly spawned players will take instant lethal acid damage!",
                        dist, p.name
                    ),
                    is_critical: true,
                });
            }
        }
    }

    // 2. Impassable Giant Radius Hazard (>350 uu)
    if config.splash_radius_uu > 350.0 && config.burst_damage_hp >= 150.0 {
        hazards.push(BioTrapHazard {
            title: "Over-Sized Impassable Chokepoint".to_string(),
            description: format!(
                "Splash radius of {:.0} uu with {:.0} HP burst completely shuts down standard corridors without dodge clearance.",
                config.splash_radius_uu, config.burst_damage_hp
            ),
            is_critical: false,
        });
    }

    // 3. Subsecond Detonation Timeout (<4s)
    if config.detonation_timeout_secs < 4.0 {
        hazards.push(BioTrapHazard {
            title: "Premature Detonation Timeout".to_string(),
            description: format!(
                "Detonation timeout of {:.1}s is too short. The gel trap will burst before any enemy player arrives.",
                config.detonation_timeout_secs
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural octagonal drainage basin brush for chemical containment
pub fn generate_bio_drain_grate_brush(config: &BioTrapConfig) -> Brush {
    let mut brush = Brush::new("BioChemicalDrainGrate".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let sides = 8;
    let r = config.grate_radius;
    let h = config.grate_height;

    let mut lower_verts = Vec::new();
    let mut upper_verts = Vec::new();

    for i in 0..sides {
        let angle = std::f32::consts::TAU * (i as f32) / (sides as f32);
        let x = r * angle.cos();
        let y = r * angle.sin();
        lower_verts.push(Vec3::new(x, y, 0.0));
        upper_verts.push(Vec3::new(x * 0.9, y * 0.9, h));
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
    fn test_bio_trap_archetypes_and_defaults() {
        let cfg = BioTrapArchetype::ChokepointDoorwayAmbushTrap.to_config();
        assert_eq!(cfg.dispersion_mode, BioDispersionMode::ConcentratedGlob);
        assert_eq!(cfg.glob_charge_count, 10);
        assert_eq!(cfg.burst_damage_hp, 220.0);
        assert_eq!(cfg.splash_radius_uu, 240.0);
    }

    #[test]
    fn test_bio_trap_metrics_evaluation() {
        let cfg = BioTrapConfig::default();
        let peers = vec![LevelBioContextActorRecord {
            name: "OtherBioGel".to_string(),
            class_name: "Botpack.BioGel".to_string(),
            position: Vec3::new(600.0, 0.0, 0.0),
        }];

        let metrics = evaluate_bio_trap_metrics(&cfg, &peers);
        assert_eq!(metrics.total_traps_in_level, 2);
        assert_eq!(metrics.total_burst_damage_hp, 220.0);
        assert!(metrics.hazard_footprint_area_sq_uu > 100_000.0);
    }

    #[test]
    fn test_bio_trap_hazard_auditing() {
        let cfg = BioTrapConfig::default(); // splash = 240.0
        let peers = vec![LevelBioContextActorRecord {
            name: "PlayerStart_0".to_string(),
            class_name: "Engine.PlayerStart".to_string(),
            position: Vec3::new(180.0, 0.0, 0.0), // 180 < 240 + 80 = 320 uu!
        }];

        let hazards = audit_bio_trap_safety(&cfg, &peers);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Spawn Point Contamination")));
    }

    #[test]
    fn test_generate_bio_drain_grate_brush() {
        let cfg = BioTrapConfig::default();
        let brush = generate_bio_drain_grate_brush(&cfg);

        assert_eq!(brush.name, "BioChemicalDrainGrate");
        // 1 bottom cap + 8 sides + 1 top cap = 10 polys
        assert_eq!(brush.polys.len(), 10);
    }
}
