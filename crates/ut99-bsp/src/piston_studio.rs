// crates/ut99-bsp/src/piston_studio.rs
//
// Interactive Industrial Steam Piston, Hydraulic Compactor & Trap Hazard Studio
// Kinematic stroke analysis, crushing force, player squish lethality, encroachment handling,
// warning siren latency, and procedural compactor brush geometry.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical archetypes of industrial steam pistons and compactor traps in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PistonArchetype {
    PressureChamberVerticalCrusher,
    HorizontalBulkheadRam,
    ReciprocatingSteamPiston,
    SubterraneanCompactorJaw,
    HighSpeedPneumaticStamper,
}

impl PistonArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            PistonArchetype::PressureChamberVerticalCrusher => "Pressure Chamber Vertical Crusher (DM-Pressure Compactor)",
            PistonArchetype::HorizontalBulkheadRam => "Horizontal Bulkhead Ram (DM-Crane Industrial Catwalk)",
            PistonArchetype::ReciprocatingSteamPiston => "Reciprocating Steam Piston (DM-Phobos Reactor Core)",
            PistonArchetype::SubterraneanCompactorJaw => "Subterranean Compactor Jaw (DM-Peak Slag Chasm)",
            PistonArchetype::HighSpeedPneumaticStamper => "High-Speed Pneumatic Stamper (AS-Overlord Mechanical Chokepoint)",
        }
    }

    pub fn to_config(&self) -> PistonConfig {
        match self {
            PistonArchetype::PressureChamberVerticalCrusher => PistonConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 384.0),
                orientation: PistonOrientation::VerticalDown,
                stroke_length_uu: 384.0,
                shaft_radius_uu: 24.0,
                head_width_uu: 192.0,
                head_depth_uu: 192.0,
                head_thickness_uu: 32.0,
                stroke_velocity_uu_s: 240.0,
                wait_time_s: 1.5,
                squish_damage: 1000, // Instant Gib
                warning_lead_time_s: 1.0,
                encroachment: PistonEncroachAction::CrushWhenEncroach,
                has_warning_siren: true,
                has_steam_emitter: true,
                has_abort_button: true,
                tag: "PressureCrusher0".to_string(),
                event: "PressurePistonActivated".to_string(),
            },
            PistonArchetype::HorizontalBulkheadRam => PistonConfig {
                archetype: *self,
                position: Vec3::new(-256.0, 0.0, 64.0),
                orientation: PistonOrientation::HorizontalX,
                stroke_length_uu: 256.0,
                shaft_radius_uu: 16.0,
                head_width_uu: 32.0,
                head_depth_uu: 128.0,
                head_thickness_uu: 96.0,
                stroke_velocity_uu_s: 320.0,
                wait_time_s: 1.0,
                squish_damage: 75,
                warning_lead_time_s: 0.8,
                encroachment: PistonEncroachAction::StopWhenEncroach,
                has_warning_siren: true,
                has_steam_emitter: false,
                has_abort_button: true,
                tag: "BulkheadRam0".to_string(),
                event: "RamTriggered".to_string(),
            },
            PistonArchetype::ReciprocatingSteamPiston => PistonConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 200.0),
                orientation: PistonOrientation::VerticalDown,
                stroke_length_uu: 192.0,
                shaft_radius_uu: 20.0,
                head_width_uu: 80.0,
                head_depth_uu: 80.0,
                head_thickness_uu: 24.0,
                stroke_velocity_uu_s: 160.0,
                wait_time_s: 0.5,
                squish_damage: 50,
                warning_lead_time_s: 0.5,
                encroachment: PistonEncroachAction::StopWhenEncroach,
                has_warning_siren: false,
                has_steam_emitter: true,
                has_abort_button: false,
                tag: "SteamPiston0".to_string(),
                event: "PistonCycled".to_string(),
            },
            PistonArchetype::SubterraneanCompactorJaw => PistonConfig {
                archetype: *self,
                position: Vec3::new(0.0, -320.0, -128.0),
                orientation: PistonOrientation::HorizontalY,
                stroke_length_uu: 320.0,
                shaft_radius_uu: 28.0,
                head_width_uu: 256.0,
                head_depth_uu: 48.0,
                head_thickness_uu: 96.0,
                stroke_velocity_uu_s: 180.0,
                wait_time_s: 2.0,
                squish_damage: 500, // Fatal crusher
                warning_lead_time_s: 1.5,
                encroachment: PistonEncroachAction::CrushWhenEncroach,
                has_warning_siren: true,
                has_steam_emitter: true,
                has_abort_button: false,
                tag: "CompactorJaw0".to_string(),
                event: "JawCycleStarted".to_string(),
            },
            PistonArchetype::HighSpeedPneumaticStamper => PistonConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 160.0),
                orientation: PistonOrientation::VerticalDown,
                stroke_length_uu: 160.0,
                shaft_radius_uu: 18.0,
                head_width_uu: 80.0,
                head_depth_uu: 80.0,
                head_thickness_uu: 40.0,
                stroke_velocity_uu_s: 400.0,
                wait_time_s: 0.4,
                squish_damage: 150, // Fatal to 100 HP unarmored
                warning_lead_time_s: 0.3,
                encroachment: PistonEncroachAction::CrushWhenEncroach,
                has_warning_siren: true,
                has_steam_emitter: true,
                has_abort_button: false,
                tag: "PneumaticStamper0".to_string(),
                event: "StamperHit".to_string(),
            },
        }
    }
}

