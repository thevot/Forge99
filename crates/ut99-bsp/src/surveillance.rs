// crates/ut99-bsp/src/surveillance.rs
//
// Interactive Security CCTV Camera, Live Video Monitor & Surveillance Network Studio
//
// Mathematical model for surveillance cameras, patrol sweep kinematics (harmonic pan/tilt),
// view frustum geometry, CRT/night-vision/thermal monitor preview generation,
// monitor brush generation, and camera sightline occlusion auditing.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Classical Unreal Tournament & Unreal 1 surveillance camera archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurveillanceArchetype {
    PerimeterTurretSecurity,
    VaultDomeCCTV,
    IndustrialFacilityMonitor,
    CorridorCheckpointScanner,
    TacticalDroneRecon,
}

impl SurveillanceArchetype {
    pub fn all() -> &'static [SurveillanceArchetype] {
        &[
            SurveillanceArchetype::PerimeterTurretSecurity,
            SurveillanceArchetype::VaultDomeCCTV,
            SurveillanceArchetype::IndustrialFacilityMonitor,
            SurveillanceArchetype::CorridorCheckpointScanner,
            SurveillanceArchetype::TacticalDroneRecon,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SurveillanceArchetype::PerimeterTurretSecurity => "Perimeter Security Pan/Tilt Camera",
            SurveillanceArchetype::VaultDomeCCTV => "Ceiling Dome 360° Fisheye CCTV",
            SurveillanceArchetype::IndustrialFacilityMonitor => "Industrial CRT Security Terminal",
            SurveillanceArchetype::CorridorCheckpointScanner => "Corridor Checkpoint Laser Scanner",
            SurveillanceArchetype::TacticalDroneRecon => "Tactical Aerial Recon & Thermal Drone",
        }
    }

    pub fn default_config(&self) -> SurveillanceConfig {
        match self {
            SurveillanceArchetype::PerimeterTurretSecurity => SurveillanceConfig {
                archetype: *self,
                camera_mount: CameraMountConfig {
                    position: Vec3::new(512.0, 512.0, 256.0),
                    base_pitch_deg: -20.0,
                    base_yaw_deg: 45.0,
                    base_roll_deg: 0.0,
                    fov_deg: 65.0,
                    max_range: 2048.0,
                    near_clip: 16.0,
                },
                patrol: CameraPatrolConfig {
                    pan_sweep_deg: 45.0,
                    tilt_sweep_deg: 10.0,
                    sweep_period_seconds: 6.0,
                    pause_at_extremes_seconds: 1.0,
                    is_continuous_spin: false,
                },
                monitor: MonitorDisplayConfig {
                    aspect_ratio_w: 4,
                    aspect_ratio_h: 3,
                    filter_mode: VideoFilterMode::ColorCCTVNoise,
                    scanline_intensity: 0.35,
                    static_noise_factor: 0.15,
                    osd_camera_name: "CAM 01 - PERIMETER WEST".to_string(),
                    show_rec_indicator: true,
                    show_timestamp_osd: true,
                },
                monitor_screen_dimensions: Vec3::new(64.0, 12.0, 48.0),
                camera_tag: "SecCam_Perimeter01".to_string(),
                screen_event: "Screen_SecFeed01".to_string(),
                monitor_texture: "Indus.Monitor.ScreenFace01".to_string(),
                bezel_texture: "Indus.Metal.TerminalBezel01".to_string(),
            },
            SurveillanceArchetype::VaultDomeCCTV => SurveillanceConfig {
                archetype: *self,
                camera_mount: CameraMountConfig {
                    position: Vec3::new(0.0, 0.0, 384.0),
                    base_pitch_deg: -65.0,
                    base_yaw_deg: 0.0,
                    base_roll_deg: 0.0,
                    fov_deg: 95.0,
                    max_range: 1536.0,
                    near_clip: 16.0,
                },
                patrol: CameraPatrolConfig {
                    pan_sweep_deg: 60.0,
                    tilt_sweep_deg: 15.0,
                    sweep_period_seconds: 8.0,
                    pause_at_extremes_seconds: 1.5,
                    is_continuous_spin: false,
                },
                monitor: MonitorDisplayConfig {
                    aspect_ratio_w: 4,
                    aspect_ratio_h: 3,
                    filter_mode: VideoFilterMode::CRTScanlinesMonochrome,
                    scanline_intensity: 0.50,
                    static_noise_factor: 0.20,
                    osd_camera_name: "CAM 04 - VAULT INNER CORE".to_string(),
                    show_rec_indicator: true,
                    show_timestamp_osd: true,
                },
                monitor_screen_dimensions: Vec3::new(80.0, 16.0, 60.0),
                camera_tag: "SecCam_VaultDome".to_string(),
                screen_event: "Screen_VaultCCTV".to_string(),
                monitor_texture: "Indus.Monitor.MonoCRT01".to_string(),
                bezel_texture: "Indus.Metal.DarkBezel01".to_string(),
            },
            SurveillanceArchetype::IndustrialFacilityMonitor => SurveillanceConfig {
                archetype: *self,
                camera_mount: CameraMountConfig {
                    position: Vec3::new(-256.0, 384.0, 192.0),
                    base_pitch_deg: -15.0,
                    base_yaw_deg: 90.0,
                    base_roll_deg: 0.0,
                    fov_deg: 75.0,
                    max_range: 1792.0,
                    near_clip: 16.0,
                },
                patrol: CameraPatrolConfig {
                    pan_sweep_deg: 30.0,
                    tilt_sweep_deg: 5.0,
                    sweep_period_seconds: 5.0,
                    pause_at_extremes_seconds: 0.8,
                    is_continuous_spin: false,
                },
                monitor: MonitorDisplayConfig {
                    aspect_ratio_w: 4,
                    aspect_ratio_h: 3,
                    filter_mode: VideoFilterMode::GreenNightVision,
                    scanline_intensity: 0.45,
                    static_noise_factor: 0.25,
                    osd_camera_name: "CAM 02 - SMELTER BAY".to_string(),
                    show_rec_indicator: true,
                    show_timestamp_osd: true,
                },
                monitor_screen_dimensions: Vec3::new(72.0, 14.0, 54.0),
                camera_tag: "SecCam_SmelterBay".to_string(),
                screen_event: "Screen_SmelterMonitor".to_string(),
                monitor_texture: "Indus.Monitor.GreenPhos01".to_string(),
                bezel_texture: "Indus.Metal.RivetBezel01".to_string(),
            },
            SurveillanceArchetype::CorridorCheckpointScanner => SurveillanceConfig {
                archetype: *self,
                camera_mount: CameraMountConfig {
                    position: Vec3::new(0.0, -512.0, 160.0),
                    base_pitch_deg: -10.0,
                    base_yaw_deg: 90.0,
                    base_roll_deg: 0.0,
                    fov_deg: 80.0,
                    max_range: 1280.0,
                    near_clip: 16.0,
                },
                patrol: CameraPatrolConfig {
                    pan_sweep_deg: 20.0,
                    tilt_sweep_deg: 0.0,
                    sweep_period_seconds: 4.0,
                    pause_at_extremes_seconds: 0.5,
                    is_continuous_spin: false,
                },
                monitor: MonitorDisplayConfig {
                    aspect_ratio_w: 16,
                    aspect_ratio_h: 9,
                    filter_mode: VideoFilterMode::HighDefClean,
                    scanline_intensity: 0.15,
                    static_noise_factor: 0.05,
                    osd_camera_name: "GATE 07 - CHECKPOINT ID".to_string(),
                    show_rec_indicator: true,
                    show_timestamp_osd: true,
                },
                monitor_screen_dimensions: Vec3::new(96.0, 12.0, 54.0),
                camera_tag: "SecCam_CheckpointGate".to_string(),
                screen_event: "Screen_GateID".to_string(),
                monitor_texture: "Indus.Monitor.HiDefScreen01".to_string(),
                bezel_texture: "Indus.Metal.SleekFrame01".to_string(),
            },
            SurveillanceArchetype::TacticalDroneRecon => SurveillanceConfig {
                archetype: *self,
                camera_mount: CameraMountConfig {
                    position: Vec3::new(0.0, 0.0, 768.0),
                    base_pitch_deg: -45.0,
                    base_yaw_deg: 0.0,
                    base_roll_deg: 0.0,
                    fov_deg: 70.0,
                    max_range: 3072.0,
                    near_clip: 32.0,
                },
                patrol: CameraPatrolConfig {
                    pan_sweep_deg: 180.0,
                    tilt_sweep_deg: 20.0,
                    sweep_period_seconds: 12.0,
                    pause_at_extremes_seconds: 0.0,
                    is_continuous_spin: true,
                },
                monitor: MonitorDisplayConfig {
                    aspect_ratio_w: 16,
                    aspect_ratio_h: 9,
                    filter_mode: VideoFilterMode::ThermalInfrared,
                    scanline_intensity: 0.30,
                    static_noise_factor: 0.10,
                    osd_camera_name: "UAV-9 RECON - THERMAL FLIR".to_string(),
                    show_rec_indicator: true,
                    show_timestamp_osd: true,
                },
                monitor_screen_dimensions: Vec3::new(110.0, 16.0, 62.0),
                camera_tag: "DroneCam_Recon09".to_string(),
                screen_event: "Screen_DroneFLIR".to_string(),
                monitor_texture: "Indus.Monitor.ThermalScreen01".to_string(),
                bezel_texture: "Indus.Metal.MilitaryBezel01".to_string(),
            },
        }
    }
}

