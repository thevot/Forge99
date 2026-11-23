// crates/ut99-bsp/src/jump_boots_studio.rs
//
// Interactive Jump Boots Cache, Anti-Gravity Booster & High-Vault Studio
// Ballistic jump physics, parabolic apex calculation, target platform reachability,
// ceiling clearance concussion hazard auditing, and procedural boot cradle brush geometry.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical archetypes of jump boot caches and anti-grav boosters in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JumpBootsArchetype {
    PhobosLowGravityVault,
    FacingWorldsTowerSpireAscent,
    MorpheusRooftopLeap,
    CoretLavaPitTraverse,
    GothicCathedralRaftersStash,
}

impl JumpBootsArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            JumpBootsArchetype::PhobosLowGravityVault => "Phobos Low-Gravity Vault (DM-Phobos Orbital Catwalk)",
            JumpBootsArchetype::FacingWorldsTowerSpireAscent => "Facing Worlds Tower Spire Ascent (CTF-Face Sniper Balcony)",
            JumpBootsArchetype::MorpheusRooftopLeap => "Morpheus Rooftop Leap (DM-Morpheus Skyscraper Pad)",
            JumpBootsArchetype::CoretLavaPitTraverse => "Coret Lava Pit Traverse (CTF-Coret Molten Chasm)",
            JumpBootsArchetype::GothicCathedralRaftersStash => "Gothic Cathedral Rafters Stash (DM-Curse][ Timber Trusses)",
        }
    }

    pub fn to_config(&self) -> JumpBootsConfig {
        match self {
            JumpBootsArchetype::PhobosLowGravityVault => JumpBootsConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                launch_boost_vz: 1100.0,
                forward_velocity_vx: 400.0,
                world_gravity_z: -400.0, // Low gravity
                charge_count: 3,
                cradle_radius_uu: 48.0,
                cradle_height_uu: 16.0,
                target_platform_offset: Vec3::new(400.0, 0.0, 1000.0),
                target_platform_size: Vec3::new(128.0, 128.0, 16.0),
                overhead_ceiling_clearance_uu: 1600.0,
                has_return_route: true,
                tag: "PhobosVault0".to_string(),
                event: "JumpBootsCollected".to_string(),
            },
            JumpBootsArchetype::FacingWorldsTowerSpireAscent => JumpBootsConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                launch_boost_vz: 1250.0,
                forward_velocity_vx: 400.0,
                world_gravity_z: -950.0, // Standard UT99 gravity
                charge_count: 3,
                cradle_radius_uu: 44.0,
                cradle_height_uu: 12.0,
                target_platform_offset: Vec3::new(300.0, 0.0, 650.0),
                target_platform_size: Vec3::new(96.0, 96.0, 16.0),
                overhead_ceiling_clearance_uu: 2000.0, // Open sky
                has_return_route: true,
                tag: "FaceTowerSpire0".to_string(),
                event: "FaceBootsTaken".to_string(),
            },
            JumpBootsArchetype::MorpheusRooftopLeap => JumpBootsConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                launch_boost_vz: 950.0,
                forward_velocity_vx: 450.0,
                world_gravity_z: -950.0,
                charge_count: 3,
                cradle_radius_uu: 52.0,
                cradle_height_uu: 16.0,
                target_platform_offset: Vec3::new(600.0, 0.0, 350.0),
                target_platform_size: Vec3::new(160.0, 160.0, 24.0),
                overhead_ceiling_clearance_uu: 1500.0,
                has_return_route: true,
                tag: "MorpheusRooftop0".to_string(),
                event: "RooftopBootsUsed".to_string(),
            },
            JumpBootsArchetype::CoretLavaPitTraverse => JumpBootsConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                launch_boost_vz: 850.0,
                forward_velocity_vx: 420.0,
                world_gravity_z: -950.0,
                charge_count: 3,
                cradle_radius_uu: 40.0,
                cradle_height_uu: 14.0,
                target_platform_offset: Vec3::new(500.0, 0.0, 280.0),
                target_platform_size: Vec3::new(112.0, 112.0, 16.0),
                overhead_ceiling_clearance_uu: 600.0,
                has_return_route: false, // Hazard if charges run out
                tag: "CoretLavaTraverse0".to_string(),
                event: "LavaVaultInitiated".to_string(),
            },
            JumpBootsArchetype::GothicCathedralRaftersStash => JumpBootsConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                launch_boost_vz: 1000.0,
                forward_velocity_vx: 200.0,
                world_gravity_z: -950.0,
                charge_count: 3,
                cradle_radius_uu: 36.0,
                cradle_height_uu: 10.0,
                target_platform_offset: Vec3::new(100.0, 0.0, 480.0),
                target_platform_size: Vec3::new(64.0, 64.0, 12.0),
                overhead_ceiling_clearance_uu: 550.0,
                has_return_route: true,
                tag: "CathedralRafters0".to_string(),
                event: "RaftersVaulted".to_string(),
            },
        }
    }
}

