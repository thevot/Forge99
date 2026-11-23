use glam::Vec3;
use crate::brush::Brush;
use crate::lighting::ray_intersects_triangle;

/// Shape typology of an optical lens flare element
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FlareShape {
    CircularGlow,
    HexagonGhost,
    AnamorphicStreak,
    RingHalo,
    ChromaSpike,
}

impl FlareShape {
    pub fn display_name(&self) -> &'static str {
        match self {
            FlareShape::CircularGlow => "Circular Soft Glow",
            FlareShape::HexagonGhost => "Hexagonal Aperture Ghost",
            FlareShape::AnamorphicStreak => "Anamorphic Horizontal Streak",
            FlareShape::RingHalo => "Fresnel Ring Halo",
            FlareShape::ChromaSpike => "Prismatic Starburst Spike",
        }
    }
}

/// Single ghost element along the glare projection axis
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LensFlareElement {
    pub distance_factor: f32, // -1.0 (behind source) .. 0.0 (at source) .. 1.0 (at screen center) .. 2.0 (opposite edge)
    pub size_scale: f32,
    pub color_rgba: [f32; 4],
    pub shape: FlareShape,
}

/// Configuration settings for an actor's corona & lens flare
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CoronaConfig {
    pub is_enabled: bool,
    pub texture_name: String,
    pub draw_scale: f32,
    pub intensity: f32,
    pub color_rgb: Vec3,
    pub use_sun_mode: bool, // OldUnreal v227 / UT99 v469 celestial sun mode
}

impl Default for CoronaConfig {
    fn default() -> Self {
        Self {
            is_enabled: true,
            texture_name: "GenIn.LensFl1".to_string(),
            draw_scale: 1.0,
            intensity: 1.0,
            color_rgb: Vec3::ONE,
            use_sun_mode: false,
        }
    }
}

/// Canonical corona and lens flare presets inspired by iconic Unreal Tournament atmospheres
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CoronaPreset {
    BlazingDesertSun,
    CyberpunkSodiumStreetlamp,
    SecurityBeaconSiren,
    HighVoltagePlasmaArc,
    AlienBioluminescentOrb,
    AnamorphicSciFiStarburst,
}