/// Simulated video filter effect for CRT / CCTV security monitors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFilterMode {
    CRTScanlinesMonochrome,
    GreenNightVision,
    ThermalInfrared,
    ColorCCTVNoise,
    HighDefClean,
}

impl VideoFilterMode {
    pub fn all() -> &'static [VideoFilterMode] {
        &[
            VideoFilterMode::CRTScanlinesMonochrome,
            VideoFilterMode::GreenNightVision,
            VideoFilterMode::ThermalInfrared,
            VideoFilterMode::ColorCCTVNoise,
            VideoFilterMode::HighDefClean,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            VideoFilterMode::CRTScanlinesMonochrome => "CRT Monochrome B&W Scanlines",
            VideoFilterMode::GreenNightVision => "Phosphor Green Night-Vision",
            VideoFilterMode::ThermalInfrared => "Thermal Infrared FLIR Heat-Map",
            VideoFilterMode::ColorCCTVNoise => "Analog Color CCTV Noise & Static",
            VideoFilterMode::HighDefClean => "High-Definition Digital Clean",
        }
    }
}

/// Camera spatial placement and optical lens parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraMountConfig {
    pub position: Vec3,
    pub base_pitch_deg: f32,
    pub base_yaw_deg: f32,
    pub base_roll_deg: f32,
    pub fov_deg: f32,
    pub max_range: f32,
    pub near_clip: f32,
}

