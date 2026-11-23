// crates/ut99-bsp/src/weapon_circuit.rs
//
// Interactive Tournament Weapon Placement & Ammo Circuit Studio
// Weapon DPS, burst damage and range modeling, weapon circuit sprint time matrix,
// ammo box pairing proximity scoring, weapon spawn starvation auditing,
// and procedural weapon altar/pedestal generation.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// UT99 Canonical Tournament Weapon Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentWeaponType {
    ShockRifle,
    FlakCannon,
    RocketLauncher,
    SniperRifle,
    Minigun,
    BioRifle,
    PulseGun,
    Ripper,
    Enforcer,
    DualEnforcers,
    Redeemer,
}

impl TournamentWeaponType {
    pub fn display_name(&self) -> &'static str {
        match self {
            TournamentWeaponType::ShockRifle => "ASMD Shock Rifle (Hitscan / Combo)",
            TournamentWeaponType::FlakCannon => "Flak Cannon (Shrapnel / Slug)",
            TournamentWeaponType::RocketLauncher => "UT_Eightball (1-6 Rockets)",
            TournamentWeaponType::SniperRifle => "Sniper Rifle (Headshot 150)",
            TournamentWeaponType::Minigun => "Minigun (Rapid Suppression)",
            TournamentWeaponType::BioRifle => "GES BioRifle (Slime Glob 150)",
            TournamentWeaponType::PulseGun => "Pulse Gun (Plasma / Beam)",
            TournamentWeaponType::Ripper => "Ripper (Razor Blade Decapitation)",
            TournamentWeaponType::Enforcer => "Enforcer (Single Pistol)",
            TournamentWeaponType::DualEnforcers => "Dual Akimbo Enforcers",
            TournamentWeaponType::Redeemer => "Warhead Launcher (Redeemer 1000)",
        }
    }

    pub fn actor_class_name(&self) -> &'static str {
        match self {
            TournamentWeaponType::ShockRifle => "Botpack.ShockRifle",
            TournamentWeaponType::FlakCannon => "Botpack.UT_FlakCannon",
            TournamentWeaponType::RocketLauncher => "Botpack.UT_Eightball",
            TournamentWeaponType::SniperRifle => "Botpack.SniperRifle",
            TournamentWeaponType::Minigun => "Botpack.minigun2",
            TournamentWeaponType::BioRifle => "Botpack.UT_BioRifle",
            TournamentWeaponType::PulseGun => "Botpack.PulseGun",
            TournamentWeaponType::Ripper => "Botpack.Ripper",
            TournamentWeaponType::Enforcer => "Botpack.Enforcer",
            TournamentWeaponType::DualEnforcers => "Botpack.Enforcer",
            TournamentWeaponType::Redeemer => "Botpack.WarheadLauncher",
        }
    }

    pub fn ammo_class_name(&self) -> &'static str {
        match self {
            TournamentWeaponType::ShockRifle => "Botpack.ShockCore",
            TournamentWeaponType::FlakCannon => "Botpack.FlakShellAmmo",
            TournamentWeaponType::RocketLauncher => "Botpack.RocketPack",
            TournamentWeaponType::SniperRifle => "Botpack.BulletBox",
            TournamentWeaponType::Minigun => "Botpack.Miniammo",
            TournamentWeaponType::BioRifle => "Botpack.BioAmmo",
            TournamentWeaponType::PulseGun => "Botpack.PAmmo",
            TournamentWeaponType::Ripper => "Botpack.BladeHopper",
            TournamentWeaponType::Enforcer | TournamentWeaponType::DualEnforcers => "Botpack.EClip",
            TournamentWeaponType::Redeemer => "Botpack.WarheadAmmo",
        }
    }

    pub fn max_burst_damage(&self) -> f32 {
        match self {
            TournamentWeaponType::ShockRifle => 200.0,  // Shock Combo
            TournamentWeaponType::FlakCannon => 160.0,  // Full shrapnel hit
            TournamentWeaponType::RocketLauncher => 600.0, // 6 loaded rockets
            TournamentWeaponType::SniperRifle => 150.0, // Headshot
            TournamentWeaponType::Minigun => 140.0,     // 1 second focused burst
            TournamentWeaponType::BioRifle => 150.0,    // Full secondary glob
            TournamentWeaponType::PulseGun => 110.0,    // Continuous beam 1s
            TournamentWeaponType::Ripper => 100.0,      // Head decapitation
            TournamentWeaponType::Enforcer => 34.0,     // 2 quick shots
            TournamentWeaponType::DualEnforcers => 68.0,// 4 akimbo shots
            TournamentWeaponType::Redeemer => 1000.0,   // Nuclear blast
        }
    }

    pub fn is_hitscan(&self) -> bool {
        matches!(
            self,
            TournamentWeaponType::ShockRifle
                | TournamentWeaponType::SniperRifle
                | TournamentWeaponType::Minigun
                | TournamentWeaponType::Enforcer
                | TournamentWeaponType::DualEnforcers
        )
    }

    pub fn weapon_color_rgb(&self) -> [f32; 4] {
        match self {
            TournamentWeaponType::ShockRifle => [0.7, 0.3, 1.0, 0.95],     // Violet
            TournamentWeaponType::FlakCannon => [1.0, 0.6, 0.1, 0.95],     // Orange
            TournamentWeaponType::RocketLauncher => [1.0, 0.2, 0.2, 0.95], // Red
            TournamentWeaponType::SniperRifle => [0.2, 0.5, 1.0, 0.95],    // Blue
            TournamentWeaponType::Minigun => [0.3, 0.9, 0.9, 0.95],        // Cyan
            TournamentWeaponType::BioRifle => [0.2, 1.0, 0.3, 0.95],       // Slime Green
            TournamentWeaponType::PulseGun => [0.4, 1.0, 0.7, 0.95],       // Teal
            TournamentWeaponType::Ripper => [0.9, 0.9, 0.2, 0.95],         // Yellow
            TournamentWeaponType::Enforcer | TournamentWeaponType::DualEnforcers => [0.7, 0.7, 0.8, 0.95], // Steel
            TournamentWeaponType::Redeemer => [1.0, 1.0, 1.0, 1.0],        // Pure White
        }
    }
}

