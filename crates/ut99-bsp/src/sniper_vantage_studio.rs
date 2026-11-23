// crates/ut99-bsp/src/sniper_vantage_studio.rs
//
// Interactive Sniper Vantage Point, Headshot Angle & Occlusion Nest Studio
// Zoom magnification FOV, headshot hitbox height clearance, sweeping cone coverage,
// defensive parapet cover exposure ratings, and procedural crenelated balcony brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Archetypes of canonical sniper vantage positions in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SniperVantageArchetype {
    HighAltitudeTowerSpire,
    ElevatedCorridorCatwalk,
    ShadowedRecessedAlcove,
    CrossRiverBridgeOverlook,
    BaseEntrancePillarPerch,
}

impl SniperVantageArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            SniperVantageArchetype::HighAltitudeTowerSpire => "High-Altitude Tower Spire (CTF-Face Spires)",
            SniperVantageArchetype::ElevatedCorridorCatwalk => "Elevated Corridor Catwalk (DM-Turbine Rafters)",
            SniperVantageArchetype::ShadowedRecessedAlcove => "Shadowed Recessed Alcove (DM-Gothic Ramparts)",
            SniperVantageArchetype::CrossRiverBridgeOverlook => "Cross-River Bridge Overlook (CTF-Gauntlet Overwatch)",
            SniperVantageArchetype::BaseEntrancePillarPerch => "Base Entrance Pillar Perch (CTF-Coret Defense)",
        }
    }

    pub fn to_config(&self) -> SniperVantageConfig {
        match self {
            SniperVantageArchetype::HighAltitudeTowerSpire => SniperVantageConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                look_direction: Vec3::new(1.0, 0.0, 0.0),
                zoom_fov_deg: 15.0,
                max_effective_range_uu: 4500.0,
                parapet_height_uu: 44.0, // Waist-high battlement
                parapet_width_uu: 160.0,
                parapet_depth_uu: 120.0,
                coverage_angle_deg: 90.0,
                tag: "TowerSniper0".to_string(),
            },
            SniperVantageArchetype::ElevatedCorridorCatwalk => SniperVantageConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                look_direction: Vec3::new(1.0, 0.0, 0.0),
                zoom_fov_deg: 30.0,
                max_effective_range_uu: 2500.0,
                parapet_height_uu: 40.0,
                parapet_width_uu: 128.0,
                parapet_depth_uu: 96.0,
                coverage_angle_deg: 60.0,
                tag: "CatwalkPerch0".to_string(),
            },
            SniperVantageArchetype::ShadowedRecessedAlcove => SniperVantageConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                look_direction: Vec3::new(1.0, 0.0, 0.0),
                zoom_fov_deg: 45.0,
                max_effective_range_uu: 1800.0,
                parapet_height_uu: 48.0,
                parapet_width_uu: 96.0,
                parapet_depth_uu: 80.0,
                coverage_angle_deg: 45.0,
                tag: "ShadowAlcove0".to_string(),
            },
            SniperVantageArchetype::CrossRiverBridgeOverlook => SniperVantageConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                look_direction: Vec3::new(1.0, 0.0, 0.0),
                zoom_fov_deg: 20.0,
                max_effective_range_uu: 3200.0,
                parapet_height_uu: 42.0,
                parapet_width_uu: 180.0,
                parapet_depth_uu: 100.0,
                coverage_angle_deg: 80.0,
                tag: "BridgeOverlook0".to_string(),
            },
            SniperVantageArchetype::BaseEntrancePillarPerch => SniperVantageConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                look_direction: Vec3::new(1.0, 0.0, 0.0),
                zoom_fov_deg: 35.0,
                max_effective_range_uu: 2000.0,
                parapet_height_uu: 44.0,
                parapet_width_uu: 140.0,
                parapet_depth_uu: 110.0,
                coverage_angle_deg: 70.0,
                tag: "BaseDefensePerch0".to_string(),
            },
        }
    }
}

/// Configuration settings for a sniper vantage nest
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SniperVantageConfig {
    pub archetype: SniperVantageArchetype,
    pub position: Vec3,
    pub look_direction: Vec3,
    pub zoom_fov_deg: f32,
    pub max_effective_range_uu: f32,
    pub parapet_height_uu: f32,
    pub parapet_width_uu: f32,
    pub parapet_depth_uu: f32,
    pub coverage_angle_deg: f32,
    pub tag: String,
}

impl Default for SniperVantageConfig {
    fn default() -> Self {
        SniperVantageArchetype::HighAltitudeTowerSpire.to_config()
    }
}

/// Context actor record for evaluating sightlines against players and items
#[derive(Debug, Clone, PartialEq)]
pub struct LevelSniperContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Telemetry metrics for a sniper vantage point
#[derive(Debug, Clone, PartialEq)]
pub struct SniperVantageMetrics {
    pub max_sightline_uu: f32,
    pub headshot_clearance_uu: f32,
    pub cover_exposure_pct: f32,
    pub estimated_ttk_secs: f32,
    pub total_vantage_points_in_level: usize,
}

