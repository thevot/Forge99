// crates/ut99-bsp/src/weather_studio.rs
//
// Interactive Dynamic Weather, Atmospheric Precipitation & Lightning Storm Studio
//
// Native 64-bit mathematical modeling, continuous precipitation kinetics,
// fractal lightning generation, speed-of-sound thunder delay calculation,
// and safety audit diagnostics for Unreal Tournament 99 / UnrealEd 2.0 / 2.1 supremacy.

use glam::Vec3;
use crate::{Brush, CsgOperation, Polygon};

/// Precipitation type for physical kinetics and visual behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PrecipitationType {
    TorrentialRain,
    GentleSnowfall,
    AcidHailstorm,
    AshVolcanicFallout,
    SandstormBlizzard,
}

impl PrecipitationType {
    pub fn display_name(&self) -> &'static str {
        match self {
            PrecipitationType::TorrentialRain => "Torrential Rain",
            PrecipitationType::GentleSnowfall => "Gentle Snowfall",
            PrecipitationType::AcidHailstorm => "Acid Hailstorm",
            PrecipitationType::AshVolcanicFallout => "Ash Volcanic Fallout",
            PrecipitationType::SandstormBlizzard => "Sandstorm Blizzard",
        }
    }

    pub fn default_fall_speed(&self) -> f32 {
        match self {
            PrecipitationType::TorrentialRain => 1200.0,
            PrecipitationType::GentleSnowfall => 180.0,
            PrecipitationType::AcidHailstorm => 950.0,
            PrecipitationType::AshVolcanicFallout => 140.0,
            PrecipitationType::SandstormBlizzard => 600.0,
        }
    }

    pub fn default_particle_color(&self) -> [f32; 3] {
        match self {
            PrecipitationType::TorrentialRain => [0.65, 0.85, 1.0],
            PrecipitationType::GentleSnowfall => [0.95, 0.98, 1.0],
            PrecipitationType::AcidHailstorm => [0.35, 0.95, 0.35],
            PrecipitationType::AshVolcanicFallout => [0.95, 0.45, 0.15],
            PrecipitationType::SandstormBlizzard => [0.85, 0.70, 0.45],
        }
    }
}

/// Pre-calibrated canonical atmospheric weather archetypes for UT99 / Unreal 1
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WeatherArchetype {
    FrigateOceanicStorm,
    ChizraMonsoonDeluge,
    MorpheusBlizzardDrift,
    NaPaliToxicAcidRain,
    PhobosMeteorAshFallout,
}

