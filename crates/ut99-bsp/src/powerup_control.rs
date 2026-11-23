// crates/ut99-bsp/src/powerup_control.rs
//
// Interactive Tournament Powerup Control & Relic Altar Studio
// Multipliers, durations, respawn clock cadence, contention hotspot scoring,
// super-powerup stacking hazard detection, and procedural powerup altar generation.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// UT99 Canonical Tournament Powerup and Relic Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentPowerupType {
    UDamage,
    Invisibility,
    JumpBoots,
    RelicStrength,
    RelicRegen,
    RelicDefense,
    RelicSpeed,
    RelicRedemption,
    RelicDeath,
}

impl TournamentPowerupType {
    pub fn display_name(&self) -> &'static str {
        match self {
            TournamentPowerupType::UDamage => "UDamage (Damage Amplifier 3x)",
            TournamentPowerupType::Invisibility => "Invisibility (Translucent Cloak)",
            TournamentPowerupType::JumpBoots => "AntiGrav Jump Boots (3 High Leaps)",
            TournamentPowerupType::RelicStrength => "Relic of Strength (Damage Booster)",
            TournamentPowerupType::RelicRegen => "Relic of Regeneration (+10 HP/s)",
            TournamentPowerupType::RelicDefense => "Relic of Defense (60% Damage Shield)",
            TournamentPowerupType::RelicSpeed => "Relic of Speed (1.3x Movement Pace)",
            TournamentPowerupType::RelicRedemption => "Relic of Redemption (Life Saver)",
            TournamentPowerupType::RelicDeath => "Relic of Vengeance (Suicide Blast)",
        }
    }

    pub fn actor_class_name(&self) -> &'static str {
        match self {
            TournamentPowerupType::UDamage => "Botpack.UDamage",
            TournamentPowerupType::Invisibility => "Botpack.UT_Invisibility",
            TournamentPowerupType::JumpBoots => "Botpack.UT_JumpBoots",
            TournamentPowerupType::RelicStrength => "Relics.RelicStrength",
            TournamentPowerupType::RelicRegen => "Relics.RelicRegen",
            TournamentPowerupType::RelicDefense => "Relics.RelicDefense",
            TournamentPowerupType::RelicSpeed => "Relics.RelicSpeed",
            TournamentPowerupType::RelicRedemption => "Relics.RelicRedemption",
            TournamentPowerupType::RelicDeath => "Relics.RelicDeath",
        }
    }

    pub fn is_relic(&self) -> bool {
        matches!(
            self,
            TournamentPowerupType::RelicStrength
                | TournamentPowerupType::RelicRegen
                | TournamentPowerupType::RelicDefense
                | TournamentPowerupType::RelicSpeed
                | TournamentPowerupType::RelicRedemption
                | TournamentPowerupType::RelicDeath
        )
    }

    pub fn active_duration_secs(&self) -> f32 {
        match self {
            TournamentPowerupType::UDamage => 30.0,
            TournamentPowerupType::Invisibility => 50.0,
            TournamentPowerupType::JumpBoots => 60.0, // or 3 jump uses
            _ => 999.0, // Relics remain until carrier dies
        }
    }

    pub fn default_respawn_delay_secs(&self) -> f32 {
        match self {
            TournamentPowerupType::UDamage => 110.0,
            TournamentPowerupType::Invisibility => 110.0,
            TournamentPowerupType::JumpBoots => 30.0,
            _ => 15.0, // Relic respawns at random altar
        }
    }

    pub fn powerup_color_rgb(&self) -> [f32; 4] {
        match self {
            TournamentPowerupType::UDamage => [0.85, 0.20, 1.00, 0.95],      // Purple
            TournamentPowerupType::Invisibility => [0.20, 0.85, 1.00, 0.85],  // Cyan
            TournamentPowerupType::JumpBoots => [1.00, 0.55, 0.10, 0.95],     // Bright Orange
            TournamentPowerupType::RelicStrength => [1.00, 0.15, 0.15, 0.95], // Ruby Red
            TournamentPowerupType::RelicRegen => [0.20, 1.00, 0.40, 0.95],    // Emerald Green
            TournamentPowerupType::RelicDefense => [0.30, 0.60, 1.00, 0.95],  // Sapphire Blue
            TournamentPowerupType::RelicSpeed => [1.00, 1.00, 0.20, 0.95],    // Electric Yellow
            TournamentPowerupType::RelicRedemption => [1.0, 1.0, 1.0, 0.95],   // Divine White
            TournamentPowerupType::RelicDeath => [0.70, 0.10, 0.10, 0.95],     // Crimson Death
        }
    }
}

