use glam::Vec3;

/// Comprehensive fluid mechanics, viscosity, friction, and environmental hazard volume model
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ZoneFluidPhysics {
    pub b_water_zone: bool,
    pub b_pain_zone: bool,
    pub zone_velocity: Vec3,          // Water current / air draft vector in uu/s
    pub zone_ground_friction: f32,    // Ground movement friction (4.0 normal, 0.2 ice, 8.0 mud)
    pub zone_fluid_friction: f32,     // Medium drag / viscosity (1.2 water, 0.1 air, 4.0 thick slime)
    pub zone_terminal_velocity: f32,  // Maximum falling speed in zone (2500 air, 1000 water)
    pub zone_gravity: Vec3,           // Local gravity vector (standard UT99 is [0, 0, -950] uu/s²)
    pub damage_per_sec: i32,          // Periodic environmental damage
    pub damage_type: String,          // Unreal damage type ("None", "Burned", "Corroded", "Drowned", "Special")
    pub buoyancy_factor: f32,         // >1.0 floats upward to surface, <1.0 sinks downward
}

impl Default for ZoneFluidPhysics {
    fn default() -> Self {
        Self {
            b_water_zone: false,
            b_pain_zone: false,
            zone_velocity: Vec3::ZERO,
            zone_ground_friction: 4.0,
            zone_fluid_friction: 0.1,
            zone_terminal_velocity: 2500.0,
            zone_gravity: Vec3::new(0.0, 0.0, -950.0),
            damage_per_sec: 0,
            damage_type: "None".to_string(),
            buoyancy_factor: 0.85,
        }
    }
}

/// Canonical environmental fluid and hazard presets from Unreal Tournament 1999
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FluidZonePreset {
    CrystalLakeWater,
    LethalMoltenLava,
    ToxicBiohazardSlime,
    ZeroGravitySpace,
    CryogenicSlipperyIce,
    PneumaticVentilationDraft,
}