/// Archetypes of canonical competitive weapon circuits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponCircuitArchetype {
    Deck16DuelMasterCircuit,
    CampgroundsHeavyTDM,
    FaceLongRangeAssault,
    GothicCloseQuartersCarnage,
    MinimalistDuel1v1,
}

impl WeaponCircuitArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            WeaponCircuitArchetype::Deck16DuelMasterCircuit => "Deck16][ Master Duel Circuit (Shock, Flak, Rocket, Sniper)",
            WeaponCircuitArchetype::CampgroundsHeavyTDM => "Campgrounds Style Heavy Circuit (Multi-Flak & Rocket)",
            WeaponCircuitArchetype::FaceLongRangeAssault => "CTF-Face Symmetric Long-Range Assault (Sniper / Shock)",
            WeaponCircuitArchetype::GothicCloseQuartersCarnage => "Gothic CQC Brawl (Flak, Bio, Ripper, Chainsaw)",
            WeaponCircuitArchetype::MinimalistDuel1v1 => "1v1 Minimalist Contention (Shock & Rocket Only)",
        }
    }

    pub fn to_config(&self) -> WeaponCircuitConfig {
        match self {
            WeaponCircuitArchetype::Deck16DuelMasterCircuit => WeaponCircuitConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                primary_weapon: TournamentWeaponType::ShockRifle,
                flanking_ammo_count: 2,
                altar_width: 80.0,
                altar_depth: 48.0,
                altar_height: 16.0,
                respawn_delay_secs: 27.5,
                b_weapon_stay: false,
                tag: "Deck16ShockStation".to_string(),
            },
            WeaponCircuitArchetype::CampgroundsHeavyTDM => WeaponCircuitConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                primary_weapon: TournamentWeaponType::RocketLauncher,
                flanking_ammo_count: 3,
                altar_width: 88.0,
                altar_depth: 54.0,
                altar_height: 20.0,
                respawn_delay_secs: 30.0,
                b_weapon_stay: true,
                tag: "HeavyRocketStation".to_string(),
            },
            WeaponCircuitArchetype::FaceLongRangeAssault => WeaponCircuitConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                primary_weapon: TournamentWeaponType::SniperRifle,
                flanking_ammo_count: 2,
                altar_width: 72.0,
                altar_depth: 40.0,
                altar_height: 14.0,
                respawn_delay_secs: 20.0,
                b_weapon_stay: true,
                tag: "FaceSniperStation".to_string(),
            },
            WeaponCircuitArchetype::GothicCloseQuartersCarnage => WeaponCircuitConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                primary_weapon: TournamentWeaponType::FlakCannon,
                flanking_ammo_count: 2,
                altar_width: 84.0,
                altar_depth: 50.0,
                altar_height: 18.0,
                respawn_delay_secs: 25.0,
                b_weapon_stay: false,
                tag: "GothicFlakStation".to_string(),
            },
            WeaponCircuitArchetype::MinimalistDuel1v1 => WeaponCircuitConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                primary_weapon: TournamentWeaponType::ShockRifle,
                flanking_ammo_count: 1,
                altar_width: 64.0,
                altar_depth: 36.0,
                altar_height: 12.0,
                respawn_delay_secs: 27.5,
                b_weapon_stay: false,
                tag: "Duel1v1ShockStation".to_string(),
            },
        }
    }
}

