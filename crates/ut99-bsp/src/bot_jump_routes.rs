// crates/ut99-bsp/src/bot_jump_routes.rs
//
// Interactive Bot AI Tactical JumpSpot & Translocator Route Studio
//
// Ballistic parabolic trajectory calculation, Impact Hammer self-damage jump modeling,
// JumpBoot vertical amplification, bot translocator toss beacon simulation,
// landing zone error tolerance, obstacle ray-clearance, and JumpSpot generation.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// UT99 Advanced Bot Tactical Jump & Toss Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotJumpType {
    /// Standard bot jump (Z velocity = +325 uu/s, max clearance ~64 uu)
    StandardJump,
    /// Dodge leap with horizontal velocity boost (XY velocity = +400 uu/s, Z = +210 uu/s)
    DodgeLeap,
    /// Impact Hammer boosted jump (Z = +720 uu/s, 36 HP self-damage, max clearance ~280 uu)
    ImpactHammerJump,
    /// Anti-Gravity JumpBoots (3x jump velocity = +950 uu/s, max clearance ~460 uu)
    JumpBootsJump,
    /// Bot Translocator Beacon Toss (V0 = 750 uu/s, parabolic toss to perch)
    TranslocatorToss,
}

impl BotJumpType {
    pub fn display_name(&self) -> &'static str {
        match self {
            BotJumpType::StandardJump => "Standard Navigation Jump (64 uu)",
            BotJumpType::DodgeLeap => "Tactical Dodge Leap (120 uu horiz)",
            BotJumpType::ImpactHammerJump => "Impact Hammer Jump Boost (280 uu, 36 HP)",
            BotJumpType::JumpBootsJump => "Anti-Grav JumpBoots Leap (460 uu)",
            BotJumpType::TranslocatorToss => "Bot Translocator Beacon Toss (750 uu/s)",
        }
    }

    pub fn initial_vertical_velocity(&self) -> f32 {
        match self {
            BotJumpType::StandardJump => 325.0,
            BotJumpType::DodgeLeap => 210.0,
            BotJumpType::ImpactHammerJump => 720.0,
            BotJumpType::JumpBootsJump => 950.0,
            BotJumpType::TranslocatorToss => 0.0, // calculated from aim pitch
        }
    }

    pub fn horizontal_speed_uu_s(&self) -> f32 {
        match self {
            BotJumpType::StandardJump => 400.0,
            BotJumpType::DodgeLeap => 600.0,
            BotJumpType::ImpactHammerJump => 440.0,
            BotJumpType::JumpBootsJump => 420.0,
            BotJumpType::TranslocatorToss => 750.0,
        }
    }

    pub fn self_damage_hp(&self) -> f32 {
        match self {
            BotJumpType::ImpactHammerJump => 36.0,
            _ => 0.0,
        }
    }

    pub fn requires_translocator(&self) -> bool {
        matches!(self, BotJumpType::TranslocatorToss)
    }

    pub fn trajectory_color_rgb(&self) -> [f32; 4] {
        match self {
            BotJumpType::StandardJump => [0.3, 0.7, 1.0, 0.85],      // Cyan
            BotJumpType::DodgeLeap => [0.2, 0.9, 0.4, 0.90],         // Emerald
            BotJumpType::ImpactHammerJump => [1.0, 0.7, 0.1, 0.95],   // Amber
            BotJumpType::JumpBootsJump => [0.8, 0.3, 1.0, 0.95],     // Violet
            BotJumpType::TranslocatorToss => [1.0, 0.25, 0.25, 0.95], // Crimson
        }
    }
}

/// Pre-configured JumpSpot & Translocator archetypes from canonical competitive maps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotJumpArchetype {
    Deck16SlimeBridgeJump,
    FaceSniperTowerClimb,
    PhobosGravityLeap,
    CoretFlagRunAscent,
    TurbineVentVault,
}

