// crates/ut99-bsp/src/special_lit_studio.rs
//
// Interactive Special Lit, Zone Lighting Mask & Light Filter Studio
// bSpecialLit light actor isolation, PF_SpecialLit surface matching, dual-pass contrast ratio,
// hue/saturation/brightness filters, and procedural recessed spotlight fixture brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Archetypes of canonical SpecialLit lighting scenarios in Unreal Tournament 99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecialLitArchetype {
    ToxicSlimeGlowIsolator,
    CelestialBackdropIlluminator,
    TeamBaseAccentSpot,
    CathedralStainedGlassBeam,
    SubterraneanLavaRimLight,
}

impl SpecialLitArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            SpecialLitArchetype::ToxicSlimeGlowIsolator => "Toxic Slime Glow Isolator (Deck16 Slime Vats)",
            SpecialLitArchetype::CelestialBackdropIlluminator => "Celestial Backdrop Illuminator (Face Asteroid Horizon)",
            SpecialLitArchetype::TeamBaseAccentSpot => "Team Base Accent Spotlight (Coret Red/Blue Arch)",
            SpecialLitArchetype::CathedralStainedGlassBeam => "Cathedral Stained Glass Beam (Curse Golden Altar)",
            SpecialLitArchetype::SubterraneanLavaRimLight => "Subterranean Lava Rim Light (Lava Trench Contrast)",
        }
    }

    pub fn to_config(&self) -> SpecialLitConfig {
        match self {
            SpecialLitArchetype::ToxicSlimeGlowIsolator => SpecialLitConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                light_radius_uu: 384.0,
                light_brightness: 180,
                light_hue: 85, // Toxic Green
                light_saturation: 25,
                light_type: "LT_Steady".to_string(),
                light_effect: "LE_WateryShimmer".to_string(),
                fixture_radius_uu: 24.0,
                fixture_depth_uu: 16.0,
                tag: "SlimeGlow0".to_string(),
            },
            SpecialLitArchetype::CelestialBackdropIlluminator => SpecialLitConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                light_radius_uu: 1200.0,
                light_brightness: 140,
                light_hue: 160, // Deep Cosmic Blue/Indigo
                light_saturation: 40,
                light_type: "LT_Steady".to_string(),
                light_effect: "LE_None".to_string(),
                fixture_radius_uu: 48.0,
                fixture_depth_uu: 24.0,
                tag: "AsteroidLight0".to_string(),
            },
            SpecialLitArchetype::TeamBaseAccentSpot => SpecialLitConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                light_radius_uu: 450.0,
                light_brightness: 200,
                light_hue: 220, // Vibrant Team Blue / Red
                light_saturation: 10,
                light_type: "LT_Steady".to_string(),
                light_effect: "LE_Spotlight".to_string(),
                fixture_radius_uu: 32.0,
                fixture_depth_uu: 20.0,
                tag: "BaseTrimSpot0".to_string(),
            },
            SpecialLitArchetype::CathedralStainedGlassBeam => SpecialLitConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                light_radius_uu: 600.0,
                light_brightness: 190,
                light_hue: 35, // Warm Golden Amber
                light_saturation: 30,
                light_type: "LT_SubtlePulse".to_string(),
                light_effect: "LE_TorchWaver".to_string(),
                fixture_radius_uu: 36.0,
                fixture_depth_uu: 18.0,
                tag: "SunbeamAltar0".to_string(),
            },
            SpecialLitArchetype::SubterraneanLavaRimLight => SpecialLitConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                light_radius_uu: 500.0,
                light_brightness: 210,
                light_hue: 15, // Smoldering Magma Ruby
                light_saturation: 15,
                light_type: "LT_Pulse".to_string(),
                light_effect: "LE_FireWaver".to_string(),
                fixture_radius_uu: 28.0,
                fixture_depth_uu: 16.0,
                tag: "LavaRimSpot0".to_string(),
            },
        }
    }
}

/// Configuration settings for a SpecialLit light actor and fixture
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpecialLitConfig {
    pub archetype: SpecialLitArchetype,
    pub position: Vec3,
    pub light_radius_uu: f32,
    pub light_brightness: u8,
    pub light_hue: u8,
    pub light_saturation: u8,
    pub light_type: String,
    pub light_effect: String,
    pub fixture_radius_uu: f32,
    pub fixture_depth_uu: f32,
    pub tag: String,
}

