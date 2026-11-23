// crates/ut99-bsp/src/stairway.rs
//
// Interactive Curved Architectural Stairway, Spiral Ramp & Smooth Collision Hull Studio
//
// Deep domain mathematical engine for parametric linear, curved, spiral, and switchback stairs,
// automated smooth collision wedge/ramp generation (PF_INVISIBLE | PF_SEMISOLID),
// UT99 MaxStepHeight (25 uu) ergonomics safety audit, and bot centerline navigation pathing.

use crate::brush::Brush;
use crate::poly::{Polygon, PF_INVISIBLE, PF_SEMISOLID};
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Classical Unreal Tournament & Unreal Engine 1 stair archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StairArchetype {
    LinearStraight,
    Curved90QuarterTurn,
    Curved180HalfTurn,
    SpiralTower,
    SwitchbackLanding,
}

impl StairArchetype {
    pub fn all() -> &'static [StairArchetype] {
        &[
            StairArchetype::LinearStraight,
            StairArchetype::Curved90QuarterTurn,
            StairArchetype::Curved180HalfTurn,
            StairArchetype::SpiralTower,
            StairArchetype::SwitchbackLanding,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            StairArchetype::LinearStraight => "Linear Straight Flight",
            StairArchetype::Curved90QuarterTurn => "Curved 90° Quarter-Turn",
            StairArchetype::Curved180HalfTurn => "Curved 180° Grand Half-Turn",
            StairArchetype::SpiralTower => "Spiral Tower Helical Stairs",
            StairArchetype::SwitchbackLanding => "Switchback with Resting Landing",
        }
    }

    pub fn default_config(&self) -> StairConfig {
        match self {
            StairArchetype::LinearStraight => StairConfig {
                archetype: *self,
                step_count: 12,
                step_height: 16.0,
                step_length: 32.0,
                step_width: 192.0,
                inner_radius: 96.0,
                sweep_angle_deg: 90.0,
                counter_clockwise: true,
                has_subgrade_solid_fill: true,
                add_smooth_collision_hull: true,
                collision_hull_solidity: CollisionHullSolidity::InvisibleSemiSolid,
                add_handrail_banister: false,
                bot_pathing_mode: StairBotPathingMode::AutoCenterlineWaypoints,
                texture_name: "Pallet.Wall.brick1".to_string(),
            },
            StairArchetype::Curved90QuarterTurn => StairConfig {
                archetype: *self,
                step_count: 8,
                step_height: 16.0,
                step_length: 32.0,
                step_width: 192.0,
                inner_radius: 128.0,
                sweep_angle_deg: 90.0,
                counter_clockwise: true,
                has_subgrade_solid_fill: true,
                add_smooth_collision_hull: true,
                collision_hull_solidity: CollisionHullSolidity::InvisibleSemiSolid,
                add_handrail_banister: false,
                bot_pathing_mode: StairBotPathingMode::AutoCenterlineWaypoints,
                texture_name: "Pallet.Wall.brick1".to_string(),
            },
            StairArchetype::Curved180HalfTurn => StairConfig {
                archetype: *self,
                step_count: 16,
                step_height: 16.0,
                step_length: 32.0,
                step_width: 256.0,
                inner_radius: 160.0,
                sweep_angle_deg: 180.0,
                counter_clockwise: true,
                has_subgrade_solid_fill: true,
                add_smooth_collision_hull: true,
                collision_hull_solidity: CollisionHullSolidity::InvisibleSemiSolid,
                add_handrail_banister: false,
                bot_pathing_mode: StairBotPathingMode::AutoCenterlineWaypoints,
                texture_name: "Pallet.Wall.brick1".to_string(),
            },
            StairArchetype::SpiralTower => StairConfig {
                archetype: *self,
                step_count: 24,
                step_height: 16.0,
                step_length: 32.0,
                step_width: 160.0,
                inner_radius: 64.0,
                sweep_angle_deg: 360.0,
                counter_clockwise: true,
                has_subgrade_solid_fill: false,
                add_smooth_collision_hull: true,
                collision_hull_solidity: CollisionHullSolidity::InvisibleSemiSolid,
                add_handrail_banister: true,
                bot_pathing_mode: StairBotPathingMode::AutoCenterlineWaypoints,
                texture_name: "Pallet.Wall.brick1".to_string(),
            },
            StairArchetype::SwitchbackLanding => StairConfig {
                archetype: *self,
                step_count: 16, // 8 lower flight + landing + 8 upper flight
                step_height: 16.0,
                step_length: 32.0,
                step_width: 160.0,
                inner_radius: 96.0,
                sweep_angle_deg: 180.0,
                counter_clockwise: true,
                has_subgrade_solid_fill: true,
                add_smooth_collision_hull: true,
                collision_hull_solidity: CollisionHullSolidity::InvisibleSemiSolid,
                add_handrail_banister: false,
                bot_pathing_mode: StairBotPathingMode::AutoCenterlineWaypoints,
                texture_name: "Pallet.Wall.brick1".to_string(),
            },
        }
    }
}

/// Collision hull surface flags and solidity mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollisionHullSolidity {
    /// PF_INVISIBLE | PF_SEMISOLID (Competitive Unreal Tournament standard: eliminates camera bobbing and zero BSP cuts)
    InvisibleSemiSolid,
    /// PF_INVISIBLE (Pure additive invisible brush)
    InvisibleSolid,
    /// PF_NOT_SOLID | PF_INVISIBLE (Visual guide / ghost only)
    NonSolid,
}

