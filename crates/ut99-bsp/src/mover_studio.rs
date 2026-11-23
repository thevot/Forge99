//! Interactive Mover Encroachment, Door Sequencing & Crushing Trap Studio
//!
//! Provides multi-keyframe trajectory evaluation, swept encroachment collision,
//! pinch-point hazard auditing, 1-click proximity trigger auto-wiring, and
//! real-time bot crushing simulation for Unreal Engine 1 movers and doors.

use crate::mover_anim::{MoverGlideType, MoverKeyframe};
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical encroachment handling modes in Unreal Engine 1 movers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MoverEncroachType {
    #[default]
    StopWhenEncroach,   // Halts movement immediately upon collision with pawn
    ReturnWhenEncroach, // Reverses trajectory back to Key 0 upon obstacle contact
    CrushWhenEncroach,  // Continuously applies crushing damage until pawn dies or leaves
    IgnoreWhenEncroach, // Clips through or pushes pawn without interrupting cycle
}

impl MoverEncroachType {
    pub const ALL: [Self; 4] = [
        Self::StopWhenEncroach,
        Self::ReturnWhenEncroach,
        Self::CrushWhenEncroach,
        Self::IgnoreWhenEncroach,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::StopWhenEncroach => "Stop When Encroached (ME_StopWhenEncroach)",
            Self::ReturnWhenEncroach => "Return When Encroached (ME_ReturnWhenEncroach)",
            Self::CrushWhenEncroach => "Crush When Encroached (ME_CrushWhenEncroach)",
            Self::IgnoreWhenEncroach => "Ignore Encroachment (ME_IgnoreWhenEncroach)",
        }
    }
}

/// Canonical trigger activation modes for Unreal Engine 1 movers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MoverTriggerType {
    #[default]
    BumpOpenTimed,    // Opens upon player touch/bump, closes after stay_open_time
    StandOnOpenTimed, // Opens when player stands atop mover deck
    TriggerOpenTimed, // Opens via external Trigger actor event, closes after stay_open_time
    TriggerToggle,    // Toggles open/closed state on each trigger pulse
    TriggerControl,   // Remains open as long as trigger is held, closes when un-triggered
    TriggerPound,     // Continuously pounds back and forth between Key 0 and Key 1
}

impl MoverTriggerType {
    pub const ALL: [Self; 6] = [
        Self::BumpOpenTimed,
        Self::StandOnOpenTimed,
        Self::TriggerOpenTimed,
        Self::TriggerToggle,
        Self::TriggerControl,
        Self::TriggerPound,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BumpOpenTimed => "Bump Open Timed (Automatic Touch)",
            Self::StandOnOpenTimed => "Stand-On Open Timed (Pressure Deck)",
            Self::TriggerOpenTimed => "Trigger Open Timed (Remote Button/Sensor)",
            Self::TriggerToggle => "Trigger Toggle (Alternate Switch)",
            Self::TriggerControl => "Trigger Control (Hold While Active)",
            Self::TriggerPound => "Trigger Pound (Continuous Hazard Cycle)",
        }
    }
}

/// Archetype templates for common competitive UT99 doors and traps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MoverTemplateArchetype {
    #[default]
    AutomaticSlidingDoor,
    VerticalBlastBulkhead,
    RotatingSecurityGate,
    HydraulicCrusherTrap,
    CastleDrawbridgeHinged,
    ContinuousRotatingFan,
}