impl BotJumpArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            BotJumpArchetype::Deck16SlimeBridgeJump => "Deck16][ Toxic Slime Bridge Jump (Impact Hammer)",
            BotJumpArchetype::FaceSniperTowerClimb => "CTF-Face Sniper Tower Ascent (Translocator Toss)",
            BotJumpArchetype::PhobosGravityLeap => "DM-Phobos Low-Gravity Chasm Leap",
            BotJumpArchetype::CoretFlagRunAscent => "CTF-Coret JumpBoot Rapid Base Skip",
            BotJumpArchetype::TurbineVentVault => "DM-Turbine Air Duct Dodge-Vault",
        }
    }

    pub fn to_config(&self) -> BotJumpRoutesConfig {
        match self {
            BotJumpArchetype::Deck16SlimeBridgeJump => BotJumpRoutesConfig {
                archetype: *self,
                start_pos: Vec3::new(0.0, 0.0, 0.0),
                target_pos: Vec3::new(380.0, 0.0, 160.0),
                jump_type: BotJumpType::ImpactHammerJump,
                gravity_uu_s2: 950.0,
                landing_tolerance_radius: 64.0,
                bot_skill_level: 5,
                b_always_translocate: false,
                b_impact_jump: true,
                plate_radius: 48.0,
                plate_height: 12.0,
                tag: "Deck16HammerJump".to_string(),
            },
            BotJumpArchetype::FaceSniperTowerClimb => BotJumpRoutesConfig {
                archetype: *self,
                start_pos: Vec3::new(0.0, 0.0, 0.0),
                target_pos: Vec3::new(180.0, 0.0, 240.0),
                jump_type: BotJumpType::TranslocatorToss,
                gravity_uu_s2: 950.0,
                landing_tolerance_radius: 48.0,
                bot_skill_level: 6,
                b_always_translocate: true,
                b_impact_jump: false,
                plate_radius: 40.0,
                plate_height: 8.0,
                tag: "FaceTowerToss".to_string(),
            },
            BotJumpArchetype::PhobosGravityLeap => BotJumpRoutesConfig {
                archetype: *self,
                start_pos: Vec3::new(0.0, 0.0, 0.0),
                target_pos: Vec3::new(600.0, 0.0, 60.0),
                jump_type: BotJumpType::DodgeLeap,
                gravity_uu_s2: 300.0,
                landing_tolerance_radius: 80.0,
                bot_skill_level: 4,
                b_always_translocate: false,
                b_impact_jump: false,
                plate_radius: 56.0,
                plate_height: 12.0,
                tag: "PhobosLowGLeap".to_string(),
            },
            BotJumpArchetype::CoretFlagRunAscent => BotJumpRoutesConfig {
                archetype: *self,
                start_pos: Vec3::new(0.0, 0.0, 0.0),
                target_pos: Vec3::new(320.0, 0.0, 380.0),
                jump_type: BotJumpType::JumpBootsJump,
                gravity_uu_s2: 950.0,
                landing_tolerance_radius: 72.0,
                bot_skill_level: 5,
                b_always_translocate: false,
                b_impact_jump: false,
                plate_radius: 48.0,
                plate_height: 16.0,
                tag: "CoretBootAscent".to_string(),
            },
            BotJumpArchetype::TurbineVentVault => BotJumpRoutesConfig {
                archetype: *self,
                start_pos: Vec3::new(0.0, 0.0, 0.0),
                target_pos: Vec3::new(280.0, 120.0, 80.0),
                jump_type: BotJumpType::StandardJump,
                gravity_uu_s2: 950.0,
                landing_tolerance_radius: 56.0,
                bot_skill_level: 3,
                b_always_translocate: false,
                b_impact_jump: false,
                plate_radius: 44.0,
                plate_height: 8.0,
                tag: "TurbineVentJump".to_string(),
            },
        }
    }
}

/// Configuration settings for Bot AI Tactical JumpSpot & Translocator Route Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BotJumpRoutesConfig {
    pub archetype: BotJumpArchetype,
    pub start_pos: Vec3,
    pub target_pos: Vec3,
    pub jump_type: BotJumpType,
    pub gravity_uu_s2: f32,
    pub landing_tolerance_radius: f32,
    pub bot_skill_level: u32, // 1 to 7 (Novice to Godlike)
    pub b_always_translocate: bool,
    pub b_impact_jump: bool,
    pub plate_radius: f32,
    pub plate_height: f32,
    pub tag: String,
}

