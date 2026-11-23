use glam::{Mat4, Vec3};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewportType {
    #[default]
    Perspective,
    Top,   // XY plane, looking -Z
    Front, // XZ plane, looking +Y
    Side,  // YZ plane, looking +X
}

impl ViewportType {
    pub fn label(&self) -> &'static str {
        match self {
            ViewportType::Perspective => "3D Perspective",
            ViewportType::Top => "Top (XY)",
            ViewportType::Front => "Front (XZ)",
            ViewportType::Side => "Side (YZ)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewportLayout {
    #[default]
    Single,
    FourViewports, // 2x2: Top (XY), Front (XZ), Side (YZ), 3D Perspective
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewportShading {
    #[default]
    Lit,             // Full dynamic lighting / lightmaps (Alt+4)
    Unlit,           // Full-bright textured (Alt+3)
    Wireframe,       // Wireframe only (Alt+1)
    ZonePortal,      // Zone & Portal distinct coloring (Alt+2, UnrealEd 2.0 View Mode)
    TextureUsage,    // Distinct color per texture name (UnrealEd 2.0 Texture Usage View Mode)
    BspCuts,         // Distinct color per CSG/BSP carved polygon (UnrealEd 2.0 BSP Cuts View Mode)
    DepthComplexity, // Overdraw / rasterization depth complexity heatmap (Alt+5)
}

impl ViewportShading {
    pub fn label(&self) -> &'static str {
        match self {
            ViewportShading::Lit => "Lit",
            ViewportShading::Unlit => "Unlit",
            ViewportShading::Wireframe => "Wireframe",
            ViewportShading::ZonePortal => "Zone/Portal",
            ViewportShading::TextureUsage => "Texture Usage",
            ViewportShading::BspCuts => "BSP Cuts",
            ViewportShading::DepthComplexity => "Depth Complexity",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ViewportShading::Lit => "☼",
            ViewportShading::Unlit => "☀",
            ViewportShading::Wireframe => "◫",
            ViewportShading::ZonePortal => "▣",
            ViewportShading::TextureUsage => "▦",
            ViewportShading::BspCuts => "✂",
            ViewportShading::DepthComplexity => "▥",
        }
    }

    pub fn display_name(&self) -> String {
        format!("{} {}", self.icon(), self.label())
    }

    pub fn next(&self) -> Self {
        match self {
            ViewportShading::Wireframe => ViewportShading::ZonePortal,
            ViewportShading::ZonePortal => ViewportShading::TextureUsage,
            ViewportShading::TextureUsage => ViewportShading::BspCuts,
            ViewportShading::BspCuts => ViewportShading::Unlit,
            ViewportShading::Unlit => ViewportShading::Lit,
            ViewportShading::Lit => ViewportShading::DepthComplexity,
            ViewportShading::DepthComplexity => ViewportShading::Wireframe,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,   // in radians (around Z axis)
    pub pitch: f32, // in radians (up/down)
    pub speed: f32,
    pub fov_y: f32,
    pub aspect: f32,
    pub z_near: f32,
    pub z_far: f32,
    pub ortho_zoom: f32, // legacy/fallback units from center to top edge in orthographic view

    // Independent 2D Orthographic Cameras (MOU-03)
    pub ortho_pos_top: glam::Vec2,
    pub ortho_zoom_top: f32,
    pub ortho_pos_front: glam::Vec2,
    pub ortho_zoom_front: f32,
    pub ortho_pos_side: glam::Vec2,
    pub ortho_zoom_side: f32,
}

impl Default for Camera {
    fn default() -> Self {
        let default_pos = Vec3::new(-300.0, -300.0, 200.0);
        Self {
            position: default_pos,
            yaw: 45.0f32.to_radians(),
            pitch: -20.0f32.to_radians(),
            speed: 600.0,
            fov_y: 65.0f32.to_radians(),
            aspect: 16.0 / 9.0,
            z_near: 1.0,
            z_far: 50_000.0,
            ortho_zoom: 1024.0,
            ortho_pos_top: glam::Vec2::new(default_pos.x, default_pos.y),
            ortho_zoom_top: 1024.0,
            ortho_pos_front: glam::Vec2::new(default_pos.x, default_pos.z),
            ortho_zoom_front: 1024.0,
            ortho_pos_side: glam::Vec2::new(default_pos.y, default_pos.z),
            ortho_zoom_side: 1024.0,
        }
    }
}

#[allow(dead_code)]
impl Camera {
    pub fn new(position: Vec3, aspect: f32) -> Self {
        Self {
            position,
            aspect,
            ortho_pos_top: glam::Vec2::new(position.x, position.y),
            ortho_pos_front: glam::Vec2::new(position.x, position.z),
            ortho_pos_side: glam::Vec2::new(position.y, position.z),
            ..Default::default()
        }
    }

    /// Center and position the camera to look at a target point from a given distance
    pub fn focus_on(&mut self, target: Vec3, distance: f32) {
        let fwd = self.forward();
        self.position = target - fwd * distance.max(10.0);
        self.ortho_pos_top = glam::Vec2::new(target.x, target.y);
        self.ortho_pos_front = glam::Vec2::new(target.x, target.z);
        self.ortho_pos_side = glam::Vec2::new(target.y, target.z);
    }

    /// Forward direction vector in Unreal coordinate system (Z up)
    pub fn forward(&self) -> Vec3 {
        let cos_p = self.pitch.cos();
        Vec3::new(
            self.yaw.cos() * cos_p,
            self.yaw.sin() * cos_p,
            self.pitch.sin(),
        )
        .normalize()
    }

    /// Right direction vector
    pub fn right(&self) -> Vec3 {
        let fwd = self.forward();
        let up = Vec3::Z;
        up.cross(fwd).normalize()
    }

    /// Up direction vector
    pub fn up(&self) -> Vec3 {
        let fwd = self.forward();
        let right = self.right();
        fwd.cross(right).normalize()
    }

    pub fn view_matrix(&self) -> Mat4 {
        let fwd = self.forward();
        Mat4::look_to_lh(self.position, fwd, Vec3::Z)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_lh(self.fov_y, self.aspect, self.z_near, self.z_far)
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn view_matrix_for_type(&self, vp_type: ViewportType) -> Mat4 {
        match vp_type {
            ViewportType::Perspective => self.view_matrix(),
            ViewportType::Top => Mat4::look_at_rh(
                Vec3::new(self.ortho_pos_top.x, self.ortho_pos_top.y, 40000.0),
                Vec3::new(self.ortho_pos_top.x, self.ortho_pos_top.y, 0.0),
                Vec3::Y,
            ),
            ViewportType::Front => Mat4::look_at_rh(
                Vec3::new(self.ortho_pos_front.x, -40000.0, self.ortho_pos_front.y),
                Vec3::new(self.ortho_pos_front.x, 0.0, self.ortho_pos_front.y),
                Vec3::Z,
            ),
            ViewportType::Side => Mat4::look_at_rh(
                Vec3::new(-40000.0, self.ortho_pos_side.x, self.ortho_pos_side.y),
                Vec3::new(0.0, self.ortho_pos_side.x, self.ortho_pos_side.y),
                Vec3::Z,
            ),
        }
    }

    pub fn projection_matrix_for_type(&self, vp_type: ViewportType, aspect: f32) -> Mat4 {
        match vp_type {
            ViewportType::Perspective => Mat4::perspective_lh(self.fov_y, aspect, self.z_near, self.z_far),
            ViewportType::Top => {
                let half_h = self.ortho_zoom_top;
                let half_w = self.ortho_zoom_top * aspect;
                Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, 1.0, 80000.0)
            }
            ViewportType::Front => {
                let half_h = self.ortho_zoom_front;
                let half_w = self.ortho_zoom_front * aspect;
                Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, 1.0, 80000.0)
            }
            ViewportType::Side => {
                let half_h = self.ortho_zoom_side;
                let half_w = self.ortho_zoom_side * aspect;
                Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, 1.0, 80000.0)
            }
        }
    }

    pub fn view_projection_matrix_for_type(&self, vp_type: ViewportType, aspect: f32) -> Mat4 {
        self.projection_matrix_for_type(vp_type, aspect) * self.view_matrix_for_type(vp_type)
    }

    pub fn get_ortho_zoom(&self, vp_type: ViewportType) -> f32 {
        match vp_type {
            ViewportType::Perspective => self.ortho_zoom,
            ViewportType::Top => self.ortho_zoom_top,
            ViewportType::Front => self.ortho_zoom_front,
            ViewportType::Side => self.ortho_zoom_side,
        }
    }

    pub fn set_ortho_zoom(&mut self, vp_type: ViewportType, zoom: f32) {
        let clamped = zoom.clamp(32.0, 100_000.0);
        match vp_type {
            ViewportType::Perspective => self.ortho_zoom = clamped,
            ViewportType::Top => self.ortho_zoom_top = clamped,
            ViewportType::Front => self.ortho_zoom_front = clamped,
            ViewportType::Side => self.ortho_zoom_side = clamped,
        }
    }

    pub fn pan_ortho(&mut self, vp_type: ViewportType, delta_x: f32, delta_y: f32) {
        match vp_type {
            ViewportType::Perspective => {}
            ViewportType::Top => {
                self.ortho_pos_top.x += delta_x;
                self.ortho_pos_top.y += delta_y;
            }
            ViewportType::Front => {
                self.ortho_pos_front.x += delta_x;
                self.ortho_pos_front.y += delta_y;
            }
            ViewportType::Side => {
                self.ortho_pos_side.x += delta_x;
                self.ortho_pos_side.y += delta_y;
            }
        }
    }

    /// Converts a 2D screen coordinate (physical pixels) inside a viewport rectangle [vx, vy, vw, vh]
    /// into 2D world coordinates (u, v) for the given orthographic viewport type:
    /// - Top: returns (world_x, world_y)
    /// - Front: returns (world_x, world_z)
    /// - Side: returns (world_y, world_z)
    /// Returns None if viewport type is Perspective or if viewport dimensions are zero.
    pub fn screen_to_ortho_world(&self, vp_type: ViewportType, screen_x: f32, screen_y: f32, vp_rect: [f32; 4]) -> Option<(f32, f32)> {
        let [vx, vy, vw, vh] = vp_rect;
        if vw <= 1.0 || vh <= 1.0 {
            return None;
        }
        let ndc_x = (screen_x - vx) / vw * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_y - vy) / vh * 2.0;
        let aspect = vw / vh;

        match vp_type {
            ViewportType::Perspective => None,
            ViewportType::Top => {
                let half_h = self.ortho_zoom_top;
                let half_w = self.ortho_zoom_top * aspect;
                let world_x = self.ortho_pos_top.x + ndc_x * half_w;
                let world_y = self.ortho_pos_top.y + ndc_y * half_h;
                Some((world_x, world_y))
            }
            ViewportType::Front => {
                let half_h = self.ortho_zoom_front;
                let half_w = self.ortho_zoom_front * aspect;
                let world_x = self.ortho_pos_front.x + ndc_x * half_w;
                let world_z = self.ortho_pos_front.y + ndc_y * half_h;
                Some((world_x, world_z))
            }
            ViewportType::Side => {
                let half_h = self.ortho_zoom_side;
                let half_w = self.ortho_zoom_side * aspect;
                let world_y = self.ortho_pos_side.x + ndc_x * half_w;
                let world_z = self.ortho_pos_side.y + ndc_y * half_h;
                Some((world_y, world_z))
            }
        }
    }


