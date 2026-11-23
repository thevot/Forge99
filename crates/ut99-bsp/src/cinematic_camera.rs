use glam::Vec3;

/// Single keyframe node along a cinematic camera spline path
#[derive(Debug, Clone, PartialEq)]
pub struct SplineCameraKeyframe {
    pub position: Vec3,
    pub rotation: Vec3, // (Pitch, Yaw, Roll) in degrees
    pub fov: f32,       // Field of view in degrees (e.g. 60.0 .. 110.0)
    pub speed: f32,     // Target flight speed in uu/s
    pub pause_time: f32,// Pause time in seconds on this node
    pub label: String,
}

impl Default for SplineCameraKeyframe {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            fov: 90.0,
            speed: 350.0,
            pause_time: 0.0,
            label: "Keyframe".to_string(),
        }
    }
}

/// Canonical cinematic camera presets for Unreal Tournament maps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CinematicPreset {
    ArenaFlybyIntro360,
    CorridorTrackingShot,
    TowerDramaticReveal,
    FlagCaptureDrama,
}

impl CinematicPreset {
    pub fn all() -> &'static [CinematicPreset] {
        &[
            CinematicPreset::ArenaFlybyIntro360,
            CinematicPreset::CorridorTrackingShot,
            CinematicPreset::TowerDramaticReveal,
            CinematicPreset::FlagCaptureDrama,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            CinematicPreset::ArenaFlybyIntro360 => "Arena 360° Orbital Flyby Intro",
            CinematicPreset::CorridorTrackingShot => "High-Speed Corridor Tracking Shot",
            CinematicPreset::TowerDramaticReveal => "Monumental Tower Spiral Ascend Reveal",
            CinematicPreset::FlagCaptureDrama => "Flag Base Dramatic Dolly Zoom",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            CinematicPreset::ArenaFlybyIntro360 => "Smooth circular orbital sweep around the center of the arena showcasing geometry.",
            CinematicPreset::CorridorTrackingShot => "Dynamic eye-level traversal through choke points with roll banking on high-speed turns.",
            CinematicPreset::TowerDramaticReveal => "Helical vertical ascent starting from base ground and pivoting upwards toward sniper perches.",
            CinematicPreset::FlagCaptureDrama => "Fast forward push towards the CTF flag pedestal with focal compression (FOV 90° -> 45°).",
        }
    }

    pub fn create_track(&self) -> CinematicCameraTrack {
        match self {
            CinematicPreset::ArenaFlybyIntro360 => {
                let center = Vec3::new(0.0, 0.0, 150.0);
                let radius = 600.0;
                let mut kfs = Vec::new();
                let steps = 8;
                for i in 0..steps {
                    let angle = (i as f32 / steps as f32) * std::f32::consts::TAU;
                    let pos = center + Vec3::new(angle.cos() * radius, angle.sin() * radius, 60.0 * (i as f32 * 0.5).sin());
                    let yaw = (angle + std::f32::consts::PI * 0.5).to_degrees();
                    kfs.push(SplineCameraKeyframe {
                        position: pos,
                        rotation: Vec3::new(-10.0, yaw, 0.0),
                        fov: 90.0,
                        speed: 300.0,
                        pause_time: 0.0,
                        label: format!("OrbitNode_{i}"),
                    });
                }
                CinematicCameraTrack {
                    name: "ArenaFlybyIntro360".to_string(),
                    keyframes: kfs,
                    is_looping: true,
                }
            }
            CinematicPreset::CorridorTrackingShot => {
                let kfs = vec![
                    SplineCameraKeyframe { position: Vec3::new(-800.0, 0.0, 80.0), rotation: Vec3::new(0.0, 0.0, 0.0), fov: 95.0, speed: 400.0, pause_time: 0.0, label: "Entry".to_string() },
                    SplineCameraKeyframe { position: Vec3::new(-200.0, 50.0, 80.0), rotation: Vec3::new(0.0, 10.0, 5.0), fov: 90.0, speed: 450.0, pause_time: 0.0, label: "Curve".to_string() },
                    SplineCameraKeyframe { position: Vec3::new(400.0, -50.0, 90.0), rotation: Vec3::new(0.0, -10.0, -5.0), fov: 85.0, speed: 450.0, pause_time: 0.0, label: "S-Bend".to_string() },
                    SplineCameraKeyframe { position: Vec3::new(900.0, 0.0, 100.0), rotation: Vec3::new(0.0, 0.0, 0.0), fov: 80.0, speed: 300.0, pause_time: 1.0, label: "Exit".to_string() },
                ];
                CinematicCameraTrack {
                    name: "CorridorTrackingShot".to_string(),
                    keyframes: kfs,
                    is_looping: false,
                }
            }
            CinematicPreset::TowerDramaticReveal => {
                let kfs = vec![
                    SplineCameraKeyframe { position: Vec3::new(0.0, 400.0, 30.0), rotation: Vec3::new(25.0, -90.0, 0.0), fov: 90.0, speed: 200.0, pause_time: 0.5, label: "BaseLookUp".to_string() },
                    SplineCameraKeyframe { position: Vec3::new(350.0, 150.0, 250.0), rotation: Vec3::new(15.0, -150.0, 0.0), fov: 88.0, speed: 280.0, pause_time: 0.0, label: "Ascent1".to_string() },
                    SplineCameraKeyframe { position: Vec3::new(0.0, -350.0, 500.0), rotation: Vec3::new(5.0, 90.0, 0.0), fov: 85.0, speed: 280.0, pause_time: 0.0, label: "Ascent2".to_string() },
                    SplineCameraKeyframe { position: Vec3::new(-350.0, 0.0, 750.0), rotation: Vec3::new(-5.0, 0.0, 0.0), fov: 80.0, speed: 250.0, pause_time: 0.0, label: "SummitApproach".to_string() },
                    SplineCameraKeyframe { position: Vec3::new(0.0, 200.0, 900.0), rotation: Vec3::new(-20.0, -90.0, 0.0), fov: 75.0, speed: 150.0, pause_time: 2.0, label: "SummitPanorama".to_string() },
                ];
                CinematicCameraTrack {
                    name: "TowerDramaticReveal".to_string(),
                    keyframes: kfs,
                    is_looping: false,
                }
            }
            CinematicPreset::FlagCaptureDrama => {
                let kfs = vec![
                    SplineCameraKeyframe { position: Vec3::new(0.0, 600.0, 160.0), rotation: Vec3::new(-15.0, -90.0, 0.0), fov: 105.0, speed: 250.0, pause_time: 0.5, label: "WideApproach".to_string() },
                    SplineCameraKeyframe { position: Vec3::new(0.0, 250.0, 100.0), rotation: Vec3::new(-8.0, -90.0, 0.0), fov: 85.0, speed: 380.0, pause_time: 0.0, label: "DollyZoomMid".to_string() },
                    SplineCameraKeyframe { position: Vec3::new(0.0, 70.0, 75.0), rotation: Vec3::new(0.0, -90.0, 0.0), fov: 48.0, speed: 120.0, pause_time: 1.5, label: "FlagCloseHero".to_string() },
                ];
                CinematicCameraTrack {
                    name: "FlagCaptureDrama".to_string(),
                    keyframes: kfs,
                    is_looping: false,
                }
            }
        }
    }
}

