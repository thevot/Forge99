// crates/ut99-bsp/src/trigger_network_studio.rs
//
// Interactive Pressure Plate & Proximity Trigger Network Studio
// Pressure plates, proximity trigger cylinders, tripwires, event-tag network graph resolution,
// orphan event diagnostics, trigger loop detection, and procedural pressure plate frame brush.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Type of trigger activation mechanism
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerActivationType {
    PlayerProximityRadius,
    DepressibleFloorPad,
    BeamBreakInfraredTripwire,
    UseKeyInteractiveConsole,
    ShootDamageTrigger,
}

impl TriggerActivationType {
    pub fn display_name(&self) -> &'static str {
        match self {
            TriggerActivationType::PlayerProximityRadius => "Player Proximity Cylinder (TT_PlayerProximity)",
            TriggerActivationType::DepressibleFloorPad => "Depressible Floor Plate (Spring-Loaded)",
            TriggerActivationType::BeamBreakInfraredTripwire => "Beam Break Tripwire (Infrared Ray)",
            TriggerActivationType::UseKeyInteractiveConsole => "Interactive Console (Use Key / Action)",
            TriggerActivationType::ShootDamageTrigger => "Shoot Damage Sensor (Weapon Hit)",
        }
    }

    pub fn default_radius_and_height(&self) -> (f32, f32) {
        match self {
            TriggerActivationType::PlayerProximityRadius => (120.0, 80.0),
            TriggerActivationType::DepressibleFloorPad => (64.0, 16.0),
            TriggerActivationType::BeamBreakInfraredTripwire => (200.0, 32.0),
            TriggerActivationType::UseKeyInteractiveConsole => (48.0, 64.0),
            TriggerActivationType::ShootDamageTrigger => (80.0, 80.0),
        }
    }
}

/// Archetypes of canonical UT99 trigger networks and environmental switches
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerNetworkArchetype {
    PressureChamberDecompressorSwitch,
    TrapdoorHiddenPitTrigger,
    ReinforcedSecurityGateTerminal,
    IntruderAlarmTripwire,
    GravityGeneratorOverrideConsole,
}

impl TriggerNetworkArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            TriggerNetworkArchetype::PressureChamberDecompressorSwitch => "Pressure Chamber Decompressor (DM-Pressure Hazard)",
            TriggerNetworkArchetype::TrapdoorHiddenPitTrigger => "Trapdoor Hidden Pit Fall (Floor Pad Trigger)",
            TriggerNetworkArchetype::ReinforcedSecurityGateTerminal => "Security Gate Access Terminal (Team Base Key)",
            TriggerNetworkArchetype::IntruderAlarmTripwire => "Intruder Alert Tripwire (Base Perimeter Siren)",
            TriggerNetworkArchetype::GravityGeneratorOverrideConsole => "Gravity Override Station (Low-G Switch)",
        }
    }

    pub fn to_config(&self) -> TriggerNetworkConfig {
        match self {
            TriggerNetworkArchetype::PressureChamberDecompressorSwitch => TriggerNetworkConfig {
                archetype: *self,
                trigger_type: TriggerActivationType::UseKeyInteractiveConsole,
                position: Vec3::new(0.0, 0.0, 0.0),
                event_name: "DecompressChamber".to_string(),
                retrigger_delay_secs: 10.0,
                is_single_use: false,
                team_filter: 0, // Any
                activation_radius_uu: 64.0,
                activation_height_uu: 64.0,
                plate_width_uu: 48.0,
                plate_length_uu: 48.0,
                plate_thickness_uu: 12.0,
                message: "Chamber Decompression Activated!".to_string(),
            },
            TriggerNetworkArchetype::TrapdoorHiddenPitTrigger => TriggerNetworkConfig {
                archetype: *self,
                trigger_type: TriggerActivationType::DepressibleFloorPad,
                position: Vec3::new(0.0, 0.0, 0.0),
                event_name: "OpenPitTrapdoor".to_string(),
                retrigger_delay_secs: 5.0,
                is_single_use: false,
                team_filter: 0,
                activation_radius_uu: 72.0,
                activation_height_uu: 24.0,
                plate_width_uu: 64.0,
                plate_length_uu: 64.0,
                plate_thickness_uu: 8.0,
                message: "".to_string(),
            },
            TriggerNetworkArchetype::ReinforcedSecurityGateTerminal => TriggerNetworkConfig {
                archetype: *self,
                trigger_type: TriggerActivationType::UseKeyInteractiveConsole,
                position: Vec3::new(0.0, 0.0, 0.0),
                event_name: "OpenSecurityGate_Red".to_string(),
                retrigger_delay_secs: 3.0,
                is_single_use: false,
                team_filter: 1, // Red Team only
                activation_radius_uu: 56.0,
                activation_height_uu: 72.0,
                plate_width_uu: 40.0,
                plate_length_uu: 40.0,
                plate_thickness_uu: 16.0,
                message: "Red Security Clearance Accepted".to_string(),
            },
            TriggerNetworkArchetype::IntruderAlarmTripwire => TriggerNetworkConfig {
                archetype: *self,
                trigger_type: TriggerActivationType::BeamBreakInfraredTripwire,
                position: Vec3::new(0.0, 0.0, 0.0),
                event_name: "SoundBaseSiren".to_string(),
                retrigger_delay_secs: 15.0,
                is_single_use: false,
                team_filter: 2, // Opposing Blue Team
                activation_radius_uu: 240.0,
                activation_height_uu: 32.0,
                plate_width_uu: 80.0,
                plate_length_uu: 16.0,
                plate_thickness_uu: 8.0,
                message: "Intruder Alert in Sector 4!".to_string(),
            },
            TriggerNetworkArchetype::GravityGeneratorOverrideConsole => TriggerNetworkConfig {
                archetype: *self,
                trigger_type: TriggerActivationType::UseKeyInteractiveConsole,
                position: Vec3::new(0.0, 0.0, 0.0),
                event_name: "ToggleSectorGravity".to_string(),
                retrigger_delay_secs: 8.0,
                is_single_use: false,
                team_filter: 0,
                activation_radius_uu: 60.0,
                activation_height_uu: 64.0,
                plate_width_uu: 52.0,
                plate_length_uu: 52.0,
                plate_thickness_uu: 12.0,
                message: "Sector Gravity Inversion Toggled".to_string(),
            },
        }
    }
}

