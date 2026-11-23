// crates/ut99-bsp/src/light_shaft.rs
//
// Interactive Volumetric Light Shaft, Window God Ray & Atmospheric Beam Studio
//
// Mathematical model for generating non-BSP-cutting volumetric light shafts,
// window god rays, parallel louver slats, optical attenuation, and ambient dust motes.

use crate::brush::Brush;
use crate::CsgOperation;
use crate::poly::{
    Polygon, PF_NOT_SOLID, PF_TRANSLUCENT, PF_TWO_SIDED, PF_UNLIT,
};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Classical Unreal Tournament & Unreal 1 atmospheric light shaft archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightShaftArchetype {
    CathedralStainedGlassBeam,
    IndustrialGratingSlats,
    SkylightConicalShaft,
    CaveFissureSunbeam,
    SearchlightVolumetricBeam,
}

impl LightShaftArchetype {
    pub fn all() -> &'static [LightShaftArchetype] {
        &[
            LightShaftArchetype::CathedralStainedGlassBeam,
            LightShaftArchetype::IndustrialGratingSlats,
            LightShaftArchetype::SkylightConicalShaft,
            LightShaftArchetype::CaveFissureSunbeam,
            LightShaftArchetype::SearchlightVolumetricBeam,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LightShaftArchetype::CathedralStainedGlassBeam => "Cathedral Stained Glass God Ray",
            LightShaftArchetype::IndustrialGratingSlats => "Industrial Grating & Louver Slats",
            LightShaftArchetype::SkylightConicalShaft => "Skylight Volumetric Conical Shaft",
            LightShaftArchetype::CaveFissureSunbeam => "Cave Fissure & Organic Sunbeam",
            LightShaftArchetype::SearchlightVolumetricBeam => "Searchlight & Collimated Spotlight",
        }
    }

    pub fn default_config(&self) -> LightShaftConfig {
        match self {
            LightShaftArchetype::CathedralStainedGlassBeam => LightShaftConfig {
                archetype: *self,
                beam_length: 768.0,
                aperture_width: 128.0,
                aperture_height: 256.0,
                divergence_angle_deg: 18.0,
                cross_section: LightShaftCrossSection::Pyramid4,
                slat_count: 5,
                has_core_hotspot: true,
                has_floor_light_disc: true,
                pitch_deg: -55.0,
                yaw_deg: 45.0,
                roll_deg: 0.0,
                color_rgb: [1.0, 0.85, 0.45], // Warm cathedral sunlight
                attenuation_factor: 1.2,
                texture_name: "GenFX.LightBeam.GodRay01".to_string(),
            },
            LightShaftArchetype::IndustrialGratingSlats => LightShaftConfig {
                archetype: *self,
                beam_length: 512.0,
                aperture_width: 192.0,
                aperture_height: 96.0,
                divergence_angle_deg: 12.0,
                cross_section: LightShaftCrossSection::ParallelSlats,
                slat_count: 7,
                has_core_hotspot: false,
                has_floor_light_disc: false,
                pitch_deg: -65.0,
                yaw_deg: 0.0,
                roll_deg: 0.0,
                color_rgb: [0.65, 0.85, 1.0], // Cold industrial fluorescent
                attenuation_factor: 1.5,
                texture_name: "GenFX.LightBeam.SlatRay01".to_string(),
            },
            LightShaftArchetype::SkylightConicalShaft => LightShaftConfig {
                archetype: *self,
                beam_length: 1024.0,
                aperture_width: 160.0,
                aperture_height: 160.0,
                divergence_angle_deg: 22.0,
                cross_section: LightShaftCrossSection::Octagonal8,
                slat_count: 6,
                has_core_hotspot: true,
                has_floor_light_disc: true,
                pitch_deg: -80.0,
                yaw_deg: 0.0,
                roll_deg: 0.0,
                color_rgb: [0.95, 0.98, 1.0], // Noon sky ambient white/blue
                attenuation_factor: 1.0,
                texture_name: "GenFX.LightBeam.ConeRay01".to_string(),
            },
            LightShaftArchetype::CaveFissureSunbeam => LightShaftConfig {
                archetype: *self,
                beam_length: 896.0,
                aperture_width: 80.0,
                aperture_height: 140.0,
                divergence_angle_deg: 26.0,
                cross_section: LightShaftCrossSection::Hexagonal6,
                slat_count: 4,
                has_core_hotspot: true,
                has_floor_light_disc: true,
                pitch_deg: -48.0,
                yaw_deg: 120.0,
                roll_deg: 15.0,
                color_rgb: [1.0, 0.72, 0.30], // Deep golden dusk amber
                attenuation_factor: 0.9,
                texture_name: "GenFX.LightBeam.DustyRay01".to_string(),
            },
            LightShaftArchetype::SearchlightVolumetricBeam => LightShaftConfig {
                archetype: *self,
                beam_length: 1536.0,
                aperture_width: 48.0,
                aperture_height: 48.0,
                divergence_angle_deg: 8.0,
                cross_section: LightShaftCrossSection::Dodecagonal12,
                slat_count: 8,
                has_core_hotspot: true,
                has_floor_light_disc: false,
                pitch_deg: -25.0,
                yaw_deg: 90.0,
                roll_deg: 0.0,
                color_rgb: [0.9, 0.95, 1.0], // High-intensity xenon
                attenuation_factor: 0.6,
                texture_name: "GenFX.LightBeam.SpotBeam01".to_string(),
            },
        }
    }
}

