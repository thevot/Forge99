// crates/ut99-bsp/src/reactor_core_studio.rs
//
// Interactive Rotating Gyroscope, Reactor Core & Kinetic Ring Studio
// Mathematical modeling of centrifugal kinetic rings, rotating gyroscopes, pulsating reactor cores,
// radiation hazard envelopes, mover synchronization, and procedural CSG geometry.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical archetypes of reactor cores, gyroscopes, and kinetic rings in UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReactorCoreArchetype {
    LiandriCentrifugalCore,
    TurbineVentilationRotor,
    PhobosQuantumGyroscope,
    NivenNuclearContainmentVessel,
    SkaarjMothershipGravitonNexus,
}

impl ReactorCoreArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            ReactorCoreArchetype::LiandriCentrifugalCore => {
                "Liandri Centrifugal Core (DM-Liandri Central Reactor Pit)"
            }
            ReactorCoreArchetype::TurbineVentilationRotor => {
                "Turbine Ventilation Rotor (DM-Turbine Floor Generator)"
            }
            ReactorCoreArchetype::PhobosQuantumGyroscope => {
                "Phobos Quantum Gyroscope (DM-Phobos Orbital Gravity Gimbal)"
            }
            ReactorCoreArchetype::NivenNuclearContainmentVessel => {
                "Niven Nuclear Containment Vessel (CTF-Niven Radiation Chamber)"
            }
            ReactorCoreArchetype::SkaarjMothershipGravitonNexus => {
                "Skaarj Mothership Graviton Nexus (Alien Levitating Plasma Sphere)"
            }
        }
    }

    pub fn to_config(&self) -> ReactorCoreConfig {
        match self {
            ReactorCoreArchetype::LiandriCentrifugalCore => ReactorCoreConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                core_radius_uu: 48.0,
                core_height_uu: 96.0,
                outer_ring_radius_uu: 160.0,
                mid_ring_radius_uu: 112.0,
                ring_thickness_uu: 16.0,
                ring_height_uu: 24.0,
                outer_ring_rpm: 15.0,
                mid_ring_rpm: -25.0, // Counter-rotating
                hazard_type: ReactorHazardType::IonizingRadiation,
                hazard_radius_uu: 256.0,
                damage_per_sec: 35.0,
                containment_health: 1200.0,
                critical_meltdown_sec: 30.0,
                tag: "LiandriCore0".to_string(),
                event: "CoreMeltdownInitiated".to_string(),
            },
            ReactorCoreArchetype::TurbineVentilationRotor => ReactorCoreConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, -64.0),
                core_radius_uu: 64.0,
                core_height_uu: 48.0,
                outer_ring_radius_uu: 220.0,
                mid_ring_radius_uu: 140.0,
                ring_thickness_uu: 24.0,
                ring_height_uu: 32.0,
                outer_ring_rpm: 45.0,
                mid_ring_rpm: 45.0,
                hazard_type: ReactorHazardType::GravitationalCrush,
                hazard_radius_uu: 180.0,
                damage_per_sec: 60.0,
                containment_health: 800.0,
                critical_meltdown_sec: 20.0,
                tag: "TurbineGenerator0".to_string(),
                event: "TurbineOverheat".to_string(),
            },
            ReactorCoreArchetype::PhobosQuantumGyroscope => ReactorCoreConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 128.0),
                core_radius_uu: 36.0,
                core_height_uu: 72.0,
                outer_ring_radius_uu: 180.0,
                mid_ring_radius_uu: 120.0,
                ring_thickness_uu: 12.0,
                ring_height_uu: 16.0,
                outer_ring_rpm: 20.0,
                mid_ring_rpm: -30.0,
                hazard_type: ReactorHazardType::PlasmaArcDischarge,
                hazard_radius_uu: 220.0,
                damage_per_sec: 45.0,
                containment_health: 1500.0,
                critical_meltdown_sec: 45.0,
                tag: "PhobosGyro0".to_string(),
                event: "GravityInversionTriggered".to_string(),
            },
            ReactorCoreArchetype::NivenNuclearContainmentVessel => ReactorCoreConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 0.0),
                core_radius_uu: 56.0,
                core_height_uu: 128.0,
                outer_ring_radius_uu: 200.0,
                mid_ring_radius_uu: 130.0,
                ring_thickness_uu: 20.0,
                ring_height_uu: 28.0,
                outer_ring_rpm: 10.0,
                mid_ring_rpm: -15.0,
                hazard_type: ReactorHazardType::IonizingRadiation,
                hazard_radius_uu: 320.0,
                damage_per_sec: 50.0,
                containment_health: 2000.0,
                critical_meltdown_sec: 60.0,
                tag: "NivenVessel0".to_string(),
                event: "RadiationBreachAlert".to_string(),
            },
            ReactorCoreArchetype::SkaarjMothershipGravitonNexus => ReactorCoreConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 64.0),
                core_radius_uu: 40.0,
                core_height_uu: 80.0,
                outer_ring_radius_uu: 150.0,
                mid_ring_radius_uu: 96.0,
                ring_thickness_uu: 14.0,
                ring_height_uu: 20.0,
                outer_ring_rpm: 60.0,
                mid_ring_rpm: -75.0,
                hazard_type: ReactorHazardType::ExtremeThermal,
                hazard_radius_uu: 280.0,
                damage_per_sec: 80.0,
                containment_health: 2500.0,
                critical_meltdown_sec: 25.0,
                tag: "SkaarjNexus0".to_string(),
                event: "NexusDestabilized".to_string(),
            },
        }
    }
}

