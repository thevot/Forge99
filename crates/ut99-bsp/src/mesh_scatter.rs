// crates/ut99-bsp/src/mesh_scatter.rs
//
// Interactive Surface Clutter, Procedural Foliage & Decoration Scatter Studio
//
// Algorithmic engine for Poisson-disc surface sampling, normal alignment,
// slope angle filtering, randomized scale/rotation jitter, and bot path clearance auditing.

use crate::poly::Polygon;
use glam::{Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Classical Unreal Tournament & Unreal Engine environmental clutter archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScatterArchetype {
    OutdoorForestFlora,
    GothicWallTorches,
    IndustrialCargoClutter,
    DungeonCeilingStalactites,
    CaveRubbleDebris,
}

impl ScatterArchetype {
    pub fn all() -> &'static [ScatterArchetype] {
        &[
            ScatterArchetype::OutdoorForestFlora,
            ScatterArchetype::GothicWallTorches,
            ScatterArchetype::IndustrialCargoClutter,
            ScatterArchetype::DungeonCeilingStalactites,
            ScatterArchetype::CaveRubbleDebris,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ScatterArchetype::OutdoorForestFlora => "Outdoor Forest Flora (Trees & Ferns)",
            ScatterArchetype::GothicWallTorches => "Gothic Wall Torches & Sconces",
            ScatterArchetype::IndustrialCargoClutter => "Industrial Cargo Crates & Barrels",
            ScatterArchetype::DungeonCeilingStalactites => "Dungeon Ceiling Stalactites & Chains",
            ScatterArchetype::CaveRubbleDebris => "Cave Rubble & Rocky Debris",
        }
    }

    pub fn default_config(&self) -> ScatterConfig {
        match self {
            ScatterArchetype::OutdoorForestFlora => ScatterConfig {
                archetype: *self,
                class_name: "UnrealShare.Tree".to_string(),
                mesh_name: "Tree01".to_string(),
                target_count: 24,
                min_spacing_radius: 64.0,
                scale_min: 0.75,
                scale_max: 1.35,
                random_yaw: true,
                max_surface_tilt_deg: 12.0,
                embed_offset_z: -4.0,
                slope_min_deg: 0.0,
                slope_max_deg: 35.0, // horizontal ground only
                alignment_mode: ScatterAlignmentMode::WorldZWithSurfaceTilt,
                bot_clearance_radius: 48.0,
                random_seed: 1337,
            },
            ScatterArchetype::GothicWallTorches => ScatterConfig {
                archetype: *self,
                class_name: "UnrealShare.TorchFlame".to_string(),
                mesh_name: "TorchSconce".to_string(),
                target_count: 12,
                min_spacing_radius: 128.0,
                scale_min: 0.9,
                scale_max: 1.1,
                random_yaw: false,
                max_surface_tilt_deg: 0.0,
                embed_offset_z: 2.0,
                slope_min_deg: 75.0,
                slope_max_deg: 105.0, // vertical walls only
                alignment_mode: ScatterAlignmentMode::AlignToSurfaceNormal,
                bot_clearance_radius: 40.0,
                random_seed: 4242,
            },
            ScatterArchetype::IndustrialCargoClutter => ScatterConfig {
                archetype: *self,
                class_name: "UnrealShare.WoodenBox".to_string(),
                mesh_name: "CrateWood".to_string(),
                target_count: 16,
                min_spacing_radius: 72.0,
                scale_min: 0.8,
                scale_max: 1.25,
                random_yaw: true,
                max_surface_tilt_deg: 5.0,
                embed_offset_z: 0.0,
                slope_min_deg: 0.0,
                slope_max_deg: 15.0, // flat floors
                alignment_mode: ScatterAlignmentMode::WorldZWithSurfaceTilt,
                bot_clearance_radius: 54.0,
                random_seed: 777,
            },
            ScatterArchetype::DungeonCeilingStalactites => ScatterConfig {
                archetype: *self,
                class_name: "UnrealShare.Stalactite".to_string(),
                mesh_name: "RockSpike".to_string(),
                target_count: 20,
                min_spacing_radius: 48.0,
                scale_min: 0.6,
                scale_max: 1.5,
                random_yaw: true,
                max_surface_tilt_deg: 20.0,
                embed_offset_z: -2.0,
                slope_min_deg: 145.0,
                slope_max_deg: 180.0, // ceilings
                alignment_mode: ScatterAlignmentMode::AlignToSurfaceNormal,
                bot_clearance_radius: 24.0,
                random_seed: 9999,
            },
            ScatterArchetype::CaveRubbleDebris => ScatterConfig {
                archetype: *self,
                class_name: "UnrealShare.RockPiece".to_string(),
                mesh_name: "Rubble01".to_string(),
                target_count: 32,
                min_spacing_radius: 36.0,
                scale_min: 0.4,
                scale_max: 1.6,
                random_yaw: true,
                max_surface_tilt_deg: 35.0,
                embed_offset_z: -6.0,
                slope_min_deg: 0.0,
                slope_max_deg: 45.0,
                alignment_mode: ScatterAlignmentMode::AlignToSurfaceNormal,
                bot_clearance_radius: 36.0,
                random_seed: 5555,
            },
        }
    }
}