/// Geometric shape and cross-section of the light beam
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightShaftCrossSection {
    Pyramid4,
    Hexagonal6,
    Octagonal8,
    Dodecagonal12,
    ParallelSlats,
}

impl LightShaftCrossSection {
    pub fn all() -> &'static [LightShaftCrossSection] {
        &[
            LightShaftCrossSection::Pyramid4,
            LightShaftCrossSection::Hexagonal6,
            LightShaftCrossSection::Octagonal8,
            LightShaftCrossSection::Dodecagonal12,
            LightShaftCrossSection::ParallelSlats,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LightShaftCrossSection::Pyramid4 => "4-Sided Pyramid Frustum (Window)",
            LightShaftCrossSection::Hexagonal6 => "6-Sided Hexagonal Beam",
            LightShaftCrossSection::Octagonal8 => "8-Sided Cylindrical Cone",
            LightShaftCrossSection::Dodecagonal12 => "12-Sided High-Fidelity Cone",
            LightShaftCrossSection::ParallelSlats => "Parallel Slat Sheets (Grates)",
        }
    }

    pub fn side_count(&self) -> usize {
        match self {
            LightShaftCrossSection::Pyramid4 => 4,
            LightShaftCrossSection::Hexagonal6 => 6,
            LightShaftCrossSection::Octagonal8 => 8,
            LightShaftCrossSection::Dodecagonal12 => 12,
            LightShaftCrossSection::ParallelSlats => 2,
        }
    }
}

/// An ambient floating dust mote inside the volumetric beam
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DustMote {
    pub position: Vec3,
    pub brightness: f32,
    pub size: f32,
}

/// Complete configuration for the volumetric light shaft studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightShaftConfig {
    pub archetype: LightShaftArchetype,
    pub beam_length: f32,
    pub aperture_width: f32,
    pub aperture_height: f32,
    pub divergence_angle_deg: f32,
    pub cross_section: LightShaftCrossSection,
    pub slat_count: usize,
    pub has_core_hotspot: bool,
    pub has_floor_light_disc: bool,
    pub pitch_deg: f32,
    pub yaw_deg: f32,
    pub roll_deg: f32,
    pub color_rgb: [f32; 3],
    pub attenuation_factor: f32,
    pub texture_name: String,
}

impl Default for LightShaftConfig {
    fn default() -> Self {
        LightShaftArchetype::CathedralStainedGlassBeam.default_config()
    }
}

