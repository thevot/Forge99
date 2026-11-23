// crates/ut99-bsp/src/track_mover.rs
//
// Interactive Spline Rail, Kinetic Track Mover & Rollercoaster Studio
//
// Mathematical model for generating continuous dual rails, cross-tie sleepers,
// physical banking angle computation, mover keyframe baking (Key0..Key7),
// bot AI station boarding path generation, and kinetic safety auditing.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Classical Unreal Tournament & Unreal 1 kinetic track archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackArchetype {
    IndustrialMinecartRail,
    HighSpeedBulletTrain,
    SuspendedGondolaCable,
    RollercoasterLoopThrill,
    SubterraneanSubwayMetro,
}

impl TrackArchetype {
    pub fn all() -> &'static [TrackArchetype] {
        &[
            TrackArchetype::IndustrialMinecartRail,
            TrackArchetype::HighSpeedBulletTrain,
            TrackArchetype::SuspendedGondolaCable,
            TrackArchetype::RollercoasterLoopThrill,
            TrackArchetype::SubterraneanSubwayMetro,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TrackArchetype::IndustrialMinecartRail => "Industrial Minecart & Ore Trestle",
            TrackArchetype::HighSpeedBulletTrain => "High-Speed Bullet Train (AS-HiSpeed)",
            TrackArchetype::SuspendedGondolaCable => "Suspended Aerial Gondola & Cableway",
            TrackArchetype::RollercoasterLoopThrill => "Rollercoaster Dynamic Banking Track",
            TrackArchetype::SubterraneanSubwayMetro => "Subterranean Subway & Third Rail",
        }
    }

    pub fn default_config(&self) -> TrackConfig {
        match self {
            TrackArchetype::IndustrialMinecartRail => TrackConfig {
                archetype: *self,
                points: vec![
                    TrackPoint::new(Vec3::new(0.0, 0.0, 0.0), 300.0, 2.0),
                    TrackPoint::new(Vec3::new(384.0, 256.0, 64.0), 450.0, 0.0),
                    TrackPoint::new(Vec3::new(896.0, 512.0, 128.0), 550.0, 0.0),
                    TrackPoint::new(Vec3::new(1408.0, 256.0, 64.0), 450.0, 0.0),
                    TrackPoint::new(Vec3::new(1792.0, 0.0, 0.0), 300.0, 2.0),
                ],
                is_closed_loop: false,
                gauge_width: 56.0,
                rail_thickness: 6.0,
                tie_spacing: 48.0,
                tie_width: 14.0,
                tie_height: 10.0,
                tie_length: 84.0,
                max_bank_angle_deg: 25.0,
                default_speed: 400.0,
                keyframe_count: 5,
                add_support_pylons: true,
                cart_dimensions: Vec3::new(64.0, 96.0, 48.0),
                rail_texture: "Mine.Rails.IronRail01".to_string(),
                tie_texture: "Mine.Wood.RusticTie01".to_string(),
            },
            TrackArchetype::HighSpeedBulletTrain => TrackConfig {
                archetype: *self,
                points: vec![
                    TrackPoint::new(Vec3::new(0.0, 0.0, 0.0), 800.0, 3.0),
                    TrackPoint::new(Vec3::new(768.0, 512.0, 0.0), 1200.0, 0.0),
                    TrackPoint::new(Vec3::new(1792.0, 1024.0, 0.0), 1400.0, 0.0),
                    TrackPoint::new(Vec3::new(2816.0, 512.0, 0.0), 1200.0, 0.0),
                    TrackPoint::new(Vec3::new(3584.0, 0.0, 0.0), 800.0, 3.0),
                ],
                is_closed_loop: false,
                gauge_width: 80.0,
                rail_thickness: 10.0,
                tie_spacing: 64.0,
                tie_width: 18.0,
                tie_height: 12.0,
                tie_length: 120.0,
                max_bank_angle_deg: 35.0,
                default_speed: 1200.0,
                keyframe_count: 6,
                add_support_pylons: true,
                cart_dimensions: Vec3::new(96.0, 256.0, 80.0),
                rail_texture: "Indus.Metal.SteelRail01".to_string(),
                tie_texture: "Indus.Concrete.SlabTie01".to_string(),
            },
            TrackArchetype::SuspendedGondolaCable => TrackConfig {
                archetype: *self,
                points: vec![
                    TrackPoint::new(Vec3::new(0.0, 0.0, 512.0), 250.0, 4.0),
                    TrackPoint::new(Vec3::new(640.0, 256.0, 640.0), 350.0, 0.0),
                    TrackPoint::new(Vec3::new(1408.0, 128.0, 576.0), 350.0, 0.0),
                    TrackPoint::new(Vec3::new(2048.0, 0.0, 768.0), 250.0, 4.0),
                ],
                is_closed_loop: false,
                gauge_width: 24.0,
                rail_thickness: 8.0,
                tie_spacing: 96.0,
                tie_width: 8.0,
                tie_height: 8.0,
                tie_length: 36.0,
                max_bank_angle_deg: 18.0,
                default_speed: 300.0,
                keyframe_count: 4,
                add_support_pylons: false,
                cart_dimensions: Vec3::new(80.0, 80.0, 96.0),
                rail_texture: "GenFX.Cable.SteelWire01".to_string(),
                tie_texture: "Indus.Metal.CableClamp01".to_string(),
            },
            TrackArchetype::RollercoasterLoopThrill => TrackConfig {
                archetype: *self,
                points: vec![
                    TrackPoint::new(Vec3::new(0.0, 0.0, 0.0), 600.0, 0.0),
                    TrackPoint::new(Vec3::new(512.0, 512.0, 256.0), 900.0, 0.0),
                    TrackPoint::new(Vec3::new(1024.0, 256.0, 512.0), 500.0, 0.0),
                    TrackPoint::new(Vec3::new(1536.0, -256.0, 128.0), 1100.0, 0.0),
                    TrackPoint::new(Vec3::new(2048.0, 0.0, 0.0), 700.0, 0.0),
                ],
                is_closed_loop: false,
                gauge_width: 64.0,
                rail_thickness: 8.0,
                tie_spacing: 36.0,
                tie_width: 12.0,
                tie_height: 8.0,
                tie_length: 92.0,
                max_bank_angle_deg: 55.0,
                default_speed: 800.0,
                keyframe_count: 7,
                add_support_pylons: true,
                cart_dimensions: Vec3::new(64.0, 128.0, 48.0),
                rail_texture: "Indus.Metal.TubularPipe01".to_string(),
                tie_texture: "Indus.Metal.SteelTruss01".to_string(),
            },
            TrackArchetype::SubterraneanSubwayMetro => TrackConfig {
                archetype: *self,
                points: vec![
                    TrackPoint::new(Vec3::new(0.0, 0.0, 0.0), 400.0, 3.0),
                    TrackPoint::new(Vec3::new(640.0, 128.0, 0.0), 750.0, 0.0),
                    TrackPoint::new(Vec3::new(1536.0, -128.0, 0.0), 850.0, 0.0),
                    TrackPoint::new(Vec3::new(2304.0, 0.0, 0.0), 400.0, 3.0),
                ],
                is_closed_loop: false,
                gauge_width: 72.0,
                rail_thickness: 10.0,
                tie_spacing: 52.0,
                tie_width: 16.0,
                tie_height: 12.0,
                tie_length: 110.0,
                max_bank_angle_deg: 20.0,
                default_speed: 650.0,
                keyframe_count: 5,
                add_support_pylons: false,
                cart_dimensions: Vec3::new(88.0, 220.0, 72.0),
                rail_texture: "Metro.Rail.HeavyTrack01".to_string(),
                tie_texture: "Metro.Wood.CreosoteTie01".to_string(),
            },
        }
    }
}

