// crates/ut99-bsp/src/alarm_beacon_studio.rs
//
// Interactive Alarm Siren Beacon, Emergency Strobe & Warning Klaxon Studio
// Warning strobe beacons, rotating reflector rotors, emergency red alerts,
// acoustic siren decibels, event trigger wiring, and procedural beacon mount brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical archetypes of emergency alarm sirens and warning beacons in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlarmBeaconArchetype {
    NuclearReactorBreachStrobe,
    PerimeterSecurityRevolvingSiren,
    HazardDecompressionPulse,
    ContainmentFailureKlaxon,
    StealthInfiltrationSilentBeacon,
}

impl AlarmBeaconArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            AlarmBeaconArchetype::NuclearReactorBreachStrobe => "Nuclear Reactor Breach Strobe (AS-Overlord Core Alert)",
            AlarmBeaconArchetype::PerimeterSecurityRevolvingSiren => "Perimeter Security Revolving Siren (Base Defense Rotor)",
            AlarmBeaconArchetype::HazardDecompressionPulse => "Hazard Decompression Pulse (AS-HiSpeed / Vacuum Breach)",
            AlarmBeaconArchetype::ContainmentFailureKlaxon => "Containment Failure Klaxon (Industrial Double Beacon)",
            AlarmBeaconArchetype::StealthInfiltrationSilentBeacon => "Stealth Infiltration Silent Beacon (Covert Intermittent)",
        }
    }

    pub fn to_config(&self) -> AlarmBeaconConfig {
        match self {
            AlarmBeaconArchetype::NuclearReactorBreachStrobe => AlarmBeaconConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                light_radius_uu: 320.0,
                light_brightness: 240,
                light_hue: 0, // Pure Red
                light_saturation: 255,
                rotor_rpm: 0.0,
                strobe_frequency_hz: 6.0,
                sound_volume_db: 90.0,
                alarm_sound: "UnrealShare.RedAlert1".to_string(),
                tag: "ReactorBreachAlert0".to_string(),
                event: "BreachObjectiveComplete".to_string(),
            },
            AlarmBeaconArchetype::PerimeterSecurityRevolvingSiren => AlarmBeaconConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                light_radius_uu: 256.0,
                light_brightness: 220,
                light_hue: 28, // Amber Orange
                light_saturation: 240,
                rotor_rpm: 120.0, // 2 revs per second
                strobe_frequency_hz: 0.0,
                sound_volume_db: 85.0,
                alarm_sound: "UnrealShare.Alarm2".to_string(),
                tag: "PerimeterSiren0".to_string(),
                event: "PerimeterBreached".to_string(),
            },
            AlarmBeaconArchetype::HazardDecompressionPulse => AlarmBeaconConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                light_radius_uu: 280.0,
                light_brightness: 210,
                light_hue: 140, // Cyan Decompression
                light_saturation: 230,
                rotor_rpm: 0.0,
                strobe_frequency_hz: 2.5,
                sound_volume_db: 80.0,
                alarm_sound: "UnrealShare.Alarm3".to_string(),
                tag: "VacuumDecompressBeacon0".to_string(),
                event: "AirLockBlown".to_string(),
            },
            AlarmBeaconArchetype::ContainmentFailureKlaxon => AlarmBeaconConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                light_radius_uu: 360.0,
                light_brightness: 250,
                light_hue: 45, // Heavy Yellow
                light_saturation: 255,
                rotor_rpm: 90.0,
                strobe_frequency_hz: 1.5,
                sound_volume_db: 95.0,
                alarm_sound: "UnrealShare.Klaxon1".to_string(),
                tag: "ContainmentFailureAlert0".to_string(),
                event: "SlimeContainmentCompromised".to_string(),
            },
            AlarmBeaconArchetype::StealthInfiltrationSilentBeacon => AlarmBeaconConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                light_radius_uu: 160.0,
                light_brightness: 140,
                light_hue: 190, // Ultraviolet / Deep Blue
                light_saturation: 200,
                rotor_rpm: 0.0,
                strobe_frequency_hz: 1.0,
                sound_volume_db: 0.0, // Silent
                alarm_sound: "".to_string(),
                tag: "CovertInfiltrationBeacon0".to_string(),
                event: "InfiltratorSpotted".to_string(),
            },
        }
    }
}

