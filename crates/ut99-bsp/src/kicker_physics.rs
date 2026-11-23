// crates/ut99-bsp/src/kicker_physics.rs
//
// Analytical trajectory solver, inverse kinematics, bot reachability and hazard auditor
// for Unreal Tournament 99 UT_JumpPad, Kicker, and Low-Gravity aerial flight.

use glam::Vec3;

/// Archetype presets for common UT99 jump pad and kicker configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KickerArchetype {
    /// High-arc low-gravity skyscraper leap (DM-Morpheus style, g = -200 uu/s²)
    MorpheusSkyscraperAscent,
    /// High-angle parabolic lob onto sniper balcony (CTF-Face style)
    FaceTowerPerchLob,
    /// Long horizontal vault across deep space void (DM-Phobos style)
    PhobosChasmCrossover,
    /// Vertical high-velocity elevator shaft boost
    CoreVentShaftBoost,
    /// Short emergency hop over toxic slime pool (DM-Deck16][ style)
    Deck16SlimeEscapePad,
}

impl KickerArchetype {
    pub fn all() -> &'static [KickerArchetype] {
        &[
            Self::MorpheusSkyscraperAscent,
            Self::FaceTowerPerchLob,
            Self::PhobosChasmCrossover,
            Self::CoreVentShaftBoost,
            Self::Deck16SlimeEscapePad,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::MorpheusSkyscraperAscent => "DM-Morpheus Low-G Skyscraper Ascent",
            Self::FaceTowerPerchLob => "CTF-Face Sniper Tower Balcony Lob",
            Self::PhobosChasmCrossover => "DM-Phobos Deep Space Void Kicker",
            Self::CoreVentShaftBoost => "Core Ventilation Shaft Vertical Boost",
            Self::Deck16SlimeEscapePad => "DM-Deck16][ Toxic Slime Escape Pad",
        }
    }

    pub fn to_config(&self, pad_pos: Vec3) -> KickerConfig {
        match self {
            Self::MorpheusSkyscraperAscent => KickerConfig {
                name: "JumpPad_Morpheus0".to_string(),
                archetype: *self,
                pad_location: pad_pos,
                target_location: pad_pos + Vec3::new(800.0, 0.0, 450.0),
                kick_velocity: Vec3::new(450.0, 0.0, 650.0),
                gravity_z: -200.0, // Low-G
                air_control: 0.35,
                actor_class: "UT_JumpPad".to_string(),
                sound_kick: "Botpack.Pickups.ArmorUT".to_string(),
                auto_place_jump_spot: true,
                max_flight_time_secs: 8.0,
                ceiling_height_z: pad_pos.z + 1200.0,
            },
            Self::FaceTowerPerchLob => KickerConfig {
                name: "Kicker_FaceBalcony0".to_string(),
                archetype: *self,
                pad_location: pad_pos,
                target_location: pad_pos + Vec3::new(600.0, 0.0, 520.0),
                kick_velocity: Vec3::new(320.0, 0.0, 1150.0),
                gravity_z: -950.0, // Standard UT99 gravity
                air_control: 0.15,
                actor_class: "Kicker".to_string(),
                sound_kick: "UnrealShare.General.Special1".to_string(),
                auto_place_jump_spot: true,
                max_flight_time_secs: 5.0,
                ceiling_height_z: pad_pos.z + 1600.0,
            },
            Self::PhobosChasmCrossover => KickerConfig {
                name: "Kicker_PhobosVoid0".to_string(),
                archetype: *self,
                pad_location: pad_pos,
                target_location: pad_pos + Vec3::new(1250.0, 0.0, 80.0),
                kick_velocity: Vec3::new(950.0, 0.0, 680.0),
                gravity_z: -950.0,
                air_control: 0.10,
                actor_class: "Kicker".to_string(),
                sound_kick: "Botpack.Translocator.throw".to_string(),
                auto_place_jump_spot: true,
                max_flight_time_secs: 4.0,
                ceiling_height_z: pad_pos.z + 900.0,
            },
            Self::CoreVentShaftBoost => KickerConfig {
                name: "JumpPad_VentBoost0".to_string(),
                archetype: *self,
                pad_location: pad_pos,
                target_location: pad_pos + Vec3::new(0.0, 0.0, 780.0),
                kick_velocity: Vec3::new(0.0, 0.0, 1350.0),
                gravity_z: -950.0,
                air_control: 0.05,
                actor_class: "UT_JumpPad".to_string(),
                sound_kick: "UnrealShare.General.Boots1".to_string(),
                auto_place_jump_spot: true,
                max_flight_time_secs: 4.0,
                ceiling_height_z: pad_pos.z + 1200.0,
            },
            Self::Deck16SlimeEscapePad => KickerConfig {
                name: "JumpPad_SlimeEscape0".to_string(),
                archetype: *self,
                pad_location: pad_pos,
                target_location: pad_pos + Vec3::new(380.0, 0.0, 160.0),
                kick_velocity: Vec3::new(420.0, 0.0, 720.0),
                gravity_z: -950.0,
                air_control: 0.20,
                actor_class: "UT_JumpPad".to_string(),
                sound_kick: "Botpack.Pickups.ArmorUT".to_string(),
                auto_place_jump_spot: true,
                max_flight_time_secs: 3.0,
                ceiling_height_z: pad_pos.z + 600.0,
            },
        }
    }
}

