use glam::Vec3;

/// Environmental effect type for atmospheric Unreal Tournament 99 particle emitters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParticleEmitterType {
    #[default]
    Smoke,             // Industrial smoke / dust rising with drag
    Sparks,            // Electrical short-circuit sparks with gravity bounce
    Steam,             // High-velocity steam jet from pipes
    WaterSplash,       // Liquid droplet spray and waterfall mist
    TorchFire,         // Torches, wall braziers and furnace flames
    ToxicSlimeBubbles, // Radioactive acid slime bubbles (Deck16 style)
}

/// Canonical pre-calibrated Particle FX templates for UT99 environments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticlePresetTemplate {
    TorchFireFlames,
    SteamVentGeyser,
    WaterfallMist,
    SparkShower,
    DenseIndustrialSmoke,
    ToxicSlimeBubbles,
}

impl ParticlePresetTemplate {
    pub fn label(&self) -> &'static str {
        match self {
            ParticlePresetTemplate::TorchFireFlames => "Torch Fire & Brazier Flames",
            ParticlePresetTemplate::SteamVentGeyser => "High-Pressure Steam Vent Geyser",
            ParticlePresetTemplate::WaterfallMist => "Waterfall Mist & Droplets",
            ParticlePresetTemplate::SparkShower => "Short-Circuit Spark Shower (Bouncing)",
            ParticlePresetTemplate::DenseIndustrialSmoke => "Dense Toxic Industrial Smoke",
            ParticlePresetTemplate::ToxicSlimeBubbles => "Acid Slime Bubbles (Deck16)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ParticlePresetTemplate::TorchFireFlames => "Flickering amber/crimson flame core with quick heat dissipation for medieval torches.",
            ParticlePresetTemplate::SteamVentGeyser => "High-velocity white steam burst with wide expansion and gradual cloud dissipation.",
            ParticlePresetTemplate::WaterfallMist => "Gravitational cascade of cyan droplets with elastic splash bounce against surfaces.",
            ParticlePresetTemplate::SparkShower => "High-speed incandescent gold sparks that bounce and scatter realistically off floors.",
            ParticlePresetTemplate::DenseIndustrialSmoke => "Opaque dark charcoal smoke that rises slowly, expands 3x in volume, and drifts with wind.",
            ParticlePresetTemplate::ToxicSlimeBubbles => "Biohazard neon green acid bubbles rising gently from toxic sludge basins.",
        }
    }

    pub fn all() -> &'static [ParticlePresetTemplate] {
        &[
            ParticlePresetTemplate::TorchFireFlames,
            ParticlePresetTemplate::SteamVentGeyser,
            ParticlePresetTemplate::WaterfallMist,
            ParticlePresetTemplate::SparkShower,
            ParticlePresetTemplate::DenseIndustrialSmoke,
            ParticlePresetTemplate::ToxicSlimeBubbles,
        ]
    }

    /// Instantiates a pre-calibrated ParticleEmitter at the specified location
    pub fn create_emitter(&self, name: &str, location: Vec3) -> ParticleEmitter {
        match self {
            ParticlePresetTemplate::TorchFireFlames => ParticleEmitter {
                name: name.to_string(),
                location,
                emitter_type: ParticleEmitterType::TorchFire,
                emission_rate: 28.0,
                gravity: Vec3::new(0.0, 0.0, 75.0),
                drag: 0.08,
                initial_speed_min: 15.0,
                initial_speed_max: 35.0,
                max_particles: 120,
                particles: Vec::new(),
                spawn_accumulator: 0.0,
                size_start: 10.0,
                size_end: 2.0,
                color_start: [1.00, 0.80, 0.15, 0.95], // Bright yellow/gold
                color_end: [0.95, 0.20, 0.05, 0.00],   // Crimson red fadeout
                floor_z: None,
                bounce_damping: 0.0,
                wind_velocity: Vec3::new(0.0, 0.0, 0.0),
                spread_cone_degrees: 15.0,
            },
            ParticlePresetTemplate::SteamVentGeyser => ParticleEmitter {
                name: name.to_string(),
                location,
                emitter_type: ParticleEmitterType::Steam,
                emission_rate: 35.0,
                gravity: Vec3::new(0.0, 0.0, 140.0),
                drag: 0.12,
                initial_speed_min: 140.0,
                initial_speed_max: 260.0,
                max_particles: 200,
                particles: Vec::new(),
                spawn_accumulator: 0.0,
                size_start: 6.0,
                size_end: 28.0,
                color_start: [0.90, 0.95, 1.00, 0.85], // Crisp white/cyan steam
                color_end: [0.70, 0.75, 0.85, 0.00],
                floor_z: None,
                bounce_damping: 0.0,
                wind_velocity: Vec3::new(10.0, 0.0, 0.0),
                spread_cone_degrees: 20.0,
            },
            ParticlePresetTemplate::WaterfallMist => ParticleEmitter {
                name: name.to_string(),
                location,
                emitter_type: ParticleEmitterType::WaterSplash,
                emission_rate: 45.0,
                gravity: Vec3::new(0.0, 0.0, -650.0),
                drag: 0.01,
                initial_speed_min: 80.0,
                initial_speed_max: 180.0,
                max_particles: 250,
                particles: Vec::new(),
                spawn_accumulator: 0.0,
                size_start: 4.0,
                size_end: 12.0,
                color_start: [0.40, 0.80, 1.00, 0.90], // Aqua cyan
                color_end: [0.70, 0.90, 1.00, 0.00],
                floor_z: Some(location.z - 160.0),
                bounce_damping: 0.35,
                wind_velocity: Vec3::ZERO,
                spread_cone_degrees: 30.0,
            },
            ParticlePresetTemplate::SparkShower => ParticleEmitter {
                name: name.to_string(),
                location,
                emitter_type: ParticleEmitterType::Sparks,
                emission_rate: 50.0,
                gravity: Vec3::new(0.0, 0.0, -700.0),
                drag: 0.02,
                initial_speed_min: 160.0,
                initial_speed_max: 380.0,
                max_particles: 220,
                particles: Vec::new(),
                spawn_accumulator: 0.0,
                size_start: 4.0,
                size_end: 1.0,
                color_start: [1.00, 0.95, 0.40, 1.00], // Brilliant electrical gold
                color_end: [1.00, 0.40, 0.05, 0.00],   // Orange burn-out
                floor_z: Some(location.z - 120.0),
                bounce_damping: 0.65, // Elastic spark bounce
                wind_velocity: Vec3::ZERO,
                spread_cone_degrees: 60.0,
            },
            ParticlePresetTemplate::DenseIndustrialSmoke => ParticleEmitter {
                name: name.to_string(),
                location,
                emitter_type: ParticleEmitterType::Smoke,
                emission_rate: 16.0,
                gravity: Vec3::new(0.0, 0.0, 35.0),
                drag: 0.04,
                initial_speed_min: 18.0,
                initial_speed_max: 42.0,
                max_particles: 180,
                particles: Vec::new(),
                spawn_accumulator: 0.0,
                size_start: 8.0,
                size_end: 32.0, // Expands 4x
                color_start: [0.25, 0.25, 0.28, 0.90], // Charcoal smoke
                color_end: [0.45, 0.45, 0.50, 0.00],
                floor_z: None,
                bounce_damping: 0.0,
                wind_velocity: Vec3::new(15.0, 8.0, 0.0),
                spread_cone_degrees: 25.0,
            },
            ParticlePresetTemplate::ToxicSlimeBubbles => ParticleEmitter {
                name: name.to_string(),
                location,
                emitter_type: ParticleEmitterType::ToxicSlimeBubbles,
                emission_rate: 14.0,
                gravity: Vec3::new(0.0, 0.0, 25.0),
                drag: 0.06,
                initial_speed_min: 8.0,
                initial_speed_max: 22.0,
                max_particles: 90,
                particles: Vec::new(),
                spawn_accumulator: 0.0,
                size_start: 5.0,
                size_end: 14.0,
                color_start: [0.20, 1.00, 0.20, 0.95], // Radioactive lime green
                color_end: [0.10, 0.70, 0.15, 0.00],
                floor_z: None,
                bounce_damping: 0.0,
                wind_velocity: Vec3::ZERO,
                spread_cone_degrees: 45.0,
            },
        }
    }
}

