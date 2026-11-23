// crates/ut99-bsp/src/ventilator_fan.rs
//
// Interactive Environmental Ventilation Fan, Air Current & Wind Tunnel Studio
//
// Mathematical and physical model for multi-blade rotating industrial fans, aerodynamic lift
// and air drag velocity zones, procedural hub and blade generation with pitch angle,
// velocity streamlines, and blade hazard contact auditing.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Classical Unreal Tournament & Unreal 1 industrial ventilation fan archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VentilatorArchetype {
    IndustrialCeilingExhaust,
    HighSpeedIntakeTurbine,
    VerticalUpdraftLiftShaft,
    BiohazardCoolingBlower,
    LethalGrinderVentilator,
}

impl VentilatorArchetype {
    pub fn all() -> &'static [VentilatorArchetype] {
        &[
            VentilatorArchetype::IndustrialCeilingExhaust,
            VentilatorArchetype::HighSpeedIntakeTurbine,
            VentilatorArchetype::VerticalUpdraftLiftShaft,
            VentilatorArchetype::BiohazardCoolingBlower,
            VentilatorArchetype::LethalGrinderVentilator,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            VentilatorArchetype::IndustrialCeilingExhaust => "Industrial Ceiling Exhaust Fan (60 RPM)",
            VentilatorArchetype::HighSpeedIntakeTurbine => "High-Speed Jet Intake Turbine (300 RPM)",
            VentilatorArchetype::VerticalUpdraftLiftShaft => "Vertical Updraft Elevator Shaft (950 uu/s Lift)",
            VentilatorArchetype::BiohazardCoolingBlower => "Biohazard Cooling Blower with Mist",
            VentilatorArchetype::LethalGrinderVentilator => "Subterranean Lethal Grinder Fan (Instagib)",
        }
    }

    pub fn default_config(&self) -> VentilatorConfig {
        match self {
            VentilatorArchetype::IndustrialCeilingExhaust => VentilatorConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 256.0),
                rotation_axis: Vec3::Z,
                spin_rpm: 60.0,
                geometry: FanGeometryConfig {
                    hub_radius: 16.0,
                    hub_length: 24.0,
                    blade_count: 4,
                    blade_length: 96.0,
                    blade_width: 24.0,
                    blade_pitch_deg: 25.0,
                    blade_thickness: 4.0,
                    has_outer_ring: true,
                    ring_thickness: 8.0,
                },
                air_current: AirCurrentConfig {
                    flow_direction: AirFlowDirection::Blowing,
                    zone_velocity_strength: 350.0,
                    effective_range: 384.0,
                    duct_radius: 112.0,
                },
                lethality: FanLethalityConfig {
                    is_lethal_contact: false,
                    blade_damage_per_sec: 25.0,
                    damage_type: "BluntBlow".to_string(),
                },
                fan_texture: "Indus.Metal.FanBlade01".to_string(),
                housing_texture: "Indus.Metal.DuctPlate01".to_string(),
                ambient_sound: "GenAmb.Fan2".to_string(),
            },
            VentilatorArchetype::HighSpeedIntakeTurbine => VentilatorConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 128.0),
                rotation_axis: Vec3::X,
                spin_rpm: 240.0,
                geometry: FanGeometryConfig {
                    hub_radius: 20.0,
                    hub_length: 32.0,
                    blade_count: 8,
                    blade_length: 80.0,
                    blade_width: 20.0,
                    blade_pitch_deg: 35.0,
                    blade_thickness: 3.0,
                    has_outer_ring: true,
                    ring_thickness: 12.0,
                },
                air_current: AirCurrentConfig {
                    flow_direction: AirFlowDirection::Sucking,
                    zone_velocity_strength: 650.0,
                    effective_range: 512.0,
                    duct_radius: 100.0,
                },
                lethality: FanLethalityConfig {
                    is_lethal_contact: true,
                    blade_damage_per_sec: 500.0,
                    damage_type: "TurbineShred".to_string(),
                },
                fan_texture: "Indus.Metal.SteelBlade02".to_string(),
                housing_texture: "Indus.Metal.TurbineCasing01".to_string(),
                ambient_sound: "AmbModern.Wind1".to_string(),
            },
            VentilatorArchetype::VerticalUpdraftLiftShaft => VentilatorConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 32.0),
                rotation_axis: Vec3::Z,
                spin_rpm: 120.0,
                geometry: FanGeometryConfig {
                    hub_radius: 24.0,
                    hub_length: 28.0,
                    blade_count: 6,
                    blade_length: 120.0,
                    blade_width: 32.0,
                    blade_pitch_deg: 30.0,
                    blade_thickness: 6.0,
                    has_outer_ring: true,
                    ring_thickness: 16.0,
                },
                air_current: AirCurrentConfig {
                    flow_direction: AirFlowDirection::Blowing,
                    zone_velocity_strength: 1050.0, // Overcomes UT99 gravity (-950 uu/s^2)
                    effective_range: 640.0,
                    duct_radius: 144.0,
                },
                lethality: FanLethalityConfig {
                    is_lethal_contact: false,
                    blade_damage_per_sec: 0.0,
                    damage_type: "None".to_string(),
                },
                fan_texture: "Indus.Metal.LiftFan01".to_string(),
                housing_texture: "Indus.Metal.ShaftWall01".to_string(),
                ambient_sound: "GenAmb.UpdraftLoop01".to_string(),
            },
            VentilatorArchetype::BiohazardCoolingBlower => VentilatorConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 160.0),
                rotation_axis: Vec3::Y,
                spin_rpm: 90.0,
                geometry: FanGeometryConfig {
                    hub_radius: 18.0,
                    hub_length: 30.0,
                    blade_count: 12,
                    blade_length: 88.0,
                    blade_width: 18.0,
                    blade_pitch_deg: 20.0,
                    blade_thickness: 3.5,
                    has_outer_ring: true,
                    ring_thickness: 10.0,
                },
                air_current: AirCurrentConfig {
                    flow_direction: AirFlowDirection::Blowing,
                    zone_velocity_strength: 420.0,
                    effective_range: 400.0,
                    duct_radius: 108.0,
                },
                lethality: FanLethalityConfig {
                    is_lethal_contact: false,
                    blade_damage_per_sec: 15.0,
                    damage_type: "CorrosiveGas".to_string(),
                },
                fan_texture: "Indus.Metal.GreenComposite01".to_string(),
                housing_texture: "Indus.Metal.BioFilter01".to_string(),
                ambient_sound: "GenAmb.GasBlower01".to_string(),
            },
            VentilatorArchetype::LethalGrinderVentilator => VentilatorConfig {
                archetype: *self,
                position: Vec3::new(0.0, 0.0, 64.0),
                rotation_axis: Vec3::Z,
                spin_rpm: 180.0,
                geometry: FanGeometryConfig {
                    hub_radius: 28.0,
                    hub_length: 36.0,
                    blade_count: 3,
                    blade_length: 140.0,
                    blade_width: 44.0,
                    blade_pitch_deg: 40.0,
                    blade_thickness: 8.0,
                    has_outer_ring: true,
                    ring_thickness: 20.0,
                },
                air_current: AirCurrentConfig {
                    flow_direction: AirFlowDirection::Sucking,
                    zone_velocity_strength: 550.0,
                    effective_range: 480.0,
                    duct_radius: 168.0,
                },
                lethality: FanLethalityConfig {
                    is_lethal_contact: true,
                    blade_damage_per_sec: 1000.0, // Instagib lethal
                    damage_type: "Shredded".to_string(),
                },
                fan_texture: "Indus.Metal.HeavyGrinder01".to_string(),
                housing_texture: "Indus.Metal.HazardStripes01".to_string(),
                ambient_sound: "GenAmb.MeatGrinder01".to_string(),
            },
        }
    }
}

