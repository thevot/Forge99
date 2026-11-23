// crates/ut99-bsp/src/elevator_studio.rs
//
// Interactive Elevator, LiftCenter/LiftExit & Vertical Transit Studio
// Multi-floor elevators, keyframe height offsets, ascent/descent velocity, stay time,
// bot LiftCenter & LiftExit pairing, crush damage encroachment, and procedural elevator platform brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Encroachment behavior when elevator hits a player or bot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncroachBehavior {
    StopWhenEncroached,
    ReturnWhenEncroached,
    CrushWhenEncroached,
    IgnoreEncroachment,
}

impl EncroachBehavior {
    pub fn display_name(&self) -> &'static str {
        match self {
            EncroachBehavior::StopWhenEncroached => "Stop (ME_StopWhenEncroach)",
            EncroachBehavior::ReturnWhenEncroached => "Return (ME_ReturnWhenEncroach)",
            EncroachBehavior::CrushWhenEncroached => "Crush (ME_CrushWhenEncroach)",
            EncroachBehavior::IgnoreEncroachment => "Ignore (ME_IgnoreWhenEncroach)",
        }
    }
}

/// Archetypes of canonical UT99 elevators and lifts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElevatorArchetype {
    StandardHydraulicLift,
    DoubleDeckerExpress,
    SubterraneanCargoDrop,
    HighSpeedTowerHoist,
    GravityPistonShaft,
}

impl ElevatorArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            ElevatorArchetype::StandardHydraulicLift => "Standard Hydraulic Arena Lift (Deck16 Style)",
            ElevatorArchetype::DoubleDeckerExpress => "Double Decker Express Lift (Curse Central Pillar)",
            ElevatorArchetype::SubterraneanCargoDrop => "Subterranean Deep Cargo Hoist (Phobos Shaft)",
            ElevatorArchetype::HighSpeedTowerHoist => "High-Speed Tower Hoist (Face Sniper Lift)",
            ElevatorArchetype::GravityPistonShaft => "Pneumatic Gravity Piston (Overlord Bunker Lift)",
        }
    }

    pub fn to_config(&self) -> ElevatorConfig {
        match self {
            ElevatorArchetype::StandardHydraulicLift => ElevatorConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                platform_width_uu: 160.0,
                platform_length_uu: 160.0,
                platform_thickness_uu: 16.0,
                travel_height_uu: 384.0,
                move_time_secs: 2.5,
                stay_open_time_secs: 3.0,
                encroach_damage: 50.0,
                encroach_behavior: EncroachBehavior::ReturnWhenEncroached,
                tag: "DeckLift0".to_string(),
                has_intermediate_stop: false,
            },
            ElevatorArchetype::DoubleDeckerExpress => ElevatorConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                platform_width_uu: 200.0,
                platform_length_uu: 200.0,
                platform_thickness_uu: 20.0,
                travel_height_uu: 640.0,
                move_time_secs: 4.0,
                stay_open_time_secs: 2.5,
                encroach_damage: 40.0,
                encroach_behavior: EncroachBehavior::StopWhenEncroached,
                tag: "ExpressLift".to_string(),
                has_intermediate_stop: true,
            },
            ElevatorArchetype::SubterraneanCargoDrop => ElevatorConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                platform_width_uu: 256.0,
                platform_length_uu: 256.0,
                platform_thickness_uu: 24.0,
                travel_height_uu: 896.0,
                move_time_secs: 5.5,
                stay_open_time_secs: 4.0,
                encroach_damage: 150.0,
                encroach_behavior: EncroachBehavior::CrushWhenEncroached,
                tag: "CargoShaftLift".to_string(),
                has_intermediate_stop: false,
            },
            ElevatorArchetype::HighSpeedTowerHoist => ElevatorConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                platform_width_uu: 128.0,
                platform_length_uu: 128.0,
                platform_thickness_uu: 12.0,
                travel_height_uu: 768.0,
                move_time_secs: 1.8,
                stay_open_time_secs: 2.0,
                encroach_damage: 80.0,
                encroach_behavior: EncroachBehavior::ReturnWhenEncroached,
                tag: "TowerHoistLift".to_string(),
                has_intermediate_stop: false,
            },
            ElevatorArchetype::GravityPistonShaft => ElevatorConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                platform_width_uu: 144.0,
                platform_length_uu: 144.0,
                platform_thickness_uu: 16.0,
                travel_height_uu: 480.0,
                move_time_secs: 1.4,
                stay_open_time_secs: 2.0,
                encroach_damage: 200.0,
                encroach_behavior: EncroachBehavior::CrushWhenEncroached,
                tag: "PistonLift".to_string(),
                has_intermediate_stop: false,
            },
        }
    }
}