/// Configuration settings for Weapon Circuit & Station Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponCircuitConfig {
    pub archetype: WeaponCircuitArchetype,
    pub position: Vec3,
    pub primary_weapon: TournamentWeaponType,
    pub flanking_ammo_count: u32,
    pub altar_width: f32,
    pub altar_depth: f32,
    pub altar_height: f32,
    pub respawn_delay_secs: f32,
    pub b_weapon_stay: bool,
    pub tag: String,
}

impl Default for WeaponCircuitConfig {
    fn default() -> Self {
        WeaponCircuitArchetype::Deck16DuelMasterCircuit.to_config()
    }
}

/// Marker representation of an existing weapon or ammo actor in the level
#[derive(Debug, Clone, PartialEq)]
pub struct LevelWeaponMarker {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
    pub is_weapon: bool,
}

/// Sprint circuit link connecting two weapons
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponCircuitLink {
    pub source_name: String,
    pub target_name: String,
    pub distance_uu: f32,
    pub sprint_time_secs: f32,
}

/// Analytical metrics for weapon economy and sprint circuit
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponCircuitMetrics {
    pub total_weapons_in_level: usize,
    pub total_ammo_boxes_in_level: usize,
    pub total_max_burst_damage_potential: f32,
    pub hitscan_vs_projectile_ratio: f32,
    pub avg_sprint_time_between_weapons_secs: f32,
    pub ammo_pairing_coverage_pct: f32,
    pub circuit_links: Vec<WeaponCircuitLink>,
}