impl MoverTemplateArchetype {
    pub const ALL: [Self; 6] = [
        Self::AutomaticSlidingDoor,
        Self::VerticalBlastBulkhead,
        Self::RotatingSecurityGate,
        Self::HydraulicCrusherTrap,
        Self::CastleDrawbridgeHinged,
        Self::ContinuousRotatingFan,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AutomaticSlidingDoor => "Automatic Sliding Door (Horizontal Split)",
            Self::VerticalBlastBulkhead => "Vertical Blast Bulkhead (Heavy Rising)",
            Self::RotatingSecurityGate => "Rotating Security Gate (90° Swing)",
            Self::HydraulicCrusherTrap => "Hydraulic Crusher Trap (Lethal Smash)",
            Self::CastleDrawbridgeHinged => "Castle Drawbridge (Hinged Descent)",
            Self::ContinuousRotatingFan => "Continuous Rotating Fan (Spinning Hazard)",
        }
    }

    pub fn to_config(&self, origin: Vec3) -> MoverSequenceConfig {
        match self {
            Self::AutomaticSlidingDoor => MoverSequenceConfig {
                mover_name: "SlidingDoor".to_string(),
                mover_tag: "Door_Slide01".to_string(),
                event: String::new(),
                encroach_type: MoverEncroachType::ReturnWhenEncroach,
                trigger_type: MoverTriggerType::BumpOpenTimed,
                glide_type: MoverGlideType::Smooth,
                move_time: 1.5,
                stay_open_time: 3.0,
                crush_damage: 0.0,
                keyframes: vec![
                    MoverKeyframe { position: origin, rotation: Vec3::ZERO },
                    MoverKeyframe { position: origin + Vec3::new(128.0, 0.0, 0.0), rotation: Vec3::ZERO },
                ],
                opening_sound: "Doors.DoorOpen1".to_string(),
                opened_sound: "Doors.DoorOpened1".to_string(),
                closing_sound: "Doors.DoorClose1".to_string(),
                closed_sound: "Doors.DoorClosed1".to_string(),
                move_ambient_sound: "AmbModern.HumLoop1".to_string(),
            },
            Self::VerticalBlastBulkhead => MoverSequenceConfig {
                mover_name: "BlastDoor".to_string(),
                mover_tag: "Bulkhead01".to_string(),
                event: String::new(),
                encroach_type: MoverEncroachType::StopWhenEncroach,
                trigger_type: MoverTriggerType::TriggerOpenTimed,
                glide_type: MoverGlideType::Linear,
                move_time: 2.2,
                stay_open_time: 4.0,
                crush_damage: 25.0,
                keyframes: vec![
                    MoverKeyframe { position: origin, rotation: Vec3::ZERO },
                    MoverKeyframe { position: origin + Vec3::new(0.0, 0.0, 160.0), rotation: Vec3::ZERO },
                ],
                opening_sound: "Doors.HeavyDoorOpen".to_string(),
                opened_sound: "Doors.HeavyLatch".to_string(),
                closing_sound: "Doors.HeavyDoorClose".to_string(),
                closed_sound: "Doors.HeavySlam".to_string(),
                move_ambient_sound: "AmbAncient.GearGrind1".to_string(),
            },
            Self::RotatingSecurityGate => MoverSequenceConfig {
                mover_name: "SecurityGate".to_string(),
                mover_tag: "Gate01".to_string(),
                event: String::new(),
                encroach_type: MoverEncroachType::ReturnWhenEncroach,
                trigger_type: MoverTriggerType::BumpOpenTimed,
                glide_type: MoverGlideType::Smooth,
                move_time: 1.8,
                stay_open_time: 3.5,
                crush_damage: 0.0,
                keyframes: vec![
                    MoverKeyframe { position: origin, rotation: Vec3::ZERO },
                    MoverKeyframe { position: origin, rotation: Vec3::new(0.0, 90.0, 0.0) },
                ],
                opening_sound: "Doors.GateOpen".to_string(),
                opened_sound: "Doors.GateClick".to_string(),
                closing_sound: "Doors.GateClose".to_string(),
                closed_sound: "Doors.GateLock".to_string(),
                move_ambient_sound: String::new(),
            },
            Self::HydraulicCrusherTrap => MoverSequenceConfig {
                mover_name: "CrusherTrap".to_string(),
                mover_tag: "CrushTrap01".to_string(),
                event: String::new(),
                encroach_type: MoverEncroachType::CrushWhenEncroach,
                trigger_type: MoverTriggerType::TriggerPound,
                glide_type: MoverGlideType::Linear,
                move_time: 0.6,
                stay_open_time: 1.0,
                crush_damage: 1000.0, // Instagib crushing
                keyframes: vec![
                    MoverKeyframe { position: origin, rotation: Vec3::ZERO },
                    MoverKeyframe { position: origin - Vec3::new(0.0, 0.0, 192.0), rotation: Vec3::ZERO },
                ],
                opening_sound: "Metal.PistonUp".to_string(),
                opened_sound: "Metal.PistonLock".to_string(),
                closing_sound: "Metal.PistonDownFast".to_string(),
                closed_sound: "Metal.SmashImpact".to_string(),
                move_ambient_sound: "AmbModern.HydraulicHum".to_string(),
            },
            Self::CastleDrawbridgeHinged => MoverSequenceConfig {
                mover_name: "Drawbridge".to_string(),
                mover_tag: "Bridge01".to_string(),
                event: String::new(),
                encroach_type: MoverEncroachType::IgnoreWhenEncroach,
                trigger_type: MoverTriggerType::TriggerToggle,
                glide_type: MoverGlideType::Smooth,
                move_time: 3.5,
                stay_open_time: 0.0,
                crush_damage: 15.0,
                keyframes: vec![
                    MoverKeyframe { position: origin, rotation: Vec3::new(80.0, 0.0, 0.0) },
                    MoverKeyframe { position: origin, rotation: Vec3::ZERO },
                ],
                opening_sound: "Chains.ChainRattle".to_string(),
                opened_sound: "Wood.BridgeThud".to_string(),
                closing_sound: "Chains.ChainCrank".to_string(),
                closed_sound: "Wood.BridgeLatch".to_string(),
                move_ambient_sound: "Chains.CreakLoop".to_string(),
            },
            Self::ContinuousRotatingFan => MoverSequenceConfig {
                mover_name: "RotatingFan".to_string(),
                mover_tag: "Fan01".to_string(),
                event: String::new(),
                encroach_type: MoverEncroachType::IgnoreWhenEncroach,
                trigger_type: MoverTriggerType::TriggerPound,
                glide_type: MoverGlideType::Linear,
                move_time: 2.0,
                stay_open_time: 0.0,
                crush_damage: 35.0,
                keyframes: vec![
                    MoverKeyframe { position: origin, rotation: Vec3::ZERO },
                    MoverKeyframe { position: origin, rotation: Vec3::new(0.0, 180.0, 0.0) },
                    MoverKeyframe { position: origin, rotation: Vec3::new(0.0, 360.0, 0.0) },
                ],
                opening_sound: String::new(),
                opened_sound: String::new(),
                closing_sound: String::new(),
                closed_sound: String::new(),
                move_ambient_sound: "AmbModern.FanRotorLoop".to_string(),
            },
        }
    }
}

