// crates/ut99-bsp/src/assault_studio.rs
//
// Interactive Assault Objective, Blast Door & Objective Chain Studio
// Sequential objective chains (FortStandard, DestroyableTarget, TriggerSwitch, TouchZone),
// attacker spawn advance shifts, time limit bonus telemetry, defense difficulty ratings,
// progression hazard auditing, and procedural reinforced blast door brush generation.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical UT99 Assault Objective Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssaultObjectiveType {
    /// Target must be damaged until destroyed (Blast Door, Power Generator, Computer Core)
    DestroyableTarget,
    /// Target must be activated / triggered (Lever, Keycard Console, Terminal Hack)
    TriggerSwitch,
    /// Player must enter touch zone / trigger volume (Command Bridge, Escape Pod)
    TouchZone,
}

impl AssaultObjectiveType {
    pub fn display_name(&self) -> &'static str {
        match self {
            AssaultObjectiveType::DestroyableTarget => "Destroyable Target (Damage/Health)",
            AssaultObjectiveType::TriggerSwitch => "Trigger Console / Switch (Use/Activate)",
            AssaultObjectiveType::TouchZone => "Infiltration Touch Zone (Player Volume)",
        }
    }

    pub fn default_actor_class(&self) -> &'static str {
        match self {
            AssaultObjectiveType::DestroyableTarget => "Botpack.FortStandard",
            AssaultObjectiveType::TriggerSwitch => "Engine.Trigger",
            AssaultObjectiveType::TouchZone => "Engine.Trigger",
        }
    }
}

/// Archetypes of canonical Assault missions in Unreal Tournament 99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssaultArchetype {
    OverlordBeachLanding,
    HiSpeedTrainInfiltration,
    FrigateNavalAssault,
    RookCastleSiege,
    MazonPowerExtraction,
}

impl AssaultArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            AssaultArchetype::OverlordBeachLanding => "AS-Overlord Beachhead (Gun Breach -> Gate -> Main Core)",
            AssaultArchetype::HiSpeedTrainInfiltration => "AS-HiSpeed Maglev Train (Caboose -> Engine Decoupler -> Cockpit)",
            AssaultArchetype::FrigateNavalAssault => "AS-Frigate Warship Infiltration (Hatch -> Torpedo Bay -> Engine Scuttle)",
            AssaultArchetype::RookCastleSiege => "AS-Rook Fortress Siege (Gate Winch -> Courtyard -> Throne Seal)",
            AssaultArchetype::MazonPowerExtraction => "AS-Mazon Substation (Perimeter Node -> Cooling Grid -> Reactor Core)",
        }
    }

    pub fn to_config(&self) -> AssaultChainConfig {
        match self {
            AssaultArchetype::OverlordBeachLanding => AssaultChainConfig {
                archetype: *self,
                objective_order: 1,
                objective_name: "Bunker Gun Breach".to_string(),
                objective_type: AssaultObjectiveType::DestroyableTarget,
                position: Vec3::new(0.0, 0.0, 0.0),
                target_health: 600.0,
                time_limit_bonus_secs: 180.0,
                forward_spawn_advance_dist_uu: 800.0,
                door_width: 160.0,
                door_height: 128.0,
                door_thickness: 16.0,
                tag: "Obj1_BunkerGun".to_string(),
                event_on_complete: "Door1_Open".to_string(),
            },
            AssaultArchetype::HiSpeedTrainInfiltration => AssaultChainConfig {
                archetype: *self,
                objective_order: 2,
                objective_name: "Decouple Engine Clamps".to_string(),
                objective_type: AssaultObjectiveType::TriggerSwitch,
                position: Vec3::new(400.0, 0.0, 32.0),
                target_health: 0.0,
                time_limit_bonus_secs: 120.0,
                forward_spawn_advance_dist_uu: 600.0,
                door_width: 120.0,
                door_height: 110.0,
                door_thickness: 12.0,
                tag: "Obj2_EngineClamp".to_string(),
                event_on_complete: "TrainDecoupled".to_string(),
            },
            AssaultArchetype::FrigateNavalAssault => AssaultChainConfig {
                archetype: *self,
                objective_order: 1,
                objective_name: "Boarding Hatch Override".to_string(),
                objective_type: AssaultObjectiveType::TriggerSwitch,
                position: Vec3::new(-300.0, 200.0, 0.0),
                target_health: 0.0,
                time_limit_bonus_secs: 150.0,
                forward_spawn_advance_dist_uu: 500.0,
                door_width: 96.0,
                door_height: 96.0,
                door_thickness: 14.0,
                tag: "Obj1_HatchSwitch".to_string(),
                event_on_complete: "HatchUnsealed".to_string(),
            },
            AssaultArchetype::RookCastleSiege => AssaultChainConfig {
                archetype: *self,
                objective_order: 2,
                objective_name: "Courtyard Blast Gate".to_string(),
                objective_type: AssaultObjectiveType::DestroyableTarget,
                position: Vec3::new(250.0, 0.0, 64.0),
                target_health: 800.0,
                time_limit_bonus_secs: 140.0,
                forward_spawn_advance_dist_uu: 750.0,
                door_width: 192.0,
                door_height: 144.0,
                door_thickness: 20.0,
                tag: "Obj2_BlastGate".to_string(),
                event_on_complete: "GateDestroyed".to_string(),
            },
            AssaultArchetype::MazonPowerExtraction => AssaultChainConfig {
                archetype: *self,
                objective_order: 3,
                objective_name: "Reactor Core Meltdown".to_string(),
                objective_type: AssaultObjectiveType::DestroyableTarget,
                position: Vec3::new(600.0, -100.0, -40.0),
                target_health: 1000.0,
                time_limit_bonus_secs: 200.0,
                forward_spawn_advance_dist_uu: 900.0,
                door_width: 140.0,
                door_height: 120.0,
                door_thickness: 16.0,
                tag: "Obj3_ReactorCore".to_string(),
                event_on_complete: "MissionSuccess".to_string(),
            },
        }
    }
}