impl Default for BotJumpRoutesConfig {
    fn default() -> Self {
        BotJumpArchetype::Deck16SlimeBridgeJump.to_config()
    }
}

/// Computed ballistic trajectory solution
#[derive(Debug, Clone, PartialEq)]
pub struct JumpTrajectorySolution {
    pub is_reachable: bool,
    pub trajectory_points: Vec<Vec3>,
    pub launch_velocity: Vec3,
    pub apex_height_uu: f32,
    pub total_flight_time_secs: f32,
    pub landing_error_uu: f32,
    pub traversal_cost: u32,
}

/// Calculates parabolic trajectory arc between start and target positions
pub fn solve_jump_trajectory(config: &BotJumpRoutesConfig) -> JumpTrajectorySolution {
    let diff = config.target_pos - config.start_pos;
    let horizontal_diff = Vec3::new(diff.x, diff.y, 0.0);
    let horizontal_dist = horizontal_diff.length();
    let vertical_dist = diff.z;

    let g = config.gravity_uu_s2.max(10.0);
    let dir_xy = if horizontal_dist > 0.001 {
        horizontal_diff / horizontal_dist
    } else {
        Vec3::X
    };

    let mut is_reachable = true;
    let launch_vel: Vec3;
    let total_time: f32;

    match config.jump_type {
        BotJumpType::TranslocatorToss => {
            // Translocator speed is fixed at 750 uu/s. Solve for launch angle theta.
            let v = 750.0_f32;
            let v2 = v * v;
            let v4 = v2 * v2;
            let discriminant = v4 - g * (g * horizontal_dist * horizontal_dist + 2.0 * vertical_dist * v2);

            if discriminant >= 0.0 {
                // High arc solution for target perches
                let angle_rad = ((v2 + discriminant.sqrt()) / (g * horizontal_dist.max(1.0))).atan();
                let vx = v * angle_rad.cos();
                let vz = v * angle_rad.sin();
                launch_vel = dir_xy * vx + Vec3::new(0.0, 0.0, vz);
                total_time = (horizontal_dist / vx.max(1.0)).clamp(0.1, 10.0);
            } else {
                is_reachable = false;
                // Best effort forward throw
                let angle_rad = 45.0_f32.to_radians();
                launch_vel = dir_xy * (v * angle_rad.cos()) + Vec3::new(0.0, 0.0, v * angle_rad.sin());
                total_time = 1.0;
            }
        }
        _ => {
            let vz0 = config.jump_type.initial_vertical_velocity();
            let v_xy_speed = config.jump_type.horizontal_speed_uu_s();

            // Calculate flight time to land at target Z:
            // z(t) = vz0 * t - 0.5 * g * t^2 => 0.5 * g * t^2 - vz0 * t + vertical_dist = 0
            let a = 0.5 * g;
            let b = -vz0;
            let c = vertical_dist;
            let disc = b * b - 4.0 * a * c;

            if disc >= 0.0 {
                let t_land = (-b + disc.sqrt()) / (2.0 * a);
                total_time = t_land.max(0.1);
                // Required horizontal speed to cover distance in that time
                let req_v_xy = horizontal_dist / total_time;
                if req_v_xy > v_xy_speed * 1.25 {
                    is_reachable = false; // Exceeds max jump speed
                }
                launch_vel = dir_xy * req_v_xy.min(v_xy_speed * 1.5) + Vec3::new(0.0, 0.0, vz0);
            } else {
                is_reachable = false;
                total_time = (vz0 / g) * 2.0;
                launch_vel = dir_xy * v_xy_speed + Vec3::new(0.0, 0.0, vz0);
            }
        }
    }

    // Discretize parabolic trajectory points (32 segments)
    let segments = 32;
    let mut trajectory_points = Vec::with_capacity(segments + 1);
    let mut max_z = config.start_pos.z;

    for i in 0..=segments {
        let t = (i as f32 / segments as f32) * total_time;
        let pos = config.start_pos
            + Vec3::new(launch_vel.x * t, launch_vel.y * t, launch_vel.z * t - 0.5 * g * t * t);
        if pos.z > max_z {
            max_z = pos.z;
        }
        trajectory_points.push(pos);
    }

    let end_pt = *trajectory_points.last().unwrap_or(&config.target_pos);
    let landing_error = end_pt.distance(config.target_pos);
    if landing_error > config.landing_tolerance_radius * 1.5 {
        is_reachable = false;
    }

    let apex_height = (max_z - config.start_pos.z).max(0.0);
    let cost = (total_time * 100.0) as u32 + (config.jump_type.self_damage_hp() * 20.0) as u32;

    JumpTrajectorySolution {
        is_reachable,
        trajectory_points,
        launch_velocity: launch_vel,
        apex_height_uu: apex_height,
        total_flight_time_secs: total_time,
        landing_error_uu: landing_error,
        traversal_cost: cost,
    }
}

