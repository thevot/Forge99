// crates/ut99-bsp/src/radar_studio.rs
//
// Interactive Team Base Radar, Jammer & Holographic Threat Map Studio
// Team base radar stations, tactical radar jammers, sweep rates, detection latencies,
// threat tracking, friendly jammer interference auditing, and procedural parabolic dish antenna generation.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Radar scanning modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadarScanMode {
    /// 360-degree continuous rotating sweep
    OmnidirectionalSweep,
    /// Directional wedge/sector surveillance cone
    DirectionalSectorCone,
    /// Periodic pulse burst with anti-jamming frequency hopping
    FrequencyHoppingPulse,
}

impl RadarScanMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            RadarScanMode::OmnidirectionalSweep => "360° Omnidirectional Sweep",
            RadarScanMode::DirectionalSectorCone => "Directional Sector Cone (Targeted Chokepoint)",
            RadarScanMode::FrequencyHoppingPulse => "Anti-Jamming Frequency Hopping Pulse",
        }
    }
}

/// Archetypes of canonical base radar stations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadarStudioArchetype {
    OrbitalCommandHQ,
    StealthInfiltrationOutpost,
    FortressPerimeterSentinel,
    SubterraneanBunkerGrid,
    MobileAPCReconVehicle,
}

impl RadarStudioArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            RadarStudioArchetype::OrbitalCommandHQ => "Orbital Command HQ (Long-Range 360° Array)",
            RadarStudioArchetype::StealthInfiltrationOutpost => "Tactical Cloaking Jammer (Scrambler Node)",
            RadarStudioArchetype::FortressPerimeterSentinel => "Fortress Perimeter Sentinel (Sector Siren)",
            RadarStudioArchetype::SubterraneanBunkerGrid => "Subterranean Seismic Grid (Tunnel Penetration)",
            RadarStudioArchetype::MobileAPCReconVehicle => "Mobile Recon Array (Doppler Moving Indicator)",
        }
    }

    pub fn to_config(&self) -> RadarStationConfig {
        match self {
            RadarStudioArchetype::OrbitalCommandHQ => RadarStationConfig {
                archetype: *self,
                scan_mode: RadarScanMode::OmnidirectionalSweep,
                team: 0, // Red
                position: Vec3::new(0.0, 0.0, 0.0),
                detection_radius_uu: 3200.0,
                sweep_rate_rpm: 30.0,
                cone_angle_degrees: 360.0,
                is_jammer: false,
                jamming_strength: 0.0,
                dish_radius: 48.0,
                dish_depth: 16.0,
                mast_height: 96.0,
                tag: "Red_CommandRadar".to_string(),
            },
            RadarStudioArchetype::StealthInfiltrationOutpost => RadarStationConfig {
                archetype: *self,
                scan_mode: RadarScanMode::FrequencyHoppingPulse,
                team: 1, // Blue
                position: Vec3::new(300.0, -200.0, 32.0),
                detection_radius_uu: 1200.0,
                sweep_rate_rpm: 0.0,
                cone_angle_degrees: 360.0,
                is_jammer: true,
                jamming_strength: 0.85,
                dish_radius: 28.0,
                dish_depth: 8.0,
                mast_height: 48.0,
                tag: "Blue_StealthJammer".to_string(),
            },
            RadarStudioArchetype::FortressPerimeterSentinel => RadarStationConfig {
                archetype: *self,
                scan_mode: RadarScanMode::DirectionalSectorCone,
                team: 0, // Red
                position: Vec3::new(-400.0, 300.0, 64.0),
                detection_radius_uu: 2400.0,
                sweep_rate_rpm: 20.0,
                cone_angle_degrees: 90.0,
                is_jammer: false,
                jamming_strength: 0.0,
                dish_radius: 40.0,
                dish_depth: 12.0,
                mast_height: 80.0,
                tag: "Red_PerimeterSentinel".to_string(),
            },
            RadarStudioArchetype::SubterraneanBunkerGrid => RadarStationConfig {
                archetype: *self,
                scan_mode: RadarScanMode::FrequencyHoppingPulse,
                team: 1, // Blue
                position: Vec3::new(0.0, 500.0, -64.0),
                detection_radius_uu: 1800.0,
                sweep_rate_rpm: 15.0,
                cone_angle_degrees: 360.0,
                is_jammer: false,
                jamming_strength: 0.0,
                dish_radius: 36.0,
                dish_depth: 10.0,
                mast_height: 64.0,
                tag: "Blue_BunkerGrid".to_string(),
            },
            RadarStudioArchetype::MobileAPCReconVehicle => RadarStationConfig {
                archetype: *self,
                scan_mode: RadarScanMode::OmnidirectionalSweep,
                team: 0, // Red
                position: Vec3::new(600.0, 0.0, 16.0),
                detection_radius_uu: 2000.0,
                sweep_rate_rpm: 45.0,
                cone_angle_degrees: 360.0,
                is_jammer: false,
                jamming_strength: 0.0,
                dish_radius: 32.0,
                dish_depth: 10.0,
                mast_height: 56.0,
                tag: "Red_ReconVehicle".to_string(),
            },
        }
    }
}

