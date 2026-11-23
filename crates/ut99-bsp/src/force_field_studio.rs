// crates/ut99-bsp/src/force_field_studio.rs
//
// Interactive Force Field, Energy Barrier & Power Core Overload Studio
// Mathematical modeling of electromagnetic deflection barriers, plasma containment gates,
// permeability filtering (Shock combo pass-through), generator linkage, and procedural CSG geometry.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical archetypes of force fields and energy barriers in UT99 & Unreal 1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForceFieldArchetype {
    AssaultBulkheadForceField,
    PhobosPlasmaContainmentGate,
    MothershipEnergyShield,
    CybrosisSecurityRepulsor,
    ReactorCoreDeflectorGrid,
}

impl ForceFieldArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            ForceFieldArchetype::AssaultBulkheadForceField => {
                "Assault Bulkhead Force Field (AS-Overlord Military Gate)"
            }
            ForceFieldArchetype::PhobosPlasmaContainmentGate => {
                "Phobos Plasma Containment Gate (DM-Phobos Decompression Seal)"
            }
            ForceFieldArchetype::MothershipEnergyShield => {
                "Mothership Energy Shield (Skaarj Hexagonal Deflector)"
            }
            ForceFieldArchetype::CybrosisSecurityRepulsor => {
                "Cybrosis Security Repulsor (DM-Cybrosis EMP Security Door)"
            }
            ForceFieldArchetype::ReactorCoreDeflectorGrid => {
                "Reactor Core Deflector Grid (CTF-Niven High-Voltage Perimeter)"
            }
        }
    }

    pub fn to_config(&self) -> ForceFieldConfig {
        match self {
            ForceFieldArchetype::AssaultBulkheadForceField => ForceFieldConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_deg: 0.0,
                width_uu: 256.0,
                height_uu: 192.0,
                thickness_uu: 8.0,
                pylon_radius_uu: 24.0,
                pylon_height_uu: 208.0,
                voltage_kv: 75.0,
                permeability: ForceFieldPermeability::BlocksAll,
                color_palette: ForceFieldColorPalette::CyanAzure,
                pulse_frequency_hz: 2.0,
                has_emitter_pylons: true,
                generator_linked: true,
                generator_tag: "ShieldGen_Alpha".to_string(),
                generator_health: 800.0,
                collapse_delay_sec: 1.5,
                tag: "AssaultForceField0".to_string(),
                event: "ForceFieldBreached".to_string(),
            },
            ForceFieldArchetype::PhobosPlasmaContainmentGate => ForceFieldConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_deg: 0.0,
                width_uu: 200.0,
                height_uu: 160.0,
                thickness_uu: 6.0,
                pylon_radius_uu: 20.0,
                pylon_height_uu: 176.0,
                voltage_kv: 45.0,
                permeability: ForceFieldPermeability::BlocksPlayersOnly,
                color_palette: ForceFieldColorPalette::VoidPurple,
                pulse_frequency_hz: 3.5,
                has_emitter_pylons: true,
                generator_linked: true,
                generator_tag: "PhobosSubstation_01".to_string(),
                generator_health: 400.0,
                collapse_delay_sec: 0.8,
                tag: "PhobosPlasmaGate0".to_string(),
                event: "PlasmaGateCollapsed".to_string(),
            },
            ForceFieldArchetype::MothershipEnergyShield => ForceFieldConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_deg: 0.0,
                width_uu: 320.0,
                height_uu: 240.0,
                thickness_uu: 12.0,
                pylon_radius_uu: 32.0,
                pylon_height_uu: 260.0,
                voltage_kv: 120.0,
                permeability: ForceFieldPermeability::BlocksProjectilesOnly,
                color_palette: ForceFieldColorPalette::SkaarjEmerald,
                pulse_frequency_hz: 1.5,
                has_emitter_pylons: true,
                generator_linked: true,
                generator_tag: "SkaarjPowerCell0".to_string(),
                generator_health: 1200.0,
                collapse_delay_sec: 2.0,
                tag: "SkaarjShield0".to_string(),
                event: "ShieldEmitterDestroyed".to_string(),
            },
            ForceFieldArchetype::CybrosisSecurityRepulsor => ForceFieldConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_deg: 0.0,
                width_uu: 192.0,
                height_uu: 144.0,
                thickness_uu: 6.0,
                pylon_radius_uu: 18.0,
                pylon_height_uu: 156.0,
                voltage_kv: 60.0,
                permeability: ForceFieldPermeability::ShockComboPassThrough,
                color_palette: ForceFieldColorPalette::GoldAmber,
                pulse_frequency_hz: 4.0,
                has_emitter_pylons: true,
                generator_linked: true,
                generator_tag: "SecurityTerminal_Main".to_string(),
                generator_health: 300.0,
                collapse_delay_sec: 1.0,
                tag: "CybrosisRepulsor0".to_string(),
                event: "RepulsorDisabled".to_string(),
            },
            ForceFieldArchetype::ReactorCoreDeflectorGrid => ForceFieldConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw_deg: 0.0,
                width_uu: 384.0,
                height_uu: 256.0,
                thickness_uu: 16.0,
                pylon_radius_uu: 36.0,
                pylon_height_uu: 280.0,
                voltage_kv: 150.0,
                permeability: ForceFieldPermeability::BlocksAll,
                color_palette: ForceFieldColorPalette::CrimsonAlert,
                pulse_frequency_hz: 5.0,
                has_emitter_pylons: true,
                generator_linked: false, // Critical core barrier powered directly by grid
                generator_tag: "".to_string(),
                generator_health: 0.0,
                collapse_delay_sec: 0.0,
                tag: "ReactorCoreGrid0".to_string(),
                event: "CoreContainmentOverload".to_string(),
            },
        }
    }
}

