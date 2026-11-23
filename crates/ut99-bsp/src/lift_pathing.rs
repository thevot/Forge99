//! Interactive LiftCenter, LiftExit & Dynamic Elevator Pathing Studio
//!
//! Provides topological analysis, diagnostic auditing, mover synchronization,
//! keyframe floor matching, automated lift setup generation, and real-time
//! bot riding trajectory simulation for Unreal Engine 1 elevator systems.

use crate::mover_anim::MoverKeyframe;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical severity classification for elevator pathing diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiftDiagnosticSeverity {
    Info,
    Warning,
    CriticalHazard,
}

impl LiftDiagnosticSeverity {
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::CriticalHazard => "CRITICAL HAZARD",
        }
    }
}

/// Category of elevator pathing issue or configuration defect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiftDiagnosticCategory {
    /// LiftCenter or LiftExit has a `LiftTag` that does not match any Mover in the level.
    MismatchedLiftTag,
    /// Mover has LiftExit actors referencing its Tag, but no `LiftCenter` is on the platform.
    MissingLiftCenter,
    /// Mover keyframe (floor) has no associated `LiftExit` actor nearby.
    MissingFloorExit,
    /// `LiftExit` is assigned to Keyframe K, but its elevation (Z) does not match Mover at Keyframe K.
    KeyframeElevationMismatch,
    /// High vertical stroke (> 500 uu) where missing railings or timing desync causes lethal fall.
    LethalPlungeHazard,
    /// Mover has `TriggerOpenTimed` state, but no `Trigger` is wired or within reach of the exit.
    UncallableLiftHazard,
    /// MoveTime is excessively long (> 15.0 s), damaging multiplayer combat flow.
    ExcessiveTransitTime,
    /// MoveTime is dangerously fast (< 0.15 s), risking physics tunneling or crush glitches.
    SubsecondGlitchSpeed,
    /// `LiftCenter` is placed far off-center (> platform radius), risking clipping off the edge.
    OffCenterLiftCenter,
    /// `LiftCenter` is floating too high (> 80 uu) above the mover platform deck.
    FloatingLiftCenter,
}

impl LiftDiagnosticCategory {
    pub fn title(&self) -> &'static str {
        match self {
            Self::MismatchedLiftTag => "Mismatched LiftTag",
            Self::MissingLiftCenter => "Missing LiftCenter on Platform",
            Self::MissingFloorExit => "Missing LiftExit at Floor",
            Self::KeyframeElevationMismatch => "Floor Elevation Mismatch",
            Self::LethalPlungeHazard => "Lethal Plunge Hazard",
            Self::UncallableLiftHazard => "Uncallable Lift (Missing Trigger)",
            Self::ExcessiveTransitTime => "Excessive Transit Time",
            Self::SubsecondGlitchSpeed => "Sub-second Glitch Speed",
            Self::OffCenterLiftCenter => "Off-Center LiftCenter",
            Self::FloatingLiftCenter => "Floating LiftCenter",
        }
    }
}

/// A diagnostic finding for an elevator system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiftDiagnostic {
    pub severity: LiftDiagnosticSeverity,
    pub category: LiftDiagnosticCategory,
    pub mover_tag: String,
    pub mover_name: String,
    pub actor_name: String,
    pub location: Vec3,
    pub message: String,
    pub suggested_fix: String,
}

/// Represents one floor / stop corresponding to a Mover keyframe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiftKeyframeFloor {
    pub keyframe_index: usize,
    pub mover_position: Vec3,
    pub mover_rotation: Vec3,
    pub associated_exits: Vec<String>,
    pub nearest_exit_distance: f32,
    pub is_aligned: bool,
}

/// Data extracted from a `LiftExit` navigation actor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiftExitData {
    pub name: String,
    pub lift_tag: String,
    pub suggested_key_frame: usize,
    pub key_frame: usize,
    pub location: Vec3,
}

/// Data extracted from a `LiftCenter` navigation actor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiftCenterData {
    pub name: String,
    pub lift_tag: String,
    pub lift_trigger: String,
    pub max_dist_2d: f32,
    pub location: Vec3,
}

/// Data extracted from a `Mover` actor for elevator analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElevatorMoverData {
    pub name: String,
    pub tag: String,
    pub initial_state: String,
    pub move_time: f32,
    pub stay_open_time: f32,
    pub num_keys: usize,
    pub keyframes: Vec<MoverKeyframe>,
    pub platform_radius: f32,
}