impl WeatherArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            WeatherArchetype::FrigateOceanicStorm => "AS-Frigate Oceanic Storm",
            WeatherArchetype::ChizraMonsoonDeluge => "Chizra Monsoon Deluge",
            WeatherArchetype::MorpheusBlizzardDrift => "DM-Morpheus Blizzard Drift",
            WeatherArchetype::NaPaliToxicAcidRain => "Na Pali Toxic Acid Rain",
            WeatherArchetype::PhobosMeteorAshFallout => "DM-Phobos Meteor Ash Fallout",
        }
    }

    pub fn to_config(&self) -> WeatherConfig {
        match self {
            WeatherArchetype::FrigateOceanicStorm => WeatherConfig {
                archetype: *self,
                precipitation_type: PrecipitationType::TorrentialRain,
                position: Vec3::new(0.0, 0.0, 512.0),
                volume_extents: Vec3::new(2048.0, 2048.0, 1024.0),
                particle_density: 180.0,
                fall_speed_uu_s: 1300.0,
                wind_vector: Vec3::new(450.0, 80.0, -100.0),
                turbulence: 0.25,
                lightning_enabled: true,
                lightning_interval_secs: 8.0,
                lightning_color_rgb: [0.85, 0.90, 1.0],
                fog_density: 0.0012,
                fog_color_rgb: [0.15, 0.20, 0.30],
                ambient_sound_name: "AmbOutside.Rain.RainLoop01".to_string(),
                thunder_sound_name: "AmbOutside.Thunder.ThunderCrack01".to_string(),
                tag: "WeatherFrigate".to_string(),
                lightning_flash_radius: 1800.0,
            },
            WeatherArchetype::ChizraMonsoonDeluge => WeatherConfig {
                archetype: *self,
                precipitation_type: PrecipitationType::TorrentialRain,
                position: Vec3::new(0.0, 0.0, 400.0),
                volume_extents: Vec3::new(1536.0, 1536.0, 800.0),
                particle_density: 250.0,
                fall_speed_uu_s: 1400.0,
                wind_vector: Vec3::new(120.0, -150.0, 0.0),
                turbulence: 0.15,
                lightning_enabled: true,
                lightning_interval_secs: 12.0,
                lightning_color_rgb: [0.95, 0.95, 0.85],
                fog_density: 0.0018,
                fog_color_rgb: [0.25, 0.35, 0.25],
                ambient_sound_name: "AmbOutside.Rain.MonsoonHeavy".to_string(),
                thunder_sound_name: "AmbOutside.Thunder.RollingThunder".to_string(),
                tag: "WeatherChizra".to_string(),
                lightning_flash_radius: 1500.0,
            },
            WeatherArchetype::MorpheusBlizzardDrift => WeatherConfig {
                archetype: *self,
                precipitation_type: PrecipitationType::GentleSnowfall,
                position: Vec3::new(0.0, 0.0, 600.0),
                volume_extents: Vec3::new(3072.0, 3072.0, 1200.0),
                particle_density: 120.0,
                fall_speed_uu_s: 180.0,
                wind_vector: Vec3::new(300.0, 600.0, -50.0),
                turbulence: 0.65,
                lightning_enabled: false,
                lightning_interval_secs: 0.0,
                lightning_color_rgb: [0.9, 0.95, 1.0],
                fog_density: 0.0008,
                fog_color_rgb: [0.55, 0.65, 0.85],
                ambient_sound_name: "AmbOutside.Wind.BlizzardHowl".to_string(),
                thunder_sound_name: "".to_string(),
                tag: "WeatherMorpheus".to_string(),
                lightning_flash_radius: 0.0,
            },
            WeatherArchetype::NaPaliToxicAcidRain => WeatherConfig {
                archetype: *self,
                precipitation_type: PrecipitationType::AcidHailstorm,
                position: Vec3::new(0.0, 0.0, 512.0),
                volume_extents: Vec3::new(1792.0, 1792.0, 900.0),
                particle_density: 140.0,
                fall_speed_uu_s: 900.0,
                wind_vector: Vec3::new(-200.0, 50.0, -80.0),
                turbulence: 0.30,
                lightning_enabled: true,
                lightning_interval_secs: 15.0,
                lightning_color_rgb: [0.35, 1.0, 0.35],
                fog_density: 0.0020,
                fog_color_rgb: [0.10, 0.40, 0.15],
                ambient_sound_name: "AmbOutside.Rain.AcidSizzle".to_string(),
                thunder_sound_name: "AmbOutside.Thunder.ChemicalClap".to_string(),
                tag: "WeatherAcidRain".to_string(),
                lightning_flash_radius: 1600.0,
            },
            WeatherArchetype::PhobosMeteorAshFallout => WeatherConfig {
                archetype: *self,
                precipitation_type: PrecipitationType::AshVolcanicFallout,
                position: Vec3::new(0.0, 0.0, 512.0),
                volume_extents: Vec3::new(2560.0, 2560.0, 1024.0),
                particle_density: 90.0,
                fall_speed_uu_s: 130.0,
                wind_vector: Vec3::new(150.0, -400.0, 20.0),
                turbulence: 0.55,
                lightning_enabled: true,
                lightning_interval_secs: 18.0,
                lightning_color_rgb: [1.0, 0.55, 0.20],
                fog_density: 0.0014,
                fog_color_rgb: [0.45, 0.20, 0.10],
                ambient_sound_name: "AmbOutside.Wind.AshWindGust".to_string(),
                thunder_sound_name: "AmbOutside.Thunder.DistantRumble".to_string(),
                tag: "WeatherPhobosAsh".to_string(),
                lightning_flash_radius: 2000.0,
            },
        }
    }
}

/// Dynamic atmospheric weather configuration parameters
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WeatherConfig {
    pub archetype: WeatherArchetype,
    pub precipitation_type: PrecipitationType,
    pub position: Vec3,
    pub volume_extents: Vec3,
    pub particle_density: f32, // particles per 1000^3 uu
    pub fall_speed_uu_s: f32,
    pub wind_vector: Vec3,
    pub turbulence: f32, // 0.0 to 1.0
    pub lightning_enabled: bool,
    pub lightning_interval_secs: f32,
    pub lightning_color_rgb: [f32; 3],
    pub fog_density: f32,
    pub fog_color_rgb: [f32; 3],
    pub ambient_sound_name: String,
    pub thunder_sound_name: String,
    pub tag: String,
    pub lightning_flash_radius: f32,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        WeatherArchetype::FrigateOceanicStorm.to_config()
    }
}