/// Air flow direction relative to the fan rotation axis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AirFlowDirection {
    Blowing,
    Sucking,
    Neutral,
}

impl AirFlowDirection {
    pub fn all() -> &'static [AirFlowDirection] {
        &[
            AirFlowDirection::Blowing,
            AirFlowDirection::Sucking,
            AirFlowDirection::Neutral,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AirFlowDirection::Blowing => "Blowing (Pushes Away)",
            AirFlowDirection::Sucking => "Sucking (Pulls Inward)",
            AirFlowDirection::Neutral => "Neutral (Visual Only)",
        }
    }
}

/// Fan geometry dimensions and blade configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FanGeometryConfig {
    pub hub_radius: f32,
    pub hub_length: f32,
    pub blade_count: usize,
    pub blade_length: f32,
    pub blade_width: f32,
    pub blade_pitch_deg: f32,
    pub blade_thickness: f32,
    pub has_outer_ring: bool,
    pub ring_thickness: f32,
}

/// ZoneVelocity air current and duct bounds
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AirCurrentConfig {
    pub flow_direction: AirFlowDirection,
    pub zone_velocity_strength: f32,
    pub effective_range: f32,
    pub duct_radius: f32,
}

/// Lethality and damage parameters for rotating blades
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FanLethalityConfig {
    pub is_lethal_contact: bool,
    pub blade_damage_per_sec: f32,
    pub damage_type: String,
}