impl Default for SpecialLitConfig {
    fn default() -> Self {
        SpecialLitArchetype::ToxicSlimeGlowIsolator.to_config()
    }
}

/// Representation of a level surface polygon for SpecialLit auditing
#[derive(Debug, Clone, PartialEq)]
pub struct LevelLightContextSurfaceRecord {
    pub poly_index: usize,
    pub is_special_lit: bool,
    pub center: Vec3,
    pub normal: Vec3,
    pub area: f32,
}

/// Telemetry metrics for SpecialLit illumination
#[derive(Debug, Clone, PartialEq)]
pub struct SpecialLitMetrics {
    pub effective_illumination_flux: f32,
    pub matched_special_surfaces_count: usize,
    pub special_lit_coverage_sq_uu: f32,
    pub light_isolation_efficiency_pct: f32,
    pub total_special_lights_in_level: usize,
}

/// Safety diagnostics for SpecialLit isolation
#[derive(Debug, Clone, PartialEq)]
pub struct SpecialLitHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates illumination flux, matched special surfaces, coverage area, and isolation efficiency
pub fn evaluate_special_lit_metrics(
    config: &SpecialLitConfig,
    surfaces: &[LevelLightContextSurfaceRecord],
) -> SpecialLitMetrics {
    // Flux is proportional to brightness * radius^2 (arbitrary luminous unit scaling)
    let flux = (config.light_brightness as f32 / 255.0) * (config.light_radius_uu / 100.0).powi(2) * 50.0;

    let mut matched_count = 0;
    let mut total_special_area = 0.0;
    let mut non_special_in_radius = 0;

    for s in surfaces {
        let dist = (s.center - config.position).length();
        if dist <= config.light_radius_uu {
            if s.is_special_lit {
                matched_count += 1;
                total_special_area += s.area;
            } else {
                non_special_in_radius += 1;
            }
        }
    }

    let total_surfs_in_radius = matched_count + non_special_in_radius;
    let isolation_efficiency = if total_surfs_in_radius > 0 {
        (matched_count as f32 / total_surfs_in_radius as f32) * 100.0
    } else {
        100.0
    };

    SpecialLitMetrics {
        effective_illumination_flux: flux,
        matched_special_surfaces_count: matched_count,
        special_lit_coverage_sq_uu: total_special_area,
        light_isolation_efficiency_pct: isolation_efficiency,
        total_special_lights_in_level: 1, // Will be accumulated with peers in state
    }
}

