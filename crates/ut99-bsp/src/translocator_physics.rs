//! Translocator & Teleport Disc Physics & Beacon Trajectory Studio
//!
//! Provides discrete-time physics simulation for the iconic UT99 TranslocatorTarget disc,
//! including ballistic flight arcs, surface bouncing/damping, resting beacon detection,
//! aperture clearance audit, and teleport transit timing compared to walking paths.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use crate::poly::Polygon;

/// Canonical toss presets for standard UT99 competitive translocator maneuvers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TranslocatorPreset {
    /// 45-degree high arc for maximum horizontal throw distance.
    MaxDistanceArc45,
    /// Low 5-degree skim for rapid straight corridor transit.
    FlatCorridorSkim,
    /// Steep 70-degree loft to reach elevated perches and sniper towers.
    HighLedgePerchLob,
    /// 12-degree bank shot designed to bounce off corridor walls around 90° corners.
    CornerBankRicochet,
    /// Downward drop for navigating vertical shafts, pipes, and lift shafts.
    VerticalShaftDrop,
    /// Precision toss aimed to thread through narrow windows and arrow slits.
    WindowApertureSniping,
}

impl TranslocatorPreset {
    pub const ALL: [TranslocatorPreset; 6] = [
        TranslocatorPreset::MaxDistanceArc45,
        TranslocatorPreset::FlatCorridorSkim,
        TranslocatorPreset::HighLedgePerchLob,
        TranslocatorPreset::CornerBankRicochet,
        TranslocatorPreset::VerticalShaftDrop,
        TranslocatorPreset::WindowApertureSniping,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            TranslocatorPreset::MaxDistanceArc45 => "Max Distance Arc (45°)",
            TranslocatorPreset::FlatCorridorSkim => "Flat Corridor Skim (5°)",
            TranslocatorPreset::HighLedgePerchLob => "High Ledge Perch Lob (70°)",
            TranslocatorPreset::CornerBankRicochet => "Corner Bank Ricochet (12°)",
            TranslocatorPreset::VerticalShaftDrop => "Vertical Shaft Drop (-80°)",
            TranslocatorPreset::WindowApertureSniping => "Window Aperture Sniping (28°)",
        }
    }

    pub fn to_config(&self) -> TranslocatorTossConfig {
        match self {
            TranslocatorPreset::MaxDistanceArc45 => TranslocatorTossConfig {
                toss_speed: 1200.0,
                pitch_deg: 45.0,
                yaw_deg: 0.0,
                gravity: -950.0,
                elasticity: 0.35,
                friction: 0.75,
                max_bounces: 4,
                disc_radius: 6.0,
            },
            TranslocatorPreset::FlatCorridorSkim => TranslocatorTossConfig {
                toss_speed: 1200.0,
                pitch_deg: 5.0,
                yaw_deg: 0.0,
                gravity: -950.0,
                elasticity: 0.30,
                friction: 0.70,
                max_bounces: 3,
                disc_radius: 6.0,
            },
            TranslocatorPreset::HighLedgePerchLob => TranslocatorTossConfig {
                toss_speed: 1150.0,
                pitch_deg: 70.0,
                yaw_deg: 0.0,
                gravity: -950.0,
                elasticity: 0.25,
                friction: 0.60,
                max_bounces: 3,
                disc_radius: 6.0,
            },
            TranslocatorPreset::CornerBankRicochet => TranslocatorTossConfig {
                toss_speed: 1050.0,
                pitch_deg: 12.0,
                yaw_deg: 35.0,
                gravity: -950.0,
                elasticity: 0.55,
                friction: 0.85,
                max_bounces: 5,
                disc_radius: 6.0,
            },
            TranslocatorPreset::VerticalShaftDrop => TranslocatorTossConfig {
                toss_speed: 400.0,
                pitch_deg: -80.0,
                yaw_deg: 0.0,
                gravity: -950.0,
                elasticity: 0.20,
                friction: 0.50,
                max_bounces: 2,
                disc_radius: 6.0,
            },
            TranslocatorPreset::WindowApertureSniping => TranslocatorTossConfig {
                toss_speed: 1200.0,
                pitch_deg: 28.0,
                yaw_deg: 0.0,
                gravity: -950.0,
                elasticity: 0.30,
                friction: 0.70,
                max_bounces: 3,
                disc_radius: 6.0,
            },
        }
    }
}

/// Physical parameters for the translocator disc launch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslocatorTossConfig {
    pub toss_speed: f32,    // Initial velocity magnitude (uu/s), canonical ~1200.0
    pub pitch_deg: f32,     // Elevation angle in degrees (-90 to +90)
    pub yaw_deg: f32,       // Horizontal azimuth in degrees (0 to 360)
    pub gravity: f32,       // Gravitational acceleration (uu/s²), canonical -950.0
    pub elasticity: f32,    // Coefficient of restitution for normal bounce (0.0 to 1.0)
    pub friction: f32,      // Tangential velocity retention on surface impact (0.0 to 1.0)
    pub max_bounces: usize, // Maximum number of surface bounces allowed
    pub disc_radius: f32,   // Collision sphere radius for the disc (uu), canonical 6.0
}