/// Analytical metrics for bot tactical jump route
#[derive(Debug, Clone, PartialEq)]
pub struct BotJumpMetrics {
    pub is_physically_viable: bool,
    pub total_horizontal_distance_uu: f32,
    pub total_vertical_drop_uu: f32,
    pub apex_clearance_uu: f32,
    pub flight_duration_secs: f32,
    pub bot_traversal_cost_rating: u32,
    pub min_bot_skill_required: u32,
    pub self_damage_penalty: f32,
}

/// Hazard diagnostic issues for bot navigation routes
#[derive(Debug, Clone, PartialEq)]
pub struct BotJumpHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

pub fn evaluate_jump_metrics(
    config: &BotJumpRoutesConfig,
    solution: &JumpTrajectorySolution,
) -> BotJumpMetrics {
    let diff = config.target_pos - config.start_pos;
    let horiz_dist = Vec3::new(diff.x, diff.y, 0.0).length();

    let min_skill = match config.jump_type {
        BotJumpType::StandardJump => 2,
        BotJumpType::DodgeLeap => 4,
        BotJumpType::ImpactHammerJump => 5,
        BotJumpType::JumpBootsJump => 3,
        BotJumpType::TranslocatorToss => 5,
    };

    BotJumpMetrics {
        is_physically_viable: solution.is_reachable,
        total_horizontal_distance_uu: horiz_dist,
        total_vertical_drop_uu: diff.z,
        apex_clearance_uu: solution.apex_height_uu,
        flight_duration_secs: solution.total_flight_time_secs,
        bot_traversal_cost_rating: solution.traversal_cost,
        min_bot_skill_required: min_skill,
        self_damage_penalty: config.jump_type.self_damage_hp(),
    }
}

