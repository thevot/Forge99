// crates/ut99-bsp/src/fluid_basin_studio.rs
//
// Interactive Toxic Slime Acid Basin, Lava Flow & Caustic Pool Studio
// Fluid excavation trough geometry, caustic under-lighting, pain zone DPS calibration,
// buoyancy/viscous drag kinematics, escape ladder safety, and procedural basin brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Archetypes of canonical hazardous fluid pools and basins in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FluidBasinArchetype {
    CorrosiveDeckSlimeVat,
    MoltenMagmaChasm,
    IndustrialCoolantReservoir,
    NaliSacredHealingSpring,
    CryogenicSubzeroBrine,
}

impl FluidBasinArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            FluidBasinArchetype::CorrosiveDeckSlimeVat => "Corrosive Deck Slime Vat (DM-Deck16][ Acid Pool)",
            FluidBasinArchetype::MoltenMagmaChasm => "Molten Magma Chasm (CTF-LavaGiant Lava River)",
            FluidBasinArchetype::IndustrialCoolantReservoir => "Industrial Coolant Reservoir (CTF-Gauntlet Water)",
            FluidBasinArchetype::NaliSacredHealingSpring => "Nali Sacred Healing Spring (Regenerative Waters)",
            FluidBasinArchetype::CryogenicSubzeroBrine => "Cryogenic Subzero Brine (Frostbite Trench)",
        }
    }

    pub fn to_config(&self) -> FluidBasinConfig {
        match self {
            FluidBasinArchetype::CorrosiveDeckSlimeVat => FluidBasinConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                width_uu: 256.0,
                length_uu: 384.0,
                depth_uu: 64.0,
                dps: 40.0,
                fluid_friction: 2.5,
                buoyancy_factor: 1.1,
                has_escape_ladder_or_ramp: true,
                damage_type: "Corroded".to_string(),
                tag: "DeckSlimeBasin0".to_string(),
            },
            FluidBasinArchetype::MoltenMagmaChasm => FluidBasinConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                width_uu: 320.0,
                length_uu: 512.0,
                depth_uu: 128.0,
                dps: 80.0,
                fluid_friction: 3.5,
                buoyancy_factor: 0.8,
                has_escape_ladder_or_ramp: false, // Intentionally testable hazard
                damage_type: "Burned".to_string(),
                tag: "MagmaChasm0".to_string(),
            },
            FluidBasinArchetype::IndustrialCoolantReservoir => FluidBasinConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                width_uu: 256.0,
                length_uu: 256.0,
                depth_uu: 96.0,
                dps: 0.0,
                fluid_friction: 1.0,
                buoyancy_factor: 1.0,
                has_escape_ladder_or_ramp: true,
                damage_type: "None".to_string(),
                tag: "CoolantReservoir0".to_string(),
            },
            FluidBasinArchetype::NaliSacredHealingSpring => FluidBasinConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                width_uu: 192.0,
                length_uu: 192.0,
                depth_uu: 40.0,
                dps: -15.0, // Health restoration
                fluid_friction: 1.0,
                buoyancy_factor: 1.05,
                has_escape_ladder_or_ramp: true,
                damage_type: "Healed".to_string(),
                tag: "HealingSpring0".to_string(),
            },
            FluidBasinArchetype::CryogenicSubzeroBrine => FluidBasinConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                width_uu: 224.0,
                length_uu: 320.0,
                depth_uu: 60.0,
                dps: 20.0,
                fluid_friction: 0.5,
                buoyancy_factor: 0.95,
                has_escape_ladder_or_ramp: true,
                damage_type: "Frozen".to_string(),
                tag: "CryoBrineTrench0".to_string(),
            },
        }
    }
}

/// Configuration settings for a fluid hazard basin and pain zone
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FluidBasinConfig {
    pub archetype: FluidBasinArchetype,
    pub position: Vec3,
    pub width_uu: f32,
    pub length_uu: f32,
    pub depth_uu: f32,
    pub dps: f32,
    pub fluid_friction: f32,
    pub buoyancy_factor: f32,
    pub has_escape_ladder_or_ramp: bool,
    pub damage_type: String,
    pub tag: String,
}

impl Default for FluidBasinConfig {
    fn default() -> Self {
        FluidBasinArchetype::CorrosiveDeckSlimeVat.to_config()
    }
}

