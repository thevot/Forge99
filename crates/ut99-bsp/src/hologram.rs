// crates/ut99-bsp/src/hologram.rs
//
// Interactive Holographic Signage, Sci-Fi Billboard & Volumetric Display Studio
//
// Mathematical, geometric, and optical model for volumetric holographic displays,
// projection beam frustums, emitter pedestals, animated scanlines, chromatic aberration,
// and rotating corporate signage inspired by DM-Liandri, DM-Turbine, CTF-FacingWorlds, and AS-Overlord.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical Unreal Tournament & Unreal 1 holographic signage archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HologramArchetype {
    LiandriCorporationEmblem,
    TournamentLiveScoreboard,
    DirectionalWaypointArrow,
    HazardWarningHoloAlert,
    TacticalTargetingReticle,
}

impl HologramArchetype {
    pub fn all() -> &'static [HologramArchetype] {
        &[
            HologramArchetype::LiandriCorporationEmblem,
            HologramArchetype::TournamentLiveScoreboard,
            HologramArchetype::DirectionalWaypointArrow,
            HologramArchetype::HazardWarningHoloAlert,
            HologramArchetype::TacticalTargetingReticle,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            HologramArchetype::LiandriCorporationEmblem => "Liandri Corporation Emblem (Rotating 3D Logo)",
            HologramArchetype::TournamentLiveScoreboard => "Tournament Live Scoreboard (Curved Billboard)",
            HologramArchetype::DirectionalWaypointArrow => "Directional Waypoint Arrow (Pulsing Nav Beacon)",
            HologramArchetype::HazardWarningHoloAlert => "Hazard Warning Holo-Alert (Flashing Breach Siren)",
            HologramArchetype::TacticalTargetingReticle => "Tactical Targeting Reticle (Concentric Range Rings)",
        }
    }

    pub fn default_config(&self) -> HologramConfig {
        match self {
            HologramArchetype::LiandriCorporationEmblem => HologramConfig {
                archetype: *self,
                position: Vec3::ZERO,
                display_shape: HoloDisplayShape::PlanarSheet,
                display_width: 192.0,
                display_height: 192.0,
                curvature_sweep_deg: 0.0,
                elevation_offset_uu: 128.0,
                rotation_speed_deg_s: 20.0,
                bob_amplitude_uu: 8.0,
                bob_frequency_hz: 0.5,
                scanline_count: 24,
                scanline_drift_speed: 1.5,
                chromatic_aberration_uu: 3.0,
                glitch_flicker_probability: 0.05,
                has_emitter_pedestal: true,
                pedestal_radius: 48.0,
                pedestal_height: 16.0,
                has_projection_beam: true,
                beam_cone_angle_deg: 35.0,
                primary_color: [0.20, 0.85, 1.00], // Electric Cyan
                secondary_color: [1.00, 0.80, 0.20], // Liandri Gold
                texture_name: "Indus.Holo.LiandriLogo01".to_string(),
                pedestal_texture: "Indus.Metal.DarkAlloy01".to_string(),
                tag: "HoloLiandri01".to_string(),
                event_name: "HoloRotate_Mover".to_string(),
            },
            HologramArchetype::TournamentLiveScoreboard => HologramConfig {
                archetype: *self,
                position: Vec3::ZERO,
                display_shape: HoloDisplayShape::CurvedCylinderArc,
                display_width: 384.0,
                display_height: 160.0,
                curvature_sweep_deg: 60.0,
                elevation_offset_uu: 96.0,
                rotation_speed_deg_s: 0.0, // stationary billboard
                bob_amplitude_uu: 2.0,
                bob_frequency_hz: 0.2,
                scanline_count: 32,
                scanline_drift_speed: 0.8,
                chromatic_aberration_uu: 1.5,
                glitch_flicker_probability: 0.02,
                has_emitter_pedestal: true,
                pedestal_radius: 64.0,
                pedestal_height: 24.0,
                has_projection_beam: true,
                beam_cone_angle_deg: 45.0,
                primary_color: [0.15, 0.45, 0.95], // Cobalt Blue
                secondary_color: [0.90, 0.95, 1.00], // Pale White
                texture_name: "Indus.Holo.Scoreboard01".to_string(),
                pedestal_texture: "Indus.Metal.SteelFrame01".to_string(),
                tag: "HoloScoreboard01".to_string(),
                event_name: "".to_string(),
            },
            HologramArchetype::DirectionalWaypointArrow => HologramConfig {
                archetype: *self,
                position: Vec3::ZERO,
                display_shape: HoloDisplayShape::PlanarSheet,
                display_width: 96.0,
                display_height: 64.0,
                curvature_sweep_deg: 0.0,
                elevation_offset_uu: 64.0,
                rotation_speed_deg_s: 0.0,
                bob_amplitude_uu: 12.0,
                bob_frequency_hz: 1.0,
                scanline_count: 16,
                scanline_drift_speed: 2.5,
                chromatic_aberration_uu: 2.0,
                glitch_flicker_probability: 0.01,
                has_emitter_pedestal: false,
                pedestal_radius: 24.0,
                pedestal_height: 8.0,
                has_projection_beam: true,
                beam_cone_angle_deg: 25.0,
                primary_color: [0.20, 1.00, 0.40], // Emerald Green
                secondary_color: [0.80, 1.00, 0.60], // Lime
                texture_name: "Indus.Holo.NavArrow01".to_string(),
                pedestal_texture: "Indus.Metal.Bracket01".to_string(),
                tag: "HoloNavArrow01".to_string(),
                event_name: "".to_string(),
            },
            HologramArchetype::HazardWarningHoloAlert => HologramConfig {
                archetype: *self,
                position: Vec3::ZERO,
                display_shape: HoloDisplayShape::PlanarSheet,
                display_width: 160.0,
                display_height: 96.0,
                curvature_sweep_deg: 0.0,
                elevation_offset_uu: 80.0,
                rotation_speed_deg_s: 0.0,
                bob_amplitude_uu: 4.0,
                bob_frequency_hz: 2.0,
                scanline_count: 20,
                scanline_drift_speed: 3.0,
                chromatic_aberration_uu: 5.0, // High glitch aberration
                glitch_flicker_probability: 0.15,
                has_emitter_pedestal: true,
                pedestal_radius: 36.0,
                pedestal_height: 12.0,
                has_projection_beam: true,
                beam_cone_angle_deg: 40.0,
                primary_color: [1.00, 0.20, 0.15], // Alert Red
                secondary_color: [1.00, 0.70, 0.10], // Warning Amber
                texture_name: "Indus.Holo.HazardWarning01".to_string(),
                pedestal_texture: "Indus.Metal.HazardYellow01".to_string(),
                tag: "HoloHazardAlert01".to_string(),
                event_name: "HazardAlert_Trigger".to_string(),
            },
            HologramArchetype::TacticalTargetingReticle => HologramConfig {
                archetype: *self,
                position: Vec3::ZERO,
                display_shape: HoloDisplayShape::VolumetricOctagon,
                display_width: 128.0,
                display_height: 128.0,
                curvature_sweep_deg: 0.0,
                elevation_offset_uu: 112.0,
                rotation_speed_deg_s: -30.0, // Reverse spin
                bob_amplitude_uu: 6.0,
                bob_frequency_hz: 0.8,
                scanline_count: 28,
                scanline_drift_speed: 2.0,
                chromatic_aberration_uu: 2.5,
                glitch_flicker_probability: 0.03,
                has_emitter_pedestal: true,
                pedestal_radius: 32.0,
                pedestal_height: 10.0,
                has_projection_beam: true,
                beam_cone_angle_deg: 30.0,
                primary_color: [0.30, 0.95, 0.70], // Tactical Teal
                secondary_color: [0.90, 0.30, 0.20], // Bullseye Orange
                texture_name: "Indus.Holo.TargetReticle01".to_string(),
                pedestal_texture: "Indus.Metal.Gunmetal01".to_string(),
                tag: "HoloTargetReticle01".to_string(),
                event_name: "".to_string(),
            },
        }
    }
}

