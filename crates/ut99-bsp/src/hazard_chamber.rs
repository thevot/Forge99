// crates/ut99-bsp/src/hazard_chamber.rs
//
// Interactive Environmental Hazard Chamber, Pressure Airlock & Gas Decompression Studio
//
// Mathematical, geometric, and gameplay model for hermetic hazard chambers, airlocks,
// sliding blast doors, pressure valves, observation windows, damage volumes, and alarm systems
// inspired by canonical maps such as DM-Pressure, DM-Shrapnel, AS-Mazon, and AS-Frigate.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical Unreal Tournament & Unreal 1 environmental hazard chamber archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HazardChamberArchetype {
    HighVacuumDecompressionChamber,
    BiohazardToxicGasIncinerator,
    SubmergedHydrostaticFloodLock,
    CryogenicFreezeVault,
    HighVoltageArcFurnace,
}

impl HazardChamberArchetype {
    pub fn all() -> &'static [HazardChamberArchetype] {
        &[
            HazardChamberArchetype::HighVacuumDecompressionChamber,
            HazardChamberArchetype::BiohazardToxicGasIncinerator,
            HazardChamberArchetype::SubmergedHydrostaticFloodLock,
            HazardChamberArchetype::CryogenicFreezeVault,
            HazardChamberArchetype::HighVoltageArcFurnace,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            HazardChamberArchetype::HighVacuumDecompressionChamber => "High-Vacuum Decompression Chamber (DM-Pressure)",
            HazardChamberArchetype::BiohazardToxicGasIncinerator => "Biohazard Toxic Gas Incinerator (DM-Shrapnel)",
            HazardChamberArchetype::SubmergedHydrostaticFloodLock => "Submerged Hydrostatic Flood Lock (AS-Frigate)",
            HazardChamberArchetype::CryogenicFreezeVault => "Cryogenic Nitrogen Freeze Vault (Cold Trap)",
            HazardChamberArchetype::HighVoltageArcFurnace => "High-Voltage Plasma Arc Furnace (Shock Trap)",
        }
    }

    pub fn default_config(&self) -> HazardChamberConfig {
        match self {
            HazardChamberArchetype::HighVacuumDecompressionChamber => HazardChamberConfig {
                archetype: *self,
                position: Vec3::ZERO,
                chamber_width: 512.0,
                chamber_length: 512.0,
                chamber_height: 256.0,
                wall_thickness: 16.0,
                door_style: ChamberDoorStyle::SlidingBiParting,
                door_width: 128.0,
                door_height: 192.0,
                observation_window: true,
                window_width: 160.0,
                window_height: 96.0,
                console_offset: Vec3::new(320.0, 0.0, 0.0),
                vent_cycle_duration_secs: 6.0,
                door_close_delay_secs: 1.0,
                damage_per_sec: 200.0,
                damage_type: HazardDamageType::Asphyxiated,
                b_kill_zone: true,
                alarm_sound: "Alarm3".to_string(),
                strobe_color: [1.0, 0.15, 0.15],
                tag: "PressureChamber01".to_string(),
                event_name: "VentCycle_Trigger".to_string(),
            },
            HazardChamberArchetype::BiohazardToxicGasIncinerator => HazardChamberConfig {
                archetype: *self,
                position: Vec3::ZERO,
                chamber_width: 448.0,
                chamber_length: 448.0,
                chamber_height: 224.0,
                wall_thickness: 16.0,
                door_style: ChamberDoorStyle::VerticalBlastDrop,
                door_width: 112.0,
                door_height: 176.0,
                observation_window: true,
                window_width: 140.0,
                window_height: 80.0,
                console_offset: Vec3::new(280.0, 0.0, 0.0),
                vent_cycle_duration_secs: 8.0,
                door_close_delay_secs: 1.5,
                damage_per_sec: 75.0,
                damage_type: HazardDamageType::Corroded,
                b_kill_zone: false,
                alarm_sound: "Steam1".to_string(),
                strobe_color: [0.35, 1.0, 0.20],
                tag: "ToxicGasCell01".to_string(),
                event_name: "GasVent_Trigger".to_string(),
            },
            HazardChamberArchetype::SubmergedHydrostaticFloodLock => HazardChamberConfig {
                archetype: *self,
                position: Vec3::ZERO,
                chamber_width: 576.0,
                chamber_length: 576.0,
                chamber_height: 288.0,
                wall_thickness: 24.0,
                door_style: ChamberDoorStyle::SlidingBiParting,
                door_width: 144.0,
                door_height: 208.0,
                observation_window: true,
                window_width: 180.0,
                window_height: 112.0,
                console_offset: Vec3::new(360.0, 0.0, 0.0),
                vent_cycle_duration_secs: 10.0,
                door_close_delay_secs: 2.0,
                damage_per_sec: 40.0,
                damage_type: HazardDamageType::Drowned,
                b_kill_zone: false,
                alarm_sound: "waterfall1".to_string(),
                strobe_color: [0.20, 0.70, 1.0],
                tag: "FloodLock01".to_string(),
                event_name: "Flood_Trigger".to_string(),
            },
            HazardChamberArchetype::CryogenicFreezeVault => HazardChamberConfig {
                archetype: *self,
                position: Vec3::ZERO,
                chamber_width: 384.0,
                chamber_length: 384.0,
                chamber_height: 192.0,
                wall_thickness: 16.0,
                door_style: ChamberDoorStyle::SlidingBiParting,
                door_width: 96.0,
                door_height: 160.0,
                observation_window: false,
                window_width: 0.0,
                window_height: 0.0,
                console_offset: Vec3::new(240.0, 0.0, 0.0),
                vent_cycle_duration_secs: 5.0,
                door_close_delay_secs: 0.8,
                damage_per_sec: 90.0,
                damage_type: HazardDamageType::Frozen,
                b_kill_zone: false,
                alarm_sound: "Hiss1".to_string(),
                strobe_color: [0.40, 0.90, 1.0],
                tag: "CryoVault01".to_string(),
                event_name: "CryoFreeze_Trigger".to_string(),
            },
            HazardChamberArchetype::HighVoltageArcFurnace => HazardChamberConfig {
                archetype: *self,
                position: Vec3::ZERO,
                chamber_width: 512.0,
                chamber_length: 512.0,
                chamber_height: 256.0,
                wall_thickness: 20.0,
                door_style: ChamberDoorStyle::VerticalBlastDrop,
                door_width: 128.0,
                door_height: 192.0,
                observation_window: true,
                window_width: 160.0,
                window_height: 96.0,
                console_offset: Vec3::new(320.0, 0.0, 0.0),
                vent_cycle_duration_secs: 4.0,
                door_close_delay_secs: 0.5,
                damage_per_sec: 250.0,
                damage_type: HazardDamageType::Electrocuted,
                b_kill_zone: true,
                alarm_sound: "Zap1".to_string(),
                strobe_color: [0.30, 0.60, 1.0],
                tag: "ArcFurnace01".to_string(),
                event_name: "ArcDischarge_Trigger".to_string(),
            },
        }
    }
}