/// A simulated 3D volumetric particle
#[derive(Debug, Clone, PartialEq)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub size: f32,
    pub color: [f32; 4], // RGBA
}

/// Dynamic particle emitter system for editor real-time simulation
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleEmitter {
    pub name: String,
    pub location: Vec3,
    pub emitter_type: ParticleEmitterType,
    pub emission_rate: f32,       // Particles generated per second
    pub gravity: Vec3,            // Acceleration (uu/s^2)
    pub drag: f32,                // Air resistance damping (0.0 .. 1.0)
    pub initial_speed_min: f32,
    pub initial_speed_max: f32,
    pub max_particles: usize,
    pub particles: Vec<Particle>,
    pub spawn_accumulator: f32,
    pub size_start: f32,
    pub size_end: f32,
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
    pub floor_z: Option<f32>,     // Elastic bounce collision plane
    pub bounce_damping: f32,      // Velocity retention on bounce (0.0 .. 1.0)
    pub wind_velocity: Vec3,      // Directional wind drift
    pub spread_cone_degrees: f32, // Angular emission spread
}

impl ParticleEmitter {
    pub fn new(name: impl Into<String>, location: Vec3, emitter_type: ParticleEmitterType) -> Self {
        let (gravity, rate, speed_min, speed_max, drag, col_st, col_ed, sz_st, sz_ed) = match emitter_type {
            ParticleEmitterType::Smoke => (
                Vec3::new(0.0, 0.0, 45.0), 12.0, 20.0, 45.0, 0.05,
                [0.70, 0.70, 0.70, 0.80], [0.50, 0.50, 0.55, 0.00], 10.0, 24.0,
            ),
            ParticleEmitterType::Sparks => (
                Vec3::new(0.0, 0.0, -650.0), 35.0, 160.0, 340.0, 0.02,
                [1.00, 0.90, 0.20, 1.00], [1.00, 0.35, 0.05, 0.00], 4.0, 1.0,
            ),
            ParticleEmitterType::Steam => (
                Vec3::new(0.0, 0.0, 120.0), 25.0, 120.0, 240.0, 0.15,
                [0.85, 0.90, 1.00, 0.80], [0.70, 0.75, 0.85, 0.00], 6.0, 22.0,
            ),
            ParticleEmitterType::WaterSplash => (
                Vec3::new(0.0, 0.0, -600.0), 40.0, 100.0, 250.0, 0.01,
                [0.40, 0.80, 1.00, 0.90], [0.70, 0.90, 1.00, 0.00], 4.0, 10.0,
            ),
            ParticleEmitterType::TorchFire => (
                Vec3::new(0.0, 0.0, 75.0), 28.0, 15.0, 35.0, 0.08,
                [1.00, 0.80, 0.15, 0.95], [0.95, 0.20, 0.05, 0.00], 10.0, 2.0,
            ),
            ParticleEmitterType::ToxicSlimeBubbles => (
                Vec3::new(0.0, 0.0, 25.0), 14.0, 8.0, 22.0, 0.06,
                [0.20, 1.00, 0.20, 0.95], [0.10, 0.70, 0.15, 0.00], 5.0, 14.0,
            ),
        };

        Self {
            name: name.into(),
            location,
            emitter_type,
            emission_rate: rate,
            gravity,
            drag,
            initial_speed_min: speed_min,
            initial_speed_max: speed_max,
            max_particles: 256,
            particles: Vec::new(),
            spawn_accumulator: 0.0,
            size_start: sz_st,
            size_end: sz_ed,
            color_start: col_st,
            color_end: col_ed,
            floor_z: None,
            bounce_damping: if emitter_type == ParticleEmitterType::Sparks { 0.6 } else { 0.0 },
            wind_velocity: Vec3::ZERO,
            spread_cone_degrees: 30.0,
        }
    }