/// Normal and orientation alignment mode relative to the surface
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScatterAlignmentMode {
    /// Up vector strictly aligns with the surface normal (wall mounts, sconces, stalactites)
    AlignToSurfaceNormal,
    /// Up vector remains upright along World Z with optional subtle tilt jitter (trees, crates, barrels)
    WorldZWithSurfaceTilt,
}

impl ScatterAlignmentMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            ScatterAlignmentMode::AlignToSurfaceNormal => "Align to Surface Normal",
            ScatterAlignmentMode::WorldZWithSurfaceTilt => "Upright (World Z + Tilt)",
        }
    }
}

/// A single scattered instance placed on a surface
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScatterInstance {
    pub position: Vec3,
    pub rotation: Vec3, // Pitch, Yaw, Roll in degrees
    pub scale: Vec3,
    pub surface_normal: Vec3,
    pub class_name: String,
    pub mesh_name: String,
    pub is_valid_clearance: bool,
}

/// Configuration parameters for the procedural scatter engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScatterConfig {
    pub archetype: ScatterArchetype,
    pub class_name: String,
    pub mesh_name: String,
    pub target_count: usize,
    pub min_spacing_radius: f32,
    pub scale_min: f32,
    pub scale_max: f32,
    pub random_yaw: bool,
    pub max_surface_tilt_deg: f32,
    pub embed_offset_z: f32,
    pub slope_min_deg: f32,
    pub slope_max_deg: f32,
    pub alignment_mode: ScatterAlignmentMode,
    pub bot_clearance_radius: f32,
    pub random_seed: u64,
}

impl Default for ScatterConfig {
    fn default() -> Self {
        ScatterArchetype::OutdoorForestFlora.default_config()
    }
}

/// Hazard or warning flagged during scatter placement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScatterHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Summary metrics of the generated scatter instances
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScatterMetrics {
    pub total_instances: usize,
    pub valid_clearance_count: usize,
    pub blocked_clearance_count: usize,
    pub average_spacing: f32,
    pub sampled_surface_area: f32,
}

/// Simple pseudorandom number generator (Xorshift64) for deterministic replication
#[derive(Debug, Clone)]
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xdeadbeef_cafebabe } else { seed },
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x & 0xffff_ffff) as u32
    }

    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
}

/// Splits a polygon with N vertices into triangles for area-weighted sampling
pub fn triangulate_polygon(poly: &Polygon) -> Vec<[Vec3; 3]> {
    let mut tris = Vec::new();
    let n = poly.vertices.len();
    if n < 3 {
        return tris;
    }
    let p0 = poly.vertices[0];
    for i in 1..(n - 1) {
        tris.push([p0, poly.vertices[i], poly.vertices[i + 1]]);
    }
    tris
}

