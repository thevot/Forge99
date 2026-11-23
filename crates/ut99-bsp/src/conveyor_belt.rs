// crates/ut99-bsp/src/conveyor_belt.rs
//
// Interactive Industrial Conveyor Belt, Treadmill & Surface Velocity Studio
//
// Mathematical, physical, and geometric model for industrial conveyor belts,
// synchronized texture panning and ZoneVelocity propulsion, end rollers,
// guardrails, bot navigational waypoints, and incline slip hazard auditing.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Classical Unreal Tournament & Unreal 1 conveyor belt archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConveyorArchetype {
    MiningOreConveyor,
    HighSpeedPackageChute,
    SubterraneanSludgeTreadmill,
    AscendingInclineBelt,
    ReversibleSortingTurntable,
}

impl ConveyorArchetype {
    pub fn all() -> &'static [ConveyorArchetype] {
        &[
            ConveyorArchetype::MiningOreConveyor,
            ConveyorArchetype::HighSpeedPackageChute,
            ConveyorArchetype::SubterraneanSludgeTreadmill,
            ConveyorArchetype::AscendingInclineBelt,
            ConveyorArchetype::ReversibleSortingTurntable,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ConveyorArchetype::MiningOreConveyor => "Mining Heavy Ore Conveyor (256 uu/s)",
            ConveyorArchetype::HighSpeedPackageChute => "High-Speed Factory Package Chute (512 uu/s)",
            ConveyorArchetype::SubterraneanSludgeTreadmill => "Subterranean Sludge Treadmill (128 uu/s)",
            ConveyorArchetype::AscendingInclineBelt => "Ascending Catwalk Incline Belt (20° Rise)",
            ConveyorArchetype::ReversibleSortingTurntable => "Reversible Industrial Sorting Treadmill",
        }
    }

    pub fn default_config(&self) -> ConveyorConfig {
        match self {
            ConveyorArchetype::MiningOreConveyor => ConveyorConfig {
                archetype: *self,
                position: Vec3::ZERO,
                forward_axis: Vec3::X,
                deck: ConveyorDeckConfig {
                    length: 512.0,
                    width: 128.0,
                    thickness: 16.0,
                    incline_angle_deg: 0.0,
                },
                kinematics: ConveyorKinematicsConfig {
                    speed_uu_per_sec: 256.0,
                    direction: ConveyorDirection::Forward,
                    is_reversible: false,
                },
                rollers: ConveyorRollerConfig {
                    has_end_rollers: true,
                    roller_radius: 12.0,
                    roller_subdivisions: 8,
                },
                guardrails: ConveyorGuardrailConfig {
                    has_guardrails: true,
                    rail_height: 24.0,
                    rail_width: 8.0,
                    has_support_legs: true,
                    leg_spacing: 128.0,
                },
                bot_nav: ConveyorBotNavConfig {
                    generate_bot_waypoints: true,
                    waypoint_offset: 32.0,
                },
                belt_texture: "Indus.Metal.TreadRubber01".to_string(),
                frame_texture: "Indus.Metal.ConveyorFrame01".to_string(),
                ambient_sound: "GenAmb.Conveyor1".to_string(),
            },
            ConveyorArchetype::HighSpeedPackageChute => ConveyorConfig {
                archetype: *self,
                position: Vec3::ZERO,
                forward_axis: Vec3::X,
                deck: ConveyorDeckConfig {
                    length: 768.0,
                    width: 96.0,
                    thickness: 12.0,
                    incline_angle_deg: 0.0,
                },
                kinematics: ConveyorKinematicsConfig {
                    speed_uu_per_sec: 512.0,
                    direction: ConveyorDirection::Forward,
                    is_reversible: false,
                },
                rollers: ConveyorRollerConfig {
                    has_end_rollers: true,
                    roller_radius: 10.0,
                    roller_subdivisions: 8,
                },
                guardrails: ConveyorGuardrailConfig {
                    has_guardrails: true,
                    rail_height: 32.0,
                    rail_width: 6.0,
                    has_support_legs: true,
                    leg_spacing: 192.0,
                },
                bot_nav: ConveyorBotNavConfig {
                    generate_bot_waypoints: true,
                    waypoint_offset: 40.0,
                },
                belt_texture: "Indus.Metal.FastPlastic01".to_string(),
                frame_texture: "Indus.Metal.SteelChrome01".to_string(),
                ambient_sound: "AmbModern.Motor1".to_string(),
            },
            ConveyorArchetype::SubterraneanSludgeTreadmill => ConveyorConfig {
                archetype: *self,
                position: Vec3::ZERO,
                forward_axis: Vec3::Y,
                deck: ConveyorDeckConfig {
                    length: 640.0,
                    width: 160.0,
                    thickness: 20.0,
                    incline_angle_deg: 0.0,
                },
                kinematics: ConveyorKinematicsConfig {
                    speed_uu_per_sec: 128.0,
                    direction: ConveyorDirection::Forward,
                    is_reversible: false,
                },
                rollers: ConveyorRollerConfig {
                    has_end_rollers: true,
                    roller_radius: 16.0,
                    roller_subdivisions: 8,
                },
                guardrails: ConveyorGuardrailConfig {
                    has_guardrails: true,
                    rail_height: 20.0,
                    rail_width: 10.0,
                    has_support_legs: false,
                    leg_spacing: 160.0,
                },
                bot_nav: ConveyorBotNavConfig {
                    generate_bot_waypoints: true,
                    waypoint_offset: 32.0,
                },
                belt_texture: "Indus.Metal.RustyGrate01".to_string(),
                frame_texture: "Indus.Metal.SludgeFrame01".to_string(),
                ambient_sound: "GenAmb.CreakSqueak01".to_string(),
            },
            ConveyorArchetype::AscendingInclineBelt => ConveyorConfig {
                archetype: *self,
                position: Vec3::ZERO,
                forward_axis: Vec3::X,
                deck: ConveyorDeckConfig {
                    length: 640.0,
                    width: 112.0,
                    thickness: 16.0,
                    incline_angle_deg: 20.0, // 20° climb
                },
                kinematics: ConveyorKinematicsConfig {
                    speed_uu_per_sec: 320.0,
                    direction: ConveyorDirection::Forward,
                    is_reversible: false,
                },
                rollers: ConveyorRollerConfig {
                    has_end_rollers: true,
                    roller_radius: 14.0,
                    roller_subdivisions: 8,
                },
                guardrails: ConveyorGuardrailConfig {
                    has_guardrails: true,
                    rail_height: 36.0,
                    rail_width: 8.0,
                    has_support_legs: true,
                    leg_spacing: 128.0,
                },
                bot_nav: ConveyorBotNavConfig {
                    generate_bot_waypoints: true,
                    waypoint_offset: 36.0,
                },
                belt_texture: "Indus.Metal.CleatedRubber01".to_string(),
                frame_texture: "Indus.Metal.CatwalkTruss01".to_string(),
                ambient_sound: "GenAmb.BeltClimb01".to_string(),
            },
            ConveyorArchetype::ReversibleSortingTurntable => ConveyorConfig {
                archetype: *self,
                position: Vec3::ZERO,
                forward_axis: Vec3::X,
                deck: ConveyorDeckConfig {
                    length: 384.0,
                    width: 128.0,
                    thickness: 14.0,
                    incline_angle_deg: 0.0,
                },
                kinematics: ConveyorKinematicsConfig {
                    speed_uu_per_sec: 200.0,
                    direction: ConveyorDirection::Forward,
                    is_reversible: true,
                },
                rollers: ConveyorRollerConfig {
                    has_end_rollers: true,
                    roller_radius: 12.0,
                    roller_subdivisions: 8,
                },
                guardrails: ConveyorGuardrailConfig {
                    has_guardrails: true,
                    rail_height: 18.0,
                    rail_width: 6.0,
                    has_support_legs: true,
                    leg_spacing: 128.0,
                },
                bot_nav: ConveyorBotNavConfig {
                    generate_bot_waypoints: true,
                    waypoint_offset: 28.0,
                },
                belt_texture: "Indus.Metal.SortingPlate01".to_string(),
                frame_texture: "Indus.Metal.YellowHazardFrame01".to_string(),
                ambient_sound: "GenAmb.SortingMotor01".to_string(),
            },
        }
    }
}