impl CoronaPreset {
    pub fn all() -> &'static [CoronaPreset] {
        &[
            CoronaPreset::BlazingDesertSun,
            CoronaPreset::CyberpunkSodiumStreetlamp,
            CoronaPreset::SecurityBeaconSiren,
            CoronaPreset::HighVoltagePlasmaArc,
            CoronaPreset::AlienBioluminescentOrb,
            CoronaPreset::AnamorphicSciFiStarburst,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CoronaPreset::BlazingDesertSun => "☀ Blazing Desert Sun (Facing Worlds / DM-Phobos)",
            CoronaPreset::CyberpunkSodiumStreetlamp => "💡 Cyberpunk Sodium Streetlamp (CityIntro / Turbine)",
            CoronaPreset::SecurityBeaconSiren => "🚨 Security Beacon Siren (CTF-Coret / High Guard)",
            CoronaPreset::HighVoltagePlasmaArc => "⚡ High-Voltage Plasma Arc (Deck 16 Core)",
            CoronaPreset::AlienBioluminescentOrb => "🔮 Alien Bioluminescent Orb (Crypt / Dark Temple)",
            CoronaPreset::AnamorphicSciFiStarburst => "🌟 Anamorphic Sci-Fi Starburst (Deep Space Rig)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            CoronaPreset::BlazingDesertSun => {
                "Intense solar corona with concentric Fresnel ring halos and dual hexagonal aperture ghosts."
            }
            CoronaPreset::CyberpunkSodiumStreetlamp => {
                "Warm sodium-vapor amber glow with subtle horizontal glare streak for wet city streets."
            }
            CoronaPreset::SecurityBeaconSiren => {
                "High-contrast pulsating red siren flare with sharp optical spikes for emergency lockdown sectors."
            }
            CoronaPreset::HighVoltagePlasmaArc => {
                "Cold electric cyan-blue discharge corona with high-frequency chromatic aberrations."
            }
            CoronaPreset::AlienBioluminescentOrb => {
                "Deep violet-magenta organic orb with soft ethereal corona and purple ring halo."
            }
            CoronaPreset::AnamorphicSciFiStarburst => {
                "Cinematic blue horizontal anamorphic streak and central 8-point diamond diffraction starburst."
            }
        }
    }

    pub fn to_config(&self) -> CoronaConfig {
        match self {
            CoronaPreset::BlazingDesertSun => CoronaConfig {
                is_enabled: true,
                texture_name: "GenIn.SunCorona".to_string(),
                draw_scale: 2.5,
                intensity: 1.2,
                color_rgb: Vec3::new(1.0, 0.92, 0.70),
                use_sun_mode: true,
            },
            CoronaPreset::CyberpunkSodiumStreetlamp => CoronaConfig {
                is_enabled: true,
                texture_name: "GenIn.LensFl1".to_string(),
                draw_scale: 1.0,
                intensity: 0.95,
                color_rgb: Vec3::new(1.0, 0.65, 0.20),
                use_sun_mode: false,
            },
            CoronaPreset::SecurityBeaconSiren => CoronaConfig {
                is_enabled: true,
                texture_name: "GenIn.LensFl2".to_string(),
                draw_scale: 1.4,
                intensity: 1.5,
                color_rgb: Vec3::new(1.0, 0.15, 0.15),
                use_sun_mode: false,
            },
            CoronaPreset::HighVoltagePlasmaArc => CoronaConfig {
                is_enabled: true,
                texture_name: "GenIn.LensFl3".to_string(),
                draw_scale: 1.2,
                intensity: 1.3,
                color_rgb: Vec3::new(0.25, 0.85, 1.0),
                use_sun_mode: false,
            },
            CoronaPreset::AlienBioluminescentOrb => CoronaConfig {
                is_enabled: true,
                texture_name: "GenIn.LensFl1".to_string(),
                draw_scale: 1.6,
                intensity: 0.85,
                color_rgb: Vec3::new(0.80, 0.25, 1.0),
                use_sun_mode: false,
            },
            CoronaPreset::AnamorphicSciFiStarburst => CoronaConfig {
                is_enabled: true,
                texture_name: "GenIn.LensFl2".to_string(),
                draw_scale: 2.0,
                intensity: 1.4,
                color_rgb: Vec3::new(0.40, 0.75, 1.0),
                use_sun_mode: true,
            },
        }
    }

    pub fn generate_flare_chain(&self) -> Vec<LensFlareElement> {
        let base_col = self.to_config().color_rgb;
        let c = [base_col.x, base_col.y, base_col.z, 0.85];

        match self {
            CoronaPreset::BlazingDesertSun => vec![
                LensFlareElement {
                    distance_factor: 0.0,
                    size_scale: 1.8,
                    color_rgba: c,
                    shape: FlareShape::CircularGlow,
                },
                LensFlareElement {
                    distance_factor: 0.25,
                    size_scale: 0.8,
                    color_rgba: [c[0], c[1] * 0.9, c[2] * 0.5, 0.6],
                    shape: FlareShape::HexagonGhost,
                },
                LensFlareElement {
                    distance_factor: 0.55,
                    size_scale: 1.2,
                    color_rgba: [c[0] * 0.9, c[1] * 0.8, c[2] * 0.4, 0.5],
                    shape: FlareShape::RingHalo,
                },
                LensFlareElement {
                    distance_factor: 0.85,
                    size_scale: 0.6,
                    color_rgba: [c[0], c[1], c[2] * 0.6, 0.4],
                    shape: FlareShape::HexagonGhost,
                },
                LensFlareElement {
                    distance_factor: 1.35,
                    size_scale: 1.4,
                    color_rgba: [c[0] * 0.8, c[1] * 0.7, c[2] * 0.3, 0.35],
                    shape: FlareShape::CircularGlow,
                },
            ],
            CoronaPreset::AnamorphicSciFiStarburst => vec![
                LensFlareElement {
                    distance_factor: 0.0,
                    size_scale: 2.2,
                    color_rgba: c,
                    shape: FlareShape::AnamorphicStreak,
                },
                LensFlareElement {
                    distance_factor: 0.0,
                    size_scale: 1.5,
                    color_rgba: [1.0, 1.0, 1.0, 0.95],
                    shape: FlareShape::ChromaSpike,
                },
                LensFlareElement {
                    distance_factor: 0.4,
                    size_scale: 0.7,
                    color_rgba: [c[0] * 0.7, c[1] * 0.9, c[2], 0.5],
                    shape: FlareShape::HexagonGhost,
                },
                LensFlareElement {
                    distance_factor: 1.1,
                    size_scale: 1.2,
                    color_rgba: [c[0] * 0.5, c[1] * 0.8, c[2], 0.3],
                    shape: FlareShape::AnamorphicStreak,
                },
            ],
            _ => vec![
                LensFlareElement {
                    distance_factor: 0.0,
                    size_scale: 1.2,
                    color_rgba: c,
                    shape: FlareShape::CircularGlow,
                },
                LensFlareElement {
                    distance_factor: 0.3,
                    size_scale: 0.5,
                    color_rgba: [c[0], c[1], c[2], 0.5],
                    shape: FlareShape::HexagonGhost,
                },
                LensFlareElement {
                    distance_factor: 0.7,
                    size_scale: 0.8,
                    color_rgba: [c[0], c[1], c[2], 0.4],
                    shape: FlareShape::RingHalo,
                },
            ],
        }
    }
}

