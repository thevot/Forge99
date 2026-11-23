// crates/ut99-bsp/src/laser_grid.rs
//
// Interactive Security Laser Grid, Tripwire Alarm & Energy Barrier Studio
//
// Mathematical model for laser tripwires, criss-cross security grids, sweeping kinematic beams,
// crossed-quad ribbon geometry generation, wall-mount diode housings, alarm trigger volumes,
// and penetration/clearance safety auditing.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Classical Unreal Tournament & Unreal 1 security laser archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaserArchetype {
    SingleInfraredTripwire,
    CrissCrossSecurityMesh,
    SweepingPatrolBeam,
    PerimeterCircularGate,
    LethalInstagibCurtain,
}

impl LaserArchetype {
    pub fn all() -> &'static [LaserArchetype] {
        &[
            LaserArchetype::SingleInfraredTripwire,
            LaserArchetype::CrissCrossSecurityMesh,
            LaserArchetype::SweepingPatrolBeam,
            LaserArchetype::PerimeterCircularGate,
            LaserArchetype::LethalInstagibCurtain,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LaserArchetype::SingleInfraredTripwire => "Single Ankle-High Infrared Tripwire",
            LaserArchetype::CrissCrossSecurityMesh => "Criss-Cross Hallway Security Mesh",
            LaserArchetype::SweepingPatrolBeam => "Oscillating Vertical Patrol Laser Beam",
            LaserArchetype::PerimeterCircularGate => "Perimeter Circular Energy Barrier Ring",
            LaserArchetype::LethalInstagibCurtain => "High-Voltage Lethal Defense Curtain",
        }
    }

    pub fn default_config(&self) -> LaserGridConfig {
        match self {
            LaserArchetype::SingleInfraredTripwire => LaserGridConfig {
                archetype: *self,
                span_width: 256.0,
                span_height: 160.0,
                origin: Vec3::new(0.0, 0.0, 24.0), // 24 uu above ground (ankle-high)
                direction: Vec3::X,
                normal: Vec3::Y,
                beam_count: 1,
                layout_mode: LaserLayoutMode::SingleBeam,
                beam_radius: 2.0,
                beam_color: [1.0, 0.15, 0.15, 0.95], // Crimson Red
                pulse_frequency_hz: 2.0,
                sweep: LaserSweepKinematics {
                    is_sweeping: false,
                    sweep_amplitude: 0.0,
                    sweep_period_sec: 4.0,
                    sweep_axis: Vec3::Z,
                },
                emitter: LaserEmitterConfig {
                    diode_radius: 6.0,
                    diode_depth: 8.0,
                    has_housing_caps: true,
                    housing_texture: "Indus.Metal.DiodeCase01".to_string(),
                },
                trigger: LaserTriggerConfig {
                    trigger_radius: 18.0,
                    alarm_event: "Alarm_VaultIntrusion".to_string(),
                    damage_per_second: 0.0,
                    alarm_sound: "Botpack.Alarm.Siren01".to_string(),
                    is_silent_alarm: false,
                },
                laser_texture: "GenFX.Laser.RedBeam01".to_string(),
            },
            LaserArchetype::CrissCrossSecurityMesh => LaserGridConfig {
                archetype: *self,
                span_width: 256.0,
                span_height: 192.0,
                origin: Vec3::new(0.0, 0.0, 96.0),
                direction: Vec3::X,
                normal: Vec3::Y,
                beam_count: 6,
                layout_mode: LaserLayoutMode::CrissCrossMesh,
                beam_radius: 2.5,
                beam_color: [0.15, 0.95, 0.35, 0.95], // Emerald Security
                pulse_frequency_hz: 3.0,
                sweep: LaserSweepKinematics {
                    is_sweeping: false,
                    sweep_amplitude: 0.0,
                    sweep_period_sec: 4.0,
                    sweep_axis: Vec3::Z,
                },
                emitter: LaserEmitterConfig {
                    diode_radius: 8.0,
                    diode_depth: 10.0,
                    has_housing_caps: true,
                    housing_texture: "Indus.Metal.HexBezel01".to_string(),
                },
                trigger: LaserTriggerConfig {
                    trigger_radius: 20.0,
                    alarm_event: "Alarm_CorridorMeshBreached".to_string(),
                    damage_per_second: 35.0,
                    alarm_sound: "Botpack.Alarm.Klaxon02".to_string(),
                    is_silent_alarm: false,
                },
                laser_texture: "GenFX.Laser.GreenBeam01".to_string(),
            },
            LaserArchetype::SweepingPatrolBeam => LaserGridConfig {
                archetype: *self,
                span_width: 320.0,
                span_height: 224.0,
                origin: Vec3::new(0.0, 0.0, 112.0),
                direction: Vec3::X,
                normal: Vec3::Y,
                beam_count: 2,
                layout_mode: LaserLayoutMode::ParallelHorizontal,
                beam_radius: 3.0,
                beam_color: [1.0, 0.85, 0.15, 0.95], // Amber Warning
                pulse_frequency_hz: 4.0,
                sweep: LaserSweepKinematics {
                    is_sweeping: true,
                    sweep_amplitude: 64.0,
                    sweep_period_sec: 5.0,
                    sweep_axis: Vec3::Z,
                },
                emitter: LaserEmitterConfig {
                    diode_radius: 10.0,
                    diode_depth: 12.0,
                    has_housing_caps: true,
                    housing_texture: "Indus.Metal.ServoMount01".to_string(),
                },
                trigger: LaserTriggerConfig {
                    trigger_radius: 24.0,
                    alarm_event: "Alarm_PatrolBeamTripped".to_string(),
                    damage_per_second: 50.0,
                    alarm_sound: "Botpack.Alarm.AlertTone01".to_string(),
                    is_silent_alarm: false,
                },
                laser_texture: "GenFX.Laser.YellowBeam01".to_string(),
            },
            LaserArchetype::PerimeterCircularGate => LaserGridConfig {
                archetype: *self,
                span_width: 256.0,
                span_height: 256.0,
                origin: Vec3::new(0.0, 0.0, 128.0),
                direction: Vec3::X,
                normal: Vec3::Y,
                beam_count: 8,
                layout_mode: LaserLayoutMode::RadialSpokes,
                beam_radius: 3.0,
                beam_color: [0.20, 0.85, 1.00, 0.95], // Cyan Energy
                pulse_frequency_hz: 5.0,
                sweep: LaserSweepKinematics {
                    is_sweeping: false,
                    sweep_amplitude: 0.0,
                    sweep_period_sec: 6.0,
                    sweep_axis: Vec3::X,
                },
                emitter: LaserEmitterConfig {
                    diode_radius: 12.0,
                    diode_depth: 16.0,
                    has_housing_caps: true,
                    housing_texture: "Indus.Metal.RingDiode01".to_string(),
                },
                trigger: LaserTriggerConfig {
                    trigger_radius: 28.0,
                    alarm_event: "Alarm_GateBreached".to_string(),
                    damage_per_second: 100.0,
                    alarm_sound: "Botpack.Alarm.EnergySpike01".to_string(),
                    is_silent_alarm: false,
                },
                laser_texture: "GenFX.Laser.CyanEnergy01".to_string(),
            },
            LaserArchetype::LethalInstagibCurtain => LaserGridConfig {
                archetype: *self,
                span_width: 256.0,
                span_height: 224.0,
                origin: Vec3::new(0.0, 0.0, 112.0),
                direction: Vec3::X,
                normal: Vec3::Y,
                beam_count: 5,
                layout_mode: LaserLayoutMode::ParallelHorizontal,
                beam_radius: 4.5,
                beam_color: [0.95, 0.10, 0.90, 0.95], // Ultraviolet Lethal
                pulse_frequency_hz: 8.0,
                sweep: LaserSweepKinematics {
                    is_sweeping: false,
                    sweep_amplitude: 0.0,
                    sweep_period_sec: 4.0,
                    sweep_axis: Vec3::Z,
                },
                emitter: LaserEmitterConfig {
                    diode_radius: 14.0,
                    diode_depth: 18.0,
                    has_housing_caps: true,
                    housing_texture: "Indus.Metal.HeavyContainment01".to_string(),
                },
                trigger: LaserTriggerConfig {
                    trigger_radius: 32.0,
                    alarm_event: "Alarm_LethalCurtainTripped".to_string(),
                    damage_per_second: 1000.0, // Instagib lethal
                    alarm_sound: "Botpack.Alarm.LethalSiren01".to_string(),
                    is_silent_alarm: false,
                },
                laser_texture: "GenFX.Laser.VioletDischarge01".to_string(),
            },
        }
    }
}