/// Conveyor operational direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConveyorDirection {
    Forward,
    Backward,
    Stopped,
}

impl ConveyorDirection {
    pub fn all() -> &'static [ConveyorDirection] {
        &[
            ConveyorDirection::Forward,
            ConveyorDirection::Backward,
            ConveyorDirection::Stopped,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ConveyorDirection::Forward => "Forward (Intake -> Discharge)",
            ConveyorDirection::Backward => "Backward (Discharge -> Intake)",
            ConveyorDirection::Stopped => "Stopped (Zero Motion)",
        }
    }
}

/// Deck dimensions and incline angle
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConveyorDeckConfig {
    pub length: f32,
    pub width: f32,
    pub thickness: f32,
    pub incline_angle_deg: f32,
}

/// Physical surface velocity and reversibility
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConveyorKinematicsConfig {
    pub speed_uu_per_sec: f32,
    pub direction: ConveyorDirection,
    pub is_reversible: bool,
}

/// End roller cylinders at intake and discharge
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConveyorRollerConfig {
    pub has_end_rollers: bool,
    pub roller_radius: f32,
    pub roller_subdivisions: usize,
}

/// Guardrails and support trusses
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConveyorGuardrailConfig {
    pub has_guardrails: bool,
    pub rail_height: f32,
    pub rail_width: f32,
    pub has_support_legs: bool,
    pub leg_spacing: f32,
}