/// Configuration settings for Elevator and Vertical Transit Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElevatorConfig {
    pub archetype: ElevatorArchetype,
    pub position: Vec3,
    pub platform_width_uu: f32,
    pub platform_length_uu: f32,
    pub platform_thickness_uu: f32,
    pub travel_height_uu: f32,
    pub move_time_secs: f32,
    pub stay_open_time_secs: f32,
    pub encroach_damage: f32,
    pub encroach_behavior: EncroachBehavior,
    pub tag: String,
    pub has_intermediate_stop: bool,
}

impl Default for ElevatorConfig {
    fn default() -> Self {
        ElevatorArchetype::StandardHydraulicLift.to_config()
    }
}

/// Representation of an existing actor in the map for lift network resolution
#[derive(Debug, Clone, PartialEq)]
pub struct LevelElevatorContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub tag: String,
    pub lift_tag: String,
    pub position: Vec3,
}

/// Telemetry metrics for Elevator
#[derive(Debug, Clone, PartialEq)]
pub struct ElevatorMetrics {
    pub ascent_velocity_uu_per_sec: f32,
    pub round_trip_period_secs: f32,
    pub total_lift_exits_wired: usize,
    pub has_lift_center: bool,
    pub platform_area_sq_uu: f32,
    pub total_elevators_in_level: usize,
}

/// Safety diagnostics and bot pathing alerts
#[derive(Debug, Clone, PartialEq)]
pub struct ElevatorSafetyHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates ascent velocity, round-trip period, bot waypoint resolution, and platform area
pub fn evaluate_elevator_metrics(
    config: &ElevatorConfig,
    peers: &[LevelElevatorContextActorRecord],
) -> ElevatorMetrics {
    let vel = if config.move_time_secs > 0.0 {
        config.travel_height_uu / config.move_time_secs
    } else {
        0.0
    };

    let round_trip = 2.0 * config.move_time_secs + 2.0 * config.stay_open_time_secs;
    let area = config.platform_width_uu * config.platform_length_uu;

    let mut lift_exits = 0;
    let mut has_center = false;
    let mut lifts_count = 1;

    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("mover") || cls_lower.contains("elevator") {
            lifts_count += 1;
        }

        let tag_matches = !config.tag.is_empty() && (
            p.lift_tag.eq_ignore_ascii_case(&config.tag) || p.tag.eq_ignore_ascii_case(&config.tag)
        );

        if tag_matches {
            if cls_lower.contains("liftcenter") {
                has_center = true;
            } else if cls_lower.contains("liftexit") {
                lift_exits += 1;
            }
        }
    }

    ElevatorMetrics {
        ascent_velocity_uu_per_sec: vel,
        round_trip_period_secs: round_trip,
        total_lift_exits_wired: lift_exits,
        has_lift_center: has_center,
        platform_area_sq_uu: area,
        total_elevators_in_level: lifts_count,
    }
}

