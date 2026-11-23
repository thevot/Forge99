// crates/ut99-bsp/src/skylight_studio.rs
//
// Interactive Glass Skylight, Observation Dome & Structural Truss Studio
// Glazed skylights, observation domes, industrial sawtooth roofs, structural mullions,
// solar lux transmittance ratings, and procedural skylight frame & glass sheet brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical archetypes of glass skylights and observation domes in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkylightArchetype {
    CurvedObservationDome,
    IndustrialSawtoothSkylight,
    VaultedAtriumBarrelVault,
    ReinforcedBlastPavementPane,
    ShipBridgeObservationCanopy,
}

impl SkylightArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            SkylightArchetype::CurvedObservationDome => "Curved Observation Dome (DM-Morpheus Penthouse)",
            SkylightArchetype::IndustrialSawtoothSkylight => "Industrial Sawtooth Skylight (DM-Turbine Factory Roof)",
            SkylightArchetype::VaultedAtriumBarrelVault => "Vaulted Atrium Barrel Vault (CTF-Niven Reactor Hall)",
            SkylightArchetype::ReinforcedBlastPavementPane => "Reinforced Blast Pavement Pane (DM-Deck16][ Floor Glazing)",
            SkylightArchetype::ShipBridgeObservationCanopy => "Ship Bridge Observation Canopy (AS-Frigate Command Deck)",
        }
    }

    pub fn to_config(&self) -> SkylightConfig {
        match self {
            SkylightArchetype::CurvedObservationDome => SkylightConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                width_uu: 256.0,
                length_uu: 256.0,
                height_or_rise_uu: 64.0,
                rib_count: 6,
                mullion_thickness_uu: 8.0,
                glass_transmittance_pct: 85.0,
                is_bulletproof: true,
                sunlight_brightness: 220,
                sunlight_hue: 35, // Warm Solar
                tag: "MorpheusObservationDome0".to_string(),
            },
            SkylightArchetype::IndustrialSawtoothSkylight => SkylightConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                width_uu: 192.0,
                length_uu: 384.0,
                height_or_rise_uu: 96.0,
                rib_count: 4,
                mullion_thickness_uu: 12.0,
                glass_transmittance_pct: 75.0,
                is_bulletproof: false,
                sunlight_brightness: 180,
                sunlight_hue: 45, // Industrial Yellow
                tag: "TurbineSawtoothSkylight0".to_string(),
            },
            SkylightArchetype::VaultedAtriumBarrelVault => SkylightConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                width_uu: 224.0,
                length_uu: 448.0,
                height_or_rise_uu: 56.0,
                rib_count: 5,
                mullion_thickness_uu: 10.0,
                glass_transmittance_pct: 90.0,
                is_bulletproof: true,
                sunlight_brightness: 240,
                sunlight_hue: 150, // Cyan Ambient
                tag: "NivenAtriumBarrelVault0".to_string(),
            },
            SkylightArchetype::ReinforcedBlastPavementPane => SkylightConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                width_uu: 160.0,
                length_uu: 160.0,
                height_or_rise_uu: 16.0,
                rib_count: 3,
                mullion_thickness_uu: 16.0,
                glass_transmittance_pct: 60.0,
                is_bulletproof: true,
                sunlight_brightness: 120,
                sunlight_hue: 80, // Slime Green Glow
                tag: "DeckFloorBlastPane0".to_string(),
            },
            SkylightArchetype::ShipBridgeObservationCanopy => SkylightConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                width_uu: 288.0,
                length_uu: 200.0,
                height_or_rise_uu: 48.0,
                rib_count: 5,
                mullion_thickness_uu: 8.0,
                glass_transmittance_pct: 80.0,
                is_bulletproof: false,
                sunlight_brightness: 200,
                sunlight_hue: 140, // Maritime Blue
                tag: "FrigateBridgeCanopy0".to_string(),
            },
        }
    }
}

/// Configuration settings for a glass skylight or observation dome
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkylightConfig {
    pub archetype: SkylightArchetype,
    pub position: Vec3,
    pub width_uu: f32,
    pub length_uu: f32,
    pub height_or_rise_uu: f32,
    pub rib_count: usize,
    pub mullion_thickness_uu: f32,
    pub glass_transmittance_pct: f32,
    pub is_bulletproof: bool,
    pub sunlight_brightness: u8,
    pub sunlight_hue: u8,
    pub tag: String,
}

impl Default for SkylightConfig {
    fn default() -> Self {
        SkylightArchetype::CurvedObservationDome.to_config()
    }
}