/// Complete configuration for Environmental Ventilation Fan Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VentilatorConfig {
    pub archetype: VentilatorArchetype,
    pub position: Vec3,
    pub rotation_axis: Vec3,
    pub spin_rpm: f32,
    pub geometry: FanGeometryConfig,
    pub air_current: AirCurrentConfig,
    pub lethality: FanLethalityConfig,
    pub fan_texture: String,
    pub housing_texture: String,
    pub ambient_sound: String,
}

impl Default for VentilatorConfig {
    fn default() -> Self {
        VentilatorArchetype::IndustrialCeilingExhaust.default_config()
    }
}

/// Aerodynamic lift profile and terminal velocity calculation
#[derive(Debug, Clone, PartialEq)]
pub struct AerodynamicLiftProfile {
    pub net_vertical_lift: f32,
    pub terminal_velocity: f32,
    pub equilibrium_hover_height: f32,
    pub can_lift_player: bool,
}

/// Dynamic streamline particle for visualizing airflow in 2D and 3D
#[derive(Debug, Clone, PartialEq)]
pub struct WindStreamline {
    pub start_point: Vec3,
    pub end_point: Vec3,
    pub velocity: Vec3,
    pub speed: f32,
    pub life_fraction: f32,
}

/// Diagnostic hazard detected during fan safety audit
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VentilatorHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Calculates aerodynamic lift, player buoyancy against gravity ($g = 950\text{ uu/s}^2$)
pub fn calculate_aerodynamic_lift(config: &VentilatorConfig) -> AerodynamicLiftProfile {
    let ut99_gravity = 950.0_f32; // Standard UT99 gravity
    let flow_sign = match config.air_current.flow_direction {
        AirFlowDirection::Blowing => 1.0,
        AirFlowDirection::Sucking => -1.0,
        AirFlowDirection::Neutral => 0.0,
    };

    let axis_z = config.rotation_axis.normalize_or_zero().z;
    let vertical_velocity = config.air_current.zone_velocity_strength * flow_sign * axis_z;

    let can_lift = vertical_velocity > ut99_gravity;
    let net_lift = vertical_velocity - ut99_gravity;

    // Terminal velocity capped by drag
    let terminal_vel = if can_lift { net_lift.min(1200.0) } else { 0.0 };

    // Estimated hover height where air velocity drops off with distance
    let hover_height = if can_lift {
        let ratio = ut99_gravity / vertical_velocity.max(1.0);
        config.air_current.effective_range * (1.0 - ratio.powf(0.5)).clamp(0.1, 0.95)
    } else {
        0.0
    };

    AerodynamicLiftProfile {
        net_vertical_lift: net_lift,
        terminal_velocity: terminal_vel,
        equilibrium_hover_height: hover_height,
        can_lift_player: can_lift,
    }
}