/// Configuration settings for Interactive Trigger Network
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerNetworkConfig {
    pub archetype: TriggerNetworkArchetype,
    pub trigger_type: TriggerActivationType,
    pub position: Vec3,
    pub event_name: String,
    pub retrigger_delay_secs: f32,
    pub is_single_use: bool,
    pub team_filter: u8,
    pub activation_radius_uu: f32,
    pub activation_height_uu: f32,
    pub plate_width_uu: f32,
    pub plate_length_uu: f32,
    pub plate_thickness_uu: f32,
    pub message: String,
}

impl Default for TriggerNetworkConfig {
    fn default() -> Self {
        TriggerNetworkArchetype::PressureChamberDecompressorSwitch.to_config()
    }
}

/// Representation of an existing actor in the map for network graph resolution
#[derive(Debug, Clone, PartialEq)]
pub struct LevelTriggerContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub tag: String,
    pub event: String,
    pub position: Vec3,
}

/// Telemetry metrics for Trigger Network
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerNetworkMetrics {
    pub total_target_actors_wired: usize,
    pub network_link_distance_uu: f32,
    pub trigger_volume_cu_uu: f32,
    pub total_triggers_in_level: usize,
}

/// Safety diagnostics and network routing alerts
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerNetworkHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates network wiring targets, link distance, and activation volume
pub fn evaluate_trigger_network_metrics(
    config: &TriggerNetworkConfig,
    peers: &[LevelTriggerContextActorRecord],
) -> TriggerNetworkMetrics {
    let mut targets = 0;
    let mut min_dist = f32::MAX;
    let mut triggers_count = 1;

    for p in peers {
        if p.class_name.to_lowercase().contains("trigger") {
            triggers_count += 1;
        }
        if !config.event_name.is_empty() && p.tag.eq_ignore_ascii_case(&config.event_name) {
            targets += 1;
            let d = p.position.distance(config.position);
            if d < min_dist {
                min_dist = d;
            }
        }
    }

    let link_dist = if targets > 0 { min_dist } else { 0.0 };
    let volume = std::f32::consts::PI * config.activation_radius_uu.powi(2) * config.activation_height_uu;

    TriggerNetworkMetrics {
        total_target_actors_wired: targets,
        network_link_distance_uu: link_dist,
        trigger_volume_cu_uu: volume,
        total_triggers_in_level: triggers_count,
    }
}

/// Audits safety for orphan events, infinite loops, and spamming retrigger delays
pub fn audit_trigger_network_safety(
    config: &TriggerNetworkConfig,
    peers: &[LevelTriggerContextActorRecord],
) -> Vec<TriggerNetworkHazard> {
    let mut hazards = Vec::new();

    // 1. Orphan Event Warning: Trigger fires an event that has 0 matching tags in the level
    if !config.event_name.trim().is_empty() {
        let has_recipient = peers.iter().any(|p| p.tag.eq_ignore_ascii_case(&config.event_name));
        if !has_recipient && !peers.is_empty() {
            hazards.push(TriggerNetworkHazard {
                title: "Orphan Event Wire (No Target Actor)".to_string(),
                description: format!(
                    "Trigger fires Event '{}', but no actor in the level has a matching Tag. The trigger will execute with no recipient effect!",
                    config.event_name
                ),
                is_critical: false,
            });
        }
    } else {
        hazards.push(TriggerNetworkHazard {
            title: "Empty Event Name".to_string(),
            description: "Trigger has no Event name assigned. It will never broadcast to any Mover, Door, or Dispatcher.".to_string(),
            is_critical: true,
        });
    }

    // 2. Retrigger Delay Spam Warning (< 0.2s without single use)
    if !config.is_single_use && config.retrigger_delay_secs < 0.2 {
        hazards.push(TriggerNetworkHazard {
            title: "Rapid Retrigger Spam".to_string(),
            description: format!(
                "Retrigger delay of {:.2}s is extremely low. Moving or standing on this trigger will flood the event dispatcher network.",
                config.retrigger_delay_secs
            ),
            is_critical: true,
        });
    }

    // 3. Trigger Loop Feedback: One of the target actors triggers an event matching this trigger's tag
    for p in peers {
        if p.tag.eq_ignore_ascii_case(&config.event_name) && !p.event.is_empty() {
            // If the recipient fires an event matching the trigger's event name -> loop!
            if p.event.eq_ignore_ascii_case(&config.event_name) {
                hazards.push(TriggerNetworkHazard {
                    title: "Trigger Feedback Loop Detected".to_string(),
                    description: format!(
                        "Actor '{}' with Tag '{}' triggers Event '{}', creating an infinite triggering cycle!",
                        p.name, p.tag, p.event
                    ),
                    is_critical: true,
                });
            }
        }
    }

    hazards
}

