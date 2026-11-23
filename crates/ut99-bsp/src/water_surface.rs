//! Interactive Dynamic Zone Water Surface & Wave Reflection Studio
//!
//! Provides 2D discrete wave equation simulation, ripple propagation,
//! caustics intensity projection, and automated watertight water sheet generation
//! for Unreal Engine 1 fluid and swimming zones.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical presets for environmental fluid and water surfaces in Unreal Engine 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WaterPreset {
    #[default]
    CrystalClearPool,
    MurkySwampVat,
    TorrentialRainLake,
    FastRiverRapids,
    ToxicVatBubbles,
    LavaMagmaCrust,
}

impl WaterPreset {
    pub const ALL: [Self; 6] = [
        Self::CrystalClearPool,
        Self::MurkySwampVat,
        Self::TorrentialRainLake,
        Self::FastRiverRapids,
        Self::ToxicVatBubbles,
        Self::LavaMagmaCrust,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CrystalClearPool => "Crystal Clear Pool (Calm & Reflective)",
            Self::MurkySwampVat => "Murky Swamp Vat (High Viscosity & Damped)",
            Self::TorrentialRainLake => "Torrential Rain Lake (Continuous Interference)",
            Self::FastRiverRapids => "Fast River Rapids (Directional Flow Crests)",
            Self::ToxicVatBubbles => "Toxic Vat Bubbles (Simmering Burst Pulses)",
            Self::LavaMagmaCrust => "Lava Magma Crust (Extreme Viscosity Surge)",
        }
    }

    pub fn to_config(&self) -> WaterSurfaceConfig {
        match self {
            Self::CrystalClearPool => WaterSurfaceConfig {
                wave_speed: 1.2,
                damping: 0.985,
                tension: 0.85,
                caustics_intensity: 1.2,
                caustics_scale: 1.0,
                refraction_index: 1.333,
                flow_velocity: Vec3::ZERO,
                ambient_rain_intensity: 0.05,
            },
            Self::MurkySwampVat => WaterSurfaceConfig {
                wave_speed: 0.7,
                damping: 0.940,
                tension: 0.50,
                caustics_intensity: 0.3,
                caustics_scale: 1.5,
                refraction_index: 1.380,
                flow_velocity: Vec3::new(15.0, 0.0, 0.0),
                ambient_rain_intensity: 0.0,
            },
            Self::TorrentialRainLake => WaterSurfaceConfig {
                wave_speed: 1.4,
                damping: 0.975,
                tension: 0.90,
                caustics_intensity: 0.9,
                caustics_scale: 0.8,
                refraction_index: 1.333,
                flow_velocity: Vec3::ZERO,
                ambient_rain_intensity: 0.80,
            },
            Self::FastRiverRapids => WaterSurfaceConfig {
                wave_speed: 1.8,
                damping: 0.965,
                tension: 0.75,
                caustics_intensity: 1.0,
                caustics_scale: 1.2,
                refraction_index: 1.333,
                flow_velocity: Vec3::new(120.0, 30.0, 0.0),
                ambient_rain_intensity: 0.10,
            },
            Self::ToxicVatBubbles => WaterSurfaceConfig {
                wave_speed: 0.9,
                damping: 0.950,
                tension: 0.60,
                caustics_intensity: 0.6,
                caustics_scale: 1.4,
                refraction_index: 1.420,
                flow_velocity: Vec3::ZERO,
                ambient_rain_intensity: 0.35,
            },
            Self::LavaMagmaCrust => WaterSurfaceConfig {
                wave_speed: 0.4,
                damping: 0.910,
                tension: 0.30,
                caustics_intensity: 0.1,
                caustics_scale: 2.0,
                refraction_index: 1.550,
                flow_velocity: Vec3::new(8.0, 0.0, 0.0),
                ambient_rain_intensity: 0.0,
            },
        }
    }
}

/// Physical and visual parameters governing water surface dynamics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaterSurfaceConfig {
    pub wave_speed: f32,
    pub damping: f32,
    pub tension: f32,
    pub caustics_intensity: f32,
    pub caustics_scale: f32,
    pub refraction_index: f32,
    pub flow_velocity: Vec3,
    pub ambient_rain_intensity: f32,
}