/// Complete configuration for a Jump Boots cache and vault launchpad
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JumpBootsConfig {
    pub archetype: JumpBootsArchetype,
    pub position: Vec3,
    pub launch_boost_vz: f32,
    pub forward_velocity_vx: f32,
    pub world_gravity_z: f32,
    pub charge_count: u32,
    pub cradle_radius_uu: f32,
    pub cradle_height_uu: f32,
    pub target_platform_offset: Vec3,
    pub target_platform_size: Vec3,
    pub overhead_ceiling_clearance_uu: f32,
    pub has_return_route: bool,
    pub tag: String,
    pub event: String,
}

impl Default for JumpBootsConfig {
    fn default() -> Self {
        JumpBootsArchetype::PhobosLowGravityVault.to_config()
    }
}

/// Evaluated ballistic trajectory and reachability metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JumpBootsMetrics {
    pub peak_apex_height_uu: f32,
    pub time_to_apex_s: f32,
    pub total_hang_time_s: f32,
    pub maximum_vault_distance_uu: f32,
    pub is_target_reachable: bool,
    pub landing_dispersion_radius_uu: f32,
    pub total_cache_potential_altitude_uu: f32,
    pub total_boot_caches_in_level: usize,
}

/// Context actor record for safety checking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelJumpBootsContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Compute ballistic flight trajectory metrics for the Jump Boots configuration
pub fn evaluate_jump_boots_metrics(
    config: &JumpBootsConfig,
    peers: &[LevelJumpBootsContextActorRecord],
) -> JumpBootsMetrics {
    let g = config.world_gravity_z.abs().max(50.0);
    let vz = config.launch_boost_vz;
    let vx = config.forward_velocity_vx;

    let time_to_apex = vz / g;
    let total_hang_time = time_to_apex * 2.0;
    let peak_apex_height = (vz * vz) / (2.0 * g);
    let max_vault_dist = vx * total_hang_time;

    // Check if target platform is reachable
    let target_z = config.target_platform_offset.z;
    let target_horiz_dist = Vec3::new(
        config.target_platform_offset.x,
        config.target_platform_offset.y,
        0.0,
    )
    .length();

    let reachable = if target_z > peak_apex_height {
        false
    } else {
        // Time to reach target altitude: vz*t - 0.5*g*t^2 = target_z
        // 0.5*g*t^2 - vz*t + target_z = 0
        let discr = vz * vz - 2.0 * g * target_z;
        if discr < 0.0 {
            false
        } else {
            let t_descent = (vz + discr.sqrt()) / g;
            let dist_at_elevation = vx * t_descent;
            (target_horiz_dist - dist_at_elevation).abs()
                <= (config.target_platform_size.x * 0.5 + 48.0)
        }
    };

    let dispersion = 32.0 + (total_hang_time * 16.0);
    let potential_altitude = peak_apex_height * (config.charge_count as f32);

    let mut count = 1;
    for p in peers {
        let cls = p.class_name.to_lowercase();
        if cls.contains("jumpboots") || cls.contains("boots") {
            count += 1;
        }
    }

    JumpBootsMetrics {
        peak_apex_height_uu: peak_apex_height,
        time_to_apex_s: time_to_apex,
        total_hang_time_s: total_hang_time,
        maximum_vault_distance_uu: max_vault_dist,
        is_target_reachable: reachable,
        landing_dispersion_radius_uu: dispersion,
        total_cache_potential_altitude_uu: potential_altitude,
        total_boot_caches_in_level: count,
    }
}