/// Geometry shape of the volumetric hologram display panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HoloDisplayShape {
    PlanarSheet,
    CurvedCylinderArc,
    VolumetricOctagon,
}

impl HoloDisplayShape {
    pub fn all() -> &'static [HoloDisplayShape] {
        &[
            HoloDisplayShape::PlanarSheet,
            HoloDisplayShape::CurvedCylinderArc,
            HoloDisplayShape::VolumetricOctagon,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            HoloDisplayShape::PlanarSheet => "Planar Sheet (Flat Billboard)",
            HoloDisplayShape::CurvedCylinderArc => "Curved Arc (Sci-Fi Panoramic)",
            HoloDisplayShape::VolumetricOctagon => "Volumetric Octagon (3D Column)",
        }
    }
}

/// Configuration structure for Hologram Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HologramConfig {
    pub archetype: HologramArchetype,
    pub position: Vec3,
    pub display_shape: HoloDisplayShape,
    pub display_width: f32,
    pub display_height: f32,
    pub curvature_sweep_deg: f32,
    pub elevation_offset_uu: f32,
    pub rotation_speed_deg_s: f32,
    pub bob_amplitude_uu: f32,
    pub bob_frequency_hz: f32,
    pub scanline_count: usize,
    pub scanline_drift_speed: f32,
    pub chromatic_aberration_uu: f32,
    pub glitch_flicker_probability: f32,
    pub has_emitter_pedestal: bool,
    pub pedestal_radius: f32,
    pub pedestal_height: f32,
    pub has_projection_beam: bool,
    pub beam_cone_angle_deg: f32,
    pub primary_color: [f32; 3],
    pub secondary_color: [f32; 3],
    pub texture_name: String,
    pub pedestal_texture: String,
    pub tag: String,
    pub event_name: String,
}