/// Diagnostic hazard or warning flagged during light shaft generation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightShaftHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Computes optical transmission along the beam axis using Beer-Lambert law
pub fn calculate_optical_transmission(config: &LightShaftConfig, distance_fraction: f32) -> f32 {
    let t = distance_fraction.clamp(0.0, 1.0);
    // Beer-Lambert exponential decay: T = exp(-mu * t)
    let decay = (-config.attenuation_factor * t).exp();
    // Fade out smoothly at the terminal ground plane
    decay * (1.0 - t * 0.4)
}

/// Generates the rotation quaternion from Pitch, Yaw, Roll angles
pub fn compute_aim_rotation(pitch_deg: f32, yaw_deg: f32, roll_deg: f32) -> Quat {
    let p = pitch_deg.to_radians();
    let y = yaw_deg.to_radians();
    let r = roll_deg.to_radians();
    Quat::from_euler(glam::EulerRot::YXZ, y, p, r)
}

/// Generates a volumetric light shaft brush with canonical UE1 translucent flags
pub fn generate_light_shaft_brush(config: &LightShaftConfig) -> Brush {
    let mut brush = Brush::new("LightShaft_Volumetric", CsgOperation::Add);
    let rot = compute_aim_rotation(config.pitch_deg, config.yaw_deg, config.roll_deg);
    let _forward = rot * Vec3::Z; // Beam projects along forward axis

    let spread_tan = (config.divergence_angle_deg.to_radians() * 0.5).tan();
    let spread_offset = config.beam_length * spread_tan;

    let base_w = config.aperture_width * 0.5 + spread_offset;
    let base_h = config.aperture_height * 0.5 + spread_offset;
    let apex_w = (config.aperture_width * 0.5).max(4.0);
    let apex_h = (config.aperture_height * 0.5).max(4.0);

    let beam_flags = PF_TRANSLUCENT | PF_UNLIT | PF_TWO_SIDED | PF_NOT_SOLID;

    match config.cross_section {
        LightShaftCrossSection::ParallelSlats => {
            // Generate multiple parallel vertical ray sheets
            let slats = config.slat_count.clamp(2, 16);
            for s in 0..slats {
                let frac = if slats > 1 {
                    (s as f32 / (slats - 1) as f32) - 0.5
                } else {
                    0.0
                };
                let x_apex = frac * config.aperture_width;
                let x_base = frac * (config.aperture_width + spread_offset * 2.0);

                let p0 = rot * Vec3::new(x_apex, -apex_h, 0.0);
                let p1 = rot * Vec3::new(x_apex, apex_h, 0.0);
                let p2 = rot * Vec3::new(x_base, base_h, config.beam_length);
                let p3 = rot * Vec3::new(x_base, -base_h, config.beam_length);

                if let Some(mut poly) = Polygon::new(vec![p0, p1, p2, p3]) {
                    poly.flags = beam_flags;
                    poly.texture = config.texture_name.clone();
                    brush.polys.push(poly);
                }
            }
        }
        _ => {
            // Radial polygonal cone / pyramid frustum
            let sides = config.cross_section.side_count().max(3);
            let mut apex_ring = Vec::with_capacity(sides);
            let mut base_ring = Vec::with_capacity(sides);

            for i in 0..sides {
                let angle = (i as f32 / sides as f32) * std::f32::consts::TAU;
                let ca = angle.cos();
                let sa = angle.sin();

                let p_apex = rot * Vec3::new(apex_w * ca, apex_h * sa, 0.0);
                let p_base = rot * Vec3::new(base_w * ca, base_h * sa, config.beam_length);

                apex_ring.push(p_apex);
                base_ring.push(p_base);
            }

            // Lateral boundary faces connecting apex to base
            for i in 0..sides {
                let next = (i + 1) % sides;
                let quad = vec![
                    apex_ring[i],
                    apex_ring[next],
                    base_ring[next],
                    base_ring[i],
                ];
                if let Some(mut poly) = Polygon::new(quad) {
                    poly.flags = beam_flags;
                    poly.texture = config.texture_name.clone();
                    brush.polys.push(poly);
                }
            }

            // Optional Core Hotspot Beam (50% scale inner cone with dual-layer bloom)
            if config.has_core_hotspot {
                let core_apex_w = apex_w * 0.45;
                let core_apex_h = apex_h * 0.45;
                let core_base_w = base_w * 0.45;
                let core_base_h = base_h * 0.45;

                for i in 0..sides {
                    let next = (i + 1) % sides;
                    let angle0 = (i as f32 / sides as f32) * std::f32::consts::TAU;
                    let angle1 = (next as f32 / sides as f32) * std::f32::consts::TAU;

                    let c0 = rot * Vec3::new(core_apex_w * angle0.cos(), core_apex_h * angle0.sin(), 0.0);
                    let c1 = rot * Vec3::new(core_apex_w * angle1.cos(), core_apex_h * angle1.sin(), 0.0);
                    let c2 = rot * Vec3::new(core_base_w * angle1.cos(), core_base_h * angle1.sin(), config.beam_length);
                    let c3 = rot * Vec3::new(core_base_w * angle0.cos(), core_base_h * angle0.sin(), config.beam_length);

                    if let Some(mut poly) = Polygon::new(vec![c0, c1, c2, c3]) {
                        poly.flags = beam_flags;
                        poly.texture = format!("{}_Core", config.texture_name);
                        brush.polys.push(poly);
                    }
                }
            }

            // Optional Floor Light Pool Disc
            if config.has_floor_light_disc {
                let mut reversed_base = base_ring.clone();
                reversed_base.reverse();
                if let Some(mut disc) = Polygon::new(reversed_base) {
                    disc.flags = beam_flags;
                    disc.texture = format!("{}_Pool", config.texture_name);
                    brush.polys.push(disc);
                }
            }
        }
    }

    brush
}