/// Configuration parameters for a jump pad or kicker
#[derive(Debug, Clone, PartialEq)]
pub struct KickerConfig {
    pub name: String,
    pub archetype: KickerArchetype,
    pub pad_location: Vec3,
    pub target_location: Vec3,
    pub kick_velocity: Vec3,
    pub gravity_z: f32, // -950 uu/s² default, -200 for Low-G
    pub air_control: f32,
    pub actor_class: String, // "UT_JumpPad" or "Kicker"
    pub sound_kick: String,
    pub auto_place_jump_spot: bool,
    pub max_flight_time_secs: f32,
    pub ceiling_height_z: f32,
}

/// A discrete point along the aerial flight trajectory
#[derive(Debug, Clone, PartialEq)]
pub struct TrajectorySample {
    pub time_secs: f32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub is_apex: bool,
}

/// Analytical results of the aerial flight trajectory simulation
#[derive(Debug, Clone, PartialEq)]
pub struct TrajectoryAnalysis {
    pub samples: Vec<TrajectorySample>,
    pub apex_time_secs: f32,
    pub apex_position: Vec3,
    pub total_flight_time_secs: f32,
    pub landing_position: Vec3,
    pub landing_velocity_z: f32,
    pub horizontal_distance_uu: f32,
    pub vertical_delta_uu: f32,
    pub target_miss_distance_uu: f32,
    pub hit_ceiling: bool,
    pub ceiling_impact_point: Option<Vec3>,
    pub estimated_fall_damage: f32,
}

/// Hazard diagnostic severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KickerHazardSeverity {
    Critical,
    Warning,
    Info,
}

/// Individual issue found during kicker diagnostics
#[derive(Debug, Clone, PartialEq)]
pub struct KickerHazard {
    pub severity: KickerHazardSeverity,
    pub title: String,
    pub description: String,
    pub suggested_fix: String,
    pub location: Vec3,
}

/// Calculates the forward aerial trajectory for the kicker configuration
pub fn calculate_kicker_trajectory(config: &KickerConfig) -> TrajectoryAnalysis {
    let dt = 0.02_f32; // 50 Hz discrete simulation
    let g = config.gravity_z.min(-50.0); // Ensure downward gravity
    let mut pos = config.pad_location + Vec3::new(0.0, 0.0, 16.0); // Player feet offset
    let mut vel = config.kick_velocity;

    let mut samples = Vec::new();
    let mut time = 0.0_f32;
    let mut apex_time = 0.0_f32;
    let mut apex_pos = pos;
    let mut apex_found = false;
    let mut prev_vel_z = vel.z;

    let mut hit_ceiling = false;
    let mut ceiling_impact_point = None;

    // Push initial sample
    samples.push(TrajectorySample {
        time_secs: 0.0,
        position: pos,
        velocity: vel,
        is_apex: false,
    });

    // Target landing plane Z (relative to target location)
    let landing_z = config.target_location.z;

    while time < config.max_flight_time_secs {
        time += dt;
        let next_pos = pos + vel * dt + Vec3::new(0.0, 0.0, 0.5 * g * dt * dt);
        let next_vel = vel + Vec3::new(0.0, 0.0, g * dt);

        // Ceiling collision detection
        if !hit_ceiling && next_pos.z >= config.ceiling_height_z {
            hit_ceiling = true;
            ceiling_impact_point = Some(Vec3::new(next_pos.x, next_pos.y, config.ceiling_height_z));
        }

        // Apex detection (zero crossing of velocity Z)
        if !apex_found && prev_vel_z >= 0.0 && next_vel.z < 0.0 {
            apex_found = true;
            apex_time = time;
            apex_pos = next_pos;
            samples.push(TrajectorySample {
                time_secs: time,
                position: next_pos,
                velocity: next_vel,
                is_apex: true,
            });
        }

        pos = next_pos;
        vel = next_vel;
        prev_vel_z = vel.z;

        samples.push(TrajectorySample {
            time_secs: time,
            position: pos,
            velocity: vel,
            is_apex: false,
        });

        // Termination when crossing target landing elevation after apex
        if apex_found && pos.z <= landing_z {
            break;
        }

        // Failsafe for falling far beneath origin
        if pos.z < config.pad_location.z - 1500.0 {
            break;
        }
    }

    if !apex_found {
        apex_pos = pos;
        apex_time = time;
    }

    let landing_pos = pos;
    let landing_vel_z = vel.z;
    let horiz_dist = (landing_pos.truncate() - config.pad_location.truncate()).length();
    let vert_delta = landing_pos.z - config.pad_location.z;
    let target_miss = (landing_pos - config.target_location).length();

    // Fall damage in Unreal Tournament 99:
    // Terminal safe velocity is ~800 uu/s downward. Above that, damage scales linearly:
    // Damage = (downward_vel - 800) * 0.15
    let downward_speed = (-landing_vel_z).max(0.0);
    let fall_damage = if downward_speed > 800.0 {
        ((downward_speed - 800.0) * 0.15).min(1000.0)
    } else {
        0.0
    };

    TrajectoryAnalysis {
        samples,
        apex_time_secs: apex_time,
        apex_position: apex_pos,
        total_flight_time_secs: time,
        landing_position: landing_pos,
        landing_velocity_z: landing_vel_z,
        horizontal_distance_uu: horiz_dist,
        vertical_delta_uu: vert_delta,
        target_miss_distance_uu: target_miss,
        hit_ceiling,
        ceiling_impact_point,
        estimated_fall_damage: fall_damage,
    }
}

