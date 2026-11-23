// crates/ut99-bsp/src/soundscape.rs
//
// Interactive Ambient Soundscape, Spatial Audio Attenuation & Acoustic Reverb Studio
//
// Mathematical, geometric, and psychoacoustic model for Unreal Engine 1 3D spatial audio
// attenuation (SoundRadius, SoundVolume, SoundPitch), geometric falloff curves,
// solid BSP wall occlusion, and EAX environmental reverb zones.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical Unreal Tournament & Unreal 1 ambient soundscape archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundscapeArchetype {
    SubterraneanSewerEcho,
    IndustrialGeneratorDrone,
    CathedralGothicVault,
    HighTechComputerCore,
    OuterSpaceVacuumBreach,
}

impl SoundscapeArchetype {
    pub fn all() -> &'static [SoundscapeArchetype] {
        &[
            SoundscapeArchetype::SubterraneanSewerEcho,
            SoundscapeArchetype::IndustrialGeneratorDrone,
            SoundscapeArchetype::CathedralGothicVault,
            SoundscapeArchetype::HighTechComputerCore,
            SoundscapeArchetype::OuterSpaceVacuumBreach,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SoundscapeArchetype::SubterraneanSewerEcho => "Subterranean Sewer Water Echo (Damp & Muffled)",
            SoundscapeArchetype::IndustrialGeneratorDrone => "Industrial Generator Turbine Drone (High SPL)",
            SoundscapeArchetype::CathedralGothicVault => "Gothic Cathedral Stone Vault (Long EAX Reverb)",
            SoundscapeArchetype::HighTechComputerCore => "High-Tech Computer Server Core (Telemetry Beeps)",
            SoundscapeArchetype::OuterSpaceVacuumBreach => "Outer Space Vacuum Hull Breach (Sub-Bass)",
        }
    }

    pub fn default_config(&self) -> SoundscapeConfig {
        match self {
            SoundscapeArchetype::SubterraneanSewerEcho => SoundscapeConfig {
                archetype: *self,
                position: Vec3::ZERO,
                emitter: AmbientEmitterConfig {
                    sound_package: "AmbAncient".to_string(),
                    sound_name: "drip1".to_string(),
                    sound_radius: 32, // 32 * 40 = 1280 uu
                    sound_volume: 190,
                    sound_pitch: 64, // 1.0x normal
                    falloff_curve: AudioFalloffCurve::Linear,
                    enable_wall_occlusion: true,
                },
                acoustics: AcousticZoneConfig {
                    reverb_preset: AcousticReverbPreset::SewerPipe,
                    reverb_decay_time_secs: 2.8,
                    damping_hf: 0.65,
                    wall_transmission_loss_db: -12.0,
                },
                tag: "SewerDripAmbient01".to_string(),
                zone_tag: "SewerZoneInfo".to_string(),
            },
            SoundscapeArchetype::IndustrialGeneratorDrone => SoundscapeConfig {
                archetype: *self,
                position: Vec3::ZERO,
                emitter: AmbientEmitterConfig {
                    sound_package: "AmbModern".to_string(),
                    sound_name: "hum1".to_string(),
                    sound_radius: 64, // 64 * 40 = 2560 uu
                    sound_volume: 255, // Max volume
                    sound_pitch: 58,
                    falloff_curve: AudioFalloffCurve::InverseSquare,
                    enable_wall_occlusion: true,
                },
                acoustics: AcousticZoneConfig {
                    reverb_preset: AcousticReverbPreset::Hangar,
                    reverb_decay_time_secs: 3.5,
                    damping_hf: 0.40,
                    wall_transmission_loss_db: -15.0,
                },
                tag: "GeneratorHumAmbient01".to_string(),
                zone_tag: "GeneratorZoneInfo".to_string(),
            },
            SoundscapeArchetype::CathedralGothicVault => SoundscapeConfig {
                archetype: *self,
                position: Vec3::ZERO,
                emitter: AmbientEmitterConfig {
                    sound_package: "AmbAncient".to_string(),
                    sound_name: "wind1".to_string(),
                    sound_radius: 48, // 48 * 40 = 1920 uu
                    sound_volume: 160,
                    sound_pitch: 64,
                    falloff_curve: AudioFalloffCurve::Linear,
                    enable_wall_occlusion: true,
                },
                acoustics: AcousticZoneConfig {
                    reverb_preset: AcousticReverbPreset::Arena,
                    reverb_decay_time_secs: 5.2,
                    damping_hf: 0.25,
                    wall_transmission_loss_db: -10.0,
                },
                tag: "CathedralWindAmbient01".to_string(),
                zone_tag: "CathedralZoneInfo".to_string(),
            },
            SoundscapeArchetype::HighTechComputerCore => SoundscapeConfig {
                archetype: *self,
                position: Vec3::ZERO,
                emitter: AmbientEmitterConfig {
                    sound_package: "AmbModern".to_string(),
                    sound_name: "Computer3".to_string(),
                    sound_radius: 24, // 24 * 40 = 960 uu
                    sound_volume: 180,
                    sound_pitch: 72,
                    falloff_curve: AudioFalloffCurve::Linear,
                    enable_wall_occlusion: true,
                },
                acoustics: AcousticZoneConfig {
                    reverb_preset: AcousticReverbPreset::Livingroom,
                    reverb_decay_time_secs: 1.2,
                    damping_hf: 0.70,
                    wall_transmission_loss_db: -14.0,
                },
                tag: "ComputerBeepsAmbient01".to_string(),
                zone_tag: "ServerRoomZoneInfo".to_string(),
            },
            SoundscapeArchetype::OuterSpaceVacuumBreach => SoundscapeConfig {
                archetype: *self,
                position: Vec3::ZERO,
                emitter: AmbientEmitterConfig {
                    sound_package: "UnrealShare".to_string(),
                    sound_name: "Decompress1".to_string(),
                    sound_radius: 16, // 16 * 40 = 640 uu
                    sound_volume: 120,
                    sound_pitch: 42, // Low pitch rumble
                    falloff_curve: AudioFalloffCurve::InverseSquare,
                    enable_wall_occlusion: false,
                },
                acoustics: AcousticZoneConfig {
                    reverb_preset: AcousticReverbPreset::Plain,
                    reverb_decay_time_secs: 0.3,
                    damping_hf: 0.95,
                    wall_transmission_loss_db: -24.0,
                },
                tag: "VacuumRumbleAmbient01".to_string(),
                zone_tag: "VoidAirlessZoneInfo".to_string(),
            },
        }
    }
}

