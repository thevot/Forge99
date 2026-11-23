// crates/ut99-bsp/src/destructible.rs
//
// Interactive Destructible Architecture, Breakable Glass & Fracture Studio
// Provides analytical polygon fracture generation (radial spiderweb, Voronoi, grid panes),
// real-time debris blast physics simulation, dynamic bot reachability pairing,
// and safety audit for Unreal Tournament 99 destructible elements.

use glam::{Vec2, Vec3};

/// Canonical gravity in Unreal Tournament 99 (Unreal Units per second squared)
pub const UT99_GRAVITY: f32 = -950.0;

/// Archetype category of destructible architectural element
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DestructibleArchetype {
    BreakableWindowGlass,
    ExplodingStoneWall,
    ReinforcedBlastDoor,
    FragileSkylightDome,
    VentGratingShatter,
}

impl DestructibleArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BreakableWindowGlass => "Breakable Window Glass (BreakingGlass)",
            Self::ExplodingStoneWall => "Exploding Stone Wall (ExplodingWall)",
            Self::ReinforcedBlastDoor => "Reinforced Blast Door (Breach Mover)",
            Self::FragileSkylightDome => "Fragile Skylight Dome (Overhead Hazard)",
            Self::VentGratingShatter => "Vent Grating Shatter (Duct Access)",
        }
    }

    pub fn to_config(&self, center: Vec3, size: Vec2) -> DestructibleConfig {
        match self {
            Self::BreakableWindowGlass => DestructibleConfig {
                name: "WindowGlass0".to_string(),
                archetype: *self,
                fracture_pattern: FracturePatternType::RadialSpiderweb,
                center,
                size,
                normal: Vec3::Y,
                health: 25.0,
                blast_force: 350.0,
                shard_count: 18,
                particle_scale: 1.0,
                sound_shatter: "Botpack.glass_break2".to_string(),
                texture_intact: "GenFluid.Glass.Window1".to_string(),
                texture_shards: "GenFluid.Glass.Shard1".to_string(),
                event_on_break: "Glass_Broken_Event".to_string(),
                auto_wire_bot_reachability: true,
                debris_bounces: 2,
            },
            Self::ExplodingStoneWall => DestructibleConfig {
                name: "ExplodingWall0".to_string(),
                archetype: *self,
                fracture_pattern: FracturePatternType::CellularVoronoi,
                center,
                size,
                normal: Vec3::X,
                health: 120.0,
                blast_force: 600.0,
                shard_count: 24,
                particle_scale: 1.8,
                sound_shatter: "UnrealShare.General.Explode1".to_string(),
                texture_intact: "Ancient.Wall.StoneWall1".to_string(),
                texture_shards: "Ancient.Rock.DebrisRock1".to_string(),
                event_on_break: "Wall_Breached_Event".to_string(),
                auto_wire_bot_reachability: true,
                debris_bounces: 3,
            },
            Self::ReinforcedBlastDoor => DestructibleConfig {
                name: "BlastDoor0".to_string(),
                archetype: *self,
                fracture_pattern: FracturePatternType::RectangularPanes,
                center,
                size,
                normal: Vec3::X,
                health: 300.0,
                blast_force: 800.0,
                shard_count: 12,
                particle_scale: 2.2,
                sound_shatter: "Botpack.Door.MetalCrush".to_string(),
                texture_intact: "Metal.Plate.BlastSteel1".to_string(),
                texture_shards: "Metal.Scrap.DebrisMetal1".to_string(),
                event_on_break: "BlastDoor_Compromised".to_string(),
                auto_wire_bot_reachability: true,
                debris_bounces: 2,
            },
            Self::FragileSkylightDome => DestructibleConfig {
                name: "SkylightDome0".to_string(),
                archetype: *self,
                fracture_pattern: FracturePatternType::DiagonalShatter,
                center,
                size,
                normal: Vec3::Z,
                health: 15.0,
                blast_force: 250.0,
                shard_count: 32,
                particle_scale: 0.8,
                sound_shatter: "Botpack.glass_break1".to_string(),
                texture_intact: "GenFluid.Glass.Skylight1".to_string(),
                texture_shards: "GenFluid.Glass.Shard1".to_string(),
                event_on_break: "Skylight_Fallen".to_string(),
                auto_wire_bot_reachability: false, // overhead falling glass
                debris_bounces: 1,
            },
            Self::VentGratingShatter => DestructibleConfig {
                name: "VentGrate0".to_string(),
                archetype: *self,
                fracture_pattern: FracturePatternType::RectangularPanes,
                center,
                size,
                normal: Vec3::Y,
                health: 40.0,
                blast_force: 400.0,
                shard_count: 8,
                particle_scale: 1.2,
                sound_shatter: "Botpack.Metal.ImpactGrating".to_string(),
                texture_intact: "Indus.Grate.IronVent1".to_string(),
                texture_shards: "Indus.Metal.BentRebar1".to_string(),
                event_on_break: "Vent_Opened".to_string(),
                auto_wire_bot_reachability: true,
                debris_bounces: 2,
            },
        }
    }
}