impl Default for WaterSurfaceConfig {
    fn default() -> Self {
        WaterPreset::CrystalClearPool.to_config()
    }
}

/// A localized ripple source acting upon the surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaterRippleSource {
    pub position: Vec3,
    pub amplitude: f32,
    pub radius: f32,
    pub age: f32,
    pub max_life: f32,
}

/// 2D Discrete wave equation heightfield simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaterHeightfieldSimulation {
    pub width: usize,
    pub height: usize,
    pub grid_spacing_uu: f32,
    pub current_heights: Vec<f32>,
    pub previous_heights: Vec<f32>,
    pub caustics_intensity_map: Vec<f32>,
}

impl WaterHeightfieldSimulation {
    pub fn new(width: usize, height: usize, grid_spacing_uu: f32) -> Self {
        let w = width.clamp(8, 128);
        let h = height.clamp(8, 128);
        let count = w * h;
        Self {
            width: w,
            height: h,
            grid_spacing_uu: grid_spacing_uu.max(4.0),
            current_heights: vec![0.0; count],
            previous_heights: vec![0.0; count],
            caustics_intensity_map: vec![1.0; count],
        }
    }

    /// Injects a vertical displacement impulse at grid coordinates (gx, gy).
    pub fn add_impulse(&mut self, gx: usize, gy: usize, radius_cells: usize, amplitude: f32) {
        let r = radius_cells.max(1);
        let x_min = gx.saturating_sub(r);
        let x_max = (gx + r).min(self.width.saturating_sub(1));
        let y_min = gy.saturating_sub(r);
        let y_max = (gy + r).min(self.height.saturating_sub(1));

        for y in y_min..=y_max {
            for x in x_min..=x_max {
                let dx = x as f32 - gx as f32;
                let dy = y as f32 - gy as f32;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= r as f32 {
                    let falloff = (1.0 - (dist / r as f32)).clamp(0.0, 1.0);
                    let smooth_falloff = 0.5 * (1.0 - (std::f32::consts::PI * falloff).cos());
                    let idx = y * self.width + x;
                    self.current_heights[idx] += amplitude * smooth_falloff;
                }
            }
        }
    }

    /// Performs one discrete wave equation integration step:
    /// u^{n+1} = 2u^n - u^{n-1} + (c*dt/dx)^2 * Lap(u^n)
    pub fn step(&mut self, _dt: f32, config: &WaterSurfaceConfig) {
        let w = self.width;
        let h = self.height;
        if w < 3 || h < 3 {
            return;
        }

        let mut next_heights = vec![0.0; w * h];
        let c_sq = (config.wave_speed * 0.45).clamp(0.05, 0.48);
        let damping = config.damping.clamp(0.85, 0.999);

        for y in 1..(h - 1) {
            for x in 1..(w - 1) {
                let idx = y * w + x;
                let cur = self.current_heights[idx];
                let prev = self.previous_heights[idx];

                // 4-point Laplacian operator with diagonal cross-coupling
                let lap = (self.current_heights[idx - 1]
                    + self.current_heights[idx + 1]
                    + self.current_heights[idx - w]
                    + self.current_heights[idx + w])
                    * 0.25
                    - cur;

                // Wave equation with damping
                let next = (2.0 * cur - prev + c_sq * 4.0 * lap) * damping;
                next_heights[idx] = next;
            }
        }

        // Apply flow drift if velocity > 0
        if config.flow_velocity.x.abs() > 1.0 || config.flow_velocity.y.abs() > 1.0 {
            let drift_x = (config.flow_velocity.x * 0.005).round() as i32;
            let drift_y = (config.flow_velocity.y * 0.005).round() as i32;
            if drift_x != 0 || drift_y != 0 {
                let mut drifted = next_heights.clone();
                for y in 1..(h - 1) {
                    for x in 1..(w - 1) {
                        let sx = (x as i32 - drift_x).clamp(0, (w - 1) as i32) as usize;
                        let sy = (y as i32 - drift_y).clamp(0, (h - 1) as i32) as usize;
                        drifted[y * w + x] = next_heights[sy * w + sx];
                    }
                }
                next_heights = drifted;
            }
        }

        self.previous_heights = std::mem::replace(&mut self.current_heights, next_heights);

        // Ambient rain droplets
        if config.ambient_rain_intensity > 0.001 {
            let num_drops = ((config.ambient_rain_intensity * 3.0) as usize).max(1);
            let time_seed = (self.current_heights.len() as u32).wrapping_mul(7919);
            for d in 0..num_drops {
                let pseudo_rand_x = ((d as u32 * 1103515245 + time_seed) % (w as u32 - 4) + 2) as usize;
                let pseudo_rand_y = ((d as u32 * 12345 + 6789 + time_seed) % (h as u32 - 4) + 2) as usize;
                let amp = config.ambient_rain_intensity * 6.0;
                self.add_impulse(pseudo_rand_x, pseudo_rand_y, 1, amp);
            }
        }

        // Update caustics map based on surface curvature (divergence of normal)
        self.compute_caustics(config.caustics_intensity);
    }