/// A control station waypoint along the track spline
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackPoint {
    pub position: Vec3,
    pub speed: f32,
    pub station_pause_seconds: f32,
}

impl TrackPoint {
    pub fn new(position: Vec3, speed: f32, station_pause_seconds: f32) -> Self {
        Self {
            position,
            speed,
            station_pause_seconds,
        }
    }
}

/// Complete configuration for the Spline Rail & Track Mover Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackConfig {
    pub archetype: TrackArchetype,
    pub points: Vec<TrackPoint>,
    pub is_closed_loop: bool,
    pub gauge_width: f32,
    pub rail_thickness: f32,
    pub tie_spacing: f32,
    pub tie_width: f32,
    pub tie_height: f32,
    pub tie_length: f32,
    pub max_bank_angle_deg: f32,
    pub default_speed: f32,
    pub keyframe_count: usize,
    pub add_support_pylons: bool,
    pub cart_dimensions: Vec3,
    pub rail_texture: String,
    pub tie_texture: String,
}

impl Default for TrackConfig {
    fn default() -> Self {
        TrackArchetype::IndustrialMinecartRail.default_config()
    }
}

/// Evaluated sample along the track spline at parameter t in [0..1]
#[derive(Debug, Clone, PartialEq)]
pub struct TrackSample {
    pub position: Vec3,
    pub tangent: Vec3,
    pub normal: Vec3,
    pub binormal: Vec3,
    pub bank_angle_deg: f32,
    pub curvature: f32,
    pub speed: f32,
    pub rotation_deg: Vec3, // Pitch, Yaw, Roll
}