/// Geometric pattern used to subdivide the intact polygon into debris shards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FracturePatternType {
    RadialSpiderweb,
    CellularVoronoi,
    RectangularPanes,
    DiagonalShatter,
}

impl FracturePatternType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::RadialSpiderweb => "Radial Spiderweb (Bullet Impact)",
            Self::CellularVoronoi => "Cellular Voronoi (Concrete Blast)",
            Self::RectangularPanes => "Rectangular Panes (Window Grid)",
            Self::DiagonalShatter => "Diagonal Shatter (Stress Lines)",
        }
    }
}

/// Configuration parameters for a destructible barrier
#[derive(Debug, Clone, PartialEq)]
pub struct DestructibleConfig {
    pub name: String,
    pub archetype: DestructibleArchetype,
    pub fracture_pattern: FracturePatternType,
    pub center: Vec3,
    pub size: Vec2, // width (local X) and height (local Y)
    pub normal: Vec3,
    pub health: f32,
    pub blast_force: f32,
    pub shard_count: usize,
    pub particle_scale: f32,
    pub sound_shatter: String,
    pub texture_intact: String,
    pub texture_shards: String,
    pub event_on_break: String,
    pub auto_wire_bot_reachability: bool,
    pub debris_bounces: u8,
}

/// A 2D shard polygon generated on the barrier's surface
#[derive(Debug, Clone, PartialEq)]
pub struct ShardPolygon {
    pub id: usize,
    pub local_vertices: Vec<Vec2>,
    pub local_center: Vec2,
    pub area: f32,
    pub initial_velocity: Vec3,
}

/// Real-time 3D physics particle representing a flying debris shard
#[derive(Debug, Clone, PartialEq)]
pub struct DebrisParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation_yaw: f32,
    pub rotation_pitch: f32,
    pub angular_velocity: Vec3,
    pub scale: f32,
    pub lifetime_secs: f32,
    pub max_lifetime_secs: f32,
    pub bounce_count: u8,
    pub max_bounces: u8,
    pub is_at_rest: bool,
}

/// Real-time physics simulation of debris explosion
#[derive(Debug, Clone, PartialEq)]
pub struct DestructibleSimulation {
    pub particles: Vec<DebrisParticle>,
    pub floor_z: f32,
    pub is_shattered: bool,
    pub total_sim_time: f32,
}

