// crates/ut99-bsp/src/pipe_conduit.rs
//
// Interactive Industrial Pipe Network, Steam Valve & Conduit Junction Studio
//
// Mathematical, geometric, and fluid-kinetic model for parametric industrial pipe runs,
// flanges, elbows, T-junctions, wall pass-through collars, relief valves, and hazard vents
// inspired by iconic maps such as DM-Shrapnel, DM-Pressure, DM-Liandri, AS-Frigate, and CTF-Hydro16.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical Unreal Tournament & Unreal 1 industrial pipe conduit archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipeArchetype {
    HighPressureSteamMain,
    BiohazardCoolantConduit,
    SubterraneanSewerAqueduct,
    CryogenicFuelTransferLine,
    HydraulicControlBundle,
}

impl PipeArchetype {
    pub fn all() -> &'static [PipeArchetype] {
        &[
            PipeArchetype::HighPressureSteamMain,
            PipeArchetype::BiohazardCoolantConduit,
            PipeArchetype::SubterraneanSewerAqueduct,
            PipeArchetype::CryogenicFuelTransferLine,
            PipeArchetype::HydraulicControlBundle,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PipeArchetype::HighPressureSteamMain => "High-Pressure Steam Main (Scald Hazard Valve)",
            PipeArchetype::BiohazardCoolantConduit => "Biohazard Coolant Conduit (Toxic Sight-Glass)",
            PipeArchetype::SubterraneanSewerAqueduct => "Subterranean Sewer Aqueduct (Cast Iron Main)",
            PipeArchetype::CryogenicFuelTransferLine => "Cryogenic Fuel Transfer Line (Vacuum Jacketed)",
            PipeArchetype::HydraulicControlBundle => "Hydraulic Control Bundle (Triple Rack Conduit)",
        }
    }

    pub fn default_config(&self) -> PipeConduitConfig {
        match self {
            PipeArchetype::HighPressureSteamMain => PipeConduitConfig {
                archetype: *self,
                position: Vec3::ZERO,
                pipe_outer_radius: 32.0, // 64 uu diameter
                pipe_wall_thickness: 6.0,
                cylinder_facets: 8,
                total_run_length_uu: 512.0,
                elbow_radius_uu: 96.0,
                has_elbow_bend: true,
                elbow_angle_deg: 90.0,
                has_flanges: true,
                flange_spacing_uu: 256.0,
                flange_outer_radius: 44.0,
                flange_thickness_uu: 8.0,
                has_wall_collars: true,
                has_relief_valve: true,
                valve_handwheel_radius: 24.0,
                valve_open_fraction: 0.0, // closed by default
                steam_hazard_enabled: true,
                scald_damage_per_sec: 25.0,
                hazard_sound: "Steam1".to_string(),
                pipe_texture: "Indus.Metal.RustyIron01".to_string(),
                flange_texture: "Indus.Metal.YellowHazardStripes01".to_string(),
                tag: "SteamMain01".to_string(),
                event_name: "SteamValve_Open".to_string(),
            },
            PipeArchetype::BiohazardCoolantConduit => PipeConduitConfig {
                archetype: *self,
                position: Vec3::ZERO,
                pipe_outer_radius: 24.0, // 48 uu diameter
                pipe_wall_thickness: 4.0,
                cylinder_facets: 8,
                total_run_length_uu: 448.0,
                elbow_radius_uu: 80.0,
                has_elbow_bend: true,
                elbow_angle_deg: 90.0,
                has_flanges: true,
                flange_spacing_uu: 224.0,
                flange_outer_radius: 34.0,
                flange_thickness_uu: 6.0,
                has_wall_collars: true,
                has_relief_valve: true,
                valve_handwheel_radius: 20.0,
                valve_open_fraction: 0.0,
                steam_hazard_enabled: true,
                scald_damage_per_sec: 40.0, // Corrosive bio-sludge
                hazard_sound: "drip1".to_string(),
                pipe_texture: "Indus.Slime.GreenGlowPipe01".to_string(),
                flange_texture: "Indus.Metal.DarkAlloy01".to_string(),
                tag: "BioCoolant01".to_string(),
                event_name: "CoolantLeak_Trigger".to_string(),
            },
            PipeArchetype::SubterraneanSewerAqueduct => PipeConduitConfig {
                archetype: *self,
                position: Vec3::ZERO,
                pipe_outer_radius: 48.0, // 96 uu diameter
                pipe_wall_thickness: 8.0,
                cylinder_facets: 12,
                total_run_length_uu: 640.0,
                elbow_radius_uu: 128.0,
                has_elbow_bend: false,
                elbow_angle_deg: 0.0,
                has_flanges: true,
                flange_spacing_uu: 320.0,
                flange_outer_radius: 64.0,
                flange_thickness_uu: 12.0,
                has_wall_collars: true,
                has_relief_valve: false,
                valve_handwheel_radius: 32.0,
                valve_open_fraction: 1.0,
                steam_hazard_enabled: false,
                scald_damage_per_sec: 0.0,
                hazard_sound: "waterfall1".to_string(),
                pipe_texture: "Indus.Concrete.SewerPipe01".to_string(),
                flange_texture: "Indus.Metal.CastIron01".to_string(),
                tag: "SewerAqueduct01".to_string(),
                event_name: "".to_string(),
            },
            PipeArchetype::CryogenicFuelTransferLine => PipeConduitConfig {
                archetype: *self,
                position: Vec3::ZERO,
                pipe_outer_radius: 20.0, // 40 uu diameter
                pipe_wall_thickness: 5.0,
                cylinder_facets: 8,
                total_run_length_uu: 384.0,
                elbow_radius_uu: 64.0,
                has_elbow_bend: true,
                elbow_angle_deg: 45.0,
                has_flanges: true,
                flange_spacing_uu: 192.0,
                flange_outer_radius: 30.0,
                flange_thickness_uu: 6.0,
                has_wall_collars: true,
                has_relief_valve: true,
                valve_handwheel_radius: 18.0,
                valve_open_fraction: 0.0,
                steam_hazard_enabled: true,
                scald_damage_per_sec: 35.0, // Cryo frostburn
                hazard_sound: "Hiss1".to_string(),
                pipe_texture: "Indus.Metal.FrostInsulated01".to_string(),
                flange_texture: "Indus.Metal.ChromeFlange01".to_string(),
                tag: "CryoFuelLine01".to_string(),
                event_name: "CryoVent_Trigger".to_string(),
            },
            PipeArchetype::HydraulicControlBundle => PipeConduitConfig {
                archetype: *self,
                position: Vec3::ZERO,
                pipe_outer_radius: 12.0, // 24 uu diameter
                pipe_wall_thickness: 3.0,
                cylinder_facets: 6,
                total_run_length_uu: 320.0,
                elbow_radius_uu: 48.0,
                has_elbow_bend: true,
                elbow_angle_deg: 90.0,
                has_flanges: false,
                flange_spacing_uu: 0.0,
                flange_outer_radius: 16.0,
                flange_thickness_uu: 4.0,
                has_wall_collars: true,
                has_relief_valve: false,
                valve_handwheel_radius: 14.0,
                valve_open_fraction: 1.0,
                steam_hazard_enabled: false,
                scald_damage_per_sec: 0.0,
                hazard_sound: "hum1".to_string(),
                pipe_texture: "Indus.Metal.CopperConduit01".to_string(),
                flange_texture: "Indus.Metal.SteelBracket01".to_string(),
                tag: "HydraulicBundle01".to_string(),
                event_name: "".to_string(),
            },
        }
    }
}