/// Physical stroke orientation vector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PistonOrientation {
    VerticalDown,
    VerticalUp,
    HorizontalX,
    HorizontalY,
}

impl PistonOrientation {
    pub fn direction_vector(&self) -> Vec3 {
        match self {
            PistonOrientation::VerticalDown => Vec3::new(0.0, 0.0, -1.0),
            PistonOrientation::VerticalUp => Vec3::new(0.0, 0.0, 1.0),
            PistonOrientation::HorizontalX => Vec3::new(1.0, 0.0, 0.0),
            PistonOrientation::HorizontalY => Vec3::new(0.0, 1.0, 0.0),
        }
    }
}

/// Encroachment behavior when hitting a player
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PistonEncroachAction {
    CrushWhenEncroach,
    StopWhenEncroach,
    ReturnWhenEncroach,
}

impl PistonEncroachAction {
    pub fn display_name(&self) -> &'static str {
        match self {
            PistonEncroachAction::CrushWhenEncroach => "Crush When Encroach (Inflict Fatal Squish Damage)",
            PistonEncroachAction::StopWhenEncroach => "Stop When Encroach (Halt Motion On Contact)",
            PistonEncroachAction::ReturnWhenEncroach => "Return When Encroach (Reverse Stroke On Contact)",
        }
    }
}

/// Complete configuration of an industrial piston compactor trap
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PistonConfig {
    pub archetype: PistonArchetype,
    pub position: Vec3,
    pub orientation: PistonOrientation,
    pub stroke_length_uu: f32,
    pub shaft_radius_uu: f32,
    pub head_width_uu: f32,
    pub head_depth_uu: f32,
    pub head_thickness_uu: f32,
    pub stroke_velocity_uu_s: f32,
    pub wait_time_s: f32,
    pub squish_damage: i32,
    pub warning_lead_time_s: f32,
    pub encroachment: PistonEncroachAction,
    pub has_warning_siren: bool,
    pub has_steam_emitter: bool,
    pub has_abort_button: bool,
    pub tag: String,
    pub event: String,
}

impl Default for PistonConfig {
    fn default() -> Self {
        PistonArchetype::PressureChamberVerticalCrusher.to_config()
    }
}

/// Evaluated telemetry and kinematic metrics for an industrial piston
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PistonMetrics {
    pub stroke_length_uu: f32,
    pub one_way_travel_time_s: f32,
    pub full_cycle_period_s: f32,
    pub compression_impulse_force: f32,
    pub is_instant_kill: bool,
    pub effective_dps: f32,
    pub compactor_surface_area_uu2: f32,
    pub head_volume_uu3: f32,
    pub total_pistons_in_level: usize,
}

/// Context actor record for safety checking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelPistonContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Compute kinematic and impact metrics for the piston configuration
pub fn evaluate_piston_metrics(
    config: &PistonConfig,
    peers: &[LevelPistonContextActorRecord],
) -> PistonMetrics {
    let travel_time = if config.stroke_velocity_uu_s > 0.0 {
        config.stroke_length_uu / config.stroke_velocity_uu_s
    } else {
        0.0
    };

    let full_cycle = (travel_time * 2.0) + (config.wait_time_s * 2.0);
    let area = config.head_width_uu * config.head_depth_uu;
    let volume = area * config.head_thickness_uu;
    let is_instant_kill = config.squish_damage >= 100;

    let effective_dps = if full_cycle > 0.0 {
        config.squish_damage as f32 / full_cycle
    } else {
        0.0
    };

    // Kinetic compression impulse ~ mass (volume / 10000) * velocity
    let compression_impulse_force = (volume / 10000.0) * config.stroke_velocity_uu_s;

    let mut count = 1;
    for p in peers {
        let cls = p.class_name.to_lowercase();
        if cls.contains("mover") || cls.contains("piston") || cls.contains("crusher") {
            count += 1;
        }
    }

    PistonMetrics {
        stroke_length_uu: config.stroke_length_uu,
        one_way_travel_time_s: travel_time,
        full_cycle_period_s: full_cycle,
        compression_impulse_force,
        is_instant_kill,
        effective_dps,
        compactor_surface_area_uu2: area,
        head_volume_uu3: volume,
        total_pistons_in_level: count,
    }
}

