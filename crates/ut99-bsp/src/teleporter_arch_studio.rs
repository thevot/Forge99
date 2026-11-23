// crates/ut99-bsp/src/teleporter_arch_studio.rs
//
// Interactive Teleporter Portal Arch, Dimensional Vortex & Gate Studio
// Physical portal archway geometry, planar vortex sheets, velocity boost redirection,
// destination node link integrity, and procedural CSG arch brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical archetypes of teleporter portal gates in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeleporterArchArchetype {
    GothicMonolithGate,
    IndustrialSludgeDrainTeleport,
    CosmicVoidStargate,
    HydraulicBulkheadPortal,
    SubterraneanCavernRift,
}

impl TeleporterArchArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            TeleporterArchArchetype::GothicMonolithGate => "Gothic Monolith Portal Gate (CTF-Face Base Arch)",
            TeleporterArchArchetype::IndustrialSludgeDrainTeleport => "Industrial Sludge Drain Teleport (DM-Deck16 Portal)",
            TeleporterArchArchetype::CosmicVoidStargate => "Cosmic Void Stargate (DM-Fractal Space Gate)",
            TeleporterArchArchetype::HydraulicBulkheadPortal => "Hydraulic Bulkhead Portal (CTF-Coret High-Tech)",
            TeleporterArchArchetype::SubterraneanCavernRift => "Subterranean Cavern Rift (DM-Peak Temple Arch)",
        }
    }

    pub fn to_config(&self) -> TeleporterArchConfig {
        match self {
            TeleporterArchArchetype::GothicMonolithGate => TeleporterArchConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                arch_width_uu: 128.0,
                arch_height_uu: 160.0,
                pillar_thickness_uu: 24.0,
                exit_boost_velocity: 300.0,
                b_changes_velocity: true,
                b_changes_yaw: true,
                url: "RedBaseExit0".to_string(),
                tag: "RedPortalEntrance".to_string(),
            },
            TeleporterArchArchetype::IndustrialSludgeDrainTeleport => TeleporterArchConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                arch_width_uu: 96.0,
                arch_height_uu: 128.0,
                pillar_thickness_uu: 16.0,
                exit_boost_velocity: 150.0,
                b_changes_velocity: false,
                b_changes_yaw: true,
                url: "SludgeDrainExit0".to_string(),
                tag: "DeckDrainPortal".to_string(),
            },
            TeleporterArchArchetype::CosmicVoidStargate => TeleporterArchConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                arch_width_uu: 160.0,
                arch_height_uu: 192.0,
                pillar_thickness_uu: 32.0,
                exit_boost_velocity: 600.0,
                b_changes_velocity: true,
                b_changes_yaw: true,
                url: "CosmicVoidExit0".to_string(),
                tag: "FractalGate0".to_string(),
            },
            TeleporterArchArchetype::HydraulicBulkheadPortal => TeleporterArchConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                arch_width_uu: 112.0,
                arch_height_uu: 144.0,
                pillar_thickness_uu: 20.0,
                exit_boost_velocity: 250.0,
                b_changes_velocity: true,
                b_changes_yaw: true,
                url: "EngineRoomExit0".to_string(),
                tag: "CoretBulkheadPortal".to_string(),
            },
            TeleporterArchArchetype::SubterraneanCavernRift => TeleporterArchConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                arch_width_uu: 104.0,
                arch_height_uu: 136.0,
                pillar_thickness_uu: 28.0,
                exit_boost_velocity: 200.0,
                b_changes_velocity: false,
                b_changes_yaw: false,
                url: "CavernExit0".to_string(),
                tag: "PeakRiftGate".to_string(),
            },
        }
    }
}

/// Configuration parameters for a teleporter portal archway
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeleporterArchConfig {
    pub archetype: TeleporterArchArchetype,
    pub position: Vec3,
    pub facing_direction: Vec3,
    pub arch_width_uu: f32,
    pub arch_height_uu: f32,
    pub pillar_thickness_uu: f32,
    pub exit_boost_velocity: f32,
    pub b_changes_velocity: bool,
    pub b_changes_yaw: bool,
    pub url: String,
    pub tag: String,
}

impl Default for TeleporterArchConfig {
    fn default() -> Self {
        TeleporterArchArchetype::GothicMonolithGate.to_config()
    }
}

/// Context actor record for evaluating teleporter network links
#[derive(Debug, Clone, PartialEq)]
pub struct LevelTeleportContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub tag: String,
    pub position: Vec3,
}