/// Generates ambient dust motes floating inside the beam frustum
pub fn generate_dust_motes(config: &LightShaftConfig, count: usize) -> Vec<DustMote> {
    let mut motes = Vec::with_capacity(count);
    let rot = compute_aim_rotation(config.pitch_deg, config.yaw_deg, config.roll_deg);
    let spread_tan = (config.divergence_angle_deg.to_radians() * 0.5).tan();

    // Deterministic pseudo-random distribution
    let mut seed: u64 = 0xbeef_5ca1;
    for i in 0..count {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r_z = ((seed >> 16) & 0xffff) as f32 / 65535.0;

        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r_angle = ((seed >> 16) & 0xffff) as f32 / 65535.0 * std::f32::consts::TAU;

        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r_radius = (((seed >> 16) & 0xffff) as f32 / 65535.0).sqrt();

        let z = r_z * config.beam_length;
        let spread = z * spread_tan;
        let cur_w = (config.aperture_width * 0.5 + spread) * r_radius;
        let cur_h = (config.aperture_height * 0.5 + spread) * r_radius;

        let local_pos = Vec3::new(cur_w * r_angle.cos(), cur_h * r_angle.sin(), z);
        let world_pos = rot * local_pos;

        let brightness = calculate_optical_transmission(config, r_z) * (0.6 + 0.4 * (i % 5) as f32 / 5.0);
        let size = 2.0 + (i % 3) as f32 * 1.5;

        motes.push(DustMote {
            position: world_pos,
            brightness,
            size,
        });
    }

    motes
}

