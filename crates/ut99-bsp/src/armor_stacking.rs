// crates/ut99-bsp/src/armor_stacking.rs
//
// Interactive Armor, Health & ShieldBelt Stacking & Regeneration Studio
//
// Native 64-bit mathematical modeling, absorption hierarchy simulation,
// Effective Health (EHP) calculation, pickup clustering hazard detection,
// and procedural pedestal geometry generation for Unreal Tournament 99 / UnrealEd 2.0 / 2.1 supremacy.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// UT99 Armor Classifications with canonical absorption percentages and respawn timings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArmorType {
    /// 100% absorption, +100 up to 150, 110s respawn
    ShieldBelt,
    /// 75% absorption, +100 up to 100, 55s respawn
    BodyArmor,
    /// 50% absorption, +50 up to 50, 30s respawn
    ThighPads,
}

impl ArmorType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ArmorType::ShieldBelt => "UT_ShieldBelt (100% Absorb, +100-150)",
            ArmorType::BodyArmor => "Armor2 / Body Armor (75% Absorb, +100)",
            ArmorType::ThighPads => "ThighPads (50% Absorb, +50)",
        }
    }

    pub fn class_name(&self) -> &'static str {
        match self {
            ArmorType::ShieldBelt => "Botpack.UT_ShieldBelt",
            ArmorType::BodyArmor => "Botpack.Armor2",
            ArmorType::ThighPads => "Botpack.ThighPads",
        }
    }

    pub fn base_value(&self) -> u32 {
        match self {
            ArmorType::ShieldBelt => 100,
            ArmorType::BodyArmor => 100,
            ArmorType::ThighPads => 50,
        }
    }

    pub fn max_cap(&self) -> u32 {
        match self {
            ArmorType::ShieldBelt => 150,
            ArmorType::BodyArmor => 100,
            ArmorType::ThighPads => 50,
        }
    }

    pub fn absorption_rate(&self) -> f32 {
        match self {
            ArmorType::ShieldBelt => 1.0,
            ArmorType::BodyArmor => 0.75,
            ArmorType::ThighPads => 0.50,
        }
    }

    pub fn default_respawn_secs(&self) -> f32 {
        match self {
            ArmorType::ShieldBelt => 110.0,
            ArmorType::BodyArmor => 55.0,
            ArmorType::ThighPads => 30.0,
        }
    }

    pub fn shield_color_rgb(&self) -> [f32; 3] {
        match self {
            ArmorType::ShieldBelt => [1.0, 0.85, 0.1], // Golden aura
            ArmorType::BodyArmor => [0.15, 0.45, 0.95], // Electric cobalt
            ArmorType::ThighPads => [0.2, 0.85, 0.4],   // Emerald green
        }
    }
}

/// UT99 Health Pickup Classifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthType {
    /// +20 HP, max 100 HP, 20s respawn
    MedBox,
    /// +5 HP, over-stack to 199 HP, 20s respawn
    HealthVial,
    /// +100 HP, over-stack to 199 HP, 110s respawn
    KegOfHealth,
}

impl HealthType {
    pub fn display_name(&self) -> &'static str {
        match self {
            HealthType::MedBox => "MedBox (+20 HP, cap 100)",
            HealthType::HealthVial => "HealthVial (+5 HP, overstack 199)",
            HealthType::KegOfHealth => "KegOfHealth (+100 HP, overstack 199)",
        }
    }

    pub fn class_name(&self) -> &'static str {
        match self {
            HealthType::MedBox => "Botpack.MedBox",
            HealthType::HealthVial => "Botpack.HealthVial",
            HealthType::KegOfHealth => "Botpack.HealthPack",
        }
    }

    pub fn heal_amount(&self) -> u32 {
        match self {
            HealthType::MedBox => 20,
            HealthType::HealthVial => 5,
            HealthType::KegOfHealth => 100,
        }
    }

    pub fn can_overstack(&self) -> bool {
        matches!(self, HealthType::HealthVial | HealthType::KegOfHealth)
    }

    pub fn default_respawn_secs(&self) -> f32 {
        match self {
            HealthType::MedBox => 20.0,
            HealthType::HealthVial => 20.0,
            HealthType::KegOfHealth => 110.0,
        }
    }
}