/// Solves the inverse problem: computes the required KickVelocity to hit target_pos exactly
/// with a designated apex clearance height above the highest point.
pub fn solve_kick_velocity_for_target(
    origin: Vec3,
    target: Vec3,
    apex_clearance_z: f32,
    gravity_z: f32,
) -> Vec3 {
    let g = gravity_z.abs().max(50.0); // positive magnitude of downward gravity
    let higher_z = origin.z.max(target.z);
    let apex_z = higher_z + apex_clearance_z.max(64.0);

    // Height from origin to apex
    let h1 = (apex_z - origin.z).max(16.0);
    // Vertical velocity needed to reach apex from origin: v_z = sqrt(2 * g * h1)
    let vz = (2.0 * g * h1).sqrt();

    // Time from origin to apex
    let t1 = vz / g;

    // Height from apex down to target
    let h2 = (apex_z - target.z).max(1.0);
    // Time from apex to target
    let t2 = (2.0 * h2 / g).sqrt();

    // Total flight duration
    let total_t = (t1 + t2).max(0.1);

    // Horizontal velocity components
    let delta_x = target.x - origin.x;
    let delta_y = target.y - origin.y;
    let vx = delta_x / total_t;
    let vy = delta_y / total_t;

    Vec3::new(vx, vy, vz)
}