/// Bot navigation waypoints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConveyorBotNavConfig {
    pub generate_bot_waypoints: bool,
    pub waypoint_offset: f32,
}

/// Complete configuration for Industrial Conveyor Belt Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConveyorConfig {
    pub archetype: ConveyorArchetype,
    pub position: Vec3,
    pub forward_axis: Vec3,
    pub deck: ConveyorDeckConfig,
    pub kinematics: ConveyorKinematicsConfig,
    pub rollers: ConveyorRollerConfig,
    pub guardrails: ConveyorGuardrailConfig,
    pub bot_nav: ConveyorBotNavConfig,
    pub belt_texture: String,
    pub frame_texture: String,
    pub ambient_sound: String,
}

impl Default for ConveyorConfig {
    fn default() -> Self {
        ConveyorArchetype::MiningOreConveyor.default_config()
    }
}

/// Metrics for synchronizing physical motion with texture panning and roller rotation
#[derive(Debug, Clone, PartialEq)]
pub struct ConveyorSyncMetrics {
    pub linear_velocity: Vec3,
    pub texture_pan_rate_pixels_sec: f32,
    pub roller_angular_velocity_rad_sec: f32,
    pub roller_rpm: f32,
    pub incline_rise_height: f32,
}

/// Waypoint specification for bot AI intake and discharge
#[derive(Debug, Clone, PartialEq)]
pub struct ConveyorBotWaypointSpec {
    pub name: String,
    pub location: Vec3,
    pub is_intake: bool,
    pub velocity_bias: Vec3,
}

/// Safety diagnostic hazard for conveyor belt systems
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConveyorHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Computes synchronized physics velocity, texture panning rate, and roller RPM
pub fn calculate_conveyor_sync_metrics(config: &ConveyorConfig) -> ConveyorSyncMetrics {
    let dir_sign = match config.kinematics.direction {
        ConveyorDirection::Forward => 1.0,
        ConveyorDirection::Backward => -1.0,
        ConveyorDirection::Stopped => 0.0,
    };

    let fwd = config.forward_axis.normalize_or_zero();
    let incline_rad = config.deck.incline_angle_deg.to_radians();

    // Direction vector incorporating incline pitch
    let pitch_quat = Quat::from_axis_angle(Vec3::Z.cross(fwd).normalize_or_zero(), incline_rad);
    let motion_dir = (pitch_quat * fwd).normalize_or_zero();

    let speed = config.kinematics.speed_uu_per_sec * dir_sign;
    let lin_vel = motion_dir * speed;

    // UE1 standard texture scale is typically 1.0 uu per pixel at 256x256
    let tex_pan_rate = speed;

    // Angular velocity: omega = v / R
    let roller_r = config.rollers.roller_radius.max(1.0);
    let omega = speed / roller_r;
    let rpm = (omega * 60.0) / std::f32::consts::TAU;

    let rise = config.deck.length * incline_rad.sin();

    ConveyorSyncMetrics {
        linear_velocity: lin_vel,
        texture_pan_rate_pixels_sec: tex_pan_rate,
        roller_angular_velocity_rad_sec: omega,
        roller_rpm: rpm,
        incline_rise_height: rise,
    }
}

