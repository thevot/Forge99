// crates/ut99-bsp/src/secret_door_studio.rs
//
// Interactive Secret Door, Concealed Passage & Keyhole Mover Studio
// Concealed trigger mechanisms, mover displacement keyframes, passage dimensions,
// anti-entrapment return triggers, and procedural secret door slab brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Archetypes of canonical secret doors and concealed passages in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretDoorArchetype {
    FlushWallSlidingPanel,
    RotatingPivotingBookshelf,
    SunkenFloorTrapdoor,
    DisplacedStoneSlab,
    HighTechCamouflagedLocker,
}

impl SecretDoorArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            SecretDoorArchetype::FlushWallSlidingPanel => "Flush Wall Sliding Panel (Deck16 Secret Teleporter)",
            SecretDoorArchetype::RotatingPivotingBookshelf => "Rotating Pivoting Bookshelf (Curse Hidden Relic)",
            SecretDoorArchetype::SunkenFloorTrapdoor => "Sunken Floor Trapdoor (November Submarine Duct)",
            SecretDoorArchetype::DisplacedStoneSlab => "Displaced Ancient Stone Slab (Peak Temple Vault)",
            SecretDoorArchetype::HighTechCamouflagedLocker => "High-Tech Camouflaged Armory Locker (Overlord Secret)",
        }
    }

    pub fn to_config(&self) -> SecretDoorConfig {
        match self {
            SecretDoorArchetype::FlushWallSlidingPanel => SecretDoorConfig {
                archetype: *self,
                concealment: SecretConcealmentType::BumpPressureSensitive,
                position: Vec3::new(0.0, 0.0, 0.0),
                door_width_uu: 64.0,
                door_height_uu: 128.0,
                door_thickness_uu: 16.0,
                travel_displacement: Vec3::new(-16.0, 96.0, 0.0),
                stay_open_time_secs: 4.0,
                open_time_secs: 1.2,
                encroach_behavior: "IgnoreWhenEncroached".to_string(),
                has_internal_exit_trigger: true,
                tag: "SecretWall0".to_string(),
                event: "OpenSecretChamber".to_string(),
            },
            SecretDoorArchetype::RotatingPivotingBookshelf => SecretDoorConfig {
                archetype: *self,
                concealment: SecretConcealmentType::ShootImpactTrigger,
                position: Vec3::new(0.0, 0.0, 0.0),
                door_width_uu: 72.0,
                door_height_uu: 144.0,
                door_thickness_uu: 24.0,
                travel_displacement: Vec3::new(48.0, 48.0, 0.0),
                stay_open_time_secs: 5.0,
                open_time_secs: 1.5,
                encroach_behavior: "ReturnWhenEncroached".to_string(),
                has_internal_exit_trigger: true,
                tag: "SecretBookshelf0".to_string(),
                event: "RevealLibraryVault".to_string(),
            },
            SecretDoorArchetype::SunkenFloorTrapdoor => SecretDoorConfig {
                archetype: *self,
                concealment: SecretConcealmentType::ProximityHiddenRadius,
                position: Vec3::new(0.0, 0.0, 0.0),
                door_width_uu: 80.0,
                door_height_uu: 80.0,
                door_thickness_uu: 12.0,
                travel_displacement: Vec3::new(0.0, 0.0, -128.0),
                stay_open_time_secs: 6.0,
                open_time_secs: 1.0,
                encroach_behavior: "IgnoreWhenEncroached".to_string(),
                has_internal_exit_trigger: true,
                tag: "SecretTrapdoor0".to_string(),
                event: "OpenCrawlway".to_string(),
            },
            SecretDoorArchetype::DisplacedStoneSlab => SecretDoorConfig {
                archetype: *self,
                concealment: SecretConcealmentType::BumpPressureSensitive,
                position: Vec3::new(0.0, 0.0, 0.0),
                door_width_uu: 96.0,
                door_height_uu: 160.0,
                door_thickness_uu: 32.0,
                travel_displacement: Vec3::new(0.0, 0.0, -160.0),
                stay_open_time_secs: 8.0,
                open_time_secs: 2.0,
                encroach_behavior: "CrushWhenEncroached".to_string(),
                has_internal_exit_trigger: false, // Intentionally testable hazard
                tag: "SecretStoneSlab0".to_string(),
                event: "DropTempleSlab".to_string(),
            },
            SecretDoorArchetype::HighTechCamouflagedLocker => SecretDoorConfig {
                archetype: *self,
                concealment: SecretConcealmentType::InteractiveUseKeypad,
                position: Vec3::new(0.0, 0.0, 0.0),
                door_width_uu: 64.0,
                door_height_uu: 120.0,
                door_thickness_uu: 16.0,
                travel_displacement: Vec3::new(-64.0, 0.0, 0.0),
                stay_open_time_secs: 4.5,
                open_time_secs: 0.8,
                encroach_behavior: "StopWhenEncroached".to_string(),
                has_internal_exit_trigger: true,
                tag: "SecretLocker0".to_string(),
                event: "UnlockArmoryLocker".to_string(),
            },
        }
    }
}