/// Safety and balance hazards in weapon placement
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponCircuitHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates circuit telemetry between all weapons in the level
pub fn evaluate_weapon_circuit_metrics(
    config: &WeaponCircuitConfig,
    markers: &[LevelWeaponMarker],
) -> WeaponCircuitMetrics {
    let mut weapons = Vec::new();
    let mut ammo_boxes = Vec::new();

    for m in markers {
        if m.is_weapon {
            weapons.push(m.clone());
        } else {
            ammo_boxes.push(m.clone());
        }
    }

    // Always include the candidate weapon from config
    weapons.push(LevelWeaponMarker {
        name: format!("{}_Candidate", config.tag),
        class_name: config.primary_weapon.actor_class_name().to_string(),
        position: config.position,
        is_weapon: true,
    });

    let mut hitscan_count = 0.0_f32;
    let mut proj_count = 0.0_f32;
    let mut total_burst = 0.0_f32;

    for w in &weapons {
        let is_hs = w.class_name.contains("Shock")
            || w.class_name.contains("Sniper")
            || w.class_name.contains("minigun")
            || w.class_name.contains("Enforcer");
        if is_hs {
            hitscan_count += 1.0;
        } else {
            proj_count += 1.0;
        }
        total_burst += config.primary_weapon.max_burst_damage();
    }

    let ratio = if proj_count > 0.0 {
        hitscan_count / proj_count
    } else {
        hitscan_count
    };

    // Calculate sprint circuit links between weapons (speed = 400 uu/s)
    let mut links = Vec::new();
    let mut total_time = 0.0_f32;
    let sprint_speed = 400.0_f32;

    for i in 0..weapons.len() {
        for j in (i + 1)..weapons.len() {
            let dist = weapons[i].position.distance(weapons[j].position);
            let time = dist / sprint_speed;
            links.push(WeaponCircuitLink {
                source_name: weapons[i].name.clone(),
                target_name: weapons[j].name.clone(),
                distance_uu: dist,
                sprint_time_secs: time,
            });
            total_time += time;
        }
    }

    let avg_sprint = if !links.is_empty() {
        total_time / links.len() as f32
    } else {
        0.0
    };

    // Ammo pairing coverage: % of weapons with ammo within 600 uu
    let mut paired_count = 0;
    for w in &weapons {
        let has_near_ammo = ammo_boxes
            .iter()
            .any(|a| a.position.distance(w.position) <= 600.0)
            || config.flanking_ammo_count > 0;
        if has_near_ammo {
            paired_count += 1;
        }
    }

    let coverage_pct = if !weapons.is_empty() {
        (paired_count as f32 / weapons.len() as f32) * 100.0
    } else {
        100.0
    };

    WeaponCircuitMetrics {
        total_weapons_in_level: weapons.len(),
        total_ammo_boxes_in_level: ammo_boxes.len() + config.flanking_ammo_count as usize,
        total_max_burst_damage_potential: total_burst,
        hitscan_vs_projectile_ratio: ratio,
        avg_sprint_time_between_weapons_secs: avg_sprint,
        ammo_pairing_coverage_pct: coverage_pct,
        circuit_links: links,
    }
}

