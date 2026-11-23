// crates/ut99-bsp/src/vent_duct_studio.rs
//
// Interactive Ventilation Duct, Crawlspace Shaft & Infiltration Grate Studio
// Hollow ventilation conduits, crouch crawlspace clearances, aerodynamic air currents,
// breakable infiltration grates, and procedural duct brush geometry.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical archetypes of ventilation crawlspaces and duct shafts in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VentDuctArchetype {
    AssaultOverlordAirVentilationBypass,
    SubterraneanMaintenanceDrainCrawlway,
    HighTechTurbineCoolingChute,
    CastleDungeonOublietteEscapeShaft,
    IndustrialCleanroomExhaustTrunk,
}

impl VentDuctArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            VentDuctArchetype::AssaultOverlordAirVentilationBypass => "Assault Air Ventilation Bypass (AS-Overlord Infiltration)",
            VentDuctArchetype::SubterraneanMaintenanceDrainCrawlway => "Subterranean Drain Crawlway (CTF-Gauntlet Sewer)",
            VentDuctArchetype::HighTechTurbineCoolingChute => "High-Tech Turbine Cooling Chute (DM-Turbine Vertical Vent)",
            VentDuctArchetype::CastleDungeonOublietteEscapeShaft => "Castle Dungeon Escape Shaft (AS-Rook Fortress Passage)",
            VentDuctArchetype::IndustrialCleanroomExhaustTrunk => "Industrial Cleanroom Exhaust Trunk (DM-Phobos HVAC Duct)",
        }
    }

    pub fn to_config(&self) -> VentDuctConfig {
        match self {
            VentDuctArchetype::AssaultOverlordAirVentilationBypass => VentDuctConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                duct_length_uu: 384.0,
                duct_width_uu: 64.0,
                duct_height_uu: 56.0, // Comfortable crouch height
                wall_thickness_uu: 8.0,
                air_draft_velocity: 150.0,
                has_intake_grate: true,
                has_exhaust_grate: true,
                grate_breakable: true,
                tag: "OverlordVentBypass0".to_string(),
                event: "VentGrateKicked".to_string(),
            },
            VentDuctArchetype::SubterraneanMaintenanceDrainCrawlway => VentDuctConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                duct_length_uu: 512.0,
                duct_width_uu: 72.0,
                duct_height_uu: 64.0,
                wall_thickness_uu: 12.0,
                air_draft_velocity: 60.0,
                has_intake_grate: true,
                has_exhaust_grate: false,
                grate_breakable: true,
                tag: "DrainCrawlway0".to_string(),
                event: "DrainGrateOpened".to_string(),
            },
            VentDuctArchetype::HighTechTurbineCoolingChute => VentDuctConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(0.0, 0.0, -1.0), // Vertical down
                duct_length_uu: 320.0,
                duct_width_uu: 80.0,
                duct_height_uu: 80.0,
                wall_thickness_uu: 8.0,
                air_draft_velocity: 300.0,
                has_intake_grate: true,
                has_exhaust_grate: true,
                grate_breakable: false, // Locked until triggered
                tag: "TurbineCoolingChute0".to_string(),
                event: "ChuteTriggered".to_string(),
            },
            VentDuctArchetype::CastleDungeonOublietteEscapeShaft => VentDuctConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                duct_length_uu: 448.0,
                duct_width_uu: 60.0,
                duct_height_uu: 52.0,
                wall_thickness_uu: 16.0,
                air_draft_velocity: 0.0,
                has_intake_grate: false,
                has_exhaust_grate: true,
                grate_breakable: true,
                tag: "CastleDungeonShaft0".to_string(),
                event: "OublietteEscaped".to_string(),
            },
            VentDuctArchetype::IndustrialCleanroomExhaustTrunk => VentDuctConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                duct_length_uu: 640.0,
                duct_width_uu: 96.0,
                duct_height_uu: 72.0,
                wall_thickness_uu: 8.0,
                air_draft_velocity: 250.0,
                has_intake_grate: true,
                has_exhaust_grate: true,
                grate_breakable: true,
                tag: "CleanroomExhaustTrunk0".to_string(),
                event: "ExhaustTrunkBreached".to_string(),
            },
        }
    }
}