/// Canonical Unreal Engine 1 EAX environmental reverb presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcousticReverbPreset {
    Generic,
    PaddedCell,
    Room,
    Bathroom,
    Livingroom,
    StoneRoom,
    Auditorium,
    ConcertHall,
    Cave,
    Arena,
    Hangar,
    CarpetedHallway,
    Hallway,
    StoneCorridor,
    Alley,
    Forest,
    City,
    Mountains,
    Quarry,
    Plain,
    ParkingLot,
    SewerPipe,
    Underwater,
    Drugged,
    Dizzy,
    Psychotic,
}

impl AcousticReverbPreset {
    pub fn all() -> &'static [AcousticReverbPreset] {
        &[
            AcousticReverbPreset::Generic,
            AcousticReverbPreset::PaddedCell,
            AcousticReverbPreset::Room,
            AcousticReverbPreset::Bathroom,
            AcousticReverbPreset::Livingroom,
            AcousticReverbPreset::StoneRoom,
            AcousticReverbPreset::Auditorium,
            AcousticReverbPreset::ConcertHall,
            AcousticReverbPreset::Cave,
            AcousticReverbPreset::Arena,
            AcousticReverbPreset::Hangar,
            AcousticReverbPreset::CarpetedHallway,
            AcousticReverbPreset::Hallway,
            AcousticReverbPreset::StoneCorridor,
            AcousticReverbPreset::Alley,
            AcousticReverbPreset::Forest,
            AcousticReverbPreset::City,
            AcousticReverbPreset::Mountains,
            AcousticReverbPreset::Quarry,
            AcousticReverbPreset::Plain,
            AcousticReverbPreset::ParkingLot,
            AcousticReverbPreset::SewerPipe,
            AcousticReverbPreset::Underwater,
            AcousticReverbPreset::Drugged,
            AcousticReverbPreset::Dizzy,
            AcousticReverbPreset::Psychotic,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AcousticReverbPreset::Generic => "Generic (Default)",
            AcousticReverbPreset::PaddedCell => "Padded Cell (Dead Room)",
            AcousticReverbPreset::Room => "Small Room",
            AcousticReverbPreset::Bathroom => "Tiled Bathroom (Bright Reflections)",
            AcousticReverbPreset::Livingroom => "Furnished Living Room",
            AcousticReverbPreset::StoneRoom => "Stone Chamber",
            AcousticReverbPreset::Auditorium => "Auditorium Hall",
            AcousticReverbPreset::ConcertHall => "Concert Hall",
            AcousticReverbPreset::Cave => "Subterranean Cave",
            AcousticReverbPreset::Arena => "Huge Arena",
            AcousticReverbPreset::Hangar => "Metallic Industrial Hangar",
            AcousticReverbPreset::CarpetedHallway => "Carpeted Hallway",
            AcousticReverbPreset::Hallway => "Standard Corridor",
            AcousticReverbPreset::StoneCorridor => "Stone Corridor",
            AcousticReverbPreset::Alley => "Urban Alley",
            AcousticReverbPreset::Forest => "Dense Forest",
            AcousticReverbPreset::City => "City Streets",
            AcousticReverbPreset::Mountains => "Mountain Echoes",
            AcousticReverbPreset::Quarry => "Stone Quarry",
            AcousticReverbPreset::Plain => "Open Plain (Zero Reverb)",
            AcousticReverbPreset::ParkingLot => "Enclosed Parking Lot",
            AcousticReverbPreset::SewerPipe => "Resonant Sewer Pipe",
            AcousticReverbPreset::Underwater => "Muffled Underwater",
            AcousticReverbPreset::Drugged => "Drugged / Warped",
            AcousticReverbPreset::Dizzy => "Dizzy / Spinning",
            AcousticReverbPreset::Psychotic => "Psychotic Disorientation",
        }
    }
}