/// Movement kinematics for chamber entrance blast doors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChamberDoorStyle {
    SlidingBiParting,
    VerticalBlastDrop,
}

impl ChamberDoorStyle {
    pub fn all() -> &'static [ChamberDoorStyle] {
        &[
            ChamberDoorStyle::SlidingBiParting,
            ChamberDoorStyle::VerticalBlastDrop,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ChamberDoorStyle::SlidingBiParting => "Sliding Bi-Parting (Left/Right)",
            ChamberDoorStyle::VerticalBlastDrop => "Vertical Blast Drop (Top/Down)",
        }
    }
}

/// Unreal Engine 1 damage types for environmental hazard chambers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HazardDamageType {
    Asphyxiated,
    Corroded,
    Burned,
    Electrocuted,
    Drowned,
    Frozen,
}

impl HazardDamageType {
    pub fn all() -> &'static [HazardDamageType] {
        &[
            HazardDamageType::Asphyxiated,
            HazardDamageType::Corroded,
            HazardDamageType::Burned,
            HazardDamageType::Electrocuted,
            HazardDamageType::Drowned,
            HazardDamageType::Frozen,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            HazardDamageType::Asphyxiated => "Asphyxiated (Vacuum Decompression)",
            HazardDamageType::Corroded => "Corroded (Bio-Sludge / Acid Gas)",
            HazardDamageType::Burned => "Burned (Thermal Incineration)",
            HazardDamageType::Electrocuted => "Electrocuted (High-Voltage Discharge)",
            HazardDamageType::Drowned => "Drowned (Hydrostatic Water Flood)",
            HazardDamageType::Frozen => "Frozen (Cryogenic Liquid Nitrogen)",
        }
    }
}