/// Geometric beam arrangement layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaserLayoutMode {
    SingleBeam,
    ParallelHorizontal,
    CrissCrossMesh,
    RadialSpokes,
}

impl LaserLayoutMode {
    pub fn all() -> &'static [LaserLayoutMode] {
        &[
            LaserLayoutMode::SingleBeam,
            LaserLayoutMode::ParallelHorizontal,
            LaserLayoutMode::CrissCrossMesh,
            LaserLayoutMode::RadialSpokes,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LaserLayoutMode::SingleBeam => "Single Beam",
            LaserLayoutMode::ParallelHorizontal => "Parallel Horizontal Slices",
            LaserLayoutMode::CrissCrossMesh => "Criss-Cross Diagonal Mesh",
            LaserLayoutMode::RadialSpokes => "Radial Circular Spokes",
        }
    }
}

/// Oscillating / sweeping kinematics parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaserSweepKinematics {
    pub is_sweeping: bool,
    pub sweep_amplitude: f32,
    pub sweep_period_sec: f32,
    pub sweep_axis: Vec3,
}

/// Diode emitter and receiver mounting parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaserEmitterConfig {
    pub diode_radius: f32,
    pub diode_depth: f32,
    pub has_housing_caps: bool,
    pub housing_texture: String,
}