/// Generates wind streamline line segments along the air current duct
pub fn generate_wind_streamlines(config: &VentilatorConfig, time_sec: f32) -> Vec<WindStreamline> {
    let mut streamlines = Vec::new();
    let flow_dir = match config.air_current.flow_direction {
        AirFlowDirection::Blowing => config.rotation_axis.normalize_or_zero(),
        AirFlowDirection::Sucking => -config.rotation_axis.normalize_or_zero(),
        AirFlowDirection::Neutral => return streamlines,
    };

    let radial_axis = if flow_dir.z.abs() < 0.9 {
        Vec3::Z.cross(flow_dir).normalize()
    } else {
        Vec3::X.cross(flow_dir).normalize()
    };
    let binormal = flow_dir.cross(radial_axis).normalize();

    let stream_count = 12;
    let range = config.air_current.effective_range;
    let radius = config.air_current.duct_radius * 0.7;

    for i in 0..stream_count {
        let angle = (i as f32 / stream_count as f32) * std::f32::consts::TAU;
        let r = radius * (0.3 + 0.7 * ((i % 3) as f32 / 2.0));
        let ring_offset = radial_axis * (angle.cos() * r) + binormal * (angle.sin() * r);

        let speed = config.air_current.zone_velocity_strength;
        let flow_phase = (time_sec * (speed / range) + i as f32 * 0.15).fract();

        let dist_along = flow_phase * range;
        let seg_len = 32.0;

        let start = config.position + ring_offset + flow_dir * dist_along;
        let end = start + flow_dir * seg_len;

        streamlines.push(WindStreamline {
            start_point: start,
            end_point: end,
            velocity: flow_dir * speed,
            speed,
            life_fraction: flow_phase,
        });
    }

    streamlines
}