/// Pre-calibrated tournament pickup balance archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArmorArchetype {
    ClassicDeck16Stack,
    HardcoreDuelStandoff,
    TeamFortressHeavyDefense,
    InstagibCleanSweep,
    RelicRegenPowerSurge,
}

impl ArmorArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            ArmorArchetype::ClassicDeck16Stack => "Classic Deck16 Balanced Stack",
            ArmorArchetype::HardcoreDuelStandoff => "Hardcore 1v1 Duel Standoff",
            ArmorArchetype::TeamFortressHeavyDefense => "Team Fortress Heavy Base Defense",
            ArmorArchetype::InstagibCleanSweep => "Instagib Clean Sweep (Zero Armor)",
            ArmorArchetype::RelicRegenPowerSurge => "Relic Regen & Power Surge",
        }
    }

    pub fn to_config(&self) -> ArmorStackingConfig {
        match self {
            ArmorArchetype::ClassicDeck16Stack => ArmorStackingConfig {
                archetype: *self,
                position: Vec3::ZERO,
                shield_belt_count: 1,
                body_armor_count: 1,
                thigh_pads_count: 1,
                keg_of_health_count: 1,
                med_box_count: 4,
                health_vial_count: 8,
                pedestal_radius: 64.0,
                pedestal_height: 24.0,
                pedestal_sides: 8,
                respawn_multiplier: 1.0,
                regeneration_rate_hp_per_sec: 0.0,
                max_health_overstack: 199,
                tag: "ClassicDeckArmor".to_string(),
            },
            ArmorArchetype::HardcoreDuelStandoff => ArmorStackingConfig {
                archetype: *self,
                position: Vec3::ZERO,
                shield_belt_count: 1,
                body_armor_count: 1,
                thigh_pads_count: 0,
                keg_of_health_count: 0,
                med_box_count: 2,
                health_vial_count: 4,
                pedestal_radius: 56.0,
                pedestal_height: 16.0,
                pedestal_sides: 6,
                respawn_multiplier: 1.0,
                regeneration_rate_hp_per_sec: 0.0,
                max_health_overstack: 199,
                tag: "DuelStandoffArmor".to_string(),
            },
            ArmorArchetype::TeamFortressHeavyDefense => ArmorStackingConfig {
                archetype: *self,
                position: Vec3::ZERO,
                shield_belt_count: 2,
                body_armor_count: 2,
                thigh_pads_count: 2,
                keg_of_health_count: 1,
                med_box_count: 6,
                health_vial_count: 12,
                pedestal_radius: 72.0,
                pedestal_height: 32.0,
                pedestal_sides: 8,
                respawn_multiplier: 0.8,
                regeneration_rate_hp_per_sec: 0.0,
                max_health_overstack: 199,
                tag: "TeamFortressArmor".to_string(),
            },
            ArmorArchetype::InstagibCleanSweep => ArmorStackingConfig {
                archetype: *self,
                position: Vec3::ZERO,
                shield_belt_count: 0,
                body_armor_count: 0,
                thigh_pads_count: 0,
                keg_of_health_count: 0,
                med_box_count: 0,
                health_vial_count: 0,
                pedestal_radius: 48.0,
                pedestal_height: 12.0,
                pedestal_sides: 6,
                respawn_multiplier: 0.0,
                regeneration_rate_hp_per_sec: 0.0,
                max_health_overstack: 100,
                tag: "InstagibZeroArmor".to_string(),
            },
            ArmorArchetype::RelicRegenPowerSurge => ArmorStackingConfig {
                archetype: *self,
                position: Vec3::ZERO,
                shield_belt_count: 1,
                body_armor_count: 2,
                thigh_pads_count: 2,
                keg_of_health_count: 2,
                med_box_count: 4,
                health_vial_count: 16,
                pedestal_radius: 64.0,
                pedestal_height: 24.0,
                pedestal_sides: 8,
                respawn_multiplier: 0.5,
                regeneration_rate_hp_per_sec: 2.0,
                max_health_overstack: 199,
                tag: "RelicRegenArmor".to_string(),
            },
        }
    }
}

/// Armor, Health, and ShieldBelt Studio configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorStackingConfig {
    pub archetype: ArmorArchetype,
    pub position: Vec3,
    pub shield_belt_count: usize,
    pub body_armor_count: usize,
    pub thigh_pads_count: usize,
    pub keg_of_health_count: usize,
    pub med_box_count: usize,
    pub health_vial_count: usize,
    pub pedestal_radius: f32,
    pub pedestal_height: f32,
    pub pedestal_sides: usize,
    pub respawn_multiplier: f32,
    pub regeneration_rate_hp_per_sec: f32,
    pub max_health_overstack: u32,
    pub tag: String,
}