/// Configuration settings for an Assault Objective
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssaultChainConfig {
    pub archetype: AssaultArchetype,
    pub objective_order: u8, // 1-indexed progression order
    pub objective_name: String,
    pub objective_type: AssaultObjectiveType,
    pub position: Vec3,
    pub target_health: f32, // For DestroyableTarget (e.g. 500-1000)
    pub time_limit_bonus_secs: f32,
    pub forward_spawn_advance_dist_uu: f32,
    pub door_width: f32,
    pub door_height: f32,
    pub door_thickness: f32,
    pub tag: String,
    pub event_on_complete: String,
}

impl Default for AssaultChainConfig {
    fn default() -> Self {
        AssaultArchetype::OverlordBeachLanding.to_config()
    }
}

/// Representation of an existing Assault Objective in the level
#[derive(Debug, Clone, PartialEq)]
pub struct LevelAssaultObjectiveRecord {
    pub name: String,
    pub order: u8,
    pub position: Vec3,
    pub objective_type: AssaultObjectiveType,
    pub event_on_complete: String,
}

/// Analytical metrics for Assault chain pacing and progression
#[derive(Debug, Clone, PartialEq)]
pub struct AssaultMetrics {
    pub total_objectives_in_chain: usize,
    pub cumulative_time_bonus_secs: f32,
    pub defense_difficulty_index: f32, // 0 to 100
    pub distance_to_next_objective_uu: f32,
    pub attacker_sprint_advance_time_secs: f32,
}

/// Hazards and balance diagnostics for Assault chains
#[derive(Debug, Clone, PartialEq)]
pub struct AssaultHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates Assault chain progression metrics
pub fn evaluate_assault_metrics(
    config: &AssaultChainConfig,
    chain: &[LevelAssaultObjectiveRecord],
) -> AssaultMetrics {
    let mut total_objs = 1; // including current config
    let mut cum_time = config.time_limit_bonus_secs;
    let mut next_dist = 9999.0_f32;

    for obj in chain {
        total_objs += 1;
        cum_time += 120.0; // default estimated bonus
        if obj.order == config.objective_order + 1 {
            next_dist = obj.position.distance(config.position);
        }
    }

    let sprint_adv = config.forward_spawn_advance_dist_uu / 400.0;

    // Difficulty heuristic based on health and order
    let diff = (config.target_health / 20.0 + (config.objective_order as f32 * 15.0)).clamp(10.0, 95.0);

    AssaultMetrics {
        total_objectives_in_chain: total_objs,
        cumulative_time_bonus_secs: cum_time,
        defense_difficulty_index: diff,
        distance_to_next_objective_uu: next_dist,
        attacker_sprint_advance_time_secs: sprint_adv,
    }
}