/// Configuration settings for an alarm beacon and siren warning system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlarmBeaconConfig {
    pub archetype: AlarmBeaconArchetype,
    pub position: Vec3,
    pub light_radius_uu: f32,
    pub light_brightness: u8,
    pub light_hue: u8,
    pub light_saturation: u8,
    pub rotor_rpm: f32,
    pub strobe_frequency_hz: f32,
    pub sound_volume_db: f32,
    pub alarm_sound: String,
    pub tag: String,
    pub event: String,
}

impl Default for AlarmBeaconConfig {
    fn default() -> Self {
        AlarmBeaconArchetype::NuclearReactorBreachStrobe.to_config()
    }
}

/// Context actor record for evaluating alarm coverage
#[derive(Debug, Clone, PartialEq)]
pub struct LevelAlarmContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Telemetry metrics for an alarm siren beacon
#[derive(Debug, Clone, PartialEq)]
pub struct AlarmBeaconMetrics {
    pub photometric_reach_uu: f32,
    pub acoustic_audible_radius_uu: f32,
    pub flash_cycle_duration_secs: f32,
    pub total_alarms_in_level: usize,
}

/// Safety diagnostics for alarm beacon installations
#[derive(Debug, Clone, PartialEq)]
pub struct AlarmBeaconHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates illumination reach, acoustic propagation distance, and strobe period
pub fn evaluate_alarm_beacon_metrics(
    config: &AlarmBeaconConfig,
    peers: &[LevelAlarmContextActorRecord],
) -> AlarmBeaconMetrics {
    let photometric_reach = config.light_radius_uu * (config.light_brightness as f32 / 255.0);

    // Acoustic audible distance: standard inverse-square attenuation in UT99
    // 90 dB ~ 1800 uu audible radius, 0 dB = 0 uu
    let audible_dist = if config.sound_volume_db > 0.0 {
        config.sound_volume_db * 20.0
    } else {
        0.0
    };

    // Flash cycle duration
    let cycle_duration = if config.strobe_frequency_hz > 0.0 {
        1.0 / config.strobe_frequency_hz
    } else if config.rotor_rpm > 0.0 {
        60.0 / config.rotor_rpm
    } else {
        0.0
    };

    let mut alarms_count = 1;
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("alarm") || cls_lower.contains("siren") || cls_lower.contains("light") {
            alarms_count += 1;
        }
    }

    AlarmBeaconMetrics {
        photometric_reach_uu: photometric_reach,
        acoustic_audible_radius_uu: audible_dist,
        flash_cycle_duration_secs: cycle_duration,
        total_alarms_in_level: alarms_count,
    }
}