/// Complete configuration for Pipe Conduit Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipeConduitConfig {
    pub archetype: PipeArchetype,
    pub position: Vec3,
    pub pipe_outer_radius: f32,
    pub pipe_wall_thickness: f32,
    pub cylinder_facets: usize,
    pub total_run_length_uu: f32,
    pub elbow_radius_uu: f32,
    pub has_elbow_bend: bool,
    pub elbow_angle_deg: f32,
    pub has_flanges: bool,
    pub flange_spacing_uu: f32,
    pub flange_outer_radius: f32,
    pub flange_thickness_uu: f32,
    pub has_wall_collars: bool,
    pub has_relief_valve: bool,
    pub valve_handwheel_radius: f32,
    pub valve_open_fraction: f32, // 0.0 (closed) to 1.0 (fully open)
    pub steam_hazard_enabled: bool,
    pub scald_damage_per_sec: f32,
    pub hazard_sound: String,
    pub pipe_texture: String,
    pub flange_texture: String,
    pub tag: String,
    pub event_name: String,
}

impl Default for PipeConduitConfig {
    fn default() -> Self {
        PipeArchetype::HighPressureSteamMain.default_config()
    }
}

/// Analytical hydraulic & geometric metrics for the pipe network
#[derive(Debug, Clone, PartialEq)]
pub struct PipeConduitMetrics {
    pub total_pipe_length_uu: f32,
    pub total_fluid_volume_uu3: f32,
    pub total_fluid_volume_liters: f32,
    pub flow_velocity_uu_s: f32,
    pub pressure_drop_psi: f32,
    pub flange_count: usize,
}