/// Audits physical clearance, landing tolerance, and translocator hazards
pub fn audit_jump_route_safety(
    config: &BotJumpRoutesConfig,
    solution: &JumpTrajectorySolution,
) -> Vec<BotJumpHazard> {
    let mut hazards = Vec::new();

    // 1. Physically Unreachable
    if !solution.is_reachable {
        hazards.push(BotJumpHazard {
            title: "Unreachable Jump / Toss Target".to_string(),
            description: format!(
                "The target position is outside the ballistic envelope of {:?}. Bots will repeatedly fail and fall.",
                config.jump_type.display_name()
            ),
            is_critical: true,
        });
    }

    // 2. High Landing Error
    if solution.landing_error_uu > config.landing_tolerance_radius {
        hazards.push(BotJumpHazard {
            title: "Dangerous Landing Dispersion".to_string(),
            description: format!(
                "Landing point misses target by {:.1} uu (exceeds tolerance of {:.1} uu).",
                solution.landing_error_uu, config.landing_tolerance_radius
            ),
            is_critical: true,
        });
    }

    // 3. Impact Hammer Suicide Risk
    if config.jump_type == BotJumpType::ImpactHammerJump && config.b_impact_jump {
        hazards.push(BotJumpHazard {
            title: "Impact Hammer Self-Damage Hazard".to_string(),
            description: "Takes 36 HP self-damage. Low-health bots (<36 HP) will self-terminate on jump launch.".to_string(),
            is_critical: false,
        });
    }

    // 4. Missing Translocator Flag
    if config.jump_type == BotJumpType::TranslocatorToss && !config.b_always_translocate {
        hazards.push(BotJumpHazard {
            title: "bAlwaysTranslocate Disabled on Toss Route".to_string(),
            description: "Bot translocator toss requires bAlwaysTranslocate=true on JumpSpot, otherwise bots attempt to walk off ledge.".to_string(),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural circular jump pad / route beacon plate
pub fn generate_jump_plate_brush(config: &BotJumpRoutesConfig) -> Brush {
    let mut brush = Brush::new("JumpPlate".to_string(), CsgOperation::Active);
    brush.location = config.start_pos;

    let sides = 8;
    let r = config.plate_radius;
    let h = config.plate_height;

    let mut bottom_verts = Vec::with_capacity(sides);
    let mut top_verts = Vec::with_capacity(sides);

    for i in 0..sides {
        let angle = (i as f32 / sides as f32) * std::f32::consts::TAU;
        let x = angle.cos() * r;
        let y = angle.sin() * r;
        bottom_verts.push(Vec3::new(x, y, 0.0));
        top_verts.push(Vec3::new(x, y, h));
    }

    // Top face
    if let Some(poly) = Polygon::new(top_verts.clone()) {
        brush.polys.push(poly);
    }

    // Bottom face
    let mut bot_rev = bottom_verts.clone();
    bot_rev.reverse();
    if let Some(poly) = Polygon::new(bot_rev) {
        brush.polys.push(poly);
    }

    // Sides
    for i in 0..sides {
        let next = (i + 1) % sides;
        let quad = vec![
            bottom_verts[i],
            bottom_verts[next],
            top_verts[next],
            top_verts[i],
        ];
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
    fn test_solve_jump_trajectory_deck16_hammer() {
        let cfg = BotJumpArchetype::Deck16SlimeBridgeJump.to_config();
        let sol = solve_jump_trajectory(&cfg);

        assert!(sol.is_reachable, "Deck16 hammer jump should be reachable");
        assert!(sol.apex_height_uu > 100.0, "Hammer boost must generate non-trivial apex height");
        assert_eq!(sol.trajectory_points.len(), 33); // 32 segments + 1
        assert!(sol.total_flight_time_secs > 0.3 && sol.total_flight_time_secs < 2.0);
    }

    #[test]
    fn test_solve_translocator_toss_face_tower() {
        let cfg = BotJumpArchetype::FaceSniperTowerClimb.to_config();
        let sol = solve_jump_trajectory(&cfg);

        assert!(sol.is_reachable, "Face sniper tower climb must be reachable with translocator");
        assert!(sol.apex_height_uu > 200.0);
        assert!(sol.launch_velocity.z > 0.0);
    }

    #[test]
    fn test_jump_metrics_and_hazard_audit() {
        let cfg = BotJumpArchetype::Deck16SlimeBridgeJump.to_config();
        let sol = solve_jump_trajectory(&cfg);
        let metrics = evaluate_jump_metrics(&cfg, &sol);
        let hazards = audit_jump_route_safety(&cfg, &sol);

        assert!(metrics.is_physically_viable);
        assert_eq!(metrics.self_damage_penalty, 36.0);
        // Hammer jump produces advisory warning for self-damage
        assert!(hazards.iter().any(|h| !h.is_critical && h.title.contains("Self-Damage")));

        // Test impossible jump (target 5000 uu away)
        let mut impossible_cfg = cfg.clone();
        impossible_cfg.target_pos = Vec3::new(5000.0, 0.0, 0.0);
        let sol_imp = solve_jump_trajectory(&impossible_cfg);
        let hazards_imp = audit_jump_route_safety(&impossible_cfg, &sol_imp);
        assert!(hazards_imp.iter().any(|h| h.is_critical && h.title.contains("Unreachable")));
    }

    #[test]
    fn test_generate_jump_plate_brush() {
        let cfg = BotJumpArchetype::Deck16SlimeBridgeJump.to_config();
        let brush = generate_jump_plate_brush(&cfg);

        assert_eq!(brush.name, "JumpPlate");
        assert_eq!(brush.polys.len(), 10); // 8 sides + 2 caps
    }
}