/// Audits kicker configuration for potential map-breaking issues
pub fn audit_kicker_hazards(
    config: &KickerConfig,
    analysis: &TrajectoryAnalysis,
) -> Vec<KickerHazard> {
    let mut hazards = Vec::new();

    // 1. Ceiling Concussion Hazard
    if analysis.hit_ceiling {
        hazards.push(KickerHazard {
            severity: KickerHazardSeverity::Critical,
            title: "Ceiling Concussion Collision".to_string(),
            description: format!(
                "Trajectory intercepts ceiling at Z={:.1}, deflecting players and stalling bots midway.",
                config.ceiling_height_z
            ),
            suggested_fix: "Lower kick Z-velocity or carve out an overhead ceiling pocket.".to_string(),
            location: analysis.ceiling_impact_point.unwrap_or(analysis.apex_position),
        });
    }

    // 2. Lethal Fall Damage Hazard
    if analysis.estimated_fall_damage >= 100.0 {
        hazards.push(KickerHazard {
            severity: KickerHazardSeverity::Critical,
            title: "Lethal Landing Impact".to_string(),
            description: format!(
                "Downward speed of {:.0} uu/s causes {:.0} HP fall damage, instantly killing players without Boots of Jumping.",
                -analysis.landing_velocity_z, analysis.estimated_fall_damage
            ),
            suggested_fix: "Reduce vertical launch force or raise the landing platform elevation.".to_string(),
            location: analysis.landing_position,
        });
    } else if analysis.estimated_fall_damage > 15.0 {
        hazards.push(KickerHazard {
            severity: KickerHazardSeverity::Warning,
            title: "Heavy Landing Fall Damage".to_string(),
            description: format!(
                "Landing inflicts {:.0} HP fall damage on impact ({:.0} uu/s downward).",
                analysis.estimated_fall_damage, -analysis.landing_velocity_z
            ),
            suggested_fix: "Fine-tune trajectory arc to touch down softly on the platform.".to_string(),
            location: analysis.landing_position,
        });
    }

    // 3. Target Miss Discrepancy
    if analysis.target_miss_distance_uu > 120.0 {
        hazards.push(KickerHazard {
            severity: KickerHazardSeverity::Warning,
            title: "Landing Overshoot/Undershoot".to_string(),
            description: format!(
                "Trajectory misses target JumpSpot by {:.0} uu. Players or bots may tumble off ledge into hazards.",
                analysis.target_miss_distance_uu
            ),
            suggested_fix: "Click 'Auto-Solve Kick Velocity' to lock trajectory perfectly onto target platform.".to_string(),
            location: analysis.landing_position,
        });
    }

    // 4. Missing JumpSpot for Bot AI
    if !config.auto_place_jump_spot {
        hazards.push(KickerHazard {
            severity: KickerHazardSeverity::Warning,
            title: "Missing Companion JumpSpot".to_string(),
            description: "Without an associated JumpSpot actor, Bot AI reachspecs cannot route through this kicker.".to_string(),
            suggested_fix: "Enable 'Auto-Place Companion JumpSpot' in parameters.".to_string(),
            location: config.target_location,
        });
    }

    // 5. Excessive Flight Time
    if analysis.total_flight_time_secs > 6.0 {
        hazards.push(KickerHazard {
            severity: KickerHazardSeverity::Info,
            title: "Prolonged Aerial Hang-Time".to_string(),
            description: format!(
                "Flight duration is {:.2}s. Player will remain vulnerable in mid-air to sniper fire.",
                analysis.total_flight_time_secs
            ),
            suggested_fix: "Consider a flatter, faster trajectory curve for competitive pacing.".to_string(),
            location: analysis.apex_position,
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archetypes_and_defaults() {
        let archs = KickerArchetype::all();
        assert_eq!(archs.len(), 5);

        for &arch in archs {
            let pad_pos = Vec3::new(100.0, 200.0, 50.0);
            let cfg = arch.to_config(pad_pos);
            assert_eq!(cfg.pad_location, pad_pos);
            assert!(cfg.kick_velocity.length() > 100.0);
            assert!(cfg.gravity_z < 0.0);
        }
    }

    #[test]
    fn test_forward_trajectory_simulation() {
        let arch = KickerArchetype::FaceTowerPerchLob;
        let pad_pos = Vec3::new(0.0, 0.0, 0.0);
        let cfg = arch.to_config(pad_pos);

        let analysis = calculate_kicker_trajectory(&cfg);

        assert!(!analysis.samples.is_empty());
        assert!(analysis.apex_time_secs > 0.5);
        assert!(analysis.apex_position.z > 500.0);
        assert!(analysis.total_flight_time_secs > analysis.apex_time_secs);
        assert!(analysis.horizontal_distance_uu > 300.0);
    }

    #[test]
    fn test_inverse_solver_accuracy() {
        let origin = Vec3::new(100.0, 200.0, 50.0);
        let target = Vec3::new(850.0, 600.0, 320.0);
        let apex_clearance = 250.0;
        let gravity = -950.0;

        let solved_vel = solve_kick_velocity_for_target(origin, target, apex_clearance, gravity);
        assert!(solved_vel.z > 0.0);

        let mut config = KickerArchetype::PhobosChasmCrossover.to_config(origin);
        config.target_location = target;
        config.kick_velocity = solved_vel;
        config.gravity_z = gravity;
        config.ceiling_height_z = 2000.0;

        let analysis = calculate_kicker_trajectory(&config);

        // Target miss distance must be within a tiny tolerance (< 35 uu)
        assert!(
            analysis.target_miss_distance_uu < 35.0,
            "Target miss was {}",
            analysis.target_miss_distance_uu
        );
    }

    #[test]
    fn test_ceiling_concussion_and_fall_damage_audit() {
        let origin = Vec3::new(0.0, 0.0, 0.0);
        let mut cfg = KickerArchetype::CoreVentShaftBoost.to_config(origin);
        // Place ceiling very low to trigger concussion
        cfg.ceiling_height_z = 300.0;

        let analysis = calculate_kicker_trajectory(&cfg);
        assert!(analysis.hit_ceiling);

        let hazards = audit_kicker_hazards(&cfg, &analysis);
        let concussion_hazard = hazards.iter().find(|h| h.title.contains("Ceiling Concussion"));
        assert!(concussion_hazard.is_some());
        assert_eq!(concussion_hazard.unwrap().severity, KickerHazardSeverity::Critical);
    }
}