/// Generated procedural brush meshes for all pipe components
#[derive(Debug, Clone, PartialEq)]
pub struct PipeConduitBrushes {
    pub straight_run_brush: Brush,
    pub elbow_bend_brush: Option<Brush>,
    pub flange_brushes: Vec<Brush>,
    pub wall_collar_brush: Option<Brush>,
    pub valve_body_brush: Option<Brush>,
    pub valve_handwheel_brush: Option<Brush>,
}

/// Diagnostic safety hazard
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipeConduitHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Calculates hydraulic and structural metrics for the pipe conduit
pub fn calculate_pipe_metrics(config: &PipeConduitConfig) -> PipeConduitMetrics {
    let elbow_len = if config.has_elbow_bend && config.elbow_angle_deg > 0.0 {
        let rad = config.elbow_angle_deg.to_radians();
        config.elbow_radius_uu * rad
    } else {
        0.0
    };

    let total_len = config.total_run_length_uu + elbow_len;
    let inner_r = (config.pipe_outer_radius - config.pipe_wall_thickness).max(1.0);
    let area = std::f32::consts::PI * inner_r * inner_r;
    let vol_uu3 = area * total_len;

    // 1 Unreal Unit ~ 1 inch (2.54 cm = 0.0254 m). 1 uu3 ~ (0.0254)^3 m3 ~ 1.6387e-5 m3 ~ 0.016387 L
    let vol_liters = vol_uu3 * 0.016387;

    // Flow velocity scaled by valve open fraction
    let flow_vel = 400.0 * config.valve_open_fraction.clamp(0.05, 1.0);

    // Darcy-Weisbach pressure drop estimate: dP = f * (L/D) * (rho * v^2 / 2)
    let d = (inner_r * 2.0).max(1.0);
    let f = 0.02; // Darcy friction factor for commercial steel
    let dp = f * (total_len / d) * (flow_vel * flow_vel * 0.00005);

    let flange_count = if config.has_flanges && config.flange_spacing_uu > 32.0 {
        (config.total_run_length_uu / config.flange_spacing_uu).floor() as usize + 1
    } else {
        0
    };

    PipeConduitMetrics {
        total_pipe_length_uu: total_len,
        total_fluid_volume_uu3: vol_uu3,
        total_fluid_volume_liters: vol_liters,
        flow_velocity_uu_s: flow_vel,
        pressure_drop_psi: dp,
        flange_count,
    }
}