/// Environmental hazard classifications for reactor cores
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReactorHazardType {
    IonizingRadiation,
    ExtremeThermal,
    PlasmaArcDischarge,
    GravitationalCrush,
    ToxicCoolantMist,
}

impl ReactorHazardType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ReactorHazardType::IonizingRadiation => "Ionizing Radiation (Nuclear Bleed)",
            ReactorHazardType::ExtremeThermal => "Extreme Thermal (Molten Heat Wave)",
            ReactorHazardType::PlasmaArcDischarge => "Plasma Arc Discharge (Electrical Arcing)",
            ReactorHazardType::GravitationalCrush => "Gravitational Crush (Centrifugal Shredding)",
            ReactorHazardType::ToxicCoolantMist => "Toxic Coolant Mist (Chemical Vapor)",
        }
    }
}

/// Comprehensive configuration for a Rotating Gyroscope and Reactor Core setup
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactorCoreConfig {
    pub archetype: ReactorCoreArchetype,
    pub position: Vec3,
    pub core_radius_uu: f32,
    pub core_height_uu: f32,
    pub outer_ring_radius_uu: f32,
    pub mid_ring_radius_uu: f32,
    pub ring_thickness_uu: f32,
    pub ring_height_uu: f32,
    pub outer_ring_rpm: f32,
    pub mid_ring_rpm: f32,
    pub hazard_type: ReactorHazardType,
    pub hazard_radius_uu: f32,
    pub damage_per_sec: f32,
    pub containment_health: f32,
    pub critical_meltdown_sec: f32,
    pub tag: String,
    pub event: String,
}

impl Default for ReactorCoreConfig {
    fn default() -> Self {
        ReactorCoreArchetype::LiandriCentrifugalCore.to_config()
    }
}

/// Telemetry metrics for kinetic rotation and reactor radiation hazard
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactorCoreMetrics {
    pub core_volume_uu3: f32,
    pub outer_rim_tangential_speed_uus: f32,
    pub centrifugal_g_force: f32,
    pub stored_kinetic_energy_mj: f32,
    pub hazard_volume_uu3: f32,
    pub time_to_lethal_sec: f32,
    pub total_reactors_in_level: usize,
}

/// Context actor record for safety checking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelReactorContextActorRecord {
    pub name: String,
    pub class_name: String,
    pub position: Vec3,
}