/// Mechanism used to reveal or trigger the secret passage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretConcealmentType {
    BumpPressureSensitive,
    ShootImpactTrigger,
    ProximityHiddenRadius,
    InteractiveUseKeypad,
}

impl SecretConcealmentType {
    pub fn display_name(&self) -> &'static str {
        match self {
            SecretConcealmentType::BumpPressureSensitive => "Bump Pressure-Sensitive (Push Wall Seam)",
            SecretConcealmentType::ShootImpactTrigger => "Shoot Impact Trigger (Hidden Target / Gong)",
            SecretConcealmentType::ProximityHiddenRadius => "Proximity Hidden Radius (Approach Silence)",
            SecretConcealmentType::InteractiveUseKeypad => "Interactive Use Keypad (Hidden Terminal)",
        }
    }
}

/// Configuration settings for a secret door mover and trigger network
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretDoorConfig {
    pub archetype: SecretDoorArchetype,
    pub concealment: SecretConcealmentType,
    pub position: Vec3,
    pub door_width_uu: f32,
    pub door_height_uu: f32,
    pub door_thickness_uu: f32,
    pub travel_displacement: Vec3,
    pub stay_open_time_secs: f32,
    pub open_time_secs: f32,
    pub encroach_behavior: String,
    pub has_internal_exit_trigger: bool,
    pub tag: String,
    pub event: String,
}

impl Default for SecretDoorConfig {
    fn default() -> Self {
        SecretDoorArchetype::FlushWallSlidingPanel.to_config()
    }
}

/// Context actor record for evaluating secret passage placement
#[derive(Debug, Clone, PartialEq)]
pub struct LevelSecretContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub tag: String,
    pub position: Vec3,
}

/// Telemetry metrics for a secret passage
#[derive(Debug, Clone, PartialEq)]
pub struct SecretDoorMetrics {
    pub stealth_rating_pct: f32,
    pub passage_cross_section_sq_uu: f32,
    pub transit_duration_secs: f32,
    pub total_secret_doors_in_level: usize,
}

/// Safety diagnostics for secret passages
#[derive(Debug, Clone, PartialEq)]
pub struct SecretDoorSafetyHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates stealth concealment rating, passage clearance, and transit duration
pub fn evaluate_secret_door_metrics(
    config: &SecretDoorConfig,
    peers: &[LevelSecretContextActorRecord],
) -> SecretDoorMetrics {
    // Stealth rating based on concealment type:
    // ShootImpact: 90% (hard to accidentally trigger)
    // UseKeypad: 80%
    // BumpPressure: 65%
    // Proximity: 50% (frequently triggered by accidental walking)
    let stealth = match config.concealment {
        SecretConcealmentType::ShootImpactTrigger => 90.0,
        SecretConcealmentType::InteractiveUseKeypad => 80.0,
        SecretConcealmentType::BumpPressureSensitive => 65.0,
        SecretConcealmentType::ProximityHiddenRadius => 50.0,
    };

    let cross_section = config.door_width_uu * config.door_height_uu;
    let transit_time = config.open_time_secs * 2.0 + config.stay_open_time_secs;

    let mut secrets_count = 1;
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("secret") || p.tag.to_lowercase().contains("secret") {
            secrets_count += 1;
        }
    }

    SecretDoorMetrics {
        stealth_rating_pct: stealth,
        passage_cross_section_sq_uu: cross_section,
        transit_duration_secs: transit_time,
        total_secret_doors_in_level: secrets_count,
    }
}