/// Telemetry metrics for a teleporter portal arch
#[derive(Debug, Clone, PartialEq)]
pub struct TeleporterArchMetrics {
    pub portal_aperture_area_sq_uu: f32,
    pub exit_launch_speed: f32,
    pub destination_clearance_uu: f32,
    pub total_portal_gates_in_level: usize,
}

/// Safety diagnostics for teleporter portal gates
#[derive(Debug, Clone, PartialEq)]
pub struct TeleporterArchHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates portal aperture area, exit launch boost, and destination clearance
pub fn evaluate_teleporter_arch_metrics(
    config: &TeleporterArchConfig,
    peers: &[LevelTeleportContextActorRecord],
) -> TeleporterArchMetrics {
    let aperture_w = (config.arch_width_uu - config.pillar_thickness_uu * 2.0).max(16.0);
    let aperture_h = (config.arch_height_uu - config.pillar_thickness_uu).max(16.0);
    let aperture_area = aperture_w * aperture_h;

    // Find destination node clearance
    let mut clearance = 256.0;
    for p in peers {
        if p.tag.eq_ignore_ascii_case(&config.url) {
            // Find closest neighbor to destination
            let mut min_neighbor_dist = f32::MAX;
            for other in peers {
                if other.name != p.name {
                    let d = (other.position - p.position).length();
                    if d < min_neighbor_dist {
                        min_neighbor_dist = d;
                    }
                }
            }
            if min_neighbor_dist < f32::MAX {
                clearance = min_neighbor_dist;
            }
        }
    }

    let mut gates_count = 1;
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("teleporter") || p.tag.to_lowercase().contains("portal") {
            gates_count += 1;
        }
    }

    TeleporterArchMetrics {
        portal_aperture_area_sq_uu: aperture_area,
        exit_launch_speed: if config.b_changes_velocity { config.exit_boost_velocity } else { 0.0 },
        destination_clearance_uu: clearance,
        total_portal_gates_in_level: gates_count,
    }
}