/// Complete configuration for Hazard Chamber Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardChamberConfig {
    pub archetype: HazardChamberArchetype,
    pub position: Vec3,
    pub chamber_width: f32,
    pub chamber_length: f32,
    pub chamber_height: f32,
    pub wall_thickness: f32,
    pub door_style: ChamberDoorStyle,
    pub door_width: f32,
    pub door_height: f32,
    pub observation_window: bool,
    pub window_width: f32,
    pub window_height: f32,
    pub console_offset: Vec3,
    pub vent_cycle_duration_secs: f32,
    pub door_close_delay_secs: f32,
    pub damage_per_sec: f32,
    pub damage_type: HazardDamageType,
    pub b_kill_zone: bool,
    pub alarm_sound: String,
    pub strobe_color: [f32; 3],
    pub tag: String,
    pub event_name: String,
}

impl Default for HazardChamberConfig {
    fn default() -> Self {
        HazardChamberArchetype::HighVacuumDecompressionChamber.default_config()
    }
}

/// Analytical metrics for hazard chamber operation
#[derive(Debug, Clone, PartialEq)]
pub struct HazardChamberMetrics {
    pub chamber_volume_uu3: f32,
    pub total_cycle_time_secs: f32,
    pub time_to_kill_secs: f32,
    pub is_instant_death: bool,
    pub effective_air_evacuation_speed_uu_s: f32,
}

/// Generated procedural brush geometry for the hazard chamber setup
#[derive(Debug, Clone, PartialEq)]
pub struct HazardChamberBrushes {
    pub chamber_brush: Brush,
    pub door_left_brush: Brush,
    pub door_right_brush: Brush,
    pub window_glass_brush: Option<Brush>,
    pub console_brush: Brush,
}

/// Bot AI navigation waypoints for trap baiting and button activation
#[derive(Debug, Clone, PartialEq)]
pub struct HazardChamberBotWaypoints {
    pub bait_inside: Vec3,
    pub console_activator: Vec3,
    pub safe_outside_door: Vec3,
}

/// Diagnostic safety hazard
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardChamberHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Calculates analytical metrics for the chamber
pub fn calculate_chamber_metrics(config: &HazardChamberConfig) -> HazardChamberMetrics {
    let vol = config.chamber_width * config.chamber_length * config.chamber_height;
    let total_time = config.door_close_delay_secs + config.vent_cycle_duration_secs + 1.5; // +1.5s reopen
    let ttk = if config.b_kill_zone {
        0.05
    } else if config.damage_per_sec > 0.0 {
        100.0 / config.damage_per_sec
    } else {
        999.0
    };

    let evac_speed = (vol / (config.vent_cycle_duration_secs.max(1.0) * 1000.0)).clamp(200.0, 2000.0);

    HazardChamberMetrics {
        chamber_volume_uu3: vol,
        total_cycle_time_secs: total_time,
        time_to_kill_secs: ttk,
        is_instant_death: config.b_kill_zone,
        effective_air_evacuation_speed_uu_s: evac_speed,
    }
}