/// Evaluates visibility and partial occlusion of a corona source by casting rays
/// through world BSP geometry. Returns a ratio in [0.0 (fully occluded) .. 1.0 (fully visible)].
pub fn calculate_corona_occlusion(
    light_pos: Vec3,
    camera_pos: Vec3,
    brushes: &[Brush],
    corona_radius_uu: f32,
) -> f32 {
    let to_light = light_pos - camera_pos;
    let dist = to_light.length();
    if dist < 1.0 {
        return 1.0;
    }
    let ray_dir = to_light / dist;

    // Up and right basis for jittered multi-sample penumbra
    let up_hint = if ray_dir.z.abs() > 0.85 { Vec3::Y } else { Vec3::Z };
    let right = ray_dir.cross(up_hint).normalize();
    let up = right.cross(ray_dir).normalize();

    let sample_offsets = [
        Vec3::ZERO,
        right * (corona_radius_uu * 0.5),
        -right * (corona_radius_uu * 0.5),
        up * (corona_radius_uu * 0.5),
        -up * (corona_radius_uu * 0.5),
    ];

    let mut unoccluded_samples = 0;

    for offset in &sample_offsets {
        let sample_target = light_pos + *offset;
        let sample_to = sample_target - camera_pos;
        let sample_dist = sample_to.length();
        let sample_dir = sample_to / sample_dist;

        let mut occluded = false;

        'brush_loop: for brush in brushes {
            // Check only solid CSG brushes
            if brush.csg_oper == crate::CsgOperation::Subtract {
                continue;
            }

            for poly in &brush.polys {
                // Ignore non-solid or portal sheets
                if (poly.flags & (crate::poly::PF_NOT_SOLID | crate::poly::PF_PORTAL)) != 0 {
                    continue;
                }

                // Check against poly triangles
                if poly.vertices.len() >= 3 {
                    let v0 = poly.vertices[0] + brush.location;
                    for k in 1..poly.vertices.len() - 1 {
                        let v1 = poly.vertices[k] + brush.location;
                        let v2 = poly.vertices[k + 1] + brush.location;

                        if let Some(t_hit) = ray_intersects_triangle(camera_pos, sample_dir, v0, v1, v2) {
                            if t_hit > 1e-2 && t_hit < sample_dist - 1e-2 {
                                occluded = true;
                                break 'brush_loop;
                            }
                        }
                    }
                }
            }
        }

        if !occluded {
            unoccluded_samples += 1;
        }
    }

    (unoccluded_samples as f32) / (sample_offsets.len() as f32)
}

/// Screen projection result for rendering lens flare chains in viewport
#[derive(Debug, Clone, PartialEq)]
pub struct LensFlareProjection {
    pub screen_pos_ndc: (f32, f32), // Normalized device coordinates (-1.0 .. 1.0)
    pub is_in_front: bool,
    pub angular_fade: f32, // Fade factor based on camera center angle (1.0 at center, fades near edges)
    pub elements: Vec<LensFlareElement>,
}