/// Continuous cinematic camera spline track
#[derive(Debug, Clone, PartialEq)]
pub struct CinematicCameraTrack {
    pub name: String,
    pub keyframes: Vec<SplineCameraKeyframe>,
    pub is_looping: bool,
}

impl Default for CinematicCameraTrack {
    fn default() -> Self {
        CinematicPreset::ArenaFlybyIntro360.create_track()
    }
}

impl CinematicCameraTrack {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            keyframes: Vec::new(),
            is_looping: false,
        }
    }

    /// Evaluates camera Position, Rotation (Pitch, Yaw, Roll degrees), and FOV at normalized t in [0.0, 1.0]
    pub fn sample_at(&self, t_norm: f32) -> (Vec3, Vec3, f32) {
        let n = self.keyframes.len();
        if n == 0 {
            return (Vec3::ZERO, Vec3::ZERO, 90.0);
        }
        if n == 1 {
            return (self.keyframes[0].position, self.keyframes[0].rotation, self.keyframes[0].fov);
        }

        let t_clamped = if self.is_looping {
            t_norm.fract().abs()
        } else {
            t_norm.clamp(0.0, 1.0)
        };

        let span_count = if self.is_looping { n } else { n - 1 };
        let scaled_t = t_clamped * span_count as f32;
        let index = (scaled_t.floor() as usize).min(span_count - 1);
        let u = scaled_t - index as f32;

        let get_node = |idx: isize| -> &SplineCameraKeyframe {
            let m = n as isize;
            let wrapped = if self.is_looping {
                ((idx % m) + m) % m
            } else {
                idx.clamp(0, m - 1)
            };
            &self.keyframes[wrapped as usize]
        };

        let i = index as isize;
        let p0 = get_node(i - 1);
        let p1 = get_node(i);
        let p2 = get_node(i + 1);
        let p3 = get_node(i + 2);

        // Catmull-Rom cubic interpolation for 3D Position
        let pos = catmull_rom_3d(p0.position, p1.position, p2.position, p3.position, u);

        // Smooth interpolation for Rotation
        let rot = Vec3::new(
            lerp_angle_deg(p1.rotation.x, p2.rotation.x, u),
            lerp_angle_deg(p1.rotation.y, p2.rotation.y, u),
            lerp_angle_deg(p1.rotation.z, p2.rotation.z, u),
        );

        // Smooth Lerp for FOV
        let fov = p1.fov + (p2.fov - p1.fov) * u;

        (pos, rot, fov)
    }

    /// Computes total path length in Unreal units
    pub fn total_length(&self) -> f32 {
        let poly = self.sample_polyline(16);
        let mut len = 0.0;
        for i in 0..poly.len().saturating_sub(1) {
            len += poly[i].distance(poly[i + 1]);
        }
        len
    }

    /// Estimates the total duration in seconds of the track based on average keyframe speed
    pub fn estimated_duration(&self) -> f32 {
        let len = self.total_length();
        let avg_speed = if self.keyframes.is_empty() {
            300.0
        } else {
            let sum: f32 = self.keyframes.iter().map(|k| k.speed.max(50.0)).sum();
            sum / self.keyframes.len() as f32
        };
        let mut pause_sum = 0.0;
        for k in &self.keyframes {
            pause_sum += k.pause_time;
        }
        (len / avg_speed) + pause_sum
    }

    /// Discretizes the continuous spline curve into line segments for 3D viewport rendering
    pub fn sample_polyline(&self, segments_per_span: usize) -> Vec<Vec3> {
        let n = self.keyframes.len();
        if n < 2 {
            return self.keyframes.iter().map(|k| k.position).collect();
        }

        let span_count = if self.is_looping { n } else { n - 1 };
        let total_steps = span_count * segments_per_span.max(2);
        let mut points = Vec::with_capacity(total_steps + 1);

        for step in 0..=total_steps {
            let t = step as f32 / total_steps as f32;
            let (pos, _, _) = self.sample_at(t);
            points.push(pos);
        }

        points
    }
}

/// Standard Catmull-Rom cubic spline interpolation in 3D
#[inline]
fn catmull_rom_3d(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;

    0.5 * (
        (2.0 * p1)
            + (-p0 + p2) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
    )
}

/// Shortest-path angular interpolation in degrees
#[inline]
fn lerp_angle_deg(a: f32, b: f32, t: f32) -> f32 {
    let mut diff = (b - a) % 360.0;
    if diff > 180.0 { diff -= 360.0; }
    if diff < -180.0 { diff += 360.0; }
    a + diff * t
}