/// Generates watertight procedural brush meshes for all chamber components
pub fn generate_hazard_chamber_brushes(config: &HazardChamberConfig) -> HazardChamberBrushes {
    let p = config.position;
    let hw = config.chamber_width * 0.5;
    let hl = config.chamber_length * 0.5;
    let hh = config.chamber_height * 0.5;

    // 1. Outer Chamber Shell Brush (Additive / SemiSolid Room)
    let mut chamber_b = Brush::new("HazardChamber_Room", CsgOperation::Add);
    let corners = [
        p + Vec3::new(-hw, -hl, -hh),
        p + Vec3::new(hw, -hl, -hh),
        p + Vec3::new(hw, hl, -hh),
        p + Vec3::new(-hw, hl, -hh),
        p + Vec3::new(-hw, -hl, hh),
        p + Vec3::new(hw, -hl, hh),
        p + Vec3::new(hw, hl, hh),
        p + Vec3::new(-hw, hl, hh),
    ];
    let faces = [
        [0, 3, 2, 1], // Floor (-Z)
        [4, 5, 6, 7], // Ceiling (+Z)
        [0, 1, 5, 4], // Front Wall (-Y)
        [2, 3, 7, 6], // Back Wall (+Y)
        [3, 0, 4, 7], // Left Wall (-X)
        [1, 2, 6, 5], // Right Wall (+X)
    ];
    for f in faces {
        let pts = vec![corners[f[0]], corners[f[1]], corners[f[2]], corners[f[3]]];
        if let Some(mut poly) = Polygon::new(pts) {
            poly.texture = "Indus.Metal.RustyIron01".to_string();
            poly.flags |= 0x00200000; // PF_SEMISOLID
            chamber_b.polys.push(poly);
        }
    }

    // 2. Door Left Brush (Mover)
    let mut door_l_b = Brush::new("ChamberDoor_Left", CsgOperation::Add);
    let dw_half = config.door_width * 0.25;
    let dh_half = config.door_height * 0.5;
    let dt_half = 6.0;
    let door_l_pos = p + Vec3::new(-dw_half, -hl, -hh + dh_half);
    let dl_corners = [
        door_l_pos + Vec3::new(-dw_half, -dt_half, -dh_half),
        door_l_pos + Vec3::new(dw_half, -dt_half, -dh_half),
        door_l_pos + Vec3::new(dw_half, dt_half, -dh_half),
        door_l_pos + Vec3::new(-dw_half, dt_half, -dh_half),
        door_l_pos + Vec3::new(-dw_half, -dt_half, dh_half),
        door_l_pos + Vec3::new(dw_half, -dt_half, dh_half),
        door_l_pos + Vec3::new(dw_half, dt_half, dh_half),
        door_l_pos + Vec3::new(-dw_half, dt_half, dh_half),
    ];
    for f in faces {
        let pts = vec![dl_corners[f[0]], dl_corners[f[1]], dl_corners[f[2]], dl_corners[f[3]]];
        if let Some(mut poly) = Polygon::new(pts) {
            poly.texture = "Indus.Metal.YellowHazardStripes01".to_string();
            poly.flags |= 0x00200000; // PF_SEMISOLID
            door_l_b.polys.push(poly);
        }
    }

    // 3. Door Right Brush (Mover)
    let mut door_r_b = Brush::new("ChamberDoor_Right", CsgOperation::Add);
    let door_r_pos = p + Vec3::new(dw_half, -hl, -hh + dh_half);
    let dr_corners = [
        door_r_pos + Vec3::new(-dw_half, -dt_half, -dh_half),
        door_r_pos + Vec3::new(dw_half, -dt_half, -dh_half),
        door_r_pos + Vec3::new(dw_half, dt_half, -dh_half),
        door_r_pos + Vec3::new(-dw_half, dt_half, -dh_half),
        door_r_pos + Vec3::new(-dw_half, -dt_half, dh_half),
        door_r_pos + Vec3::new(dw_half, -dt_half, dh_half),
        door_r_pos + Vec3::new(dw_half, dt_half, dh_half),
        door_r_pos + Vec3::new(-dw_half, dt_half, dh_half),
    ];
    for f in faces {
        let pts = vec![dr_corners[f[0]], dr_corners[f[1]], dr_corners[f[2]], dr_corners[f[3]]];
        if let Some(mut poly) = Polygon::new(pts) {
            poly.texture = "Indus.Metal.YellowHazardStripes01".to_string();
            poly.flags |= 0x00200000; // PF_SEMISOLID
            door_r_b.polys.push(poly);
        }
    }

    // 4. Observation Window Glass Brush (Sheet)
    let window_glass_b = if config.observation_window && config.window_width > 16.0 {
        let mut win_b = Brush::new("Chamber_Observation_Glass", CsgOperation::Add);
        let win_hw = config.window_width * 0.5;
        let win_hh = config.window_height * 0.5;
        let win_pos = p + Vec3::new(hw, 0.0, 0.0);
        let win_pts = vec![
            win_pos + Vec3::new(0.0, -win_hw, -win_hh),
            win_pos + Vec3::new(0.0, win_hw, -win_hh),
            win_pos + Vec3::new(0.0, win_hw, win_hh),
            win_pos + Vec3::new(0.0, -win_hw, win_hh),
        ];
        if let Some(mut poly) = Polygon::new(win_pts) {
            poly.texture = "Indus.Glass.ReinforcedWindow01".to_string();
            // PF_TRANSLUCENT (0x00000004) | PF_NOT_SOLID (0x00000008) | PF_TWO_SIDED (0x00000100)
            poly.flags |= 0x0000010C;
            win_b.polys.push(poly);
        }
        Some(win_b)
    } else {
        None
    };

    // 5. Control Console Pedestal Brush
    let mut console_b = Brush::new("Chamber_Control_Console", CsgOperation::Add);
    let con_pos = p + config.console_offset + Vec3::new(0.0, 0.0, -hh + 32.0);
    let con_hw = 24.0;
    let con_hl = 24.0;
    let con_hh = 32.0;
    let con_corners = [
        con_pos + Vec3::new(-con_hw, -con_hl, -con_hh),
        con_pos + Vec3::new(con_hw, -con_hl, -con_hh),
        con_pos + Vec3::new(con_hw, con_hl, -con_hh),
        con_pos + Vec3::new(-con_hw, con_hl, -con_hh),
        con_pos + Vec3::new(-con_hw, -con_hl, con_hh),
        con_pos + Vec3::new(con_hw, -con_hl, con_hh),
        con_pos + Vec3::new(con_hw, con_hl, con_hh),
        con_pos + Vec3::new(-con_hw, con_hl, con_hh),
    ];
    for f in faces {
        let pts = vec![con_corners[f[0]], con_corners[f[1]], con_corners[f[2]], con_corners[f[3]]];
        if let Some(mut poly) = Polygon::new(pts) {
            poly.texture = "Indus.Control.ConsoleSlanted01".to_string();
            poly.flags |= 0x00200000; // PF_SEMISOLID
            console_b.polys.push(poly);
        }
    }

    HazardChamberBrushes {
        chamber_brush: chamber_b,
        door_left_brush: door_l_b,
        door_right_brush: door_r_b,
        window_glass_brush: window_glass_b,
        console_brush: console_b,
    }
}