/// Full configuration profile for a mover sequence and its physical parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoverSequenceConfig {
    pub mover_name: String,
    pub mover_tag: String,
    pub event: String,
    pub encroach_type: MoverEncroachType,
    pub trigger_type: MoverTriggerType,
    pub glide_type: MoverGlideType,
    pub move_time: f32,
    pub stay_open_time: f32,
    pub crush_damage: f32,
    pub keyframes: Vec<MoverKeyframe>,
    pub opening_sound: String,
    pub opened_sound: String,
    pub closing_sound: String,
    pub closed_sound: String,
    pub move_ambient_sound: String,
}

impl Default for MoverSequenceConfig {
    fn default() -> Self {
        MoverTemplateArchetype::AutomaticSlidingDoor.to_config(Vec3::ZERO)
    }
}

impl MoverSequenceConfig {
    /// Evaluates current position and rotation at normalized progress t in [0.0, 1.0].
    pub fn evaluate_pose(&self, t: f32) -> (Vec3, Vec3) {
        if self.keyframes.is_empty() {
            return (Vec3::ZERO, Vec3::ZERO);
        }
        if self.keyframes.len() == 1 {
            return (self.keyframes[0].position, self.keyframes[0].rotation);
        }

        let clamped_t = t.clamp(0.0, 1.0);
        let segment_count = (self.keyframes.len() - 1) as f32;
        let scaled_t = clamped_t * segment_count;
        let seg_idx = (scaled_t.floor() as usize).min(self.keyframes.len() - 2);
        let local_t = (scaled_t - seg_idx as f32).clamp(0.0, 1.0);

        let eased_t = match self.glide_type {
            MoverGlideType::Linear => local_t,
            MoverGlideType::Smooth => 0.5 * (1.0 - (std::f32::consts::PI * local_t).cos()),
        };

        let k0 = &self.keyframes[seg_idx];
        let k1 = &self.keyframes[seg_idx + 1];

        (k0.position.lerp(k1.position, eased_t), k0.rotation.lerp(k1.rotation, eased_t))
    }

    /// Computes the total swept axis-aligned bounding box enclosing all keyframe positions.
    pub fn compute_swept_bounds(&self, half_extent: Vec3) -> (Vec3, Vec3) {
        if self.keyframes.is_empty() {
            return (-half_extent, half_extent);
        }

        let mut b_min = Vec3::splat(f32::MAX);
        let mut b_max = Vec3::splat(f32::MIN);

        for k in &self.keyframes {
            b_min = b_min.min(k.position - half_extent);
            b_max = b_max.max(k.position + half_extent);
        }

        (b_min, b_max)
    }
}

