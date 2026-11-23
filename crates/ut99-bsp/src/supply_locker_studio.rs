// crates/ut99-bsp/src/supply_locker_studio.rs
//
// Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio
// Recessed armory locker geometry, weapon & ammo multi-pickup synchronization,
// replenishment throughput rates, supply starvation diagnostics, and procedural cabinet brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical archetypes of tournament supply lockers and armory cabinets in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupplyLockerArchetype {
    LiandriStandardHeavyArmory,
    ShockCoreEnergyDepot,
    BioHazardToxicCache,
    SniperOverwatchResupply,
    EmergencyFieldHospitalLocker,
}

impl SupplyLockerArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            SupplyLockerArchetype::LiandriStandardHeavyArmory => "Liandri Heavy Armory (Rocket Launcher & Flak Cannon)",
            SupplyLockerArchetype::ShockCoreEnergyDepot => "Shock Core Energy Depot (Shock Rifle & Plasma Amp)",
            SupplyLockerArchetype::BioHazardToxicCache => "Bio-Hazard Slime Cache (GES BioRifle & Slime Vats)",
            SupplyLockerArchetype::SniperOverwatchResupply => "Sniper Overwatch Resupply (Sniper Rifle & Rounds)",
            SupplyLockerArchetype::EmergencyFieldHospitalLocker => "Emergency Field Hospital Locker (MedBoxes & Vials)",
        }
    }

    pub fn to_config(&self) -> SupplyLockerConfig {
        match self {
            SupplyLockerArchetype::LiandriStandardHeavyArmory => SupplyLockerConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                locker_width_uu: 96.0,
                locker_height_uu: 128.0,
                locker_depth_uu: 32.0,
                shelf_count: 3,
                primary_weapon_class: "Botpack.UT_Eightball".to_string(),
                primary_ammo_class: "Botpack.RocketPack".to_string(),
                ammo_pack_count: 2,
                respawn_interval_secs: 27.0,
                tag: "HeavyArmoryLocker0".to_string(),
            },
            SupplyLockerArchetype::ShockCoreEnergyDepot => SupplyLockerConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                locker_width_uu: 80.0,
                locker_height_uu: 112.0,
                locker_depth_uu: 28.0,
                shelf_count: 2,
                primary_weapon_class: "Botpack.ShockRifle".to_string(),
                primary_ammo_class: "Botpack.ShockCore".to_string(),
                ammo_pack_count: 3,
                respawn_interval_secs: 25.0,
                tag: "ShockDepotLocker0".to_string(),
            },
            SupplyLockerArchetype::BioHazardToxicCache => SupplyLockerConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                locker_width_uu: 72.0,
                locker_height_uu: 96.0,
                locker_depth_uu: 24.0,
                shelf_count: 2,
                primary_weapon_class: "Botpack.UT_BioRifle".to_string(),
                primary_ammo_class: "Botpack.BioAmmo".to_string(),
                ammo_pack_count: 4,
                respawn_interval_secs: 20.0,
                tag: "BioSlimeCache0".to_string(),
            },
            SupplyLockerArchetype::SniperOverwatchResupply => SupplyLockerConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                locker_width_uu: 88.0,
                locker_height_uu: 120.0,
                locker_depth_uu: 30.0,
                shelf_count: 3,
                primary_weapon_class: "Botpack.SniperRifle".to_string(),
                primary_ammo_class: "Botpack.BulletBox".to_string(),
                ammo_pack_count: 2,
                respawn_interval_secs: 30.0,
                tag: "SniperLocker0".to_string(),
            },
            SupplyLockerArchetype::EmergencyFieldHospitalLocker => SupplyLockerConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                facing_direction: Vec3::new(1.0, 0.0, 0.0),
                locker_width_uu: 64.0,
                locker_height_uu: 96.0,
                locker_depth_uu: 24.0,
                shelf_count: 2,
                primary_weapon_class: "".to_string(),
                primary_ammo_class: "Botpack.MedBox".to_string(),
                ammo_pack_count: 3,
                respawn_interval_secs: 20.0,
                tag: "FieldHospitalLocker0".to_string(),
            },
        }
    }
}