/// Alarm trigger and damage properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaserTriggerConfig {
    pub trigger_radius: f32,
    pub alarm_event: String,
    pub damage_per_second: f32,
    pub alarm_sound: String,
    pub is_silent_alarm: bool,
}

/// Complete configuration for Security Laser Grid Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaserGridConfig {
    pub archetype: LaserArchetype,
    pub span_width: f32,
    pub span_height: f32,
    pub origin: Vec3,
    pub direction: Vec3,
    pub normal: Vec3,
    pub beam_count: usize,
    pub layout_mode: LaserLayoutMode,
    pub beam_radius: f32,
    pub beam_color: [f32; 4],
    pub pulse_frequency_hz: f32,
    pub sweep: LaserSweepKinematics,
    pub emitter: LaserEmitterConfig,
    pub trigger: LaserTriggerConfig,
    pub laser_texture: String,
}

impl Default for LaserGridConfig {
    fn default() -> Self {
        LaserArchetype::SingleInfraredTripwire.default_config()
    }
}

/// Spatial representation of an individual active laser beam
#[derive(Debug, Clone, PartialEq)]
pub struct LaserBeamData {
    pub index: usize,
    pub start_point: Vec3,
    pub end_point: Vec3,
    pub length: f32,
    pub beam_radius: f32,
    pub color: [f32; 4],
    pub is_lethal: bool,
}

/// Bounding specification for an Engine.Trigger actor aligned with a beam
#[derive(Debug, Clone, PartialEq)]
pub struct BeamTriggerSpec {
    pub name: String,
    pub location: Vec3,
    pub collision_radius: f32,
    pub collision_height: f32,
    pub event: String,
    pub damage_per_sec: f32,
}