/// Computes the surface area of a 3D triangle
pub fn triangle_area(tri: &[Vec3; 3]) -> f32 {
    let v0 = tri[1] - tri[0];
    let v1 = tri[2] - tri[0];
    0.5 * v0.cross(v1).length()
}

/// Generates procedural scatter instances across a collection of world polygons,
/// enforcing slope filtering, Poisson-disc spacing, and bot path clearance.
pub fn generate_scatter_instances(
    polygons: &[Polygon],
    config: &ScatterConfig,
    bot_nodes: &[Vec3],
) -> Vec<ScatterInstance> {
    let mut rng = SimpleRng::new(config.random_seed);
    let mut candidate_triangles = Vec::new();
    let mut total_area = 0.0f32;

    // 1. Gather all triangles matching slope angle constraints
    for poly in polygons {
        let normal = poly.plane.normal.normalize();
        // Slope angle relative to World Up (0 degrees = flat floor, 90 = vertical wall, 180 = ceiling)
        let cos_slope = normal.dot(Vec3::Z).clamp(-1.0, 1.0);
        let slope_deg = cos_slope.acos().to_degrees();

        if slope_deg >= config.slope_min_deg && slope_deg <= config.slope_max_deg {
            let tris = triangulate_polygon(poly);
            for tri in tris {
                let a = triangle_area(&tri);
                if a > 0.01 {
                    total_area += a;
                    candidate_triangles.push((tri, normal, a));
                }
            }
        }
    }

    if candidate_triangles.is_empty() || total_area <= 0.001 {
        return Vec::new();
    }

    let mut instances: Vec<ScatterInstance> = Vec::new();
    let max_attempts = config.target_count * 20;
    let min_dist_sq = config.min_spacing_radius * config.min_spacing_radius;

    // 2. Poisson-Disc Sampling Loop
    for _ in 0..max_attempts {
        if instances.len() >= config.target_count {
            break;
        }

        // Pick triangle with probability proportional to its area
        let mut pick_val = rng.next_f32() * total_area;
        let mut chosen_tri = None;
        let mut chosen_normal = Vec3::Z;

        for (tri, norm, area) in &candidate_triangles {
            if pick_val <= *area {
                chosen_tri = Some(*tri);
                chosen_normal = *norm;
                break;
            }
            pick_val -= *area;
        }

        let tri = match chosen_tri {
            Some(t) => t,
            None => {
                if let Some((t, n, _)) = candidate_triangles.last() {
                    chosen_normal = *n;
                    *t
                } else {
                    continue;
                }
            }
        };

        // Uniform random point on triangle using square root method
        let r1 = rng.next_f32();
        let r2 = rng.next_f32();
        let sqrt_r1 = r1.sqrt();
        let u = 1.0 - sqrt_r1;
        let v = sqrt_r1 * (1.0 - r2);
        let w = sqrt_r1 * r2;

        let mut pt = tri[0] * u + tri[1] * v + tri[2] * w;
        // Apply normal embed offset
        pt += chosen_normal * config.embed_offset_z;

        // Check Poisson-disc distance against already accepted instances
        let mut too_close = false;
        for existing in &instances {
            if pt.distance_squared(existing.position) < min_dist_sq {
                too_close = true;
                break;
            }
        }
        if too_close {
            continue;
        }

        // Check clearance to bot navigation waypoints
        let mut has_bot_clearance = true;
        for &bot_pos in bot_nodes {
            let dist = pt.distance(bot_pos);
            if dist < config.bot_clearance_radius {
                has_bot_clearance = false;
                break;
            }
        }

        // 3. Compute Orientation (Pitch, Yaw, Roll)
        let rot_deg = match config.alignment_mode {
            ScatterAlignmentMode::AlignToSurfaceNormal => {
                // Orient local Z along surface normal
                let up = chosen_normal;
                let ref_tangent = if up.z.abs() < 0.9 { Vec3::Z } else { Vec3::Y };
                let right = up.cross(ref_tangent).normalize();
                let fwd = right.cross(up).normalize();

                let mut q = Quat::from_mat3(&Mat3::from_cols(right, fwd, up));
                if config.random_yaw {
                    let yaw_jitter = Quat::from_axis_angle(up, rng.range_f32(0.0, std::f32::consts::TAU));
                    q = yaw_jitter * q;
                }
                let (pitch, yaw, roll) = q.to_euler(glam::EulerRot::YXZ);
                Vec3::new(pitch.to_degrees(), yaw.to_degrees(), roll.to_degrees())
            }
            ScatterAlignmentMode::WorldZWithSurfaceTilt => {
                let yaw = if config.random_yaw {
                    rng.range_f32(0.0, 360.0)
                } else {
                    0.0
                };
                let max_tilt = config.max_surface_tilt_deg;
                let pitch = rng.range_f32(-max_tilt, max_tilt);
                let roll = rng.range_f32(-max_tilt, max_tilt);
                Vec3::new(pitch, yaw, roll)
            }
        };

        // 4. Compute Scale Jitter
        let s_val = rng.range_f32(config.scale_min, config.scale_max);
        let scale = Vec3::splat(s_val);

        instances.push(ScatterInstance {
            position: pt,
            rotation: rot_deg,
            scale,
            surface_normal: chosen_normal,
            class_name: config.class_name.clone(),
            mesh_name: config.mesh_name.clone(),
            is_valid_clearance: has_bot_clearance,
        });
    }

    instances
}