/// Audits safety for orphan SpecialLit lights, unlit special surfaces, and wash-out brightness clipping
pub fn audit_special_lit_safety(
    config: &SpecialLitConfig,
    surfaces: &[LevelLightContextSurfaceRecord],
) -> Vec<SpecialLitHazard> {
    let mut hazards = Vec::new();

    let mut special_in_radius = 0;
    for s in surfaces {
        let dist = (s.center - config.position).length();
        if dist <= config.light_radius_uu && s.is_special_lit {
            special_in_radius += 1;
        }
    }

    // 1. Orphan SpecialLit Light (zero PF_SpecialLit surfaces in radius)
    if !surfaces.is_empty() && special_in_radius == 0 {
        hazards.push(SpecialLitHazard {
            title: "Orphan SpecialLit Light".to_string(),
            description: format!(
                "Light is marked bSpecialLit=true, but 0 surfaces with PF_SpecialLit exist within its radius ({:.0} uu). This light illuminates nothing in-game!",
                config.light_radius_uu
            ),
            is_critical: true,
        });
    }

    // 2. Extreme Brightness Clipping (> 230)
    if config.light_brightness > 230 {
        hazards.push(SpecialLitHazard {
            title: "Over-Saturated Light Brightness".to_string(),
            description: format!(
                "Light brightness of {}/255 will cause harsh white clipping and lose color hue saturation on target surfaces.",
                config.light_brightness
            ),
            is_critical: false,
        });
    }

    // 3. Radius Waste (> 1000 uu on localized archetype)
    if config.archetype != SpecialLitArchetype::CelestialBackdropIlluminator && config.light_radius_uu > 900.0 {
        hazards.push(SpecialLitHazard {
            title: "Excessive Light Radius".to_string(),
            description: format!(
                "Radius of {:.0} uu is unusually large for an indoor/architectural SpecialLit accent. Consider tightening radius to conserve lightmap baking time.",
                config.light_radius_uu
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural recessed hexagonal spotlight fixture brush in CSG
pub fn generate_special_lit_fixture_brush(config: &SpecialLitConfig) -> Brush {
    let mut brush = Brush::new("SpecialLit_Fixture".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let sides = 6;
    let r = config.fixture_radius_uu;
    let d = config.fixture_depth_uu;

    let mut front_verts = Vec::with_capacity(sides);
    let mut back_verts = Vec::with_capacity(sides);

    for i in 0..sides {
        let angle = (i as f32) / (sides as f32) * std::f32::consts::TAU;
        let x = angle.cos() * r;
        let y = angle.sin() * r;
        front_verts.push(Vec3::new(x, y, 0.0));
        back_verts.push(Vec3::new(x * 0.7, y * 0.7, -d));
    }

    // Side Casing Faces
    for i in 0..sides {
        let next = (i + 1) % sides;
        let p0 = front_verts[i];
        let p1 = front_verts[next];
        let p2 = back_verts[next];
        let p3 = back_verts[i];

        if let Some(mut poly) = Polygon::new(vec![p0, p1, p2, p3]) {
            poly.texture = "Indus1.Indus_B01".to_string();
            brush.polys.push(poly);
        }
    }

    // Front Reflector Ring
    if let Some(mut poly) = Polygon::new(front_verts) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    // Back Housing Base
    let mut back_rev = back_verts.clone();
    back_rev.reverse();
    if let Some(mut poly) = Polygon::new(back_rev) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_special_lit_archetypes_and_defaults() {
        let cfg = SpecialLitArchetype::ToxicSlimeGlowIsolator.to_config();
        assert_eq!(cfg.light_hue, 85);
        assert_eq!(cfg.light_brightness, 180);
        assert_eq!(cfg.light_effect, "LE_WateryShimmer");

        let celestial = SpecialLitArchetype::CelestialBackdropIlluminator.to_config();
        assert_eq!(celestial.light_radius_uu, 1200.0);
    }

    #[test]
    fn test_special_lit_metrics_evaluation() {
        let cfg = SpecialLitConfig::default(); // radius 384 uu
        let surfs = vec![
            LevelLightContextSurfaceRecord {
                poly_index: 0,
                is_special_lit: true,
                center: Vec3::new(100.0, 0.0, 0.0), // in radius
                normal: Vec3::Z,
                area: 1024.0,
            },
            LevelLightContextSurfaceRecord {
                poly_index: 1,
                is_special_lit: false,
                center: Vec3::new(200.0, 0.0, 0.0), // in radius
                normal: Vec3::Z,
                area: 2048.0,
            },
            LevelLightContextSurfaceRecord {
                poly_index: 2,
                is_special_lit: true,
                center: Vec3::new(800.0, 0.0, 0.0), // outside radius
                normal: Vec3::Z,
                area: 4096.0,
            },
        ];

        let metrics = evaluate_special_lit_metrics(&cfg, &surfs);
        assert_eq!(metrics.matched_special_surfaces_count, 1);
        assert_eq!(metrics.special_lit_coverage_sq_uu, 1024.0);
        assert_eq!(metrics.light_isolation_efficiency_pct, 50.0); // 1 special vs 1 non-special in radius
    }

    #[test]
    fn test_special_lit_safety_hazard_orphan_light() {
        let cfg = SpecialLitConfig::default();
        let surfs = vec![
            LevelLightContextSurfaceRecord {
                poly_index: 0,
                is_special_lit: false,
                center: Vec3::new(50.0, 0.0, 0.0),
                normal: Vec3::Z,
                area: 500.0,
            }
        ];

        let hazards = audit_special_lit_safety(&cfg, &surfs);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Orphan SpecialLit Light")));
    }

    #[test]
    fn test_generate_special_lit_fixture_brush() {
        let cfg = SpecialLitConfig::default();
        let brush = generate_special_lit_fixture_brush(&cfg);

        assert_eq!(brush.name, "SpecialLit_Fixture");
        // 6 side casing + front + back = 8 polygons
        assert_eq!(brush.polys.len(), 8);
    }
}