/// Diagnostic safety hazard detected during laser grid audit
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaserHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Generates all active laser beam line segments at simulation time `time_sec`
pub fn generate_laser_beams(config: &LaserGridConfig, time_sec: f32) -> Vec<LaserBeamData> {
    let mut beams = Vec::new();
    let half_w = config.span_width * 0.5;
    let half_h = config.span_height * 0.5;

    let dir = config.direction.normalize_or_zero();
    let up = Vec3::Z;

    // Kinematic sweep offset
    let sweep_offset = if config.sweep.is_sweeping {
        let period = config.sweep.sweep_period_sec.max(0.5);
        let wave = (time_sec * std::f32::consts::TAU / period).sin();
        config.sweep.sweep_axis.normalize_or_zero() * (wave * config.sweep.sweep_amplitude)
    } else {
        Vec3::ZERO
    };

    match config.layout_mode {
        LaserLayoutMode::SingleBeam => {
            let start = config.origin - dir * half_w + sweep_offset;
            let end = config.origin + dir * half_w + sweep_offset;
            beams.push(LaserBeamData {
                index: 0,
                start_point: start,
                end_point: end,
                length: start.distance(end),
                beam_radius: config.beam_radius,
                color: config.beam_color,
                is_lethal: config.trigger.damage_per_second >= 100.0,
            });
        }
        LaserLayoutMode::ParallelHorizontal => {
            let count = config.beam_count.max(1);
            let step = if count > 1 {
                config.span_height / (count - 1) as f32
            } else {
                0.0
            };

            for i in 0..count {
                let z_rel = -half_h + i as f32 * step;
                let center = config.origin + up * z_rel + sweep_offset;
                let start = center - dir * half_w;
                let end = center + dir * half_w;
                beams.push(LaserBeamData {
                    index: i,
                    start_point: start,
                    end_point: end,
                    length: start.distance(end),
                    beam_radius: config.beam_radius,
                    color: config.beam_color,
                    is_lethal: config.trigger.damage_per_second >= 100.0,
                });
            }
        }
        LaserLayoutMode::CrissCrossMesh => {
            let count = config.beam_count.max(2);
            let half_count = count / 2;
            let step = config.span_height / half_count.max(1) as f32;

            let mut idx = 0;
            for i in 0..half_count {
                let z_a = -half_h + i as f32 * step;
                let z_b = z_a + step;

                // Diagonal /
                let start1 = config.origin - dir * half_w + up * z_a + sweep_offset;
                let end1 = config.origin + dir * half_w + up * z_b + sweep_offset;
                beams.push(LaserBeamData {
                    index: idx,
                    start_point: start1,
                    end_point: end1,
                    length: start1.distance(end1),
                    beam_radius: config.beam_radius,
                    color: config.beam_color,
                    is_lethal: config.trigger.damage_per_second >= 100.0,
                });
                idx += 1;

                // Diagonal \
                let start2 = config.origin - dir * half_w + up * z_b + sweep_offset;
                let end2 = config.origin + dir * half_w + up * z_a + sweep_offset;
                beams.push(LaserBeamData {
                    index: idx,
                    start_point: start2,
                    end_point: end2,
                    length: start2.distance(end2),
                    beam_radius: config.beam_radius,
                    color: config.beam_color,
                    is_lethal: config.trigger.damage_per_second >= 100.0,
                });
                idx += 1;
            }
        }
        LaserLayoutMode::RadialSpokes => {
            let count = config.beam_count.max(3);
            let radius = half_w.min(half_h);

            for i in 0..count {
                let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                let spoke_dir = dir * angle.cos() + up * angle.sin();

                let start = config.origin + sweep_offset;
                let end = config.origin + spoke_dir * radius + sweep_offset;
                beams.push(LaserBeamData {
                    index: i,
                    start_point: start,
                    end_point: end,
                    length: start.distance(end),
                    beam_radius: config.beam_radius,
                    color: config.beam_color,
                    is_lethal: config.trigger.damage_per_second >= 100.0,
                });
            }
        }
    }

    beams
}

/// Generates 3D polygonal geometry for:
/// 1. Crossed-quad translucent unlit laser ribbon brushes (`PF_TRANSLUCENT | PF_UNLIT | PF_NOT_SOLID`)
/// 2. Wall-mounted diode housing prisms (`PF_SEMISOLID`)
pub fn generate_laser_brush_geometry(
    config: &LaserGridConfig,
    beams: &[LaserBeamData],
) -> (Brush, Brush) {
    let mut beam_brush = Brush::new("Laser_Beams", CsgOperation::Add);
    let mut diode_brush = Brush::new("Laser_Diodes", CsgOperation::Add);

    let norm = config.normal.normalize_or_zero();
    let up = Vec3::Z;

    // 1. Generate Crossed-Quad Laser Ribbons
    for beam in beams {
        let start = beam.start_point;
        let end = beam.end_point;
        let r = beam.beam_radius.max(1.0);

        // Quad 1: ribbon perpendicular to Normal
        let offset1 = norm * r;
        let p0 = start - offset1;
        let p1 = start + offset1;
        let p2 = end + offset1;
        let p3 = end - offset1;

        if let Some(mut poly1) = Polygon::new(vec![p0, p1, p2, p3]) {
            poly1.texture = config.laser_texture.clone();
            // PF_TRANSLUCENT (0x00000004) | PF_UNLIT (0x00400000) | PF_NOT_SOLID (0x00000008) | PF_TWO_SIDED (0x00000100)
            poly1.flags |= 0x0040010C;
            beam_brush.polys.push(poly1);
        }

        // Quad 2: ribbon perpendicular to Up
        let offset2 = up * r;
        let q0 = start - offset2;
        let q1 = start + offset2;
        let q2 = end + offset2;
        let q3 = end - offset2;

        if let Some(mut poly2) = Polygon::new(vec![q0, q1, q2, q3]) {
            poly2.texture = config.laser_texture.clone();
            poly2.flags |= 0x0040010C;
            beam_brush.polys.push(poly2);
        }

        // 2. Generate Emitter & Reflector Diode Boxes at endpoints
        if config.emitter.has_housing_caps {
            let dr = config.emitter.diode_radius;
            let dd = config.emitter.diode_depth;

            // Diode at start
            generate_diode_box(&mut diode_brush, start, dr, dd, &config.emitter.housing_texture);
            // Diode at end
            generate_diode_box(&mut diode_brush, end, dr, dd, &config.emitter.housing_texture);
        }
    }

    (beam_brush, diode_brush)
}