/// Potential safety hazards and gameplay balancing alerts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpBootsHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Audit safety and gameplay hazards for the Jump Boots cache setup
pub fn audit_jump_boots_safety(
    config: &JumpBootsConfig,
    peers: &[LevelJumpBootsContextActorRecord],
) -> Vec<JumpBootsHazard> {
    let mut hazards = Vec::new();
    let metrics = evaluate_jump_boots_metrics(config, peers);

    // 1. Ceiling concussion hazard: ceiling clearance less than jump apex or < 280 uu
    if config.overhead_ceiling_clearance_uu < metrics.peak_apex_height_uu
        || config.overhead_ceiling_clearance_uu < 280.0
    {
        hazards.push(JumpBootsHazard {
            title: "Ceiling Concussion Hazard".to_string(),
            description: format!(
                "Overhead ceiling clearance of {:.0} uu is lower than peak apex height ({:.0} uu) or minimum 280 uu, causing players to bump their heads.",
                config.overhead_ceiling_clearance_uu, metrics.peak_apex_height_uu
            ),
            is_critical: true,
        });
    }

    // 2. Target reachability / overshoot
    if !metrics.is_target_reachable {
        hazards.push(JumpBootsHazard {
            title: "Hazardous Platform Overshoot or Unreachable Target".to_string(),
            description: "Target platform offset is beyond the parabolic ballistic flight envelope of the Jump Boots.".to_string(),
            is_critical: true,
        });
    }

    // 3. Stranding when charges deplete
    if config.target_platform_offset.z > 200.0 && !config.has_return_route {
        hazards.push(JumpBootsHazard {
            title: "Depleted Charge Stranding Hazard".to_string(),
            description: "High destination platform lacks return route (stairs/lift) trapping players once boots charges are spent.".to_string(),
            is_critical: true,
        });
    }

    // 4. Insufficient landing clearance
    if config.target_platform_size.x < 48.0 || config.target_platform_size.y < 48.0 {
        hazards.push(JumpBootsHazard {
            title: "Insufficient Landing Clearance".to_string(),
            description: format!(
                "Target landing platform size ({:.0}x{:.0} uu) is smaller than 48 uu, creating a high risk of missed vaults.",
                config.target_platform_size.x, config.target_platform_size.y
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a solid watertight CSG brush for the anti-grav charging cradle
pub fn generate_jump_boots_brush(config: &JumpBootsConfig) -> Brush {
    let mut brush = Brush::new("Jump_Boots_Cradle", CsgOperation::Add);
    let r = config.cradle_radius_uu;
    let h = config.cradle_height_uu;
    let p = config.position;

    let tex_cradle = "Indus1.Metal.IronBase";
    let tex_disc = "Indus1.Energy.BlueGlow";

    // 1. Octagonal Pedestal Base
    let sides = 8;
    let mut base_pts = Vec::new();
    let mut top_pts = Vec::new();

    for i in 0..sides {
        let angle = (i as f32) * (std::f32::consts::TAU / sides as f32);
        let offset = Vec3::new(angle.cos() * r, angle.sin() * r, 0.0);
        base_pts.push(p + offset);
        top_pts.push(p + offset + Vec3::new(0.0, 0.0, h));
    }

    // Bottom face (-Z)
    if let Some(mut poly) = Polygon::new(base_pts.clone()) {
        poly.texture = tex_cradle.to_string();
        brush.polys.push(poly);
    }

    // 8 Side faces
    for i in 0..sides {
        let next = (i + 1) % sides;
        if let Some(mut poly) = Polygon::new(vec![
            base_pts[i],
            base_pts[next],
            top_pts[next],
            top_pts[i],
        ]) {
            poly.texture = tex_cradle.to_string();
            brush.polys.push(poly);
        }
    }

    // Top face (+Z)
    let mut top_reversed = top_pts.clone();
    top_reversed.reverse();
    if let Some(mut poly) = Polygon::new(top_reversed) {
        poly.texture = tex_cradle.to_string();
        brush.polys.push(poly);
    }

    // 2. Central recessed induction disc polygon with translucent glow polyflags
    let inner_r = r * 0.65;
    let mut inner_pts = Vec::new();
    for i in 0..sides {
        let angle = (i as f32) * (std::f32::consts::TAU / sides as f32);
        let offset = Vec3::new(angle.cos() * inner_r, angle.sin() * inner_r, h + 1.0);
        inner_pts.push(p + offset);
    }
    inner_pts.reverse();
    if let Some(mut disc_poly) = Polygon::new(inner_pts) {
        disc_poly.texture = tex_disc.to_string();
        disc_poly.flags = crate::PF_TRANSLUCENT | crate::PF_UNLIT;
        brush.polys.push(disc_poly);
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jump_boots_archetypes_and_defaults() {
        let archetypes = [
            JumpBootsArchetype::PhobosLowGravityVault,
            JumpBootsArchetype::FacingWorldsTowerSpireAscent,
            JumpBootsArchetype::MorpheusRooftopLeap,
            JumpBootsArchetype::CoretLavaPitTraverse,
            JumpBootsArchetype::GothicCathedralRaftersStash,
        ];

        for a in &archetypes {
            let config = a.to_config();
            assert_eq!(config.archetype, *a);
            assert!(config.launch_boost_vz > 0.0);
            assert!(config.cradle_radius_uu > 0.0);
            assert!(config.charge_count > 0);
            let metrics = evaluate_jump_boots_metrics(&config, &[]);
            assert!(metrics.peak_apex_height_uu > 0.0);
            assert!(metrics.time_to_apex_s > 0.0);
            assert!(metrics.maximum_vault_distance_uu > 0.0);
            assert_eq!(metrics.total_boot_caches_in_level, 1);
        }
    }

    #[test]
    fn test_jump_boots_ballistics_calculation() {
        let config = JumpBootsArchetype::FacingWorldsTowerSpireAscent.to_config();
        let metrics = evaluate_jump_boots_metrics(&config, &[]);

        // H = vz^2 / (2g) = 1250^2 / (2 * 950) = 1562500 / 1900 = 822.36 uu
        assert!((metrics.peak_apex_height_uu - 822.36).abs() < 1.0);
        // t_apex = vz / g = 1250 / 950 = 1.315s
        assert!((metrics.time_to_apex_s - 1.315).abs() < 0.01);
        assert!(metrics.maximum_vault_distance_uu > 1000.0);
    }

    #[test]
    fn test_jump_boots_safety_hazard_concussion_and_stranding() {
        let mut config = JumpBootsArchetype::GothicCathedralRaftersStash.to_config();
        config.overhead_ceiling_clearance_uu = 200.0; // Under 280 and under apex!
        config.has_return_route = false; // High stash without ladder
        config.target_platform_size = Vec3::new(32.0, 32.0, 16.0); // Too narrow!

        let hazards = audit_jump_boots_safety(&config, &[]);
        assert!(hazards.iter().any(|h| h.title.contains("Ceiling Concussion")));
        assert!(hazards.iter().any(|h| h.title.contains("Stranding")));
        assert!(hazards.iter().any(|h| h.title.contains("Insufficient Landing Clearance")));
    }

    #[test]
    fn test_generate_jump_boots_brush() {
        let config = JumpBootsArchetype::PhobosLowGravityVault.to_config();
        let brush = generate_jump_boots_brush(&config);

        // 1 bottom + 8 sides + 1 top + 1 glow disc = 11 faces
        assert_eq!(brush.polys.len(), 11);
        assert_eq!(brush.csg_oper, CsgOperation::Add);
        assert_eq!(brush.name, "Jump_Boots_Cradle");
        // Disc has translucent & unlit flags
        assert!(brush.polys.last().unwrap().flags & crate::PF_TRANSLUCENT != 0);
        assert!(brush.polys.last().unwrap().flags & crate::PF_UNLIT != 0);
    }
}