/// Force field permeability filtering modes for competitive gameplay
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForceFieldPermeability {
    BlocksAll,
    BlocksPlayersOnly,
    BlocksProjectilesOnly,
    ShockComboPassThrough,
}

impl ForceFieldPermeability {
    pub fn display_name(&self) -> &'static str {
        match self {
            ForceFieldPermeability::BlocksAll => "Blocks All (Pawns & Projectiles)",
            ForceFieldPermeability::BlocksPlayersOnly => "Blocks Players Only (Weapons Pass Through)",
            ForceFieldPermeability::BlocksProjectilesOnly => "Blocks Projectiles Only (Pawns Pass Through)",
            ForceFieldPermeability::ShockComboPassThrough => "Shock Combo & Translocator Pass-Through",
        }
    }
}

/// Luminescent color palette for force field visual rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForceFieldColorPalette {
    CyanAzure,
    SkaarjEmerald,
    CrimsonAlert,
    VoidPurple,
    GoldAmber,
}

impl ForceFieldColorPalette {
    pub fn display_name(&self) -> &'static str {
        match self {
            ForceFieldColorPalette::CyanAzure => "Cyan Azure (Standard Energy)",
            ForceFieldColorPalette::SkaarjEmerald => "Skaarj Emerald (Alien Bio-Plasma)",
            ForceFieldColorPalette::CrimsonAlert => "Crimson Alert (High-Voltage Meltdown)",
            ForceFieldColorPalette::VoidPurple => "Void Purple (Quantum Containment)",
            ForceFieldColorPalette::GoldAmber => "Gold Amber (Liandri Industrial Grid)",
        }
    }

    pub fn to_rgba(&self) -> [f32; 4] {
        match self {
            ForceFieldColorPalette::CyanAzure => [0.15, 0.75, 1.0, 0.75],
            ForceFieldColorPalette::SkaarjEmerald => [0.15, 1.0, 0.35, 0.80],
            ForceFieldColorPalette::CrimsonAlert => [1.0, 0.15, 0.15, 0.85],
            ForceFieldColorPalette::VoidPurple => [0.75, 0.20, 1.0, 0.75],
            ForceFieldColorPalette::GoldAmber => [1.0, 0.80, 0.15, 0.80],
        }
    }
}