/// Configuration settings for a ventilation crawlspace duct and infiltration grate
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VentDuctConfig {
    pub archetype: VentDuctArchetype,
    pub position: Vec3,
    pub facing_direction: Vec3,
    pub duct_length_uu: f32,
    pub duct_width_uu: f32,
    pub duct_height_uu: f32,
    pub wall_thickness_uu: f32,
    pub air_draft_velocity: f32,
    pub has_intake_grate: bool,
    pub has_exhaust_grate: bool,
    pub grate_breakable: bool,
    pub tag: String,
    pub event: String,
}

impl Default for VentDuctConfig {
    fn default() -> Self {
        VentDuctArchetype::AssaultOverlordAirVentilationBypass.to_config()
    }
}

/// Context actor record for evaluating vent duct neighborhood
#[derive(Debug, Clone, PartialEq)]
pub struct LevelVentContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Telemetry metrics for a ventilation duct installation
#[derive(Debug, Clone, PartialEq)]
pub struct VentDuctMetrics {
    pub internal_clearance_cross_section_sq_uu: f32,
    pub player_crouch_transit_time_secs: f32,
    pub aerodynamic_convection_force: f32,
    pub total_vent_ducts_in_level: usize,
}

/// Safety diagnostics for ventilation duct crawlspaces
#[derive(Debug, Clone, PartialEq)]
pub struct VentDuctHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates internal clearance area, crouch transit duration, and convection draft
pub fn evaluate_vent_duct_metrics(
    config: &VentDuctConfig,
    peers: &[LevelVentContextActorRecord],
) -> VentDuctMetrics {
    let internal_w = (config.duct_width_uu - config.wall_thickness_uu * 2.0).max(1.0);
    let internal_h = (config.duct_height_uu - config.wall_thickness_uu * 2.0).max(1.0);
    let cross_section = internal_w * internal_h;

    // Standard player crouch speed in UT99 is ~160 uu/s
    // Effective transit speed modified by air draft velocity along duct
    let effective_speed = (160.0 + config.air_draft_velocity * 0.5).max(40.0);
    let transit_time = config.duct_length_uu / effective_speed;

    let convection_force = config.air_draft_velocity * 1.5;

    let mut ducts_count = 1;
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("mover") || cls_lower.contains("trigger") || cls_lower.contains("duct") {
            ducts_count += 1;
        }
    }

    VentDuctMetrics {
        internal_clearance_cross_section_sq_uu: cross_section,
        player_crouch_transit_time_secs: transit_time,
        aerodynamic_convection_force: convection_force,
        total_vent_ducts_in_level: ducts_count,
    }
}