impl Default for TranslocatorTossConfig {
    fn default() -> Self {
        TranslocatorPreset::MaxDistanceArc45.to_config()
    }
}

/// A discrete point sampled along the flight trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslocatorTrajectoryPoint {
    pub position: Vec3,
    pub velocity: Vec3,
    pub time: f32,
    pub is_bounce: bool,
    pub surface_normal: Option<Vec3>,
    pub is_at_rest: bool,
}

/// Complete simulation result for a translocator disc toss.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslocatorFlightSimulation {
    pub points: Vec<TranslocatorTrajectoryPoint>,
    pub final_beacon_location: Option<Vec3>,
    pub total_flight_time: f32,
    pub bounce_count: usize,
    pub min_aperture_clearance: f32,
    pub walking_travel_time: f32,
    pub teleport_transit_time: f32,
    pub time_saved_seconds: f32,
}

/// Ray-polygon intersection helper for surface reflection.
fn find_nearest_surface_hit(start: Vec3, dir: Vec3, max_dist: f32, world_polys: &[Polygon]) -> Option<(f32, Vec3)> {
    let mut closest_t = max_dist;
    let mut hit_normal = None;

    for poly in world_polys {
        if poly.vertices.len() < 3 {
            continue;
        }

        let normal = poly.plane.normal;
        let denom = normal.dot(dir);

        // Discard backfaces or near-parallel rays
        if denom >= -1e-5 {
            continue;
        }

        let d = poly.plane.dist;
        let t = (d - normal.dot(start)) / denom;

        if t > 0.001 && t < closest_t {
            let hit_pt = start + dir * t;

            // In-polygon test using cross product winding consistency
            let mut inside = true;
            let n_verts = poly.vertices.len();
            for i in 0..n_verts {
                let v0 = poly.vertices[i];
                let v1 = poly.vertices[(i + 1) % n_verts];
                let edge = v1 - v0;
                let to_hit = hit_pt - v0;
                if edge.cross(to_hit).dot(normal) < -0.01 {
                    inside = false;
                    break;
                }
            }

            if inside {
                closest_t = t;
                hit_normal = Some(normal);
            }
        }
    }

    hit_normal.map(|n| (closest_t, n))
}