/// Safety diagnostics for sniper balance and fair play
#[derive(Debug, Clone, PartialEq)]
pub struct SniperVantageSafetyAlert {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates sightline range, headshot vertical clearance, cover exposure, and TTK
pub fn evaluate_sniper_vantage_metrics(
    config: &SniperVantageConfig,
    peers: &[LevelSniperContextActorRecord],
) -> SniperVantageMetrics {
    // In UT99 player height is 78 uu. Standing eye height ~ 64 uu, crouch ~ 38 uu.
    // Headshot vertical hitbox in UT99 is top 14 uu (Z in [58..72] uu above floor).
    // Clearance above parapet: eye_height - parapet_height
    let eye_height = 64.0;
    let headshot_clearance = (eye_height - config.parapet_height_uu).max(0.0);

    // Defensive cover rating: percentage of 360-degree perimeter protected by parapet/walls
    // Forward coverage angle is open; rear and sides are considered covered if angle is narrow
    let cover_exposure = (config.coverage_angle_deg / 360.0) * 100.0;

    // In UT99 sniper rifle does 67 base damage and 150 damage on headshot (instant lethal).
    // Firing delay is 0.7s per round. Estimated TTK with 100% headshot accuracy is 0.0s (instant kill).
    // For body shots it requires 2 shots = 0.7s.
    let estimated_ttk = 0.7;

    let mut vantages_count = 1;
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("sniper") || cls_lower.contains("vantage") {
            vantages_count += 1;
        }
    }

    SniperVantageMetrics {
        max_sightline_uu: config.max_effective_range_uu,
        headshot_clearance_uu: headshot_clearance,
        cover_exposure_pct: cover_exposure,
        estimated_ttk_secs: estimated_ttk,
        total_vantage_points_in_level: vantages_count,
    }
}

/// Audits safety for spawn-camping sightlines, excessive vulnerability, and view obstruction
pub fn audit_sniper_vantage_safety(
    config: &SniperVantageConfig,
    peers: &[LevelSniperContextActorRecord],
) -> Vec<SniperVantageSafetyAlert> {
    let mut alerts = Vec::new();

    // 1. Direct Unblocked Sightline to PlayerStart (Spawn-Camping Hazard)
    let look_dir = config.look_direction.normalize_or_zero();
    for p in peers {
        if p.class_name.eq_ignore_ascii_case("Engine.PlayerStart") {
            let to_spawn = p.position - config.position;
            let dist = to_spawn.length();
            if dist > 1.0 && dist <= config.max_effective_range_uu {
                let dot = to_spawn.normalize_or_zero().dot(look_dir);
                // Angle within half coverage cone
                let half_cone = (config.coverage_angle_deg * 0.5).to_radians().cos();
                if dot >= half_cone {
                    alerts.push(SniperVantageSafetyAlert {
                        title: "Spawn-Camping Vantage Sightline".to_string(),
                        description: format!(
                            "Sniper nest has direct line-of-sight onto PlayerStart '{}' at {:.0} uu. Spawning players can be instantly headshot before gaining weapon cover!",
                            p.name, dist
                        ),
                        is_critical: true,
                    });
                }
            }
        }
    }

    // 2. Zero-Cover Exposure (> 75% exposed to 360-degree flanking)
    let exposure_pct = (config.coverage_angle_deg / 360.0) * 100.0;
    if exposure_pct > 75.0 {
        alerts.push(SniperVantageSafetyAlert {
            title: "Excessive Exposure Angle".to_string(),
            description: format!(
                "Vantage point has a wide open firing angle of {:.0} deg ({:.0}% exposed). Snipers have negligible lateral cover and will be easily flanked.",
                config.coverage_angle_deg, exposure_pct
            ),
            is_critical: false,
        });
    }

    // 3. Parapet Height Too Tall (Blocks Standing Line of Sight)
    if config.parapet_height_uu >= 64.0 {
        alerts.push(SniperVantageSafetyAlert {
            title: "Obstructed Parapet Battlement".to_string(),
            description: format!(
                "Parapet height of {:.0} uu exceeds standard standing eye level (64 uu). Players cannot fire over this wall without jumping!",
                config.parapet_height_uu
            ),
            is_critical: true,
        });
    }

    // 4. Extreme Sightline Range (> 5000 uu)
    if config.max_effective_range_uu > 5000.0 {
        alerts.push(SniperVantageSafetyAlert {
            title: "Extreme Sightline Range".to_string(),
            description: format!(
                "Max effective range of {:.0} uu exceeds typical zone distance fog cutoffs and engine culling bounds.",
                config.max_effective_range_uu
            ),
            is_critical: false,
        });
    }

    alerts
}