/// Generates bot navigation waypoints for chamber baiting and activator interaction
pub fn generate_chamber_bot_waypoints(config: &HazardChamberConfig) -> HazardChamberBotWaypoints {
    let p = config.position;
    let hl = config.chamber_length * 0.5;
    let hh = config.chamber_height * 0.5;

    let bait = p + Vec3::new(0.0, 0.0, -hh + 32.0);
    let console = p + config.console_offset + Vec3::new(0.0, 0.0, -hh + 32.0);
    let safe_exit = p + Vec3::new(0.0, -hl - 64.0, -hh + 32.0);

    HazardChamberBotWaypoints {
        bait_inside: bait,
        console_activator: console,
        safe_outside_door: safe_exit,
    }
}

/// Audits chamber configuration for softlocks, unsealed geometries, or gameplay hazards
pub fn audit_hazard_chamber_safety(
    config: &HazardChamberConfig,
    _world_brushes: &[Brush],
) -> Vec<HazardChamberHazard> {
    let mut hazards = Vec::new();

    // 1. Zero damage and non-killzone
    if config.damage_per_sec <= 0.0 && !config.b_kill_zone {
        hazards.push(HazardChamberHazard {
            is_critical: true,
            title: "Zero Hazard Chamber Lethality".to_string(),
            description: "DamagePerSec is 0 and bKillZone is false. Chamber cycle will trap players with zero threat.".to_string(),
            recommendation: "Set DamagePerSec to at least 40..200 HP/s or enable bKillZone.".to_string(),
        });
    }

    // 2. Door cycle time too short (< 2.0s)
    if config.vent_cycle_duration_secs < 2.0 {
        hazards.push(HazardChamberHazard {
            is_critical: false,
            title: "Abrupt Vent Cycle Duration".to_string(),
            description: format!("Vent cycle is only {:.1}s. Players barely register the trap before doors reopen.", config.vent_cycle_duration_secs),
            recommendation: "Increase Vent Cycle Duration to 4.0..8.0 seconds for dramatic tension.".to_string(),
        });
    }

    // 3. Console button located inside chamber
    let half_w = config.chamber_width * 0.5;
    let half_l = config.chamber_length * 0.5;
    if config.console_offset.x.abs() < half_w && config.console_offset.y.abs() < half_l {
        hazards.push(HazardChamberHazard {
            is_critical: true,
            title: "Console Activator Inside Trap Chamber".to_string(),
            description: "The control button is located INSIDE the hazard room. Players cannot activate it from outside.".to_string(),
            recommendation: "Move console_offset outside the chamber bounds (e.g. +320 uu along X or -320 uu along Y).".to_string(),
        });
    }

    // 4. Missing event tag
    if config.event_name.trim().is_empty() {
        hazards.push(HazardChamberHazard {
            is_critical: true,
            title: "Unbound Vent Event Name".to_string(),
            description: "event_name is empty. Trigger cannot communicate with the airlock doors or damage zone.".to_string(),
            recommendation: "Provide a unique event name (e.g. VentCycle_Trigger).".to_string(),
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chamber_archetypes_and_defaults() {
        for &arch in HazardChamberArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.chamber_width >= 384.0);
            assert!(cfg.vent_cycle_duration_secs >= 4.0);
            assert!(!cfg.event_name.is_empty());
        }
    }

    #[test]
    fn test_chamber_metrics_calculation() {
        let cfg = HazardChamberArchetype::HighVacuumDecompressionChamber.default_config();
        let m = calculate_chamber_metrics(&cfg);
        assert_eq!(m.chamber_volume_uu3, 512.0 * 512.0 * 256.0);
        assert!(m.is_instant_death);
        assert!(m.time_to_kill_secs <= 0.1);
        assert!(m.total_cycle_time_secs >= 6.0);
    }

    #[test]
    fn test_generate_chamber_brushes_geometry() {
        let cfg = HazardChamberArchetype::HighVacuumDecompressionChamber.default_config();
        let brushes = generate_hazard_chamber_brushes(&cfg);

        assert_eq!(brushes.chamber_brush.polys.len(), 6);
        assert_eq!(brushes.door_left_brush.polys.len(), 6);
        assert_eq!(brushes.door_right_brush.polys.len(), 6);
        assert!(brushes.window_glass_brush.is_some());
        assert_eq!(brushes.console_brush.polys.len(), 6);

        // Check semisolid flag on chamber and doors
        assert_eq!(brushes.chamber_brush.polys[0].flags & 0x00200000, 0x00200000);
        assert_eq!(brushes.door_left_brush.polys[0].flags & 0x00200000, 0x00200000);
    }

    #[test]
    fn test_chamber_safety_audit() {
        let mut cfg = HazardChamberConfig::default();
        cfg.damage_per_sec = 0.0;
        cfg.b_kill_zone = false;
        cfg.console_offset = Vec3::ZERO; // Inside chamber!
        cfg.event_name = "".to_string();

        let hazards = audit_hazard_chamber_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Zero Hazard")));
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Console Activator Inside")));
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Unbound Vent")));
    }
}