/// Severity classification for mover collision and timing diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoverHazardSeverity {
    Info,
    Warning,
    CriticalLethalHazard,
}

/// Diagnostic report item for pinch points, missing triggers, or glitch timing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoverHazardDiagnostic {
    pub hazard_type: String,
    pub location: Vec3,
    pub severity: MoverHazardSeverity,
    pub description: String,
}

/// Audits a mover configuration against surrounding geometry for pinch points and logic issues.
pub fn audit_mover_hazards(
    config: &MoverSequenceConfig,
    mover_half_extent: Vec3,
    surrounding_surfaces: &[Vec3],
) -> Vec<MoverHazardDiagnostic> {
    let mut diags = Vec::new();

    // 1. Check MoveTime
    if config.move_time <= 0.08 {
        diags.push(MoverHazardDiagnostic {
            hazard_type: "SubsecondGlitchSpeed".to_string(),
            location: config.keyframes.first().map(|k| k.position).unwrap_or(Vec3::ZERO),
            severity: MoverHazardSeverity::CriticalLethalHazard,
            description: format!(
                "MoveTime ({:.2}s) is dangerously small (<0.08s); triggers physics engine clipping and player ejection glitches.",
                config.move_time
            ),
        });
    }

    // 2. Check Trigger requirements
    if matches!(config.trigger_type, MoverTriggerType::TriggerOpenTimed | MoverTriggerType::TriggerToggle | MoverTriggerType::TriggerControl) {
        if config.mover_tag.trim().is_empty() {
            diags.push(MoverHazardDiagnostic {
                hazard_type: "UncallableMoverNoTag".to_string(),
                location: config.keyframes.first().map(|k| k.position).unwrap_or(Vec3::ZERO),
                severity: MoverHazardSeverity::CriticalLethalHazard,
                description: "Mover is set to a Trigger activation mode but has an empty Tag; cannot be operated by any Trigger actor.".to_string(),
            });
        }
    }

    // 3. Check for Pinch-Point Hazards with surrounding geometry
    for (key_idx, k) in config.keyframes.iter().enumerate() {
        for &surf in surrounding_surfaces {
            let dist = (k.position - surf).length();
            let min_clearance = mover_half_extent.length() + 8.0;

            if dist < min_clearance && dist > 1.0 {
                let clearance_gap = dist - mover_half_extent.length();
                if clearance_gap < 24.0 {
                    if config.encroach_type == MoverEncroachType::IgnoreWhenEncroach {
                        diags.push(MoverHazardDiagnostic {
                            hazard_type: "LethalPinchPointHazard".to_string(),
                            location: k.position,
                            severity: MoverHazardSeverity::CriticalLethalHazard,
                            description: format!(
                                "Keyframe {} closes to {:.1} uu from wall with ME_IgnoreWhenEncroach; players and bots will be pinned or clipped through BSP.",
                                key_idx, clearance_gap.max(0.0)
                            ),
                        });
                    } else if config.encroach_type == MoverEncroachType::StopWhenEncroach {
                        diags.push(MoverHazardDiagnostic {
                            hazard_type: "DoorBlockageHazard".to_string(),
                            location: k.position,
                            severity: MoverHazardSeverity::Warning,
                            description: format!(
                                "Keyframe {} has tight gap ({:.1} uu); door will permanently halt if a bot stands in jamb.",
                                key_idx, clearance_gap.max(0.0)
                            ),
                        });
                    }
                }
            }
        }
    }

    // 4. Check Crushing trap effectiveness
    if config.encroach_type == MoverEncroachType::CrushWhenEncroach && config.crush_damage < 5.0 {
        diags.push(MoverHazardDiagnostic {
            hazard_type: "IneffectiveCrushTrap".to_string(),
            location: config.keyframes.last().map(|k| k.position).unwrap_or(Vec3::ZERO),
            severity: MoverHazardSeverity::Warning,
            description: format!(
                "Mover has CrushWhenEncroach set, but CrushDamage is only {:.1} HP; trap deals negligible damage.",
                config.crush_damage
            ),
        });
    }

    // 5. Sound assignment check
    if config.opening_sound.is_empty() && config.closing_sound.is_empty() {
        diags.push(MoverHazardDiagnostic {
            hazard_type: "MissingAudioFeedback".to_string(),
            location: config.keyframes.first().map(|k| k.position).unwrap_or(Vec3::ZERO),
            severity: MoverHazardSeverity::Info,
            description: "No OpeningSound or ClosingSound assigned; mover will operate silently with no player cue.".to_string(),
        });
    }

    diags
}