/// Context actor record for evaluating basin surroundings
#[derive(Debug, Clone, PartialEq)]
pub struct LevelBasinContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Telemetry metrics for a fluid hazard basin
#[derive(Debug, Clone, PartialEq)]
pub struct FluidBasinMetrics {
    pub fluid_volume_k_cu_uu: f32,
    pub player_survival_time_secs: f32,
    pub terminal_sink_speed_uu_s: f32,
    pub total_hazard_basins_in_level: usize,
}

/// Safety diagnostics for fluid basins
#[derive(Debug, Clone, PartialEq)]
pub struct FluidBasinHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates fluid capacity, player survival duration under DPS, and sink velocity
pub fn evaluate_fluid_basin_metrics(
    config: &FluidBasinConfig,
    peers: &[LevelBasinContextActorRecord],
) -> FluidBasinMetrics {
    let vol_cu_uu = config.width_uu * config.length_uu * config.depth_uu;
    let vol_k = vol_cu_uu / 1000.0;

    // Player survival time: standard 100 HP / DPS
    let survival_time = if config.dps > 0.0 {
        100.0 / config.dps
    } else {
        f32::INFINITY
    };

    // Terminal sink speed in fluid: UT99 terminal velocity in fluid ~ 250 uu/s / friction
    let terminal_sink = (250.0 / config.fluid_friction.max(0.1)) * (1.0 / config.buoyancy_factor.max(0.1));

    let mut basins_count = 1;
    for p in peers {
        let cls_lower = p.class_name.to_lowercase();
        if cls_lower.contains("zone") || cls_lower.contains("pain") || cls_lower.contains("water") {
            basins_count += 1;
        }
    }

    FluidBasinMetrics {
        fluid_volume_k_cu_uu: vol_k,
        player_survival_time_secs: survival_time,
        terminal_sink_speed_uu_s: terminal_sink,
        total_hazard_basins_in_level: basins_count,
    }
}

/// Audits safety for inescapable traps, instant-gib lava, and zero friction
pub fn audit_fluid_basin_safety(
    config: &FluidBasinConfig,
    _peers: &[LevelBasinContextActorRecord],
) -> Vec<FluidBasinHazard> {
    let mut hazards = Vec::new();

    // 1. Inescapable Fluid Trap (> 48 uu depth without ladder/ramp)
    // In UT99, max step height is 25 uu, and jump height is ~55 uu.
    // An excavated depth of > 48 uu without a ramp or ladder traps players until death.
    if config.depth_uu > 48.0 && !config.has_escape_ladder_or_ramp {
        hazards.push(FluidBasinHazard {
            title: "Inescapable Submerged Death Trap".to_string(),
            description: format!(
                "Basin depth of {:.0} uu exceeds step-climb threshold (25 uu) and has no escape ladder or ramp! Trapped players will suffocate or burn.",
                config.depth_uu
            ),
            is_critical: true,
        });
    }

    // 2. Extreme Lethal DPS (> 100 DPS)
    if config.dps > 100.0 {
        hazards.push(FluidBasinHazard {
            title: "Catastrophic Instant-Kill Damage".to_string(),
            description: format!(
                "DPS of {:.0} kills a 100 HP player in under 1 second ({:.2}s). Ensure hazard is clearly telegraphed with warning signs or edge railings.",
                config.dps, 100.0 / config.dps
            ),
            is_critical: false,
        });
    }

    // 3. Negative or Zero Fluid Friction (Physics Glitch)
    if config.fluid_friction <= 0.1 {
        hazards.push(FluidBasinHazard {
            title: "Unstable Fluid Friction".to_string(),
            description: format!(
                "Fluid friction of {:.2} is abnormally low and may cause runaway acceleration or physics clipping.",
                config.fluid_friction
            ),
            is_critical: true,
        });
    }

    hazards
}