impl CollisionHullSolidity {
    pub fn poly_flags(&self) -> u32 {
        match self {
            CollisionHullSolidity::InvisibleSemiSolid => PF_INVISIBLE | PF_SEMISOLID,
            CollisionHullSolidity::InvisibleSolid => PF_INVISIBLE,
            CollisionHullSolidity::NonSolid => PF_INVISIBLE | crate::poly::PF_NOT_SOLID,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CollisionHullSolidity::InvisibleSemiSolid => "Invisible Semi-Solid (Recommended)",
            CollisionHullSolidity::InvisibleSolid => "Invisible Solid",
            CollisionHullSolidity::NonSolid => "Non-Solid (Guide Only)",
        }
    }
}

/// Bot pathing waypoint generation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StairBotPathingMode {
    AutoCenterlineWaypoints,
    None,
}

/// Complete configuration for stairway procedural generator
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StairConfig {
    pub archetype: StairArchetype,
    pub step_count: usize,
    /// Step riser height in Unreal Units (MaxStepHeight limit in UT99 is 25.0 uu)
    pub step_height: f32,
    /// Step tread run (depth) in Unreal Units
    pub step_length: f32,
    /// Step tread flight width in Unreal Units
    pub step_width: f32,
    /// Inner radius for curved and spiral stairs
    pub inner_radius: f32,
    /// Total sweep angle for curved and spiral stairs in degrees
    pub sweep_angle_deg: f32,
    pub counter_clockwise: bool,
    /// When true, step blocks extend down to floor Z=0; when false, steps are cantilevered / open-soffit
    pub has_subgrade_solid_fill: bool,
    /// When true, generates a matching smooth collision wedge / ramp
    pub add_smooth_collision_hull: bool,
    pub collision_hull_solidity: CollisionHullSolidity,
    /// When true, generates architectural guard rail / handrail
    pub add_handrail_banister: bool,
    pub bot_pathing_mode: StairBotPathingMode,
    pub texture_name: String,
}

impl Default for StairConfig {
    fn default() -> Self {
        StairArchetype::LinearStraight.default_config()
    }
}

/// Bot navigation waypoint generated along stairway centerline
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StairBotWaypoint {
    pub position: Vec3,
    pub name: String,
    pub is_landing: bool,
    pub reachspec_valid: bool,
}

/// Ergonomics and architectural metrics of the stairway
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StairMetrics {
    pub total_height: f32,
    pub total_horizontal_run: f32,
    pub slope_pitch_degrees: f32,
    pub step_count: usize,
    pub step_height: f32,
    pub step_run: f32,
    pub step_width: f32,
    /// Blondel's architectural cadence score (2 * Riser + Tread), ideal is ~64 uu for UT99
    pub blondel_score: f32,
}

/// Hazard severity rating
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StairHazardSeverity {
    Info,
    Warning,
    Critical,
}