/// Data extracted from a `Trigger` actor for lift wiring analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiftTriggerData {
    pub name: String,
    pub event: String,
    pub location: Vec3,
    pub trigger_radius: f32,
}

/// Complete profile of an elevator system, linking a Mover to its LiftCenter, LiftExits, and Triggers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiftSystemProfile {
    pub mover_name: String,
    pub mover_tag: String,
    pub initial_state: String,
    pub move_time: f32,
    pub stay_open_time: f32,
    pub num_keys: usize,
    pub keyframes: Vec<MoverKeyframe>,
    pub lift_center: Option<LiftCenterData>,
    pub lift_exits: Vec<LiftExitData>,
    pub triggers: Vec<LiftTriggerData>,
    pub floors: Vec<LiftKeyframeFloor>,
    pub diagnostics: Vec<LiftDiagnostic>,
    pub total_cycle_time: f32,
    pub is_fully_synchronized: bool,
    pub platform_radius: f32,
}

/// Lifecycle state of a bot riding the elevator during interactive simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiftBotSimulationState {
    WaitingAtBottom,
    BoardingPlatform,
    Ascending,
    ArrivedAtFloor,
    ExitingPlatform,
    WaitingAtTop,
    BoardingReturn,
    Descending,
}

impl LiftBotSimulationState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::WaitingAtBottom => "Waiting at Bottom Floor",
            Self::BoardingPlatform => "Boarding Elevator Platform",
            Self::Ascending => "Ascending to Upper Floor",
            Self::ArrivedAtFloor => "Platform Settled / Doors Open",
            Self::ExitingPlatform => "Stepping Off at Destination",
            Self::WaitingAtTop => "Waiting at Upper Floor",
            Self::BoardingReturn => "Boarding Platform for Return",
            Self::Descending => "Descending to Base Floor",
        }
    }
}

/// A discrete step in the elevator and bot riding simulation timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiftSimulationStep {
    pub time: f32,
    pub mover_pos: Vec3,
    pub mover_rot: Vec3,
    pub current_keyframe: usize,
    pub target_keyframe: usize,
    pub bot_pos: Vec3,
    pub bot_state: LiftBotSimulationState,
    pub bot_progress: f32,
}

/// Summary of an entire level's elevator and lift pathing audit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ElevatorAuditSummary {
    pub systems: Vec<LiftSystemProfile>,
    pub total_movers: usize,
    pub total_lift_centers: usize,
    pub total_lift_exits: usize,
    pub synchronized_systems_count: usize,
    pub critical_hazard_count: usize,
    pub warning_count: usize,
}