    /// Advances particle simulation by timestep dt seconds with elastic collision and wind
    pub fn update(&mut self, dt: f32) {
        let step = dt.clamp(0.0, 0.1);

        // 1. Update existing particles
        for p in &mut self.particles {
            p.lifetime += step;
            p.velocity += (self.gravity + self.wind_velocity) * step;
            p.velocity *= (1.0 - self.drag * step).max(0.0);
            p.position += p.velocity * step;

            // Elastic Floor Collision
            if let Some(floor) = self.floor_z {
                if p.position.z <= floor && p.velocity.z < 0.0 {
                    p.position.z = floor;
                    p.velocity.z = -p.velocity.z * self.bounce_damping;
                    p.velocity.x *= 0.85; // Floor friction
                    p.velocity.y *= 0.85;
                }
            }

            // Life ratio interpolation (0.0 .. 1.0)
            let life_ratio = (p.lifetime / p.max_lifetime).clamp(0.0, 1.0);

            // Interpolate color and opacity
            for c in 0..4 {
                p.color[c] = self.color_start[c] + (self.color_end[c] - self.color_start[c]) * life_ratio;
            }

            // Interpolate size curve
            p.size = self.size_start + (self.size_end - self.size_start) * life_ratio;
        }

        // Retain alive particles
        self.particles.retain(|p| p.lifetime < p.max_lifetime);

        // 2. Spawn new particles based on emission rate
        self.spawn_accumulator += self.emission_rate * step;
        while self.spawn_accumulator >= 1.0 && self.particles.len() < self.max_particles {
            self.spawn_particle();
            self.spawn_accumulator -= 1.0;
        }
    }