/// Audits the light shaft configuration for potential level design hazards
pub fn audit_light_shaft_safety(config: &LightShaftConfig) -> Vec<LightShaftHazard> {
    let mut hazards = Vec::new();

    // 1. Extreme Beam Length Warning
    if config.beam_length > 4096.0 {
        hazards.push(LightShaftHazard {
            is_critical: false,
            title: "Excessive Beam Length (>4096 uu)".to_string(),
            description: format!("Beam length of {:.0} uu may puncture through exterior map boundaries or cause depth sorting artifacts.", config.beam_length),
            recommendation: "Clamp beam length to the distance to the floor or nearest architectural obstacle.".to_string(),
        });
    }

    // 2. Short Beam Hazard
    if config.beam_length < 64.0 {
        hazards.push(LightShaftHazard {
            is_critical: true,
            title: "Degenerate Beam Length (<64 uu)".to_string(),
            description: "The beam is too short to produce a visible volumetric light shaft.".to_string(),
            recommendation: "Increase beam length to at least 128.0 uu.".to_string(),
        });
    }

    // 3. Wide Divergence Angle Hazard
    if config.divergence_angle_deg > 60.0 {
        hazards.push(LightShaftHazard {
            is_critical: false,
            title: "Wide Divergence Angle (>60°)".to_string(),
            description: "Angles over 60° produce flared bell-shaped shafts that look unnatural and create heavy overdraw.".to_string(),
            recommendation: "Keep beam divergence between 10° and 35° for authentic architectural sunlight.".to_string(),
        });
    }

    // 4. Tiny Aperture Area Warning
    let aperture_area = config.aperture_width * config.aperture_height;
    if aperture_area < 32.0 {
        hazards.push(LightShaftHazard {
            is_critical: false,
            title: "Sub-Pixel Aperture Window Area".to_string(),
            description: "The light window aperture is nearly microscopic and may not be perceptible.".to_string(),
            recommendation: "Set aperture width and height to at least 32.0 uu.".to_string(),
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_shaft_archetypes_and_defaults() {
        for &arch in LightShaftArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.beam_length > 0.0);
            assert!(cfg.aperture_width > 0.0);
            assert!(cfg.aperture_height > 0.0);
            assert!(cfg.divergence_angle_deg > 0.0);
        }
    }

    #[test]
    fn test_generate_pyramid_light_shaft_mesh() {
        let mut cfg = LightShaftConfig::default();
        cfg.cross_section = LightShaftCrossSection::Pyramid4;
        cfg.has_core_hotspot = false;
        cfg.has_floor_light_disc = true;

        let brush = generate_light_shaft_brush(&cfg);
        // 4 lateral faces + 1 floor disc = 5 polys
        assert_eq!(brush.polys.len(), 5);

        for poly in &brush.polys {
            assert_ne!(poly.flags & PF_TRANSLUCENT, 0, "Must have PF_TRANSLUCENT");
            assert_ne!(poly.flags & PF_UNLIT, 0, "Must have PF_UNLIT");
            assert_ne!(poly.flags & PF_TWO_SIDED, 0, "Must have PF_TWO_SIDED");
            assert_ne!(poly.flags & PF_NOT_SOLID, 0, "Must have PF_NOT_SOLID to avoid blocking players");
        }
    }

    #[test]
    fn test_generate_parallel_slats_brush() {
        let mut cfg = LightShaftConfig::default();
        cfg.cross_section = LightShaftCrossSection::ParallelSlats;
        cfg.slat_count = 6;

        let brush = generate_light_shaft_brush(&cfg);
        assert_eq!(brush.polys.len(), 6, "Must generate 6 parallel slat sheets");
        for poly in &brush.polys {
            assert_ne!(poly.flags & PF_NOT_SOLID, 0);
        }
    }

    #[test]
    fn test_dust_motes_generation_and_transmission() {
        let cfg = LightShaftConfig::default();
        let motes = generate_dust_motes(&cfg, 25);
        assert_eq!(motes.len(), 25);

        let t_apex = calculate_optical_transmission(&cfg, 0.0);
        let t_mid = calculate_optical_transmission(&cfg, 0.5);
        let t_base = calculate_optical_transmission(&cfg, 1.0);

        assert!(t_apex > t_mid, "Transmission must decay with distance");
        assert!(t_mid > t_base, "Transmission must continue decaying toward floor");
    }

    #[test]
    fn test_safety_audit_detects_extreme_lengths() {
        let mut cfg = LightShaftConfig::default();
        cfg.beam_length = 32.0; // too short

        let hazards = audit_light_shaft_safety(&cfg);
        let short_hazard = hazards.iter().find(|h| h.is_critical && h.title.contains("Degenerate"));
        assert!(short_hazard.is_some(), "Must catch degenerate beam length");
    }
}