/// Autonomous pan/tilt patrol sweep motion kinematics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraPatrolConfig {
    pub pan_sweep_deg: f32,
    pub tilt_sweep_deg: f32,
    pub sweep_period_seconds: f32,
    pub pause_at_extremes_seconds: f32,
    pub is_continuous_spin: bool,
}

/// Video screen appearance, scanlines, and on-screen display (OSD) HUD
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorDisplayConfig {
    pub aspect_ratio_w: u32,
    pub aspect_ratio_h: u32,
    pub filter_mode: VideoFilterMode,
    pub scanline_intensity: f32,
    pub static_noise_factor: f32,
    pub osd_camera_name: String,
    pub show_rec_indicator: bool,
    pub show_timestamp_osd: bool,
}

/// Complete configuration for Surveillance Camera & Video Monitor Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurveillanceConfig {
    pub archetype: SurveillanceArchetype,
    pub camera_mount: CameraMountConfig,
    pub patrol: CameraPatrolConfig,
    pub monitor: MonitorDisplayConfig,
    pub monitor_screen_dimensions: Vec3, // W, D, H
    pub camera_tag: String,
    pub screen_event: String,
    pub monitor_texture: String,
    pub bezel_texture: String,
}

impl Default for SurveillanceConfig {
    fn default() -> Self {
        SurveillanceArchetype::PerimeterTurretSecurity.default_config()
    }
}