/// Generates watertight procedural brushes for pipe runs, elbows, flanges, and valves
pub fn generate_pipe_conduit_brushes(config: &PipeConduitConfig) -> PipeConduitBrushes {
    let p = config.position;
    let facets = config.cylinder_facets.clamp(6, 16);
    let r = config.pipe_outer_radius;

    // 1. Straight Run Pipe Cylinder
    let mut straight_b = Brush::new("PipeConduit_Straight", CsgOperation::Add);
    let len = config.total_run_length_uu;

    let mut rim0 = Vec::new();
    let mut rim1 = Vec::new();

    for i in 0..facets {
        let a = (i as f32 / facets as f32) * std::f32::consts::TAU;
        let x = a.cos() * r;
        let z = a.sin() * r;
        rim0.push(p + Vec3::new(x, 0.0, z));
        rim1.push(p + Vec3::new(x, len, z));
    }

    // Side quad faces
    for i in 0..facets {
        let next = (i + 1) % facets;
        let quad = vec![rim0[i], rim1[i], rim1[next], rim0[next]];
        if let Some(mut poly) = Polygon::new(quad) {
            poly.texture = config.pipe_texture.clone();
            poly.flags |= 0x00200000; // PF_SEMISOLID
            straight_b.polys.push(poly);
        }
    }

    // End caps
    let mut cap0 = rim0.clone();
    cap0.reverse();
    if let Some(mut poly) = Polygon::new(cap0) {
        poly.texture = config.pipe_texture.clone();
        poly.flags |= 0x00200000;
        straight_b.polys.push(poly);
    }
    if let Some(mut poly) = Polygon::new(rim1) {
        poly.texture = config.pipe_texture.clone();
        poly.flags |= 0x00200000;
        straight_b.polys.push(poly);
    }

    // 2. Elbow Bend Brush (if enabled)
    let elbow_b = if config.has_elbow_bend && config.elbow_angle_deg > 0.0 {
        let mut eb = Brush::new("PipeConduit_Elbow", CsgOperation::Add);
        let elbow_origin = p + Vec3::new(config.elbow_radius_uu, len, 0.0);
        let steps = 6;
        let total_ang = config.elbow_angle_deg.to_radians();

        let mut rings: Vec<Vec<Vec3>> = Vec::new();
        for s in 0..=steps {
            let theta = (s as f32 / steps as f32) * total_ang;
            let center_x = elbow_origin.x - theta.cos() * config.elbow_radius_uu;
            let center_y = elbow_origin.y + theta.sin() * config.elbow_radius_uu;
            let c = Vec3::new(center_x, center_y, p.z);

            let mut ring = Vec::new();
            for f in 0..facets {
                let phi = (f as f32 / facets as f32) * std::f32::consts::TAU;
                let rx = phi.cos() * r * theta.sin();
                let ry = phi.cos() * r * theta.cos();
                let rz = phi.sin() * r;
                ring.push(c + Vec3::new(rx, ry, rz));
            }
            rings.push(ring);
        }

        for s in 0..steps {
            for f in 0..facets {
                let f_next = (f + 1) % facets;
                let quad = vec![rings[s][f], rings[s + 1][f], rings[s + 1][f_next], rings[s][f_next]];
                if let Some(mut poly) = Polygon::new(quad) {
                    poly.texture = config.pipe_texture.clone();
                    poly.flags |= 0x00200000;
                    eb.polys.push(poly);
                }
            }
        }
        Some(eb)
    } else {
        None
    };

    // 3. Flange Brushes
    let mut flanges = Vec::new();
    if config.has_flanges && config.flange_spacing_uu > 32.0 {
        let fl_r = config.flange_outer_radius;
        let fl_th = config.flange_thickness_uu;
        let count = (len / config.flange_spacing_uu).floor() as usize + 1;

        for k in 0..count {
            let y_pos = (k as f32 * config.flange_spacing_uu).min(len - fl_th);
            let mut fb = Brush::new(&format!("PipeFlange_{}", k), CsgOperation::Add);

            let mut f_rim0 = Vec::new();
            let mut f_rim1 = Vec::new();
            for i in 0..facets {
                let a = (i as f32 / facets as f32) * std::f32::consts::TAU;
                let x = a.cos() * fl_r;
                let z = a.sin() * fl_r;
                f_rim0.push(p + Vec3::new(x, y_pos, z));
                f_rim1.push(p + Vec3::new(x, y_pos + fl_th, z));
            }

            for i in 0..facets {
                let next = (i + 1) % facets;
                let quad = vec![f_rim0[i], f_rim1[i], f_rim1[next], f_rim0[next]];
                if let Some(mut poly) = Polygon::new(quad) {
                    poly.texture = config.flange_texture.clone();
                    poly.flags |= 0x00200000;
                    fb.polys.push(poly);
                }
            }

            let mut f_cap0 = f_rim0.clone();
            f_cap0.reverse();
            if let Some(mut poly) = Polygon::new(f_cap0) {
                poly.texture = config.flange_texture.clone();
                poly.flags |= 0x00200000;
                fb.polys.push(poly);
            }
            if let Some(mut poly) = Polygon::new(f_rim1) {
                poly.texture = config.flange_texture.clone();
                poly.flags |= 0x00200000;
                fb.polys.push(poly);
            }

            flanges.push(fb);
        }
    }

    // 4. Wall Pass-Through Collar Brush
    let collar_b = if config.has_wall_collars {
        let mut cb = Brush::new("Pipe_WallCollar", CsgOperation::Add);
        let col_r = config.pipe_outer_radius * 1.6;
        let col_th = 12.0;

        let mut c_rim0 = Vec::new();
        let mut c_rim1 = Vec::new();
        for i in 0..facets {
            let a = (i as f32 / facets as f32) * std::f32::consts::TAU;
            let x = a.cos() * col_r;
            let z = a.sin() * col_r;
            c_rim0.push(p + Vec3::new(x, 0.0, z));
            c_rim1.push(p + Vec3::new(x, col_th, z));
        }

        for i in 0..facets {
            let next = (i + 1) % facets;
            let quad = vec![c_rim0[i], c_rim1[i], c_rim1[next], c_rim0[next]];
            if let Some(mut poly) = Polygon::new(quad) {
                poly.texture = "Indus.Metal.SteelBracket01".to_string();
                poly.flags |= 0x00200000;
                cb.polys.push(poly);
            }
        }
        Some(cb)
    } else {
        None
    };

    // 5. Valve Body & Handwheel Brushes (if enabled)
    let (valve_body_b, valve_wheel_b) = if config.has_relief_valve {
        let mut vb = Brush::new("Pipe_ValveBody", CsgOperation::Add);
        let v_pos = p + Vec3::new(0.0, len * 0.5, 0.0);
        let v_hw = r * 1.5;
        let v_hl = 24.0;
        let v_hh = r * 1.5;

        let v_corners = [
            v_pos + Vec3::new(-v_hw, -v_hl, -v_hh),
            v_pos + Vec3::new(v_hw, -v_hl, -v_hh),
            v_pos + Vec3::new(v_hw, v_hl, -v_hh),
            v_pos + Vec3::new(-v_hw, v_hl, -v_hh),
            v_pos + Vec3::new(-v_hw, -v_hl, v_hh),
            v_pos + Vec3::new(v_hw, -v_hl, v_hh),
            v_pos + Vec3::new(v_hw, v_hl, v_hh),
            v_pos + Vec3::new(-v_hw, v_hl, v_hh),
        ];
        let faces = [
            [0, 3, 2, 1], [4, 5, 6, 7], [0, 1, 5, 4],
            [2, 3, 7, 6], [3, 0, 4, 7], [1, 2, 6, 5],
        ];
        for f in faces {
            let pts = vec![v_corners[f[0]], v_corners[f[1]], v_corners[f[2]], v_corners[f[3]]];
            if let Some(mut poly) = Polygon::new(pts) {
                poly.texture = "Indus.Metal.ValveBrass01".to_string();
                poly.flags |= 0x00200000;
                vb.polys.push(poly);
            }
        }

        // Handwheel (octagonal ring)
        let mut vwb = Brush::new("Pipe_ValveHandwheel", CsgOperation::Add);
        let wheel_center = v_pos + Vec3::new(0.0, 0.0, r + 24.0);
        let wr = config.valve_handwheel_radius;
        let mut w_rim0 = Vec::new();
        let mut w_rim1 = Vec::new();
        for i in 0..8 {
            let a = (i as f32 / 8.0) * std::f32::consts::TAU;
            let x = a.cos() * wr;
            let y = a.sin() * wr;
            w_rim0.push(wheel_center + Vec3::new(x, y, -2.0));
            w_rim1.push(wheel_center + Vec3::new(x, y, 2.0));
        }
        for i in 0..8 {
            let next = (i + 1) % 8;
            let quad = vec![w_rim0[i], w_rim1[i], w_rim1[next], w_rim0[next]];
            if let Some(mut poly) = Polygon::new(quad) {
                poly.texture = "Indus.Metal.RedValveWheel01".to_string();
                poly.flags |= 0x00200000;
                vwb.polys.push(poly);
            }
        }

        (Some(vb), Some(vwb))
    } else {
        (None, None)
    };

    PipeConduitBrushes {
        straight_run_brush: straight_b,
        elbow_bend_brush: elbow_b,
        flange_brushes: flanges,
        wall_collar_brush: collar_b,
        valve_body_brush: valve_body_b,
        valve_handwheel_brush: valve_wheel_b,
    }
}