/// Archetypes of strategic tournament powerup placements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerupArchetype {
    Deck16HighLedgeUDamage,
    LiandriCoreCrossfireUDamage,
    PhobosGravityJumpBoots,
    AgonySacredRelicAltar,
    GothicDualPowerupClash,
}

impl PowerupArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            PowerupArchetype::Deck16HighLedgeUDamage => "Deck16][ High Ledge UDamage (Elevated Vantage)",
            PowerupArchetype::LiandriCoreCrossfireUDamage => "Liandri Core Bridge UDamage (Open Crossfire)",
            PowerupArchetype::PhobosGravityJumpBoots => "Phobos Orbital JumpBoots (Low-G High Mobility)",
            PowerupArchetype::AgonySacredRelicAltar => "Agony Sacred Relic Altar (Relic of Strength)",
            PowerupArchetype::GothicDualPowerupClash => "Gothic Dual Powerup Hub (UDamage & Invisibility)",
        }
    }

    pub fn to_config(&self) -> PowerupControlConfig {
        match self {
            PowerupArchetype::Deck16HighLedgeUDamage => PowerupControlConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                powerup_type: TournamentPowerupType::UDamage,
                respawn_delay_secs: 110.0,
                altar_radius: 48.0,
                altar_height: 18.0,
                containment_beam_height: 120.0,
                b_require_jump_skills: true,
                tag: "Deck16UDamageAltar".to_string(),
            },
            PowerupArchetype::LiandriCoreCrossfireUDamage => PowerupControlConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                powerup_type: TournamentPowerupType::UDamage,
                respawn_delay_secs: 110.0,
                altar_radius: 54.0,
                altar_height: 20.0,
                containment_beam_height: 140.0,
                b_require_jump_skills: false,
                tag: "LiandriCoreUDamageAltar".to_string(),
            },
            PowerupArchetype::PhobosGravityJumpBoots => PowerupControlConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                powerup_type: TournamentPowerupType::JumpBoots,
                respawn_delay_secs: 30.0,
                altar_radius: 42.0,
                altar_height: 14.0,
                containment_beam_height: 90.0,
                b_require_jump_skills: false,
                tag: "PhobosJumpBootsStation".to_string(),
            },
            PowerupArchetype::AgonySacredRelicAltar => PowerupControlConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                powerup_type: TournamentPowerupType::RelicStrength,
                respawn_delay_secs: 15.0,
                altar_radius: 50.0,
                altar_height: 24.0,
                containment_beam_height: 160.0,
                b_require_jump_skills: false,
                tag: "SacredStrengthRelicAltar".to_string(),
            },
            PowerupArchetype::GothicDualPowerupClash => PowerupControlConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                powerup_type: TournamentPowerupType::Invisibility,
                respawn_delay_secs: 110.0,
                altar_radius: 46.0,
                altar_height: 16.0,
                containment_beam_height: 110.0,
                b_require_jump_skills: true,
                tag: "GothicInvisAltar".to_string(),
            },
        }
    }
}

/// Configuration settings for Powerup Control & Relic Altar Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerupControlConfig {
    pub archetype: PowerupArchetype,
    pub position: Vec3,
    pub powerup_type: TournamentPowerupType,
    pub respawn_delay_secs: f32,
    pub altar_radius: f32,
    pub altar_height: f32,
    pub containment_beam_height: f32,
    pub b_require_jump_skills: bool,
    pub tag: String,
}

impl Default for PowerupControlConfig {
    fn default() -> Self {
        PowerupArchetype::Deck16HighLedgeUDamage.to_config()
    }
}