    /// Spawns a single particle with initial randomized velocity vector inside spread cone
    fn spawn_particle(&mut self) {
        let count = self.particles.len();
        // Deterministic pseudo-random seed based on index
        let angle = (count as f32 * 2.39996) % std::f32::consts::TAU;
        let cone_rad = (self.spread_cone_degrees * 0.5).to_radians();
        let pitch = ((count as f32 * 1.618) % 1.0) * cone_rad;
        let speed = self.initial_speed_min + ((count as f32 * 3.1415) % 1.0) * (self.initial_speed_max - self.initial_speed_min);

        let dir = match self.emitter_type {
            ParticleEmitterType::Smoke | ParticleEmitterType::Steam | ParticleEmitterType::TorchFire | ParticleEmitterType::ToxicSlimeBubbles => {
                Vec3::new(angle.cos() * pitch.sin(), angle.sin() * pitch.sin(), pitch.cos().abs()).normalize()
            }
            ParticleEmitterType::Sparks => {
                Vec3::new(angle.cos() * 0.8, angle.sin() * 0.8, -pitch.cos().abs() - 0.2).normalize()
            }
            ParticleEmitterType::WaterSplash => {
                Vec3::new(angle.cos() * 0.6, angle.sin() * 0.6, -0.8).normalize()
            }
        };

        let max_life = match self.emitter_type {
            ParticleEmitterType::Smoke => 2.5,
            ParticleEmitterType::Sparks => 1.4,
            ParticleEmitterType::Steam => 1.2,
            ParticleEmitterType::WaterSplash => 1.0,
            ParticleEmitterType::TorchFire => 0.7,
            ParticleEmitterType::ToxicSlimeBubbles => 1.8,
        };

        self.particles.push(Particle {
            position: self.location,
            velocity: dir * speed,
            lifetime: 0.0,
            max_lifetime: max_life,
            size: self.size_start,
            color: self.color_start,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_particle_emitter_simulation() {
        let mut emitter = ParticleEmitter::new("StackSmoke", Vec3::new(0.0, 0.0, 100.0), ParticleEmitterType::Smoke);
        assert_eq!(emitter.particles.len(), 0);

        // Step simulation 0.5s -> should spawn particles rising upwards
        emitter.update(0.5);
        assert!(!emitter.particles.is_empty(), "Must emit smoke particles");

        let p0 = &emitter.particles[0];
        assert!(p0.position.z >= 100.0, "Smoke must rise upward in Z");
        assert!(p0.color[3] > 0.0, "Particle must have positive alpha");
    }

    #[test]
    fn test_sparks_emitter_gravity() {
        let mut emitter = ParticleEmitter::new("ShortCircuit", Vec3::new(0.0, 0.0, 200.0), ParticleEmitterType::Sparks);
        emitter.update(0.2);
        assert!(!emitter.particles.is_empty());

        let p0 = &emitter.particles[0];
        assert_eq!(p0.color[0], 1.0); // Yellow/gold spark
        assert!(emitter.gravity.z < 0.0, "Sparks must fall under gravity");
    }
}