/// Audits weapon starvation, clustering, and pairing hazards
pub fn audit_weapon_circuit_safety(
    config: &WeaponCircuitConfig,
    markers: &[LevelWeaponMarker],
) -> Vec<WeaponCircuitHazard> {
    let mut hazards = Vec::new();

    // 1. Clustered weapons (< 300 uu)
    for m in markers {
        if m.is_weapon && m.position.distance(config.position) < 300.0 {
            hazards.push(WeaponCircuitHazard {
                title: "Excessive Weapon Clustering".to_string(),
                description: format!(
                    "Weapon station is only {:.1} uu from existing weapon '{}'. Space weapons out by at least 600 uu.",
                    m.position.distance(config.position), m.name
                ),
                is_critical: true,
            });
            break;
        }
    }

    // 2. Orphaned weapon (0 ammo boxes and flanking_ammo_count == 0)
    let has_any_ammo = markers.iter().any(|m| !m.is_weapon) || config.flanking_ammo_count > 0;
    if !has_any_ammo {
        hazards.push(WeaponCircuitHazard {
            title: "Orphaned Weapon (Ammo Starvation)".to_string(),
            description: "No ammunition boxes exist for this weapon. Players will rapidly run dry.".to_string(),
            is_critical: false,
        });
    }

    // 3. Redeemer in small match
    if config.primary_weapon == TournamentWeaponType::Redeemer {
        hazards.push(WeaponCircuitHazard {
            title: "Nuclear Devastation Risk (Redeemer)".to_string(),
            description: "Redeemer produces 1000 AoE lethal damage. Ensure long line of sight or high risk to acquire.".to_string(),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a beveled rectangular weapon spawn altar brush
pub fn generate_weapon_altar_brush(config: &WeaponCircuitConfig) -> Brush {
    let mut brush = Brush::new("WeaponAltar".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let hw = config.altar_width * 0.5;
    let hd = config.altar_depth * 0.5;
    let h = config.altar_height;
    let bevel = (hw.min(hd) * 0.25).clamp(4.0, 16.0);

    // 8-sided beveled top and bottom outlines
    let top_verts = vec![
        Vec3::new(-hw + bevel, -hd, h),
        Vec3::new(hw - bevel, -hd, h),
        Vec3::new(hw, -hd + bevel, h),
        Vec3::new(hw, hd - bevel, h),
        Vec3::new(hw - bevel, hd, h),
        Vec3::new(-hw + bevel, hd, h),
        Vec3::new(-hw, hd - bevel, h),
        Vec3::new(-hw, -hd + bevel, h),
    ];

    let mut bot_verts = vec![
        Vec3::new(-hw + bevel, -hd, 0.0),
        Vec3::new(hw - bevel, -hd, 0.0),
        Vec3::new(hw, -hd + bevel, 0.0),
        Vec3::new(hw, hd - bevel, 0.0),
        Vec3::new(hw - bevel, hd, 0.0),
        Vec3::new(-hw + bevel, hd, 0.0),
        Vec3::new(-hw, hd - bevel, 0.0),
        Vec3::new(-hw, -hd + bevel, 0.0),
    ];

    // Top polygon
    if let Some(poly) = Polygon::new(top_verts.clone()) {
        brush.polys.push(poly);
    }

    // Bottom polygon
    bot_verts.reverse();
    if let Some(poly) = Polygon::new(bot_verts) {
        brush.polys.push(poly);
    }

    // Re-create bottom forward for side quads
    let bot_fwd = vec![
        Vec3::new(-hw + bevel, -hd, 0.0),
        Vec3::new(hw - bevel, -hd, 0.0),
        Vec3::new(hw, -hd + bevel, 0.0),
        Vec3::new(hw, hd - bevel, 0.0),
        Vec3::new(hw - bevel, hd, 0.0),
        Vec3::new(-hw + bevel, hd, 0.0),
        Vec3::new(-hw, hd - bevel, 0.0),
        Vec3::new(-hw, -hd + bevel, 0.0),
    ];

    let n = top_verts.len();
    for i in 0..n {
        let next = (i + 1) % n;
        let quad = vec![bot_fwd[i], bot_fwd[next], top_verts[next], top_verts[i]];
        if let Some(poly) = Polygon::new(quad) {
            brush.polys.push(poly);
        }
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weapon_circuit_archetypes_and_defaults() {
        let cfg = WeaponCircuitArchetype::Deck16DuelMasterCircuit.to_config();
        assert_eq!(cfg.primary_weapon, TournamentWeaponType::ShockRifle);
        assert_eq!(cfg.flanking_ammo_count, 2);
        assert_eq!(cfg.respawn_delay_secs, 27.5);
    }

    #[test]
    fn test_weapon_circuit_telemetry_evaluation() {
        let cfg = WeaponCircuitConfig::default();
        let markers = vec![
            LevelWeaponMarker {
                name: "Flak0".to_string(),
                class_name: "Botpack.UT_FlakCannon".to_string(),
                position: Vec3::new(400.0, 0.0, 0.0),
                is_weapon: true,
            },
            LevelWeaponMarker {
                name: "Rocket0".to_string(),
                class_name: "Botpack.UT_Eightball".to_string(),
                position: Vec3::new(0.0, 800.0, 0.0),
                is_weapon: true,
            },
        ];

        let metrics = evaluate_weapon_circuit_metrics(&cfg, &markers);
        assert_eq!(metrics.total_weapons_in_level, 3); // 2 markers + 1 candidate
        assert!(metrics.avg_sprint_time_between_weapons_secs > 0.5);
        assert_eq!(metrics.circuit_links.len(), 3); // 3 pairs
    }

    #[test]
    fn test_weapon_safety_audit_and_clustering() {
        let cfg = WeaponCircuitConfig::default();
        let markers = vec![LevelWeaponMarker {
            name: "NearWeapon".to_string(),
            class_name: "Botpack.UT_FlakCannon".to_string(),
            position: Vec3::new(150.0, 0.0, 0.0), // < 300 uu!
            is_weapon: true,
        }];

        let hazards = audit_weapon_circuit_safety(&cfg, &markers);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Clustering")));
    }

    #[test]
    fn test_generate_weapon_altar_brush() {
        let cfg = WeaponCircuitConfig::default();
        let brush = generate_weapon_altar_brush(&cfg);

        assert_eq!(brush.name, "WeaponAltar");
        assert_eq!(brush.polys.len(), 10); // 8 sides + 2 caps
    }
}