/// Simulated outcome of a pawn encroaching into a mover's path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoverEncroachSimResult {
    pub impact_time_norm: f32,
    pub impact_point: Vec3,
    pub damage_dealt: f32,
    pub pawn_pushed_offset: Vec3,
    pub mover_reaction: &'static str,
}

/// Simulates what occurs when a bot/pawn is standing in a mover's path.
pub fn simulate_mover_encroachment(
    config: &MoverSequenceConfig,
    bot_location: Vec3,
    bot_radius: f32,
    mover_half_extent: Vec3,
) -> Option<MoverEncroachSimResult> {
    if config.keyframes.len() < 2 {
        return None;
    }

    // Test 32 timeline samples
    for i in 1..=32 {
        let t = i as f32 / 32.0;
        let (pos, _) = config.evaluate_pose(t);
        let b_min = pos - mover_half_extent;
        let b_max = pos + mover_half_extent;

        // Check AABB vs Bot cylinder/sphere approximation
        let bot_min = bot_location - Vec3::new(bot_radius, bot_radius, 38.0);
        let bot_max = bot_location + Vec3::new(bot_radius, bot_radius, 38.0);

        let overlap_x = b_min.x <= bot_max.x && b_max.x >= bot_min.x;
        let overlap_y = b_min.y <= bot_max.y && b_max.y >= bot_min.y;
        let overlap_z = b_min.z <= bot_max.z && b_max.z >= bot_min.z;

        if overlap_x && overlap_y && overlap_z {
            let push_dir = (bot_location - pos).normalize_or_zero();
            return match config.encroach_type {
                MoverEncroachType::StopWhenEncroach => Some(MoverEncroachSimResult {
                    impact_time_norm: t,
                    impact_point: bot_location,
                    damage_dealt: 0.0,
                    pawn_pushed_offset: Vec3::ZERO,
                    mover_reaction: "Halted at encroachment position",
                }),
                MoverEncroachType::ReturnWhenEncroach => Some(MoverEncroachSimResult {
                    impact_time_norm: t,
                    impact_point: bot_location,
                    damage_dealt: 0.0,
                    pawn_pushed_offset: Vec3::ZERO,
                    mover_reaction: "Reversing trajectory back to Key 0",
                }),
                MoverEncroachType::CrushWhenEncroach => Some(MoverEncroachSimResult {
                    impact_time_norm: t,
                    impact_point: bot_location,
                    damage_dealt: config.crush_damage.max(20.0),
                    pawn_pushed_offset: push_dir * 16.0,
                    mover_reaction: "Crushing pawn with continuous damage",
                }),
                MoverEncroachType::IgnoreWhenEncroach => Some(MoverEncroachSimResult {
                    impact_time_norm: t,
                    impact_point: bot_location,
                    damage_dealt: 5.0,
                    pawn_pushed_offset: push_dir * 48.0,
                    mover_reaction: "Displacing pawn along travel vector",
                }),
            };
        }
    }

    None
}

/// Generated Trigger actor record for 1-click door proximity wiring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedDoorTrigger {
    pub name: String,
    pub event: String,
    pub location: Vec3,
    pub collision_radius: f32,
    pub collision_height: f32,
}