/// Configuration settings for Radar or Jammer station
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadarStationConfig {
    pub archetype: RadarStudioArchetype,
    pub scan_mode: RadarScanMode,
    pub team: u8, // 0=Red, 1=Blue, 2=Green, 3=Gold
    pub position: Vec3,
    pub detection_radius_uu: f32,
    pub sweep_rate_rpm: f32, // Rotations per minute
    pub cone_angle_degrees: f32,
    pub is_jammer: bool,
    pub jamming_strength: f32, // 0.0 to 1.0
    pub dish_radius: f32,
    pub dish_depth: f32,
    pub mast_height: f32,
    pub tag: String,
}

impl Default for RadarStationConfig {
    fn default() -> Self {
        RadarStudioArchetype::OrbitalCommandHQ.to_config()
    }
}

/// Representation of an existing Radar/Jammer node in the map
#[derive(Debug, Clone, PartialEq)]
pub struct LevelRadarNodeRecord {
    pub name: String,
    pub team: u8,
    pub position: Vec3,
    pub is_jammer: bool,
    pub radius: f32,
}

/// Telemetry metrics for Radar and Threat Tracking
#[derive(Debug, Clone, PartialEq)]
pub struct RadarMetrics {
    pub effective_coverage_area_sq_uu: f32,
    pub threat_detection_latency_secs: f32,
    pub jammer_counter_effectiveness_pct: f32,
    pub total_nodes_in_level: usize,
    pub distance_to_closest_peer_uu: f32,
}

/// Hazards and interference diagnostics
#[derive(Debug, Clone, PartialEq)]
pub struct RadarHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates radar coverage metrics and sweep delays
pub fn evaluate_radar_metrics(
    config: &RadarStationConfig,
    peers: &[LevelRadarNodeRecord],
) -> RadarMetrics {
    let area = std::f32::consts::PI * config.detection_radius_uu.powi(2)
        * (config.cone_angle_degrees / 360.0);

    // Latency = time for half rotation (on average)
    let latency = if config.sweep_rate_rpm > 0.0 {
        30.0 / config.sweep_rate_rpm
    } else {
        0.5
    };

    let mut closest = 9999.0_f32;
    for p in peers {
        let d = p.position.distance(config.position);
        if d < closest {
            closest = d;
        }
    }

    let counter_eff = if config.is_jammer {
        config.jamming_strength * 100.0
    } else {
        (1.0 - config.jamming_strength) * 100.0
    };

    RadarMetrics {
        effective_coverage_area_sq_uu: area,
        threat_detection_latency_secs: latency,
        jammer_counter_effectiveness_pct: counter_eff,
        total_nodes_in_level: peers.len() + 1,
        distance_to_closest_peer_uu: closest,
    }
}

