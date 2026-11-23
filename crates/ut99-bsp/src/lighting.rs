use crate::brush::Brush;
use crate::poly::{LightmapData, PF_NOT_SOLID, PF_PORTAL, PF_TRANSLUCENT, PF_UNLIT};
use glam::Vec3;
use rayon::prelude::*;
use std::time::Instant;

/// Canonical Unreal Engine 1 Light Styles / Types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    Steady = 0,
    Pulse = 1,
    Blink = 2,
    Flicker = 3,
    Strobe = 4,
    BackdropLight = 5,
    SubtlePulse = 6,
    TexturePaletteOnce = 7,
    TexturePaletteLoop = 8,
}

impl From<u8> for LightType {
    fn from(val: u8) -> Self {
        match val {
            1 => LightType::Pulse,
            2 => LightType::Blink,
            3 => LightType::Flicker,
            4 => LightType::Strobe,
            5 => LightType::BackdropLight,
            6 => LightType::SubtlePulse,
            7 => LightType::TexturePaletteOnce,
            8 => LightType::TexturePaletteLoop,
            _ => LightType::Steady,
        }
    }
}

/// Canonical Unreal Engine 1 Light Effects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightEffect {
    None = 0,
    TorchWaver = 1,
    FireWaver = 2,
    WateryShimmer = 3,
    Searchlight = 4,
    SlowWave = 5,
    FastWave = 6,
    CloudCast = 7,
    StaticSpot = 8,
    Shock = 9,
    Disco = 10,
    Cylinder = 11,
    Spotlight = 12,
    NonIncidence = 13,
}

impl From<u8> for LightEffect {
    fn from(val: u8) -> Self {
        match val {
            1 => LightEffect::TorchWaver,
            2 => LightEffect::FireWaver,
            3 => LightEffect::WateryShimmer,
            4 => LightEffect::Searchlight,
            5 => LightEffect::SlowWave,
            6 => LightEffect::FastWave,
            7 => LightEffect::CloudCast,
            8 => LightEffect::StaticSpot,
            9 => LightEffect::Shock,
            10 => LightEffect::Disco,
            11 | 17 => LightEffect::Cylinder,
            12 => LightEffect::Spotlight,
            13 => LightEffect::NonIncidence,
            _ => LightEffect::None,
        }
    }
}

/// Evaluates directional/effect-aware attenuation for a UE1 `SceneLight` at surface point `pt` with normal `eff_normal`.
#[inline]
fn compute_light_factor(light: &SceneLight, pt: Vec3, eff_normal: Vec3, world_radius: f32) -> f32 {
    let to_light = light.location - pt;
    let dist = to_light.length();
    if dist >= world_radius || dist <= 1e-4 {
        return 0.0;
    }
    let light_dir = to_light / dist;
    let raw_ndotl = eff_normal.dot(light_dir);
    if raw_ndotl <= 0.0 {
        return 0.0;
    }

    let effect = LightEffect::from(light.light_effect);
    let n_dot_l = match effect {
        LightEffect::NonIncidence => 1.0,
        _ => raw_ndotl,
    };

    let eff_dist = match effect {
        LightEffect::Cylinder => Vec3::new(to_light.x, to_light.y, 0.0).length(),
        _ => dist,
    };
    if eff_dist >= world_radius {
        return 0.0;
    }
    let norm_dist = eff_dist / world_radius;
    let mut atten = (1.0 - norm_dist).powi(2);

    if matches!(effect, LightEffect::StaticSpot | LightEffect::Spotlight) {
        // Downward cone along -Z: ray from light to pt is -light_dir, so dot(-light_dir, -Z) == +light_dir.z
        let cos_theta = light_dir.z;
        let cone_norm = (light.cone.max(8) as f32) / 255.0;
        let cos_cutoff = (1.0 - cone_norm * 1.5).clamp(-0.5, 0.98);
        if cos_theta <= cos_cutoff {
            return 0.0;
        }
        let spot_falloff = ((cos_theta - cos_cutoff) / (1.0 - cos_cutoff).max(0.05)).clamp(0.0, 1.0);
        atten *= spot_falloff * spot_falloff;
    }

    atten * n_dot_l * 2.0
}

/// Scene light actor representation for the lighting compiler.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneLight {
    pub name: String,
    pub location: Vec3,
    pub brightness: u8,
    pub radius: u8,
    pub hue: u8,
    pub saturation: u8,
    pub light_type: u8,
    pub light_effect: u8,
    pub period: u8,
    pub phase: u8,
    pub cone: u8,
    pub is_special_lit: bool,
}

impl Default for SceneLight {
    fn default() -> Self {
        Self {
            name: "Light".to_string(),
            location: Vec3::ZERO,
            brightness: 64,
            radius: 32,
            hue: 0,
            saturation: 255, // white default
            light_type: 0,
            light_effect: 0,
            period: 32,
            phase: 0,
            cone: 64,
            is_special_lit: false,
        }
    }
}

/// Quality profile for multithreaded Rayon lighting compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LightingBakeQuality {
    DraftFast,
    #[default]
    StandardTournament,
    ProductionRadiosity,
}

impl LightingBakeQuality {
    pub fn label(&self) -> &'static str {
        match self {
            LightingBakeQuality::DraftFast => "Draft Fast (64 uu, Direct Only)",
            LightingBakeQuality::StandardTournament => "Standard Tournament (32 uu, Soft Shadows)",
            LightingBakeQuality::ProductionRadiosity => "Production Radiosity (16 uu, 2-Bounce Diffuse)",
        }
    }
}