/// 3D View Frustum corners and directional vectors for a camera
#[derive(Debug, Clone, PartialEq)]
pub struct CameraFrustum {
    pub origin: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub right: Vec3,
    pub near_corners: [Vec3; 4], // TopLeft, TopRight, BotRight, BotLeft
    pub far_corners: [Vec3; 4],  // TopLeft, TopRight, BotRight, BotLeft
    pub target_point: Vec3,
}

/// Diagnostic safety hazard detected during surveillance network audit
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurveillanceHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Evaluates current pan and tilt angles at time `time_sec`
pub fn sample_patrol_orientation(
    patrol: &CameraPatrolConfig,
    base_yaw: f32,
    base_pitch: f32,
    time_sec: f32,
) -> (f32, f32) {
    if patrol.is_continuous_spin {
        let period = patrol.sweep_period_seconds.max(1.0);
        let cycle = (time_sec % period) / period;
        let yaw = (base_yaw + cycle * 360.0).rem_euclid(360.0);
        return (yaw, base_pitch);
    }

    let period = patrol.sweep_period_seconds.max(1.0);
    let t_mod = time_sec % period;

    // Harmonic sinusoidal oscillation: -1.0 to 1.0
    let wave = (t_mod * std::f32::consts::TAU / period).sin();

    let yaw = base_yaw + wave * patrol.pan_sweep_deg;
    let pitch = base_pitch + wave * patrol.tilt_sweep_deg;

    (yaw, pitch)
}

/// Computes the 3D camera viewing frustum
pub fn calculate_camera_frustum(
    mount: &CameraMountConfig,
    current_yaw: f32,
    current_pitch: f32,
    aspect_ratio: f32,
) -> CameraFrustum {
    let rot = Quat::from_rotation_z(current_yaw.to_radians())
        * Quat::from_rotation_y(current_pitch.to_radians())
        * Quat::from_rotation_x(mount.base_roll_deg.to_radians());

    let forward = rot * Vec3::X; // Unreal coordinate forward
    let right = rot * Vec3::Y;   // Unreal coordinate right
    let up = rot * Vec3::Z;      // Unreal coordinate up

    let half_fov_rad = (mount.fov_deg * 0.5).to_radians();
    let tan_half_fov = half_fov_rad.tan();

    // Near plane dimensions
    let near_d = mount.near_clip.max(8.0);
    let near_h = near_d * tan_half_fov;
    let near_w = near_h * aspect_ratio;
    let near_center = mount.position + forward * near_d;

    let near_tl = near_center + up * near_h - right * near_w;
    let near_tr = near_center + up * near_h + right * near_w;
    let near_br = near_center - up * near_h + right * near_w;
    let near_bl = near_center - up * near_h - right * near_w;

    // Far plane dimensions
    let far_d = mount.max_range.max(128.0);
    let far_h = far_d * tan_half_fov;
    let far_w = far_h * aspect_ratio;
    let far_center = mount.position + forward * far_d;

    let far_tl = far_center + up * far_h - right * far_w;
    let far_tr = far_center + up * far_h + right * far_w;
    let far_br = far_center - up * far_h + right * far_w;
    let far_bl = far_center - up * far_h - right * far_w;

    CameraFrustum {
        origin: mount.position,
        forward,
        up,
        right,
        near_corners: [near_tl, near_tr, near_br, near_bl],
        far_corners: [far_tl, far_tr, far_br, far_bl],
        target_point: far_center,
    }
}