impl Default for ArmorStackingConfig {
    fn default() -> Self {
        ArmorArchetype::ClassicDeck16Stack.to_config()
    }
}

/// Simulation result of incoming damage across the UT99 absorption hierarchy
#[derive(Debug, Clone, PartialEq)]
pub struct DamageAbsorptionResult {
    pub initial_damage: f32,
    pub final_health: f32,
    pub final_shield_belt: f32,
    pub final_body_armor: f32,
    pub final_thigh_pads: f32,
    pub health_damage_taken: f32,
    pub total_armor_absorbed: f32,
    pub player_survived: bool,
}

/// Canonical UT99 Damage Absorption Pipeline:
/// 1. ShieldBelt takes 100% damage until depleted.
/// 2. Remaining damage passes to Body Armor (75% absorbed, 25% to health).
/// 3. If Body Armor depleted, remaining damage passes to ThighPads (50% absorbed, 50% to health).
/// 4. Any unabsorbed damage penetrates directly to Health.
pub fn simulate_damage_absorption(
    incoming_damage: f32,
    mut health: f32,
    mut shield_belt: f32,
    mut body_armor: f32,
    mut thigh_pads: f32,
) -> DamageAbsorptionResult {
    let mut remaining_dmg = incoming_damage;
    let mut total_armor_absorbed = 0.0_f32;
    let mut health_damage_taken = 0.0_f32;

    // Phase 1: ShieldBelt (100% absorption)
    if shield_belt > 0.0 && remaining_dmg > 0.0 {
        let absorbed = shield_belt.min(remaining_dmg);
        shield_belt -= absorbed;
        remaining_dmg -= absorbed;
        total_armor_absorbed += absorbed;
    }

    // Phase 2: Body Armor (75% absorption)
    if body_armor > 0.0 && remaining_dmg > 0.0 {
        let max_armor_absorb = remaining_dmg * 0.75;
        let actual_absorb = body_armor.min(max_armor_absorb);
        let actual_dmg_processed = actual_absorb / 0.75;

        body_armor -= actual_absorb;
        total_armor_absorbed += actual_absorb;

        let pass_through_to_hp = actual_dmg_processed * 0.25;
        health_damage_taken += pass_through_to_hp;
        remaining_dmg -= actual_dmg_processed;
    }

    // Phase 3: ThighPads (50% absorption)
    if thigh_pads > 0.0 && remaining_dmg > 0.0 {
        let max_absorb = remaining_dmg * 0.50;
        let actual_absorb = thigh_pads.min(max_absorb);
        let actual_dmg_processed = actual_absorb / 0.50;

        thigh_pads -= actual_absorb;
        total_armor_absorbed += actual_absorb;

        let pass_through_to_hp = actual_dmg_processed * 0.50;
        health_damage_taken += pass_through_to_hp;
        remaining_dmg -= actual_dmg_processed;
    }

    // Phase 4: Remaining raw damage hits health
    if remaining_dmg > 0.0 {
        health_damage_taken += remaining_dmg;
    }

    health = (health - health_damage_taken).max(0.0);

    DamageAbsorptionResult {
        initial_damage: incoming_damage,
        final_health: health,
        final_shield_belt: shield_belt,
        final_body_armor: body_armor,
        final_thigh_pads: thigh_pads,
        health_damage_taken,
        total_armor_absorbed,
        player_survived: health > 0.0,
    }
}

/// Evaluates Maximum Effective Health (EHP) from a player holding a specific stack
pub fn calculate_effective_health(health: f32, shield: f32, body: f32, thigh: f32) -> f32 {
    // ShieldBelt is 1:1 direct EHP
    // Body Armor absorbs 75%, meaning for every 1 HP lost, 3 Armor is lost. Max EHP = Health + Armor + ...
    // More precisely: while ShieldBelt is up: absorbs S damage directly.
    // While Body Armor is up: each 1 damage takes 0.25 HP and 0.75 Armor. So B armor protects for B / 0.75 damage, costing B/3 HP.
    // If player has enough HP to sustain the 25% bleed-through, body armor adds 1.333x its value in effective damage absorption.
    let ehp_shield = shield;
    let ehp_body = body / 0.75;
    let ehp_thigh = thigh / 0.50;
    health + ehp_shield + ehp_body + ehp_thigh
}