/// Generates intake and discharge navigation waypoints for bot AI
pub fn generate_conveyor_bot_waypoints(
    config: &ConveyorConfig,
) -> (ConveyorBotWaypointSpec, ConveyorBotWaypointSpec) {
    let fwd = config.forward_axis.normalize_or_zero();
    let incline_rad = config.deck.incline_angle_deg.to_radians();
    let pitch_quat = Quat::from_axis_angle(Vec3::Z.cross(fwd).normalize_or_zero(), incline_rad);
    let motion_dir = (pitch_quat * fwd).normalize_or_zero();

    let half_l = config.deck.length * 0.5;
    let offset = config.bot_nav.waypoint_offset;
    let z_clear = config.deck.thickness * 0.5 + 32.0;

    let intake_pos = config.position - motion_dir * (half_l + offset) + Vec3::new(0.0, 0.0, z_clear);
    let discharge_pos = config.position + motion_dir * (half_l + offset) + Vec3::new(0.0, 0.0, z_clear);

    let speed = config.kinematics.speed_uu_per_sec;
    let intake_spec = ConveyorBotWaypointSpec {
        name: "ConveyorIntake_PathNode".to_string(),
        location: intake_pos,
        is_intake: true,
        velocity_bias: motion_dir * speed,
    };

    let discharge_spec = ConveyorBotWaypointSpec {
        name: "ConveyorDischarge_PathNode".to_string(),
        location: discharge_pos,
        is_intake: false,
        velocity_bias: motion_dir * speed,
    };

    (intake_spec, discharge_spec)
}