/// Simulated procedural lightning strike branch
#[derive(Debug, Clone, PartialEq)]
pub struct LightningBranch {
    pub points: Vec<Vec3>,
    pub intensity: f32,
    pub thickness: f32,
}

/// Calculated metrics and analytical telemetry of the weather simulation
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherMetrics {
    pub volume_cubed_uu: f32,
    pub total_simulated_particles: usize,
    pub estimated_visibility_uu: f32,
    pub thunder_delay_at_listener_secs: f32,
    pub net_wind_speed_uu_s: f32,
    pub lightning_cycle_phase: f32,
    pub is_flashing_now: bool,
    pub flash_brightness: f32,
}

/// Diagnostic health inspection for atmospheric weather setup
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

/// Evaluates real-time kinematics, simulated lightning timing, and acoustics
pub fn calculate_weather_metrics(
    config: &WeatherConfig,
    simulated_time_secs: f32,
    listener_pos: Vec3,
) -> WeatherMetrics {
    let vol_uu = (config.volume_extents.x * config.volume_extents.y * config.volume_extents.z).max(1.0);
    let volume_m3_scale = vol_uu / 1_000_000_000.0;
    let total_simulated_particles = ((config.particle_density * volume_m3_scale * 100.0) as usize).clamp(50, 10_000);

    // Visibility estimation from fog density and particle density
    let fog_factor = (config.fog_density * 1000.0).clamp(0.01, 10.0);
    let density_factor = (config.particle_density / 100.0).clamp(0.2, 5.0);
    let estimated_visibility_uu = (2500.0 / (fog_factor * density_factor)).clamp(256.0, 16384.0);

    // Lightning cycle and flash
    let mut lightning_cycle_phase = 0.0;
    let mut is_flashing_now = false;
    let mut flash_brightness = 0.0;
    let mut thunder_delay_at_listener_secs = 0.0;

    if config.lightning_enabled && config.lightning_interval_secs > 0.1 {
        let period = config.lightning_interval_secs;
        lightning_cycle_phase = (simulated_time_secs % period) / period;

        // Lightning strikes in the first 0.25 seconds of each period
        let strike_time = simulated_time_secs % period;
        if strike_time < 0.25 {
            is_flashing_now = true;
            // Quick multi-flash strobe decay
            let pulse = (strike_time * 40.0).sin().abs();
            flash_brightness = ((1.0 - strike_time / 0.25) * 0.7 + pulse * 0.3).clamp(0.0, 1.0);

            // Speed of sound: 343 m/s = ~13720 uu/s in Unreal Tournament
            let strike_origin = config.position + Vec3::new(0.0, 0.0, config.volume_extents.z * 0.5);
            let dist_to_listener = (strike_origin - listener_pos).length();
            thunder_delay_at_listener_secs = dist_to_listener / 13720.0;
        }
    }

    let net_wind_speed_uu_s = config.wind_vector.length();

    WeatherMetrics {
        volume_cubed_uu: vol_uu,
        total_simulated_particles,
        estimated_visibility_uu,
        thunder_delay_at_listener_secs,
        net_wind_speed_uu_s,
        lightning_cycle_phase,
        is_flashing_now,
        flash_brightness,
    }
}

/// Generates a procedural 3D fractal lightning bolt polyline branch
pub fn generate_lightning_bolt(
    cloud_origin: Vec3,
    ground_target: Vec3,
    subdivisions: usize,
    jitter_scale: f32,
    seed: u32,
) -> Vec<Vec3> {
    let mut points = vec![cloud_origin, ground_target];

    let mut current_jitter = jitter_scale;
    let mut rng_state = seed.wrapping_mul(1664525).wrapping_add(1013904223);

    for _ in 0..subdivisions {
        let mut next_points = Vec::with_capacity(points.len() * 2);
        for i in 0..points.len() - 1 {
            let p0 = points[i];
            let p1 = points[i + 1];
            let mid = (p0 + p1) * 0.5;

            // Simple pseudo-random hash for displacement
            rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
            let rx = ((rng_state & 0xFFFF) as f32 / 32768.0 - 1.0) * current_jitter;
            rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
            let ry = ((rng_state & 0xFFFF) as f32 / 32768.0 - 1.0) * current_jitter;

            let displaced = mid + Vec3::new(rx, ry, 0.0);
            next_points.push(p0);
            next_points.push(displaced);
        }
        if let Some(&last) = points.last() {
            next_points.push(last);
        }
        points = next_points;
        current_jitter *= 0.55;
    }

    points
}