/// Potential safety hazards and gameplay balancing alerts for pistons
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PistonHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Audit safety and gameplay hazards for the piston setup
pub fn audit_piston_safety(
    config: &PistonConfig,
    _nearby_actors: &[LevelPistonContextActorRecord],
) -> Vec<PistonHazard> {
    let mut hazards = Vec::new();

    // 1. Inescapable lockout
    if config.squish_damage >= 100 && !config.has_abort_button && config.stroke_length_uu >= 256.0 {
        hazards.push(PistonHazard {
            title: "Inescapable Compactor Lockout Hazard".to_string(),
            description: "Piston creates an inescapable crushing chamber without an emergency abort console or return trigger.".to_string(),
            is_critical: true,
        });
    }

    // 2. Lethal squish without warning
    if config.squish_damage >= 100 && (!config.has_warning_siren || config.warning_lead_time_s < 0.4) {
        hazards.push(PistonHazard {
            title: "Lethal Squish Without Adequate Warning".to_string(),
            description: format!(
                "Piston inflicts lethal squish damage ({} HP) but warning lead time is only {:.1}s with warning siren: {}.",
                config.squish_damage, config.warning_lead_time_s, config.has_warning_siren
            ),
            is_critical: true,
        });
    }

    // 3. Extreme velocity
    if config.stroke_velocity_uu_s > 1000.0 {
        hazards.push(PistonHazard {
            title: "Extreme Piston Velocity Hazard".to_string(),
            description: format!(
                "Piston compression velocity of {:.0} uu/s exceeds 1000 uu/s risking physics collision tunneling through players.",
                config.stroke_velocity_uu_s
            ),
            is_critical: false,
        });
    }

    // 4. Zero wait time entrapment
    if config.wait_time_s <= 0.15 {
        hazards.push(PistonHazard {
            title: "Zero Wait Time Entrapment Hazard".to_string(),
            description: format!(
                "Piston dwell time at stroke apex is only {:.2}s, giving players insufficient time to react or jump away.",
                config.wait_time_s
            ),
            is_critical: true,
        });
    }

    hazards
}