/// Compute telemetry and physical metrics for the reactor core setup
pub fn evaluate_reactor_core_metrics(
    config: &ReactorCoreConfig,
    peers: &[LevelReactorContextActorRecord],
) -> ReactorCoreMetrics {
    let pi = std::f32::consts::PI;
    let core_vol = pi * config.core_radius_uu * config.core_radius_uu * config.core_height_uu;

    // Tangential speed: v = 2 * pi * R * (RPM / 60)
    let outer_v = (2.0 * pi * config.outer_ring_radius_uu * (config.outer_ring_rpm.abs() / 60.0)).max(0.0);

    // Centrifugal g-force: a = v^2 / R, in g-units (1g = 980 uu/s^2 approx)
    let g_force = if config.outer_ring_radius_uu > 0.0 {
        (outer_v * outer_v) / (config.outer_ring_radius_uu * 980.0)
    } else {
        0.0
    };

    // Stored kinetic energy approximation: E = 0.5 * I * omega^2
    let mass_approx = config.outer_ring_radius_uu * 0.25;
    let _omega = config.outer_ring_rpm.abs() * (2.0 * pi / 60.0);
    let kinetic_mj = 0.5 * mass_approx * outer_v * outer_v * 0.0001;

    let hazard_vol = (4.0 / 3.0) * pi * config.hazard_radius_uu.powi(3);
    let ttl = if config.damage_per_sec > 0.0 {
        100.0 / config.damage_per_sec
    } else {
        999.0
    };

    let mut count = 1;
    for p in peers {
        let cls = p.class_name.to_lowercase();
        if cls.contains("reactor") || cls.contains("core") || cls.contains("gyro") {
            count += 1;
        }
    }

    ReactorCoreMetrics {
        core_volume_uu3: core_vol,
        outer_rim_tangential_speed_uus: outer_v,
        centrifugal_g_force: g_force,
        stored_kinetic_energy_mj: kinetic_mj,
        hazard_volume_uu3: hazard_vol,
        time_to_lethal_sec: ttl,
        total_reactors_in_level: count,
    }
}

/// Potential safety hazards and gameplay balancing alerts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactorCoreHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Audit safety and gameplay hazards for the reactor core setup
pub fn audit_reactor_core_safety(
    config: &ReactorCoreConfig,
    peers: &[LevelReactorContextActorRecord],
) -> Vec<ReactorCoreHazard> {
    let mut hazards = Vec::new();
    let metrics = evaluate_reactor_core_metrics(config, peers);

    // 1. Lethal radiation boundary encroachment on player spawns
    for p in peers {
        let cls = p.class_name.to_lowercase();
        if cls.contains("playerstart") {
            let dist = (p.position - config.position).length();
            if dist < config.hazard_radius_uu {
                hazards.push(ReactorCoreHazard {
                    title: "Lethal Radiation Encroaching Spawn Point".to_string(),
                    description: format!(
                        "PlayerStart '{}' is inside the hazard radius ({:.0} uu vs {:.0} uu limit). Players will take damage upon spawning.",
                        p.name, dist, config.hazard_radius_uu
                    ),
                    is_critical: true,
                });
            }
        }
    }

    // 2. Excessive rim speed (> 2000 uu/s) causing physics tunneling or compenetration
    if metrics.outer_rim_tangential_speed_uus > 2000.0 {
        hazards.push(ReactorCoreHazard {
            title: "Excessive Rim Tangential Velocity".to_string(),
            description: format!(
                "Outer rim speed is {:.1} uu/s. Velocities exceeding 2000 uu/s may cause mover collision tunneling.",
                metrics.outer_rim_tangential_speed_uus
            ),
            is_critical: true,
        });
    }

    // 3. Unshielded rapid meltdown hazard
    if config.critical_meltdown_sec < 15.0 && config.containment_health < 500.0 {
        hazards.push(ReactorCoreHazard {
            title: "Unstable Rapid Meltdown Hazard".to_string(),
            description: format!(
                "Meltdown timer is only {:.1}s with low containment health ({:.0} HP). Reactor may trigger unintended match collapse.",
                config.critical_meltdown_sec, config.containment_health
            ),
            is_critical: false,
        });
    }

    // 4. Large hazard radius with zero damage
    if config.hazard_radius_uu > 100.0 && config.damage_per_sec <= 0.0 {
        hazards.push(ReactorCoreHazard {
            title: "Ineffective Hazard Field".to_string(),
            description: format!(
                "Reactor hazard radius is {:.0} uu, but damage per second is 0 HP/s.",
                config.hazard_radius_uu
            ),
            is_critical: false,
        });
    }

    hazards
}