/// Generates a procedural CSG parapet balcony brush with standing platform and battlement wall
pub fn generate_sniper_parapet_brush(config: &SniperVantageConfig) -> Brush {
    let mut brush = Brush::new("Sniper_Parapet".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let w = config.parapet_width_uu;
    let d = config.parapet_depth_uu;
    let h = config.parapet_height_uu;
    let half_w = w * 0.5;
    let half_d = d * 0.5;

    // Platform Base Slab (16 uu thick below floor level z=0)
    let slab_thick = 16.0;
    let b_p0 = Vec3::new(-half_w, -half_d, -slab_thick);
    let b_p1 = Vec3::new(half_w, -half_d, -slab_thick);
    let b_p2 = Vec3::new(half_w, half_d, -slab_thick);
    let b_p3 = Vec3::new(-half_w, half_d, -slab_thick);

    let t_p0 = Vec3::new(-half_w, -half_d, 0.0);
    let t_p1 = Vec3::new(half_w, -half_d, 0.0);
    let t_p2 = Vec3::new(half_w, half_d, 0.0);
    let t_p3 = Vec3::new(-half_w, half_d, 0.0);

    // Floor top face
    if let Some(mut poly) = Polygon::new(vec![t_p0, t_p1, t_p2, t_p3]) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    // Floor bottom face
    if let Some(mut poly) = Polygon::new(vec![b_p3, b_p2, b_p1, b_p0]) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    // Front Parapet Battlement Wall (along front edge +X)
    let wall_thick = 12.0;
    let w_b0 = Vec3::new(half_w - wall_thick, -half_d, 0.0);
    let w_b1 = Vec3::new(half_w, -half_d, 0.0);
    let w_b2 = Vec3::new(half_w, half_d, 0.0);
    let w_b3 = Vec3::new(half_w - wall_thick, half_d, 0.0);

    let w_t0 = Vec3::new(half_w - wall_thick, -half_d, h);
    let w_t1 = Vec3::new(half_w, -half_d, h);
    let w_t2 = Vec3::new(half_w, half_d, h);
    let w_t3 = Vec3::new(half_w - wall_thick, half_d, h);

    // Wall top face
    if let Some(mut poly) = Polygon::new(vec![w_t0, w_t1, w_t2, w_t3]) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    // Wall front face (facing +X)
    if let Some(mut poly) = Polygon::new(vec![w_b1, w_b2, w_t2, w_t1]) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    // Wall back face (facing inside -X towards standing player)
    if let Some(mut poly) = Polygon::new(vec![w_b3, w_b0, w_t0, w_t3]) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sniper_vantage_archetypes_and_defaults() {
        let tower = SniperVantageArchetype::HighAltitudeTowerSpire.to_config();
        assert_eq!(tower.max_effective_range_uu, 4500.0);
        assert_eq!(tower.zoom_fov_deg, 15.0);
        assert_eq!(tower.parapet_height_uu, 44.0);

        let alcove = SniperVantageArchetype::ShadowedRecessedAlcove.to_config();
        assert_eq!(alcove.coverage_angle_deg, 45.0);
        assert_eq!(alcove.max_effective_range_uu, 1800.0);
    }

    #[test]
    fn test_sniper_vantage_metrics_evaluation() {
        let cfg = SniperVantageConfig::default(); // parapet 44 uu
        let peers = vec![
            LevelSniperContextActorRecord {
                name: "SniperRifle0".to_string(),
                class_name: "Botpack.SniperRifle".to_string(),
                position: Vec3::new(100.0, 0.0, 0.0),
            }
        ];

        let metrics = evaluate_sniper_vantage_metrics(&cfg, &peers);
        // Headshot clearance: 64 - 44 = 20 uu
        assert_eq!(metrics.headshot_clearance_uu, 20.0);
        assert_eq!(metrics.cover_exposure_pct, 25.0); // 90 / 360 = 25%
        assert_eq!(metrics.total_vantage_points_in_level, 2);
    }

    #[test]
    fn test_sniper_vantage_safety_alert_spawn_camping() {
        let cfg = SniperVantageConfig::default(); // looking along +X, range 4500, cone 90
        let peers = vec![
            LevelSniperContextActorRecord {
                name: "PlayerStart0".to_string(),
                class_name: "Engine.PlayerStart".to_string(),
                position: Vec3::new(1200.0, 50.0, 0.0), // along look vector
            }
        ];

        let alerts = audit_sniper_vantage_safety(&cfg, &peers);
        assert!(alerts.iter().any(|a| a.is_critical && a.title.contains("Spawn-Camping Vantage Sightline")));
    }

    #[test]
    fn test_generate_sniper_parapet_brush() {
        let cfg = SniperVantageConfig::default();
        let brush = generate_sniper_parapet_brush(&cfg);

        assert_eq!(brush.name, "Sniper_Parapet");
        assert!(brush.polys.len() >= 5);
    }
}