/// Generates a solid watertight CSG brush for the piston compactor head and shaft
pub fn generate_piston_brush(config: &PistonConfig) -> Brush {
    let mut brush = Brush::new("Industrial_Piston", CsgOperation::Add);
    let hw = config.head_width_uu * 0.5;
    let hd = config.head_depth_uu * 0.5;
    let ht = config.head_thickness_uu * 0.5;
    let sr = config.shaft_radius_uu;
    let shaft_len = config.stroke_length_uu;

    // 1. Compactor Head Box (Rectangular prism centered at config.position)
    let p = config.position;
    let c_p0 = p + Vec3::new(-hw, -hd, -ht);
    let c_p1 = p + Vec3::new(hw, -hd, -ht);
    let c_p2 = p + Vec3::new(hw, hd, -ht);
    let c_p3 = p + Vec3::new(-hw, hd, -ht);
    let c_p4 = p + Vec3::new(-hw, -hd, ht);
    let c_p5 = p + Vec3::new(hw, -hd, ht);
    let c_p6 = p + Vec3::new(hw, hd, ht);
    let c_p7 = p + Vec3::new(-hw, hd, ht);

    let tex_head = "Indus1.Metal.Iron01";
    let tex_shaft = "Indus1.Metal.PistonSteel";

    // Head Bottom (-Z)
    if let Some(mut poly) = Polygon::new(vec![c_p0, c_p3, c_p2, c_p1]) { poly.texture = tex_head.to_string(); brush.polys.push(poly); }
    // Head Top (+Z)
    if let Some(mut poly) = Polygon::new(vec![c_p4, c_p5, c_p6, c_p7]) { poly.texture = tex_head.to_string(); brush.polys.push(poly); }
    // Head Front (-Y)
    if let Some(mut poly) = Polygon::new(vec![c_p0, c_p1, c_p5, c_p4]) { poly.texture = tex_head.to_string(); brush.polys.push(poly); }
    // Head Right (+X)
    if let Some(mut poly) = Polygon::new(vec![c_p1, c_p2, c_p6, c_p5]) { poly.texture = tex_head.to_string(); brush.polys.push(poly); }
    // Head Back (+Y)
    if let Some(mut poly) = Polygon::new(vec![c_p2, c_p3, c_p7, c_p6]) { poly.texture = tex_head.to_string(); brush.polys.push(poly); }
    // Head Left (-X)
    if let Some(mut poly) = Polygon::new(vec![c_p3, c_p0, c_p4, c_p7]) { poly.texture = tex_head.to_string(); brush.polys.push(poly); }

    // 2. Shaft (Hexagonal cylinder extending in reverse stroke direction)
    let rev_dir = -config.orientation.direction_vector();
    let shaft_base = p;
    let shaft_top = p + rev_dir * shaft_len;

    // Generate 6-sided shaft prism
    let sides = 6;
    let mut base_pts = Vec::new();
    let mut top_pts = Vec::new();

    // Determine orthogonal basis for shaft cross-section
    let u_axis = if rev_dir.z.abs() > 0.8 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 0.0, 1.0).cross(rev_dir).normalize()
    };
    let v_axis = rev_dir.cross(u_axis).normalize();

    for i in 0..sides {
        let angle = (i as f32) * (std::f32::consts::TAU / sides as f32);
        let offset = (u_axis * angle.cos() + v_axis * angle.sin()) * sr;
        base_pts.push(shaft_base + offset);
        top_pts.push(shaft_top + offset);
    }

    // Shaft side faces
    for i in 0..sides {
        let next = (i + 1) % sides;
        if let Some(mut poly) = Polygon::new(vec![
            base_pts[i],
            base_pts[next],
            top_pts[next],
            top_pts[i],
        ]) {
            poly.texture = tex_shaft.to_string();
            brush.polys.push(poly);
        }
    }

    // Shaft top cap
    if let Some(mut poly) = Polygon::new(top_pts) {
        poly.texture = tex_shaft.to_string();
        brush.polys.push(poly);
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piston_archetypes_and_defaults() {
        let archetypes = [
            PistonArchetype::PressureChamberVerticalCrusher,
            PistonArchetype::HorizontalBulkheadRam,
            PistonArchetype::ReciprocatingSteamPiston,
            PistonArchetype::SubterraneanCompactorJaw,
            PistonArchetype::HighSpeedPneumaticStamper,
        ];

        for a in &archetypes {
            let config = a.to_config();
            assert_eq!(config.archetype, *a);
            assert!(config.stroke_length_uu > 0.0);
            assert!(config.head_width_uu > 0.0);
            assert!(config.stroke_velocity_uu_s > 0.0);
            let metrics = evaluate_piston_metrics(&config, &[]);
            assert!(metrics.one_way_travel_time_s > 0.0);
            assert!(metrics.full_cycle_period_s > 0.0);
            assert_eq!(metrics.total_pistons_in_level, 1);
        }
    }

    #[test]
    fn test_piston_metrics_calculation() {
        let config = PistonArchetype::PressureChamberVerticalCrusher.to_config();
        let metrics = evaluate_piston_metrics(&config, &[]);

        assert_eq!(metrics.stroke_length_uu, 384.0);
        assert!((metrics.one_way_travel_time_s - (384.0 / 240.0)).abs() < 1e-4);
        assert!(metrics.is_instant_kill);
        assert!(metrics.compactor_surface_area_uu2 > 10000.0);
        assert!(metrics.head_volume_uu3 > 100000.0);
    }

    #[test]
    fn test_piston_safety_audit() {
        let mut config = PistonArchetype::PressureChamberVerticalCrusher.to_config();
        config.has_abort_button = false;
        config.has_warning_siren = false;
        config.stroke_velocity_uu_s = 1200.0;
        config.wait_time_s = 0.1;

        let hazards = audit_piston_safety(&config, &[]);
        assert!(hazards.iter().any(|h| h.title.contains("Lockout")));
        assert!(hazards.iter().any(|h| h.title.contains("Lethal Squish")));
        assert!(hazards.iter().any(|h| h.title.contains("Extreme Piston Velocity")));
        assert!(hazards.iter().any(|h| h.title.contains("Zero Wait Time")));
    }

    #[test]
    fn test_generate_piston_brush() {
        let config = PistonArchetype::PressureChamberVerticalCrusher.to_config();
        let brush = generate_piston_brush(&config);

        // 6 box faces + 6 cylinder side faces + 1 cylinder top cap = 13 faces
        assert_eq!(brush.polys.len(), 13);
        assert_eq!(brush.csg_oper, CsgOperation::Add);
        assert_eq!(brush.name, "Industrial_Piston");
    }
}