/// Output of the 1-click synchronized lift setup generator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedLiftSetup {
    pub lift_center: GeneratedLiftCenter,
    pub lift_exits: Vec<GeneratedLiftExit>,
    pub triggers: Vec<GeneratedTrigger>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedLiftCenter {
    pub name: String,
    pub lift_tag: String,
    pub lift_trigger: String,
    pub location: Vec3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedLiftExit {
    pub name: String,
    pub lift_tag: String,
    pub suggested_key_frame: usize,
    pub key_frame: usize,
    pub location: Vec3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedTrigger {
    pub name: String,
    pub event: String,
    pub location: Vec3,
}

/// Performs a comprehensive diagnostic audit of all elevator systems in a level.
pub fn audit_lift_systems(
    movers: &[ElevatorMoverData],
    lift_centers: &[LiftCenterData],
    lift_exits: &[LiftExitData],
    triggers: &[LiftTriggerData],
) -> ElevatorAuditSummary {
    let mut systems = Vec::new();
    let mut critical_count = 0;
    let mut warning_count = 0;
    let mut synchronized_count = 0;

    for mover in movers {
        let mut diagnostics = Vec::new();

        // 1. MoveTime checks
        if mover.move_time < 0.15 {
            diagnostics.push(LiftDiagnostic {
                severity: LiftDiagnosticSeverity::CriticalHazard,
                category: LiftDiagnosticCategory::SubsecondGlitchSpeed,
                mover_tag: mover.tag.clone(),
                mover_name: mover.name.clone(),
                actor_name: mover.name.clone(),
                location: mover.keyframes.first().map(|k| k.position).unwrap_or(Vec3::ZERO),
                message: format!(
                    "Mover '{}' has MoveTime={:.2}s. Speeds under 0.15s cause player crush and collision tunneling.",
                    mover.name, mover.move_time
                ),
                suggested_fix: "Set MoveTime to at least 1.0s - 3.0s for standard elevators.".to_string(),
            });
        } else if mover.move_time > 15.0 {
            diagnostics.push(LiftDiagnostic {
                severity: LiftDiagnosticSeverity::Warning,
                category: LiftDiagnosticCategory::ExcessiveTransitTime,
                mover_tag: mover.tag.clone(),
                mover_name: mover.name.clone(),
                actor_name: mover.name.clone(),
                location: mover.keyframes.first().map(|k| k.position).unwrap_or(Vec3::ZERO),
                message: format!(
                    "Mover '{}' has MoveTime={:.1}s. Transit times over 15s damage multiplayer pacing.",
                    mover.name, mover.move_time
                ),
                suggested_fix: "Adjust MoveTime to 2.0s - 6.0s for competitive gameplay.".to_string(),
            });
        }

        // 2. Find matching LiftCenter by LiftTag == mover.tag
        let center = lift_centers.iter().find(|c| !c.lift_tag.is_empty() && c.lift_tag.eq_ignore_ascii_case(&mover.tag));

        // Also check if any LiftCenter is spatially on top of the mover even if tag is wrong
        let base_pos = mover.keyframes.first().map(|k| k.position).unwrap_or(Vec3::ZERO);
        if center.is_none() {
            // Check if there are any exits matching this mover
            let has_matching_exits = lift_exits.iter().any(|e| !e.lift_tag.is_empty() && e.lift_tag.eq_ignore_ascii_case(&mover.tag));
            if has_matching_exits {
                diagnostics.push(LiftDiagnostic {
                    severity: LiftDiagnosticSeverity::CriticalHazard,
                    category: LiftDiagnosticCategory::MissingLiftCenter,
                    mover_tag: mover.tag.clone(),
                    mover_name: mover.name.clone(),
                    actor_name: mover.name.clone(),
                    location: base_pos,
                    message: format!(
                        "Mover '{}' has LiftExit actors referencing tag '{}', but no matching LiftCenter on the platform.",
                        mover.name, mover.tag
                    ),
                    suggested_fix: "Place a LiftCenter actor centered on the elevator platform with LiftTag matching Mover.Tag.".to_string(),
                });
            }
        } else if let Some(c) = center {
            // Check LiftCenter positioning relative to keyframe 0
            let horiz_dist = Vec3::new(c.location.x - base_pos.x, c.location.y - base_pos.y, 0.0).length();
            let vert_offset = c.location.z - base_pos.z;

            let max_horiz = (mover.platform_radius * 0.8).max(128.0);
            if horiz_dist > max_horiz {
                diagnostics.push(LiftDiagnostic {
                    severity: LiftDiagnosticSeverity::Warning,
                    category: LiftDiagnosticCategory::OffCenterLiftCenter,
                    mover_tag: mover.tag.clone(),
                    mover_name: mover.name.clone(),
                    actor_name: c.name.clone(),
                    location: c.location,
                    message: format!(
                        "LiftCenter '{}' is {:.1} uu off-center from platform (max recommended {:.1} uu).",
                        c.name, horiz_dist, max_horiz
                    ),
                    suggested_fix: "Center the LiftCenter directly on the platform pivot.".to_string(),
                });
            }

            if vert_offset > 80.0 {
                diagnostics.push(LiftDiagnostic {
                    severity: LiftDiagnosticSeverity::Warning,
                    category: LiftDiagnosticCategory::FloatingLiftCenter,
                    mover_tag: mover.tag.clone(),
                    mover_name: mover.name.clone(),
                    actor_name: c.name.clone(),
                    location: c.location,
                    message: format!(
                        "LiftCenter '{}' is floating {:.1} uu above the platform deck (max recommended 32 uu).",
                        c.name, vert_offset
                    ),
                    suggested_fix: "Lower LiftCenter to +20 uu above the platform deck surface.".to_string(),
                });
            }
        }

        // 3. Find matching LiftExits by LiftTag == mover.tag
        let matching_exits: Vec<LiftExitData> = lift_exits
            .iter()
            .filter(|e| !e.lift_tag.is_empty() && e.lift_tag.eq_ignore_ascii_case(&mover.tag))
            .cloned()
            .collect();

        // 4. Floor / Keyframe analysis
        let mut floors = Vec::new();
        let num_keys = mover.keyframes.len().max(mover.num_keys).max(1);

        for (k_idx, k_frame) in mover.keyframes.iter().enumerate() {
            let k_pos = k_frame.position;

            // Find exits assigned or closest to this keyframe
            let exits_at_floor: Vec<String> = matching_exits
                .iter()
                .filter(|e| {
                    let key_matches = e.suggested_key_frame == k_idx || e.key_frame == k_idx;
                    let vert_delta = (e.location.z - k_pos.z).abs();
                    key_matches || (vert_delta < 96.0 && k_pos.distance(e.location) < 600.0)
                })
                .map(|e| e.name.clone())
                .collect();

            let nearest_dist = matching_exits
                .iter()
                .map(|e| e.location.distance(k_pos))
                .fold(f32::INFINITY, f32::min);

            let is_aligned = !exits_at_floor.is_empty();

            if !is_aligned && !matching_exits.is_empty() {
                diagnostics.push(LiftDiagnostic {
                    severity: LiftDiagnosticSeverity::CriticalHazard,
                    category: LiftDiagnosticCategory::MissingFloorExit,
                    mover_tag: mover.tag.clone(),
                    mover_name: mover.name.clone(),
                    actor_name: mover.name.clone(),
                    location: k_pos,
                    message: format!(
                        "Mover '{}' has Keyframe {} (Floor {}) with no associated LiftExit actor.",
                        mover.name, k_idx, k_idx + 1
                    ),
                    suggested_fix: format!(
                        "Place a LiftExit near Keyframe {} with LiftTag='{}' and SuggestedKeyFrame={}.",
                        k_idx, mover.tag, k_idx
                    ),
                });
            }

            floors.push(LiftKeyframeFloor {
                keyframe_index: k_idx,
                mover_position: k_pos,
                mover_rotation: k_frame.rotation,
                associated_exits: exits_at_floor,
                nearest_exit_distance: if nearest_dist.is_finite() { nearest_dist } else { 0.0 },
                is_aligned,
            });
        }

        // Check for KeyframeElevationMismatch on exits
        for exit in &matching_exits {
            let assigned_key = exit.suggested_key_frame.min(num_keys.saturating_sub(1));
            if let Some(target_floor) = mover.keyframes.get(assigned_key) {
                let vert_diff = (exit.location.z - target_floor.position.z).abs();
                if vert_diff > 120.0 {
                    diagnostics.push(LiftDiagnostic {
                        severity: LiftDiagnosticSeverity::Warning,
                        category: LiftDiagnosticCategory::KeyframeElevationMismatch,
                        mover_tag: mover.tag.clone(),
                        mover_name: mover.name.clone(),
                        actor_name: exit.name.clone(),
                        location: exit.location,
                        message: format!(
                            "LiftExit '{}' is assigned to Keyframe {} but has vertical difference of {:.1} uu from floor (max recommended 64 uu).",
                            exit.name, assigned_key, vert_diff
                        ),
                        suggested_fix: format!(
                            "Adjust LiftExit Z to match Keyframe {} Z ({:.0} uu).",
                            assigned_key, target_floor.position.z
                        ),
                    });
                }
            }
        }

        // Check for LethalPlungeHazard (large drop > 500 uu between floors)
        if mover.keyframes.len() >= 2 {
            let z_min = mover.keyframes.iter().map(|k| k.position.z).fold(f32::INFINITY, f32::min);
            let z_max = mover.keyframes.iter().map(|k| k.position.z).fold(f32::NEG_INFINITY, f32::max);
            let drop = z_max - z_min;
            if drop > 500.0 && matching_exits.len() < mover.keyframes.len() {
                diagnostics.push(LiftDiagnostic {
                    severity: LiftDiagnosticSeverity::CriticalHazard,
                    category: LiftDiagnosticCategory::LethalPlungeHazard,
                    mover_tag: mover.tag.clone(),
                    mover_name: mover.name.clone(),
                    actor_name: mover.name.clone(),
                    location: base_pos,
                    message: format!(
                        "Elevator has a {:.0} uu vertical drop and missing exits. Bots may walk off into the abyss.",
                        drop
                    ),
                    suggested_fix: "Ensure all floors have aligned LiftExits with SuggestedKeyFrame set correctly.".to_string(),
                });
            }
        }

        // 5. Trigger wiring analysis (for TriggerOpenTimed or similar initial states)
        let is_trigger_operated = mover.initial_state.to_ascii_lowercase().contains("trigger");
        let matching_triggers: Vec<LiftTriggerData> = triggers
            .iter()
            .filter(|t| !t.event.is_empty() && (t.event.eq_ignore_ascii_case(&mover.tag) || (center.is_some() && t.event.eq_ignore_ascii_case(&center.unwrap().lift_trigger))))
            .cloned()
            .collect();

        if is_trigger_operated && matching_triggers.is_empty() {
            diagnostics.push(LiftDiagnostic {
                severity: LiftDiagnosticSeverity::CriticalHazard,
                category: LiftDiagnosticCategory::UncallableLiftHazard,
                mover_tag: mover.tag.clone(),
                mover_name: mover.name.clone(),
                actor_name: mover.name.clone(),
                location: base_pos,
                message: format!(
                    "Mover '{}' has InitialState='{}' but no Trigger actor with Event='{}' was found.",
                    mover.name, mover.initial_state, mover.tag
                ),
                suggested_fix: "Add a Trigger actor near each LiftExit with Event set to the Mover's Tag.".to_string(),
            });
        }

        // Check for orphaned / mismatched tags on exits
        for exit in &matching_exits {
            if !exit.lift_tag.eq_ignore_ascii_case(&mover.tag) {
                diagnostics.push(LiftDiagnostic {
                    severity: LiftDiagnosticSeverity::CriticalHazard,
                    category: LiftDiagnosticCategory::MismatchedLiftTag,
                    mover_tag: mover.tag.clone(),
                    mover_name: mover.name.clone(),
                    actor_name: exit.name.clone(),
                    location: exit.location,
                    message: format!(
                        "LiftExit '{}' has LiftTag='{}' which does not match Mover.Tag='{}'.",
                        exit.name, exit.lift_tag, mover.tag
                    ),
                    suggested_fix: format!("Set LiftExit.LiftTag='{}'.", mover.tag),
                });
            }
        }

        // Calculate cycle time: (MoveTime * segments * 2) + (StayOpenTime * 2)
        let segments = num_keys.saturating_sub(1).max(1) as f32;
        let total_cycle_time = (mover.move_time * segments * 2.0) + (mover.stay_open_time * 2.0);

        let has_crit = diagnostics.iter().any(|d| d.severity == LiftDiagnosticSeverity::CriticalHazard);
        let is_sync = center.is_some() && matching_exits.len() >= num_keys && !has_crit;

        if has_crit {
            critical_count += 1;
        } else if !diagnostics.is_empty() {
            warning_count += 1;
        }

        if is_sync {
            synchronized_count += 1;
        }

        systems.push(LiftSystemProfile {
            mover_name: mover.name.clone(),
            mover_tag: mover.tag.clone(),
            initial_state: mover.initial_state.clone(),
            move_time: mover.move_time,
            stay_open_time: mover.stay_open_time,
            num_keys,
            keyframes: mover.keyframes.clone(),
            lift_center: center.cloned(),
            lift_exits: matching_exits,
            triggers: matching_triggers,
            floors,
            diagnostics,
            total_cycle_time,
            is_fully_synchronized: is_sync,
            platform_radius: mover.platform_radius,
        });
    }

    // Check for orphaned LiftCenters that don't match ANY mover
    for center in lift_centers {
        let matches_any = movers.iter().any(|m| !m.tag.is_empty() && m.tag.eq_ignore_ascii_case(&center.lift_tag));
        if !matches_any {
            warning_count += 1;
            // Add orphaned diagnostic to a dummy or first system, or record count
        }
    }

    ElevatorAuditSummary {
        total_movers: movers.len(),
        total_lift_centers: lift_centers.len(),
        total_lift_exits: lift_exits.len(),
        synchronized_systems_count: synchronized_count,
        critical_hazard_count: critical_count,
        warning_count,
        systems,
    }
}

/// Generates a perfectly synchronized set of LiftCenter and LiftExit navigation actors
/// for a given mover and its keyframe floor positions.
pub fn generate_synchronized_lift_setup(
    mover_name: &str,
    mover_tag: &str,
    keyframe_positions: &[Vec3],
    platform_radius: f32,
    exit_yaw_degrees: f32,
) -> GeneratedLiftSetup {
    let clean_tag = if mover_tag.is_empty() {
        format!("{}_Lift", mover_name)
    } else {
        mover_tag.to_string()
    };

    let base_pos = keyframe_positions.first().copied().unwrap_or(Vec3::ZERO);

    // Place LiftCenter directly on the platform deck (+20 uu Z offset)
    let lift_center = GeneratedLiftCenter {
        name: format!("LiftCenter_{}", clean_tag),
        lift_tag: clean_tag.clone(),
        lift_trigger: format!("{}_Trigger", clean_tag),
        location: base_pos + Vec3::new(0.0, 0.0, 20.0),
    };

    // Calculate exit offset vector based on exit_yaw_degrees
    let rad = exit_yaw_degrees.to_radians();
    let exit_dist = platform_radius.max(80.0) + 64.0; // Clear of platform edge
    let horiz_offset = Vec3::new(rad.cos() * exit_dist, rad.sin() * exit_dist, 0.0);

    let mut lift_exits = Vec::new();
    let mut triggers = Vec::new();

    for (idx, pos) in keyframe_positions.iter().enumerate() {
        let exit_loc = *pos + horiz_offset + Vec3::new(0.0, 0.0, 20.0);
        lift_exits.push(GeneratedLiftExit {
            name: format!("LiftExit_{}_{}", clean_tag, idx),
            lift_tag: clean_tag.clone(),
            suggested_key_frame: idx,
            key_frame: idx,
            location: exit_loc,
        });

        // Generate matching trigger nearby for each exit floor
        triggers.push(GeneratedTrigger {
            name: format!("Trigger_{}_{}", clean_tag, idx),
            event: clean_tag.clone(),
            location: exit_loc + Vec3::new(0.0, 0.0, 10.0),
        });
    }

    GeneratedLiftSetup {
        lift_center,
        lift_exits,
        triggers,
    }
}

/// Computes a continuous simulation of the elevator travel and bot riding cycle.
pub fn simulate_elevator_bot_ride(
    profile: &LiftSystemProfile,
    sample_count: usize,
) -> Vec<LiftSimulationStep> {
    if profile.keyframes.is_empty() || profile.total_cycle_time <= 0.0 {
        return Vec::new();
    }

    let samples = sample_count.clamp(16, 256);
    let total_time = profile.total_cycle_time.max(0.5);
    let num_keys = profile.keyframes.len();
    let move_time = profile.move_time.max(0.1);
    let stay_time = profile.stay_open_time.max(0.0);

    let key0_pos = profile.keyframes[0].position;
    let key_top_pos = profile.keyframes.last().map(|k| k.position).unwrap_or(key0_pos);

    // Bottom exit loc & Top exit loc
    let exit_bottom = profile
        .lift_exits
        .iter()
        .find(|e| e.suggested_key_frame == 0)
        .map(|e| e.location)
        .unwrap_or(key0_pos + Vec3::new(128.0, 0.0, 0.0));

    let exit_top = profile
        .lift_exits
        .iter()
        .find(|e| e.suggested_key_frame == num_keys.saturating_sub(1))
        .map(|e| e.location)
        .unwrap_or(key_top_pos + Vec3::new(128.0, 0.0, 0.0));

    // Timeline phases:
    // 0: [0.0 .. stay_time * 0.5] => Bot boards platform from bottom exit
    // 1: [stay_time * 0.5 .. stay_time * 0.5 + move_time] => Lift ascends to top
    // 2: [.. + stay_time * 0.5] => Bot exits to top floor
    // 3: [.. + stay_time * 0.5] => Waiting at top floor
    // 4: [.. + move_time] => Lift descends back to bottom
    // 5: [remaining] => Cycle resets

    let t_board = stay_time * 0.5;
    let t_ascend_end = t_board + move_time;
    let t_exit_end = t_ascend_end + (stay_time * 0.5);
    let t_descend_start = t_exit_end + stay_time;
    let t_descend_end = t_descend_start + move_time;

    let mut steps = Vec::with_capacity(samples);

    for i in 0..samples {
        let norm_t = (i as f32) / (samples as f32 - 1.0);
        let time = norm_t * total_time;

        let (mover_pos, bot_pos, bot_state, cur_key, tgt_key) = if time <= t_board {
            // Boarding at bottom
            let board_t = if t_board > 0.0 { (time / t_board).clamp(0.0, 1.0) } else { 1.0 };
            let bot_p = exit_bottom.lerp(key0_pos + Vec3::new(0.0, 0.0, 20.0), board_t);
            (key0_pos, bot_p, LiftBotSimulationState::BoardingPlatform, 0, 1)
        } else if time <= t_ascend_end {
            // Ascending
            let ascend_t = ((time - t_board) / move_time).clamp(0.0, 1.0);
            let smooth_t = 0.5 * (1.0 - (std::f32::consts::PI * ascend_t).cos());
            let m_pos = key0_pos.lerp(key_top_pos, smooth_t);
            let bot_p = m_pos + Vec3::new(0.0, 0.0, 20.0);
            (m_pos, bot_p, LiftBotSimulationState::Ascending, 0, num_keys.saturating_sub(1))
        } else if time <= t_exit_end {
            // Arrived at top, stepping off
            let exit_t = if stay_time > 0.0 {
                ((time - t_ascend_end) / (stay_time * 0.5)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let bot_p = (key_top_pos + Vec3::new(0.0, 0.0, 20.0)).lerp(exit_top, exit_t);
            (key_top_pos, bot_p, LiftBotSimulationState::ExitingPlatform, num_keys.saturating_sub(1), num_keys.saturating_sub(1))
        } else if time <= t_descend_start {
            // Waiting at top
            (key_top_pos, exit_top, LiftBotSimulationState::WaitingAtTop, num_keys.saturating_sub(1), 0)
        } else if time <= t_descend_end {
            // Descending back empty
            let desc_t = ((time - t_descend_start) / move_time).clamp(0.0, 1.0);
            let smooth_t = 0.5 * (1.0 - (std::f32::consts::PI * desc_t).cos());
            let m_pos = key_top_pos.lerp(key0_pos, smooth_t);
            (m_pos, exit_top, LiftBotSimulationState::Descending, num_keys.saturating_sub(1), 0)
        } else {
            // Resting at bottom
            (key0_pos, exit_top, LiftBotSimulationState::WaitingAtBottom, 0, 0)
        };

        steps.push(LiftSimulationStep {
            time,
            mover_pos,
            mover_rot: Vec3::ZERO,
            current_keyframe: cur_key,
            target_keyframe: tgt_key,
            bot_pos,
            bot_state,
            bot_progress: norm_t,
        });
    }

    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_synchronized_two_floor_elevator() {
        let mover = ElevatorMoverData {
            name: "Mover0".to_string(),
            tag: "Lift1".to_string(),
            initial_state: "StandOpenTimed".to_string(),
            move_time: 2.5,
            stay_open_time: 2.0,
            num_keys: 2,
            keyframes: vec![
                MoverKeyframe { position: Vec3::new(0.0, 0.0, 0.0), rotation: Vec3::ZERO },
                MoverKeyframe { position: Vec3::new(0.0, 0.0, 512.0), rotation: Vec3::ZERO },
            ],
            platform_radius: 128.0,
        };

        let lift_center = LiftCenterData {
            name: "LiftCenter0".to_string(),
            lift_tag: "Lift1".to_string(),
            lift_trigger: "".to_string(),
            max_dist_2d: 128.0,
            location: Vec3::new(0.0, 0.0, 20.0),
        };

        let lift_exits = vec![
            LiftExitData {
                name: "LiftExit0".to_string(),
                lift_tag: "Lift1".to_string(),
                suggested_key_frame: 0,
                key_frame: 0,
                location: Vec3::new(160.0, 0.0, 20.0),
            },
            LiftExitData {
                name: "LiftExit1".to_string(),
                lift_tag: "Lift1".to_string(),
                suggested_key_frame: 1,
                key_frame: 1,
                location: Vec3::new(160.0, 0.0, 532.0),
            },
        ];

        let summary = audit_lift_systems(&[mover], &[lift_center], &lift_exits, &[]);

        assert_eq!(summary.total_movers, 1);
        assert_eq!(summary.total_lift_centers, 1);
        assert_eq!(summary.total_lift_exits, 2);
        assert_eq!(summary.synchronized_systems_count, 1);
        assert_eq!(summary.critical_hazard_count, 0);

        let sys = &summary.systems[0];
        assert!(sys.is_fully_synchronized);
        assert_eq!(sys.floors.len(), 2);
        assert!(sys.floors[0].is_aligned);
        assert!(sys.floors[1].is_aligned);
    }

    #[test]
    fn test_audit_detects_missing_lift_center_and_mismatches() {
        let mover = ElevatorMoverData {
            name: "ElevatorBroken".to_string(),
            tag: "LiftBrokenTag".to_string(),
            initial_state: "StandOpenTimed".to_string(),
            move_time: 0.05, // Glitch speed!
            stay_open_time: 1.0,
            num_keys: 2,
            keyframes: vec![
                MoverKeyframe { position: Vec3::new(100.0, 0.0, 0.0), rotation: Vec3::ZERO },
                MoverKeyframe { position: Vec3::new(100.0, 0.0, 800.0), rotation: Vec3::ZERO },
            ],
            platform_radius: 128.0,
        };

        let lift_exits = vec![
            LiftExitData {
                name: "ExitA".to_string(),
                lift_tag: "LiftBrokenTag".to_string(),
                suggested_key_frame: 0,
                key_frame: 0,
                location: Vec3::new(260.0, 0.0, 0.0),
            },
        ];

        let summary = audit_lift_systems(&[mover], &[], &lift_exits, &[]);

        assert_eq!(summary.synchronized_systems_count, 0);
        assert!(summary.critical_hazard_count > 0);

        let sys = &summary.systems[0];
        assert!(!sys.is_fully_synchronized);

        let has_speed_hazard = sys.diagnostics.iter().any(|d| d.category == LiftDiagnosticCategory::SubsecondGlitchSpeed);
        let has_missing_center = sys.diagnostics.iter().any(|d| d.category == LiftDiagnosticCategory::MissingLiftCenter);
        let has_missing_exit = sys.diagnostics.iter().any(|d| d.category == LiftDiagnosticCategory::MissingFloorExit);

        assert!(has_speed_hazard);
        assert!(has_missing_center);
        assert!(has_missing_exit);
    }

    #[test]
    fn test_generate_synchronized_lift_setup() {
        let keyframes = vec![
            Vec3::new(500.0, 200.0, 100.0),
            Vec3::new(500.0, 200.0, 700.0),
            Vec3::new(500.0, 200.0, 1200.0),
        ];

        let setup = generate_synchronized_lift_setup("LiftShaft", "ShaftMover", &keyframes, 128.0, 0.0);

        assert_eq!(setup.lift_center.lift_tag, "ShaftMover");
        assert_eq!(setup.lift_center.location, Vec3::new(500.0, 200.0, 120.0));
        assert_eq!(setup.lift_exits.len(), 3);

        for (i, exit) in setup.lift_exits.iter().enumerate() {
            assert_eq!(exit.suggested_key_frame, i);
            assert_eq!(exit.lift_tag, "ShaftMover");
            assert!(exit.location.x > 500.0); // Offset by platform_radius + 64
        }

        assert_eq!(setup.triggers.len(), 3);
    }

    #[test]
    fn test_simulate_elevator_bot_ride() {
        let mover = ElevatorMoverData {
            name: "MoverSim".to_string(),
            tag: "SimTag".to_string(),
            initial_state: "StandOpenTimed".to_string(),
            move_time: 2.0,
            stay_open_time: 1.0,
            num_keys: 2,
            keyframes: vec![
                MoverKeyframe { position: Vec3::new(0.0, 0.0, 0.0), rotation: Vec3::ZERO },
                MoverKeyframe { position: Vec3::new(0.0, 0.0, 400.0), rotation: Vec3::ZERO },
            ],
            platform_radius: 128.0,
        };

        let setup = generate_synchronized_lift_setup(&mover.name, &mover.tag, &[mover.keyframes[0].position, mover.keyframes[1].position], 128.0, 0.0);

        let center = LiftCenterData {
            name: setup.lift_center.name,
            lift_tag: setup.lift_center.lift_tag,
            lift_trigger: setup.lift_center.lift_trigger,
            max_dist_2d: 128.0,
            location: setup.lift_center.location,
        };

        let exits: Vec<LiftExitData> = setup.lift_exits.into_iter().map(|e| LiftExitData {
            name: e.name,
            lift_tag: e.lift_tag,
            suggested_key_frame: e.suggested_key_frame,
            key_frame: e.key_frame,
            location: e.location,
        }).collect();

        let summary = audit_lift_systems(&[mover], &[center], &exits, &[]);
        let steps = simulate_elevator_bot_ride(&summary.systems[0], 32);

        assert_eq!(steps.len(), 32);
        assert_eq!(steps[0].bot_state, LiftBotSimulationState::BoardingPlatform);
        assert!(steps.iter().any(|s| s.bot_state == LiftBotSimulationState::Ascending));
        assert!(steps.iter().any(|s| s.bot_state == LiftBotSimulationState::ExitingPlatform));
    }
}