/// Evaluates metrics for the generated scatter set
pub fn evaluate_scatter_metrics(
    instances: &[ScatterInstance],
    polygons: &[Polygon],
    config: &ScatterConfig,
) -> ScatterMetrics {
    let total = instances.len();
    let valid = instances.iter().filter(|i| i.is_valid_clearance).count();
    let blocked = total.saturating_sub(valid);

    let mut total_spacing = 0.0f32;
    let mut spacing_pairs = 0;

    for i in 0..total {
        for j in (i + 1)..total {
            total_spacing += instances[i].position.distance(instances[j].position);
            spacing_pairs += 1;
        }
    }

    let avg_spacing = if spacing_pairs > 0 {
        total_spacing / spacing_pairs as f32
    } else {
        0.0
    };

    let mut area = 0.0f32;
    for poly in polygons {
        let normal = poly.plane.normal.normalize();
        let cos_slope = normal.dot(Vec3::Z).clamp(-1.0, 1.0);
        let slope_deg = cos_slope.acos().to_degrees();
        if slope_deg >= config.slope_min_deg && slope_deg <= config.slope_max_deg {
            for tri in triangulate_polygon(poly) {
                area += triangle_area(&tri);
            }
        }
    }

    ScatterMetrics {
        total_instances: total,
        valid_clearance_count: valid,
        blocked_clearance_count: blocked,
        average_spacing: avg_spacing,
        sampled_surface_area: area,
    }
}