impl Default for HologramConfig {
    fn default() -> Self {
        HologramArchetype::LiandriCorporationEmblem.default_config()
    }
}

/// Analytical optical and kinematic metrics for the hologram display
#[derive(Debug, Clone, PartialEq)]
pub struct HologramMetrics {
    pub surface_area_sq_uu: f32,
    pub projection_volume_uu3: f32,
    pub current_yaw_deg: f32,
    pub current_bob_offset_uu: f32,
    pub effective_luminosity: f32,
    pub is_glitching_now: bool,
}

/// Generated procedural brush meshes for all hologram components
#[derive(Debug, Clone, PartialEq)]
pub struct HologramBrushes {
    pub display_surface_brush: Brush,
    pub emitter_pedestal_brush: Option<Brush>,
    pub projection_cone_brush: Option<Brush>,
}

/// Diagnostic safety hazard
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HologramHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Evaluates dynamic kinematics, floating bob, scanline wave, and flicker at time t
pub fn evaluate_hologram_kinematics(config: &HologramConfig, time_secs: f32) -> HologramMetrics {
    let area = match config.display_shape {
        HoloDisplayShape::PlanarSheet => config.display_width * config.display_height,
        HoloDisplayShape::CurvedCylinderArc => {
            let rad = config.curvature_sweep_deg.to_radians();
            let arc_len = config.display_width * (rad / std::f32::consts::FRAC_PI_2.max(0.1));
            arc_len * config.display_height
        }
        HoloDisplayShape::VolumetricOctagon => {
            let p_circ = config.display_width * 8.0 * 0.4142; // Octagonal perimeter
            p_circ * config.display_height
        }
    };

    let proj_vol = if config.has_projection_beam {
        let h = config.elevation_offset_uu;
        let r = (config.beam_cone_angle_deg.to_radians() * 0.5).tan() * h;
        (std::f32::consts::PI * r * r * h) / 3.0
    } else {
        0.0
    };

    let cur_yaw = (time_secs * config.rotation_speed_deg_s) % 360.0;
    let bob = (time_secs * config.bob_frequency_hz * std::f32::consts::TAU).sin() * config.bob_amplitude_uu;

    // Pseudo-random glitch evaluation based on fractional sine hash
    let glitch_hash = ((time_secs * 23.456).sin() * 43758.5453).fract().abs();
    let is_glitch = glitch_hash < config.glitch_flicker_probability;

    let lum = if is_glitch { 0.3 } else { 0.85 + ((time_secs * 3.0).sin() * 0.15) };

    HologramMetrics {
        surface_area_sq_uu: area,
        projection_volume_uu3: proj_vol,
        current_yaw_deg: cur_yaw,
        current_bob_offset_uu: bob,
        effective_luminosity: lum,
        is_glitching_now: is_glitch,
    }
}