/// Configuration settings for a supply locker and armory cabinet
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupplyLockerConfig {
    pub archetype: SupplyLockerArchetype,
    pub position: Vec3,
    pub facing_direction: Vec3,
    pub locker_width_uu: f32,
    pub locker_height_uu: f32,
    pub locker_depth_uu: f32,
    pub shelf_count: usize,
    pub primary_weapon_class: String,
    pub primary_ammo_class: String,
    pub ammo_pack_count: usize,
    pub respawn_interval_secs: f32,
    pub tag: String,
}

impl Default for SupplyLockerConfig {
    fn default() -> Self {
        SupplyLockerArchetype::LiandriStandardHeavyArmory.to_config()
    }
}

/// Context actor record for evaluating armory locker neighborhood
#[derive(Debug, Clone, PartialEq)]
pub struct LevelLockerContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Telemetry metrics for a supply locker
#[derive(Debug, Clone, PartialEq)]
pub struct SupplyLockerMetrics {
    pub total_contained_firepower_rating: f32,
    pub shelf_usable_area_sq_uu: f32,
    pub replenishment_throughput_per_min: f32,
    pub total_supply_lockers_in_level: usize,
}

/// Safety diagnostics for supply locker placement and contents
#[derive(Debug, Clone, PartialEq)]
pub struct SupplyLockerHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates firepower index, shelf surface area, and restock throughput
pub fn evaluate_supply_locker_metrics(
    config: &SupplyLockerConfig,
    peers: &[LevelLockerContextActorRecord],
) -> SupplyLockerMetrics {
    let shelf_area = config.locker_width_uu * config.locker_depth_uu * (config.shelf_count as f32);

    // Firepower score based on weapon and ammo quantity
    let weapon_score = if !config.primary_weapon_class.is_empty() { 50.0 } else { 0.0 };
    let ammo_score = (config.ammo_pack_count as f32) * 15.0;
    let total_firepower = weapon_score + ammo_score;

    // Replenishment throughput per minute = 60s / respawn_interval * items
    let items_count = (if !config.primary_weapon_class.is_empty() { 1 } else { 0 }) + config.ammo_pack_count;
    let throughput = if config.respawn_interval_secs > 0.0 {
        (60.0 / config.respawn_interval_secs) * (items_count as f32)
    } else {
        0.0
    };

    let mut lockers_count = 1;
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("weapon")
            || cls_lower.contains("pickup")
            || cls_lower.contains("ammo")
            || cls_lower.contains("rifle")
            || cls_lower.contains("cannon")
            || cls_lower.contains("eightball")
            || cls_lower.contains("locker")
        {
            lockers_count += 1;
        }
    }

    SupplyLockerMetrics {
        total_contained_firepower_rating: total_firepower,
        shelf_usable_area_sq_uu: shelf_area,
        replenishment_throughput_per_min: throughput,
        total_supply_lockers_in_level: lockers_count,
    }
}