/// Generates procedural 3D polygonal geometry:
/// 1. Main Conveyor Deck Brush (carries belt texture and ZoneVelocity)
/// 2. End Roller Cylinders Brush (two multi-sided cylinders)
/// 3. Guardrails & Support Truss Frame Brush (`PF_SEMISOLID`)
pub fn generate_conveyor_brushes(
    config: &ConveyorConfig,
) -> (Brush, Brush, Brush) {
    let mut deck_brush = Brush::new("Conveyor_Deck", CsgOperation::Add);
    let mut rollers_brush = Brush::new("Conveyor_Rollers", CsgOperation::Add);
    let mut frame_brush = Brush::new("Conveyor_Frame", CsgOperation::Add);

    let fwd = config.forward_axis.normalize_or_zero();
    let right = fwd.cross(Vec3::Z).normalize_or_zero();
    let up = Vec3::Z;

    let incline_rad = config.deck.incline_angle_deg.to_radians();
    let pitch_quat = Quat::from_axis_angle(right, incline_rad);

    let fwd_inclined = (pitch_quat * fwd).normalize_or_zero();
    let up_inclined = (pitch_quat * up).normalize_or_zero();

    let half_l = config.deck.length * 0.5;
    let half_w = config.deck.width * 0.5;
    let half_t = config.deck.thickness * 0.5;

    // 1. Generate Main Deck Box
    let corners = [
        config.position - fwd_inclined * half_l - right * half_w - up_inclined * half_t, // 0: rear left bot
        config.position + fwd_inclined * half_l - right * half_w - up_inclined * half_t, // 1: front left bot
        config.position + fwd_inclined * half_l + right * half_w - up_inclined * half_t, // 2: front right bot
        config.position - fwd_inclined * half_l + right * half_w - up_inclined * half_t, // 3: rear right bot
        config.position - fwd_inclined * half_l - right * half_w + up_inclined * half_t, // 4: rear left top
        config.position + fwd_inclined * half_l - right * half_w + up_inclined * half_t, // 5: front left top
        config.position + fwd_inclined * half_l + right * half_w + up_inclined * half_t, // 6: front right top
        config.position - fwd_inclined * half_l + right * half_w + up_inclined * half_t, // 7: rear right top
    ];

    let faces = [
        ([4, 5, 6, 7], true),  // Top (conveyor belt surface)
        ([3, 2, 1, 0], false), // Bottom
        ([0, 1, 5, 4], false), // Left side
        ([2, 3, 7, 6], false), // Right side
        ([1, 2, 6, 5], false), // Front
        ([3, 0, 4, 7], false), // Back
    ];

    for (f_indices, is_top) in faces {
        let pts = vec![corners[f_indices[0]], corners[f_indices[1]], corners[f_indices[2]], corners[f_indices[3]]];
        if let Some(mut poly) = Polygon::new(pts) {
            poly.texture = if is_top {
                config.belt_texture.clone()
            } else {
                config.frame_texture.clone()
            };
            if is_top {
                // Top surface carries PF_TEX_PAN (0x00000008) in UE1
                poly.flags |= 0x00000008;
            }
            poly.flags |= 0x00200000; // PF_SEMISOLID
            deck_brush.polys.push(poly);
        }
    }

    // 2. Generate End Rollers (Intake & Discharge cylinders)
    if config.rollers.has_end_rollers {
        let subs = config.rollers.roller_subdivisions.max(6);
        let r = config.rollers.roller_radius;
        let centers = [
            config.position - fwd_inclined * half_l,
            config.position + fwd_inclined * half_l,
        ];

        for center in centers {
            let mut rim_left = Vec::new();
            let mut rim_right = Vec::new();

            for s in 0..subs {
                let a = (s as f32 / subs as f32) * std::f32::consts::TAU;
                let radial = fwd_inclined * a.cos() + up_inclined * a.sin();
                rim_left.push(center + radial * r - right * (half_w + 2.0));
                rim_right.push(center + radial * r + right * (half_w + 2.0));
            }

            for s in 0..subs {
                let next = (s + 1) % subs;
                let face = vec![rim_left[s], rim_right[s], rim_right[next], rim_left[next]];
                if let Some(mut poly) = Polygon::new(face) {
                    poly.texture = config.frame_texture.clone();
                    poly.flags |= 0x00200000;
                    rollers_brush.polys.push(poly);
                }
            }

            if let Some(mut poly) = Polygon::new(rim_left) {
                poly.texture = config.frame_texture.clone();
                poly.flags |= 0x00200000;
                rollers_brush.polys.push(poly);
            }
            let mut rev_right = rim_right.clone();
            rev_right.reverse();
            if let Some(mut poly) = Polygon::new(rev_right) {
                poly.texture = config.frame_texture.clone();
                poly.flags |= 0x00200000;
                rollers_brush.polys.push(poly);
            }
        }
    }

    // 3. Generate Guardrails & Support Frame
    if config.guardrails.has_guardrails {
        let rh = config.guardrails.rail_height;
        let rw = config.guardrails.rail_width;

        // Left Guardrail
        let left_c1 = config.position - fwd_inclined * half_l - right * (half_w + rw) + up_inclined * half_t;
        let left_c2 = config.position + fwd_inclined * half_l - right * (half_w + rw) + up_inclined * half_t;
        let left_c3 = left_c2 + up_inclined * rh;
        let left_c4 = left_c1 + up_inclined * rh;
        if let Some(mut poly) = Polygon::new(vec![left_c1, left_c2, left_c3, left_c4]) {
            poly.texture = config.frame_texture.clone();
            poly.flags |= 0x00200000;
            frame_brush.polys.push(poly);
        }

        // Right Guardrail
        let right_c1 = config.position - fwd_inclined * half_l + right * half_w + up_inclined * half_t;
        let right_c2 = config.position + fwd_inclined * half_l + right * half_w + up_inclined * half_t;
        let right_c3 = right_c2 + up_inclined * rh;
        let right_c4 = right_c1 + up_inclined * rh;
        if let Some(mut poly) = Polygon::new(vec![right_c4, right_c3, right_c2, right_c1]) {
            poly.texture = config.frame_texture.clone();
            poly.flags |= 0x00200000;
            frame_brush.polys.push(poly);
        }

        // Support Legs
        if config.guardrails.has_support_legs {
            let spacing = config.guardrails.leg_spacing.max(64.0);
            let leg_count = (config.deck.length / spacing).ceil() as usize + 1;
            let leg_h = 48.0;

            for i in 0..leg_count {
                let u = (i as f32 / (leg_count - 1).max(1) as f32) - 0.5;
                let leg_center = config.position + fwd_inclined * (u * config.deck.length) - up_inclined * half_t;

                let leg_pts_l = vec![
                    leg_center - right * (half_w + rw),
                    leg_center - right * (half_w + rw) - up_inclined * leg_h,
                    leg_center - right * half_w - up_inclined * leg_h,
                    leg_center - right * half_w,
                ];
                if let Some(mut poly) = Polygon::new(leg_pts_l) {
                    poly.texture = config.frame_texture.clone();
                    poly.flags |= 0x00200000;
                    frame_brush.polys.push(poly);
                }

                let leg_pts_r = vec![
                    leg_center + right * half_w,
                    leg_center + right * half_w - up_inclined * leg_h,
                    leg_center + right * (half_w + rw) - up_inclined * leg_h,
                    leg_center + right * (half_w + rw),
                ];
                if let Some(mut poly) = Polygon::new(leg_pts_r) {
                    poly.texture = config.frame_texture.clone();
                    poly.flags |= 0x00200000;
                    frame_brush.polys.push(poly);
                }
            }
        }
    }

    (deck_brush, rollers_brush, frame_brush)
}