/// Comprehensive configuration for a Force Field and Energy Barrier setup
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForceFieldConfig {
    pub archetype: ForceFieldArchetype,
    pub position: Vec3,
    pub yaw_deg: f32,
    pub width_uu: f32,
    pub height_uu: f32,
    pub thickness_uu: f32,
    pub pylon_radius_uu: f32,
    pub pylon_height_uu: f32,
    pub voltage_kv: f32,
    pub permeability: ForceFieldPermeability,
    pub color_palette: ForceFieldColorPalette,
    pub pulse_frequency_hz: f32,
    pub has_emitter_pylons: bool,
    pub generator_linked: bool,
    pub generator_tag: String,
    pub generator_health: f32,
    pub collapse_delay_sec: f32,
    pub tag: String,
    pub event: String,
}

impl Default for ForceFieldConfig {
    fn default() -> Self {
        ForceFieldArchetype::AssaultBulkheadForceField.to_config()
    }
}

/// Telemetry metrics for physical and electromagnetic barrier properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForceFieldMetrics {
    pub surface_area_uu2: f32,
    pub absorbed_energy_kj: f32,
    pub power_consumption_kw: f32,
    pub deflection_efficiency_pct: f32,
    pub pylon_clearance_width_uu: f32,
    pub is_safe_for_assault: bool,
    pub total_fields_in_level: usize,
}

/// Context actor record for safety auditing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelForceFieldContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Compute telemetry and physical metrics for the force field setup
pub fn evaluate_force_field_metrics(
    config: &ForceFieldConfig,
    peers: &[LevelForceFieldContextActorRecord],
) -> ForceFieldMetrics {
    let area = config.width_uu * config.height_uu;
    let power_kw = config.voltage_kv * (area / 1000.0) * 0.08;
    let energy_kj = power_kw * 12.5;

    let deflection_pct = match config.permeability {
        ForceFieldPermeability::BlocksAll => 100.0,
        ForceFieldPermeability::BlocksPlayersOnly => 80.0,
        ForceFieldPermeability::BlocksProjectilesOnly => 85.0,
        ForceFieldPermeability::ShockComboPassThrough => 70.0,
    };

    let clearance_width = if config.has_emitter_pylons {
        (config.width_uu - (config.pylon_radius_uu * 2.0)).max(0.0)
    } else {
        config.width_uu
    };

    let mut count = 1;
    for p in peers {
        let cls = p.class_name.to_lowercase();
        if cls.contains("forcefield") || cls.contains("barrier") || cls.contains("shield") {
            count += 1;
        }
    }

    let is_safe = config.generator_linked || !config.generator_tag.is_empty() || clearance_width >= 96.0;

    ForceFieldMetrics {
        surface_area_uu2: area,
        absorbed_energy_kj: energy_kj,
        power_consumption_kw: power_kw,
        deflection_efficiency_pct: deflection_pct,
        pylon_clearance_width_uu: clearance_width,
        is_safe_for_assault: is_safe,
        total_fields_in_level: count,
    }
}