/// Audits radar setup for blind spots, excessive sweep latency, and friendly jammer interference
pub fn audit_radar_safety(
    config: &RadarStationConfig,
    peers: &[LevelRadarNodeRecord],
) -> Vec<RadarHazard> {
    let mut hazards = Vec::new();

    // 1. Critical Friendly Jammer Interference Hazard
    if config.is_jammer {
        for p in peers {
            if !p.is_jammer && p.team == config.team {
                let dist = p.position.distance(config.position);
                if dist < config.detection_radius_uu {
                    hazards.push(RadarHazard {
                        title: "Critical Friendly Radar Jamming Interference".to_string(),
                        description: format!(
                            "Jammer is within {:.0} uu of friendly radar '{}'. It will blind your own team's radar network!",
                            dist, p.name
                        ),
                        is_critical: true,
                    });
                }
            }
        }
    }

    // 2. High Sweep Latency Warning
    if !config.is_jammer && config.sweep_rate_rpm > 0.0 && config.sweep_rate_rpm < 12.0 {
        hazards.push(RadarHazard {
            title: "Excessive Threat Detection Latency".to_string(),
            description: format!(
                "Sweep rate of {:.1} RPM causes over 2.5s scan delay. Fast enemy flag runners or translocators can pass undetected.",
                config.sweep_rate_rpm
            ),
            is_critical: false,
        });
    }

    // 3. Narrow Chokepoint Cone Warning
    if config.scan_mode == RadarScanMode::DirectionalSectorCone && config.cone_angle_degrees < 45.0 {
        hazards.push(RadarHazard {
            title: "Extremely Narrow Surveillance Sector".to_string(),
            description: "Surveillance cone is under 45 degrees, creating large peripheral blind spots around the base entrance.".to_string(),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a procedural hexagonal mast and parabolic dish brush
pub fn generate_radar_mast_and_dish_brush(config: &RadarStationConfig) -> Brush {
    let mut brush = Brush::new("RadarMastAndDish".to_string(), CsgOperation::Active);
    brush.location = config.position;

    // Mast: Hexagonal prism
    let mast_h = config.mast_height;
    let mast_r = (config.dish_radius * 0.25).clamp(8.0, 24.0);
    let sides = 6;
    let mut bot_ring = Vec::new();
    let mut top_ring = Vec::new();

    for i in 0..sides {
        let angle = std::f32::consts::TAU * (i as f32) / (sides as f32);
        let x = mast_r * angle.cos();
        let y = mast_r * angle.sin();
        bot_ring.push(Vec3::new(x, y, 0.0));
        top_ring.push(Vec3::new(x, y, mast_h));
    }

    // Mast sides
    for i in 0..sides {
        let next = (i + 1) % sides;
        if let Some(poly) = Polygon::new(vec![
            bot_ring[i],
            bot_ring[next],
            top_ring[next],
            top_ring[i],
        ]) {
            brush.polys.push(poly);
        }
    }

    // Dish: Octagonal parabolic reflector atop mast
    let dish_z = mast_h + 12.0;
    let dish_r = config.dish_radius;
    let dish_sides = 8;
    let mut dish_rim = Vec::new();

    for i in 0..dish_sides {
        let angle = std::f32::consts::TAU * (i as f32) / (dish_sides as f32);
        let x = dish_r * angle.cos();
        let y = dish_r * angle.sin();
        dish_rim.push(Vec3::new(x, y, dish_z + config.dish_depth));
    }

    let dish_apex = Vec3::new(0.0, 0.0, dish_z);

    // Dish facet triangles
    for i in 0..dish_sides {
        let next = (i + 1) % dish_sides;
        if let Some(poly) = Polygon::new(vec![
            dish_apex,
            dish_rim[i],
            dish_rim[next],
        ]) {
            brush.polys.push(poly);
        }
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radar_archetypes_and_defaults() {
        let cfg = RadarStudioArchetype::OrbitalCommandHQ.to_config();
        assert_eq!(cfg.scan_mode, RadarScanMode::OmnidirectionalSweep);
        assert_eq!(cfg.detection_radius_uu, 3200.0);
        assert_eq!(cfg.sweep_rate_rpm, 30.0);
        assert!(!cfg.is_jammer);
    }

    #[test]
    fn test_radar_metrics_evaluation() {
        let cfg = RadarStationConfig::default();
        let peers = vec![LevelRadarNodeRecord {
            name: "OutpostRadar".to_string(),
            team: 0,
            position: Vec3::new(1200.0, 0.0, 0.0),
            is_jammer: false,
            radius: 2000.0,
        }];

        let metrics = evaluate_radar_metrics(&cfg, &peers);
        assert_eq!(metrics.total_nodes_in_level, 2);
        assert_eq!(metrics.threat_detection_latency_secs, 1.0); // 30 / 30 = 1.0s
        assert_eq!(metrics.distance_to_closest_peer_uu, 1200.0);
        assert!(metrics.effective_coverage_area_sq_uu > 10_000_000.0);
    }

    #[test]
    fn test_radar_hazard_auditing() {
        let mut jammer_cfg = RadarStudioArchetype::StealthInfiltrationOutpost.to_config();
        jammer_cfg.team = 0; // Same team!

        let peers = vec![LevelRadarNodeRecord {
            name: "FriendlyRadar".to_string(),
            team: 0,
            position: Vec3::new(400.0, -200.0, 32.0), // Inside 1200 uu radius!
            is_jammer: false,
            radius: 3000.0,
        }];

        let hazards = audit_radar_safety(&jammer_cfg, &peers);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Friendly Radar Jamming")));
    }

    #[test]
    fn test_generate_radar_mast_and_dish_brush() {
        let cfg = RadarStationConfig::default();
        let brush = generate_radar_mast_and_dish_brush(&cfg);

        assert_eq!(brush.name, "RadarMastAndDish");
        // 6 mast sides + 8 dish facets = 14 polys
        assert_eq!(brush.polys.len(), 14);
    }
}