/// Generates a physical 3D CRT Security Monitor Box brush with recessed screen face
pub fn generate_monitor_brush(config: &SurveillanceConfig) -> Brush {
    let mut brush = Brush::new("Surveillance_Monitor", CsgOperation::Add);

    let dims = config.monitor_screen_dimensions;
    let half_w = dims.x * 0.5;
    let half_d = dims.y * 0.5;
    let half_h = dims.z * 0.5;

    // 8 vertices of the monitor housing
    let v000 = Vec3::new(-half_w, -half_d, -half_h);
    let v001 = Vec3::new(-half_w, -half_d, half_h);
    let v010 = Vec3::new(-half_w, half_d, -half_h);
    let v011 = Vec3::new(-half_w, half_d, half_h);
    let v100 = Vec3::new(half_w, -half_d, -half_h);
    let v101 = Vec3::new(half_w, -half_d, half_h);
    let v110 = Vec3::new(half_w, half_d, -half_h);
    let v111 = Vec3::new(half_w, half_d, half_h);

    // Front Screen Face (+Y direction) - flagged with PF_UNLIT
    if let Some(mut front) = Polygon::new(vec![v010, v110, v111, v011]) {
        front.texture = config.monitor_texture.clone();
        front.flags |= 0x00400000; // PF_UNLIT
        brush.polys.push(front);
    }

    // Rear Casing (-Y direction)
    if let Some(mut back) = Polygon::new(vec![v100, v000, v001, v101]) {
        back.texture = config.bezel_texture.clone();
        brush.polys.push(back);
    }

    // Left Casing (-X direction)
    if let Some(mut left) = Polygon::new(vec![v000, v010, v011, v001]) {
        left.texture = config.bezel_texture.clone();
        brush.polys.push(left);
    }

    // Right Casing (+X direction)
    if let Some(mut right) = Polygon::new(vec![v110, v100, v101, v111]) {
        right.texture = config.bezel_texture.clone();
        brush.polys.push(right);
    }

    // Top Casing (+Z direction)
    if let Some(mut top) = Polygon::new(vec![v001, v011, v111, v101]) {
        top.texture = config.bezel_texture.clone();
        brush.polys.push(top);
    }

    // Bottom Casing (-Z direction)
    if let Some(mut bot) = Polygon::new(vec![v010, v000, v100, v110]) {
        bot.texture = config.bezel_texture.clone();
        brush.polys.push(bot);
    }

    brush
}