/// Audits safety for unlinked destination URLs, telefrag proximity, and tight apertures
pub fn audit_teleporter_arch_safety(
    config: &TeleporterArchConfig,
    peers: &[LevelTeleportContextActorRecord],
) -> Vec<TeleporterArchHazard> {
    let mut hazards = Vec::new();

    // 1. Unlinked Destination URL (Orphan portal)
    if !config.url.is_empty() && !peers.is_empty() {
        let exists = peers.iter().any(|p| p.tag.eq_ignore_ascii_case(&config.url));
        if !exists {
            hazards.push(TeleporterArchHazard {
                title: "Unlinked Destination Portal URL".to_string(),
                description: format!(
                    "Teleporter specifies URL '{}', but no actor in the map has a matching Tag! Entering players will not be transported.",
                    config.url
                ),
                is_critical: true,
            });
        }
    }

    // 2. Telefrag Hazard (< 64 uu clearance at destination)
    for p in peers {
        if p.tag.eq_ignore_ascii_case(&config.url) {
            for other in peers {
                if other.name != p.name {
                    let d = (other.position - p.position).length();
                    if d < 64.0 {
                        hazards.push(TeleporterArchHazard {
                            title: "Telefrag Hazard at Destination".to_string(),
                            description: format!(
                                "Destination '{}' is dangerously close ({:.0} uu) to actor '{}'. Arriving players may instantly telefrag teammates or snag geometry!",
                                p.name, d, other.name
                            ),
                            is_critical: true,
                        });
                    }
                }
            }
        }
    }

    // 3. Narrow Portal Aperture (< 48 uu clearance)
    let net_width = config.arch_width_uu - config.pillar_thickness_uu * 2.0;
    if net_width < 48.0 {
        hazards.push(TeleporterArchHazard {
            title: "Restricted Portal Aperture".to_string(),
            description: format!(
                "Passage clearance between pillars is only {:.0} uu (standard player collision is 34 uu diameter). Players may collide with the portal frame.",
                net_width
            ),
            is_critical: false,
        });
    }

    // 4. Excessive Exit Boost Velocity (> 1000 uu/s)
    if config.b_changes_velocity && config.exit_boost_velocity > 1000.0 {
        hazards.push(TeleporterArchHazard {
            title: "High-Velocity Ejection".to_string(),
            description: format!(
                "Exit boost velocity of {:.0} uu/s will violently fling players upon arrival, risking impact damage or falling into out-of-bounds voids.",
                config.exit_boost_velocity
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural CSG portal arch brush with twin pillars, top lintel, and recessed vortex sheet
pub fn generate_teleporter_arch_brush(config: &TeleporterArchConfig) -> Brush {
    let mut brush = Brush::new("Teleporter_Arch".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let w = config.arch_width_uu;
    let h = config.arch_height_uu;
    let p_thick = config.pillar_thickness_uu;
    let depth = p_thick * 1.5; // depth along X

    let half_w = w * 0.5;
    let half_d = depth * 0.5;

    // 1. Left Pillar Box (-Y side)
    let lp_min_y = -half_w;
    let lp_max_y = -half_w + p_thick;
    add_box_to_brush(&mut brush, -half_d, half_d, lp_min_y, lp_max_y, 0.0, h, "Indus1.Indus_B01");

    // 2. Right Pillar Box (+Y side)
    let rp_min_y = half_w - p_thick;
    let rp_max_y = half_w;
    add_box_to_brush(&mut brush, -half_d, half_d, rp_min_y, rp_max_y, 0.0, h, "Indus1.Indus_B01");

    // 3. Top Lintel Arch Header
    let lintel_bot_z = h - p_thick;
    add_box_to_brush(&mut brush, -half_d, half_d, lp_max_y, rp_min_y, lintel_bot_z, h, "Indus1.Indus_B01");

    // 4. Planar Vortex Sheet (Single two-sided quad in center X=0)
    let sheet_v0 = Vec3::new(0.0, lp_max_y, 0.0);
    let sheet_v1 = Vec3::new(0.0, rp_min_y, 0.0);
    let sheet_v2 = Vec3::new(0.0, rp_min_y, lintel_bot_z);
    let sheet_v3 = Vec3::new(0.0, lp_max_y, lintel_bot_z);

    if let Some(mut poly) = Polygon::new(vec![sheet_v0, sheet_v1, sheet_v2, sheet_v3]) {
        poly.texture = "Palettes.Vortex".to_string();
        poly.flags = crate::PF_PORTAL | crate::PF_NOT_SOLID | crate::PF_TWO_SIDED | crate::PF_TRANSLUCENT;
        brush.polys.push(poly);
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
    fn test_teleporter_arch_archetypes_and_defaults() {
        let gothic = TeleporterArchArchetype::GothicMonolithGate.to_config();
        assert_eq!(gothic.arch_width_uu, 128.0);
        assert_eq!(gothic.arch_height_uu, 160.0);
        assert!(gothic.b_changes_velocity);

        let sludge = TeleporterArchArchetype::IndustrialSludgeDrainTeleport.to_config();
        assert_eq!(sludge.arch_width_uu, 96.0);
        assert_eq!(sludge.exit_boost_velocity, 150.0);
    }

    #[test]
    fn test_teleporter_arch_metrics_evaluation() {
        let cfg = TeleporterArchConfig::default(); // 128 x 160, pillars 24
        let peers = vec![
            LevelTeleportContextActorRecord {
                name: "TeleporterExit0".to_string(),
                class_name: "Engine.Teleporter".to_string(),
                tag: "RedBaseExit0".to_string(),
                position: Vec3::new(1000.0, 0.0, 0.0),
            }
        ];

        let metrics = evaluate_teleporter_arch_metrics(&cfg, &peers);
        // net w = 128 - 48 = 80, net h = 160 - 24 = 136
        assert_eq!(metrics.portal_aperture_area_sq_uu, 80.0 * 136.0);
        assert_eq!(metrics.exit_launch_speed, 300.0);
        assert_eq!(metrics.total_portal_gates_in_level, 2);
    }

    #[test]
    fn test_teleporter_arch_safety_hazard_orphan_url() {
        let mut cfg = TeleporterArchConfig::default();
        cfg.url = "NonExistentExitTag".to_string();

        let peers = vec![
            LevelTeleportContextActorRecord {
                name: "TeleporterExit0".to_string(),
                class_name: "Engine.Teleporter".to_string(),
                tag: "OtherTag".to_string(),
                position: Vec3::new(500.0, 0.0, 0.0),
            }
        ];

        let hazards = audit_teleporter_arch_safety(&cfg, &peers);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Unlinked Destination Portal URL")));
    }

    #[test]
    fn test_generate_teleporter_arch_brush() {
        let cfg = TeleporterArchConfig::default();
        let brush = generate_teleporter_arch_brush(&cfg);

        assert_eq!(brush.name, "Teleporter_Arch");
        // 3 boxes (6 polys each = 18) + 1 vortex sheet = 19 polys
        assert_eq!(brush.polys.len(), 19);
    }
}