/// Generates a procedural CSG excavated basin trough brush with floor, rim walls, and planar fluid sheet
pub fn generate_fluid_basin_brush(config: &FluidBasinConfig) -> Brush {
    let mut brush = Brush::new("Fluid_Basin".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let w = config.width_uu;
    let l = config.length_uu;
    let d = config.depth_uu;
    let wall_t = 16.0;

    let half_w = w * 0.5;
    let half_l = l * 0.5;

    // 1. Basin Floor Slab (-Z from -d-wall_t to -d)
    let f_min_x = -half_w - wall_t;
    let f_max_x = half_w + wall_t;
    let f_min_y = -half_l - wall_t;
    let f_max_y = half_l + wall_t;
    add_box_to_brush(&mut brush, f_min_x, f_max_x, f_min_y, f_max_y, -d - wall_t, -d, "Indus1.Indus_B01");

    // 2. West Rim Wall (-X)
    add_box_to_brush(&mut brush, -half_w - wall_t, -half_w, f_min_y, f_max_y, -d, 0.0, "Indus1.Indus_B01");

    // 3. East Rim Wall (+X)
    add_box_to_brush(&mut brush, half_w, half_w + wall_t, f_min_y, f_max_y, -d, 0.0, "Indus1.Indus_B01");

    // 4. South Rim Wall (-Y)
    add_box_to_brush(&mut brush, -half_w, half_w, -half_l - wall_t, -half_l, -d, 0.0, "Indus1.Indus_B01");

    // 5. North Rim Wall (+Y)
    add_box_to_brush(&mut brush, -half_w, half_w, half_l, half_l + wall_t, -d, 0.0, "Indus1.Indus_B01");

    // 6. Planar Fluid Surface Sheet (Quad at Z = 0)
    let s0 = Vec3::new(-half_w, -half_l, 0.0);
    let s1 = Vec3::new(half_w, -half_l, 0.0);
    let s2 = Vec3::new(half_w, half_l, 0.0);
    let s3 = Vec3::new(-half_w, half_l, 0.0);

    if let Some(mut poly) = Polygon::new(vec![s0, s1, s2, s3]) {
        poly.texture = match config.archetype {
            FluidBasinArchetype::CorrosiveDeckSlimeVat => "Slime.Slime01".to_string(),
            FluidBasinArchetype::MoltenMagmaChasm => "FireEngine.Lava01".to_string(),
            FluidBasinArchetype::IndustrialCoolantReservoir => "Water.Water01".to_string(),
            FluidBasinArchetype::NaliSacredHealingSpring => "Water.CleanWater".to_string(),
            FluidBasinArchetype::CryogenicSubzeroBrine => "Ice.Brine01".to_string(),
        };
        poly.flags = crate::PF_NOT_SOLID | crate::PF_TWO_SIDED | crate::PF_TRANSLUCENT;
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
    fn test_fluid_basin_archetypes_and_defaults() {
        let slime = FluidBasinArchetype::CorrosiveDeckSlimeVat.to_config();
        assert_eq!(slime.dps, 40.0);
        assert_eq!(slime.fluid_friction, 2.5);
        assert!(slime.has_escape_ladder_or_ramp);

        let magma = FluidBasinArchetype::MoltenMagmaChasm.to_config();
        assert_eq!(magma.dps, 80.0);
        assert!(!magma.has_escape_ladder_or_ramp);
    }

    #[test]
    fn test_fluid_basin_metrics_evaluation() {
        let cfg = FluidBasinConfig::default(); // 256 x 384 x 64, dps 40
        let peers = vec![
            LevelBasinContextActorRecord {
                name: "WaterZone0".to_string(),
                class_name: "Botpack.WaterZone".to_string(),
                position: Vec3::new(500.0, 0.0, 0.0),
            }
        ];

        let metrics = evaluate_fluid_basin_metrics(&cfg, &peers);
        // vol = 256 * 384 * 64 / 1000 = 6291.456 k cu uu
        assert_eq!(metrics.fluid_volume_k_cu_uu, 256.0 * 384.0 * 64.0 / 1000.0);
        assert_eq!(metrics.player_survival_time_secs, 2.5); // 100 / 40 = 2.5s
        assert_eq!(metrics.total_hazard_basins_in_level, 2);
    }

    #[test]
    fn test_fluid_basin_safety_hazard_inescapable_trap() {
        let mut cfg = FluidBasinConfig::default();
        cfg.depth_uu = 64.0;
        cfg.has_escape_ladder_or_ramp = false;

        let hazards = audit_fluid_basin_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Inescapable Submerged Death Trap")));
    }

    #[test]
    fn test_generate_fluid_basin_brush() {
        let cfg = FluidBasinConfig::default();
        let brush = generate_fluid_basin_brush(&cfg);

        assert_eq!(brush.name, "Fluid_Basin");
        // 5 box slabs (6 polys each = 30) + 1 fluid surface sheet = 31 polys
        assert_eq!(brush.polys.len(), 31);
    }
}