/// Audits safety for depleted empty lockers, clustering with adjacent weapons, and narrow depth
pub fn audit_supply_locker_safety(
    config: &SupplyLockerConfig,
    peers: &[LevelLockerContextActorRecord],
) -> Vec<SupplyLockerHazard> {
    let mut hazards = Vec::new();

    // 1. Depleted Empty Locker (No weapon and 0 ammo packs)
    if config.primary_weapon_class.trim().is_empty() && config.ammo_pack_count == 0 {
        hazards.push(SupplyLockerHazard {
            title: "Depleted Empty Supply Locker".to_string(),
            description: "Locker has no primary weapon and ammo count is 0. Players will find nothing inside.".to_string(),
            is_critical: true,
        });
    }

    // 2. Dangerous Weapon Clustering (< 120 uu from another weapon pickup)
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("weapon") || cls_lower.contains("eightball") || cls_lower.contains("shock") {
            let dist = (p.position - config.position).length();
            if dist < 120.0 {
                hazards.push(SupplyLockerHazard {
                    title: "Excessive Weapon Clustering".to_string(),
                    description: format!(
                        "Locker is situated only {:.0} uu from weapon actor '{}'. Excessive weapon clustering ruins tournament weapon distribution balance.",
                        dist, p.name
                    ),
                    is_critical: false,
                });
            }
        }
    }

    // 3. Shallow Locker Depth (< 16 uu clipping hazard)
    if config.locker_depth_uu < 16.0 {
        hazards.push(SupplyLockerHazard {
            title: "Shallow Locker Depth Collision Clipping".to_string(),
            description: format!(
                "Locker depth of {:.0} uu is too shallow for standard weapon pickup collision (min 20-30 uu diameter). Items will visibly clip through the back wall.",
                config.locker_depth_uu
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural CSG recessed cabinet housing brush with rear plate and side walls
pub fn generate_supply_locker_brush(config: &SupplyLockerConfig) -> Brush {
    let mut brush = Brush::new("Supply_Locker".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let w = config.locker_width_uu;
    let h = config.locker_height_uu;
    let d = config.locker_depth_uu;
    let wall_t = 8.0;

    let half_w = w * 0.5;

    // 1. Rear Wall Plate (-X from -d to -d+wall_t)
    add_box_to_brush(&mut brush, -d, -d + wall_t, -half_w, half_w, 0.0, h, "Indus1.Indus_B01");

    // 2. Left Side Wall (-Y)
    add_box_to_brush(&mut brush, -d + wall_t, 0.0, -half_w, -half_w + wall_t, 0.0, h, "Indus1.Indus_B01");

    // 3. Right Side Wall (+Y)
    add_box_to_brush(&mut brush, -d + wall_t, 0.0, half_w - wall_t, half_w, 0.0, h, "Indus1.Indus_B01");

    // 4. Bottom Base Floor Slab
    add_box_to_brush(&mut brush, -d + wall_t, 0.0, -half_w + wall_t, half_w - wall_t, 0.0, wall_t, "Indus1.Indus_B01");

    // 5. Top Roof Slab
    add_box_to_brush(&mut brush, -d + wall_t, 0.0, -half_w + wall_t, half_w - wall_t, h - wall_t, h, "Indus1.Indus_B01");

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
    fn test_supply_locker_archetypes_and_defaults() {
        let heavy = SupplyLockerArchetype::LiandriStandardHeavyArmory.to_config();
        assert_eq!(heavy.locker_width_uu, 96.0);
        assert_eq!(heavy.primary_weapon_class, "Botpack.UT_Eightball");
        assert_eq!(heavy.ammo_pack_count, 2);

        let hospital = SupplyLockerArchetype::EmergencyFieldHospitalLocker.to_config();
        assert!(hospital.primary_weapon_class.is_empty());
        assert_eq!(hospital.primary_ammo_class, "Botpack.MedBox");
    }

    #[test]
    fn test_supply_locker_metrics_evaluation() {
        let cfg = SupplyLockerConfig::default(); // 96 x 32, 3 shelves, 1 weapon + 2 ammo, 27s
        let peers = vec![
            LevelLockerContextActorRecord {
                name: "ShockPickup0".to_string(),
                class_name: "Botpack.ShockRifle".to_string(),
                position: Vec3::new(500.0, 0.0, 0.0),
            }
        ];

        let metrics = evaluate_supply_locker_metrics(&cfg, &peers);
        assert_eq!(metrics.shelf_usable_area_sq_uu, 96.0 * 32.0 * 3.0);
        assert_eq!(metrics.total_contained_firepower_rating, 50.0 + 2.0 * 15.0); // 80.0
        assert_eq!(metrics.total_supply_lockers_in_level, 2);
    }

    #[test]
    fn test_supply_locker_safety_hazard_depleted() {
        let mut cfg = SupplyLockerConfig::default();
        cfg.primary_weapon_class = "".to_string();
        cfg.ammo_pack_count = 0;

        let hazards = audit_supply_locker_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Depleted Empty Supply Locker")));
    }

    #[test]
    fn test_generate_supply_locker_brush() {
        let cfg = SupplyLockerConfig::default();
        let brush = generate_supply_locker_brush(&cfg);

        assert_eq!(brush.name, "Supply_Locker");
        // 5 box slabs (6 polys each = 30)
        assert_eq!(brush.polys.len(), 30);
    }
}