/// Analytical metrics of armor and health configuration in the level
#[derive(Debug, Clone, PartialEq)]
pub struct ArmorStackingMetrics {
    pub total_armor_pool: u32,
    pub total_health_pool: u32,
    pub max_effective_hp: f32,
    pub armor_to_health_ratio: f32,
    pub stacking_balance_score: f32, // 0.0 to 100.0%
    pub shield_belt_count: usize,
    pub body_armor_count: usize,
    pub thigh_pads_count: usize,
    pub keg_count: usize,
    pub clustering_hazard_count: usize,
}

/// Diagnostic health issues for armor and health distribution
#[derive(Debug, Clone, PartialEq)]
pub struct ArmorHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

pub type ArmorStackingHazard = ArmorHazard;

/// Pickup position marker in level space for clustering analysis
#[derive(Debug, Clone, PartialEq)]
pub struct PickupMarker {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
    pub is_major_armor_or_keg: bool,
}

/// Calculates analytical metrics for armor and health distribution
pub fn evaluate_armor_stacking_metrics(
    config: &ArmorStackingConfig,
    world_pickups: &[PickupMarker],
) -> ArmorStackingMetrics {
    let mut total_armor = (config.shield_belt_count as u32 * 100)
        + (config.body_armor_count as u32 * 100)
        + (config.thigh_pads_count as u32 * 50);

    let mut total_health = (config.keg_of_health_count as u32 * 100)
        + (config.med_box_count as u32 * 20)
        + (config.health_vial_count as u32 * 5);

    let mut sb_count = config.shield_belt_count;
    let mut ba_count = config.body_armor_count;
    let mut tp_count = config.thigh_pads_count;
    let mut keg_count = config.keg_of_health_count;

    // Factor in actual world pickups if present
    for p in world_pickups {
        let lower = p.class_name.to_lowercase();
        if lower.contains("shieldbelt") {
            total_armor += 100;
            sb_count += 1;
        } else if lower.contains("armor2") {
            total_armor += 100;
            ba_count += 1;
        } else if lower.contains("thighpads") {
            total_armor += 50;
            tp_count += 1;
        } else if lower.contains("healthpack") || lower.contains("keg") {
            total_health += 100;
            keg_count += 1;
        } else if lower.contains("medbox") {
            total_health += 20;
        } else if lower.contains("vial") {
            total_health += 5;
        }
    }

    let max_ehp = calculate_effective_health(
        config.max_health_overstack as f32,
        if sb_count > 0 { 150.0 } else { 0.0 },
        if ba_count > 0 { 100.0 } else { 0.0 },
        if tp_count > 0 { 50.0 } else { 0.0 },
    );

    let armor_to_health_ratio = if total_health > 0 {
        total_armor as f32 / total_health as f32
    } else {
        0.0
    };

    // Calculate clustering hazards
    let mut cluster_count = 0;
    let majors: Vec<&PickupMarker> = world_pickups.iter().filter(|p| p.is_major_armor_or_keg).collect();
    for i in 0..majors.len() {
        for j in (i + 1)..majors.len() {
            if majors[i].position.distance(majors[j].position) < 400.0 {
                cluster_count += 1;
            }
        }
    }

    // Competitive Balance Score
    let mut score = 88.0_f32;
    if config.archetype == ArmorArchetype::InstagibCleanSweep {
        score = 100.0;
    } else {
        if sb_count > 2 {
            score -= 25.0; // Tank hoarding penalty
        }
        if cluster_count > 0 {
            score -= (cluster_count as f32 * 15.0).min(30.0);
        }
        if total_armor > 0 && (armor_to_health_ratio < 0.4 || armor_to_health_ratio > 2.5) {
            score -= 15.0; // Ratio skew penalty
        }
    }

    ArmorStackingMetrics {
        total_armor_pool: total_armor,
        total_health_pool: total_health,
        max_effective_hp: max_ehp,
        armor_to_health_ratio,
        stacking_balance_score: score.clamp(0.0, 100.0),
        shield_belt_count: sb_count,
        body_armor_count: ba_count,
        thigh_pads_count: tp_count,
        keg_count,
        clustering_hazard_count: cluster_count,
    }
}