/// A baked Unreal Tournament 99 mover keyframe (Key0..Key7)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoverKeyframeRecord {
    pub key_index: usize,
    pub position: Vec3,
    pub rotation_deg: Vec3, // Pitch, Yaw, Roll
    pub unreal_rotation: (i32, i32, i32), // Pitch, Yaw, Roll in 0..65535 units
    pub move_time: f32,
    pub is_station_stop: bool,
}

/// A bot navigation station boarding waypoint (LiftCenter / LiftExit)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StationWaypoint {
    pub name: String,
    pub class_name: String,
    pub location: Vec3,
    pub is_cart_mounted: bool,
}

/// Kinetic safety hazard flagged during track analysis
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Evaluates a cubic Catmull-Rom spline position
pub fn catmull_rom_eval(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// Evaluates Catmull-Rom first derivative (velocity / tangent vector)
pub fn catmull_rom_derivative(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    0.5 * ((-p0 + p2)
        + 2.0 * (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t
        + 3.0 * (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t2)
}

/// Samples the track spline at global parameter t in [0.0..1.0]
pub fn sample_track_spline(config: &TrackConfig, t: f32) -> TrackSample {
    let pts = &config.points;
    let n = pts.len();
    if n < 2 {
        return TrackSample {
            position: pts.first().map(|p| p.position).unwrap_or(Vec3::ZERO),
            tangent: Vec3::X,
            normal: Vec3::Z,
            binormal: Vec3::Y,
            bank_angle_deg: 0.0,
            curvature: 0.0,
            speed: config.default_speed,
            rotation_deg: Vec3::ZERO,
        };
    }

    let clamped_t = t.clamp(0.0, 1.0);
    let segments = if config.is_closed_loop { n } else { n - 1 };
    let seg_float = clamped_t * segments as f32;
    let seg_idx = (seg_float.floor() as usize).min(segments - 1);
    let seg_t = (seg_float - seg_idx as f32).clamp(0.0, 1.0);

    let (i0, i1, i2, i3) = if config.is_closed_loop {
        (
            (seg_idx + n - 1) % n,
            seg_idx,
            (seg_idx + 1) % n,
            (seg_idx + 2) % n,
        )
    } else {
        (
            seg_idx.saturating_sub(1),
            seg_idx,
            (seg_idx + 1).min(n - 1),
            (seg_idx + 2).min(n - 1),
        )
    };

    let p0 = pts[i0].position;
    let p1 = pts[i1].position;
    let p2 = pts[i2].position;
    let p3 = pts[i3].position;

    let pos = catmull_rom_eval(p0, p1, p2, p3, seg_t);
    let vel = catmull_rom_derivative(p0, p1, p2, p3, seg_t);
    let tangent = if vel.length_squared() > 1e-6 {
        vel.normalize()
    } else {
        Vec3::X
    };

    // Calculate approximate curvature via finite difference of tangent
    let dt = 0.01;
    let vel_next = catmull_rom_derivative(p0, p1, p2, p3, (seg_t + dt).min(1.0));
    let tangent_next = if vel_next.length_squared() > 1e-6 {
        vel_next.normalize()
    } else {
        tangent
    };
    let d_tan = (tangent_next - tangent) / dt;
    let curvature = d_tan.length() / vel.length().max(1.0);

    // Speed interpolation between station points
    let speed = pts[i1].speed + (pts[i2].speed - pts[i1].speed) * seg_t;

    // Physical banking calculation: phi = atan(v^2 / (g * R)) = atan(v^2 * curvature / g)
    let g = 950.0; // UT99 gravity units (uu/s^2)
    let lateral_accel = speed * speed * curvature;
    let raw_bank_rad = (lateral_accel / g).atan();
    let mut bank_deg = raw_bank_rad.to_degrees().clamp(
        -config.max_bank_angle_deg,
        config.max_bank_angle_deg,
    );

    // Determine banking sign from cross product of tangent and lateral acceleration
    if d_tan.cross(tangent).z < 0.0 {
        bank_deg = -bank_deg;
    }

    // Build frame: world Up adjusted by banking
    let world_up = Vec3::Z;
    let right = tangent.cross(world_up).normalize_or_zero();
    let _right = if right.length_squared() < 0.1 { Vec3::Y } else { right };

    let bank_rot = Quat::from_axis_angle(tangent, bank_deg.to_radians());
    let normal = (bank_rot * world_up).normalize();
    let binormal = tangent.cross(normal).normalize();

    // Compute Pitch, Yaw, Roll angles
    let yaw = tangent.y.atan2(tangent.x).to_degrees();
    let pitch = (-tangent.z).asin().to_degrees();
    let roll = bank_deg;

    TrackSample {
        position: pos,
        tangent,
        normal,
        binormal,
        bank_angle_deg: bank_deg,
        curvature,
        speed,
        rotation_deg: Vec3::new(pitch, yaw, roll),
    }
}

/// Generates dual steel rail meshes and cross-tie sleeper meshes
pub fn generate_track_brushes(config: &TrackConfig) -> (Brush, Brush) {
    let mut rails_brush = Brush::new("Track_DualRails", CsgOperation::Add);
    let mut ties_brush = Brush::new("Track_CrossTies", CsgOperation::Add);

    let steps = (config.points.len() * 16).max(32);
    let half_gauge = config.gauge_width * 0.5;
    let half_rail = config.rail_thickness * 0.5;

    let mut left_rail_samples = Vec::with_capacity(steps);
    let mut right_rail_samples = Vec::with_capacity(steps);

    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let sample = sample_track_spline(config, t);

        let left_center = sample.position - sample.binormal * half_gauge;
        let right_center = sample.position + sample.binormal * half_gauge;

        left_rail_samples.push((left_center, sample.normal, sample.binormal));
        right_rail_samples.push((right_center, sample.normal, sample.binormal));
    }

    // 1. Extrude Left and Right Steel Rails
    for rail_pts in [&left_rail_samples, &right_rail_samples] {
        for i in 0..(rail_pts.len() - 1) {
            let (c0, n0, b0) = rail_pts[i];
            let (c1, n1, b1) = rail_pts[i + 1];

            // 4 corners of rail cross-section
            let r0_top_l = c0 + n0 * half_rail - b0 * half_rail;
            let r0_top_r = c0 + n0 * half_rail + b0 * half_rail;
            let r0_bot_r = c0 - n0 * half_rail + b0 * half_rail;
            let r0_bot_l = c0 - n0 * half_rail - b0 * half_rail;

            let r1_top_l = c1 + n1 * half_rail - b1 * half_rail;
            let r1_top_r = c1 + n1 * half_rail + b1 * half_rail;
            let r1_bot_r = c1 - n1 * half_rail + b1 * half_rail;
            let r1_bot_l = c1 - n1 * half_rail - b1 * half_rail;

            // Top quad
            if let Some(mut p) = Polygon::new(vec![r0_top_l, r1_top_l, r1_top_r, r0_top_r]) {
                p.texture = config.rail_texture.clone();
                rails_brush.polys.push(p);
            }
            // Bottom quad
            if let Some(mut p) = Polygon::new(vec![r0_bot_r, r1_bot_r, r1_bot_l, r0_bot_l]) {
                p.texture = config.rail_texture.clone();
                rails_brush.polys.push(p);
            }
            // Left quad
            if let Some(mut p) = Polygon::new(vec![r0_bot_l, r1_bot_l, r1_top_l, r0_top_l]) {
                p.texture = config.rail_texture.clone();
                rails_brush.polys.push(p);
            }
            // Right quad
            if let Some(mut p) = Polygon::new(vec![r0_top_r, r1_top_r, r1_bot_r, r0_bot_r]) {
                p.texture = config.rail_texture.clone();
                rails_brush.polys.push(p);
            }
        }
    }

    // 2. Generate Wooden / Concrete Cross-Ties (Sleepers)
    let tie_count = (config.points.len() * 12).max(16);
    for i in 0..tie_count {
        let t = i as f32 / tie_count as f32;
        let sample = sample_track_spline(config, t);

        let half_w = config.tie_width * 0.5;
        let half_h = config.tie_height * 0.5;
        let half_l = config.tie_length * 0.5;

        // Position tie slightly below rail bottom
        let tie_center = sample.position - sample.normal * (half_rail + half_h);

        // 8 vertices of the sleeper box
        let t_fwd = sample.tangent * half_w;
        let t_up = sample.normal * half_h;
        let t_side = sample.binormal * half_l;

        let v000 = tie_center - t_fwd - t_up - t_side;
        let v001 = tie_center - t_fwd - t_up + t_side;
        let v010 = tie_center - t_fwd + t_up - t_side;
        let v011 = tie_center - t_fwd + t_up + t_side;
        let v100 = tie_center + t_fwd - t_up - t_side;
        let v101 = tie_center + t_fwd - t_up + t_side;
        let v110 = tie_center + t_fwd + t_up - t_side;
        let v111 = tie_center + t_fwd + t_up + t_side;

        // 6 faces of the tie box
        let faces = [
            vec![v010, v011, v111, v110], // Top
            vec![v000, v100, v101, v001], // Bottom
            vec![v100, v110, v111, v101], // Front
            vec![v000, v001, v011, v010], // Back
            vec![v000, v010, v110, v100], // Left
            vec![v001, v101, v111, v011], // Right
        ];

        for quad in faces {
            if let Some(mut poly) = Polygon::new(quad) {
                poly.texture = config.tie_texture.clone();
                ties_brush.polys.push(poly);
            }
        }
    }

    (rails_brush, ties_brush)
}

/// Bakes exactly N mover keyframes along the spline (Key0..Key7)
pub fn bake_mover_keyframes(config: &TrackConfig) -> Vec<MoverKeyframeRecord> {
    let key_count = config.keyframe_count.clamp(2, 8);
    let mut keyframes = Vec::with_capacity(key_count);

    let deg_to_unr = 65536.0 / 360.0;

    for i in 0..key_count {
        let t = i as f32 / (key_count - 1) as f32;
        let sample = sample_track_spline(config, t);

        // Unreal Engine 1 rotation units (0..65535)
        let pitch_unr = ((sample.rotation_deg.x * deg_to_unr) as i32) & 0xffff;
        let yaw_unr = ((sample.rotation_deg.y * deg_to_unr) as i32) & 0xffff;
        let roll_unr = ((sample.rotation_deg.z * deg_to_unr) as i32) & 0xffff;

        let is_station = i == 0 || i == key_count - 1;
        let move_time = if i < key_count - 1 {
            let next_t = (i + 1) as f32 / (key_count - 1) as f32;
            let next_sample = sample_track_spline(config, next_t);
            let dist = sample.position.distance(next_sample.position);
            (dist / sample.speed.max(10.0)).max(0.5)
        } else {
            0.0
        };

        keyframes.push(MoverKeyframeRecord {
            key_index: i,
            position: sample.position,
            rotation_deg: sample.rotation_deg,
            unreal_rotation: (pitch_unr, yaw_unr, roll_unr),
            move_time,
            is_station_stop: is_station,
        });
    }

    keyframes
}

/// Generates LiftCenter and LiftExit waypoints at track stations for bot AI
pub fn generate_station_bot_waypoints(config: &TrackConfig) -> Vec<StationWaypoint> {
    let mut waypoints = Vec::new();

    // Station 1: Start (Key0)
    let start_sample = sample_track_spline(config, 0.0);
    let platform_offset = start_sample.binormal * (config.gauge_width * 0.5 + 48.0);

    waypoints.push(StationWaypoint {
        name: "LiftExit_Start".to_string(),
        class_name: "Engine.LiftExit".to_string(),
        location: start_sample.position + platform_offset,
        is_cart_mounted: false,
    });

    waypoints.push(StationWaypoint {
        name: "LiftCenter_Cart".to_string(),
        class_name: "Engine.LiftCenter".to_string(),
        location: start_sample.position + Vec3::new(0.0, 0.0, 16.0),
        is_cart_mounted: true,
    });

    // Station 2: End (KeyN-1)
    let end_sample = sample_track_spline(config, 1.0);
    let end_platform = end_sample.binormal * (config.gauge_width * 0.5 + 48.0);

    waypoints.push(StationWaypoint {
        name: "LiftExit_End".to_string(),
        class_name: "Engine.LiftExit".to_string(),
        location: end_sample.position + end_platform,
        is_cart_mounted: false,
    });

    waypoints
}

/// Audits track configuration for kinetic, G-force, and level design hazards
pub fn audit_track_safety(config: &TrackConfig) -> Vec<TrackHazard> {
    let mut hazards = Vec::new();

    // 1. Check Keyframe count
    if config.keyframe_count > 8 {
        hazards.push(TrackHazard {
            is_critical: true,
            title: "Excessive Mover Keyframes (>8)".to_string(),
            description: format!("UT99 Mover actors only support Key0..Key7 (8 keyframes). Config requested {}.", config.keyframe_count),
            recommendation: "Clamp keyframe count to 8 or split track into multiple sequenced mover segments.".to_string(),
        });
    }

    // 2. Sample track for high lateral G-forces and extreme grades
    let steps = 30;
    let mut max_g = 0.0f32;
    let mut max_grade_deg = 0.0f32;

    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let sample = sample_track_spline(config, t);
        let lateral_g = (sample.speed * sample.speed * sample.curvature) / 950.0;
        max_g = max_g.max(lateral_g);

        let grade = sample.tangent.z.abs().asin().to_degrees();
        max_grade_deg = max_grade_deg.max(grade);
    }

    if max_g > 3.0 {
        hazards.push(TrackHazard {
            is_critical: false,
            title: format!("High Lateral Acceleration ({:.1} G)", max_g),
            description: "Curvature or train speed creates high centrifugal G-forces that would eject passengers.".to_string(),
            recommendation: "Increase turn radius or lower speed at sharp corners.".to_string(),
        });
    }

    if max_grade_deg > 45.0 {
        hazards.push(TrackHazard {
            is_critical: false,
            title: format!("Steep Track Grade ({:.1}°)", max_grade_deg),
            description: "Track slope exceeds 45°, which may cause player physics jitter or camera clipping.".to_string(),
            recommendation: "Smooth elevation changes between control stations.".to_string(),
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_archetypes_and_defaults() {
        for &arch in TrackArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.points.len() >= 2);
            assert!(cfg.gauge_width > 0.0);
            assert!(cfg.default_speed > 0.0);
            assert!(cfg.keyframe_count >= 2 && cfg.keyframe_count <= 8);
        }
    }

    #[test]
    fn test_track_spline_sampling_and_banking() {
        let cfg = TrackArchetype::IndustrialMinecartRail.default_config();
        let sample_start = sample_track_spline(&cfg, 0.0);
        let sample_mid = sample_track_spline(&cfg, 0.5);
        let sample_end = sample_track_spline(&cfg, 1.0);

        assert_eq!(sample_start.position, cfg.points[0].position);
        assert!(sample_mid.position.length() > 0.0);
        assert_eq!(sample_end.position, cfg.points.last().unwrap().position);

        assert!(sample_start.tangent.length() > 0.99);
        assert!(sample_start.normal.length() > 0.99);
    }

    #[test]
    fn test_generate_track_brushes_and_ties() {
        let cfg = TrackArchetype::IndustrialMinecartRail.default_config();
        let (rails, ties) = generate_track_brushes(&cfg);

        assert!(!rails.polys.is_empty(), "Rails brush must contain polygons");
        assert!(!ties.polys.is_empty(), "Ties brush must contain polygons");

        for poly in &rails.polys {
            assert_eq!(poly.texture, cfg.rail_texture);
        }
        for poly in &ties.polys {
            assert_eq!(poly.texture, cfg.tie_texture);
        }
    }

    #[test]
    fn test_bake_mover_keyframes_and_bot_waypoints() {
        let mut cfg = TrackArchetype::HighSpeedBulletTrain.default_config();
        cfg.keyframe_count = 6;

        let keyframes = bake_mover_keyframes(&cfg);
        assert_eq!(keyframes.len(), 6);
        assert_eq!(keyframes[0].key_index, 0);
        assert_eq!(keyframes[5].key_index, 5);

        for kf in &keyframes {
            assert!(kf.move_time >= 0.0);
            assert!(kf.unreal_rotation.0 >= 0 && kf.unreal_rotation.0 <= 65535);
        }

        let waypoints = generate_station_bot_waypoints(&cfg);
        assert_eq!(waypoints.len(), 3, "Must generate 2 LiftExits + 1 LiftCenter");
        assert!(waypoints.iter().any(|wp| wp.class_name == "Engine.LiftCenter"));
        assert!(waypoints.iter().any(|wp| wp.class_name == "Engine.LiftExit"));
    }

    #[test]
    fn test_track_safety_audit() {
        let mut cfg = TrackConfig::default();
        cfg.keyframe_count = 10; // Exceeds UT99 8-keyframe limit

        let hazards = audit_track_safety(&cfg);
        let has_crit = hazards.iter().any(|h| h.is_critical && h.title.contains(">8"));
        assert!(has_crit, "Audit must catch excessive keyframes");
    }
}