impl DestructibleSimulation {
    pub fn new(floor_z: f32) -> Self {
        Self {
            particles: Vec::new(),
            floor_z,
            is_shattered: false,
            total_sim_time: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.particles.clear();
        self.is_shattered = false;
        self.total_sim_time = 0.0;
    }

    pub fn trigger_shatter(&mut self, config: &DestructibleConfig, shards: &[ShardPolygon]) {
        self.particles.clear();
        self.is_shattered = true;
        self.total_sim_time = 0.0;

        // Build right-handed orthonormal basis from normal
        let n = if config.normal.length_squared() > 0.001 {
            config.normal.normalize()
        } else {
            Vec3::Y
        };
        let u = if n.z.abs() < 0.9 {
            n.cross(Vec3::Z).normalize()
        } else {
            n.cross(Vec3::X).normalize()
        };
        let v = n.cross(u).normalize();

        for shard in shards {
            let world_pos = config.center + u * shard.local_center.x + v * shard.local_center.y;
            // Ejection velocity: combination of blast normal + lateral shard dispersion
            let outward_blast = n * config.blast_force;
            let lateral_disp = (u * shard.local_center.x + v * shard.local_center.y) * 2.5;
            let upward_boost = Vec3::Z * (config.blast_force * 0.35);
            let vel = outward_blast + lateral_disp + upward_boost;

            self.particles.push(DebrisParticle {
                position: world_pos,
                velocity: vel,
                rotation_yaw: (shard.id as f32 * 47.0) % 360.0,
                rotation_pitch: (shard.id as f32 * 31.0) % 360.0,
                angular_velocity: Vec3::new(
                    (shard.id as f32 * 13.0).sin() * 180.0,
                    (shard.id as f32 * 17.0).cos() * 180.0,
                    240.0,
                ),
                scale: config.particle_scale * (0.8 + (shard.id % 5) as f32 * 0.1),
                lifetime_secs: 0.0,
                max_lifetime_secs: 8.0,
                bounce_count: 0,
                max_bounces: config.debris_bounces,
                is_at_rest: false,
            });
        }
    }

    pub fn step(&mut self, dt: f32) {
        if !self.is_shattered {
            return;
        }

        self.total_sim_time += dt;

        for p in &mut self.particles {
            if p.is_at_rest {
                continue;
            }

            p.lifetime_secs += dt;
            if p.lifetime_secs >= p.max_lifetime_secs {
                p.is_at_rest = true;
                continue;
            }

            // Gravity integration
            p.velocity.z += UT99_GRAVITY * dt;

            // Drag
            p.velocity *= (1.0 - 0.25 * dt).max(0.0);

            // Position update
            p.position += p.velocity * dt;

            // Rotation update
            p.rotation_yaw = (p.rotation_yaw + p.angular_velocity.z * dt) % 360.0;
            p.rotation_pitch = (p.rotation_pitch + p.angular_velocity.x * dt) % 360.0;

            // Floor collision test
            if p.position.z <= self.floor_z {
                p.position.z = self.floor_z;
                if p.bounce_count < p.max_bounces && p.velocity.z.abs() > 40.0 {
                    p.bounce_count += 1;
                    p.velocity.z = -p.velocity.z * 0.42; // restitution
                    p.velocity.x *= 0.65; // friction
                    p.velocity.y *= 0.65;
                } else {
                    p.velocity = Vec3::ZERO;
                    p.angular_velocity = Vec3::ZERO;
                    p.is_at_rest = true;
                }
            }
        }
    }
}

// ============================================================================
// Algorithmic Routines: Fracture Generation & Hazard Audit
// ============================================================================

/// Generates a set of 2D convex shard polygons covering the rectangular bounds of the barrier
pub fn generate_fracture_shards(
    size: Vec2,
    pattern: FracturePatternType,
    shard_count: usize,
) -> Vec<ShardPolygon> {
    let mut shards = Vec::new();
    let hw = size.x * 0.5;
    let hh = size.y * 0.5;

    match pattern {
        FracturePatternType::RadialSpiderweb => {
            // Concentric rings & radial spoke wedges
            let rings = 3.max((shard_count as f32).sqrt().round() as usize);
            let spokes = (shard_count / rings).max(4);

            let center = Vec2::ZERO;
            for r in 0..rings {
                let r_inner = r as f32 / rings as f32;
                let r_outer = (r + 1) as f32 / rings as f32;

                for s in 0..spokes {
                    let a0 = (s as f32 / spokes as f32) * std::f32::consts::TAU;
                    let a1 = ((s + 1) as f32 / spokes as f32) * std::f32::consts::TAU;

                    let p0 = center + Vec2::new(a0.cos() * hw * r_inner, a0.sin() * hh * r_inner);
                    let p1 = center + Vec2::new(a1.cos() * hw * r_inner, a1.sin() * hh * r_inner);
                    let p2 = center + Vec2::new(a1.cos() * hw * r_outer, a1.sin() * hh * r_outer);
                    let p3 = center + Vec2::new(a0.cos() * hw * r_outer, a0.sin() * hh * r_outer);

                    let local_center = (p0 + p1 + p2 + p3) * 0.25;
                    let area = ((hw * r_outer) - (hw * r_inner)) * ((hh * r_outer) - (hh * r_inner));

                    shards.push(ShardPolygon {
                        id: shards.len(),
                        local_vertices: vec![p0, p1, p2, p3],
                        local_center,
                        area: area.abs(),
                        initial_velocity: Vec3::ZERO,
                    });
                }
            }
        }
        FracturePatternType::RectangularPanes => {
            let cols = (shard_count as f32).sqrt().ceil() as usize;
            let rows = (shard_count + cols - 1) / cols;

            let dw = size.x / cols as f32;
            let dh = size.y / rows as f32;

            for r in 0..rows {
                for c in 0..cols {
                    let x0 = -hw + c as f32 * dw;
                    let x1 = x0 + dw;
                    let y0 = -hh + r as f32 * dh;
                    let y1 = y0 + dh;

                    let p0 = Vec2::new(x0, y0);
                    let p1 = Vec2::new(x1, y0);
                    let p2 = Vec2::new(x1, y1);
                    let p3 = Vec2::new(x0, y1);

                    let local_center = Vec2::new((x0 + x1) * 0.5, (y0 + y1) * 0.5);
                    shards.push(ShardPolygon {
                        id: shards.len(),
                        local_vertices: vec![p0, p1, p2, p3],
                        local_center,
                        area: dw * dh,
                        initial_velocity: Vec3::ZERO,
                    });
                }
            }
        }
        FracturePatternType::CellularVoronoi | FracturePatternType::DiagonalShatter => {
            // Triangular / diagonal shards from quadrants with jitter
            let cols = (shard_count as f32).sqrt().ceil() as usize;
            let rows = (shard_count + cols - 1) / cols;
            let dw = size.x / cols as f32;
            let dh = size.y / rows as f32;

            for r in 0..rows {
                for c in 0..cols {
                    let x0 = -hw + c as f32 * dw;
                    let x1 = x0 + dw;
                    let y0 = -hh + r as f32 * dh;
                    let y1 = y0 + dh;

                    let mid = Vec2::new((x0 + x1) * 0.5, (y0 + y1) * 0.5);

                    // Split into two triangles
                    let tri1 = vec![Vec2::new(x0, y0), Vec2::new(x1, y0), mid];
                    let tri2 = vec![Vec2::new(x1, y0), Vec2::new(x1, y1), mid];
                    let tri3 = vec![Vec2::new(x1, y1), Vec2::new(x0, y1), mid];
                    let tri4 = vec![Vec2::new(x0, y1), Vec2::new(x0, y0), mid];

                    for tri in [tri1, tri2, tri3, tri4] {
                        let c_tri = (tri[0] + tri[1] + tri[2]) / 3.0;
                        shards.push(ShardPolygon {
                            id: shards.len(),
                            local_vertices: tri,
                            local_center: c_tri,
                            area: (dw * dh) * 0.25,
                            initial_velocity: Vec3::ZERO,
                        });
                        if shards.len() >= shard_count {
                            return shards;
                        }
                    }
                }
            }
        }
    }

    if shards.is_empty() {
        // Fallback quad
        shards.push(ShardPolygon {
            id: 0,
            local_vertices: vec![
                Vec2::new(-hw, -hh),
                Vec2::new(hw, -hh),
                Vec2::new(hw, hh),
                Vec2::new(-hw, hh),
            ],
            local_center: Vec2::ZERO,
            area: size.x * size.y,
            initial_velocity: Vec3::ZERO,
        });
    }

    shards
}

/// Hazard diagnostic severity for destructibles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestructibleHazardSeverity {
    Critical,
    Warning,
    Info,
}