/// Generates procedural 3D polygonal geometry:
/// 1. Rotating Fan Mover Brush (hub cylinder + angled blades)
/// 2. Static Housing Brush (outer duct ring + mounting struts)
pub fn generate_ventilator_brush(
    config: &VentilatorConfig,
    time_sec: f32,
) -> (Brush, Brush) {
    let mut fan_brush = Brush::new("Fan_Rotator", CsgOperation::Add);
    let mut housing_brush = Brush::new("Fan_Housing", CsgOperation::Add);

    let axis = config.rotation_axis.normalize_or_zero();
    let rot_deg = (time_sec * (config.spin_rpm * 6.0)) % 360.0;
    let spin_quat = Quat::from_axis_angle(axis, rot_deg.to_radians());

    let (u_axis, v_axis) = if axis.z.abs() < 0.9 {
        let u = Vec3::Z.cross(axis).normalize();
        let v = axis.cross(u).normalize();
        (u, v)
    } else {
        let u = Vec3::X.cross(axis).normalize();
        let v = axis.cross(u).normalize();
        (u, v)
    };

    // 1. Generate Hub Cylinder (8-sided prism)
    let hub_sides = 8;
    let hr = config.geometry.hub_radius;
    let hl = config.geometry.hub_length * 0.5;

    let mut top_rim = Vec::new();
    let mut bot_rim = Vec::new();

    for i in 0..hub_sides {
        let a = (i as f32 / hub_sides as f32) * std::f32::consts::TAU;
        let radial = spin_quat * (u_axis * a.cos() + v_axis * a.sin());
        let pt_top = config.position + axis * hl + radial * hr;
        let pt_bot = config.position - axis * hl + radial * hr;
        top_rim.push(pt_top);
        bot_rim.push(pt_bot);
    }

    // Hub side faces
    for i in 0..hub_sides {
        let next = (i + 1) % hub_sides;
        let face = vec![bot_rim[i], bot_rim[next], top_rim[next], top_rim[i]];
        if let Some(mut poly) = Polygon::new(face) {
            poly.texture = config.fan_texture.clone();
            poly.flags |= 0x00200000; // PF_SEMISOLID
            fan_brush.polys.push(poly);
        }
    }
    // Hub caps
    if let Some(mut poly) = Polygon::new(top_rim.clone()) {
        poly.texture = config.fan_texture.clone();
        poly.flags |= 0x00200000;
        fan_brush.polys.push(poly);
    }
    let mut bot_rev = bot_rim.clone();
    bot_rev.reverse();
    if let Some(mut poly) = Polygon::new(bot_rev) {
        poly.texture = config.fan_texture.clone();
        poly.flags |= 0x00200000;
        fan_brush.polys.push(poly);
    }

    // 2. Generate Angled Fan Blades
    let count = config.geometry.blade_count.max(2);
    let bl = config.geometry.blade_length;
    let bw = config.geometry.blade_width * 0.5;
    let bt = config.geometry.blade_thickness * 0.5;
    let pitch_rad = config.geometry.blade_pitch_deg.to_radians();

    for b in 0..count {
        let blade_angle = (b as f32 / count as f32) * std::f32::consts::TAU;
        let blade_dir = u_axis * blade_angle.cos() + v_axis * blade_angle.sin();
        let blade_tan = axis.cross(blade_dir).normalize();

        // Local blade orientation with pitch
        let blade_pitch_quat = Quat::from_axis_angle(blade_dir, pitch_rad);
        let normal_tilted = blade_pitch_quat * axis;
        let tan_tilted = blade_pitch_quat * blade_tan;

        let b_start = hr;
        let b_end = hr + bl;

        let corners = [
            // Inner bottom, inner top, outer top, outer bottom
            -tan_tilted * bw - normal_tilted * bt,
            tan_tilted * bw - normal_tilted * bt,
            tan_tilted * bw + normal_tilted * bt,
            -tan_tilted * bw + normal_tilted * bt,
        ];

        let mut pts_inner = Vec::new();
        let mut pts_outer = Vec::new();

        for c in &corners {
            let p_in = config.position + blade_dir * b_start + *c;
            let p_out = config.position + blade_dir * b_end + *c;
            pts_inner.push(config.position + spin_quat * (p_in - config.position));
            pts_outer.push(config.position + spin_quat * (p_out - config.position));
        }

        // Blade front & back
        let front = vec![pts_inner[0], pts_outer[0], pts_outer[1], pts_inner[1]];
        let back = vec![pts_inner[2], pts_outer[2], pts_outer[3], pts_inner[3]];
        let top = vec![pts_inner[1], pts_outer[1], pts_outer[2], pts_inner[2]];
        let bottom = vec![pts_inner[3], pts_outer[3], pts_outer[0], pts_inner[0]];
        let tip = vec![pts_outer[0], pts_outer[3], pts_outer[2], pts_outer[1]];

        for face in [front, back, top, bottom, tip] {
            if let Some(mut poly) = Polygon::new(face) {
                poly.texture = config.fan_texture.clone();
                poly.flags |= 0x00200000; // PF_SEMISOLID
                fan_brush.polys.push(poly);
            }
        }
    }

    // 3. Generate Outer Housing Duct Ring (12-sided prism ring)
    if config.geometry.has_outer_ring {
        let ring_sides = 12;
        let inner_r = hr + bl + 2.0;
        let outer_r = inner_r + config.geometry.ring_thickness;
        let rh = config.geometry.hub_length * 0.6;

        let mut ring_inner_top = Vec::new();
        let mut ring_inner_bot = Vec::new();
        let mut ring_outer_top = Vec::new();
        let mut ring_outer_bot = Vec::new();

        for i in 0..ring_sides {
            let a = (i as f32 / ring_sides as f32) * std::f32::consts::TAU;
            let dir = u_axis * a.cos() + v_axis * a.sin();

            ring_inner_top.push(config.position + axis * rh + dir * inner_r);
            ring_inner_bot.push(config.position - axis * rh + dir * inner_r);
            ring_outer_top.push(config.position + axis * rh + dir * outer_r);
            ring_outer_bot.push(config.position - axis * rh + dir * outer_r);
        }

        for i in 0..ring_sides {
            let next = (i + 1) % ring_sides;

            // Outer cylinder face
            let out_face = vec![ring_outer_bot[i], ring_outer_bot[next], ring_outer_top[next], ring_outer_top[i]];
            // Inner cylinder face
            let in_face = vec![ring_inner_top[i], ring_inner_top[next], ring_inner_bot[next], ring_inner_bot[i]];
            // Top rim
            let top_face = vec![ring_inner_top[i], ring_outer_top[i], ring_outer_top[next], ring_inner_top[next]];
            // Bottom rim
            let bot_face = vec![ring_outer_bot[i], ring_inner_bot[i], ring_inner_bot[next], ring_outer_bot[next]];

            for face in [out_face, in_face, top_face, bot_face] {
                if let Some(mut poly) = Polygon::new(face) {
                    poly.texture = config.housing_texture.clone();
                    poly.flags |= 0x00200000; // PF_SEMISOLID
                    housing_brush.polys.push(poly);
                }
            }
        }
    }

    (fan_brush, housing_brush)
}