/// Audits safety, over-saturation, and clustering hazards
pub fn audit_armor_stacking_safety(
    config: &ArmorStackingConfig,
    world_pickups: &[PickupMarker],
) -> Vec<ArmorHazard> {
    let mut hazards = Vec::new();

    if config.archetype == ArmorArchetype::InstagibCleanSweep {
        return hazards; // Clean sweep has no pickups
    }

    let metrics = evaluate_armor_stacking_metrics(config, world_pickups);

    // 1. ShieldBelt Over-Saturation
    if metrics.shield_belt_count > 2 {
        hazards.push(ArmorHazard {
            title: "ShieldBelt Over-Saturation Hazard".to_string(),
            description: format!(
                "Level contains {} ShieldBelts (> 2). Players can chain golden invulnerability indefinitely.",
                metrics.shield_belt_count
            ),
            is_critical: true,
        });
    }

    // 2. Pickup Clustering Hazard (< 400 uu)
    let majors: Vec<&PickupMarker> = world_pickups.iter().filter(|p| p.is_major_armor_or_keg).collect();
    for i in 0..majors.len() {
        for j in (i + 1)..majors.len() {
            let dist = majors[i].position.distance(majors[j].position);
            if dist < 400.0 {
                hazards.push(ArmorHazard {
                    title: "Dangerous Major Pickup Clustering".to_string(),
                    description: format!(
                        "'{}' and '{}' are only {:.0} uu apart (< 400 uu). Allows instant 200+ armor/HP hoards.",
                        majors[i].name, majors[j].name, dist
                    ),
                    is_critical: true,
                });
            }
        }
    }

    // 3. Armor Starvation in Standard Combat
    if metrics.total_armor_pool < 50 && config.shield_belt_count == 0 && config.body_armor_count == 0 {
        hazards.push(ArmorHazard {
            title: "Severe Armor Starvation Hazard".to_string(),
            description: "No major armors or ShieldBelts are placed. Time-To-Kill will be excessively brutal.".to_string(),
            is_critical: false,
        });
    }

    // 4. Missing Over-Stacking Support
    if metrics.keg_count == 0 && config.health_vial_count == 0 {
        hazards.push(ArmorHazard {
            title: "No Health Over-Stacking Mechanism".to_string(),
            description: "Level lacks both Keg of Health and Health Vials. Players are locked at 100 HP max, removing clutch mechanics.".to_string(),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural stepped pedestal brush for armor / powerup spawn stations
pub fn generate_armor_pedestal_brush(config: &ArmorStackingConfig) -> Brush {
    let mut brush = Brush::new("ArmorPedestal".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let sides = config.pedestal_sides.max(6).min(16);
    let r = config.pedestal_radius;
    let h = config.pedestal_height;

    let mut bottom_verts = Vec::with_capacity(sides);
    let mut top_verts = Vec::with_capacity(sides);

    for i in 0..sides {
        let angle = (i as f32 / sides as f32) * std::f32::consts::TAU;
        let x = angle.cos() * r;
        let y = angle.sin() * r;
        bottom_verts.push(Vec3::new(x, y, 0.0));
        top_verts.push(Vec3::new(x, y, h));
    }

    // Top face
    if let Some(poly) = Polygon::new(top_verts.clone()) {
        brush.polys.push(poly);
    }

    // Bottom face (reversed for outward normal)
    let mut bot_rev = bottom_verts.clone();
    bot_rev.reverse();
    if let Some(poly) = Polygon::new(bot_rev) {
        brush.polys.push(poly);
    }

    // Side quad faces
    for i in 0..sides {
        let next = (i + 1) % sides;
        let quad = vec![
            bottom_verts[i],
            bottom_verts[next],
            top_verts[next],
            top_verts[i],
        ];
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
    fn test_damage_absorption_priority_hierarchy() {
        // Player has 100 HP, 100 ShieldBelt, 100 Body Armor, 50 ThighPads
        // Scenario 1: 50 Damage from Shock Rifle Beam
        let res1 = simulate_damage_absorption(50.0, 100.0, 100.0, 100.0, 50.0);
        assert_eq!(res1.final_health, 100.0, "Health must not be damaged when ShieldBelt is up");
        assert_eq!(res1.final_shield_belt, 50.0);
        assert_eq!(res1.final_body_armor, 100.0, "Body Armor should not be touched while ShieldBelt absorbs");
        assert_eq!(res1.final_thigh_pads, 50.0);
        assert!(res1.player_survived);

        // Scenario 2: 120 Damage from Rocket (Depletes 100 ShieldBelt, remaining 20 hits Body Armor)
        // 20 damage on Body Armor: 75% (15) absorbed by Body Armor, 25% (5) hits Health
        let res2 = simulate_damage_absorption(120.0, 100.0, 100.0, 100.0, 50.0);
        assert_eq!(res2.final_shield_belt, 0.0);
        assert_eq!(res2.final_body_armor, 85.0); // 100 - 15 = 85
        assert_eq!(res2.final_health, 95.0); // 100 - 5 = 95
        assert_eq!(res2.final_thigh_pads, 50.0);

        // Scenario 3: Fatal Overkill 500 Damage from Redeemer
        let res3 = simulate_damage_absorption(500.0, 100.0, 100.0, 100.0, 50.0);
        assert_eq!(res3.final_health, 0.0);
        assert!(!res3.player_survived);
    }

    #[test]
    fn test_effective_health_calculation() {
        // Base 100 HP only
        let ehp1 = calculate_effective_health(100.0, 0.0, 0.0, 0.0);
        assert_eq!(ehp1, 100.0);

        // 199 Health + 150 ShieldBelt + 100 BodyArmor + 50 ThighPads
        let ehp_max = calculate_effective_health(199.0, 150.0, 100.0, 50.0);
        // 199 + 150 + (100 / 0.75 = 133.33) + (50 / 0.50 = 100) = 582.33
        assert!(ehp_max > 580.0 && ehp_max < 585.0);
    }

    #[test]
    fn test_armor_archetypes_and_defaults() {
        let deck = ArmorArchetype::ClassicDeck16Stack.to_config();
        assert_eq!(deck.shield_belt_count, 1);
        assert_eq!(deck.body_armor_count, 1);
        assert_eq!(deck.keg_of_health_count, 1);

        let duel = ArmorArchetype::HardcoreDuelStandoff.to_config();
        assert_eq!(duel.keg_of_health_count, 0);
        assert_eq!(duel.thigh_pads_count, 0);

        let insta = ArmorArchetype::InstagibCleanSweep.to_config();
        assert_eq!(insta.shield_belt_count, 0);
        assert_eq!(insta.body_armor_count, 0);
    }

    #[test]
    fn test_audit_armor_safety_detects_clustering_and_saturation() {
        let cfg = ArmorArchetype::ClassicDeck16Stack.to_config();

        // 1. Clean world
        let h_clean = audit_armor_stacking_safety(&cfg, &[]);
        assert!(h_clean.is_empty());

        // 2. Clustered pickups (ShieldBelt and Keg within 200 uu)
        let pickups = vec![
            PickupMarker {
                name: "ShieldBelt0".to_string(),
                class_name: "Botpack.UT_ShieldBelt".to_string(),
                position: Vec3::new(100.0, 0.0, 0.0),
                is_major_armor_or_keg: true,
            },
            PickupMarker {
                name: "KegOfHealth0".to_string(),
                class_name: "Botpack.HealthPack".to_string(),
                position: Vec3::new(250.0, 0.0, 0.0), // distance = 150 uu (< 400)
                is_major_armor_or_keg: true,
            },
        ];

        let h_cluster = audit_armor_stacking_safety(&cfg, &pickups);
        assert!(h_cluster.iter().any(|h| h.is_critical && h.title.contains("Clustering")));

        // 3. ShieldBelt over-saturation (> 2)
        let mut cfg_saturated = cfg.clone();
        cfg_saturated.shield_belt_count = 3;
        let h_sat = audit_armor_stacking_safety(&cfg_saturated, &[]);
        assert!(h_sat.iter().any(|h| h.is_critical && h.title.contains("Over-Saturation")));
    }

    #[test]
    fn test_generate_armor_pedestal_brush() {
        let cfg = ArmorArchetype::ClassicDeck16Stack.to_config();
        let brush = generate_armor_pedestal_brush(&cfg);

        assert_eq!(brush.name, "ArmorPedestal");
        assert_eq!(brush.polys.len(), cfg.pedestal_sides + 2); // 8 sides + top + bottom = 10
    }
}