/// Generates a perfectly positioned Trigger actor to operate a door.
pub fn generate_door_proximity_trigger(
    mover_name: &str,
    mover_tag: &str,
    door_origin: Vec3,
    forward_dir: Vec3,
    approach_distance: f32,
    trigger_radius: f32,
) -> GeneratedDoorTrigger {
    let fwd = if forward_dir.length_squared() > 0.01 {
        forward_dir.normalize()
    } else {
        Vec3::X
    };

    let loc = door_origin + fwd * approach_distance.max(32.0);
    let tag = if !mover_tag.trim().is_empty() {
        mover_tag.to_string()
    } else {
        format!("{}_Event", mover_name)
    };

    GeneratedDoorTrigger {
        name: format!("Trigger_{}", mover_name),
        event: tag,
        location: loc,
        collision_radius: trigger_radius.clamp(32.0, 256.0),
        collision_height: 48.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mover_templates_to_config() {
        assert_eq!(MoverTemplateArchetype::ALL.len(), 6);
        for template in &MoverTemplateArchetype::ALL {
            let cfg = template.to_config(Vec3::new(100.0, 200.0, 300.0));
            assert!(!cfg.mover_name.is_empty());
            assert!(cfg.move_time > 0.0);
            assert!(cfg.keyframes.len() >= 2);
            assert_eq!(cfg.keyframes[0].position, Vec3::new(100.0, 200.0, 300.0));
        }
    }

    #[test]
    fn test_pose_evaluation_and_swept_bounds() {
        let mut cfg = MoverTemplateArchetype::AutomaticSlidingDoor.to_config(Vec3::ZERO);
        cfg.keyframes = vec![
            MoverKeyframe { position: Vec3::ZERO, rotation: Vec3::ZERO },
            MoverKeyframe { position: Vec3::new(200.0, 0.0, 0.0), rotation: Vec3::ZERO },
        ];

        let (p_start, _) = cfg.evaluate_pose(0.0);
        let (p_mid, _) = cfg.evaluate_pose(0.5);
        let (p_end, _) = cfg.evaluate_pose(1.0);

        assert_eq!(p_start, Vec3::ZERO);
        assert!((p_mid.x - 100.0).abs() < 1e-2);
        assert_eq!(p_end, Vec3::new(200.0, 0.0, 0.0));

        let (b_min, b_max) = cfg.compute_swept_bounds(Vec3::new(32.0, 16.0, 64.0));
        assert_eq!(b_min, Vec3::new(-32.0, -16.0, -64.0));
        assert_eq!(b_max, Vec3::new(232.0, 16.0, 64.0));
    }

    #[test]
    fn test_hazard_audit_detects_subsecond_and_pinch() {
        let mut cfg = MoverTemplateArchetype::VerticalBlastBulkhead.to_config(Vec3::ZERO);
        cfg.move_time = 0.02; // Dangerously fast
        cfg.encroach_type = MoverEncroachType::IgnoreWhenEncroach;
        cfg.keyframes = vec![
            MoverKeyframe { position: Vec3::ZERO, rotation: Vec3::ZERO },
            MoverKeyframe { position: Vec3::new(0.0, 0.0, 100.0), rotation: Vec3::ZERO },
        ];

        let wall = vec![Vec3::new(0.0, 0.0, 102.0)];
        let hazards = audit_mover_hazards(&cfg, Vec3::new(16.0, 16.0, 16.0), &wall);

        assert!(hazards.iter().any(|h| h.hazard_type == "SubsecondGlitchSpeed"));
        assert!(hazards.iter().any(|h| h.hazard_type == "LethalPinchPointHazard"));
    }

    #[test]
    fn test_simulate_mover_encroachment_crush_and_return() {
        let mut cfg = MoverTemplateArchetype::HydraulicCrusherTrap.to_config(Vec3::new(0.0, 0.0, 200.0));
        cfg.keyframes = vec![
            MoverKeyframe { position: Vec3::new(0.0, 0.0, 200.0), rotation: Vec3::ZERO },
            MoverKeyframe { position: Vec3::new(0.0, 0.0, 0.0), rotation: Vec3::ZERO },
        ];
        cfg.crush_damage = 500.0;
        cfg.encroach_type = MoverEncroachType::CrushWhenEncroach;

        let bot_pos = Vec3::new(0.0, 0.0, 50.0);
        let res = simulate_mover_encroachment(&cfg, bot_pos, 17.0, Vec3::new(32.0, 32.0, 16.0));
        assert!(res.is_some());
        let sim = res.unwrap();
        assert_eq!(sim.damage_dealt, 500.0);

        // Test with return
        cfg.encroach_type = MoverEncroachType::ReturnWhenEncroach;
        let res2 = simulate_mover_encroachment(&cfg, bot_pos, 17.0, Vec3::new(32.0, 32.0, 16.0)).unwrap();
        assert_eq!(res2.damage_dealt, 0.0);
        assert_eq!(res2.mover_reaction, "Reversing trajectory back to Key 0");
    }

    #[test]
    fn test_generate_door_proximity_trigger() {
        let trig = generate_door_proximity_trigger(
            "MainVault",
            "VaultTag",
            Vec3::new(100.0, 200.0, 50.0),
            Vec3::Y,
            96.0,
            64.0,
        );

        assert_eq!(trig.name, "Trigger_MainVault");
        assert_eq!(trig.event, "VaultTag");
        assert_eq!(trig.location, Vec3::new(100.0, 296.0, 50.0));
        assert_eq!(trig.collision_radius, 64.0);
    }
}