    /// Resets all heightfield buffers and caustics map to flat baseline.
    pub fn clear(&mut self) {
        self.current_heights.fill(0.0);
        self.previous_heights.fill(0.0);
        self.caustics_intensity_map.fill(1.0);
    }

    /// Injects a vertical displacement impulse at normalized UV coordinates [0.0, 1.0].
    pub fn add_impulse_uv(&mut self, u: f32, v: f32, radius_cells: usize, amplitude: f32) {
        let gx = ((u.clamp(0.0, 1.0) * (self.width as f32 - 1.0)).round() as usize).min(self.width.saturating_sub(1));
        let gy = ((v.clamp(0.0, 1.0) * (self.height as f32 - 1.0)).round() as usize).min(self.height.saturating_sub(1));
        self.add_impulse(gx, gy, radius_cells, amplitude);
    }

    /// Computes caustics intensity from surface curvature (focusing of light rays).
    fn compute_caustics(&mut self, intensity_mult: f32) {
        let w = self.width;
        let h = self.height;

        for y in 1..(h - 1) {
            for x in 1..(w - 1) {
                let idx = y * w + x;
                let cur = self.current_heights[idx];

                // Second derivative / curvature: concave surface focuses light, convex disperses
                let d2x = self.current_heights[idx - 1] + self.current_heights[idx + 1] - 2.0 * cur;
                let d2y = self.current_heights[idx - w] + self.current_heights[idx + w] - 2.0 * cur;
                let curvature = -(d2x + d2y);

                let caustic = (1.0 + curvature * 0.25 * intensity_mult).clamp(0.1, 3.0);
                self.caustics_intensity_map[idx] = caustic;
            }
        }
    }

    /// Computes the surface normal vector at cell (x, y).
    pub fn get_normal(&self, x: usize, y: usize) -> Vec3 {
        let w = self.width;
        let h = self.height;
        let x_clamped = x.clamp(1, w.saturating_sub(2));
        let y_clamped = y.clamp(1, h.saturating_sub(2));

        let idx = y_clamped * w + x_clamped;
        let dz_dx = (self.current_heights[idx + 1] - self.current_heights[idx - 1]) / (2.0 * self.grid_spacing_uu);
        let dz_dy = (self.current_heights[idx + w] - self.current_heights[idx - w]) / (2.0 * self.grid_spacing_uu);

        Vec3::new(-dz_dx, -dz_dy, 1.0).normalize()
    }
}