/// Generates a procedural rectangular pressure plate frame brush in CSG
pub fn generate_pressure_plate_brush(config: &TriggerNetworkConfig) -> Brush {
    let mut brush = Brush::new("PressurePlate_Frame".to_string(), CsgOperation::Active);
    brush.location = config.position;

    let hw = config.plate_width_uu * 0.5;
    let hl = config.plate_length_uu * 0.5;
    let h = config.plate_thickness_uu;

    let p0 = Vec3::new(-hw, -hl, 0.0);
    let p1 = Vec3::new(hw, -hl, 0.0);
    let p2 = Vec3::new(hw, hl, 0.0);
    let p3 = Vec3::new(-hw, hl, 0.0);

    let p4 = Vec3::new(-hw * 0.9, -hl * 0.9, h);
    let p5 = Vec3::new(hw * 0.9, -hl * 0.9, h);
    let p6 = Vec3::new(hw * 0.9, hl * 0.9, h);
    let p7 = Vec3::new(-hw * 0.9, hl * 0.9, h);

    // 1. Bottom Cap
    if let Some(poly) = Polygon::new(vec![p0, p1, p2, p3]) {
        brush.polys.push(poly);
    }

    // 2. Sides
    if let Some(poly) = Polygon::new(vec![p0, p4, p5, p1]) { brush.polys.push(poly); }
    if let Some(poly) = Polygon::new(vec![p1, p5, p6, p2]) { brush.polys.push(poly); }
    if let Some(poly) = Polygon::new(vec![p2, p6, p7, p3]) { brush.polys.push(poly); }
    if let Some(poly) = Polygon::new(vec![p3, p7, p4, p0]) { brush.polys.push(poly); }

    // 3. Top Cap
    if let Some(poly) = Polygon::new(vec![p7, p6, p5, p4]) {
        brush.polys.push(poly);
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_network_archetypes_and_defaults() {
        let cfg = TriggerNetworkArchetype::PressureChamberDecompressorSwitch.to_config();
        assert_eq!(cfg.trigger_type, TriggerActivationType::UseKeyInteractiveConsole);
        assert_eq!(cfg.event_name, "DecompressChamber");
        assert_eq!(cfg.retrigger_delay_secs, 10.0);
    }

    #[test]
    fn test_trigger_network_metrics_evaluation() {
        let cfg = TriggerNetworkConfig::default();
        let peers = vec![
            LevelTriggerContextActorRecord {
                name: "PressureDoor0".to_string(),
                class_name: "Engine.Mover".to_string(),
                tag: "DecompressChamber".to_string(),
                event: "".to_string(),
                position: Vec3::new(400.0, 0.0, 0.0),
            },
        ];

        let metrics = evaluate_trigger_network_metrics(&cfg, &peers);
        assert_eq!(metrics.total_target_actors_wired, 1);
        assert_eq!(metrics.network_link_distance_uu, 400.0);
        assert!(metrics.trigger_volume_cu_uu > 10_000.0);
    }

    #[test]
    fn test_trigger_network_hazard_auditing() {
        let cfg = TriggerNetworkConfig::default(); // Event = "DecompressChamber"
        let peers = vec![
            LevelTriggerContextActorRecord {
                name: "UnrelatedActor".to_string(),
                class_name: "Engine.Mover".to_string(),
                tag: "DifferentTag".to_string(),
                event: "".to_string(),
                position: Vec3::new(100.0, 0.0, 0.0),
            },
        ];

        let hazards = audit_trigger_network_safety(&cfg, &peers);
        assert!(hazards.iter().any(|h| !h.is_critical && h.title.contains("Orphan Event Wire")));
    }

    #[test]
    fn test_generate_pressure_plate_brush() {
        let cfg = TriggerNetworkConfig::default();
        let brush = generate_pressure_plate_brush(&cfg);

        assert_eq!(brush.name, "PressurePlate_Frame");
        // 1 bottom cap + 4 sides + 1 top cap = 6 polys
        assert_eq!(brush.polys.len(), 6);
    }
}