fn generate_diode_box(brush: &mut Brush, center: Vec3, r: f32, depth: f32, tex: &str) {
    let half_r = r * 0.5;
    let half_d = depth * 0.5;

    let v000 = center + Vec3::new(-half_r, -half_d, -half_r);
    let v001 = center + Vec3::new(-half_r, -half_d, half_r);
    let v010 = center + Vec3::new(-half_r, half_d, -half_r);
    let v011 = center + Vec3::new(-half_r, half_d, half_r);
    let v100 = center + Vec3::new(half_r, -half_d, -half_r);
    let v101 = center + Vec3::new(half_r, -half_d, half_r);
    let v110 = center + Vec3::new(half_r, half_d, -half_r);
    let v111 = center + Vec3::new(half_r, half_d, half_r);

    let faces = [
        vec![v010, v110, v111, v011], // Front
        vec![v100, v000, v001, v101], // Back
        vec![v000, v010, v011, v001], // Left
        vec![v110, v100, v101, v111], // Right
        vec![v001, v011, v111, v101], // Top
        vec![v010, v000, v100, v110], // Bottom
    ];

    for face in faces {
        if let Some(mut poly) = Polygon::new(face) {
            poly.texture = tex.to_string();
            poly.flags |= 0x00200000; // PF_SEMISOLID
            brush.polys.push(poly);
        }
    }
}

/// Generates touch trigger specifications aligned with the active laser beams
pub fn generate_beam_trigger_specs(
    config: &LaserGridConfig,
    beams: &[LaserBeamData],
) -> Vec<BeamTriggerSpec> {
    beams
        .iter()
        .map(|beam| {
            let mid = (beam.start_point + beam.end_point) * 0.5;
            let radius = (beam.length * 0.5).max(config.trigger.trigger_radius);
            let height = config.trigger.trigger_radius;

            BeamTriggerSpec {
                name: format!("LaserTrigger_{}", beam.index),
                location: mid,
                collision_radius: radius,
                collision_height: height,
                event: config.trigger.alarm_event.clone(),
                damage_per_sec: config.trigger.damage_per_second,
            }
        })
        .collect()
}