/// Projects a 3D light location into 2D camera viewport coordinates and computes the lens flare chain
pub fn project_lens_flare(
    light_pos: Vec3,
    camera_pos: Vec3,
    camera_forward: Vec3,
    camera_right: Vec3,
    camera_up: Vec3,
    fov_deg: f32,
    aspect_ratio: f32,
    preset: CoronaPreset,
) -> Option<LensFlareProjection> {
    let to_light = light_pos - camera_pos;
    let dist = to_light.length();
    if dist < 1.0 {
        return None;
    }

    let fwd_dist = to_light.dot(camera_forward);
    if fwd_dist <= 1e-2 {
        return None; // Behind camera
    }

    let half_fov_rad = (fov_deg * 0.5).to_radians();
    let tan_half_fov = half_fov_rad.tan();

    let x_cam = to_light.dot(camera_right) / (fwd_dist * tan_half_fov * aspect_ratio);
    let y_cam = to_light.dot(camera_up) / (fwd_dist * tan_half_fov);

    // Angular fade: full intensity at center, fading out towards boundary
    let r2 = x_cam * x_cam + y_cam * y_cam;
    if r2 > 2.5 {
        return None; // Too far off-screen
    }
    let angular_fade = (1.0 - (r2 / 2.5)).clamp(0.0, 1.0);

    Some(LensFlareProjection {
        screen_pos_ndc: (x_cam, y_cam),
        is_in_front: true,
        angular_fade,
        elements: preset.generate_flare_chain(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corona_presets_and_configs() {
        let presets = CoronaPreset::all();
        assert_eq!(presets.len(), 6);

        for p in presets {
            assert!(!p.display_name().is_empty());
            assert!(!p.description().is_empty());
            let config = p.to_config();
            assert!(config.is_enabled);
            assert!(config.draw_scale > 0.0);
            assert!(config.intensity > 0.0);
            assert!(!config.texture_name.is_empty());

            let chain = p.generate_flare_chain();
            assert!(!chain.is_empty());
            assert!(chain.iter().any(|e| e.distance_factor == 0.0), "Must contain primary source element");
        }
    }

    #[test]
    fn test_calculate_corona_occlusion_unobstructed() {
        let camera_pos = Vec3::new(0.0, 0.0, 0.0);
        let light_pos = Vec3::new(0.0, 1000.0, 0.0);
        let brushes: Vec<Brush> = Vec::new();

        let occlusion = calculate_corona_occlusion(light_pos, camera_pos, &brushes, 32.0);
        assert_eq!(occlusion, 1.0, "Empty world must have 1.0 unoccluded ratio");
    }

    #[test]
    fn test_project_lens_flare_behind_camera() {
        let camera_pos = Vec3::new(0.0, 0.0, 0.0);
        let camera_fwd = Vec3::new(0.0, 1.0, 0.0);
        let camera_right = Vec3::new(1.0, 0.0, 0.0);
        let camera_up = Vec3::new(0.0, 0.0, 1.0);

        // Light behind camera (Y = -500)
        let light_behind = Vec3::new(0.0, -500.0, 0.0);
        let proj = project_lens_flare(
            light_behind,
            camera_pos,
            camera_fwd,
            camera_right,
            camera_up,
            90.0,
            1.333,
            CoronaPreset::BlazingDesertSun,
        );
        assert!(proj.is_none(), "Light behind camera must not produce lens flare");
    }

    #[test]
    fn test_project_lens_flare_in_front_center() {
        let camera_pos = Vec3::new(0.0, 0.0, 0.0);
        let camera_fwd = Vec3::new(0.0, 1.0, 0.0);
        let camera_right = Vec3::new(1.0, 0.0, 0.0);
        let camera_up = Vec3::new(0.0, 0.0, 1.0);

        // Light dead center in front
        let light_center = Vec3::new(0.0, 500.0, 0.0);
        let proj = project_lens_flare(
            light_center,
            camera_pos,
            camera_fwd,
            camera_right,
            camera_up,
            90.0,
            1.0,
            CoronaPreset::BlazingDesertSun,
        );
        assert!(proj.is_some());
        let p = proj.unwrap();
        assert_eq!(p.screen_pos_ndc, (0.0, 0.0));
        assert!(p.angular_fade > 0.99);
        assert_eq!(p.elements.len(), 5);
    }
}