/// Simulates the translocator disc flight with discrete integration and surface bounces.
pub fn simulate_translocator_flight(
    start: Vec3,
    config: &TranslocatorTossConfig,
    world_polys: &[Polygon],
) -> TranslocatorFlightSimulation {
    let pitch_rad = config.pitch_deg.to_radians();
    let yaw_rad = config.yaw_deg.to_radians();
    let cp = pitch_rad.cos();
    let init_dir = Vec3::new(cp * yaw_rad.cos(), cp * yaw_rad.sin(), pitch_rad.sin()).normalize();

    let mut current_pos = start;
    let mut current_vel = init_dir * config.toss_speed;
    let mut current_time = 0.0;
    let mut bounces = 0;

    let dt = 0.02; // 50 Hz discrete physics integration
    let max_time = 6.0; // Max simulated flight duration

    let mut points = Vec::new();
    points.push(TranslocatorTrajectoryPoint {
        position: current_pos,
        velocity: current_vel,
        time: 0.0,
        is_bounce: false,
        surface_normal: None,
        is_at_rest: false,
    });

    let mut min_clearance = f32::MAX;

    while current_time < max_time {
        // Compute displacement for this step
        let step_vel = current_vel + Vec3::new(0.0, 0.0, config.gravity * dt * 0.5);
        let step_disp = step_vel * dt;
        let step_dist = step_disp.length();

        if step_dist < 1e-4 {
            break;
        }

        let step_dir = step_disp / step_dist;

        // Check for collision along the step
        if let Some((hit_dist, normal)) = find_nearest_surface_hit(current_pos, step_dir, step_dist + config.disc_radius, world_polys) {
            // Move up to contact point
            let actual_dist = (hit_dist - config.disc_radius).max(0.0);
            current_pos += step_dir * actual_dist;
            current_time += (actual_dist / step_dist) * dt;

            bounces += 1;

            // Rest condition: low speed on a walkable floor (normal.z > 0.6)
            let speed_sq = current_vel.length_squared();
            if bounces >= config.max_bounces || (speed_sq < (35.0 * 35.0) && normal.z > 0.6) {
                points.push(TranslocatorTrajectoryPoint {
                    position: current_pos,
                    velocity: Vec3::ZERO,
                    time: current_time,
                    is_bounce: true,
                    surface_normal: Some(normal),
                    is_at_rest: true,
                });
                break;
            }

            // Normal and tangential decomposition for surface reflection
            let vn = normal * current_vel.dot(normal);
            let vt = current_vel - vn;
            current_vel = vt * config.friction - vn * config.elasticity;

            points.push(TranslocatorTrajectoryPoint {
                position: current_pos,
                velocity: current_vel,
                time: current_time,
                is_bounce: true,
                surface_normal: Some(normal),
                is_at_rest: false,
            });

            // Prevent snagging on next step
            current_pos += normal * 0.1;
        } else {
            // Free flight step
            current_pos += step_disp;
            current_vel = current_vel + Vec3::new(0.0, 0.0, config.gravity * dt);
            current_time += dt;

            // Simple floor check at z = 0 if no world polys were passed
            if world_polys.is_empty() && current_pos.z <= 0.0 {
                current_pos.z = 0.0;
                bounces += 1;
                let normal = Vec3::new(0.0, 0.0, 1.0);

                if bounces >= config.max_bounces || current_vel.length() < 50.0 {
                    points.push(TranslocatorTrajectoryPoint {
                        position: current_pos,
                        velocity: Vec3::ZERO,
                        time: current_time,
                        is_bounce: true,
                        surface_normal: Some(normal),
                        is_at_rest: true,
                    });
                    break;
                }

                current_vel.z = -current_vel.z * config.elasticity;
                current_vel.x *= config.friction;
                current_vel.y *= config.friction;

                points.push(TranslocatorTrajectoryPoint {
                    position: current_pos,
                    velocity: current_vel,
                    time: current_time,
                    is_bounce: true,
                    surface_normal: Some(normal),
                    is_at_rest: false,
                });
            } else {
                points.push(TranslocatorTrajectoryPoint {
                    position: current_pos,
                    velocity: current_vel,
                    time: current_time,
                    is_bounce: false,
                    surface_normal: None,
                    is_at_rest: false,
                });
            }
        }

        // Track minimum clearance to any poly
        for poly in world_polys {
            let dist_to_plane = (poly.plane.normal.dot(current_pos) - poly.plane.dist).abs();
            if dist_to_plane < min_clearance {
                min_clearance = dist_to_plane;
            }
        }
    }

    let final_pos = points.last().map(|p| p.position);
    let final_time = points.last().map(|p| p.time).unwrap_or(0.0);

    // Compute travel timings
    let horizontal_dist = if let Some(dest) = final_pos {
        let diff = dest - start;
        (diff.x * diff.x + diff.y * diff.y).sqrt()
    } else {
        0.0
    };

    let walking_speed = 400.0; // Standard UT99 player running speed (uu/s)
    let walking_travel_time = horizontal_dist / walking_speed;
    let weapon_switch_latency = 0.20; // Time to switch/activate translocator secondary fire
    let teleport_transit_time = final_time + weapon_switch_latency;
    let time_saved = walking_travel_time - teleport_transit_time;

    TranslocatorFlightSimulation {
        points,
        final_beacon_location: final_pos,
        total_flight_time: final_time,
        bounce_count: bounces,
        min_aperture_clearance: if min_clearance == f32::MAX { 999.0 } else { min_clearance },
        walking_travel_time,
        teleport_transit_time,
        time_saved_seconds: time_saved,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translocator_presets_and_configs() {
        for preset in TranslocatorPreset::ALL {
            let cfg = preset.to_config();
            assert!(cfg.toss_speed > 0.0);
            assert!(cfg.gravity < 0.0);
            assert!(cfg.elasticity > 0.0 && cfg.elasticity < 1.0);
            assert!(cfg.friction > 0.0 && cfg.friction <= 1.0);
            assert!(cfg.disc_radius > 0.0);
            assert!(!preset.display_name().is_empty());
        }
    }

    #[test]
    fn test_translocator_flight_in_open_air() {
        let config = TranslocatorPreset::MaxDistanceArc45.to_config();
        let start = Vec3::new(0.0, 0.0, 50.0);
        let sim = simulate_translocator_flight(start, &config, &[]);

        assert!(!sim.points.is_empty());
        assert!(sim.total_flight_time > 0.0);
        assert!(sim.bounce_count > 0);
        assert!(sim.final_beacon_location.is_some());

        let final_pos = sim.final_beacon_location.unwrap();
        // Settle on the floor z = 0
        assert_eq!(final_pos.z, 0.0);
        // Traveled far in X (since yaw = 0)
        assert!(final_pos.x > 500.0);
        // Teleport transit time is calculated
        assert!(sim.teleport_transit_time > 0.0);
    }

    #[test]
    fn test_translocator_bounces_and_timing() {
        let config = TranslocatorPreset::FlatCorridorSkim.to_config();
        let start = Vec3::new(0.0, 0.0, 100.0);
        let sim = simulate_translocator_flight(start, &config, &[]);

        assert!(sim.bounce_count <= config.max_bounces);
        let last_pt = sim.points.last().unwrap();
        assert!(last_pt.is_at_rest);
    }
}