impl FluidZonePreset {
    pub fn all() -> &'static [FluidZonePreset] {
        &[
            FluidZonePreset::CrystalLakeWater,
            FluidZonePreset::LethalMoltenLava,
            FluidZonePreset::ToxicBiohazardSlime,
            FluidZonePreset::ZeroGravitySpace,
            FluidZonePreset::CryogenicSlipperyIce,
            FluidZonePreset::PneumaticVentilationDraft,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            FluidZonePreset::CrystalLakeWater => "💧 Crystal Lake Water (Swimmable Fluid / Buoyant)",
            FluidZonePreset::LethalMoltenLava => "🔥 Lethal Molten Lava (Extreme Heat / 40 DPS Burn)",
            FluidZonePreset::ToxicBiohazardSlime => "🧪 Toxic Biohazard Slime (Viscous Acid / 20 DPS Corrosive)",
            FluidZonePreset::ZeroGravitySpace => "🪐 Zero-Gravity Orbital Void (Inertial Drift / Microgravity)",
            FluidZonePreset::CryogenicSlipperyIce => "❄ Cryogenic Slippery Ice (Frictionless Skating / Low Drag)",
            FluidZonePreset::PneumaticVentilationDraft => "💨 Pneumatic Ventilation Draft (Conveyor Current / Fast Push)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            FluidZonePreset::CrystalLakeWater => {
                "Swimmable water zone with realistic buoyancy, terminal velocity damping (1000 uu/s) and muffled acoustics."
            }
            FluidZonePreset::LethalMoltenLava => {
                "Incinerating magma causing 40 damage per second. High fluid viscosity with instant player vaporisation hazard."
            }
            FluidZonePreset::ToxicBiohazardSlime => {
                "Thick industrial acid with heavy drag (4.0 friction) causing 20 corrosion damage per second."
            }
            FluidZonePreset::ZeroGravitySpace => {
                "Weightless cosmic vacuum. Zero gravity (0 uu/s²) and minimal friction allowing frictionless player projectile flight."
            }
            FluidZonePreset::CryogenicSlipperyIce => {
                "Ultra-low ground friction (0.15) for high-speed skating and momentum conservation across slick surfaces."
            }
            FluidZonePreset::PneumaticVentilationDraft => {
                "High-velocity pneumatic airflow pushing pawns and projectiles along a 1200 uu/s directional vector."
            }
        }
    }

    pub fn to_physics(&self) -> ZoneFluidPhysics {
        match self {
            FluidZonePreset::CrystalLakeWater => ZoneFluidPhysics {
                b_water_zone: true,
                b_pain_zone: false,
                zone_velocity: Vec3::ZERO,
                zone_ground_friction: 4.0,
                zone_fluid_friction: 1.2,
                zone_terminal_velocity: 1000.0,
                zone_gravity: Vec3::new(0.0, 0.0, -950.0),
                damage_per_sec: 0,
                damage_type: "Drowned".to_string(),
                buoyancy_factor: 1.15,
            },
            FluidZonePreset::LethalMoltenLava => ZoneFluidPhysics {
                b_water_zone: true,
                b_pain_zone: true,
                zone_velocity: Vec3::new(0.0, 0.0, -20.0),
                zone_ground_friction: 6.0,
                zone_fluid_friction: 3.5,
                zone_terminal_velocity: 600.0,
                zone_gravity: Vec3::new(0.0, 0.0, -950.0),
                damage_per_sec: 40,
                damage_type: "Burned".to_string(),
                buoyancy_factor: 1.4,
            },
            FluidZonePreset::ToxicBiohazardSlime => ZoneFluidPhysics {
                b_water_zone: true,
                b_pain_zone: true,
                zone_velocity: Vec3::new(0.0, 0.0, -10.0),
                zone_ground_friction: 5.0,
                zone_fluid_friction: 2.8,
                zone_terminal_velocity: 800.0,
                zone_gravity: Vec3::new(0.0, 0.0, -950.0),
                damage_per_sec: 20,
                damage_type: "Corroded".to_string(),
                buoyancy_factor: 1.05,
            },
            FluidZonePreset::ZeroGravitySpace => ZoneFluidPhysics {
                b_water_zone: false,
                b_pain_zone: false,
                zone_velocity: Vec3::ZERO,
                zone_ground_friction: 0.5,
                zone_fluid_friction: 0.02,
                zone_terminal_velocity: 4000.0,
                zone_gravity: Vec3::ZERO,
                damage_per_sec: 0,
                damage_type: "Suffocated".to_string(),
                buoyancy_factor: 1.0,
            },
            FluidZonePreset::CryogenicSlipperyIce => ZoneFluidPhysics {
                b_water_zone: false,
                b_pain_zone: false,
                zone_velocity: Vec3::ZERO,
                zone_ground_friction: 0.18,
                zone_fluid_friction: 0.08,
                zone_terminal_velocity: 2500.0,
                zone_gravity: Vec3::new(0.0, 0.0, -950.0),
                damage_per_sec: 0,
                damage_type: "None".to_string(),
                buoyancy_factor: 0.85,
            },
            FluidZonePreset::PneumaticVentilationDraft => ZoneFluidPhysics {
                b_water_zone: false,
                b_pain_zone: false,
                zone_velocity: Vec3::new(1200.0, 0.0, 300.0),
                zone_ground_friction: 2.0,
                zone_fluid_friction: 0.15,
                zone_terminal_velocity: 3000.0,
                zone_gravity: Vec3::new(0.0, 0.0, -600.0),
                damage_per_sec: 0,
                damage_type: "None".to_string(),
                buoyancy_factor: 1.0,
            },
        }
    }

    pub fn hazard_color_rgba(&self) -> [f32; 4] {
        match self {
            FluidZonePreset::CrystalLakeWater => [0.15, 0.55, 0.95, 0.45],
            FluidZonePreset::LethalMoltenLava => [1.0, 0.25, 0.05, 0.75],
            FluidZonePreset::ToxicBiohazardSlime => [0.20, 0.90, 0.20, 0.65],
            FluidZonePreset::ZeroGravitySpace => [0.65, 0.35, 0.95, 0.40],
            FluidZonePreset::CryogenicSlipperyIce => [0.70, 0.92, 1.0, 0.50],
            FluidZonePreset::PneumaticVentilationDraft => [0.95, 0.85, 0.25, 0.50],
        }
    }
}