/// Audits safety for inescapable entrapment rooms, lethal crush encroachment, and rapid relock
pub fn audit_secret_door_safety(
    config: &SecretDoorConfig,
    _peers: &[LevelSecretContextActorRecord],
) -> Vec<SecretDoorSafetyHazard> {
    let mut hazards = Vec::new();

    // 1. Inescapable Entrapment Room Hazard
    if !config.has_internal_exit_trigger {
        hazards.push(SecretDoorSafetyHazard {
            title: "Inescapable Room Entrapment".to_string(),
            description: format!(
                "Secret door '{}' will automatically close after {:.1}s, but has no internal return trigger. Players entering the hidden chamber will be permanently trapped!",
                config.tag, config.stay_open_time_secs
            ),
            is_critical: true,
        });
    }

    // 2. Lethal Crush Encroachment Hazard
    if config.encroach_behavior.eq_ignore_ascii_case("CrushWhenEncroached") {
        hazards.push(SecretDoorSafetyHazard {
            title: "Lethal Crush Encroachment".to_string(),
            description: format!(
                "Secret door has 'CrushWhenEncroached' set. A player standing in the doorway when it closes will be instantly gibbed. Consider 'StopWhenEncroached' or 'ReturnWhenEncroached'.",
            ),
            is_critical: false,
        });
    }

    // 3. Instant Relock Lockout (< 1.5s stay open)
    if config.stay_open_time_secs < 1.5 {
        hazards.push(SecretDoorSafetyHazard {
            title: "Insufficient StayOpen Window".to_string(),
            description: format!(
                "StayOpen time of {:.1}s is too brief for a player running at standard speed (400 uu/s) to negotiate the threshold before the passage re-seals.",
                config.stay_open_time_secs
            ),
            is_critical: false,
        });
    }

    // 4. Narrow Passage Clearance (< 48 uu width)
    if config.door_width_uu < 48.0 {
        hazards.push(SecretDoorSafetyHazard {
            title: "Narrow Passage Clearance".to_string(),
            description: format!(
                "Passage width of {:.0} uu is tight for standard UT99 player collision (radius 17 uu = 34 uu diameter). Players may snag on the door jamb.",
                config.door_width_uu
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural CSG rectangular secret door slab brush
pub fn generate_secret_door_brush(config: &SecretDoorConfig) -> Brush {
    let mut brush = Brush::new("Secret_Door".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let w = config.door_width_uu;
    let h = config.door_height_uu;
    let t = config.door_thickness_uu;
    let half_w = w * 0.5;
    let half_t = t * 0.5;

    // 8 box vertices
    let p0 = Vec3::new(-half_w, -half_t, 0.0);
    let p1 = Vec3::new(half_w, -half_t, 0.0);
    let p2 = Vec3::new(half_w, half_t, 0.0);
    let p3 = Vec3::new(-half_w, half_t, 0.0);

    let p4 = Vec3::new(-half_w, -half_t, h);
    let p5 = Vec3::new(half_w, -half_t, h);
    let p6 = Vec3::new(half_w, half_t, h);
    let p7 = Vec3::new(-half_w, half_t, h);

    // Front (-Y)
    if let Some(mut poly) = Polygon::new(vec![p0, p1, p5, p4]) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    // Back (+Y)
    if let Some(mut poly) = Polygon::new(vec![p2, p3, p7, p6]) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    // Left (-X)
    if let Some(mut poly) = Polygon::new(vec![p3, p0, p4, p7]) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    // Right (+X)
    if let Some(mut poly) = Polygon::new(vec![p1, p2, p6, p5]) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    // Top (+Z)
    if let Some(mut poly) = Polygon::new(vec![p4, p5, p6, p7]) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    // Bottom (-Z)
    if let Some(mut poly) = Polygon::new(vec![p3, p2, p1, p0]) {
        poly.texture = "Indus1.Indus_B01".to_string();
        brush.polys.push(poly);
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_door_archetypes_and_defaults() {
        let wall = SecretDoorArchetype::FlushWallSlidingPanel.to_config();
        assert_eq!(wall.stay_open_time_secs, 4.0);
        assert_eq!(wall.door_width_uu, 64.0);
        assert!(wall.has_internal_exit_trigger);

        let stone = SecretDoorArchetype::DisplacedStoneSlab.to_config();
        assert_eq!(stone.door_height_uu, 160.0);
        assert!(!stone.has_internal_exit_trigger);
    }

    #[test]
    fn test_secret_door_metrics_evaluation() {
        let cfg = SecretDoorConfig::default(); // 64 x 128
        let peers = vec![
            LevelSecretContextActorRecord {
                name: "SecretDoorMover0".to_string(),
                class_name: "Engine.Mover".to_string(),
                tag: "SecretWall0".to_string(),
                position: Vec3::new(200.0, 0.0, 0.0),
            }
        ];

        let metrics = evaluate_secret_door_metrics(&cfg, &peers);
        assert_eq!(metrics.passage_cross_section_sq_uu, 64.0 * 128.0);
        assert_eq!(metrics.stealth_rating_pct, 65.0); // BumpPressureSensitive
        assert_eq!(metrics.total_secret_doors_in_level, 2);
    }

    #[test]
    fn test_secret_door_safety_hazard_entrapment() {
        let mut cfg = SecretDoorConfig::default();
        cfg.has_internal_exit_trigger = false;

        let hazards = audit_secret_door_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Inescapable Room Entrapment")));
    }

    #[test]
    fn test_generate_secret_door_brush() {
        let cfg = SecretDoorConfig::default();
        let brush = generate_secret_door_brush(&cfg);

        assert_eq!(brush.name, "Secret_Door");
        // 6 rectangular cube faces
        assert_eq!(brush.polys.len(), 6);
    }
}