/// Mathematical model of volume decay over distance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFalloffCurve {
    Linear,
    InverseSquare,
    LogarithmicDecibel,
}

impl AudioFalloffCurve {
    pub fn all() -> &'static [AudioFalloffCurve] {
        &[
            AudioFalloffCurve::Linear,
            AudioFalloffCurve::InverseSquare,
            AudioFalloffCurve::LogarithmicDecibel,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AudioFalloffCurve::Linear => "Linear Decay (UE1 Default)",
            AudioFalloffCurve::InverseSquare => "Inverse Square (Physical 1/r²)",
            AudioFalloffCurve::LogarithmicDecibel => "Logarithmic Decibel (Psychoacoustic)",
        }
    }
}

/// Ambient sound emitter specifications
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbientEmitterConfig {
    pub sound_package: String,
    pub sound_name: String,
    pub sound_radius: u8, // 1 SoundRadius = 40 uu in Unreal Engine 1
    pub sound_volume: u8, // 0..255 (255 = 100%)
    pub sound_pitch: u8,  // 32..128 (64 = 1.0x normal pitch)
    pub falloff_curve: AudioFalloffCurve,
    pub enable_wall_occlusion: bool,
}

/// Environmental EAX acoustic zone settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticZoneConfig {
    pub reverb_preset: AcousticReverbPreset,
    pub reverb_decay_time_secs: f32,
    pub damping_hf: f32,
    pub wall_transmission_loss_db: f32, // typically -10 to -20 dB
}