/// Audits the scatter configuration for potential level design hazards
pub fn audit_scatter_hazards(
    instances: &[ScatterInstance],
    config: &ScatterConfig,
) -> Vec<ScatterHazard> {
    let mut hazards = Vec::new();

    // 1. Bot Path Blockage Hazard
    let blocked_count = instances.iter().filter(|i| !i.is_valid_clearance).count();
    if blocked_count > 0 {
        hazards.push(ScatterHazard {
            is_critical: true,
            title: format!("{} Instances Encroach on Bot Navigation Waypoints", blocked_count),
            description: format!("Clutter instances placed within {:.0} uu of PathNodes will cause bots to bump, stall, or fail reachspec connectivity.", config.bot_clearance_radius),
            recommendation: "Increase bot_clearance_radius or filter blocked instances before committing.".to_string(),
        });
    }

    // 2. High Instance Density Performance Warning
    if instances.len() > 100 {
        hazards.push(ScatterHazard {
            is_critical: false,
            title: format!("High Decoration Density ({} Actors)", instances.len()),
            description: "Placing more than 100 separate actor instances in a single zone increases CPU actor tick overhead in Unreal Engine 1 multiplayer servers.".to_string(),
            recommendation: "Consider increasing min_spacing_radius to reduce the overall count.".to_string(),
        });
    }

    // 3. Extreme Scale Variation Warning
    if config.scale_min < 0.25 || config.scale_max > 4.0 {
        hazards.push(ScatterHazard {
            is_critical: false,
            title: "Extreme Scale Range Configured".to_string(),
            description: "Scales under 0.25x or over 4.0x can distort collision hulls and texture luxel resolution.".to_string(),
            recommendation: "Keep scale ranges between 0.5x and 2.0x for harmonious architectural scale.".to_string(),
        });
    }

    // 4. Zero Instances Generated
    if instances.is_empty() {
        hazards.push(ScatterHazard {
            is_critical: false,
            title: "No Surfaces Matched Slope Filter".to_string(),
            description: format!("The current slope range [{:.0}°, {:.0}°] did not match any eligible surfaces.", config.slope_min_deg, config.slope_max_deg),
            recommendation: "Broaden the slope range (e.g. 0° to 90°) or select different world polygons.".to_string(),
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_quad(min_x: f32, max_x: f32, min_y: f32, max_y: f32, z: f32) -> Polygon {
        Polygon::new(vec![
            Vec3::new(min_x, min_y, z),
            Vec3::new(max_x, min_y, z),
            Vec3::new(max_x, max_y, z),
            Vec3::new(min_x, max_y, z),
        ]).unwrap()
    }

    #[test]
    fn test_scatter_archetypes_and_defaults() {
        for &arch in ScatterArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.target_count > 0);
            assert!(cfg.min_spacing_radius > 0.0);
            assert!(cfg.scale_max >= cfg.scale_min);
        }
    }

    #[test]
    fn test_poisson_disc_spacing_and_generation() {
        let quad = make_test_quad(0.0, 512.0, 0.0, 512.0, 0.0);
        let mut cfg = ScatterConfig::default();
        cfg.target_count = 10;
        cfg.min_spacing_radius = 64.0;

        let instances = generate_scatter_instances(&[quad], &cfg, &[]);
        assert!(!instances.is_empty());
        assert!(instances.len() <= cfg.target_count);

        // Verify minimum distance constraint
        for i in 0..instances.len() {
            for j in (i + 1)..instances.len() {
                let dist = instances[i].position.distance(instances[j].position);
                assert!(dist >= cfg.min_spacing_radius - 0.01, "Distance ({:.1}) must be >= min spacing ({:.1})", dist, cfg.min_spacing_radius);
            }
        }
    }

    #[test]
    fn test_slope_filtering_excludes_vertical_walls() {
        // Vertical wall polygon (normal along Y+)
        let wall = Polygon::new(vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(256.0, 0.0, 0.0),
            Vec3::new(256.0, 0.0, 256.0),
            Vec3::new(0.0, 0.0, 256.0),
        ]).unwrap();

        // Config requiring horizontal ground: slope 0° to 30°
        let mut cfg = ScatterConfig::default();
        cfg.slope_min_deg = 0.0;
        cfg.slope_max_deg = 30.0;

        let instances = generate_scatter_instances(&[wall], &cfg, &[]);
        assert!(instances.is_empty(), "Vertical wall (90 deg slope) must be filtered out by 0-30 deg slope filter");
    }

    #[test]
    fn test_bot_clearance_hazard_detection() {
        let quad = make_test_quad(0.0, 256.0, 0.0, 256.0, 0.0);
        let mut cfg = ScatterConfig::default();
        cfg.target_count = 15;
        cfg.bot_clearance_radius = 64.0;

        // Place bot node in center of quad
        let bot_node = Vec3::new(128.0, 128.0, 0.0);
        let instances = generate_scatter_instances(&[quad], &cfg, &[bot_node]);

        let hazards = audit_scatter_hazards(&instances, &cfg);
        // Verify hazard detection is deterministic and doesn't panic
        assert!(!hazards.is_empty() || instances.iter().all(|i| i.is_valid_clearance));
    }
}