/// Simulates a pawn trajectory over time within the fluid physics zone,
/// integrating gravity, buoyancy, fluid drag, and ambient zone velocity drift.
pub fn simulate_swimming_trajectory(
    initial_pos: Vec3,
    initial_vel: Vec3,
    physics: &ZoneFluidPhysics,
    dt_secs: f32,
    steps: usize,
) -> Vec<(Vec3, Vec3)> {
    let mut trajectory = Vec::with_capacity(steps);
    let mut pos = initial_pos;
    let mut vel = initial_vel;

    trajectory.push((pos, vel));

    for _ in 0..steps {
        // Net gravitational and Archimedes buoyant acceleration
        let g_accel = if physics.b_water_zone {
            physics.zone_gravity * (1.0 - physics.buoyancy_factor)
        } else {
            physics.zone_gravity
        };

        // Relative velocity to fluid current
        let rel_vel = vel - physics.zone_velocity;

        // Fluid drag force opposing relative motion
        let drag_accel = -rel_vel * physics.zone_fluid_friction;

        // Total acceleration
        let total_accel = g_accel + drag_accel;

        // Velocity integration
        vel += total_accel * dt_secs;

        // Terminal velocity clamp
        let speed = vel.length();
        if speed > physics.zone_terminal_velocity {
            vel = vel.normalize() * physics.zone_terminal_velocity;
        }

        // Position integration
        pos += vel * dt_secs;

        trajectory.push((pos, vel));
    }

    trajectory
}

/// Computes seconds until pawn death inside a pain zone given current health.
/// Returns None if the zone is not hazardous or does zero damage.
pub fn evaluate_player_survival_time(
    physics: &ZoneFluidPhysics,
    player_health: i32,
) -> Option<f32> {
    if !physics.b_pain_zone || physics.damage_per_sec <= 0 || player_health <= 0 {
        None
    } else {
        Some((player_health as f32) / (physics.damage_per_sec as f32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presets_coverage() {
        let presets = FluidZonePreset::all();
        assert_eq!(presets.len(), 6);

        for &p in presets {
            assert!(!p.display_name().is_empty());
            assert!(!p.description().is_empty());
            let phys = p.to_physics();
            assert!(phys.zone_terminal_velocity >= 500.0);
            assert!(phys.zone_fluid_friction >= 0.0);
        }
    }

    #[test]
    fn test_buoyancy_and_trajectory() {
        let water = FluidZonePreset::CrystalLakeWater.to_physics();
        assert!(water.b_water_zone);
        assert!(water.buoyancy_factor > 1.0, "Water must be buoyant");

        let traj = simulate_swimming_trajectory(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -100.0),
            &water,
            0.05,
            40,
        );

        assert_eq!(traj.len(), 41);
        let final_vel = traj.last().unwrap().1;
        // In buoyant water, sinking downward should decelerate and eventually turn upward
        assert!(final_vel.z > -50.0);
    }

    #[test]
    fn test_survival_time_in_lava_and_slime() {
        let lava = FluidZonePreset::LethalMoltenLava.to_physics();
        let survival_lava = evaluate_player_survival_time(&lava, 100);
        assert_eq!(survival_lava, Some(2.5)); // 100 HP / 40 DPS = 2.5 seconds

        let slime = FluidZonePreset::ToxicBiohazardSlime.to_physics();
        let survival_slime = evaluate_player_survival_time(&slime, 100);
        assert_eq!(survival_slime, Some(5.0)); // 100 HP / 20 DPS = 5.0 seconds

        let safe_water = FluidZonePreset::CrystalLakeWater.to_physics();
        assert_eq!(evaluate_player_survival_time(&safe_water, 100), None);
    }
}