/// Ergonomics or physics hazard flagged during stairway safety audit
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StairHazard {
    pub severity: StairHazardSeverity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Calculates comprehensive architectural and physics metrics for a stairway configuration
pub fn calculate_stair_metrics(config: &StairConfig) -> StairMetrics {
    let steps = config.step_count.max(1);
    let total_height = steps as f32 * config.step_height;

    let total_run = match config.archetype {
        StairArchetype::LinearStraight => steps as f32 * config.step_length,
        StairArchetype::SwitchbackLanding => {
            let half_steps = steps / 2;
            (half_steps as f32 * config.step_length) + config.step_width // including landing
        }
        StairArchetype::Curved90QuarterTurn
        | StairArchetype::Curved180HalfTurn
        | StairArchetype::SpiralTower => {
            let mid_radius = config.inner_radius + config.step_width * 0.5;
            let sweep_rad = config.sweep_angle_deg.to_radians();
            mid_radius * sweep_rad
        }
    };

    let step_run = match config.archetype {
        StairArchetype::LinearStraight | StairArchetype::SwitchbackLanding => config.step_length,
        _ => {
            let mid_radius = config.inner_radius + config.step_width * 0.5;
            let sweep_rad = config.sweep_angle_deg.to_radians();
            (mid_radius * sweep_rad) / steps as f32
        }
    };

    let slope_pitch_degrees = if total_run > 0.001 {
        (total_height / total_run).atan().to_degrees()
    } else {
        90.0
    };

    let blondel_score = (2.0 * config.step_height) + step_run;

    StairMetrics {
        total_height,
        total_horizontal_run: total_run,
        slope_pitch_degrees,
        step_count: steps,
        step_height: config.step_height,
        step_run,
        step_width: config.step_width,
        blondel_score,
    }
}

/// Audits the stairway for UT99 Pawn MaxStepHeight violations, excessive slope angles,
/// narrow treads, and missing smooth collision hulls.
pub fn audit_stair_safety(config: &StairConfig) -> Vec<StairHazard> {
    let mut hazards = Vec::new();
    let metrics = calculate_stair_metrics(config);

    // 1. MaxStepHeight Check (UT99 Pawn.MaxStepHeight is strictly 25.0 uu)
    if config.step_height > 24.0 {
        hazards.push(StairHazard {
            severity: StairHazardSeverity::Critical,
            title: format!("Step Riser ({:.1} uu) Exceeds UT99 MaxStepHeight (25.0 uu)", config.step_height),
            description: "In Unreal Tournament 99 physics, pawns and bots have a hard-coded MaxStepHeight of 25.0 uu. Any step riser > 24.0 uu prevents players and bots from walking up without jumping.".to_string(),
            recommendation: "Reduce step_height to <= 16.0 uu (mapping standard is 8 uu or 16 uu), or increase step_count.".to_string(),
        });
    } else if config.step_height > 18.0 {
        hazards.push(StairHazard {
            severity: StairHazardSeverity::Warning,
            title: format!("Steep Step Riser ({:.1} uu)", config.step_height),
            description: "Step risers between 18 uu and 24 uu cause noticeable player movement friction and occasional bot path hesitation.".to_string(),
            recommendation: "Consider lowering step_height to 16.0 uu for optimal competitive gameplay flow.".to_string(),
        });
    }

    // 2. Slope Pitch Angle Check
    if metrics.slope_pitch_degrees > 45.0 {
        hazards.push(StairHazard {
            severity: StairHazardSeverity::Warning,
            title: format!("Stairway Slope ({:.1}°) Exceeds 45° Incline", metrics.slope_pitch_degrees),
            description: "Slopes steeper than 45° trigger downhill player sliding and fall damage risks when descending at high speeds.".to_string(),
            recommendation: "Increase step_length/run or increase radius to broaden the slope below 40°.".to_string(),
        });
    }

    // 3. Tread Run Depth Check
    if metrics.step_run < 16.0 {
        hazards.push(StairHazard {
            severity: StairHazardSeverity::Warning,
            title: format!("Shallow Tread Run ({:.1} uu)", metrics.step_run),
            description: "Step treads under 16 uu are smaller than the standard player collision cylinder radius (17 uu), causing players to easily overshoot or slip.".to_string(),
            recommendation: "Increase step tread depth to at least 24 uu (recommended 32 uu).".to_string(),
        });
    }

    // 4. Curved / Spiral Inner Radius Pinch
    match config.archetype {
        StairArchetype::Curved90QuarterTurn
        | StairArchetype::Curved180HalfTurn
        | StairArchetype::SpiralTower => {
            if config.inner_radius < 34.0 {
                hazards.push(StairHazard {
                    severity: StairHazardSeverity::Warning,
                    title: format!("Tight Spiral Inner Radius ({:.1} uu)", config.inner_radius),
                    description: "An inner radius under 34 uu causes the inner step tread width to pinch smaller than the player collision radius (17 uu), jamming players against the inner column.".to_string(),
                    recommendation: "Set inner_radius to at least 64 uu (or 96 uu) to allow comfortable two-way passage.".to_string(),
                });
            }
        }
        _ => {}
    }

    // 5. Flight Width Check
    if config.step_width < 64.0 {
        hazards.push(StairHazard {
            severity: StairHazardSeverity::Warning,
            title: format!("Narrow Flight Width ({:.1} uu)", config.step_width),
            description: "Stairway width is too narrow for two combatants or bots to pass each other (UT99 standard doorway width is 128 uu).".to_string(),
            recommendation: "Increase step_width to at least 128 uu (recommended 192 uu for competitive arenas).".to_string(),
        });
    }

    // 6. Missing Smooth Collision Hull Warning
    if config.step_count >= 5 && !config.add_smooth_collision_hull {
        hazards.push(StairHazard {
            severity: StairHazardSeverity::Info,
            title: "No Smooth Collision Ramp Hull Configured".to_string(),
            description: "Competitive UT99 level design best practice places an invisible smooth collision wedge (PF_INVISIBLE | PF_SEMISOLID) over stairs to eliminate violent camera bobbing and step snagging.".to_string(),
            recommendation: "Enable 'add_smooth_collision_hull' to automatically generate an invisible SemiSolid collision wedge.".to_string(),
        });
    }

    hazards
}

/// Generates the visual stair step geometry brush
pub fn generate_stair_brush(config: &StairConfig) -> Brush {
    match config.archetype {
        StairArchetype::LinearStraight => generate_linear_stair_brush(config),
        StairArchetype::Curved90QuarterTurn
        | StairArchetype::Curved180HalfTurn
        | StairArchetype::SpiralTower => generate_curved_stair_brush(config),
        StairArchetype::SwitchbackLanding => generate_switchback_stair_brush(config),
    }
}

/// Helper to generate linear straight flight of steps
fn generate_linear_stair_brush(config: &StairConfig) -> Brush {
    let mut brush = Brush::new("Stair_Linear", CsgOperation::Active);
    let steps = config.step_count.max(1);
    let w = config.step_width;
    let l = config.step_length;
    let h = config.step_height;

    let half_w = w * 0.5;

    for i in 0..steps {
        let y0 = i as f32 * l;
        let y1 = (i + 1) as f32 * l;
        let z_top = (i + 1) as f32 * h;
        let z_bot = if config.has_subgrade_solid_fill {
            0.0
        } else {
            i as f32 * h
        };

        // 8 box vertices for this step block
        let p0 = Vec3::new(-half_w, y0, z_bot);
        let p1 = Vec3::new(half_w, y0, z_bot);
        let p2 = Vec3::new(half_w, y1, z_bot);
        let p3 = Vec3::new(-half_w, y1, z_bot);

        let p4 = Vec3::new(-half_w, y0, z_top);
        let p5 = Vec3::new(half_w, y0, z_top);
        let p6 = Vec3::new(half_w, y1, z_top);
        let p7 = Vec3::new(-half_w, y1, z_top);

        // Top tread (Z+)
        if let Some(mut poly) = Polygon::new(vec![p4, p7, p6, p5]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        // Front riser (Y-)
        if let Some(mut poly) = Polygon::new(vec![p0, p4, p5, p1]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        // Left side (X-)
        if let Some(mut poly) = Polygon::new(vec![p0, p3, p7, p4]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        // Right side (X+)
        if let Some(mut poly) = Polygon::new(vec![p1, p5, p6, p2]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        // Bottom (Z-)
        if let Some(mut poly) = Polygon::new(vec![p0, p1, p2, p3]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        // Back riser on last step or cantilever (Y+)
        if i == steps - 1 || !config.has_subgrade_solid_fill {
            if let Some(mut poly) = Polygon::new(vec![p2, p6, p7, p3]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
        }
    }

    brush
}

/// Helper to generate curved and spiral stairs
fn generate_curved_stair_brush(config: &StairConfig) -> Brush {
    let mut brush = Brush::new("Stair_Curved", CsgOperation::Active);
    let steps = config.step_count.max(1);
    let inner_r = config.inner_radius.max(8.0);
    let outer_r = inner_r + config.step_width;
    let total_rad = config.sweep_angle_deg.to_radians();
    let step_rad = total_rad / steps as f32;
    let dir = if config.counter_clockwise { 1.0 } else { -1.0 };

    for i in 0..steps {
        let a0 = dir * (i as f32 * step_rad);
        let a1 = dir * ((i + 1) as f32 * step_rad);

        let z_top = (i + 1) as f32 * config.step_height;
        let z_bot = if config.has_subgrade_solid_fill {
            0.0
        } else {
            i as f32 * config.step_height
        };

        let (sin0, cos0) = a0.sin_cos();
        let (sin1, cos1) = a1.sin_cos();

        let p_in_b0 = Vec3::new(inner_r * cos0, inner_r * sin0, z_bot);
        let p_out_b0 = Vec3::new(outer_r * cos0, outer_r * sin0, z_bot);
        let p_out_b1 = Vec3::new(outer_r * cos1, outer_r * sin1, z_bot);
        let p_in_b1 = Vec3::new(inner_r * cos1, inner_r * sin1, z_bot);

        let p_in_t0 = Vec3::new(inner_r * cos0, inner_r * sin0, z_top);
        let p_out_t0 = Vec3::new(outer_r * cos0, outer_r * sin0, z_top);
        let p_out_t1 = Vec3::new(outer_r * cos1, outer_r * sin1, z_top);
        let p_in_t1 = Vec3::new(inner_r * cos1, inner_r * sin1, z_top);

        if config.counter_clockwise {
            // Top tread
            if let Some(mut poly) = Polygon::new(vec![p_in_t0, p_out_t0, p_out_t1, p_in_t1]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
            // Front riser
            if let Some(mut poly) = Polygon::new(vec![p_in_b0, p_out_b0, p_out_t0, p_in_t0]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
            // Outer wall
            if let Some(mut poly) = Polygon::new(vec![p_out_b0, p_out_b1, p_out_t1, p_out_t0]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
            // Inner wall
            if let Some(mut poly) = Polygon::new(vec![p_in_b1, p_in_b0, p_in_t0, p_in_t1]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
            // Bottom
            if let Some(mut poly) = Polygon::new(vec![p_in_b1, p_out_b1, p_out_b0, p_in_b0]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
            // Back wall (last step or open soffit)
            if i == steps - 1 || !config.has_subgrade_solid_fill {
                if let Some(mut poly) = Polygon::new(vec![p_out_b1, p_in_b1, p_in_t1, p_out_t1]) {
                    poly.texture = config.texture_name.clone();
                    brush.polys.push(poly);
                }
            }
        } else {
            // Clockwise winding
            if let Some(mut poly) = Polygon::new(vec![p_in_t1, p_out_t1, p_out_t0, p_in_t0]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
            if let Some(mut poly) = Polygon::new(vec![p_in_t0, p_out_t0, p_out_b0, p_in_b0]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
            if let Some(mut poly) = Polygon::new(vec![p_out_t0, p_out_t1, p_out_b1, p_out_b0]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
            if let Some(mut poly) = Polygon::new(vec![p_in_t1, p_in_t0, p_in_b0, p_in_b1]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
            if let Some(mut poly) = Polygon::new(vec![p_in_b0, p_out_b0, p_out_b1, p_in_b1]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
            if i == steps - 1 || !config.has_subgrade_solid_fill {
                if let Some(mut poly) = Polygon::new(vec![p_out_t1, p_in_t1, p_in_b1, p_out_b1]) {
                    poly.texture = config.texture_name.clone();
                    brush.polys.push(poly);
                }
            }
        }
    }

    brush
}

/// Helper to generate switchback stairs with an intermediate resting landing
fn generate_switchback_stair_brush(config: &StairConfig) -> Brush {
    let mut brush = Brush::new("Stair_Switchback", CsgOperation::Active);
    let steps = config.step_count.max(2);
    let half_steps = steps / 2;
    let w = config.step_width;
    let l = config.step_length;
    let h = config.step_height;

    // Flight 1: Lower flight going Y+
    for i in 0..half_steps {
        let x0 = 0.0;
        let x1 = w;
        let y0 = i as f32 * l;
        let y1 = (i + 1) as f32 * l;
        let z_top = (i + 1) as f32 * h;
        let z_bot = 0.0;

        let p0 = Vec3::new(x0, y0, z_bot);
        let p1 = Vec3::new(x1, y0, z_bot);
        let p2 = Vec3::new(x1, y1, z_bot);
        let p3 = Vec3::new(x0, y1, z_bot);

        let p4 = Vec3::new(x0, y0, z_top);
        let p5 = Vec3::new(x1, y0, z_top);
        let p6 = Vec3::new(x1, y1, z_top);
        let p7 = Vec3::new(x0, y1, z_top);

        if let Some(mut poly) = Polygon::new(vec![p4, p7, p6, p5]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        if let Some(mut poly) = Polygon::new(vec![p0, p4, p5, p1]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        if let Some(mut poly) = Polygon::new(vec![p0, p3, p7, p4]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        if let Some(mut poly) = Polygon::new(vec![p1, p5, p6, p2]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        if let Some(mut poly) = Polygon::new(vec![p0, p1, p2, p3]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
    }

    // Intermediate Landing Platform at Y = half_steps * l
    let land_y0 = half_steps as f32 * l;
    let land_y1 = land_y0 + w; // square landing depth = flight width
    let land_z = half_steps as f32 * h;

    let lp0 = Vec3::new(-w, land_y0, 0.0);
    let lp1 = Vec3::new(w, land_y0, 0.0);
    let lp2 = Vec3::new(w, land_y1, 0.0);
    let lp3 = Vec3::new(-w, land_y1, 0.0);

    let lp4 = Vec3::new(-w, land_y0, land_z);
    let lp5 = Vec3::new(w, land_y0, land_z);
    let lp6 = Vec3::new(w, land_y1, land_z);
    let lp7 = Vec3::new(-w, land_y1, land_z);

    // Landing top
    if let Some(mut poly) = Polygon::new(vec![lp4, lp7, lp6, lp5]) {
        poly.texture = config.texture_name.clone();
        brush.polys.push(poly);
    }
    // Landing sides
    if let Some(mut poly) = Polygon::new(vec![lp0, lp4, lp7, lp3]) {
        poly.texture = config.texture_name.clone();
        brush.polys.push(poly);
    }
    if let Some(mut poly) = Polygon::new(vec![lp1, lp2, lp6, lp5]) {
        poly.texture = config.texture_name.clone();
        brush.polys.push(poly);
    }
    if let Some(mut poly) = Polygon::new(vec![lp2, lp3, lp7, lp6]) {
        poly.texture = config.texture_name.clone();
        brush.polys.push(poly);
    }
    if let Some(mut poly) = Polygon::new(vec![lp0, lp1, lp2, lp3]) {
        poly.texture = config.texture_name.clone();
        brush.polys.push(poly);
    }

    // Flight 2: Upper flight returning Y- along X in [-w, 0]
    for i in 0..half_steps {
        let x0 = -w;
        let x1 = 0.0;
        let y0 = land_y0 - (i as f32 * l);
        let y1 = land_y0 - ((i + 1) as f32 * l);
        let z_top = land_z + (i + 1) as f32 * h;

        let p0 = Vec3::new(x0, y0, land_z);
        let p1 = Vec3::new(x1, y0, land_z);
        let p2 = Vec3::new(x1, y1, land_z);
        let p3 = Vec3::new(x0, y1, land_z);

        let p4 = Vec3::new(x0, y0, z_top);
        let p5 = Vec3::new(x1, y0, z_top);
        let p6 = Vec3::new(x1, y1, z_top);
        let p7 = Vec3::new(x0, y1, z_top);

        // Top tread
        if let Some(mut poly) = Polygon::new(vec![p4, p5, p6, p7]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        // Front riser (in negative Y direction)
        if let Some(mut poly) = Polygon::new(vec![p2, p6, p7, p3]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        // Side walls
        if let Some(mut poly) = Polygon::new(vec![p0, p4, p7, p3]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
        if let Some(mut poly) = Polygon::new(vec![p1, p2, p6, p5]) {
            poly.texture = config.texture_name.clone();
            brush.polys.push(poly);
        }
    }

    brush
}

/// Generates a smooth collision wedge / ramp hull matching the stairway dimensions.
/// The polygons are flagged with `PF_INVISIBLE | PF_SEMISOLID` (or custom setting).
pub fn generate_smooth_collision_ramp_brush(config: &StairConfig) -> Brush {
    let mut brush = Brush::new("Stair_CollisionHull", CsgOperation::Active);
    let flags = config.collision_hull_solidity.poly_flags();

    match config.archetype {
        StairArchetype::LinearStraight => {
            let steps = config.step_count.max(1);
            let half_w = config.step_width * 0.5;
            let total_l = steps as f32 * config.step_length;
            let total_h = steps as f32 * config.step_height;

            // Triangular prism wedge spanning (0, 0, 0) to (0, total_l, total_h)
            let p0 = Vec3::new(-half_w, 0.0, 0.0);
            let p1 = Vec3::new(half_w, 0.0, 0.0);
            let p2 = Vec3::new(half_w, total_l, 0.0);
            let p3 = Vec3::new(-half_w, total_l, 0.0);

            let p4 = Vec3::new(-half_w, total_l, total_h);
            let p5 = Vec3::new(half_w, total_l, total_h);

            // Sloped ramp top (p0, p4, p5, p1)
            if let Some(mut poly) = Polygon::new(vec![p0, p4, p5, p1]) {
                poly.flags = flags;
                brush.polys.push(poly);
            }
            // Vertical back wall (p3, p2, p5, p4)
            if let Some(mut poly) = Polygon::new(vec![p3, p2, p5, p4]) {
                poly.flags = flags;
                brush.polys.push(poly);
            }
            // Horizontal floor base (p0, p1, p2, p3)
            if let Some(mut poly) = Polygon::new(vec![p0, p1, p2, p3]) {
                poly.flags = flags;
                brush.polys.push(poly);
            }
            // Left triangle (p0, p3, p4)
            if let Some(mut poly) = Polygon::new(vec![p0, p3, p4]) {
                poly.flags = flags;
                brush.polys.push(poly);
            }
            // Right triangle (p1, p5, p2)
            if let Some(mut poly) = Polygon::new(vec![p1, p5, p2]) {
                poly.flags = flags;
                brush.polys.push(poly);
            }
        }

        StairArchetype::Curved90QuarterTurn
        | StairArchetype::Curved180HalfTurn
        | StairArchetype::SpiralTower => {
            let steps = config.step_count.max(1);
            let inner_r = config.inner_radius.max(8.0);
            let outer_r = inner_r + config.step_width;
            let total_rad = config.sweep_angle_deg.to_radians();
            let step_rad = total_rad / steps as f32;
            let dir = if config.counter_clockwise { 1.0 } else { -1.0 };

            // Generate smooth helical ramp strip connecting the lip of each step
            for i in 0..steps {
                let a0 = dir * (i as f32 * step_rad);
                let a1 = dir * ((i + 1) as f32 * step_rad);

                let z0 = i as f32 * config.step_height;
                let z1 = (i + 1) as f32 * config.step_height;

                let (sin0, cos0) = a0.sin_cos();
                let (sin1, cos1) = a1.sin_cos();

                let p_in_b0 = Vec3::new(inner_r * cos0, inner_r * sin0, 0.0);
                let p_out_b0 = Vec3::new(outer_r * cos0, outer_r * sin0, 0.0);
                let p_out_b1 = Vec3::new(outer_r * cos1, outer_r * sin1, 0.0);
                let p_in_b1 = Vec3::new(inner_r * cos1, inner_r * sin1, 0.0);

                let p_in_t0 = Vec3::new(inner_r * cos0, inner_r * sin0, z0);
                let p_out_t0 = Vec3::new(outer_r * cos0, outer_r * sin0, z0);
                let p_out_t1 = Vec3::new(outer_r * cos1, outer_r * sin1, z1);
                let p_in_t1 = Vec3::new(inner_r * cos1, inner_r * sin1, z1);

                if config.counter_clockwise {
                    // Sloped helical top ramp surface
                    if let Some(mut poly) = Polygon::new(vec![p_in_t0, p_out_t0, p_out_t1, p_in_t1]) {
                        poly.flags = flags;
                        brush.polys.push(poly);
                    }
                    // Outer curved wall
                    if let Some(mut poly) = Polygon::new(vec![p_out_b0, p_out_b1, p_out_t1, p_out_t0]) {
                        poly.flags = flags;
                        brush.polys.push(poly);
                    }
                    // Inner curved wall
                    if let Some(mut poly) = Polygon::new(vec![p_in_b1, p_in_b0, p_in_t0, p_in_t1]) {
                        poly.flags = flags;
                        brush.polys.push(poly);
                    }
                    // Base
                    if let Some(mut poly) = Polygon::new(vec![p_in_b1, p_out_b1, p_out_b0, p_in_b0]) {
                        poly.flags = flags;
                        brush.polys.push(poly);
                    }
                    // Start vertical face
                    if i == 0 {
                        if let Some(mut poly) = Polygon::new(vec![p_in_b0, p_out_b0, p_out_t0, p_in_t0]) {
                            poly.flags = flags;
                            brush.polys.push(poly);
                        }
                    }
                    // End vertical face
                    if i == steps - 1 {
                        if let Some(mut poly) = Polygon::new(vec![p_out_b1, p_in_b1, p_in_t1, p_out_t1]) {
                            poly.flags = flags;
                            brush.polys.push(poly);
                        }
                    }
                } else {
                    // Clockwise
                    if let Some(mut poly) = Polygon::new(vec![p_in_t1, p_out_t1, p_out_t0, p_in_t0]) {
                        poly.flags = flags;
                        brush.polys.push(poly);
                    }
                    if let Some(mut poly) = Polygon::new(vec![p_out_t0, p_out_t1, p_out_b1, p_out_b0]) {
                        poly.flags = flags;
                        brush.polys.push(poly);
                    }
                    if let Some(mut poly) = Polygon::new(vec![p_in_t1, p_in_t0, p_in_b0, p_in_b1]) {
                        poly.flags = flags;
                        brush.polys.push(poly);
                    }
                    if let Some(mut poly) = Polygon::new(vec![p_in_b0, p_out_b0, p_out_b1, p_in_b1]) {
                        poly.flags = flags;
                        brush.polys.push(poly);
                    }
                    if i == 0 {
                        if let Some(mut poly) = Polygon::new(vec![p_in_t0, p_out_t0, p_out_b0, p_in_b0]) {
                            poly.flags = flags;
                            brush.polys.push(poly);
                        }
                    }
                    if i == steps - 1 {
                        if let Some(mut poly) = Polygon::new(vec![p_out_t1, p_in_t1, p_in_b1, p_out_b1]) {
                            poly.flags = flags;
                            brush.polys.push(poly);
                        }
                    }
                }
            }
        }

        StairArchetype::SwitchbackLanding => {
            let steps = config.step_count.max(2);
            let half_steps = steps / 2;
            let w = config.step_width;
            let l = config.step_length;
            let h = config.step_height;

            let total_flight_l = half_steps as f32 * l;
            let half_flight_h = half_steps as f32 * h;

            // Ramp 1: Lower flight
            let p0 = Vec3::new(0.0, 0.0, 0.0);
            let p1 = Vec3::new(w, 0.0, 0.0);
            let p2 = Vec3::new(w, total_flight_l, half_flight_h);
            let p3 = Vec3::new(0.0, total_flight_l, half_flight_h);

            if let Some(mut poly) = Polygon::new(vec![p0, p3, p2, p1]) {
                poly.flags = flags;
                brush.polys.push(poly);
            }
            if let Some(mut poly) = Polygon::new(vec![p0, p1, Vec3::new(w, total_flight_l, 0.0), Vec3::new(0.0, total_flight_l, 0.0)]) {
                poly.flags = flags;
                brush.polys.push(poly);
            }

            // Landing slab
            let lp0 = Vec3::new(-w, total_flight_l, half_flight_h);
            let lp1 = Vec3::new(w, total_flight_l, half_flight_h);
            let lp2 = Vec3::new(w, total_flight_l + w, half_flight_h);
            let lp3 = Vec3::new(-w, total_flight_l + w, half_flight_h);

            if let Some(mut poly) = Polygon::new(vec![lp0, lp3, lp2, lp1]) {
                poly.flags = flags;
                brush.polys.push(poly);
            }

            // Ramp 2: Upper flight returning
            let top_h = steps as f32 * h;
            let up0 = Vec3::new(-w, total_flight_l, half_flight_h);
            let up1 = Vec3::new(0.0, total_flight_l, half_flight_h);
            let up2 = Vec3::new(0.0, 0.0, top_h);
            let up3 = Vec3::new(-w, 0.0, top_h);

            if let Some(mut poly) = Polygon::new(vec![up0, up1, up2, up3]) {
                poly.flags = flags;
                brush.polys.push(poly);
            }
        }
    }

    brush
}

/// Generates architectural banister/handrail brush along outer staircase perimeter
pub fn generate_handrail_brush(config: &StairConfig) -> Brush {
    let mut brush = Brush::new("Stair_Handrail", CsgOperation::Active);
    let rail_h = 40.0; // handrail height (waist level)
    let rail_thick = 8.0;

    match config.archetype {
        StairArchetype::LinearStraight => {
            let steps = config.step_count.max(1);
            let half_w = config.step_width * 0.5;
            let total_l = steps as f32 * config.step_length;
            let total_h = steps as f32 * config.step_height;

            let x0 = half_w - rail_thick;
            let x1 = half_w;

            let p0 = Vec3::new(x0, 0.0, rail_h);
            let p1 = Vec3::new(x1, 0.0, rail_h);
            let p2 = Vec3::new(x1, total_l, total_h + rail_h);
            let p3 = Vec3::new(x0, total_l, total_h + rail_h);

            if let Some(mut poly) = Polygon::new(vec![p0, p3, p2, p1]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
            if let Some(mut poly) = Polygon::new(vec![p1, p2, Vec3::new(x1, total_l, 0.0), Vec3::new(x1, 0.0, 0.0)]) {
                poly.texture = config.texture_name.clone();
                brush.polys.push(poly);
            }
        }
        StairArchetype::Curved90QuarterTurn
        | StairArchetype::Curved180HalfTurn
        | StairArchetype::SpiralTower => {
            let steps = config.step_count.max(1);
            let outer_r = config.inner_radius + config.step_width;
            let total_rad = config.sweep_angle_deg.to_radians();
            let step_rad = total_rad / steps as f32;
            let dir = if config.counter_clockwise { 1.0 } else { -1.0 };

            for i in 0..steps {
                let a0 = dir * (i as f32 * step_rad);
                let a1 = dir * ((i + 1) as f32 * step_rad);

                let z0 = (i as f32 * config.step_height) + rail_h;
                let z1 = ((i + 1) as f32 * config.step_height) + rail_h;

                let (sin0, cos0) = a0.sin_cos();
                let (sin1, cos1) = a1.sin_cos();

                let p0 = Vec3::new((outer_r - rail_thick) * cos0, (outer_r - rail_thick) * sin0, z0);
                let p1 = Vec3::new(outer_r * cos0, outer_r * sin0, z0);
                let p2 = Vec3::new(outer_r * cos1, outer_r * sin1, z1);
                let p3 = Vec3::new((outer_r - rail_thick) * cos1, (outer_r - rail_thick) * sin1, z1);

                if let Some(mut poly) = Polygon::new(vec![p0, p3, p2, p1]) {
                    poly.texture = config.texture_name.clone();
                    brush.polys.push(poly);
                }
            }
        }
        StairArchetype::SwitchbackLanding => {
            // Handrail along outer edges
        }
    }

    brush
}

/// Generates bot navigation path waypoints along the staircase centerline
pub fn generate_bot_path_nodes(config: &StairConfig, base_pos: Vec3) -> Vec<StairBotWaypoint> {
    let mut waypoints = Vec::new();
    let steps = config.step_count.max(1);
    let is_valid = config.step_height <= 25.0; // ReachSpec traversability rule

    let eye_offset = Vec3::new(0.0, 0.0, 40.0);

    match config.archetype {
        StairArchetype::LinearStraight => {
            let total_l = steps as f32 * config.step_length;
            let total_h = steps as f32 * config.step_height;

            // Entry waypoint (base)
            waypoints.push(StairBotWaypoint {
                position: base_pos + Vec3::new(0.0, -32.0, 0.0) + eye_offset,
                name: "PathNode_StairBase".to_string(),
                is_landing: false,
                reachspec_valid: is_valid,
            });

            // Intermediate midpoint
            waypoints.push(StairBotWaypoint {
                position: base_pos + Vec3::new(0.0, total_l * 0.5, total_h * 0.5) + eye_offset,
                name: "PathNode_StairMid".to_string(),
                is_landing: false,
                reachspec_valid: is_valid,
            });

            // Exit waypoint (top)
            waypoints.push(StairBotWaypoint {
                position: base_pos + Vec3::new(0.0, total_l + 32.0, total_h) + eye_offset,
                name: "PathNode_StairTop".to_string(),
                is_landing: false,
                reachspec_valid: is_valid,
            });
        }

        StairArchetype::Curved90QuarterTurn
        | StairArchetype::Curved180HalfTurn
        | StairArchetype::SpiralTower => {
            let mid_r = config.inner_radius + config.step_width * 0.5;
            let total_rad = config.sweep_angle_deg.to_radians();
            let dir = if config.counter_clockwise { 1.0 } else { -1.0 };

            // Entry
            let a_entry: f32 = 0.0;
            let p_entry = Vec3::new(mid_r * a_entry.cos(), mid_r * a_entry.sin(), 0.0);
            waypoints.push(StairBotWaypoint {
                position: base_pos + p_entry + eye_offset,
                name: "PathNode_CurvedBase".to_string(),
                is_landing: false,
                reachspec_valid: is_valid,
            });

            // Mid-stations (every ~90 degrees or half-flight)
            let mid_count = (config.sweep_angle_deg / 90.0).ceil() as usize;
            for s in 1..=mid_count {
                let frac = s as f32 / (mid_count + 1) as f32;
                let a_mid = dir * (frac * total_rad);
                let z_mid = frac * (steps as f32 * config.step_height);
                let p_mid = Vec3::new(mid_r * a_mid.cos(), mid_r * a_mid.sin(), z_mid);
                waypoints.push(StairBotWaypoint {
                    position: base_pos + p_mid + eye_offset,
                    name: format!("PathNode_CurvedMid_{}", s),
                    is_landing: false,
                    reachspec_valid: is_valid,
                });
            }

            // Exit
            let a_exit = dir * total_rad;
            let z_exit = steps as f32 * config.step_height;
            let p_exit = Vec3::new(mid_r * a_exit.cos(), mid_r * a_exit.sin(), z_exit);
            waypoints.push(StairBotWaypoint {
                position: base_pos + p_exit + eye_offset,
                name: "PathNode_CurvedTop".to_string(),
                is_landing: false,
                reachspec_valid: is_valid,
            });
        }

        StairArchetype::SwitchbackLanding => {
            let half_steps = steps / 2;
            let w = config.step_width;
            let l = config.step_length;
            let h = config.step_height;

            // Lower flight entry
            waypoints.push(StairBotWaypoint {
                position: base_pos + Vec3::new(w * 0.5, -32.0, 0.0) + eye_offset,
                name: "PathNode_SwitchbackBase".to_string(),
                is_landing: false,
                reachspec_valid: is_valid,
            });

            // Intermediate Landing waypoint
            let land_y = (half_steps as f32 * l) + (w * 0.5);
            let land_z = half_steps as f32 * h;
            waypoints.push(StairBotWaypoint {
                position: base_pos + Vec3::new(0.0, land_y, land_z) + eye_offset,
                name: "PathNode_SwitchbackLanding".to_string(),
                is_landing: true,
                reachspec_valid: is_valid,
            });

            // Upper flight exit
            let top_z = steps as f32 * h;
            waypoints.push(StairBotWaypoint {
                position: base_pos + Vec3::new(-w * 0.5, -32.0, top_z) + eye_offset,
                name: "PathNode_SwitchbackTop".to_string(),
                is_landing: false,
                reachspec_valid: is_valid,
            });
        }
    }

    waypoints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stair_archetypes_and_defaults() {
        for &arch in StairArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.step_count > 0);
            assert!(cfg.step_height > 0.0);
            assert!(cfg.step_width > 0.0);
            let metrics = calculate_stair_metrics(&cfg);
            assert!(metrics.total_height > 0.0);
            assert!(metrics.slope_pitch_degrees > 0.0 && metrics.slope_pitch_degrees < 90.0);
        }
    }

    #[test]
    fn test_linear_stair_and_smooth_collision_wedge_generation() {
        let mut cfg = StairConfig::default();
        cfg.step_count = 8;
        cfg.step_height = 16.0;
        cfg.step_length = 32.0;
        cfg.step_width = 128.0;

        let stair_brush = generate_stair_brush(&cfg);
        assert!(!stair_brush.polys.is_empty());

        let ramp_brush = generate_smooth_collision_ramp_brush(&cfg);
        assert_eq!(ramp_brush.polys.len(), 5); // 5 faces of a triangular wedge prism
        for poly in &ramp_brush.polys {
            assert_ne!(poly.flags & PF_INVISIBLE, 0, "Collision ramp must be invisible");
            assert_ne!(poly.flags & PF_SEMISOLID, 0, "Default ramp must be SemiSolid");
        }
    }

    #[test]
    fn test_curved_and_spiral_stairs_generation() {
        let spiral_cfg = StairArchetype::SpiralTower.default_config();
        let spiral_brush = generate_stair_brush(&spiral_cfg);
        assert!(!spiral_brush.polys.is_empty());

        let ramp_brush = generate_smooth_collision_ramp_brush(&spiral_cfg);
        assert!(!ramp_brush.polys.is_empty());
        for poly in &ramp_brush.polys {
            assert_ne!(poly.flags & PF_INVISIBLE, 0);
        }
    }

    #[test]
    fn test_safety_audit_detects_max_step_height_and_steep_slope() {
        // Safe configuration
        let safe_cfg = StairConfig {
            step_height: 16.0,
            step_length: 32.0,
            step_width: 192.0,
            ..Default::default()
        };
        let hazards = audit_stair_safety(&safe_cfg);
        let critical_count = hazards.iter().filter(|h| h.severity == StairHazardSeverity::Critical).count();
        assert_eq!(critical_count, 0);

        // Violating configuration: step_height = 32 uu (> 25 uu MaxStepHeight)
        let unsafe_cfg = StairConfig {
            step_height: 32.0,
            step_length: 16.0,
            step_width: 48.0,
            ..Default::default()
        };
        let hazards = audit_stair_safety(&unsafe_cfg);
        let has_max_step = hazards.iter().any(|h| h.severity == StairHazardSeverity::Critical && h.title.contains("MaxStepHeight"));
        assert!(has_max_step, "Audit must catch step risers exceeding UT99 MaxStepHeight");
    }

    #[test]
    fn test_bot_waypoint_path_generation() {
        let cfg = StairConfig::default();
        let waypoints = generate_bot_path_nodes(&cfg, Vec3::ZERO);
        assert!(waypoints.len() >= 3, "Linear stair should have base, mid, and top waypoints");
        assert!(waypoints.iter().all(|wp| wp.reachspec_valid));
    }
}