/// Audits surveillance configuration against level geometry and optical hazards
pub fn audit_surveillance_safety(
    config: &SurveillanceConfig,
    world_brushes: &[Brush],
) -> Vec<SurveillanceHazard> {
    let mut hazards = Vec::new();

    // 1. Check for extreme FOV
    if config.camera_mount.fov_deg > 115.0 {
        hazards.push(SurveillanceHazard {
            is_critical: false,
            title: format!("Extreme Fisheye FOV ({:.0}°)", config.camera_mount.fov_deg),
            description: "Field of view exceeds 115°, which may cause severe edge distortion on monitor feeds.".to_string(),
            recommendation: "Reduce FOV to 75°..95° for clearer surveillance images.".to_string(),
        });
    }

    // 2. Check for missing Tag / Event binding
    if config.camera_tag.trim().is_empty() || config.screen_event.trim().is_empty() {
        hazards.push(SurveillanceHazard {
            is_critical: true,
            title: "Unbound Camera / Screen Event Tag".to_string(),
            description: "Camera Tag or Screen Event name is empty. Video feed will fail to display in-game.".to_string(),
            recommendation: "Provide matching unique Tag and Event names (e.g. 'SecCam_Vault01').".to_string(),
        });
    }

    // 3. Sightline Raycast Occlusion Audit
    // Check if the camera origin is inside or within 32 uu of any solid world brush
    let cam_pos = config.camera_mount.position;
    for (idx, brush) in world_brushes.iter().enumerate() {
        if brush.csg_oper == CsgOperation::Add {
            for poly in &brush.polys {
                let center = poly.vertices.iter().copied().sum::<Vec3>() / poly.vertices.len().max(1) as f32;
                if cam_pos.distance(center) < 32.0 {
                    hazards.push(SurveillanceHazard {
                        is_critical: true,
                        title: format!("Camera Lens Obstructed by Brush #{}", idx),
                        description: "Camera mount is embedded inside or touching world solid geometry.".to_string(),
                        recommendation: "Offset camera mount away from walls or ceilings.".to_string(),
                    });
                    break;
                }
            }
        }
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surveillance_archetypes_and_defaults() {
        for &arch in SurveillanceArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.camera_mount.fov_deg > 30.0 && cfg.camera_mount.fov_deg < 130.0);
            assert!(cfg.camera_mount.max_range > 500.0);
            assert!(!cfg.camera_tag.is_empty());
            assert!(!cfg.screen_event.is_empty());
        }
    }

    #[test]
    fn test_patrol_kinematics_and_continuous_spin() {
        let mut cfg = SurveillanceArchetype::PerimeterTurretSecurity.default_config();
        cfg.patrol.sweep_period_seconds = 4.0;
        cfg.patrol.pan_sweep_deg = 30.0;

        let (yaw_0, _) = sample_patrol_orientation(&cfg.patrol, 90.0, 0.0, 0.0);
        let (yaw_1, _) = sample_patrol_orientation(&cfg.patrol, 90.0, 0.0, 1.0);
        let (yaw_2, _) = sample_patrol_orientation(&cfg.patrol, 90.0, 0.0, 2.0);

        // Sinusoidal wave: at t=0 wave=0, at t=1 wave=1, at t=2 wave=0
        assert!((yaw_0 - 90.0).abs() < 1e-3);
        assert!((yaw_1 - 120.0).abs() < 1e-3);
        assert!((yaw_2 - 90.0).abs() < 1e-3);

        // Test continuous spin
        cfg.patrol.is_continuous_spin = true;
        let (yaw_spin_0, _) = sample_patrol_orientation(&cfg.patrol, 0.0, 0.0, 0.0);
        let (yaw_spin_half, _) = sample_patrol_orientation(&cfg.patrol, 0.0, 0.0, 2.0);
        assert_eq!(yaw_spin_0, 0.0);
        assert!((yaw_spin_half - 180.0).abs() < 1e-3);
    }

    #[test]
    fn test_calculate_camera_frustum() {
        let cfg = SurveillanceArchetype::VaultDomeCCTV.default_config();
        let frustum = calculate_camera_frustum(&cfg.camera_mount, 0.0, 0.0, 4.0 / 3.0);

        assert_eq!(frustum.origin, cfg.camera_mount.position);
        assert!((frustum.forward.length() - 1.0).abs() < 1e-4);
        assert!((frustum.up.length() - 1.0).abs() < 1e-4);
        assert!((frustum.right.length() - 1.0).abs() < 1e-4);

        // Far corners should be further away than near corners
        assert!(frustum.far_corners[0].distance(frustum.origin) > frustum.near_corners[0].distance(frustum.origin));
    }

    #[test]
    fn test_generate_monitor_brush() {
        let cfg = SurveillanceArchetype::IndustrialFacilityMonitor.default_config();
        let brush = generate_monitor_brush(&cfg);

        assert_eq!(brush.polys.len(), 6, "Must generate 6-sided box");

        // Front face must be PF_UNLIT (0x00400000)
        let unlit_poly = brush.polys.iter().find(|p| p.texture == cfg.monitor_texture);
        assert!(unlit_poly.is_some());
        assert_eq!(unlit_poly.unwrap().flags & 0x00400000, 0x00400000);
    }

    #[test]
    fn test_surveillance_safety_audit() {
        let mut cfg = SurveillanceConfig::default();
        cfg.camera_mount.fov_deg = 125.0; // Extreme fisheye
        cfg.camera_tag = String::new(); // Unbound tag

        let hazards = audit_surveillance_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Unbound")));
        assert!(hazards.iter().any(|h| !h.is_critical && h.title.contains("Fisheye")));
    }
}