/// Individual issue found during destructible audit
#[derive(Debug, Clone, PartialEq)]
pub struct DestructibleHazard {
    pub severity: DestructibleHazardSeverity,
    pub title: String,
    pub description: String,
    pub suggested_fix: String,
    pub location: Vec3,
}

/// Audits destructible configuration for game-breaking setup hazards
pub fn audit_destructible_hazards(config: &DestructibleConfig) -> Vec<DestructibleHazard> {
    let mut hazards = Vec::new();

    // 1. Health checks
    if config.health <= 0.0 {
        hazards.push(DestructibleHazard {
            severity: DestructibleHazardSeverity::Critical,
            title: "Zero or Negative Health Hazard".to_string(),
            description: format!("Barrier '{}' has health {:.1}. It will shatter immediately on match start.", config.name, config.health),
            suggested_fix: "Set health to at least 15.0 uu for fragile glass or 100.0+ for walls.".to_string(),
            location: config.center,
        });
    } else if config.health > 1500.0 {
        hazards.push(DestructibleHazard {
            severity: DestructibleHazardSeverity::Warning,
            title: "Excessive Barrier Health".to_string(),
            description: format!("Barrier has {:.0} HP, requiring dozens of rockets to breach.", config.health),
            suggested_fix: "Lower health to standard tournament thresholds (25-300 HP).".to_string(),
            location: config.center,
        });
    }

    // 2. Event string check
    if config.event_on_break.trim().is_empty() {
        hazards.push(DestructibleHazard {
            severity: DestructibleHazardSeverity::Warning,
            title: "Unnotified Breach Event".to_string(),
            description: format!("Barrier '{}' has an empty event_on_break string. Breaching will trigger zero level events.", config.name),
            suggested_fix: "Provide an Event tag to activate sirens, open doors, or notify bot AI.".to_string(),
            location: config.center,
        });
    }

    // 3. Shard count check
    if config.shard_count < 4 {
        hazards.push(DestructibleHazard {
            severity: DestructibleHazardSeverity::Warning,
            title: "Insufficient Shard Count".to_string(),
            description: format!("Shard count is {} (< 4), resulting in overly blocky destruction.", config.shard_count),
            suggested_fix: "Increase shard count to 12-32 for realistic shattered fragments.".to_string(),
            location: config.center,
        });
    } else if config.shard_count > 64 {
        hazards.push(DestructibleHazard {
            severity: DestructibleHazardSeverity::Warning,
            title: "High Debris Density Hazard".to_string(),
            description: format!("Shard count is {} (> 64), which may cause CPU/network lag in multiplayer.", config.shard_count),
            suggested_fix: "Cap shard count at 48 for optimal netcode performance in UT99.".to_string(),
            location: config.center,
        });
    }

    // 4. Blast force check
    if config.blast_force < 50.0 {
        hazards.push(DestructibleHazard {
            severity: DestructibleHazardSeverity::Warning,
            title: "Low Blast Ejection Force".to_string(),
            description: format!("Blast force is {:.0} uu/s. Debris will clump and fall straight down.", config.blast_force),
            suggested_fix: "Increase blast force to 300-600 uu/s for dramatic outward explosion.".to_string(),
            location: config.center,
        });
    }

    if hazards.is_empty() {
        hazards.push(DestructibleHazard {
            severity: DestructibleHazardSeverity::Info,
            title: "Destructible Barrier Verified".to_string(),
            description: format!("Barrier '{}' has valid health, explosive physics, shard tessellation, and event notification.", config.name),
            suggested_fix: "Ready for gameplay.".to_string(),
            location: config.center,
        });
    }

    hazards
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destructible_archetypes_and_defaults() {
        let center = Vec3::new(100.0, 200.0, 50.0);
        let size = Vec2::new(128.0, 256.0);

        let glass = DestructibleArchetype::BreakableWindowGlass.to_config(center, size);
        assert_eq!(glass.health, 25.0);
        assert_eq!(glass.fracture_pattern, FracturePatternType::RadialSpiderweb);
        assert!(glass.auto_wire_bot_reachability);

        let wall = DestructibleArchetype::ExplodingStoneWall.to_config(center, size);
        assert_eq!(wall.health, 120.0);
        assert_eq!(wall.fracture_pattern, FracturePatternType::CellularVoronoi);
        assert_eq!(wall.blast_force, 600.0);
    }

    #[test]
    fn test_fracture_shards_generation() {
        let size = Vec2::new(200.0, 100.0);

        // Radial Spiderweb
        let spider_shards = generate_fracture_shards(size, FracturePatternType::RadialSpiderweb, 16);
        assert!(spider_shards.len() >= 12);
        assert_eq!(spider_shards[0].local_vertices.len(), 4);

        // Rectangular Panes
        let pane_shards = generate_fracture_shards(size, FracturePatternType::RectangularPanes, 8);
        assert_eq!(pane_shards.len(), 9); // 3x3 grid for count 8
        assert_eq!(pane_shards[0].local_vertices.len(), 4);

        // Triangular/Voronoi
        let tri_shards = generate_fracture_shards(size, FracturePatternType::CellularVoronoi, 16);
        assert!(!tri_shards.is_empty());
        assert_eq!(tri_shards[0].local_vertices.len(), 3);
    }

    #[test]
    fn test_debris_physics_simulation_steps_and_floor_bounce() {
        let mut sim = DestructibleSimulation::new(0.0);
        let config = DestructibleArchetype::BreakableWindowGlass.to_config(
            Vec3::new(0.0, 0.0, 100.0),
            Vec2::new(64.0, 64.0),
        );
        let shards = generate_fracture_shards(config.size, config.fracture_pattern, 6);

        sim.trigger_shatter(&config, &shards);
        assert_eq!(sim.particles.len(), shards.len());
        assert!(sim.is_shattered);

        // Step simulation for 0.5s
        sim.step(0.5);
        assert!(sim.total_sim_time >= 0.5);

        // Step for another 2.0s until particles reach floor and bounce/rest
        for _ in 0..20 {
            sim.step(0.1);
        }
        for p in &sim.particles {
            assert!(p.position.z >= 0.0, "Particles must not penetrate below floor Z");
        }
    }

    #[test]
    fn test_audit_detects_zero_health_and_empty_event() {
        let mut config = DestructibleArchetype::BreakableWindowGlass.to_config(
            Vec3::ZERO,
            Vec2::new(64.0, 64.0),
        );
        config.health = 0.0;
        config.event_on_break = "   ".to_string();

        let hazards = audit_destructible_hazards(&config);
        assert!(hazards.iter().any(|h| h.severity == DestructibleHazardSeverity::Critical && h.title.contains("Zero or Negative Health")));
        assert!(hazards.iter().any(|h| h.severity == DestructibleHazardSeverity::Warning && h.title.contains("Unnotified Breach Event")));
    }

    #[test]
    fn test_audit_detects_excessive_shard_count() {
        let mut config = DestructibleArchetype::ExplodingStoneWall.to_config(
            Vec3::ZERO,
            Vec2::new(64.0, 64.0),
        );
        config.shard_count = 100;

        let hazards = audit_destructible_hazards(&config);
        assert!(hazards.iter().any(|h| h.severity == DestructibleHazardSeverity::Warning && h.title.contains("High Debris Density")));
    }
}