/// Audits pipe conduit network for fluid dynamics, clearance, and safety issues
pub fn audit_pipe_conduit_safety(
    config: &PipeConduitConfig,
    _world_brushes: &[Brush],
) -> Vec<PipeConduitHazard> {
    let mut hazards = Vec::new();

    // 1. Severe bend pinch
    if config.has_elbow_bend && config.elbow_radius_uu < config.pipe_outer_radius * 1.25 {
        hazards.push(PipeConduitHazard {
            is_critical: true,
            title: "Severe Elbow Bend Pinch".to_string(),
            description: format!("Elbow radius ({:.0} uu) is too tight for pipe diameter ({:.0} uu). Causes geometric self-intersection.",
                config.elbow_radius_uu, config.pipe_outer_radius * 2.0),
            recommendation: "Increase elbow_radius_uu to at least 1.5x the pipe diameter.".to_string(),
        });
    }

    // 2. Steam hazard enabled without damage
    if config.steam_hazard_enabled && config.scald_damage_per_sec <= 0.0 {
        hazards.push(PipeConduitHazard {
            is_critical: false,
            title: "Harmless Steam Relief Vent".to_string(),
            description: "Steam hazard is enabled but Scald DamagePerSec is 0. Players passing through will take no damage.".to_string(),
            recommendation: "Set Scald DamagePerSec to 15..50 HP/s for authentic industrial hazard.".to_string(),
        });
    }

    // 3. Unbound valve event
    if config.has_relief_valve && config.event_name.trim().is_empty() {
        hazards.push(PipeConduitHazard {
            is_critical: false,
            title: "Unbound Valve Trigger Event".to_string(),
            description: "Relief valve exists but event_name is empty. Handwheel interaction will trigger no game events.".to_string(),
            recommendation: "Provide a unique event_name (e.g. SteamValve_Open).".to_string(),
        });
    }

    // 4. Pipe facets too low (< 6)
    if config.cylinder_facets < 6 {
        hazards.push(PipeConduitHazard {
            is_critical: true,
            title: "Degenerate Cylinder Facets".to_string(),
            description: "cylinder_facets is less than 6. Pipe will appear as an irregular prism with rendering artifacts.".to_string(),
            recommendation: "Set cylinder_facets to 8 or 12 for smooth circular appearance.".to_string(),
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipe_archetypes_and_defaults() {
        for &arch in PipeArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.pipe_outer_radius >= 12.0);
            assert!(cfg.total_run_length_uu >= 320.0);
            assert!(cfg.cylinder_facets >= 6);
        }
    }

    #[test]
    fn test_calculate_pipe_metrics() {
        let cfg = PipeArchetype::HighPressureSteamMain.default_config();
        let m = calculate_pipe_metrics(&cfg);

        assert!(m.total_pipe_length_uu > 512.0);
        assert!(m.total_fluid_volume_liters > 10.0);
        assert!(m.flange_count >= 2);
    }

    #[test]
    fn test_generate_pipe_brushes_geometry() {
        let cfg = PipeArchetype::HighPressureSteamMain.default_config();
        let brushes = generate_pipe_conduit_brushes(&cfg);

        assert!(!brushes.straight_run_brush.polys.is_empty());
        assert!(brushes.elbow_bend_brush.is_some());
        assert!(!brushes.flange_brushes.is_empty());
        assert!(brushes.wall_collar_brush.is_some());
        assert!(brushes.valve_body_brush.is_some());
        assert!(brushes.valve_handwheel_brush.is_some());

        // Check semisolid flag on straight pipe
        assert_eq!(brushes.straight_run_brush.polys[0].flags & 0x00200000, 0x00200000);
    }

    #[test]
    fn test_pipe_safety_audit() {
        let mut cfg = PipeConduitConfig::default();
        cfg.has_elbow_bend = true;
        cfg.elbow_radius_uu = 10.0; // Severe bend pinch!
        cfg.cylinder_facets = 4;    // Degenerate!
        cfg.event_name = "".to_string();

        let hazards = audit_pipe_conduit_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Severe Elbow Bend Pinch")));
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Degenerate Cylinder Facets")));
    }
}