/// Audits conveyor belt configuration against kinematic and safety hazards
pub fn audit_conveyor_safety(
    config: &ConveyorConfig,
    _world_brushes: &[Brush],
) -> Vec<ConveyorHazard> {
    let mut hazards = Vec::new();

    // 1. Extreme Velocity (>1000 uu/s)
    if config.kinematics.speed_uu_per_sec > 1000.0 {
        hazards.push(ConveyorHazard {
            is_critical: true,
            title: format!("Excessive Conveyor Velocity ({:.0} uu/s)", config.kinematics.speed_uu_per_sec),
            description: "Belt velocity exceeds 1000 uu/s. Players landing on the belt will be ejected violently or glitch into walls.".to_string(),
            recommendation: "Reduce conveyor speed to 128..512 uu/s.".to_string(),
        });
    }

    // 2. Steep Incline Slip Hazard (>30° incline)
    if config.deck.incline_angle_deg > 30.0 {
        hazards.push(ConveyorHazard {
            is_critical: false,
            title: format!("Steep Incline Angle ({:.1}°)", config.deck.incline_angle_deg),
            description: "Incline exceeds 30°. Players will slide backwards down the belt due to UT99 slope friction physics unless cleats/steps are used.".to_string(),
            recommendation: "Reduce incline angle to <= 25° or increase belt speed and add cleat textures.".to_string(),
        });
    }

    // 3. Narrow Conveyor Width (<64 uu)
    if config.deck.width < 64.0 {
        hazards.push(ConveyorHazard {
            is_critical: true,
            title: format!("Sub-Clearance Belt Width ({:.0} uu)", config.deck.width),
            description: "Belt width is narrower than standard UT99 player collision diameter (34 uu) with safety margin. Players and bots will clip guardrails.".to_string(),
            recommendation: "Increase belt width to at least 96..128 uu.".to_string(),
        });
    }

    // 4. Missing End Rollers on Exposed Belt
    if !config.rollers.has_end_rollers {
        hazards.push(ConveyorHazard {
            is_critical: false,
            title: "Missing Intake/Discharge Rollers".to_string(),
            description: "Conveyor belt ends terminate abruptly without cylindrical roller transitions, causing an unnatural geometric break.".to_string(),
            recommendation: "Enable End Rollers for realistic industrial visual finish.".to_string(),
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conveyor_archetypes_and_defaults() {
        for &arch in ConveyorArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.deck.length >= 256.0);
            assert!(cfg.deck.width >= 64.0);
            assert!(cfg.kinematics.speed_uu_per_sec > 0.0);
            assert!(!cfg.belt_texture.is_empty());
        }
    }

    #[test]
    fn test_conveyor_sync_metrics_and_pan_rate() {
        let cfg = ConveyorArchetype::MiningOreConveyor.default_config();
        let metrics = calculate_conveyor_sync_metrics(&cfg);

        // Speed is 256 uu/s forward
        assert_eq!(metrics.linear_velocity.x, 256.0);
        assert_eq!(metrics.texture_pan_rate_pixels_sec, 256.0);
        assert!(metrics.roller_rpm > 0.0);
    }

    #[test]
    fn test_generate_conveyor_brushes_and_flags() {
        let cfg = ConveyorArchetype::MiningOreConveyor.default_config();
        let (deck_b, rollers_b, frame_b) = generate_conveyor_brushes(&cfg);

        // Deck has 6 faces
        assert_eq!(deck_b.polys.len(), 6);
        // Top face carries PF_TEX_PAN (0x08) and PF_SEMISOLID (0x200000)
        assert_eq!(deck_b.polys[0].flags & 0x00000008, 0x00000008);
        assert_eq!(deck_b.polys[0].flags & 0x00200000, 0x00200000);

        // Rollers have polys
        assert!(!rollers_b.polys.is_empty());
        // Frame has guardrails and legs
        assert!(!frame_b.polys.is_empty());
    }

    #[test]
    fn test_conveyor_safety_audit() {
        let mut cfg = ConveyorConfig::default();
        cfg.kinematics.speed_uu_per_sec = 1500.0; // Excessive
        cfg.deck.width = 40.0; // Narrow

        let hazards = audit_conveyor_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Excessive")));
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Sub-Clearance")));
    }
}