/// Audits safety and validity of the atmospheric weather configuration
pub fn audit_weather_safety(config: &WeatherConfig) -> Vec<WeatherHazard> {
    let mut hazards = Vec::new();

    // 1. Extreme hurricane wind speed (>2000 uu/s)
    if config.wind_vector.length() > 2000.0 {
        hazards.push(WeatherHazard {
            title: "Extreme Hurricane Wind Velocity".to_string(),
            description: format!(
                "Wind speed {:.0} uu/s exceeds 2000 uu/s. Player movement will be severely swept sideways.",
                config.wind_vector.length()
            ),
            is_critical: true,
        });
    }

    // 2. Sub-miniature volume extent (<256 uu)
    if config.volume_extents.x < 256.0 || config.volume_extents.y < 256.0 || config.volume_extents.z < 256.0 {
        hazards.push(WeatherHazard {
            title: "Sub-Miniature Weather Volume".to_string(),
            description: "Volume dimensions are under 256 uu. Precipitation will feel cramped or invisible.".to_string(),
            is_critical: true,
        });
    }

    // 3. Stagnant terminal fall speed (<= 0 uu/s)
    if config.fall_speed_uu_s <= 0.0 {
        hazards.push(WeatherHazard {
            title: "Zero or Negative Terminal Fall Speed".to_string(),
            description: "Precipitation particles have 0 fall speed and will float motionless in air.".to_string(),
            is_critical: true,
        });
    }

    // 4. Strobe / epilepsy hazard for rapid lightning (< 0.8s)
    if config.lightning_enabled && config.lightning_interval_secs < 0.8 {
        hazards.push(WeatherHazard {
            title: "Rapid Lightning Strobe Hazard".to_string(),
            description: format!(
                "Lightning interval {:.2}s is below 0.8s. Rapid flashes pose a visual discomfort/epilepsy risk.",
                config.lightning_interval_secs
            ),
            is_critical: false,
        });
    }

    // 5. Unbound weather tag
    if config.tag.trim().is_empty() {
        hazards.push(WeatherHazard {
            title: "Empty Weather Volume Tag".to_string(),
            description: "Weather configuration has an empty tag identifier, preventing level scripting access.".to_string(),
            is_critical: false,
        });
    }

    hazards
}