/// Audits ventilator fan and air current configuration against safety hazards
pub fn audit_ventilator_safety(
    config: &VentilatorConfig,
    _world_brushes: &[Brush],
) -> Vec<VentilatorHazard> {
    let mut hazards = Vec::new();

    let lift = calculate_aerodynamic_lift(config);

    // 1. Vertical updraft with insufficient lift
    let axis_z = config.rotation_axis.normalize_or_zero().z;
    if axis_z > 0.8 && config.air_current.flow_direction == AirFlowDirection::Blowing && !lift.can_lift_player {
        hazards.push(VentilatorHazard {
            is_critical: false,
            title: format!("Insufficient Updraft Lift ({:.0} uu/s)", config.air_current.zone_velocity_strength),
            description: "Updraft velocity is lower than UT99 gravity (950 uu/s^2). Players will not be lifted up the shaft.".to_string(),
            recommendation: "Increase ZoneVelocity strength to at least 1000..1200 uu/s.".to_string(),
        });
    }

    // 2. Violent excessive air blast (>2500 uu/s)
    if config.air_current.zone_velocity_strength > 2500.0 {
        hazards.push(VentilatorHazard {
            is_critical: true,
            title: format!("Excessive Air Blast Velocity ({:.0} uu/s)", config.air_current.zone_velocity_strength),
            description: "Velocity exceeds 2500 uu/s. Players entering the air current will be launched at glitching speeds through ceiling geometry.".to_string(),
            recommendation: "Reduce velocity strength to 600..1500 uu/s.".to_string(),
        });
    }

    // 3. High-RPM fan without lethal contact trigger
    if config.spin_rpm >= 180.0 && !config.lethality.is_lethal_contact {
        hazards.push(VentilatorHazard {
            is_critical: false,
            title: format!("High-Speed Fan ({:.0} RPM) Missing Lethal Contact", config.spin_rpm),
            description: "High-RPM turbine blades allow players to touch them without fatal damage, causing immersion breakage and mover clipping.".to_string(),
            recommendation: "Enable Lethal Blade Contact with instagib damage or add a protective safety grille.".to_string(),
        });
    }

    // 4. Zero effective duct range
    if config.air_current.effective_range < 64.0 {
        hazards.push(VentilatorHazard {
            is_critical: true,
            title: "Trunkated Air Current Range (<64 uu)".to_string(),
            description: "Air current effective range is too short for players to feel any aerodynamic force.".to_string(),
            recommendation: "Increase effective range to at least 256..512 uu.".to_string(),
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ventilator_archetypes_and_defaults() {
        for &arch in VentilatorArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.spin_rpm > 0.0);
            assert!(cfg.geometry.blade_count >= 3);
            assert!(cfg.geometry.blade_length > 50.0);
            assert!(!cfg.fan_texture.is_empty());
        }
    }

    #[test]
    fn test_aerodynamic_lift_calculation() {
        let mut cfg = VentilatorArchetype::VerticalUpdraftLiftShaft.default_config();
        cfg.air_current.zone_velocity_strength = 1100.0;
        let lift = calculate_aerodynamic_lift(&cfg);

        assert!(lift.can_lift_player);
        assert!(lift.net_vertical_lift > 0.0);
        assert!(lift.equilibrium_hover_height > 0.0);

        // Insufficient lift test
        cfg.air_current.zone_velocity_strength = 600.0;
        let weak_lift = calculate_aerodynamic_lift(&cfg);
        assert!(!weak_lift.can_lift_player);
        assert!(weak_lift.net_vertical_lift < 0.0);
    }

    #[test]
    fn test_generate_ventilator_brush_polygons() {
        let cfg = VentilatorArchetype::IndustrialCeilingExhaust.default_config();
        let (fan_b, house_b) = generate_ventilator_brush(&cfg, 0.0);

        // Hub (8 sides + 2 caps = 10) + 4 blades * 5 faces = 30 polys
        assert_eq!(fan_b.polys.len(), 30);
        // Outer ring has 12 sides * 4 faces = 48 polys
        assert_eq!(house_b.polys.len(), 48);

        // All polys must carry PF_SEMISOLID
        assert!(fan_b.polys.iter().all(|p| p.flags & 0x00200000 != 0));
        assert!(house_b.polys.iter().all(|p| p.flags & 0x00200000 != 0));
    }

    #[test]
    fn test_ventilator_safety_audit() {
        let mut cfg = VentilatorConfig::default();
        cfg.air_current.zone_velocity_strength = 3000.0; // Excessive
        cfg.air_current.effective_range = 30.0; // Short

        let hazards = audit_ventilator_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Excessive")));
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Trunkated")));
    }
}