/// Audits safety for missing LiftCenter/LiftExit pathing, excessive velocity, and lethal squishing
pub fn audit_elevator_safety(
    config: &ElevatorConfig,
    peers: &[LevelElevatorContextActorRecord],
) -> Vec<ElevatorSafetyHazard> {
    let mut hazards = Vec::new();

    // 1. Missing LiftCenter: Bots cannot identify where to stand on the lift
    let has_center = peers.iter().any(|p| {
        p.class_name.to_lowercase().contains("liftcenter") &&
        (p.lift_tag.eq_ignore_ascii_case(&config.tag) || p.tag.eq_ignore_ascii_case(&config.tag))
    });
    if !has_center && !peers.is_empty() {
        hazards.push(ElevatorSafetyHazard {
            title: "Missing LiftCenter PathNode".to_string(),
            description: format!(
                "Lift with Tag '{}' has no Botpack.LiftCenter actor. Bots will not board or ride this elevator!",
                config.tag
            ),
            is_critical: true,
        });
    }

    // 2. Missing LiftExits: At least 2 LiftExits needed (top & bottom)
    let exit_count = peers.iter().filter(|p| {
        p.class_name.to_lowercase().contains("liftexit") &&
        (p.lift_tag.eq_ignore_ascii_case(&config.tag) || p.tag.eq_ignore_ascii_case(&config.tag))
    }).count();

    if exit_count < 2 && !peers.is_empty() {
        hazards.push(ElevatorSafetyHazard {
            title: "Incomplete LiftExit Landings".to_string(),
            description: format!(
                "Lift requires LiftExit actors at both bottom and top landings (found {}). Bots may get stranded on the platform.",
                exit_count
            ),
            is_critical: false,
        });
    }

    // 3. Excessive Transit Velocity (>1000 uu/s)
    let vel = if config.move_time_secs > 0.0 {
        config.travel_height_uu / config.move_time_secs
    } else {
        0.0
    };
    if vel > 1000.0 {
        hazards.push(ElevatorSafetyHazard {
            title: "Dangerous Transit Velocity".to_string(),
            description: format!(
                "Elevator velocity of {:.0} uu/s is extremely high. Players may clip through the platform or launch violently into ceilings.",
                vel
            ),
            is_critical: false,
        });
    }

    // 4. Lethal Squish Hazard (EncroachDamage >= 100 with Crush)
    if config.encroach_damage >= 100.0 && config.encroach_behavior == EncroachBehavior::CrushWhenEncroached {
        hazards.push(ElevatorSafetyHazard {
            title: "Instant Lethal Crush Hazard".to_string(),
            description: format!(
                "Elevator deals {:.0} damage on encroachment with Crush enabled. Players caught underneath will be instantly gibbed.",
                config.encroach_damage
            ),
            is_critical: false,
        });
    }

    // 5. Brief Stay Time (< 1.5s)
    if config.stay_open_time_secs < 1.5 {
        hazards.push(ElevatorSafetyHazard {
            title: "Short Boarding Window".to_string(),
            description: format!(
                "Stay open time of {:.1}s is very short. Bots and players may miss the boarding window.",
                config.stay_open_time_secs
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural rectangular elevator platform brush in CSG
pub fn generate_elevator_platform_brush(config: &ElevatorConfig) -> Brush {
    let mut brush = Brush::new("Elevator_Platform".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let hw = config.platform_width_uu * 0.5;
    let hl = config.platform_length_uu * 0.5;
    let h = config.platform_thickness_uu;

    let p0 = Vec3::new(-hw, -hl, 0.0);
    let p1 = Vec3::new(hw, -hl, 0.0);
    let p2 = Vec3::new(hw, hl, 0.0);
    let p3 = Vec3::new(-hw, hl, 0.0);

    let p4 = Vec3::new(-hw, -hl, h);
    let p5 = Vec3::new(hw, -hl, h);
    let p6 = Vec3::new(hw, hl, h);
    let p7 = Vec3::new(-hw, hl, h);

    // 1. Bottom Cap
    if let Some(poly) = Polygon::new(vec![p0, p1, p2, p3]) {
        brush.polys.push(poly);
    }

    // 2. Sides
    if let Some(poly) = Polygon::new(vec![p0, p4, p5, p1]) { brush.polys.push(poly); }
    if let Some(poly) = Polygon::new(vec![p1, p5, p6, p2]) { brush.polys.push(poly); }
    if let Some(poly) = Polygon::new(vec![p2, p6, p7, p3]) { brush.polys.push(poly); }
    if let Some(poly) = Polygon::new(vec![p3, p7, p4, p0]) { brush.polys.push(poly); }

    // 3. Top Cap
    if let Some(poly) = Polygon::new(vec![p7, p6, p5, p4]) {
        brush.polys.push(poly);
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevator_archetypes_and_defaults() {
        let cfg = ElevatorArchetype::StandardHydraulicLift.to_config();
        assert_eq!(cfg.travel_height_uu, 384.0);
        assert_eq!(cfg.move_time_secs, 2.5);
        assert_eq!(cfg.stay_open_time_secs, 3.0);
        assert_eq!(cfg.encroach_behavior, EncroachBehavior::ReturnWhenEncroached);
    }

    #[test]
    fn test_elevator_metrics_evaluation() {
        let cfg = ElevatorConfig::default();
        let peers = vec![
            LevelElevatorContextActorRecord {
                name: "LiftCenter0".to_string(),
                class_name: "Botpack.LiftCenter".to_string(),
                tag: "DeckLift0".to_string(),
                lift_tag: "DeckLift0".to_string(),
                position: Vec3::new(0.0, 0.0, 20.0),
            },
            LevelElevatorContextActorRecord {
                name: "LiftExit0".to_string(),
                class_name: "Botpack.LiftExit".to_string(),
                tag: "DeckLift0".to_string(),
                lift_tag: "DeckLift0".to_string(),
                position: Vec3::new(100.0, 0.0, 0.0),
            },
            LevelElevatorContextActorRecord {
                name: "LiftExit1".to_string(),
                class_name: "Botpack.LiftExit".to_string(),
                tag: "DeckLift0".to_string(),
                lift_tag: "DeckLift0".to_string(),
                position: Vec3::new(100.0, 0.0, 384.0),
            },
        ];

        let metrics = evaluate_elevator_metrics(&cfg, &peers);
        assert!(metrics.has_lift_center);
        assert_eq!(metrics.total_lift_exits_wired, 2);
        assert_eq!(metrics.ascent_velocity_uu_per_sec, 384.0 / 2.5);
        assert_eq!(metrics.platform_area_sq_uu, 160.0 * 160.0);
    }

    #[test]
    fn test_elevator_safety_hazard_auditing() {
        let mut cfg = ElevatorConfig::default();
        cfg.move_time_secs = 0.2; // 384 / 0.2 = 1920 uu/s -> dangerous!
        cfg.stay_open_time_secs = 0.5; // Short stay

        let peers = vec![
            LevelElevatorContextActorRecord {
                name: "Unrelated".to_string(),
                class_name: "Engine.Light".to_string(),
                tag: "".to_string(),
                lift_tag: "".to_string(),
                position: Vec3::ZERO,
            }
        ];

        let hazards = audit_elevator_safety(&cfg, &peers);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Missing LiftCenter")));
        assert!(hazards.iter().any(|h| h.title.contains("Dangerous Transit Velocity")));
        assert!(hazards.iter().any(|h| h.title.contains("Short Boarding Window")));
    }

    #[test]
    fn test_generate_elevator_platform_brush() {
        let cfg = ElevatorConfig::default();
        let brush = generate_elevator_platform_brush(&cfg);

        assert_eq!(brush.name, "Elevator_Platform");
        // 1 bottom cap + 4 sides + 1 top cap = 6 polys
        assert_eq!(brush.polys.len(), 6);
    }
}