/// Audits safety for crouch height clearance, trapped dead-end ducts, and high convection
pub fn audit_vent_duct_safety(
    config: &VentDuctConfig,
    _peers: &[LevelVentContextActorRecord],
) -> Vec<VentDuctHazard> {
    let mut hazards = Vec::new();

    let internal_h = config.duct_height_uu - config.wall_thickness_uu * 2.0;

    // 1. Insufficient Player Crouch Height (< 40 uu)
    if internal_h < 40.0 {
        hazards.push(VentDuctHazard {
            title: "Insufficient Player Crouch Clearance".to_string(),
            description: format!(
                "Internal duct height is only {:.0} uu (minimum required for UT99 crouch collision is 40-44 uu). Players will collide with the ceiling and cannot crawl through.",
                internal_h
            ),
            is_critical: true,
        });
    }

    // 2. Trapped Dead-End Duct (Unbreakable grates on both ends)
    if config.has_intake_grate && config.has_exhaust_grate && !config.grate_breakable && config.event.is_empty() {
        hazards.push(VentDuctHazard {
            title: "Trapped Dead-End Ventilation Duct".to_string(),
            description: "Duct has solid grates on both intake and exhaust, grates are unbreakable, and no unlock event is configured. Players inside would be permanently entrapped.".to_string(),
            is_critical: true,
        });
    }

    // 3. Excessive Convection Drag (> 600 uu/s)
    if config.air_draft_velocity > 600.0 {
        hazards.push(VentDuctHazard {
            title: "Excessive Air Current Velocity".to_string(),
            description: format!(
                "Air draft velocity of {:.0} uu/s causes extreme aerodynamic drag, pushing players violently and preventing reverse navigation.",
                config.air_draft_velocity
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural CSG hollow rectangular duct conduit brush with 4 surrounding wall slabs
pub fn generate_vent_duct_brush(config: &VentDuctConfig) -> Brush {
    let mut brush = Brush::new("Vent_Duct".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let l = config.duct_length_uu;
    let w = config.duct_width_uu;
    let h = config.duct_height_uu;
    let t = config.wall_thickness_uu;

    let half_w = w * 0.5;
    let half_h = h * 0.5;

    // 1. Bottom Floor Slab (-Z)
    add_box_to_brush(&mut brush, 0.0, l, -half_w, half_w, -half_h, -half_h + t, "Indus1.Indus_B01");

    // 2. Top Ceiling Slab (+Z)
    add_box_to_brush(&mut brush, 0.0, l, -half_w, half_w, half_h - t, half_h, "Indus1.Indus_B01");

    // 3. Left Wall Slab (-Y)
    add_box_to_brush(&mut brush, 0.0, l, -half_w, -half_w + t, -half_h + t, half_h - t, "Indus1.Indus_B01");

    // 4. Right Wall Slab (+Y)
    add_box_to_brush(&mut brush, 0.0, l, half_w - t, half_w, -half_h + t, half_h - t, "Indus1.Indus_B01");

    // 5. Intake Grate Sheet at X = 0 (if enabled)
    if config.has_intake_grate {
        let p0 = Vec3::new(0.0, -half_w + t, -half_h + t);
        let p1 = Vec3::new(0.0, half_w - t, -half_h + t);
        let p2 = Vec3::new(0.0, half_w - t, half_h - t);
        let p3 = Vec3::new(0.0, -half_w + t, half_h - t);

        if let Some(mut poly) = Polygon::new(vec![p0, p1, p2, p3]) {
            poly.texture = "Indus1.Grating.Grate01".to_string();
            poly.flags = crate::PF_MASKED | crate::PF_TWO_SIDED;
            brush.polys.push(poly);
        }
    }

    // 6. Exhaust Grate Sheet at X = l (if enabled)
    if config.has_exhaust_grate {
        let p0 = Vec3::new(l, -half_w + t, -half_h + t);
        let p1 = Vec3::new(l, half_w - t, -half_h + t);
        let p2 = Vec3::new(l, half_w - t, half_h - t);
        let p3 = Vec3::new(l, -half_w + t, half_h - t);

        if let Some(mut poly) = Polygon::new(vec![p0, p1, p2, p3]) {
            poly.texture = "Indus1.Grating.Grate01".to_string();
            poly.flags = crate::PF_MASKED | crate::PF_TWO_SIDED;
            brush.polys.push(poly);
        }
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
    fn test_vent_duct_archetypes_and_defaults() {
        let bypass = VentDuctArchetype::AssaultOverlordAirVentilationBypass.to_config();
        assert_eq!(bypass.duct_length_uu, 384.0);
        assert_eq!(bypass.duct_height_uu, 56.0);
        assert!(bypass.has_intake_grate);
        assert!(bypass.grate_breakable);

        let chute = VentDuctArchetype::HighTechTurbineCoolingChute.to_config();
        assert_eq!(chute.air_draft_velocity, 300.0);
        assert!(!chute.grate_breakable);
    }

    #[test]
    fn test_vent_duct_metrics_evaluation() {
        let cfg = VentDuctConfig::default(); // 384 length, 64x56, 8 wall, 150 draft
        let peers = vec![
            LevelVentContextActorRecord {
                name: "GrateMover0".to_string(),
                class_name: "Engine.Mover".to_string(),
                position: Vec3::new(0.0, 0.0, 0.0),
            }
        ];

        let metrics = evaluate_vent_duct_metrics(&cfg, &peers);
        // Internal w = 64 - 16 = 48, internal h = 56 - 16 = 40 -> 1920 sq uu
        assert_eq!(metrics.internal_clearance_cross_section_sq_uu, 48.0 * 40.0);
        assert_eq!(metrics.aerodynamic_convection_force, 150.0 * 1.5);
        assert_eq!(metrics.total_vent_ducts_in_level, 2);
    }

    #[test]
    fn test_vent_duct_safety_hazard_crouch() {
        let mut cfg = VentDuctConfig::default();
        cfg.duct_height_uu = 40.0;
        cfg.wall_thickness_uu = 8.0; // 40 - 16 = 24 uu < 40 uu

        let hazards = audit_vent_duct_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Insufficient Player Crouch Clearance")));
    }

    #[test]
    fn test_generate_vent_duct_brush() {
        let cfg = VentDuctConfig::default();
        let brush = generate_vent_duct_brush(&cfg);

        assert_eq!(brush.name, "Vent_Duct");
        // 4 wall slabs (6 polys each = 24) + 1 intake grate + 1 exhaust grate = 26 polys
        assert_eq!(brush.polys.len(), 26);
    }
}