/// Representation of an existing nearby item/actor in the level
#[derive(Debug, Clone, PartialEq)]
pub struct NearbyLevelActor {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Analytical metrics for powerup control and match balance
#[derive(Debug, Clone, PartialEq)]
pub struct PowerupControlMetrics {
    pub active_duration_secs: f32,
    pub respawn_delay_secs: f32,
    pub powerup_uptime_pct: f32,
    pub contention_hotspot_rating: f32, // 0.0 to 100.0
    pub nearest_powerup_distance_uu: f32,
    pub nearest_shield_distance_uu: f32,
    pub nearest_weapon_distance_uu: f32,
}

/// Safety and balance hazards for tournament powerups
#[derive(Debug, Clone, PartialEq)]
pub struct PowerupHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates telemetry and contention dynamics for powerup placement
pub fn evaluate_powerup_metrics(
    config: &PowerupControlConfig,
    actors: &[NearbyLevelActor],
) -> PowerupControlMetrics {
    let active_dur = config.powerup_type.active_duration_secs();
    let total_cycle = active_dur + config.respawn_delay_secs;
    let uptime_pct = if total_cycle > 0.0 {
        (active_dur / total_cycle) * 100.0
    } else {
        0.0
    };

    let mut min_dist_powerup = f32::MAX;
    let mut min_dist_shield = f32::MAX;
    let mut min_dist_weapon = f32::MAX;

    for a in actors {
        let dist = a.position.distance(config.position);
        let lower = a.class_name.to_lowercase();
        if lower.contains("udamage") || lower.contains("invisibility") || lower.contains("relic") {
            if dist < min_dist_powerup { min_dist_powerup = dist; }
        }
        if lower.contains("shieldbelt") || lower.contains("armor") {
            if dist < min_dist_shield { min_dist_shield = dist; }
        }
        if lower.contains("weapon") || lower.contains("shock") || lower.contains("flak") || lower.contains("eightball") || lower.contains("sniper") {
            if dist < min_dist_weapon { min_dist_weapon = dist; }
        }
    }

    // Contention Hotspot Score: higher when central and near weapons, but punished if tucked into dead ends
    let mut contention_score = 65.0_f32;
    if min_dist_weapon < 600.0 {
        contention_score += 20.0;
    }
    if config.b_require_jump_skills {
        contention_score += 10.0;
    }
    contention_score = contention_score.clamp(10.0, 100.0);

    PowerupControlMetrics {
        active_duration_secs: active_dur,
        respawn_delay_secs: config.respawn_delay_secs,
        powerup_uptime_pct: uptime_pct,
        contention_hotspot_rating: contention_score,
        nearest_powerup_distance_uu: if min_dist_powerup < f32::MAX { min_dist_powerup } else { 9999.0 },
        nearest_shield_distance_uu: if min_dist_shield < f32::MAX { min_dist_shield } else { 9999.0 },
        nearest_weapon_distance_uu: if min_dist_weapon < f32::MAX { min_dist_weapon } else { 9999.0 },
    }
}

/// Audits powerup stacking hazards, shieldbelt contention and telefrag risks
pub fn audit_powerup_safety(
    config: &PowerupControlConfig,
    actors: &[NearbyLevelActor],
) -> Vec<PowerupHazard> {
    let mut hazards = Vec::new();

    // 1. Super-Stacking Hazard: UDamage placed too close to ShieldBelt (< 500 uu)
    for a in actors {
        let lower = a.class_name.to_lowercase();
        let dist = a.position.distance(config.position);
        if config.powerup_type == TournamentPowerupType::UDamage && lower.contains("shieldbelt") && dist < 500.0 {
            hazards.push(PowerupHazard {
                title: "Super-Stacking Imbalance Hazard".to_string(),
                description: format!(
                    "UDamage is only {:.1} uu from '{}' (ShieldBelt). A player claiming both simultaneously creates an unstoppable 3x damage 150HP juggernaut.",
                    dist, a.name
                ),
                is_critical: true,
            });
            break;
        }
    }

    // 2. Powerup Clustering (< 400 uu)
    for a in actors {
        let lower = a.class_name.to_lowercase();
        let dist = a.position.distance(config.position);
        if (lower.contains("udamage") || lower.contains("invisibility") || lower.contains("relic")) && dist < 400.0 {
            hazards.push(PowerupHazard {
                title: "Excessive Powerup Clustering".to_string(),
                description: format!(
                    "Altar is only {:.1} uu from existing powerup '{}'. Distribute powerups across opposing sides of the arena.",
                    dist, a.name
                ),
                is_critical: true,
            });
            break;
        }
    }

    // 3. Relic of Death Suicide Warning
    if config.powerup_type == TournamentPowerupType::RelicDeath {
        hazards.push(PowerupHazard {
            title: "Relic of Death Explosive Peril".to_string(),
            description: "Carrier death triggers a massive radial explosion. Do not place in tight spawn corridors.".to_string(),
            is_critical: false,
        });
    }

    // 4. Low Respawn Time on UDamage (< 60s)
    if config.powerup_type == TournamentPowerupType::UDamage && config.respawn_delay_secs < 60.0 {
        hazards.push(PowerupHazard {
            title: "Fast UDamage Cadence Risk".to_string(),
            description: format!("Respawn delay is {:.0}s (< standard 110s). Fast respawn leads to perpetual UDamage dominance.", config.respawn_delay_secs),
            is_critical: false,
        });
    }

    hazards
}

/// Generates an octagonal powerup altar brush with an excavated circular central energy core
pub fn generate_powerup_altar_brush(config: &PowerupControlConfig) -> Brush {
    let mut brush = Brush::new("PowerupAltar".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let r = config.altar_radius;
    let h = config.altar_height;
    let num_sides = 8;

    let mut top_ring = Vec::new();
    let mut bot_ring = Vec::new();

    for i in 0..num_sides {
        let angle = std::f32::consts::TAU * (i as f32) / (num_sides as f32);
        let x = r * angle.cos();
        let y = r * angle.sin();
        top_ring.push(Vec3::new(x, y, h));
        bot_ring.push(Vec3::new(x, y, 0.0));
    }

    // Top polygon
    if let Some(poly) = Polygon::new(top_ring.clone()) {
        brush.polys.push(poly);
    }

    // Bottom polygon (reversed winding)
    let mut bot_rev = bot_ring.clone();
    bot_rev.reverse();
    if let Some(poly) = Polygon::new(bot_rev) {
        brush.polys.push(poly);
    }

    // Side quad polygons
    for i in 0..num_sides {
        let next = (i + 1) % num_sides;
        let quad = vec![bot_ring[i], bot_ring[next], top_ring[next], top_ring[i]];
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
    fn test_powerup_archetypes_and_defaults() {
        let cfg = PowerupArchetype::Deck16HighLedgeUDamage.to_config();
        assert_eq!(cfg.powerup_type, TournamentPowerupType::UDamage);
        assert_eq!(cfg.respawn_delay_secs, 110.0);
        assert!(cfg.b_require_jump_skills);
    }

    #[test]
    fn test_powerup_metrics_evaluation() {
        let cfg = PowerupControlConfig::default();
        let actors = vec![
            NearbyLevelActor {
                name: "ShockRifle0".to_string(),
                class_name: "Botpack.ShockRifle".to_string(),
                position: Vec3::new(300.0, 0.0, 0.0),
            },
            NearbyLevelActor {
                name: "ShieldBelt0".to_string(),
                class_name: "Botpack.UT_ShieldBelt".to_string(),
                position: Vec3::new(1200.0, 0.0, 0.0),
            },
        ];

        let metrics = evaluate_powerup_metrics(&cfg, &actors);
        assert!(metrics.powerup_uptime_pct > 0.0);
        assert!(metrics.contention_hotspot_rating >= 80.0);
        assert_eq!(metrics.nearest_weapon_distance_uu, 300.0);
        assert_eq!(metrics.nearest_shield_distance_uu, 1200.0);
    }

    #[test]
    fn test_powerup_stacking_hazard_audit() {
        let cfg = PowerupControlConfig::default(); // UDamage
        let actors = vec![NearbyLevelActor {
            name: "NearShieldBelt".to_string(),
            class_name: "Botpack.UT_ShieldBelt".to_string(),
            position: Vec3::new(200.0, 0.0, 0.0), // < 500 uu!
        }];

        let hazards = audit_powerup_safety(&cfg, &actors);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Super-Stacking")));
    }

    #[test]
    fn test_generate_powerup_altar_brush() {
        let cfg = PowerupControlConfig::default();
        let brush = generate_powerup_altar_brush(&cfg);

        assert_eq!(brush.name, "PowerupAltar");
        assert_eq!(brush.polys.len(), 10); // 8 sides + 2 caps
    }
}