    pub fn pan_perspective(&mut self, delta_right: f32, delta_up: f32) {
        let rgt = self.right();
        let up = self.up();
        self.position += rgt * delta_right + up * delta_up;
    }

    pub fn rotate(&mut self, delta_x: f32, delta_y: f32, sensitivity: f32) {
        self.yaw += delta_x * sensitivity;
        self.pitch -= delta_y * sensitivity;

        // Clamp pitch to avoid gimbal flip
        let max_pitch = 89.0f32.to_radians();
        self.pitch = self.pitch.clamp(-max_pitch, max_pitch);
    }

    pub fn update_movement(
        &mut self,
        forward: f32,
        right: f32,
        up: f32,
        dt: Duration,
        fast: bool,
    ) {
        let speed_multiplier = if fast { 3.0 } else { 1.0 };
        let dist = self.speed * speed_multiplier * dt.as_secs_f32();

        let fwd = self.forward();
        let rgt = self.right();
        let up_vec = Vec3::Z;

        self.position += (fwd * forward + rgt * right + up_vec * up) * dist;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CameraBookmark {
    pub name: String,
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub ortho_zoom: f32,
}

impl CameraBookmark {
    pub fn new(slot: usize, camera: &Camera) -> Self {
        Self {
            name: format!("Bookmark {}", slot),
            position: camera.position,
            yaw: camera.yaw,
            pitch: camera.pitch,
            ortho_zoom: camera.ortho_zoom,
        }
    }
}