/// Generates procedural brushes for hologram display sheet, emitter base, and projection beam
pub fn generate_hologram_brushes(config: &HologramConfig) -> HologramBrushes {
    let p = config.position;
    let hw = config.display_width * 0.5;
    let hh = config.display_height * 0.5;
    let center_z = p.z + config.elevation_offset_uu;

    // 1. Display Surface Brush
    let mut disp_b = Brush::new("HoloDisplay_Surface", CsgOperation::Add);
    let holo_flags = 0x00000004 | 0x00000002 | 0x00000100 | 0x00000008; // PF_TRANSLUCENT | PF_UNLIT | PF_TWO_SIDED | PF_NOT_SOLID

    match config.display_shape {
        HoloDisplayShape::PlanarSheet => {
            let pts = vec![
                Vec3::new(p.x - hw, p.y, center_z - hh),
                Vec3::new(p.x + hw, p.y, center_z - hh),
                Vec3::new(p.x + hw, p.y, center_z + hh),
                Vec3::new(p.x - hw, p.y, center_z + hh),
            ];
            if let Some(mut poly) = Polygon::new(pts) {
                poly.texture = config.texture_name.clone();
                poly.flags = holo_flags;
                disp_b.polys.push(poly);
            }
        }
        HoloDisplayShape::CurvedCylinderArc => {
            let segments = 8;
            let sweep_rad = config.curvature_sweep_deg.to_radians().max(0.1);
            let arc_r = hw / (sweep_rad * 0.5).sin().max(0.01);

            let mut arc_pts_bottom = Vec::new();
            let mut arc_pts_top = Vec::new();

            for i in 0..=segments {
                let frac = i as f32 / segments as f32;
                let ang = -sweep_rad * 0.5 + frac * sweep_rad;
                let x = p.x + ang.sin() * arc_r;
                let y = p.y + arc_r - ang.cos() * arc_r;
                arc_pts_bottom.push(Vec3::new(x, y, center_z - hh));
                arc_pts_top.push(Vec3::new(x, y, center_z + hh));
            }

            for i in 0..segments {
                let quad = vec![
                    arc_pts_bottom[i],
                    arc_pts_bottom[i + 1],
                    arc_pts_top[i + 1],
                    arc_pts_top[i],
                ];
                if let Some(mut poly) = Polygon::new(quad) {
                    poly.texture = config.texture_name.clone();
                    poly.flags = holo_flags;
                    disp_b.polys.push(poly);
                }
            }
        }
        HoloDisplayShape::VolumetricOctagon => {
            let r = hw;
            let facets = 8;
            let mut bot = Vec::new();
            let mut top = Vec::new();

            for i in 0..facets {
                let a = (i as f32 / facets as f32) * std::f32::consts::TAU;
                let x = p.x + a.cos() * r;
                let y = p.y + a.sin() * r;
                bot.push(Vec3::new(x, y, center_z - hh));
                top.push(Vec3::new(x, y, center_z + hh));
            }

            for i in 0..facets {
                let next = (i + 1) % facets;
                let quad = vec![bot[i], bot[next], top[next], top[i]];
                if let Some(mut poly) = Polygon::new(quad) {
                    poly.texture = config.texture_name.clone();
                    poly.flags = holo_flags;
                    disp_b.polys.push(poly);
                }
            }
        }
    }

    // 2. Emitter Pedestal Brush (Solid housing on floor)
    let pedestal_b = if config.has_emitter_pedestal {
        let mut pb = Brush::new("HoloEmitter_Pedestal", CsgOperation::Add);
        let pr = config.pedestal_radius;
        let ph = config.pedestal_height;
        let facets = 8;

        let mut base_pts = Vec::new();
        let mut top_pts = Vec::new();

        for i in 0..facets {
            let a = (i as f32 / facets as f32) * std::f32::consts::TAU;
            let x = p.x + a.cos() * pr;
            let y = p.y + a.sin() * pr;
            base_pts.push(Vec3::new(x, y, p.z));
            top_pts.push(Vec3::new(x, y, p.z + ph));
        }

        // Side quads
        for i in 0..facets {
            let next = (i + 1) % facets;
            let quad = vec![base_pts[i], base_pts[next], top_pts[next], top_pts[i]];
            if let Some(mut poly) = Polygon::new(quad) {
                poly.texture = config.pedestal_texture.clone();
                poly.flags |= 0x00200000; // PF_SEMISOLID
                pb.polys.push(poly);
            }
        }

        // Top cap
        if let Some(mut poly) = Polygon::new(top_pts) {
            poly.texture = config.pedestal_texture.clone();
            poly.flags |= 0x00200000;
            pb.polys.push(poly);
        }

        Some(pb)
    } else {
        None
    };

    // 3. Volumetric Projection Cone Frustum (Beam from emitter lens to display)
    let beam_b = if config.has_projection_beam {
        let mut bb = Brush::new("HoloBeam_Cone", CsgOperation::Add);
        let lens_z = p.z + if config.has_emitter_pedestal { config.pedestal_height } else { 0.0 };
        let lens_pos = Vec3::new(p.x, p.y, lens_z);
        let display_bottom_z = center_z - hh;

        let cone_r = hw * 1.1;
        let facets = 8;
        let mut beam_top_pts = Vec::new();

        for i in 0..facets {
            let a = (i as f32 / facets as f32) * std::f32::consts::TAU;
            let x = p.x + a.cos() * cone_r;
            let y = p.y + a.sin() * cone_r;
            beam_top_pts.push(Vec3::new(x, y, display_bottom_z));
        }

        for i in 0..facets {
            let next = (i + 1) % facets;
            let tri = vec![lens_pos, beam_top_pts[next], beam_top_pts[i]];
            if let Some(mut poly) = Polygon::new(tri) {
                poly.texture = "Indus.Holo.BeamFade01".to_string();
                poly.flags = 0x00000004 | 0x00000002 | 0x00000100 | 0x00000008; // Translucent unlit not-solid
                bb.polys.push(poly);
            }
        }

        Some(bb)
    } else {
        None
    };

    HologramBrushes {
        display_surface_brush: disp_b,
        emitter_pedestal_brush: pedestal_b,
        projection_cone_brush: beam_b,
    }
}