/// Complete configuration for Soundscape Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundscapeConfig {
    pub archetype: SoundscapeArchetype,
    pub position: Vec3,
    pub emitter: AmbientEmitterConfig,
    pub acoustics: AcousticZoneConfig,
    pub tag: String,
    pub zone_tag: String,
}

impl Default for SoundscapeConfig {
    fn default() -> Self {
        SoundscapeArchetype::SubterraneanSewerEcho.default_config()
    }
}

/// Spatial audio attenuation metrics
#[derive(Debug, Clone, PartialEq)]
pub struct SoundscapeMetrics {
    pub outer_radius_uu: f32,
    pub inner_radius_uu: f32,
    pub spl_decibels_at_1m: f32,
    pub volume_fraction: f32,
    pub pitch_multiplier: f32,
    pub wavelength_at_1khz_uu: f32,
}

/// Probe measurement for acoustic propagation at a specific listener location
#[derive(Debug, Clone, PartialEq)]
pub struct AcousticProbeSample {
    pub distance_uu: f32,
    pub raw_volume_fraction: f32,
    pub effective_spl_db: f32,
    pub is_occluded_by_wall: bool,
    pub low_pass_cutoff_hz: f32,
    pub reverb_wet_ratio: f32,
}

/// Diagnostic safety hazard for audio configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundscapeHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Calculates spatial attenuation metrics
pub fn calculate_soundscape_metrics(config: &SoundscapeConfig) -> SoundscapeMetrics {
    // In UE1, 1 SoundRadius = 40.0 uu
    let outer_r = (config.emitter.sound_radius as f32) * 40.0;
    let inner_r = outer_r * 0.25; // Inner 25% zone is at 100% volume
    let vol_frac = config.emitter.sound_volume as f32 / 255.0;
    let pitch_mult = config.emitter.sound_pitch as f32 / 64.0;

    // Approximate reference sound pressure level at 1m (~60 uu) in dB SPL
    let spl_1m = 40.0 + vol_frac * 50.0; // 40..90 dB SPL
    let speed_of_sound_uu_s = 14000.0; // ~340 m/s
    let wavelength = speed_of_sound_uu_s / (1000.0 * pitch_mult.max(0.2));

    SoundscapeMetrics {
        outer_radius_uu: outer_r,
        inner_radius_uu: inner_r,
        spl_decibels_at_1m: spl_1m,
        volume_fraction: vol_frac,
        pitch_multiplier: pitch_mult,
        wavelength_at_1khz_uu: wavelength,
    }
}

/// Samples sound attenuation, occlusion, and perceived volume at an arbitrary listener location
pub fn sample_sound_propagation(
    config: &SoundscapeConfig,
    listener_pos: Vec3,
    world_brushes: &[Brush],
) -> AcousticProbeSample {
    let metrics = calculate_soundscape_metrics(config);
    let dist = (listener_pos - config.position).length();

    let raw_vol = if dist <= metrics.inner_radius_uu {
        metrics.volume_fraction
    } else if dist >= metrics.outer_radius_uu {
        0.0
    } else {
        let t = (dist - metrics.inner_radius_uu) / (metrics.outer_radius_uu - metrics.inner_radius_uu).max(1.0);
        match config.emitter.falloff_curve {
            AudioFalloffCurve::Linear => metrics.volume_fraction * (1.0 - t),
            AudioFalloffCurve::InverseSquare => {
                let r_norm = (dist / metrics.inner_radius_uu.max(1.0)).max(1.0);
                (metrics.volume_fraction / (r_norm * r_norm)).clamp(0.0, 1.0)
            }
            AudioFalloffCurve::LogarithmicDecibel => {
                let db_attenuation = t * 40.0; // 0 to -40 dB
                metrics.volume_fraction * 10.0_f32.powf(-db_attenuation / 20.0)
            }
        }
    };

    // Check line-of-sight occlusion through solid world brushes
    let mut is_occluded = false;
    if config.emitter.enable_wall_occlusion && !world_brushes.is_empty() && dist > 10.0 {
        let ray_dir = (listener_pos - config.position).normalize_or_zero();
        for brush in world_brushes {
            for poly in &brush.polys {
                // Plane intersection test
                let n = poly.plane.normal;
                let denom = n.dot(ray_dir);
                if denom.abs() > 0.001 {
                    let p0 = poly.vertices[0];
                    let d = (p0 - config.position).dot(n) / denom;
                    if d > 16.0 && d < (dist - 16.0) {
                        is_occluded = true;
                        break;
                    }
                }
            }
            if is_occluded {
                break;
            }
        }
    }

    let effective_db = if raw_vol > 0.001 {
        let base_db = metrics.spl_decibels_at_1m + 20.0 * (raw_vol.clamp(0.001, 1.0)).log10();
        if is_occluded {
            base_db + config.acoustics.wall_transmission_loss_db
        } else {
            base_db
        }
    } else {
        0.0
    };

    let cutoff_hz = if is_occluded {
        800.0 // Muffled low-pass through wall
    } else {
        20000.0 // Full spectrum open air
    };

    let wet_ratio = (dist / metrics.outer_radius_uu.max(1.0) * 0.7).clamp(0.1, 0.85);

    AcousticProbeSample {
        distance_uu: dist,
        raw_volume_fraction: raw_vol,
        effective_spl_db: effective_db.max(0.0),
        is_occluded_by_wall: is_occluded,
        low_pass_cutoff_hz: cutoff_hz,
        reverb_wet_ratio: wet_ratio,
    }
}