/// Configuration settings for the multithreaded lighting compiler.
#[derive(Debug, Clone)]
pub struct LightingConfig {
    pub ambient_color: Vec3,
    pub enable_shadows: bool,
    pub lightmap_resolution: f32, // World units per luxel (e.g. 16.0 or 32.0)
    pub enable_overbright: bool,
    pub quality: LightingBakeQuality,
    pub radiosity_bounces: u8,
    pub radiosity_albedo: f32,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            ambient_color: Vec3::new(0.04, 0.04, 0.05),
            enable_shadows: true,
            lightmap_resolution: 32.0,
            enable_overbright: true,
            quality: LightingBakeQuality::StandardTournament,
            radiosity_bounces: 0,
            radiosity_albedo: 0.25,
        }
    }
}

/// Statistics returned after light compilation.
#[derive(Debug, Clone, Default)]
pub struct LightingStats {
    pub total_polygons: usize,
    pub total_luxels: usize,
    pub shadow_rays_cast: usize,
    pub elapsed_ms: u128,
}

/// Converts UT99 Light Hue, Saturation, and Brightness into normalized linear RGB.
/// Note: UT99 saturation is inverted: Saturation 0 is fully saturated chromatic color,
/// Saturation 255 is pure neutral white.
pub fn ut99_light_to_rgb(hue: u8, sat: u8, brightness: u8) -> Vec3 {
    let h = (hue as f32) / 255.0 * 6.0;
    let s = 1.0 - ((sat as f32) / 255.0);
    let v = (brightness as f32) / 255.0;

    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Vec3::new(r + m, g + m, b + m)
}

/// Evaluates light style animation intensity factor at given time in seconds.
pub fn evaluate_light_style(light: &SceneLight, time: f32) -> f32 {
    let l_type = LightType::from(light.light_type);
    let period_sec = (light.period as f32 / 32.0).max(0.05);
    let phase_norm = light.phase as f32 / 255.0;
    let t = (time / period_sec) + phase_norm;
    let cycle_pos = t.fract();

    match l_type {
        LightType::Steady => 1.0,
        LightType::Pulse => {
            let angle = cycle_pos * std::f32::consts::TAU;
            0.5 + 0.5 * angle.cos()
        }
        LightType::Blink => {
            if cycle_pos < 0.5 {
                1.0
            } else {
                0.0
            }
        }
        LightType::Flicker => {
            // Deterministic pseudo-random flicker based on integer hash
            let step = (t * 12.0) as u32;
            let mut h = step.wrapping_mul(0x45d9f3b);
            h = ((h >> 16) ^ h).wrapping_mul(0x45d9f3b);
            h = (h >> 16) ^ h;
            let rand_val = (h & 0xFF) as f32 / 255.0;
            0.35 + 0.65 * rand_val
        }
        LightType::Strobe => {
            if cycle_pos < 0.15 {
                1.0
            } else {
                0.0
            }
        }
        LightType::SubtlePulse => {
            let angle = cycle_pos * std::f32::consts::TAU;
            0.80 + 0.20 * angle.cos()
        }
        _ => 1.0,
    }
}

/// Real-time dynamic light evaluation state
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicLightState {
    pub intensity_multiplier: f32,
    pub effective_brightness: u8,
    pub effective_rgb: Vec3,
    pub spot_direction: Vec3,
    pub spot_cone_angle_rad: f32,
    pub spot_inner_cone_rad: f32,
    pub current_radius_uu: f32,
    pub is_directional: bool,
}

/// Canonical dynamic lighting presets inspired by iconic UT99 environments
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DynamicLightPreset {
    EerieFlickeringTorch,
    EmergencyAlarmStrobe,
    CyberSearchlightRotor,
    SubmergedWaterCaustics,
    IndustrialFluorescentGlitch,
    DiscoDanceParty,
}

impl DynamicLightPreset {
    pub fn all() -> &'static [DynamicLightPreset] {
        &[
            DynamicLightPreset::EerieFlickeringTorch,
            DynamicLightPreset::EmergencyAlarmStrobe,
            DynamicLightPreset::CyberSearchlightRotor,
            DynamicLightPreset::SubmergedWaterCaustics,
            DynamicLightPreset::IndustrialFluorescentGlitch,
            DynamicLightPreset::DiscoDanceParty,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DynamicLightPreset::EerieFlickeringTorch => "🔥 Eerie Flickering Torch (Cathedral/Crypt)",
            DynamicLightPreset::EmergencyAlarmStrobe => "🚨 Emergency Alarm Strobe (Cyber Red Alert)",
            DynamicLightPreset::CyberSearchlightRotor => "🔦 Cyber Searchlight Rotor (Tower Prison)",
            DynamicLightPreset::SubmergedWaterCaustics => "🌊 Submerged Water Caustics (Oceanic Cyan)",
            DynamicLightPreset::IndustrialFluorescentGlitch => "⚡ Industrial Fluorescent Glitch (Factory)",
            DynamicLightPreset::DiscoDanceParty => "🪩 Disco Dance Party (Polychromatic Loop)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            DynamicLightPreset::EerieFlickeringTorch => {
                "Multi-harmonic organic flame turbulence with warm amber temperature shifts."
            }
            DynamicLightPreset::EmergencyAlarmStrobe => {
                "High-intensity sharp red pulse with 15% active duty cycle for alerts."
            }
            DynamicLightPreset::CyberSearchlightRotor => {
                "Continuous 360-degree rotating directional spotlight beam with penumbra falloff."
            }
            DynamicLightPreset::SubmergedWaterCaustics => {
                "Underwater wave interference simulation with oscillating azure-cyan luminescence."
            }
            DynamicLightPreset::IndustrialFluorescentGlitch => {
                "Fluorescent starter misfire with low-frequency voltage sag and cold phosphor tint."
            }
            DynamicLightPreset::DiscoDanceParty => {
                "Fast chromatic spectrum sweep through full 360-degree HSV rainbow palette."
            }
        }
    }

    pub fn apply_to_light(&self, light: &mut SceneLight) {
        match self {
            DynamicLightPreset::EerieFlickeringTorch => {
                light.light_type = LightType::Steady as u8;
                light.light_effect = LightEffect::TorchWaver as u8;
                light.period = 32;
                light.phase = 0;
                light.cone = 0;
                light.brightness = 170;
                light.hue = 26; // Amber orange
                light.saturation = 20;
                light.radius = 48;
            }
            DynamicLightPreset::EmergencyAlarmStrobe => {
                light.light_type = LightType::Strobe as u8;
                light.light_effect = LightEffect::None as u8;
                light.period = 16; // Fast cycle
                light.phase = 0;
                light.cone = 0;
                light.brightness = 250;
                light.hue = 0; // Pure red
                light.saturation = 0;
                light.radius = 64;
            }
            DynamicLightPreset::CyberSearchlightRotor => {
                light.light_type = LightType::Steady as u8;
                light.light_effect = LightEffect::Searchlight as u8;
                light.period = 48; // Smooth sweep
                light.phase = 0;
                light.cone = 45; // Tight spotlight cone
                light.brightness = 230;
                light.hue = 150; // Cyan-white
                light.saturation = 100;
                light.radius = 96;
            }
            DynamicLightPreset::SubmergedWaterCaustics => {
                light.light_type = LightType::SubtlePulse as u8;
                light.light_effect = LightEffect::WateryShimmer as u8;
                light.period = 40;
                light.phase = 14;
                light.cone = 0;
                light.brightness = 140;
                light.hue = 135; // Cyan
                light.saturation = 35;
                light.radius = 52;
            }
            DynamicLightPreset::IndustrialFluorescentGlitch => {
                light.light_type = LightType::Flicker as u8;
                light.light_effect = LightEffect::SlowWave as u8;
                light.period = 22;
                light.phase = 45;
                light.cone = 0;
                light.brightness = 185;
                light.hue = 75; // Cold green-white
                light.saturation = 210;
                light.radius = 44;
            }
            DynamicLightPreset::DiscoDanceParty => {
                light.light_type = LightType::Pulse as u8;
                light.light_effect = LightEffect::Disco as u8;
                light.period = 18;
                light.phase = 0;
                light.cone = 0;
                light.brightness = 210;
                light.hue = 0;
                light.saturation = 0;
                light.radius = 58;
            }
        }
    }
}