/// Context actor record for evaluating skylight neighborhood
#[derive(Debug, Clone, PartialEq)]
pub struct LevelSkylightContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Telemetry metrics for a glass skylight installation
#[derive(Debug, Clone, PartialEq)]
pub struct SkylightMetrics {
    pub total_glazed_surface_area_sq_uu: f32,
    pub structural_aspect_ratio: f32,
    pub lux_solar_transmittance_rating: f32,
    pub total_skylights_in_level: usize,
}

/// Safety diagnostics for glass skylights and observation domes
#[derive(Debug, Clone, PartialEq)]
pub struct SkylightHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates glazed area, structural aspect ratio, and solar illuminance rating
pub fn evaluate_skylight_metrics(
    config: &SkylightConfig,
    peers: &[LevelSkylightContextActorRecord],
) -> SkylightMetrics {
    let base_area = config.width_uu * config.length_uu;
    // Curvature/dome arc factor increases glazed surface area
    let curvature_factor = 1.0 + (config.height_or_rise_uu / config.width_uu.max(1.0)).powi(2);
    let glazed_area = base_area * curvature_factor;

    let aspect_ratio = config.length_uu / config.width_uu.max(1.0);

    let solar_rating = (config.glass_transmittance_pct / 100.0) * (config.sunlight_brightness as f32);

    let mut skylights_count = 1;
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("light") || cls_lower.contains("glass") || cls_lower.contains("skylight") {
            skylights_count += 1;
        }
    }

    SkylightMetrics {
        total_glazed_surface_area_sq_uu: glazed_area,
        structural_aspect_ratio: aspect_ratio,
        lux_solar_transmittance_rating: solar_rating,
        total_skylights_in_level: skylights_count,
    }
}