/// Audits safety for high-frequency strobe epilepsy risks, muted alarms, and unassigned events
pub fn audit_alarm_beacon_safety(
    config: &AlarmBeaconConfig,
    _peers: &[LevelAlarmContextActorRecord],
) -> Vec<AlarmBeaconHazard> {
    let mut hazards = Vec::new();

    // 1. High Frequency Strobe Flash (> 12 Hz epilepsy hazard)
    if config.strobe_frequency_hz > 12.0 {
        hazards.push(AlarmBeaconHazard {
            title: "Stroboscopic Photosensitivity Risk".to_string(),
            description: format!(
                "Strobe flashing frequency of {:.1} Hz exceeds accessibility thresholds (max 10-12 Hz) and may cause disorientation or trigger photosensitive epilepsy.",
                config.strobe_frequency_hz
            ),
            is_critical: true,
        });
    }

    // 2. Unassigned Activation Event (Dud alarm)
    if config.event.is_empty() {
        hazards.push(AlarmBeaconHazard {
            title: "Unlinked Alarm Activation Event".to_string(),
            description: "Alarm beacon has no Event configured. It will remain dormant unless triggered by level scripting.".to_string(),
            is_critical: false,
        });
    }

    // 3. Audible Klaxon without Audio File
    if config.sound_volume_db > 0.0 && config.alarm_sound.trim().is_empty() {
        hazards.push(AlarmBeaconHazard {
            title: "Missing Alarm Sound Asset".to_string(),
            description: format!(
                "Alarm volume is set to {:.0} dB, but no alarm_sound asset is assigned! No siren sound will play.",
                config.sound_volume_db
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural CSG mounting base and dome lens cap brush for the siren beacon
pub fn generate_alarm_beacon_brush(config: &AlarmBeaconConfig) -> Brush {
    let mut brush = Brush::new("Alarm_Beacon".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let base_radius = 24.0;
    let base_height = 8.0;
    let dome_radius = 16.0;
    let dome_height = 20.0;
    let segs = 8;

    // 1. Octagonal Mounting Base Ring (Z = -base_height to 0)
    for i in 0..segs {
        let a0 = (i as f32) / (segs as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32) / (segs as f32) * std::f32::consts::TAU;

        let b0 = Vec3::new(a0.cos() * base_radius, a0.sin() * base_radius, -base_height);
        let b1 = Vec3::new(a1.cos() * base_radius, a1.sin() * base_radius, -base_height);
        let t0 = Vec3::new(a0.cos() * base_radius, a0.sin() * base_radius, 0.0);
        let t1 = Vec3::new(a1.cos() * base_radius, a1.sin() * base_radius, 0.0);

        if let Some(mut poly) = Polygon::new(vec![b0, b1, t1, t0]) {
            poly.texture = "Indus1.Indus_B01".to_string();
            brush.polys.push(poly);
        }
    }

    // 2. Octagonal Faceted Dome Lens Cap (Z = 0 to dome_height)
    let apex = Vec3::new(0.0, 0.0, dome_height);
    for i in 0..segs {
        let a0 = (i as f32) / (segs as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32) / (segs as f32) * std::f32::consts::TAU;

        let p0 = Vec3::new(a0.cos() * dome_radius, a0.sin() * dome_radius, 0.0);
        let p1 = Vec3::new(a1.cos() * dome_radius, a1.sin() * dome_radius, 0.0);

        if let Some(mut poly) = Polygon::new(vec![p0, p1, apex]) {
            poly.texture = match config.light_hue {
                0..=20 | 240..=255 => "GenFX.Glass.RedGlass01".to_string(),
                21..=60 => "GenFX.Glass.YellowGlass01".to_string(),
                120..=160 => "GenFX.Glass.CyanGlass01".to_string(),
                _ => "GenFX.Glass.BlueGlass01".to_string(),
            };
            poly.flags = crate::PF_TRANSLUCENT | crate::PF_TWO_SIDED;
            brush.polys.push(poly);
        }
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alarm_beacon_archetypes_and_defaults() {
        let breach = AlarmBeaconArchetype::NuclearReactorBreachStrobe.to_config();
        assert_eq!(breach.light_hue, 0); // Red
        assert_eq!(breach.strobe_frequency_hz, 6.0);
        assert_eq!(breach.sound_volume_db, 90.0);

        let siren = AlarmBeaconArchetype::PerimeterSecurityRevolvingSiren.to_config();
        assert_eq!(siren.rotor_rpm, 120.0);
        assert_eq!(siren.light_hue, 28); // Amber
    }

    #[test]
    fn test_alarm_beacon_metrics_evaluation() {
        let cfg = AlarmBeaconConfig::default(); // 320 uu radius, 240 brightness, 90 dB
        let peers = vec![
            LevelAlarmContextActorRecord {
                name: "AlarmLight0".to_string(),
                class_name: "Engine.Light".to_string(),
                position: Vec3::new(400.0, 0.0, 0.0),
            }
        ];

        let metrics = evaluate_alarm_beacon_metrics(&cfg, &peers);
        assert_eq!(metrics.photometric_reach_uu, 320.0 * (240.0 / 255.0));
        assert_eq!(metrics.acoustic_audible_radius_uu, 90.0 * 20.0); // 1800 uu
        assert_eq!(metrics.flash_cycle_duration_secs, 1.0 / 6.0);
        assert_eq!(metrics.total_alarms_in_level, 2);
    }

    #[test]
    fn test_alarm_beacon_safety_hazard_high_strobe() {
        let mut cfg = AlarmBeaconConfig::default();
        cfg.strobe_frequency_hz = 16.0;

        let hazards = audit_alarm_beacon_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Stroboscopic Photosensitivity Risk")));
    }

    #[test]
    fn test_generate_alarm_beacon_brush() {
        let cfg = AlarmBeaconConfig::default();
        let brush = generate_alarm_beacon_brush(&cfg);

        assert_eq!(brush.name, "Alarm_Beacon");
        // 8 base wall quads + 8 dome lens triangles = 16 polys
        assert_eq!(brush.polys.len(), 16);
    }
}