/// Generates procedural 3D brush geometry for the acoustic zone boundary (`PF_SEMISOLID`)
pub fn generate_acoustic_volume_brush(
    config: &SoundscapeConfig,
    zone_half_extent: f32,
) -> Brush {
    let mut brush = Brush::new("Acoustic_Zone_Volume", CsgOperation::Add);
    let r = zone_half_extent.max(128.0);
    let p = config.position;

    // 8-corner cube representing the ZoneInfo acoustic room
    let corners = [
        p + Vec3::new(-r, -r, -r),
        p + Vec3::new(r, -r, -r),
        p + Vec3::new(r, r, -r),
        p + Vec3::new(-r, r, -r),
        p + Vec3::new(-r, -r, r),
        p + Vec3::new(r, -r, r),
        p + Vec3::new(r, r, r),
        p + Vec3::new(-r, r, r),
    ];

    let faces = [
        [0, 3, 2, 1], // Bottom (-Z)
        [4, 5, 6, 7], // Top (+Z)
        [0, 1, 5, 4], // Front (-Y)
        [2, 3, 7, 6], // Back (+Y)
        [3, 0, 4, 7], // Left (-X)
        [1, 2, 6, 5], // Right (+X)
    ];

    for f in faces {
        let pts = vec![corners[f[0]], corners[f[1]], corners[f[2]], corners[f[3]]];
        if let Some(mut poly) = Polygon::new(pts) {
            poly.texture = "Indus.Metal.RustyIron01".to_string();
            poly.flags |= 0x00200000; // PF_SEMISOLID
            brush.polys.push(poly);
        }
    }

    brush
}