/// Audits safety for unreinforced spans, pitch-black glass, and fragile glass over voids
pub fn audit_skylight_safety(
    config: &SkylightConfig,
    _peers: &[LevelSkylightContextActorRecord],
) -> Vec<SkylightHazard> {
    let mut hazards = Vec::new();

    // 1. Unreinforced Structural Span (> 320 uu width with fewer than 3 ribs)
    if config.width_uu > 320.0 && config.rib_count < 3 {
        hazards.push(SkylightHazard {
            title: "Excessive Unreinforced Structural Span".to_string(),
            description: format!(
                "Skylight width of {:.0} uu has only {} structural ribs. Unreinforced spans over 320 uu require at least 3-4 trusses to prevent structural shear failure.",
                config.width_uu, config.rib_count
            ),
            is_critical: true,
        });
    }

    // 2. Opaque Glass Transmittance (<= 0%)
    if config.glass_transmittance_pct <= 0.0 {
        hazards.push(SkylightHazard {
            title: "Opaque Zero-Transmittance Glazing".to_string(),
            description: "Glass transmittance is 0%. The skylight will appear pitch black and admit no ambient sunlight.".to_string(),
            is_critical: false,
        });
    }

    // 3. Fragile Non-Bulletproof Glass on Large Atrium (> 50,000 sq uu)
    let base_area = config.width_uu * config.length_uu;
    if !config.is_bulletproof && base_area > 50000.0 {
        hazards.push(SkylightHazard {
            title: "Fragile Non-Reinforced Glass Over Large Atrium".to_string(),
            description: format!(
                "Large atrium glass surface ({:.0} sq uu) is marked non-bulletproof. Explosive splash damage (Rockets/Flak) may shatter canopy leaving gaping unsealed holes.",
                base_area
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural CSG structural frame and glazed glass pane sheet brush
pub fn generate_skylight_brush(config: &SkylightConfig) -> Brush {
    let mut brush = Brush::new("Skylight_Glazing".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let half_w = config.width_uu * 0.5;
    let half_l = config.length_uu * 0.5;
    let t = config.mullion_thickness_uu;
    let rise = config.height_or_rise_uu;

    // 1. Outer Perimeter Frame Box (4 Mullion Beams)
    // Front Beam (+X)
    add_box_to_brush(&mut brush, half_l - t, half_l, -half_w, half_w, 0.0, t, "Indus1.Indus_B01");
    // Back Beam (-X)
    add_box_to_brush(&mut brush, -half_l, -half_l + t, -half_w, half_w, 0.0, t, "Indus1.Indus_B01");
    // Left Beam (-Y)
    add_box_to_brush(&mut brush, -half_l + t, half_l - t, -half_w, -half_w + t, 0.0, t, "Indus1.Indus_B01");
    // Right Beam (+Y)
    add_box_to_brush(&mut brush, -half_l + t, half_l - t, half_w - t, half_w, 0.0, t, "Indus1.Indus_B01");

    // 2. Glazed Glass Sheet (Single two-sided translucent quad or peaked gabled sheet)
    let p0 = Vec3::new(-half_l + t, -half_w + t, rise * 0.5);
    let p1 = Vec3::new(half_l - t, -half_w + t, rise * 0.5);
    let p2 = Vec3::new(half_l - t, half_w - t, rise * 0.5);
    let p3 = Vec3::new(-half_l + t, half_w - t, rise * 0.5);

    if let Some(mut poly) = Polygon::new(vec![p0, p1, p2, p3]) {
        poly.texture = "GenFX.Glass.BlueGlass01".to_string();
        poly.flags = crate::PF_TRANSLUCENT | crate::PF_TWO_SIDED;
        brush.polys.push(poly);
    }

    // 3. Central Ridge Mullion Truss if rib_count >= 2
    if config.rib_count >= 2 {
        let half_t = t * 0.5;
        add_box_to_brush(&mut brush, -half_l + t, half_l - t, -half_t, half_t, 0.0, rise, "Indus1.Indus_B01");
    }

    brush
}

fn add_box_to_brush(
    brush: &mut Brush,
    min_x: f32, max_x: f32,
    min_y: f32, max_y: f32,
    min_z: f32, max_z: f32,
    tex: &str,
) {
    let p0 = Vec3::new(min_x, min_y, min_z);
    let p1 = Vec3::new(max_x, min_y, min_z);
    let p2 = Vec3::new(max_x, max_y, min_z);
    let p3 = Vec3::new(min_x, max_y, min_z);

    let p4 = Vec3::new(min_x, min_y, max_z);
    let p5 = Vec3::new(max_x, min_y, max_z);
    let p6 = Vec3::new(max_x, max_y, max_z);
    let p7 = Vec3::new(min_x, max_y, max_z);

    // Front (-X)
    if let Some(mut p) = Polygon::new(vec![p0, p3, p7, p4]) { p.texture = tex.to_string(); brush.polys.push(p); }
    // Back (+X)
    if let Some(mut p) = Polygon::new(vec![p1, p5, p6, p2]) { p.texture = tex.to_string(); brush.polys.push(p); }
    // Left (-Y)
    if let Some(mut p) = Polygon::new(vec![p0, p4, p5, p1]) { p.texture = tex.to_string(); brush.polys.push(p); }
    // Right (+Y)
    if let Some(mut p) = Polygon::new(vec![p3, p2, p6, p7]) { p.texture = tex.to_string(); brush.polys.push(p); }
    // Top (+Z)
    if let Some(mut p) = Polygon::new(vec![p4, p7, p6, p5]) { p.texture = tex.to_string(); brush.polys.push(p); }
    // Bottom (-Z)
    if let Some(mut p) = Polygon::new(vec![p0, p1, p2, p3]) { p.texture = tex.to_string(); brush.polys.push(p); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skylight_archetypes_and_defaults() {
        let dome = SkylightArchetype::CurvedObservationDome.to_config();
        assert_eq!(dome.width_uu, 256.0);
        assert_eq!(dome.glass_transmittance_pct, 85.0);
        assert!(dome.is_bulletproof);

        let sawtooth = SkylightArchetype::IndustrialSawtoothSkylight.to_config();
        assert_eq!(sawtooth.length_uu, 384.0);
        assert!(!sawtooth.is_bulletproof);
    }

    #[test]
    fn test_skylight_metrics_evaluation() {
        let cfg = SkylightConfig::default(); // 256x256, rise 64, trans 85%, brightness 220
        let peers = vec![
            LevelSkylightContextActorRecord {
                name: "Sunlight0".to_string(),
                class_name: "Engine.Light".to_string(),
                position: Vec3::new(0.0, 0.0, 200.0),
            }
        ];

        let metrics = evaluate_skylight_metrics(&cfg, &peers);
        assert!(metrics.total_glazed_surface_area_sq_uu > 256.0 * 256.0);
        assert_eq!(metrics.structural_aspect_ratio, 1.0);
        assert_eq!(metrics.lux_solar_transmittance_rating, 0.85 * 220.0);
        assert_eq!(metrics.total_skylights_in_level, 2);
    }

    #[test]
    fn test_skylight_safety_hazard_span() {
        let mut cfg = SkylightConfig::default();
        cfg.width_uu = 400.0;
        cfg.rib_count = 1;

        let hazards = audit_skylight_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Excessive Unreinforced Structural Span")));
    }

    #[test]
    fn test_generate_skylight_brush() {
        let cfg = SkylightConfig::default();
        let brush = generate_skylight_brush(&cfg);

        assert_eq!(brush.name, "Skylight_Glazing");
        // 4 perimeter beams (6 polys each = 24) + 1 glass sheet poly + 1 central ridge beam (6 polys) = 31 polys
        assert_eq!(brush.polys.len(), 31);
    }
}