/// Evaluates dynamic light parameters (intensity, color, spotlight direction/cone, radius)
/// continuously at time `time_sec`.
pub fn evaluate_dynamic_light(light: &SceneLight, time_sec: f32) -> DynamicLightState {
    let period_sec = (light.period as f32 / 32.0).max(0.05);
    let phase_norm = light.phase as f32 / 255.0;
    let t = (time_sec / period_sec) + phase_norm;
    let cycle_pos = t.fract();
    let l_eff = LightEffect::from(light.light_effect);

    // 1. Base light style evaluation (Pulse, Blink, Strobe, Flicker, etc.)
    let base_style_intensity = evaluate_light_style(light, time_sec);

    // 2. Light effect modulation
    let mut effect_mult = 1.0_f32;
    let mut eff_hue = light.hue;
    let mut eff_sat = light.saturation;
    let mut spot_direction = Vec3::new(0.0, 0.0, -1.0);
    let mut is_directional = false;

    match l_eff {
        LightEffect::None => {}
        LightEffect::TorchWaver => {
            // Organic multi-harmonic flame flicker with temperature shift
            let h1 = (t * 7.3 * std::f32::consts::TAU).sin() * 0.16;
            let h2 = (t * 19.1 * std::f32::consts::TAU + 1.2).sin() * 0.10;
            let h3 = (t * 31.4 * std::f32::consts::TAU).cos() * 0.06;
            effect_mult = (1.0 + h1 + h2 + h3).max(0.2);
            // Slight amber shift when dipping lower
            if effect_mult < 0.9 {
                eff_hue = eff_hue.saturating_sub(4);
            }
        }
        LightEffect::FireWaver => {
            // Slower, rolling larger flame surges
            let f1 = (t * 4.8 * std::f32::consts::TAU).sin() * 0.22;
            let f2 = (t * 10.7 * std::f32::consts::TAU + 0.9).cos() * 0.12;
            effect_mult = (0.88 + f1 + f2).max(0.3);
        }
        LightEffect::WateryShimmer => {
            // Caustic wave interference
            let w1 = (t * 3.14 * std::f32::consts::TAU).sin();
            let w2 = (t * 4.60 * std::f32::consts::TAU).cos();
            effect_mult = (0.78 + 0.22 * w1 * w2).max(0.4);
            // Cyan/azure saturation boost
            eff_sat = eff_sat.saturating_sub(15);
        }
        LightEffect::Searchlight => {
            is_directional = true;
            let angle = cycle_pos * std::f32::consts::TAU;
            spot_direction = Vec3::new(angle.cos(), angle.sin(), -0.35).normalize();
            effect_mult = 1.0;
        }
        LightEffect::SlowWave => {
            effect_mult = 0.65 + 0.35 * (cycle_pos * std::f32::consts::TAU).sin();
        }
        LightEffect::FastWave => {
            effect_mult = 0.50 + 0.50 * (cycle_pos * std::f32::consts::TAU * 2.0).sin();
        }
        LightEffect::CloudCast => {
            effect_mult = 0.70 + 0.30 * (cycle_pos * std::f32::consts::PI).sin();
        }
        LightEffect::StaticSpot | LightEffect::Spotlight => {
            is_directional = true;
            spot_direction = Vec3::new(0.0, 0.0, -1.0);
            effect_mult = 1.0;
        }
        LightEffect::Shock => {
            // Shockwave burst and fade
            let decay = (1.0 - cycle_pos).powi(2);
            effect_mult = decay * 1.5;
        }
        LightEffect::Disco => {
            // Chromatic hue rotation across full spectrum
            let hue_offset = (cycle_pos * 255.0) as u16;
            eff_hue = ((light.hue as u16 + hue_offset) % 256) as u8;
            eff_sat = 0; // Pure vivid chromatic color
            effect_mult = 1.0;
        }
        LightEffect::Cylinder | LightEffect::NonIncidence => {
            effect_mult = 1.0;
        }
    }

    let combined_mult = (base_style_intensity * effect_mult).clamp(0.0, 2.5);
    let eff_brightness = ((light.brightness as f32 * combined_mult).clamp(0.0, 255.0)) as u8;
    let base_rgb = ut99_light_to_rgb(eff_hue, eff_sat, eff_brightness);
    let effective_rgb = base_rgb * combined_mult.max(0.05);

    let cone_val = if light.cone == 0 { 64 } else { light.cone };
    let cone_factor = (cone_val as f32 / 255.0).clamp(0.05, 1.0);
    let spot_cone_angle_rad = cone_factor * (std::f32::consts::PI * 0.45);
    let spot_inner_cone_rad = spot_cone_angle_rad * 0.70;

    let current_radius_uu = light.radius as f32 * 25.0 * (0.85 + 0.15 * combined_mult.min(1.0));

    DynamicLightState {
        intensity_multiplier: combined_mult,
        effective_brightness: eff_brightness,
        effective_rgb,
        spot_direction,
        spot_cone_angle_rad,
        spot_inner_cone_rad,
        current_radius_uu,
        is_directional,
    }
}