/// Audits Assault setup for sequence breaks, zero health and missing trigger links
pub fn audit_assault_safety(
    config: &AssaultChainConfig,
    chain: &[LevelAssaultObjectiveRecord],
) -> Vec<AssaultHazard> {
    let mut hazards = Vec::new();

    // 1. Critical Invulnerable Target Hazard
    if config.objective_type == AssaultObjectiveType::DestroyableTarget && config.target_health <= 0.0 {
        hazards.push(AssaultHazard {
            title: "Critical Invulnerable Assault Target".to_string(),
            description: "DestroyableTarget has 0 or negative health. Attackers will be completely unable to destroy this objective to progress.".to_string(),
            is_critical: true,
        });
    }

    // 2. Critical Missing Event Trigger Hazard
    if config.event_on_complete.trim().is_empty() {
        hazards.push(AssaultHazard {
            title: "Critical Missing Completion Event Trigger".to_string(),
            description: "No EventOnComplete string specified. The game engine cannot fire forward spawn shift or unlock sequential blast doors.".to_string(),
            is_critical: true,
        });
    }

    // 3. Sequence Order Conflict Warning
    for obj in chain {
        if obj.order == config.objective_order && obj.name != config.tag {
            hazards.push(AssaultHazard {
                title: "Duplicate Objective Order Index".to_string(),
                description: format!(
                    "Objective order index #{} is already used by '{}'. Ensure unique sequential order indices (1, 2, 3...) for proper HUD assault progression.",
                    config.objective_order, obj.name
                ),
                is_critical: false,
            });
            break;
        }
    }

    // 4. Backward Spawn Shift Warning
    if config.forward_spawn_advance_dist_uu < 0.0 {
        hazards.push(AssaultHazard {
            title: "Negative Spawn Shift Distance".to_string(),
            description: "Forward spawn shift distance is negative, causing attackers to spawn further back upon completing an objective.".to_string(),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a reinforced industrial blast door brush for the objective
pub fn generate_blast_door_brush(config: &AssaultChainConfig) -> Brush {
    let mut brush = Brush::new("AssaultBlastDoor".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let hw = config.door_width * 0.5;
    let hh = config.door_height * 0.5;
    let ht = config.door_thickness * 0.5;

    // 8 vertices for rectangular door slab
    let v = [
        Vec3::new(-hw, -ht, -hh),
        Vec3::new(hw, -ht, -hh),
        Vec3::new(hw, ht, -hh),
        Vec3::new(-hw, ht, -hh),
        Vec3::new(-hw, -ht, hh),
        Vec3::new(hw, -ht, hh),
        Vec3::new(hw, ht, hh),
        Vec3::new(-hw, ht, hh),
    ];

    let faces = [
        vec![v[0], v[1], v[5], v[4]], // Front
        vec![v[2], v[3], v[7], v[6]], // Back
        vec![v[3], v[0], v[4], v[7]], // Left
        vec![v[1], v[2], v[6], v[5]], // Right
        vec![v[4], v[5], v[6], v[7]], // Top
        vec![v[3], v[2], v[1], v[0]], // Bottom
    ];

    for f in faces {
        if let Some(poly) = Polygon::new(f) {
            brush.polys.push(poly);
        }
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assault_archetypes_and_defaults() {
        let cfg = AssaultArchetype::OverlordBeachLanding.to_config();
        assert_eq!(cfg.objective_order, 1);
        assert_eq!(cfg.objective_type, AssaultObjectiveType::DestroyableTarget);
        assert_eq!(cfg.target_health, 600.0);
        assert!(cfg.door_width >= 100.0);
    }

    #[test]
    fn test_assault_metrics_evaluation() {
        let cfg = AssaultChainConfig::default();
        let chain = vec![LevelAssaultObjectiveRecord {
            name: "Objective2".to_string(),
            order: 2,
            position: Vec3::new(750.0, 0.0, 0.0),
            objective_type: AssaultObjectiveType::TriggerSwitch,
            event_on_complete: "Door2Open".to_string(),
        }];

        let metrics = evaluate_assault_metrics(&cfg, &chain);
        assert_eq!(metrics.total_objectives_in_chain, 2);
        assert_eq!(metrics.distance_to_next_objective_uu, 750.0);
        assert_eq!(metrics.attacker_sprint_advance_time_secs, 2.0); // 800 / 400 = 2.0s
        assert!(metrics.defense_difficulty_index > 30.0);
    }

    #[test]
    fn test_assault_hazard_auditing() {
        let mut cfg = AssaultChainConfig::default();
        cfg.target_health = 0.0; // Invulnerable!
        cfg.event_on_complete = "".to_string(); // Missing trigger!

        let hazards = audit_assault_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Invulnerable")));
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Missing Completion Event")));
    }

    #[test]
    fn test_generate_blast_door_brush() {
        let cfg = AssaultChainConfig::default();
        let brush = generate_blast_door_brush(&cfg);

        assert_eq!(brush.name, "AssaultBlastDoor");
        assert_eq!(brush.polys.len(), 6);
    }
}