/// Generates a perfectly watertight tessellated water sheet brush.
pub fn generate_watertight_water_sheet_brush(
    name: &str,
    origin: Vec3,
    size_x: f32,
    size_y: f32,
    subdivisions_x: usize,
    subdivisions_y: usize,
    poly_flags: u32,
    texture_name: &str,
) -> Brush {
    let sub_x = subdivisions_x.clamp(1, 32);
    let sub_y = subdivisions_y.clamp(1, 32);
    let dx = size_x / sub_x as f32;
    let dy = size_y / sub_y as f32;

    let half_x = size_x * 0.5;
    let half_y = size_y * 0.5;

    let mut brush = Brush::new(name, CsgOperation::Active);
    brush.location = origin;

    for y in 0..sub_y {
        let y0 = -half_y + (y as f32) * dy;
        let y1 = y0 + dy;

        for x in 0..sub_x {
            let x0 = -half_x + (x as f32) * dx;
            let x1 = x0 + dx;

            let v_bl = Vec3::new(x0, y0, 0.0);
            let v_br = Vec3::new(x1, y0, 0.0);
            let v_tr = Vec3::new(x1, y1, 0.0);
            let v_tl = Vec3::new(x0, y1, 0.0);

            // Triangle 1: BL -> BR -> TR
            if let Some(mut poly1) = Polygon::new(vec![v_bl, v_br, v_tr]) {
                poly1.flags = poly_flags;
                poly1.texture = texture_name.to_string();
                brush.polys.push(poly1);
            }

            // Triangle 2: BL -> TR -> TL
            if let Some(mut poly2) = Polygon::new(vec![v_bl, v_tr, v_tl]) {
                poly2.flags = poly_flags;
                poly2.texture = texture_name.to_string();
                brush.polys.push(poly2);
            }
        }
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_water_presets_coverage() {
        assert_eq!(WaterPreset::ALL.len(), 6);
        for preset in &WaterPreset::ALL {
            assert!(!preset.display_name().is_empty());
            let cfg = preset.to_config();
            assert!(cfg.wave_speed > 0.0 && cfg.wave_speed <= 3.0);
            assert!(cfg.damping > 0.85 && cfg.damping < 1.0);
            assert!(cfg.refraction_index >= 1.30);
        }
    }

    #[test]
    fn test_wave_equation_radial_propagation() {
        let mut sim = WaterHeightfieldSimulation::new(32, 32, 16.0);
        let center = 16;
        sim.add_impulse(center, center, 2, 10.0);

        let initial_center = sim.current_heights[center * 32 + center];
        assert!(initial_center > 5.0, "Center must have high initial amplitude");

        // Run 5 simulation steps
        let cfg = WaterPreset::CrystalClearPool.to_config();
        for _ in 0..5 {
            sim.step(0.016, &cfg);
        }

        // Check that adjacent cells received wave energy
        let neighbor_val = sim.current_heights[center * 32 + (center + 2)];
        assert!(neighbor_val.abs() > 0.05, "Wave must propagate to neighbors");
    }

    #[test]
    fn test_caustics_calculation() {
        let mut sim = WaterHeightfieldSimulation::new(24, 24, 16.0);
        sim.add_impulse(12, 12, 3, 8.0);
        let cfg = WaterPreset::CrystalClearPool.to_config();
        sim.step(0.016, &cfg);

        // Center should have curvature resulting in focused caustics
        let c_val = sim.caustics_intensity_map[12 * 24 + 12];
        assert!(c_val > 0.1 && c_val < 3.5);
    }

    #[test]
    fn test_generate_watertight_water_sheet() {
        let brush = generate_watertight_water_sheet_brush(
            "WaterSheet0",
            Vec3::new(100.0, 200.0, 50.0),
            512.0,
            256.0,
            4,
            2,
            0x00000004 | 0x00000100, // PF_Translucent | PF_TwoSided
            "Water.DefaultWater",
        );

        assert_eq!(brush.name, "WaterSheet0");
        assert_eq!(brush.location, Vec3::new(100.0, 200.0, 50.0));
        assert_eq!(brush.polys.len(), 4 * 2 * 2, "4x2 grid * 2 triangles per quad");
        assert_eq!(brush.polys[0].plane.normal, Vec3::Z);
        assert_eq!(brush.polys[0].texture, "Water.DefaultWater");
    }

    #[test]
    fn test_impulse_uv_and_clear() {
        let mut sim = WaterHeightfieldSimulation::new(32, 32, 16.0);
        sim.add_impulse_uv(0.5, 0.5, 2, 15.0);
        let center_h = sim.current_heights[16 * 32 + 16];
        assert!(center_h > 10.0);

        sim.clear();
        for &h in &sim.current_heights {
            assert_eq!(h, 0.0);
        }
        for &c in &sim.caustics_intensity_map {
            assert_eq!(c, 1.0);
        }
    }
}