/// Samples the light intensity waveform over a specified duration in seconds
pub fn sample_light_waveform(
    light: &SceneLight,
    duration_sec: f32,
    sample_count: usize,
) -> Vec<(f32, f32)> {
    let count = sample_count.max(2);
    let mut samples = Vec::with_capacity(count);
    let dt = duration_sec / (count - 1) as f32;

    for i in 0..count {
        let t = i as f32 * dt;
        let state = evaluate_dynamic_light(light, t);
        samples.push((t, state.intensity_multiplier));
    }

    samples
}

/// Möller–Trumbore ray-triangle intersection algorithm.
#[inline]
pub fn ray_intersects_triangle(
    ray_origin: Vec3,
    ray_dir: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<f32> {
    const EPSILON: f32 = 1e-6;
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let pvec = ray_dir.cross(edge2);
    let det = edge1.dot(pvec);

    if det.abs() < EPSILON {
        return None;
    }

    let inv_det = 1.0 / det;
    let tvec = ray_origin - v0;
    let u = tvec.dot(pvec) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let qvec = tvec.cross(edge1);
    let v = ray_dir.dot(qvec) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = edge2.dot(qvec) * inv_det;
    if t > EPSILON {
        Some(t)
    } else {
        None
    }
}

/// Tests whether the line of sight from point to light is occluded by any opaque geometry.
pub fn is_occluded(
    from: Vec3,
    to: Vec3,
    normal: Vec3,
    occluders: &[[Vec3; 3]],
) -> bool {
    let to_light = to - from;
    let dist = to_light.length();
    if dist < 1e-4 {
        return false;
    }

    let dir = to_light / dist;
    // Bias start point away from surface and along ray direction to prevent self-shadowing at shared brush corners
    let start = from + normal * 0.5 + dir * 2.0;

    for tri in occluders {
        if let Some(t) = ray_intersects_triangle(start, dir, tri[0], tri[1], tri[2]) {
            if t > 2.0 && t < dist - 4.0 {
                return true;
            }
        }
    }
    false
}

/// Gathers all opaque shadow-casting triangles from post-CSG world polygons so subtracted
/// doorways, windows, and cavities do not falsely block light rays.
fn gather_post_csg_occluders(brushes: &[Brush]) -> Vec<[Vec3; 3]> {
    let mut occluders: Vec<[Vec3; 3]> = Vec::new();
    let csg_polys = crate::bsp::evaluate_csg_brushes(brushes);
    for poly in csg_polys {
        // Skip non-solid, portals, translucent, invisible
        if (poly.flags & (PF_NOT_SOLID | PF_PORTAL | PF_TRANSLUCENT | crate::poly::PF_INVISIBLE)) != 0 {
            continue;
        }
        if poly.vertices.len() >= 3 {
            let v0 = poly.vertices[0];
            for i in 1..poly.vertices.len() - 1 {
                occluders.push([v0, poly.vertices[i], poly.vertices[i + 1]]);
            }
        }
    }
    occluders
}

/// Multithreaded raytraced light compiler using Rayon.
/// Bakes vertex lighting and surface lightmaps for all world brushes.
pub fn bake_lighting_for_brushes(
    brushes: &mut [Brush],
    lights: &[SceneLight],
    config: &LightingConfig,
) -> LightingStats {
    let start_time = Instant::now();

    // 1. Gather all occluder triangles from post-CSG opaque world polygons
    let occluders = gather_post_csg_occluders(brushes);

    let total_polys: usize = brushes.iter().map(|b| b.polys.len()).sum();
    let mut total_luxels = 0;
    let mut rays_cast = 0;

    // 2. Parallel lighting computation per brush
    for brush in brushes.iter_mut() {
        let rad_pitch = brush.rotation.x.to_radians();
        let rad_yaw = brush.rotation.y.to_radians();
        let rad_roll = brush.rotation.z.to_radians();
        let rot_matrix = glam::Mat4::from_euler(glam::EulerRot::YXZ, rad_yaw, rad_pitch, rad_roll);
        let scale_matrix = glam::Mat4::from_scale(brush.scale);
        let trans_matrix = glam::Mat4::from_translation(brush.location);
        let model_matrix = trans_matrix * rot_matrix * scale_matrix;
        let is_subtract = brush.csg_oper == crate::CsgOperation::Subtract;
        let occluders_ref = &occluders;
        let config_ref = config;

        // Process all polygons of this brush in parallel
        brush.polys.par_iter_mut().for_each(|poly| {
            let n = poly.vertices.len();
            if n < 3 {
                return;
            }

            let normal = model_matrix.transform_vector3(poly.plane.normal).normalize_or_zero();
            let poly_special_lit = (poly.flags & crate::poly::PF_SPECIAL_LIT) != 0;
            let is_two_sided_or_sub = (poly.flags & crate::poly::PF_TWO_SIDED) != 0 || is_subtract;

            // Check if surface is unlit
            if (poly.flags & PF_UNLIT) != 0 {
                poly.vertex_colors = vec![[1.0, 1.0, 1.0]; n];
                poly.lightmap = None;
                return;
            }

            let poly_center = poly
                .vertices
                .iter()
                .fold(Vec3::ZERO, |acc, v| acc + model_matrix.transform_point3(*v))
                / (n as f32);

            // A. Bake Vertex Lighting
            let mut v_colors = Vec::with_capacity(n);
            for v in &poly.vertices {
                let world_v = model_matrix.transform_point3(*v);
                let sample_v = world_v.lerp(poly_center, 0.35);
                let mut accumulated = config_ref.ambient_color;

                for light in lights {
                    if light.is_special_lit != poly_special_lit {
                        continue;
                    }

                    let world_radius = (light.radius as f32) * 25.0;
                    let mut best_factor = 0.0_f32;

                    for (pt, weight) in [(world_v, 1.0_f32), (sample_v, 0.85_f32)] {
                        let to_light = light.location - pt;
                        let dist = to_light.length();
                        if dist >= world_radius || dist <= 1e-4 {
                            continue;
                        }
                        let light_dir = to_light / dist;
                        let eff_normal = if is_two_sided_or_sub && normal.dot(light_dir) < 0.0 {
                            -normal
                        } else {
                            normal
                        };
                        if config_ref.enable_shadows && is_occluded(pt, light.location, eff_normal, occluders_ref) {
                            continue;
                        }
                        let factor = compute_light_factor(light, pt, eff_normal, world_radius);
                        if factor > 0.0 {
                            best_factor = best_factor.max(factor * weight);
                        }
                    }

                    if best_factor > 0.0 {
                        let light_col = ut99_light_to_rgb(light.hue, light.saturation, light.brightness);
                        accumulated += light_col * best_factor;
                    }
                }

                if !config_ref.enable_overbright {
                    accumulated = accumulated.min(Vec3::ONE);
                }
                v_colors.push([accumulated.x, accumulated.y, accumulated.z]);
            }
            poly.vertex_colors = v_colors;

            // B. Bake Lightmap Grid
            let world_base = model_matrix.transform_point3(poly.base);
            let world_tu = model_matrix.transform_vector3(poly.texture_u);
            let u_dir = if world_tu.length_squared() > 1e-6 {
                world_tu.normalize()
            } else {
                Vec3::X
            };
            let world_tv = model_matrix.transform_vector3(poly.texture_v);
            let v_dir = if world_tv.length_squared() > 1e-6 {
                world_tv.normalize()
            } else {
                Vec3::Y
            };

            let mut min_u = f32::MAX;
            let mut max_u = f32::MIN;
            let mut min_v = f32::MAX;
            let mut max_v = f32::MIN;

            for v in &poly.vertices {
                let d = model_matrix.transform_point3(*v) - world_base;
                let pu = d.dot(u_dir);
                let pv = d.dot(v_dir);
                min_u = min_u.min(pu);
                max_u = max_u.max(pu);
                min_v = min_v.min(pv);
                max_v = max_v.max(pv);
            }

            let base_res = config_ref.lightmap_resolution.max(8.0);
            let (res, max_dim) = if (poly.flags & crate::poly::PF_HIGH_SHADOW_DETAIL) != 0 {
                ((base_res * 0.5).max(4.0), 64usize)
            } else if (poly.flags & crate::poly::PF_LOW_SHADOW_DETAIL) != 0 {
                (base_res * 2.0, 16usize)
            } else {
                (base_res, 32usize)
            };
            let w_lux = (((max_u - min_u) / res).ceil() as usize).clamp(2, max_dim);
            let h_lux = (((max_v - min_v) / res).ceil() as usize).clamp(2, max_dim);

            let mut luxels = Vec::with_capacity(w_lux * h_lux);
            for y in 0..h_lux {
                let v_pos = min_v + (y as f32 + 0.5) * res;
                for x in 0..w_lux {
                    let u_pos = min_u + (x as f32 + 0.5) * res;
                    let world_luxel = world_base + u_dir * u_pos + v_dir * v_pos;
                    let mut col = config_ref.ambient_color;

                    for light in lights {
                        if light.is_special_lit != poly_special_lit {
                            continue;
                        }

                        let world_radius = (light.radius as f32) * 25.0;
                        let to_light = light.location - world_luxel;
                        let dist = to_light.length();

                        if dist >= world_radius || dist <= 1e-4 {
                            continue;
                        }

                        let light_dir = to_light / dist;
                        let eff_normal = if is_two_sided_or_sub && normal.dot(light_dir) < 0.0 {
                            -normal
                        } else {
                            normal
                        };

                        if config_ref.enable_shadows && is_occluded(world_luxel, light.location, eff_normal, occluders_ref) {
                            continue;
                        }

                        let factor = compute_light_factor(light, world_luxel, eff_normal, world_radius);
                        if factor > 0.0 {
                            let light_col = ut99_light_to_rgb(light.hue, light.saturation, light.brightness);
                            col += light_col * factor;
                        }
                    }

                    if !config_ref.enable_overbright {
                        col = col.min(Vec3::ONE);
                    }
                    luxels.push([col.x, col.y, col.z]);
                }
            }

            poly.lightmap = Some(LightmapData {
                width: w_lux,
                height: h_lux,
                luxels,
            });
        });

        // Count totals
        for poly in &brush.polys {
            if let Some(ref lm) = poly.lightmap {
                total_luxels += lm.luxels.len();
                rays_cast += lm.luxels.len() * lights.len();
            }
        }
    }

    // 3. Multi-bounce diffuse radiosity pass (when configured or in ProductionRadiosity mode)
    rays_cast += apply_radiosity_bounces(brushes, &occluders, config);

    let elapsed = start_time.elapsed().as_millis();
    LightingStats {
        total_polygons: total_polys,
        total_luxels,
        shadow_rays_cast: rays_cast,
        elapsed_ms: elapsed,
    }
}

#[derive(Clone)]
struct RadiosityPatch {
    brush_idx: usize,
    poly_idx: usize,
    centroid: Vec3,
    normal: Vec3,
    area: f32,
    energy: Vec3,
}

fn apply_radiosity_bounces(
    brushes: &mut [Brush],
    occluders: &[[Vec3; 3]],
    config: &LightingConfig,
) -> usize {
    let bounces = if config.radiosity_bounces > 0 {
        config.radiosity_bounces
    } else if config.quality == LightingBakeQuality::ProductionRadiosity {
        2
    } else {
        0
    };
    if bounces == 0 {
        return 0;
    }

    let albedo = config.radiosity_albedo.clamp(0.05, 0.95);
    let mut patches: Vec<RadiosityPatch> = Vec::new();

    for (b_idx, brush) in brushes.iter().enumerate() {
        let rad_pitch = brush.rotation.x.to_radians();
        let rad_yaw = brush.rotation.y.to_radians();
        let rad_roll = brush.rotation.z.to_radians();
        let rot_matrix = glam::Mat4::from_euler(glam::EulerRot::YXZ, rad_yaw, rad_pitch, rad_roll);
        let scale_matrix = glam::Mat4::from_scale(brush.scale);
        let trans_matrix = glam::Mat4::from_translation(brush.location);
        let model_matrix = trans_matrix * rot_matrix * scale_matrix;
        let is_sub = brush.csg_oper == crate::CsgOperation::Subtract;

        for (p_idx, poly) in brush.polys.iter().enumerate() {
            if poly.vertices.len() < 3 || (poly.flags & PF_UNLIT) != 0 {
                continue;
            }
            let world_verts: Vec<Vec3> = poly
                .vertices
                .iter()
                .map(|&v| model_matrix.transform_point3(v))
                .collect();
            let centroid = world_verts.iter().copied().sum::<Vec3>() / (world_verts.len() as f32);
            let mut normal = model_matrix.transform_vector3(poly.plane.normal).normalize_or_zero();
            if is_sub {
                normal = -normal;
            }
            let p0 = world_verts[0];
            let mut area_vec = Vec3::ZERO;
            for i in 1..(world_verts.len() - 1) {
                area_vec += (world_verts[i] - p0).cross(world_verts[i + 1] - p0);
            }
            let area = (area_vec.length() * 0.5).max(1.0);

            let avg_energy = if !poly.vertex_colors.is_empty() {
                let sum = poly
                    .vertex_colors
                    .iter()
                    .fold(Vec3::ZERO, |acc, c| acc + Vec3::from_array(*c));
                (sum / (poly.vertex_colors.len() as f32) - config.ambient_color).max(Vec3::ZERO)
            } else {
                Vec3::ZERO
            };

            patches.push(RadiosityPatch {
                brush_idx: b_idx,
                poly_idx: p_idx,
                centroid,
                normal,
                area,
                energy: avg_energy,
            });
        }
    }

    if patches.len() < 2 {
        return 0;
    }

    let mut extra_rays = 0usize;
    let mut accumulated_indirect = vec![Vec3::ZERO; patches.len()];

    for _bounce in 0..bounces {
        let prev_patches = patches.clone();
        let next_energies: Vec<(Vec3, usize)> = (0..patches.len())
            .into_par_iter()
            .map(|i| {
                let pi = &prev_patches[i];
                let mut gathered = Vec3::ZERO;
                let mut local_rays = 0usize;

                for (j, pj) in prev_patches.iter().enumerate() {
                    if i == j || pj.energy.length_squared() < 1e-5 {
                        continue;
                    }
                    let delta = pj.centroid - pi.centroid;
                    let dist_sq = delta.length_squared();
                    if dist_sq < 4.0 {
                        continue;
                    }
                    let dist = dist_sq.sqrt();
                    let dir = delta / dist;
                    let cos_i = pi.normal.dot(dir);
                    let cos_j = pj.normal.dot(-dir);
                    if cos_i <= 0.01 || cos_j <= 0.01 {
                        continue;
                    }
                    if config.enable_shadows {
                        local_rays += 1;
                        if is_occluded(pi.centroid, pj.centroid, pi.normal, occluders) {
                            continue;
                        }
                    }
                    let form_factor =
                        ((cos_i * cos_j * pj.area) / (std::f32::consts::PI * dist_sq + pj.area))
                            .clamp(0.0, 0.35);
                    gathered += pj.energy * (form_factor * albedo);
                }
                (gathered, local_rays)
            })
            .collect();

        for (i, (ind, r_cnt)) in next_energies.into_iter().enumerate() {
            accumulated_indirect[i] += ind;
            patches[i].energy = ind;
            extra_rays += r_cnt;
        }
    }

    for (idx, patch) in patches.iter().enumerate() {
        let bounce_rgb = accumulated_indirect[idx];
        if bounce_rgb.length_squared() < 1e-6 {
            continue;
        }
        let poly = &mut brushes[patch.brush_idx].polys[patch.poly_idx];
        for vc in &mut poly.vertex_colors {
            let mut c = Vec3::from_array(*vc) + bounce_rgb;
            if !config.enable_overbright {
                c = c.min(Vec3::ONE);
            }
            *vc = [c.x, c.y, c.z];
        }
        if let Some(ref mut lm) = poly.lightmap {
            for lux in &mut lm.luxels {
                let mut c = Vec3::from_array(*lux) + bounce_rgb;
                if !config.enable_overbright {
                    c = c.min(Vec3::ONE);
                }
                *lux = [c.x, c.y, c.z];
            }
        }
    }

    extra_rays
}

/// Bakes lighting selectively for a targeted subset of world brushes (incremental differential bake)
pub fn bake_lighting_selective(
    brushes: &mut [Brush],
    lights: &[SceneLight],
    target_brush_indices: &[usize],
    config: &LightingConfig,
) -> LightingStats {
    let start_time = Instant::now();

    // 1. Occluders are gathered from ALL post-CSG opaque world geometry
    let occluders = gather_post_csg_occluders(brushes);

    let mut total_polys = 0;
    let mut total_luxels = 0;
    let mut rays_cast = 0;

    for (b_idx, brush) in brushes.iter_mut().enumerate() {
        if !target_brush_indices.contains(&b_idx) {
            continue;
        }

        total_polys += brush.polys.len();
        let rad_pitch = brush.rotation.x.to_radians();
        let rad_yaw = brush.rotation.y.to_radians();
        let rad_roll = brush.rotation.z.to_radians();
        let rot_matrix = glam::Mat4::from_euler(glam::EulerRot::YXZ, rad_yaw, rad_pitch, rad_roll);
        let scale_matrix = glam::Mat4::from_scale(brush.scale);
        let trans_matrix = glam::Mat4::from_translation(brush.location);
        let model_matrix = trans_matrix * rot_matrix * scale_matrix;
        let is_subtract = brush.csg_oper == crate::CsgOperation::Subtract;
        let occluders_ref = &occluders;
        let config_ref = config;

        brush.polys.par_iter_mut().for_each(|poly| {
            let n = poly.vertices.len();
            if n < 3 {
                return;
            }

            let normal = model_matrix.transform_vector3(poly.plane.normal).normalize_or_zero();
            let poly_special_lit = (poly.flags & crate::poly::PF_SPECIAL_LIT) != 0;
            let is_two_sided_or_sub = (poly.flags & crate::poly::PF_TWO_SIDED) != 0 || is_subtract;

            if (poly.flags & PF_UNLIT) != 0 {
                poly.vertex_colors = vec![[1.0, 1.0, 1.0]; n];
                poly.lightmap = None;
                return;
            }

            // Bake vertex lighting
            let mut v_colors = Vec::with_capacity(n);
            for v in &poly.vertices {
                let world_v = model_matrix.transform_point3(*v);
                let mut accumulated = config_ref.ambient_color;

                for light in lights {
                    if light.is_special_lit != poly_special_lit {
                        continue;
                    }

                    let world_radius = (light.radius as f32) * 25.0;
                    let to_light = light.location - world_v;
                    let dist = to_light.length();

                    if dist >= world_radius || dist <= 1e-4 {
                        continue;
                    }

                    let light_dir = to_light / dist;
                    let eff_normal = if is_two_sided_or_sub && normal.dot(light_dir) < 0.0 {
                        -normal
                    } else {
                        normal
                    };

                    if config_ref.enable_shadows && is_occluded(world_v, light.location, eff_normal, occluders_ref) {
                        continue;
                    }

                    let n_dot_l = eff_normal.dot(light_dir).max(0.0);

                    if n_dot_l > 0.0 {
                        let norm_dist = dist / world_radius;
                        let atten = (1.0 - norm_dist).powi(2);
                        let light_col = ut99_light_to_rgb(light.hue, light.saturation, light.brightness);
                        accumulated += light_col * (atten * n_dot_l * 2.0);
                    }
                }

                if !config_ref.enable_overbright {
                    accumulated = accumulated.min(Vec3::ONE);
                }
                v_colors.push([accumulated.x, accumulated.y, accumulated.z]);
            }
            poly.vertex_colors = v_colors;

            // Bake lightmap grid
            let world_base = model_matrix.transform_point3(poly.base);
            let world_tu = model_matrix.transform_vector3(poly.texture_u);
            let u_dir = if world_tu.length_squared() > 1e-6 {
                world_tu.normalize()
            } else {
                Vec3::X
            };
            let world_tv = model_matrix.transform_vector3(poly.texture_v);
            let v_dir = if world_tv.length_squared() > 1e-6 {
                world_tv.normalize()
            } else {
                Vec3::Y
            };

            let mut min_u = f32::MAX;
            let mut max_u = f32::MIN;
            let mut min_v = f32::MAX;
            let mut max_v = f32::MIN;

            for v in &poly.vertices {
                let d = model_matrix.transform_point3(*v) - world_base;
                let pu = d.dot(u_dir);
                let pv = d.dot(v_dir);
                min_u = min_u.min(pu);
                max_u = max_u.max(pu);
                min_v = min_v.min(pv);
                max_v = max_v.max(pv);
            }

            let res = match config_ref.quality {
                LightingBakeQuality::DraftFast => 64.0,
                LightingBakeQuality::StandardTournament => config_ref.lightmap_resolution.max(16.0),
                LightingBakeQuality::ProductionRadiosity => 16.0,
            };
            let w_lux = (((max_u - min_u) / res).ceil() as usize).clamp(2, 32);
            let h_lux = (((max_v - min_v) / res).ceil() as usize).clamp(2, 32);

            let mut luxels = Vec::with_capacity(w_lux * h_lux);
            for y in 0..h_lux {
                let v_pos = min_v + (y as f32 + 0.5) * res;
                for x in 0..w_lux {
                    let u_pos = min_u + (x as f32 + 0.5) * res;
                    let world_luxel = world_base + u_dir * u_pos + v_dir * v_pos;
                    let mut col = config_ref.ambient_color;

                    for light in lights {
                        if light.is_special_lit != poly_special_lit {
                            continue;
                        }

                        let world_radius = (light.radius as f32) * 25.0;
                        let to_light = light.location - world_luxel;
                        let dist = to_light.length();

                        if dist >= world_radius || dist <= 1e-4 {
                            continue;
                        }

                        let light_dir = to_light / dist;
                        let eff_normal = if is_two_sided_or_sub && normal.dot(light_dir) < 0.0 {
                            -normal
                        } else {
                            normal
                        };

                        if config_ref.enable_shadows && is_occluded(world_luxel, light.location, eff_normal, occluders_ref) {
                            continue;
                        }

                        let n_dot_l = eff_normal.dot(light_dir).max(0.0);

                        if n_dot_l > 0.0 {
                            let norm_dist = dist / world_radius;
                            let atten = (1.0 - norm_dist).powi(2);
                            let light_col = ut99_light_to_rgb(light.hue, light.saturation, light.brightness);
                            col += light_col * (atten * n_dot_l * 2.0);
                        }
                    }

                    if !config_ref.enable_overbright {
                        col = col.min(Vec3::ONE);
                    }
                    luxels.push([col.x, col.y, col.z]);
                }
            }

            poly.lightmap = Some(LightmapData {
                width: w_lux,
                height: h_lux,
                luxels,
            });
        });

        for poly in &brush.polys {
            if let Some(ref lm) = poly.lightmap {
                total_luxels += lm.luxels.len();
                rays_cast += lm.luxels.len() * lights.len();
            }
        }
    }

    rays_cast += apply_radiosity_bounces(brushes, &occluders, config);

    let elapsed = start_time.elapsed().as_millis();
    LightingStats {
        total_polygons: total_polys,
        total_luxels,
        shadow_rays_cast: rays_cast,
        elapsed_ms: elapsed,
    }
}

#[cfg(test)]
mod radiosity_tests {
    use super::*;
    use crate::brush::build_cube;
    use crate::CsgOperation;

    #[test]
    fn test_radiosity_multi_bounce_increases_indirect_illumination() {
        let room = build_cube("Room", CsgOperation::Subtract, Vec3::new(512.0, 512.0, 512.0));
        // Place a small radius light close to the floor (Z = -200, radius = 12 => 300 uu)
        // so the floor (Z = -256) is brightly lit directly, while the ceiling (Z = +256, dist = 456 > 300)
        // receives 0 direct light and is only lit by diffuse bounced radiosity!
        let light = SceneLight {
            name: "FloorLight".into(),
            location: Vec3::new(0.0, 0.0, -200.0),
            brightness: 255,
            radius: 12,
            hue: 0,
            saturation: 255,
            ..Default::default()
        };

        let mut brushes_direct = vec![room.clone()];
        let cfg_direct = LightingConfig {
            radiosity_bounces: 0,
            enable_shadows: false,
            ..Default::default()
        };
        bake_lighting_for_brushes(&mut brushes_direct, &[light.clone()], &cfg_direct);

        let mut brushes_rad = vec![room];
        let cfg_rad = LightingConfig {
            radiosity_bounces: 2,
            radiosity_albedo: 0.5,
            enable_shadows: false,
            ..Default::default()
        };
        bake_lighting_for_brushes(&mut brushes_rad, &[light], &cfg_rad);

        // Top face (index 1, Z = +256) receives 0 direct light, but receives bounced light in brushes_rad
        let direct_ceiling_lum: f32 = brushes_direct[0].polys[1].vertex_colors[0].iter().sum();
        let rad_ceiling_lum: f32 = brushes_rad[0].polys[1].vertex_colors[0].iter().sum();
        assert!(
            rad_ceiling_lum > direct_ceiling_lum + 0.001,
            "Multi-bounce radiosity must increase indirect ceiling illumination ({} vs {})",
            rad_ceiling_lum,
            direct_ceiling_lum
        );
    }

    #[test]
    fn test_light_passes_through_subtracted_doorway() {
        // Outer Subtract room (1024 x 512 x 512) divided at X=0 by an Add wall (32 x 512 x 512),
        // then carved at the center by a Subtract doorway (128 x 256 x 256).
        let room = build_cube("OuterRoom", CsgOperation::Subtract, Vec3::new(1024.0, 512.0, 512.0));
        let dividing_wall = build_cube("DividingWall", CsgOperation::Add, Vec3::new(32.0, 512.0, 512.0));
        let doorway = build_cube("Doorway", CsgOperation::Subtract, Vec3::new(128.0, 256.0, 256.0));

        let occluders = gather_post_csg_occluders(&[room, dividing_wall, doorway]);
        let light_pos = Vec3::new(-256.0, 0.0, 0.0);
        let target_pos = Vec3::new(256.0, 0.0, 0.0);
        let target_normal = Vec3::new(-1.0, 0.0, 0.0);

        assert!(
            !is_occluded(target_pos, light_pos, target_normal, &occluders),
            "Light ray passing through the center of a Subtract doorway must NOT be blocked by the pre-CSG dividing wall"
        );
    }

    #[test]
    fn test_spotlight_cone_and_non_incidence_effects() {
        let mut spot = SceneLight {
            name: "Spot".into(),
            location: Vec3::new(0.0, 0.0, 128.0),
            brightness: 200,
            radius: 32,
            light_effect: LightEffect::Spotlight as u8,
            cone: 48, // Narrow downward cone
            ..Default::default()
        };
        let world_r = (spot.radius as f32) * 25.0;
        let floor_n = Vec3::Z;
        // Point directly beneath spotlight (0,0,0) -> inside cone
        let f_center = compute_light_factor(&spot, Vec3::ZERO, floor_n, world_r);
        // Point far off to the side (300,0,0) -> outside narrow cone
        let f_side = compute_light_factor(&spot, Vec3::new(300.0, 0.0, 0.0), floor_n, world_r);
        assert!(f_center > 0.5);
        assert_eq!(f_side, 0.0);

        // NonIncidence vs standard at grazing angle
        spot.light_effect = LightEffect::None as u8;
        let f_grazing_std = compute_light_factor(&spot, Vec3::new(250.0, 0.0, 0.0), floor_n, world_r);
        spot.light_effect = LightEffect::NonIncidence as u8;
        let f_grazing_non_inc = compute_light_factor(&spot, Vec3::new(250.0, 0.0, 0.0), floor_n, world_r);
        assert!(f_grazing_non_inc > f_grazing_std);
    }
}