/// Audits audio emitter and acoustic parameters against volume, range, and psychoacoustic safety
pub fn audit_soundscape_safety(
    config: &SoundscapeConfig,
    _world_brushes: &[Brush],
) -> Vec<SoundscapeHazard> {
    let mut hazards = Vec::new();

    // 1. Inaudible sound radius (< 8 = 320 uu)
    if config.emitter.sound_radius < 8 {
        hazards.push(SoundscapeHazard {
            is_critical: true,
            title: "Sub-Audible Sound Radius".to_string(),
            description: format!("SoundRadius is only {} ({} uu). Players will only hear this sound when practically standing inside the actor pivot.",
                config.emitter.sound_radius, config.emitter.sound_radius as usize * 40),
            recommendation: "Increase SoundRadius to at least 16..32 (640..1280 uu).".to_string(),
        });
    }

    // 2. Muted volume (volume == 0)
    if config.emitter.sound_volume == 0 {
        hazards.push(SoundscapeHazard {
            is_critical: true,
            title: "Zero Sound Volume (Muted)".to_string(),
            description: "SoundVolume is set to 0. The ambient sound will never produce audible sound pressure.".to_string(),
            recommendation: "Set SoundVolume to at least 128..255.".to_string(),
        });
    }

    // 3. Extreme pitch distortion (< 32 or > 120)
    if config.emitter.sound_pitch < 32 {
        hazards.push(SoundscapeHazard {
            is_critical: false,
            title: "Severe Low Pitch Rumble".to_string(),
            description: format!("SoundPitch is set to {} (pitch multiplier {:.2}x). May cause inaudible infrasound or subwoofer rumbling.",
                config.emitter.sound_pitch, config.emitter.sound_pitch as f32 / 64.0),
            recommendation: "Keep SoundPitch between 48 and 96 for natural sound playback.".to_string(),
        });
    } else if config.emitter.sound_pitch > 120 {
        hazards.push(SoundscapeHazard {
            is_critical: false,
            title: "High Pitch Artifacting".to_string(),
            description: format!("SoundPitch is set to {} (pitch multiplier {:.2}x). May cause screeching high-frequency aliasing.",
                config.emitter.sound_pitch, config.emitter.sound_pitch as f32 / 64.0),
            recommendation: "Keep SoundPitch between 48 and 96 for natural sound playback.".to_string(),
        });
    }

    // 4. Excessive decay time for small rooms (> 6.0s with small radius)
    if config.acoustics.reverb_decay_time_secs > 5.0 && config.emitter.sound_radius < 32 {
        hazards.push(SoundscapeHazard {
            is_critical: false,
            title: "Excessive Acoustic Reverb Tail".to_string(),
            description: format!("Reverb decay time is {:.1}s for a relatively small room ({} uu radius). Audio mudding and intelligibility loss will occur.",
                config.acoustics.reverb_decay_time_secs, config.emitter.sound_radius as usize * 40),
            recommendation: "Reduce Reverb Decay Time to 1.5..3.0s or choose Room/Hallway preset.".to_string(),
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soundscape_archetypes_and_defaults() {
        for &arch in SoundscapeArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.emitter.sound_radius >= 16);
            assert!(cfg.emitter.sound_volume > 0);
            assert!(cfg.emitter.sound_pitch >= 32);
        }
    }

    #[test]
    fn test_soundscape_metrics_calculation() {
        let cfg = SoundscapeArchetype::SubterraneanSewerEcho.default_config();
        let m = calculate_soundscape_metrics(&cfg);
        assert_eq!(m.outer_radius_uu, 32.0 * 40.0); // 1280 uu
        assert_eq!(m.inner_radius_uu, 1280.0 * 0.25); // 320 uu
        assert!(m.spl_decibels_at_1m > 60.0);
    }

    #[test]
    fn test_sample_sound_propagation_attenuation() {
        let cfg = SoundscapeArchetype::IndustrialGeneratorDrone.default_config();

        // At pivot: 100% volume
        let s0 = sample_sound_propagation(&cfg, cfg.position, &[]);
        assert_eq!(s0.distance_uu, 0.0);
        assert_eq!(s0.raw_volume_fraction, 1.0);
        assert!(s0.effective_spl_db > 80.0);

        // At beyond outer radius: 0 volume
        let far_pos = cfg.position + Vec3::new(3000.0, 0.0, 0.0);
        let s_far = sample_sound_propagation(&cfg, far_pos, &[]);
        assert_eq!(s_far.raw_volume_fraction, 0.0);
    }

    #[test]
    fn test_soundscape_safety_audit() {
        let mut cfg = SoundscapeConfig::default();
        cfg.emitter.sound_radius = 4; // Sub-audible
        cfg.emitter.sound_volume = 0; // Muted
        cfg.emitter.sound_pitch = 20; // Extreme low pitch

        let hazards = audit_soundscape_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Sub-Audible")));
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Zero Sound Volume")));
        assert!(hazards.iter().any(|h| !h.is_critical && h.title.contains("Low Pitch")));
    }
}