/// Potential safety hazards and gameplay balancing alerts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForceFieldHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Audit safety and gameplay hazards for the force field setup
pub fn audit_force_field_safety(
    config: &ForceFieldConfig,
    peers: &[LevelForceFieldContextActorRecord],
) -> Vec<ForceFieldHazard> {
    let mut hazards = Vec::new();
    let metrics = evaluate_force_field_metrics(config, peers);

    // 1. Orphan unlinked force field hazard: blocks passage permanently with no switch/generator
    if config.permeability == ForceFieldPermeability::BlocksAll && !config.generator_linked {
        hazards.push(ForceFieldHazard {
            title: "Permanent Impassable Barrier Hazard".to_string(),
            description: format!(
                "Force Field '{}' blocks all players and weapons but has no linked power generator or control switch. Players cannot bypass it.",
                config.tag
            ),
            is_critical: true,
        });
    }

    // 2. Narrow corridor clearance hazard: pylons pinch passage below standard pawn width (48 uu)
    if config.has_emitter_pylons && metrics.pylon_clearance_width_uu < 48.0 {
        hazards.push(ForceFieldHazard {
            title: "Corridor Chokepoint Encroachment".to_string(),
            description: format!(
                "Emitter pylons leave only {:.1} uu of opening width, which is below the minimum pawn traversal width (48 uu).",
                metrics.pylon_clearance_width_uu
            ),
            is_critical: true,
        });
    }

    // 3. Excessive generator resilience: generator has over 1500 HP, stalling assault pacing
    if config.generator_linked && config.generator_health > 1500.0 {
        hazards.push(ForceFieldHazard {
            title: "Over-Reinforced Power Generator".to_string(),
            description: format!(
                "Generator '{}' has {:.0} HP, which requires excessive ammunition to breach and may stall match flow.",
                config.generator_tag, config.generator_health
            ),
            is_critical: false,
        });
    }

    // 4. Excessive collapse delay: barrier takes longer than 4 seconds to collapse after generator destruction
    if config.generator_linked && config.collapse_delay_sec > 4.0 {
        hazards.push(ForceFieldHazard {
            title: "Excessive Shield Collapse Latency".to_string(),
            description: format!(
                "Collapse delay is {:.1}s. Attackers may mistakenly believe the generator destruction failed.",
                config.collapse_delay_sec
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Procedural CSG brush generator for the Force Field and Emitter Pylons
pub fn generate_force_field_brushes(config: &ForceFieldConfig) -> (Brush, Vec<Brush>) {
    let hw = config.width_uu * 0.5;
    let _hh = config.height_uu * 0.5;
    let ht = config.thickness_uu * 0.5;
    let p = config.position;

    // Rotation around Z axis (yaw)
    let rad = config.yaw_deg.to_radians();
    let cos_y = rad.cos();
    let sin_y = rad.sin();
    let rot = |v: Vec3| -> Vec3 {
        Vec3::new(
            v.x * cos_y - v.y * sin_y,
            v.x * sin_y + v.y * cos_y,
            v.z,
        )
    };

    // 1. Central Force Field Slab Brush (Translucent, Unlit, Two-Sided)
    let mut field_brush = Brush::new("ForceField_Sheet", CsgOperation::Add);
    field_brush.location = p;

    let f_p0 = rot(Vec3::new(-hw, -ht, 0.0));
    let f_p1 = rot(Vec3::new(hw, -ht, 0.0));
    let f_p2 = rot(Vec3::new(hw, ht, 0.0));
    let f_p3 = rot(Vec3::new(-hw, ht, 0.0));
    let f_p4 = rot(Vec3::new(-hw, -ht, config.height_uu));
    let f_p5 = rot(Vec3::new(hw, -ht, config.height_uu));
    let f_p6 = rot(Vec3::new(hw, ht, config.height_uu));
    let f_p7 = rot(Vec3::new(-hw, ht, config.height_uu));

    let tex_field = "GenFX.Energy.ForceField01";
    let field_flags = crate::PF_TRANSLUCENT | crate::PF_UNLIT | crate::PF_TWO_SIDED | crate::PF_NOT_SOLID;

    // Bottom
    if let Some(mut poly) = Polygon::new(vec![f_p0, f_p3, f_p2, f_p1]) {
        poly.texture = tex_field.to_string();
        poly.flags = field_flags;
        field_brush.polys.push(poly);
    }
    // Top
    if let Some(mut poly) = Polygon::new(vec![f_p4, f_p5, f_p6, f_p7]) {
        poly.texture = tex_field.to_string();
        poly.flags = field_flags;
        field_brush.polys.push(poly);
    }
    // Front (-Y)
    if let Some(mut poly) = Polygon::new(vec![f_p0, f_p1, f_p5, f_p4]) {
        poly.texture = tex_field.to_string();
        poly.flags = field_flags;
        field_brush.polys.push(poly);
    }
    // Right (+X)
    if let Some(mut poly) = Polygon::new(vec![f_p1, f_p2, f_p6, f_p5]) {
        poly.texture = tex_field.to_string();
        poly.flags = field_flags;
        field_brush.polys.push(poly);
    }
    // Back (+Y)
    if let Some(mut poly) = Polygon::new(vec![f_p2, f_p3, f_p7, f_p6]) {
        poly.texture = tex_field.to_string();
        poly.flags = field_flags;
        field_brush.polys.push(poly);
    }
    // Left (-X)
    if let Some(mut poly) = Polygon::new(vec![f_p3, f_p0, f_p4, f_p7]) {
        poly.texture = tex_field.to_string();
        poly.flags = field_flags;
        field_brush.polys.push(poly);
    }

    // 2. Left & Right Emitter Pylons (Octagonal semi-solid pillars)
    let mut pylon_brushes = Vec::new();
    if config.has_emitter_pylons {
        let pr = config.pylon_radius_uu;
        let ph = config.pylon_height_uu;
        let sides = 8;
        let tex_pylon = "Indus1.Metal.PylonSteel";

        for side_sign in &[-1.0f32, 1.0f32] {
            let pylon_center_local = Vec3::new(*side_sign * hw, 0.0, 0.0);
            let pylon_world_pos = p + rot(pylon_center_local);

            let mut pylon_brush = Brush::new("ForceField_Pylon", CsgOperation::Add);
            pylon_brush.location = pylon_world_pos;

            let mut base_pts = Vec::new();
            let mut top_pts = Vec::new();

            for i in 0..sides {
                let angle = (i as f32) * (std::f32::consts::TAU / sides as f32);
                let offset = rot(Vec3::new(angle.cos() * pr, angle.sin() * pr, 0.0));
                base_pts.push(offset);
                top_pts.push(offset + Vec3::new(0.0, 0.0, ph));
            }

            // Bottom face
            if let Some(mut poly) = Polygon::new(base_pts.clone()) {
                poly.texture = tex_pylon.to_string();
                poly.flags = crate::PF_SEMISOLID;
                pylon_brush.polys.push(poly);
            }

            // Side faces
            for i in 0..sides {
                let next = (i + 1) % sides;
                if let Some(mut poly) = Polygon::new(vec![
                    base_pts[i],
                    base_pts[next],
                    top_pts[next],
                    top_pts[i],
                ]) {
                    poly.texture = tex_pylon.to_string();
                    poly.flags = crate::PF_SEMISOLID;
                    pylon_brush.polys.push(poly);
                }
            }

            // Top face
            let mut top_reversed = top_pts.clone();
            top_reversed.reverse();
            if let Some(mut poly) = Polygon::new(top_reversed) {
                poly.texture = tex_pylon.to_string();
                poly.flags = crate::PF_SEMISOLID;
                pylon_brush.polys.push(poly);
            }

            pylon_brushes.push(pylon_brush);
        }
    }

    (field_brush, pylon_brushes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_force_field_archetypes_and_defaults() {
        let archetypes = [
            ForceFieldArchetype::AssaultBulkheadForceField,
            ForceFieldArchetype::PhobosPlasmaContainmentGate,
            ForceFieldArchetype::MothershipEnergyShield,
            ForceFieldArchetype::CybrosisSecurityRepulsor,
            ForceFieldArchetype::ReactorCoreDeflectorGrid,
        ];

        for a in &archetypes {
            let cfg = a.to_config();
            assert_eq!(cfg.archetype, *a);
            assert!(cfg.width_uu > 0.0);
            assert!(cfg.height_uu > 0.0);
            assert!(cfg.voltage_kv > 0.0);
            let metrics = evaluate_force_field_metrics(&cfg, &[]);
            assert!(metrics.surface_area_uu2 > 0.0);
            assert!(metrics.power_consumption_kw > 0.0);
            assert!(metrics.absorbed_energy_kj > 0.0);
            assert!(metrics.deflection_efficiency_pct > 0.0);
            assert_eq!(metrics.total_fields_in_level, 1);
        }
    }

    #[test]
    fn test_force_field_brush_generation() {
        let cfg = ForceFieldArchetype::AssaultBulkheadForceField.to_config();
        let (field_brush, pylons) = generate_force_field_brushes(&cfg);

        assert_eq!(field_brush.polys.len(), 6);
        assert_eq!(pylons.len(), 2);
        for pylon in &pylons {
            // 8 sides + top + bottom = 10 polygons
            assert_eq!(pylon.polys.len(), 10);
        }
    }

    #[test]
    fn test_force_field_safety_auditing() {
        let mut cfg = ForceFieldArchetype::ReactorCoreDeflectorGrid.to_config();
        cfg.permeability = ForceFieldPermeability::BlocksAll;
        cfg.generator_linked = false; // Permanent impassable barrier

        let hazards = audit_force_field_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.title == "Permanent Impassable Barrier Hazard" && h.is_critical));
    }
}