/// Audits laser grid configuration against level geometry and gameplay hazards
pub fn audit_laser_grid_safety(
    config: &LaserGridConfig,
    beams: &[LaserBeamData],
    _world_brushes: &[Brush],
) -> Vec<LaserHazard> {
    let mut hazards = Vec::new();

    // 1. Check for unbound alarm event
    if config.trigger.alarm_event.trim().is_empty() {
        hazards.push(LaserHazard {
            is_critical: true,
            title: "Unbound Laser Alarm Event".to_string(),
            description: "Laser tripwire has an empty alarm event. Tripping the beam will produce no in-game reaction.".to_string(),
            recommendation: "Assign a target alarm event name (e.g. 'Alarm_VaultIntrusion').".to_string(),
        });
    }

    // 2. Check for narrow trigger radius allowing players to slip through
    if config.trigger.trigger_radius < 12.0 {
        hazards.push(LaserHazard {
            is_critical: false,
            title: format!("Narrow Trigger Detection Radius ({:.1} uu)", config.trigger.trigger_radius),
            description: "Trigger radius is under 12 uu. High-speed dodges or crouching players might slip through undetected.".to_string(),
            recommendation: "Increase trigger radius to 18.0..24.0 uu for reliable tripwire detection.".to_string(),
        });
    }

    // 3. Check for excessive beam count (>32)
    if beams.len() > 32 {
        hazards.push(LaserHazard {
            is_critical: false,
            title: format!("Excessive Laser Beam Count ({})", beams.len()),
            description: "More than 32 translucent laser quads may cause rendering fillrate drops on legacy UT99 Glide/Software renderers.".to_string(),
            recommendation: "Keep beam count under 16 for optimal competitive multiplayer performance.".to_string(),
        });
    }

    // 4. Ankle-high crawl bypass clearance check
    if config.layout_mode == LaserLayoutMode::SingleBeam && config.origin.z > 36.0 && config.origin.z < 64.0 {
        hazards.push(LaserHazard {
            is_critical: false,
            title: format!("Crawl Bypass Vulnerability (Height {:.0} uu)", config.origin.z),
            description: "Beam is suspended at mid-height, allowing crouched players (height 24 uu) to crawl under without tripping.".to_string(),
            recommendation: "Lower beam to ankle-height (16..24 uu) or add secondary cross-beams.".to_string(),
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_laser_archetypes_and_defaults() {
        for &arch in LaserArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.span_width > 100.0);
            assert!(cfg.beam_radius >= 1.0);
            assert!(!cfg.trigger.alarm_event.is_empty());
            assert!(!cfg.laser_texture.is_empty());
        }
    }

    #[test]
    fn test_generate_beams_and_sweep_kinematics() {
        let mut cfg = LaserArchetype::SweepingPatrolBeam.default_config();
        cfg.sweep.is_sweeping = true;
        cfg.sweep.sweep_amplitude = 50.0;
        cfg.sweep.sweep_period_sec = 4.0;

        let beams_t0 = generate_laser_beams(&cfg, 0.0);
        let beams_t1 = generate_laser_beams(&cfg, 1.0); // Quarter period = peak sweep (+50 uu)
        let beams_t2 = generate_laser_beams(&cfg, 2.0); // Half period = zero sweep (0 uu)

        assert_eq!(beams_t0.len(), 2);
        assert_eq!(beams_t1.len(), 2);

        // At t=1.0 sin(2*pi*1/4) = 1.0 -> Z offset should be +50
        assert!((beams_t1[0].start_point.z - beams_t0[0].start_point.z - 50.0).abs() < 1e-2);
        assert!((beams_t2[0].start_point.z - beams_t0[0].start_point.z).abs() < 1e-2);
    }

    #[test]
    fn test_generate_laser_brush_geometry() {
        let cfg = LaserArchetype::SingleInfraredTripwire.default_config();
        let beams = generate_laser_beams(&cfg, 0.0);
        let (beam_brush, diode_brush) = generate_laser_brush_geometry(&cfg, &beams);

        // 1 beam -> 2 crossed quads
        assert_eq!(beam_brush.polys.len(), 2);
        // Translucent unlit flags
        assert_eq!(beam_brush.polys[0].flags & 0x00400000, 0x00400000); // PF_UNLIT
        assert_eq!(beam_brush.polys[0].flags & 0x00000004, 0x00000004); // PF_TRANSLUCENT

        // 2 diodes (start and end) -> 6 faces each = 12 polys
        assert_eq!(diode_brush.polys.len(), 12);
        // Semi-solid flags
        assert_eq!(diode_brush.polys[0].flags & 0x00200000, 0x00200000); // PF_SEMISOLID
    }

    #[test]
    fn test_laser_safety_audit() {
        let mut cfg = LaserGridConfig::default();
        cfg.trigger.alarm_event = String::new(); // Unbound
        cfg.trigger.trigger_radius = 8.0; // Narrow

        let beams = generate_laser_beams(&cfg, 0.0);
        let hazards = audit_laser_grid_safety(&cfg, &beams, &[]);

        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Unbound")));
        assert!(hazards.iter().any(|h| !h.is_critical && h.title.contains("Narrow")));
    }
}