/// Procedural CSG brush generator for the Reactor Core and Concentric Kinetic Rings
pub fn generate_reactor_core_brushes(config: &ReactorCoreConfig) -> (Brush, Vec<Brush>) {
    let p = config.position;
    let sides = 8;

    // 1. Central Core Octagonal Prism (Translucent Plasma / Radiation)
    let mut core_brush = Brush::new("Reactor_PlasmaCore", CsgOperation::Add);
    core_brush.location = p;

    let cr = config.core_radius_uu;
    let ch = config.core_height_uu;
    let mut base_pts = Vec::new();
    let mut top_pts = Vec::new();

    for i in 0..sides {
        let angle = (i as f32) * (std::f32::consts::TAU / sides as f32);
        let offset = Vec3::new(angle.cos() * cr, angle.sin() * cr, -ch * 0.5);
        base_pts.push(offset);
        top_pts.push(offset + Vec3::new(0.0, 0.0, ch));
    }

    let tex_core = "GenFX.Energy.PlasmaCore01";
    let core_flags = crate::PF_TRANSLUCENT | crate::PF_UNLIT;

    // Bottom
    if let Some(mut poly) = Polygon::new(base_pts.clone()) {
        poly.texture = tex_core.to_string();
        poly.flags = core_flags;
        core_brush.polys.push(poly);
    }

    // Sides
    for i in 0..sides {
        let next = (i + 1) % sides;
        if let Some(mut poly) = Polygon::new(vec![
            base_pts[i],
            base_pts[next],
            top_pts[next],
            top_pts[i],
        ]) {
            poly.texture = tex_core.to_string();
            poly.flags = core_flags;
            core_brush.polys.push(poly);
        }
    }

    // Top
    let mut top_reversed = top_pts.clone();
    top_reversed.reverse();
    if let Some(mut poly) = Polygon::new(top_reversed) {
        poly.texture = tex_core.to_string();
        poly.flags = core_flags;
        core_brush.polys.push(poly);
    }

    // 2. Outer and Mid Concentric Ring Segment Brushes (Semi-Solid Metallic Shrouds)
    let mut ring_brushes = Vec::new();
    let tex_ring = "Indus1.Metal.CoreGimbal";

    let ring_specs = [
        ("OuterRing", config.outer_ring_radius_uu, config.ring_height_uu),
        ("MidRing", config.mid_ring_radius_uu, config.ring_height_uu * 0.8),
    ];

    for (name, r, h) in &ring_specs {
        let r_in = r - (config.ring_thickness_uu * 0.5);
        let r_out = r + (config.ring_thickness_uu * 0.5);
        let ring_sides = 12;

        let mut ring_brush = Brush::new(*name, CsgOperation::Add);
        ring_brush.location = p;

        for seg in 0..ring_sides {
            let a0 = (seg as f32) * (std::f32::consts::TAU / ring_sides as f32);
            let a1 = ((seg + 1) as f32) * (std::f32::consts::TAU / ring_sides as f32);

            let p_in0 = Vec3::new(a0.cos() * r_in, a0.sin() * r_in, -h * 0.5);
            let p_in1 = Vec3::new(a1.cos() * r_in, a1.sin() * r_in, -h * 0.5);
            let p_out0 = Vec3::new(a0.cos() * r_out, a0.sin() * r_out, -h * 0.5);
            let p_out1 = Vec3::new(a1.cos() * r_out, a1.sin() * r_out, -h * 0.5);

            let t_in0 = p_in0 + Vec3::new(0.0, 0.0, *h);
            let t_in1 = p_in1 + Vec3::new(0.0, 0.0, *h);
            let t_out0 = p_out0 + Vec3::new(0.0, 0.0, *h);
            let t_out1 = p_out1 + Vec3::new(0.0, 0.0, *h);

            // Top quad
            if let Some(mut poly) = Polygon::new(vec![t_in0, t_in1, t_out1, t_out0]) {
                poly.texture = tex_ring.to_string();
                poly.flags = crate::PF_SEMISOLID;
                ring_brush.polys.push(poly);
            }
            // Bottom quad
            if let Some(mut poly) = Polygon::new(vec![p_out0, p_out1, p_in1, p_in0]) {
                poly.texture = tex_ring.to_string();
                poly.flags = crate::PF_SEMISOLID;
                ring_brush.polys.push(poly);
            }
            // Outer wall
            if let Some(mut poly) = Polygon::new(vec![p_out0, t_out0, t_out1, p_out1]) {
                poly.texture = tex_ring.to_string();
                poly.flags = crate::PF_SEMISOLID;
                ring_brush.polys.push(poly);
            }
            // Inner wall
            if let Some(mut poly) = Polygon::new(vec![p_in1, t_in1, t_in0, p_in0]) {
                poly.texture = tex_ring.to_string();
                poly.flags = crate::PF_SEMISOLID;
                ring_brush.polys.push(poly);
            }
        }

        ring_brushes.push(ring_brush);
    }

    (core_brush, ring_brushes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reactor_core_archetypes_and_defaults() {
        let archetypes = [
            ReactorCoreArchetype::LiandriCentrifugalCore,
            ReactorCoreArchetype::TurbineVentilationRotor,
            ReactorCoreArchetype::PhobosQuantumGyroscope,
            ReactorCoreArchetype::NivenNuclearContainmentVessel,
            ReactorCoreArchetype::SkaarjMothershipGravitonNexus,
        ];

        for a in &archetypes {
            let cfg = a.to_config();
            assert_eq!(cfg.archetype, *a);
            assert!(cfg.core_radius_uu > 0.0);
            assert!(cfg.outer_ring_radius_uu > cfg.mid_ring_radius_uu);
            let metrics = evaluate_reactor_core_metrics(&cfg, &[]);
            assert!(metrics.core_volume_uu3 > 0.0);
            assert!(metrics.outer_rim_tangential_speed_uus > 0.0);
            assert!(metrics.centrifugal_g_force >= 0.0);
            assert_eq!(metrics.total_reactors_in_level, 1);
        }
    }

    #[test]
    fn test_reactor_core_brush_generation() {
        let cfg = ReactorCoreArchetype::LiandriCentrifugalCore.to_config();
        let (core, rings) = generate_reactor_core_brushes(&cfg);

        // Core: 8 sides + top + bottom = 10 polygons
        assert_eq!(core.polys.len(), 10);
        // 2 rings: Outer and Mid
        assert_eq!(rings.len(), 2);
        for ring in &rings {
            assert!(ring.polys.len() >= 48); // 12 segments * 4 quads
        }
    }

    #[test]
    fn test_reactor_core_safety_audits() {
        let mut cfg = ReactorCoreArchetype::LiandriCentrifugalCore.to_config();
        cfg.position = Vec3::new(0.0, 0.0, 0.0);
        cfg.hazard_radius_uu = 256.0;

        let spawn_near = LevelReactorContextActorRecord {
            name: "PlayerStart0".to_string(),
            class_name: "Engine.PlayerStart".to_string(),
            position: Vec3::new(100.0, 0.0, 0.0), // Inside 256 uu hazard
        };

        let hazards = audit_reactor_core_safety(&cfg, &[spawn_near]);
        assert!(hazards.iter().any(|h| h.title == "Lethal Radiation Encroaching Spawn Point" && h.is_critical));
    }
}