/// Audits hologram configuration for optical clarity, dimensions, and clipping
pub fn audit_hologram_safety(
    config: &HologramConfig,
    _world_brushes: &[Brush],
) -> Vec<HologramHazard> {
    let mut hazards = Vec::new();

    // 1. Degenerate display dimensions
    if config.display_width < 32.0 || config.display_height < 32.0 {
        hazards.push(HologramHazard {
            is_critical: true,
            title: "Sub-Miniature Hologram Display".to_string(),
            description: format!("Display dimensions ({:.0}x{:.0} uu) are too small to render legible text or icons in Unreal Engine.",
                config.display_width, config.display_height),
            recommendation: "Increase display_width and display_height to at least 64 uu.".to_string(),
        });
    }

    // 2. Excessive rotation speed
    if config.rotation_speed_deg_s.abs() > 90.0 {
        hazards.push(HologramHazard {
            is_critical: false,
            title: "Disorienting Hologram Rotation Speed".to_string(),
            description: format!("Rotation speed ({:.0} deg/s) is extremely rapid. May cause visual distraction and motion sickness in competitive play.",
                config.rotation_speed_deg_s),
            recommendation: "Keep rotation speed between 10 and 30 deg/s for graceful corporate aesthetics.".to_string(),
        });
    }

    // 3. Elevation offset too low (displays below floor level)
    if config.elevation_offset_uu < config.display_height * 0.5 {
        hazards.push(HologramHazard {
            is_critical: true,
            title: "Floor Clipping Hologram Display".to_string(),
            description: "elevation_offset_uu is smaller than half height. Hologram surface penetrates the floor plane.".to_string(),
            recommendation: "Increase elevation_offset_uu to elevate the display completely above the emitter base.".to_string(),
        });
    }

    // 4. Excessive glitch probability
    if config.glitch_flicker_probability > 0.25 {
        hazards.push(HologramHazard {
            is_critical: false,
            title: "Excessive Glitch Strobe Flicker".to_string(),
            description: format!("glitch_flicker_probability ({:.0}%) will cause aggressive flashing. Can be strenuous on mapper eyes.",
                config.glitch_flicker_probability * 100.0),
            recommendation: "Lower glitch probability to under 10% for subtle sci-fi interference.".to_string(),
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hologram_archetypes_and_defaults() {
        for &arch in HologramArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.display_width >= 96.0);
            assert!(cfg.display_height >= 64.0);
            assert!(cfg.elevation_offset_uu > 32.0);
        }
    }

    #[test]
    fn test_evaluate_hologram_kinematics() {
        let cfg = HologramArchetype::LiandriCorporationEmblem.default_config();
        let m = evaluate_hologram_kinematics(&cfg, 1.0);

        assert!(m.surface_area_sq_uu > 1000.0);
        assert!(m.projection_volume_uu3 > 1000.0);
        assert_eq!(m.current_yaw_deg, 20.0);
        assert!(m.effective_luminosity > 0.0);
    }

    #[test]
    fn test_generate_hologram_brushes_geometry() {
        let cfg = HologramArchetype::LiandriCorporationEmblem.default_config();
        let brushes = generate_hologram_brushes(&cfg);

        assert!(!brushes.display_surface_brush.polys.is_empty());
        assert!(brushes.emitter_pedestal_brush.is_some());
        assert!(brushes.projection_cone_brush.is_some());

        // Check translucent and unlit flags on display surface
        let poly = &brushes.display_surface_brush.polys[0];
        assert_eq!(poly.flags & 0x00000004, 0x00000004); // PF_TRANSLUCENT
        assert_eq!(poly.flags & 0x00000002, 0x00000002); // PF_UNLIT
    }

    #[test]
    fn test_hologram_safety_audit() {
        let mut cfg = HologramConfig::default();
        cfg.display_width = 16.0; // Sub-miniature!
        cfg.elevation_offset_uu = 5.0; // Floor clipping!
        cfg.rotation_speed_deg_s = 120.0; // Rapid!

        let hazards = audit_hologram_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Sub-Miniature")));
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Floor Clipping")));
        assert!(hazards.iter().any(|h| !h.is_critical && h.title.contains("Disorienting")));
    }
}