/// Generates a 6-faced bounding precipitation volume World brush (PF_NOT_SOLID | PF_TRANSLUCENT)
pub fn generate_weather_volume_brush(config: &WeatherConfig) -> Brush {
    let hw = config.volume_extents.x * 0.5;
    let hl = config.volume_extents.y * 0.5;
    let hh = config.volume_extents.z * 0.5;

    let v = [
        config.position + Vec3::new(-hw, -hl, -hh), // 0: ---
        config.position + Vec3::new( hw, -hl, -hh), // 1: +--
        config.position + Vec3::new( hw,  hl, -hh), // 2: ++-
        config.position + Vec3::new(-hw,  hl, -hh), // 3: -+-
        config.position + Vec3::new(-hw, -hl,  hh), // 4: --+
        config.position + Vec3::new( hw, -hl,  hh), // 5: +-+
        config.position + Vec3::new( hw,  hl,  hh), // 6: +++
        config.position + Vec3::new(-hw,  hl,  hh), // 7: -++
    ];

    let poly_flags = crate::PF_NOT_SOLID | crate::PF_TRANSLUCENT;
    let mut brush = Brush::new(&format!("{}_Volume", config.tag), CsgOperation::Add);

    // Helper to build a quad polygon
    let mut add_quad = |pts: Vec<Vec3>| {
        if let Some(mut poly) = Polygon::new(pts) {
            poly.texture = "GenFX.RainVolume01".to_string();
            poly.flags = poly_flags;
            brush.polys.push(poly);
        }
    };

    // Bottom face (-Z)
    add_quad(vec![v[3], v[2], v[1], v[0]]);
    // Top face (+Z)
    add_quad(vec![v[4], v[5], v[6], v[7]]);
    // Front face (-Y)
    add_quad(vec![v[0], v[1], v[5], v[4]]);
    // Back face (+Y)
    add_quad(vec![v[2], v[3], v[7], v[6]]);
    // Left face (-X)
    add_quad(vec![v[3], v[0], v[4], v[7]]);
    // Right face (+X)
    add_quad(vec![v[1], v[2], v[6], v[5]]);

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_archetypes_and_defaults() {
        let storm = WeatherArchetype::FrigateOceanicStorm.to_config();
        assert_eq!(storm.precipitation_type, PrecipitationType::TorrentialRain);
        assert!(storm.lightning_enabled);
        assert!(storm.fall_speed_uu_s >= 1000.0);

        let snow = WeatherArchetype::MorpheusBlizzardDrift.to_config();
        assert_eq!(snow.precipitation_type, PrecipitationType::GentleSnowfall);
        assert!(!snow.lightning_enabled);
        assert!(snow.turbulence > 0.5);

        let acid = WeatherArchetype::NaPaliToxicAcidRain.to_config();
        assert_eq!(acid.precipitation_type, PrecipitationType::AcidHailstorm);
        assert_eq!(acid.lightning_color_rgb[1], 1.0); // Green dominant
    }

    #[test]
    fn test_calculate_weather_metrics_and_thunder_delay() {
        let storm = WeatherArchetype::FrigateOceanicStorm.to_config();
        let listener = Vec3::new(0.0, 0.0, 0.0);

        // At time 0.05, lightning should be flashing
        let m_flash = calculate_weather_metrics(&storm, 0.05, listener);
        assert!(m_flash.is_flashing_now);
        assert!(m_flash.flash_brightness > 0.0);
        assert!(m_flash.thunder_delay_at_listener_secs > 0.0);

        // At time 2.0, flash is finished
        let m_quiet = calculate_weather_metrics(&storm, 2.0, listener);
        assert!(!m_quiet.is_flashing_now);
        assert_eq!(m_quiet.flash_brightness, 0.0);
    }

    #[test]
    fn test_generate_lightning_bolt_fractal_geometry() {
        let p0 = Vec3::new(0.0, 0.0, 1000.0);
        let p1 = Vec3::new(100.0, 50.0, 0.0);
        let bolt = generate_lightning_bolt(p0, p1, 4, 80.0, 1337);

        assert_eq!(bolt.first().copied(), Some(p0));
        assert_eq!(bolt.last().copied(), Some(p1));
        assert!(bolt.len() > 10, "Subdivided bolt must have multiple branch vertices");
    }

    #[test]
    fn test_weather_safety_audit() {
        let mut cfg = WeatherConfig::default();

        // 1. Extreme hurricane wind
        cfg.wind_vector = Vec3::new(2500.0, 0.0, 0.0);
        let h1 = audit_weather_safety(&cfg);
        assert!(h1.iter().any(|h| h.is_critical && h.title.contains("Hurricane Wind")));

        // 2. Zero fall speed
        cfg.wind_vector = Vec3::new(100.0, 0.0, 0.0);
        cfg.fall_speed_uu_s = 0.0;
        let h2 = audit_weather_safety(&cfg);
        assert!(h2.iter().any(|h| h.is_critical && h.title.contains("Zero or Negative")));

        // 3. Strobe hazard
        cfg.fall_speed_uu_s = 1000.0;
        cfg.lightning_enabled = true;
        cfg.lightning_interval_secs = 0.4;
        let h3 = audit_weather_safety(&cfg);
        assert!(h3.iter().any(|h| !h.is_critical && h.title.contains("Rapid Lightning Strobe")));
    }

    #[test]
    fn test_generate_weather_volume_brush() {
        let cfg = WeatherArchetype::ChizraMonsoonDeluge.to_config();
        let brush = generate_weather_volume_brush(&cfg);

        assert_eq!(brush.polys.len(), 6);
        for p in &brush.polys {
            assert!(p.flags & crate::PF_NOT_SOLID != 0);
            assert!(p.flags & crate::PF_TRANSLUCENT != 0);
        }
    }
}
