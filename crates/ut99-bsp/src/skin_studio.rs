//! UT99 Skin & Character Authoring Studio Engine
//!
//! Provides data models, texture slot routing, multi-team color matrix (Red, Blue, Green, Gold),
//! automated 64x64 TalkTexture cropping, 256-color palette quantization, mipmap chain synthesis,
//! and UT99 `.int` registration manifest emission.
//!
//! Based on forensic analysis of `Unreal Tournament Skin Maker` (UTSM), Gildor's Skeletal Mesh tools,
//! and official Unreal Tournament 99 character packages (`SoldierSkins.int`, `CommandoSkins.int`, `SGirlSkins.int`).

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use glam::Vec3;
use ut99_format::package::PackageFileSummary;
use ut99_format::utx::{UtxPackage, UtxTexture};
use ut99_format::Package;

/// Supported team color channels in Unreal Tournament 99.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SkinTeamColor {
    #[default]
    None,      // Default / DeathMatch (DM)
    Red,       // Team 0 (Red)
    Blue,      // Team 1 (Blue)
    Green,     // Team 2 (Green)
    Gold,      // Team 3 (Gold / Yellow)
}

impl SkinTeamColor {
    /// UT99 texture filename suffix for team skins (e.g. "T_0" for Red, "" for DM).
    pub fn suffix(&self) -> &'static str {
        match self {
            SkinTeamColor::None => "",
            SkinTeamColor::Red => "T_0",
            SkinTeamColor::Blue => "T_1",
            SkinTeamColor::Green => "T_2",
            SkinTeamColor::Gold => "T_3",
        }
    }

    pub fn team_index(&self) -> Option<usize> {
        match self {
            SkinTeamColor::None => None,
            SkinTeamColor::Red => Some(0),
            SkinTeamColor::Blue => Some(1),
            SkinTeamColor::Green => Some(2),
            SkinTeamColor::Gold => Some(3),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SkinTeamColor::None => "Default (DM)",
            SkinTeamColor::Red => "Red Team (0)",
            SkinTeamColor::Blue => "Blue Team (1)",
            SkinTeamColor::Green => "Green Team (2)",
            SkinTeamColor::Gold => "Gold Team (3)",
        }
    }

    pub fn badge_rgb(&self) -> [u8; 3] {
        match self {
            SkinTeamColor::None => [180, 180, 180],
            SkinTeamColor::Red => [230, 45, 45],
            SkinTeamColor::Blue => [45, 120, 240],
            SkinTeamColor::Green => [45, 200, 60],
            SkinTeamColor::Gold => [240, 200, 30],
        }
    }
}

/// Functional body sections of an Unreal Tournament 99 player model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkinSectionKind {
    Chest,        // Slot 1: Torso / Chest (supports 4 team variations)
    Legs,         // Slot 2: Legs / Lower body (supports 4 team variations)
    Arms,         // Slot 3: Arms / Shoulders (usually team-invariant)
    HeadAndFace,  // Slot 4: Head & Face mesh texture (named faces)
    TalkTexture,  // Slot 5: 64x64 HUD portrait icon
    Custom(u8),   // Custom extra slot for non-standard models
}

impl SkinSectionKind {
    pub fn slot_index(&self) -> u8 {
        match self {
            SkinSectionKind::Chest => 1,
            SkinSectionKind::Legs => 2,
            SkinSectionKind::Arms => 3,
            SkinSectionKind::HeadAndFace => 4,
            SkinSectionKind::TalkTexture => 5,
            SkinSectionKind::Custom(idx) => *idx,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SkinSectionKind::Chest => "Torso / Chest",
            SkinSectionKind::Legs => "Legs / Lower Body",
            SkinSectionKind::Arms => "Arms / Shoulders",
            SkinSectionKind::HeadAndFace => "Head & Face",
            SkinSectionKind::TalkTexture => "Talk Texture (HUD 64x64)",
            SkinSectionKind::Custom(_) => "Custom Section",
        }
    }
}

/// Camera preview presets for evaluating skins on 3D meshes.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinCameraPreset {
    pub name: &'static str,
    pub camera_target: Vec3,
    pub distance: f32,
    pub yaw_deg: f32,
    pub pitch_deg: f32,
}

/// Definition of an authentic player model section (from `.skme` specifications).
#[derive(Debug, Clone, PartialEq)]
pub struct SkinModelSectionDef {
    pub slot: u8,
    pub name: &'static str,
    pub kind: SkinSectionKind,
    pub requires_team: bool,
    pub requires_face: bool,
    pub is_talk_texture: bool,
}

/// Architectural metadata definition for player and bot models in UT99.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinModelDefinition {
    pub display_name: &'static str,
    pub model_prefix: &'static str,
    pub mesh_package: &'static str,
    pub default_scale: Vec3,
    pub default_offset: Vec3,
    pub default_rotation_deg: Vec3, // pitch, roll, yaw
    pub sections: &'static [SkinModelSectionDef],
    pub camera_presets: &'static [SkinCameraPreset],
    pub default_animations: &'static [&'static str],
}

// -------------------------------------------------------------------------------------------------
// Standard UT99 Model Catalog (10 Official + 4 Community Classics)
// -------------------------------------------------------------------------------------------------

static SOLDIER_SECTIONS: &[SkinModelSectionDef] = &[
    SkinModelSectionDef { slot: 1, name: "Chest", kind: SkinSectionKind::Chest, requires_team: true, requires_face: false, is_talk_texture: false },
    SkinModelSectionDef { slot: 2, name: "Legs", kind: SkinSectionKind::Legs, requires_team: true, requires_face: false, is_talk_texture: false },
    SkinModelSectionDef { slot: 3, name: "Arms", kind: SkinSectionKind::Arms, requires_team: false, requires_face: false, is_talk_texture: false },
    SkinModelSectionDef { slot: 4, name: "Head & Hands", kind: SkinSectionKind::HeadAndFace, requires_team: false, requires_face: true, is_talk_texture: false },
    SkinModelSectionDef { slot: 5, name: "Talk Texture", kind: SkinSectionKind::TalkTexture, requires_team: false, requires_face: true, is_talk_texture: true },
];

static COMMANDO_SECTIONS: &[SkinModelSectionDef] = &[
    SkinModelSectionDef { slot: 1, name: "Chest", kind: SkinSectionKind::Chest, requires_team: true, requires_face: false, is_talk_texture: false },
    SkinModelSectionDef { slot: 2, name: "Legs", kind: SkinSectionKind::Legs, requires_team: true, requires_face: false, is_talk_texture: false },
    SkinModelSectionDef { slot: 3, name: "Arms", kind: SkinSectionKind::Arms, requires_team: false, requires_face: false, is_talk_texture: false },
    SkinModelSectionDef { slot: 4, name: "Head", kind: SkinSectionKind::HeadAndFace, requires_team: false, requires_face: true, is_talk_texture: false },
    SkinModelSectionDef { slot: 5, name: "Talk Texture", kind: SkinSectionKind::TalkTexture, requires_team: false, requires_face: true, is_talk_texture: true },
];

static TSKM_SECTIONS: &[SkinModelSectionDef] = &[
    SkinModelSectionDef { slot: 1, name: "Head Overlay", kind: SkinSectionKind::Custom(1), requires_team: false, requires_face: true, is_talk_texture: false },
    SkinModelSectionDef { slot: 2, name: "Head", kind: SkinSectionKind::HeadAndFace, requires_team: false, requires_face: true, is_talk_texture: false },
    SkinModelSectionDef { slot: 3, name: "Arms & Legs", kind: SkinSectionKind::Legs, requires_team: true, requires_face: false, is_talk_texture: false },
    SkinModelSectionDef { slot: 4, name: "Chest", kind: SkinSectionKind::Chest, requires_team: true, requires_face: false, is_talk_texture: false },
    SkinModelSectionDef { slot: 5, name: "Talk Texture", kind: SkinSectionKind::TalkTexture, requires_team: false, requires_face: true, is_talk_texture: true },
];

static DEFAULT_CAMERA_PRESETS: &[SkinCameraPreset] = &[
    SkinCameraPreset { name: "Full Body", camera_target: Vec3::new(0.0, 0.0, 0.0), distance: 160.0, yaw_deg: 0.0, pitch_deg: 5.0 },
    SkinCameraPreset { name: "Torso & Arms", camera_target: Vec3::new(0.0, 0.0, 10.0), distance: 95.0, yaw_deg: 0.0, pitch_deg: 2.0 },
    SkinCameraPreset { name: "Head & Face", camera_target: Vec3::new(0.0, 0.0, 32.0), distance: 50.0, yaw_deg: -10.0, pitch_deg: -4.0 },
    SkinCameraPreset { name: "Back / Rear View", camera_target: Vec3::new(0.0, 0.0, 5.0), distance: 130.0, yaw_deg: 180.0, pitch_deg: 5.0 },
    SkinCameraPreset { name: "Legs & Boots", camera_target: Vec3::new(0.0, 0.0, -22.0), distance: 80.0, yaw_deg: 15.0, pitch_deg: 12.0 },
];

static DEFAULT_ANIMS: &[&str] = &["Breath", "Walk", "Run", "Victory1", "Taunt", "Look"];

/// Returns the complete registry of built-in models calibrated from `UTSkinMaker` `.skme` database.
pub fn get_builtin_skin_models() -> Vec<SkinModelDefinition> {
    vec![
        SkinModelDefinition {
            display_name: "Male Soldier",
            model_prefix: "Soldier",
            mesh_package: "Botpack.Soldier",
            default_scale: Vec3::splat(1.13),
            default_offset: Vec3::new(16.25, -10.937, 9.875),
            default_rotation_deg: Vec3::ZERO,
            sections: SOLDIER_SECTIONS,
            camera_presets: DEFAULT_CAMERA_PRESETS,
            default_animations: DEFAULT_ANIMS,
        },
        SkinModelDefinition {
            display_name: "Female Soldier",
            model_prefix: "SGirl",
            mesh_package: "Botpack.SGirl",
            default_scale: Vec3::splat(1.19),
            default_offset: Vec3::new(-4.0, 18.625, -15.25),
            default_rotation_deg: Vec3::ZERO,
            sections: SOLDIER_SECTIONS,
            camera_presets: DEFAULT_CAMERA_PRESETS,
            default_animations: DEFAULT_ANIMS,
        },
        SkinModelDefinition {
            display_name: "Male Commando",
            model_prefix: "Commando",
            mesh_package: "Botpack.Commando",
            default_scale: Vec3::splat(1.15),
            default_offset: Vec3::new(18.938, -10.375, 10.50),
            default_rotation_deg: Vec3::ZERO,
            sections: COMMANDO_SECTIONS,
            camera_presets: DEFAULT_CAMERA_PRESETS,
            default_animations: DEFAULT_ANIMS,
        },
        SkinModelDefinition {
            display_name: "Female Commando",
            model_prefix: "FCommando",
            mesh_package: "Botpack.FCommando",
            default_scale: Vec3::splat(1.11),
            default_offset: Vec3::new(-1.0, 16.25, -11.625),
            default_rotation_deg: Vec3::ZERO,
            sections: COMMANDO_SECTIONS,
            camera_presets: DEFAULT_CAMERA_PRESETS,
            default_animations: DEFAULT_ANIMS,
        },
        SkinModelDefinition {
            display_name: "Boss (Xan Mark I)",
            model_prefix: "Boss",
            mesh_package: "Botpack.Boss",
            default_scale: Vec3::splat(1.08),
            default_offset: Vec3::new(-20.437, -12.50, 0.125),
            default_rotation_deg: Vec3::ZERO,
            sections: SOLDIER_SECTIONS,
            camera_presets: DEFAULT_CAMERA_PRESETS,
            default_animations: DEFAULT_ANIMS,
        },
        SkinModelDefinition {
            display_name: "Xan Mark II",
            model_prefix: "NewXan",
            mesh_package: "SkeletalChars.NewXan",
            default_scale: Vec3::splat(0.74),
            default_offset: Vec3::new(0.0, -99.76, 8.156),
            default_rotation_deg: Vec3::new(90.0, 0.0, 0.0),
            sections: SOLDIER_SECTIONS,
            camera_presets: DEFAULT_CAMERA_PRESETS,
            default_animations: DEFAULT_ANIMS,
        },
        SkinModelDefinition {
            display_name: "WarBoss",
            model_prefix: "WarMachineBoss",
            mesh_package: "SkeletalChars.WarMachineBoss",
            default_scale: Vec3::splat(0.71),
            default_offset: Vec3::new(0.0, -99.105, -0.443),
            default_rotation_deg: Vec3::new(90.0, 0.0, 0.0),
            sections: SOLDIER_SECTIONS,
            camera_presets: DEFAULT_CAMERA_PRESETS,
            default_animations: DEFAULT_ANIMS,
        },
        SkinModelDefinition {
            display_name: "Skaarj Hybrid",
            model_prefix: "TSkM",
            mesh_package: "EpicCustomModels.TSkM",
            default_scale: Vec3::splat(0.98),
            default_offset: Vec3::new(-2.375, 8.0, 1.375),
            default_rotation_deg: Vec3::new(90.0, 0.0, -90.0),
            sections: TSKM_SECTIONS,
            camera_presets: DEFAULT_CAMERA_PRESETS,
            default_animations: DEFAULT_ANIMS,
        },
        SkinModelDefinition {
            display_name: "Nali",
            model_prefix: "tnalimesh",
            mesh_package: "EpicCustomModels.tnalimesh",
            default_scale: Vec3::splat(0.95),
            default_offset: Vec3::new(0.063, 0.625, 0.625),
            default_rotation_deg: Vec3::ZERO,
            sections: SOLDIER_SECTIONS,
            camera_presets: DEFAULT_CAMERA_PRESETS,
            default_animations: DEFAULT_ANIMS,
        },
        SkinModelDefinition {
            display_name: "Nali Cow",
            model_prefix: "TCowMesh",
            mesh_package: "EpicCustomModels.TCowMesh",
            default_scale: Vec3::splat(1.10),
            default_offset: Vec3::new(-6.438, -1.375, -0.125),
            default_rotation_deg: Vec3::new(90.0, 0.0, -90.0),
            sections: SOLDIER_SECTIONS,
            camera_presets: DEFAULT_CAMERA_PRESETS,
            default_animations: DEFAULT_ANIMS,
        },
    ]
}

// -------------------------------------------------------------------------------------------------
// Texture Representation & Mipmap Data Structures
// -------------------------------------------------------------------------------------------------

/// Single mipmap layer in downsampled texture pyramid.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinMipLevel {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub indices: Vec<u8>,
}

/// An individual skin texture with raw RGBA8 data, optional 256-color palette, and mipmaps.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinTexture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub palette: Option<[[u8; 3]; 256]>,
    pub indices: Option<Vec<u8>>,
    pub is_masked: bool,
    pub is_translucent: bool,
    pub mipmaps: Vec<SkinMipLevel>,
}

impl SkinTexture {
    /// Creates a new texture from raw RGBA8 pixel buffer.
    pub fn from_rgba(name: String, width: u32, height: u32, rgba: Vec<u8>) -> Self {
        let mut tex = Self {
            name,
            width,
            height,
            rgba,
            palette: None,
            indices: None,
            is_masked: false,
            is_translucent: false,
            mipmaps: Vec::new(),
        };
        tex.recompute_palettization_and_mipmaps();
        tex
    }

    /// Creates a solid color test texture (e.g. 256x256).
    pub fn create_solid(name: String, width: u32, height: u32, color: [u8; 4]) -> Self {
        let count = (width * height) as usize;
        let mut rgba = Vec::with_capacity(count * 4);
        for _ in 0..count {
            rgba.extend_from_slice(&color);
        }
        Self::from_rgba(name, width, height, rgba)
    }

    /// Recomputes 256-color palette quantization and mipmap pyramid.
    pub fn recompute_palettization_and_mipmaps(&mut self) {
        let (pal, idx) = quantize_to_256_colors(&self.rgba, self.is_masked);
        self.palette = Some(pal);
        self.indices = Some(idx);
        self.mipmaps = generate_mipmap_pyramid(self.width, self.height, &self.rgba, &pal, self.is_masked);
    }
}

/// Named character face with face texture and accompanying 64x64 TalkTexture.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinFaceEntry {
    pub face_name: String,
    pub display_name: String,
    pub face_texture: SkinTexture,
    pub talk_texture: SkinTexture,
    pub team_talk_textures: [Option<SkinTexture>; 4],
}

impl SkinFaceEntry {
    pub fn new(face_name: String, display_name: String, face_texture: SkinTexture) -> Self {
        let talk = generate_talk_texture_from_face(&face_texture);
        Self {
            face_name,
            display_name,
            face_texture,
            talk_texture: talk,
            team_talk_textures: [None, None, None, None],
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Skin Project & Authoring Session
// -------------------------------------------------------------------------------------------------

/// Root authoring state for a UT99 player skin project.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinStudioProject {
    pub package_name: String,        // e.g. "SoldierSkins_Army"
    pub skin_id: String,             // e.g. "army"
    pub skin_description: String,    // e.g. "Army Soldier"
    pub author: String,              // e.g. "LevelDesigner"
    pub selected_model_index: usize,
    pub active_team: SkinTeamColor,
    pub active_face_index: usize,
    pub active_camera_preset_index: usize,
    pub active_animation: String,

    // Section 1: Torso / Chest (Slot 1)
    pub chest_base: Option<SkinTexture>,
    pub chest_teams: [Option<SkinTexture>; 4], // Red, Blue, Green, Gold

    // Section 2: Legs / Lower Body (Slot 2)
    pub legs_base: Option<SkinTexture>,
    pub legs_teams: [Option<SkinTexture>; 4],  // Red, Blue, Green, Gold

    // Section 3: Arms / Shoulders (Slot 3)
    pub arms_base: Option<SkinTexture>,

    // Section 4 & 5: Faces and TalkTextures (Slot 4 & 5)
    pub faces: Vec<SkinFaceEntry>,

    // Options
    pub use_shared_palette: bool,
    pub single_mipmap_only: bool,
}

impl Default for SkinStudioProject {
    fn default() -> Self {
        let mut proj = Self {
            package_name: "SoldierSkins_Custom".to_string(),
            skin_id: "cust".to_string(),
            skin_description: "Custom Soldier".to_string(),
            author: "Author".to_string(),
            selected_model_index: 0,
            active_team: SkinTeamColor::None,
            active_face_index: 0,
            active_camera_preset_index: 0,
            active_animation: "Breath".to_string(),
            chest_base: None,
            chest_teams: [None, None, None, None],
            legs_base: None,
            legs_teams: [None, None, None, None],
            arms_base: None,
            faces: Vec::new(),
            use_shared_palette: true,
            single_mipmap_only: false,
        };

        // Populate starter face
        let face_tex = SkinTexture::create_solid("Face_Base".to_string(), 256, 256, [210, 160, 130, 255]);
        proj.faces.push(SkinFaceEntry::new("Marcus".to_string(), "Marcus".to_string(), face_tex));

        // Create default base textures
        proj.chest_base = Some(SkinTexture::create_solid("Chest_Base".to_string(), 256, 256, [60, 80, 60, 255]));
        proj.legs_base = Some(SkinTexture::create_solid("Legs_Base".to_string(), 256, 256, [50, 50, 55, 255]));
        proj.arms_base = Some(SkinTexture::create_solid("Arms_Base".to_string(), 256, 256, [70, 70, 65, 255]));

        // Create starter team textures
        proj.chest_teams[0] = Some(SkinTexture::create_solid("Chest_Red".to_string(), 256, 256, [180, 30, 30, 255]));
        proj.chest_teams[1] = Some(SkinTexture::create_solid("Chest_Blue".to_string(), 256, 256, [30, 70, 190, 255]));
        proj.chest_teams[2] = Some(SkinTexture::create_solid("Chest_Green".to_string(), 256, 256, [30, 160, 40, 255]));
        proj.chest_teams[3] = Some(SkinTexture::create_solid("Chest_Gold".to_string(), 256, 256, [200, 170, 20, 255]));

        proj.legs_teams[0] = Some(SkinTexture::create_solid("Legs_Red".to_string(), 256, 256, [140, 25, 25, 255]));
        proj.legs_teams[1] = Some(SkinTexture::create_solid("Legs_Blue".to_string(), 256, 256, [25, 55, 150, 255]));
        proj.legs_teams[2] = Some(SkinTexture::create_solid("Legs_Green".to_string(), 256, 256, [25, 120, 35, 255]));
        proj.legs_teams[3] = Some(SkinTexture::create_solid("Legs_Gold".to_string(), 256, 256, [160, 135, 15, 255]));

        proj
    }
}

// -------------------------------------------------------------------------------------------------
// Algorithmic Pipelines: TalkTexture Extraction, Quantization & Mipmaps
// -------------------------------------------------------------------------------------------------

/// Automatically extracts and frames an authentic 64x64 HUD TalkTexture from a 256x256 face texture.
pub fn generate_talk_texture_from_face(face: &SkinTexture) -> SkinTexture {
    let src_w = face.width as usize;
    let src_h = face.height as usize;
    let out_w = 64usize;
    let out_h = 64usize;

    // Face UV crop center: typically top-left 0.1 to 0.7 region contains the frontal face in UT99 UV maps
    let crop_x0 = (src_w as f32 * 0.15) as usize;
    let crop_y0 = (src_h as f32 * 0.10) as usize;
    let crop_size = (src_w as f32 * 0.60) as usize;

    let mut out_rgba = Vec::with_capacity(out_w * out_h * 4);

    for oy in 0..out_h {
        let v = oy as f32 / out_h as f32;
        let sy = crop_y0 + (v * crop_size as f32) as usize;
        let clamped_sy = sy.min(src_h - 1);

        for ox in 0..out_w {
            let u = ox as f32 / out_w as f32;
            let sx = crop_x0 + (u * crop_size as f32) as usize;
            let clamped_sx = sx.min(src_w - 1);

            let src_idx = (clamped_sy * src_w + clamped_sx) * 4;
            let r = face.rgba.get(src_idx).copied().unwrap_or(0);
            let g = face.rgba.get(src_idx + 1).copied().unwrap_or(0);
            let b = face.rgba.get(src_idx + 2).copied().unwrap_or(0);
            let a = face.rgba.get(src_idx + 3).copied().unwrap_or(255);

            // Subtle contrast enhancement for HUD visibility
            let rf = ((r as f32 - 128.0) * 1.15 + 128.0).clamp(0.0, 255.0) as u8;
            let gf = ((g as f32 - 128.0) * 1.15 + 128.0).clamp(0.0, 255.0) as u8;
            let bf = ((b as f32 - 128.0) * 1.15 + 128.0).clamp(0.0, 255.0) as u8;

            out_rgba.extend_from_slice(&[rf, gf, bf, a]);
        }
    }

    SkinTexture::from_rgba(format!("{}_Talk", face.name), 64, 64, out_rgba)
}

/// Quantizes an arbitrary RGBA buffer into a deterministic 256-color palette + index buffer.
/// If `is_masked` is true, index 0 is reserved as transparent black `(0, 0, 0)`.
pub fn quantize_to_256_colors(rgba: &[u8], is_masked: bool) -> ([[u8; 3]; 256], Vec<u8>) {
    let mut palette = [[0u8; 3]; 256];
    let total_pixels = rgba.len() / 4;

    let start_idx = if is_masked {
        palette[0] = [0, 0, 0]; // Masked transparent key
        1
    } else {
        0
    };

    // Fast and robust uniform sampling + median quantization seed
    let step = (total_pixels / (256 - start_idx)).max(1);
    let mut pal_idx = start_idx;
    for i in 0..total_pixels {
        if i % step == 0 && pal_idx < 256 {
            let offset = i * 4;
            palette[pal_idx] = [rgba[offset], rgba[offset + 1], rgba[offset + 2]];
            pal_idx += 1;
        }
    }
    // Fill any remainder
    while pal_idx < 256 {
        palette[pal_idx] = [((pal_idx * 37) % 256) as u8, ((pal_idx * 79) % 256) as u8, ((pal_idx * 131) % 256) as u8];
        pal_idx += 1;
    }

    // Map each pixel to nearest palette index via Euclidean color distance
    let mut indices = Vec::with_capacity(total_pixels);
    for i in 0..total_pixels {
        let offset = i * 4;
        let r = rgba[offset] as i32;
        let g = rgba[offset + 1] as i32;
        let b = rgba[offset + 2] as i32;
        let a = rgba[offset + 3];

        if is_masked && a < 128 {
            indices.push(0);
            continue;
        }

        let mut best_dist = i32::MAX;
        let mut best_pal = start_idx as u8;

        for p in start_idx..256 {
            let pr = palette[p][0] as i32;
            let pg = palette[p][1] as i32;
            let pb = palette[p][2] as i32;
            let dist = (r - pr) * (r - pr) + (g - pg) * (g - pg) + (b - pb) * (b - pb);
            if dist < best_dist {
                best_dist = dist;
                best_pal = p as u8;
                if dist == 0 {
                    break;
                }
            }
        }
        indices.push(best_pal);
    }

    (palette, indices)
}

/// Generates a power-of-two mipmap pyramid down to 1x1.
pub fn generate_mipmap_pyramid(
    base_w: u32,
    base_h: u32,
    base_rgba: &[u8],
    palette: &[[u8; 3]; 256],
    is_masked: bool,
) -> Vec<SkinMipLevel> {
    let mut levels = Vec::new();
    let mut cur_w = base_w;
    let mut cur_h = base_h;
    let mut cur_rgba = base_rgba.to_vec();

    while cur_w > 1 || cur_h > 1 {
        let next_w = (cur_w / 2).max(1);
        let next_h = (cur_h / 2).max(1);
        let mut next_rgba = Vec::with_capacity((next_w * next_h * 4) as usize);

        for ny in 0..next_h {
            for nx in 0..next_w {
                let sx0 = (nx * 2).min(cur_w - 1);
                let sx1 = (nx * 2 + 1).min(cur_w - 1);
                let sy0 = (ny * 2).min(cur_h - 1);
                let sy1 = (ny * 2 + 1).min(cur_h - 1);

                let p00 = ((sy0 * cur_w + sx0) * 4) as usize;
                let p10 = ((sy0 * cur_w + sx1) * 4) as usize;
                let p01 = ((sy1 * cur_w + sx0) * 4) as usize;
                let p11 = ((sy1 * cur_w + sx1) * 4) as usize;

                let r = (cur_rgba[p00] as u32 + cur_rgba[p10] as u32 + cur_rgba[p01] as u32 + cur_rgba[p11] as u32) / 4;
                let g = (cur_rgba[p00 + 1] as u32 + cur_rgba[p10 + 1] as u32 + cur_rgba[p01 + 1] as u32 + cur_rgba[p11 + 1] as u32) / 4;
                let b = (cur_rgba[p00 + 2] as u32 + cur_rgba[p10 + 2] as u32 + cur_rgba[p01 + 2] as u32 + cur_rgba[p11 + 2] as u32) / 4;
                let a = (cur_rgba[p00 + 3] as u32 + cur_rgba[p10 + 3] as u32 + cur_rgba[p01 + 3] as u32 + cur_rgba[p11 + 3] as u32) / 4;

                next_rgba.extend_from_slice(&[r as u8, g as u8, b as u8, a as u8]);
            }
        }

        // Map downsampled pixels to palette
        let mut indices = Vec::with_capacity((next_w * next_h) as usize);
        let start_p = if is_masked { 1 } else { 0 };

        for i in 0..(next_w * next_h) as usize {
            let offset = i * 4;
            let r = next_rgba[offset] as i32;
            let g = next_rgba[offset + 1] as i32;
            let b = next_rgba[offset + 2] as i32;
            let a = next_rgba[offset + 3];

            if is_masked && a < 128 {
                indices.push(0);
                continue;
            }

            let mut best_dist = i32::MAX;
            let mut best_pal = start_p as u8;

            for p in start_p..256 {
                let pr = palette[p][0] as i32;
                let pg = palette[p][1] as i32;
                let pb = palette[p][2] as i32;
                let dist = (r - pr) * (r - pr) + (g - pg) * (g - pg) + (b - pb) * (b - pb);
                if dist < best_dist {
                    best_dist = dist;
                    best_pal = p as u8;
                }
            }
            indices.push(best_pal);
        }

        levels.push(SkinMipLevel {
            width: next_w,
            height: next_h,
            rgba: next_rgba.clone(),
            indices,
        });

        cur_w = next_w;
        cur_h = next_h;
        cur_rgba = next_rgba;
    }

    levels
}

// -------------------------------------------------------------------------------------------------
// UT99 `.int` Registration Manifest Emission
// -------------------------------------------------------------------------------------------------

/// Generates the authentic UT99 `.int` file registration content for the skin package.
pub fn generate_int_registration_manifest(project: &SkinStudioProject) -> String {
    let mut out = String::new();
    out.push_str("[public]\n");

    let pkg = &project.package_name;
    let sid = &project.skin_id;

    // Slot 1: Chest (Base has Description, registering the skin!)
    out.push_str(&format!(
        "Object=(Name={pkg}.{sid}1,Class=Texture,Description=\"{}\")\n",
        project.skin_description
    ));

    // Slot 2: Legs (Base)
    out.push_str(&format!("Object=(Name={pkg}.{sid}2,Class=Texture)\n"));

    // Slot 3: Arms (Base)
    out.push_str(&format!("Object=(Name={pkg}.{sid}3,Class=Texture)\n"));

    // Slot 4: Default Face
    out.push_str(&format!("Object=(Name={pkg}.{sid}4,Class=Texture)\n"));

    // Named Faces & Talk Textures
    for face in &project.faces {
        out.push_str(&format!(
            "Object=(Name={pkg}.{sid}4{},Class=Texture,Description=\"{}\")\n",
            face.face_name, face.display_name
        ));
        out.push_str(&format!(
            "Object=(Name={pkg}.{sid}5{},Class=Texture)\n",
            face.face_name
        ));
    }

    // Team variations for Slot 1 (Chest) & Slot 2 (Legs)
    let team_suffixes = ["1T_0", "1T_1", "1T_2", "1T_3", "2T_0", "2T_1", "2T_2", "2T_3"];
    for suff in team_suffixes {
        out.push_str(&format!("Object=(Name={pkg}.{sid}{suff},Class=Texture)\n"));
    }

    out
}

/// Detailed result of a skin package compilation and export operation.
#[derive(Debug, Clone, Default)]
pub struct SkinExportResult {
    pub package_path: PathBuf,
    pub manifest_path: PathBuf,
    pub textures_compiled: usize,
    pub package_bytes: usize,
    pub manifest_bytes: usize,
    pub errors: Vec<String>,
}

/// Compiles a complete `SkinStudioProject` into a binary `.utx` Unreal package and matching `.int` registration manifest on disk.
pub fn export_skin_project(
    project: &SkinStudioProject,
    output_dir: &Path,
    subfolder_structure: bool,
) -> Result<SkinExportResult, String> {
    if project.package_name.trim().is_empty() {
        return Err("Skin package name cannot be empty.".to_string());
    }
    if project.skin_id.trim().is_empty() {
        return Err("Skin ID / prefix cannot be empty.".to_string());
    }

    let mut utx_textures = Vec::new();
    let sid = &project.skin_id;

    // Helper closure to push a texture
    let mut add_tex = |name: String, skin_tex: &SkinTexture| {
        utx_textures.push(UtxTexture {
            name,
            group: String::new(),
            width: skin_tex.width as u32,
            height: skin_tex.height as u32,
            format: 0, // P8 paletted
            masked: false,
            rgba_pixels: skin_tex.rgba.clone(),
        });
    };

    // 1. Slot 1: Chest
    if let Some(ref chest_base) = project.chest_base {
        add_tex(format!("{sid}1"), chest_base);
    }
    for (t_idx, suff) in ["1T_0", "1T_1", "1T_2", "1T_3"].iter().enumerate() {
        if let Some(ref chest_team) = project.chest_teams[t_idx] {
            add_tex(format!("{sid}{suff}"), chest_team);
        }
    }

    // 2. Slot 2: Legs
    if let Some(ref legs_base) = project.legs_base {
        add_tex(format!("{sid}2"), legs_base);
    }
    for (t_idx, suff) in ["2T_0", "2T_1", "2T_2", "2T_3"].iter().enumerate() {
        if let Some(ref legs_team) = project.legs_teams[t_idx] {
            add_tex(format!("{sid}{suff}"), legs_team);
        }
    }

    // 3. Slot 3: Arms
    if let Some(ref arms_base) = project.arms_base {
        add_tex(format!("{sid}3"), arms_base);
    }

    // 4. Slot 4: Default Face
    if let Some(first_face) = project.faces.first() {
        add_tex(format!("{sid}4"), &first_face.face_texture);
    }

    // 5. Named Faces & TalkTextures
    for face in &project.faces {
        add_tex(format!("{sid}4{}", face.face_name), &face.face_texture);
        add_tex(format!("{sid}5{}", face.face_name), &face.talk_texture);
    }

    if utx_textures.is_empty() {
        return Err("Project contains no textures to compile.".to_string());
    }

    let textures_count = utx_textures.len();

    // Create Package & write textures
    let mut pkg = Package::default();
    pkg.summary = PackageFileSummary::default();
    UtxPackage::write_textures_into_package(&utx_textures, &mut pkg, 0)
        .map_err(|e| format!("Failed writing textures into package: {e}"))?;

    let package_bytes = pkg.to_bytes()
        .map_err(|e| format!("Failed serializing package bytes: {e}"))?;

    let manifest_text = generate_int_registration_manifest(project);

    // Prepare target paths
    let (pkg_dir, int_dir) = if subfolder_structure {
        (output_dir.join("Textures"), output_dir.join("System"))
    } else {
        (output_dir.to_path_buf(), output_dir.to_path_buf())
    };

    fs::create_dir_all(&pkg_dir).map_err(|e| format!("Failed creating directory {}: {e}", pkg_dir.display()))?;
    fs::create_dir_all(&int_dir).map_err(|e| format!("Failed creating directory {}: {e}", int_dir.display()))?;

    let package_path = pkg_dir.join(format!("{}.utx", project.package_name));
    let manifest_path = int_dir.join(format!("{}.int", project.package_name));

    let mut pkg_file = File::create(&package_path)
        .map_err(|e| format!("Failed creating package file {}: {e}", package_path.display()))?;
    pkg_file.write_all(&package_bytes)
        .map_err(|e| format!("Failed writing package file {}: {e}", package_path.display()))?;

    let mut int_file = File::create(&manifest_path)
        .map_err(|e| format!("Failed creating manifest file {}: {e}", manifest_path.display()))?;
    int_file.write_all(manifest_text.as_bytes())
        .map_err(|e| format!("Failed writing manifest file {}: {e}", manifest_path.display()))?;

    Ok(SkinExportResult {
        package_path,
        manifest_path,
        textures_compiled: textures_count,
        package_bytes: package_bytes.len(),
        manifest_bytes: manifest_text.len(),
        errors: Vec::new(),
    })
}

// -------------------------------------------------------------------------------------------------
// Diagnostics & Verification Audit
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkinAuditSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinAuditItem {
    pub severity: SkinAuditSeverity,
    pub title: &'static str,
    pub detail: String,
}

/// Validates a skin project against Unreal Tournament 99 engine rules.
pub fn audit_skin_project(project: &SkinStudioProject) -> Vec<SkinAuditItem> {
    let mut issues = Vec::new();

    if project.package_name.trim().is_empty() {
        issues.push(SkinAuditItem {
            severity: SkinAuditSeverity::Error,
            title: "Empty Package Name",
            detail: "The skin package name (.utx) cannot be blank.".to_string(),
        });
    }

    if project.skin_id.trim().is_empty() {
        issues.push(SkinAuditItem {
            severity: SkinAuditSeverity::Error,
            title: "Empty Skin Prefix",
            detail: "The 3-5 character skin prefix (e.g. 'army', 'sldr') is missing.".to_string(),
        });
    }

    if project.chest_base.is_none() {
        issues.push(SkinAuditItem {
            severity: SkinAuditSeverity::Error,
            title: "Missing Chest Base Texture",
            detail: "Slot 1 (Chest) base texture is required for player registration.".to_string(),
        });
    }

    if project.legs_base.is_none() {
        issues.push(SkinAuditItem {
            severity: SkinAuditSeverity::Error,
            title: "Missing Legs Base Texture",
            detail: "Slot 2 (Legs) base texture is required.".to_string(),
        });
    }

    if project.faces.is_empty() {
        issues.push(SkinAuditItem {
            severity: SkinAuditSeverity::Warning,
            title: "No Named Faces Configured",
            detail: "At least one named face is required for bots and player selection.".to_string(),
        });
    }

    // Check team color completeness
    let red_chest = project.chest_teams[0].is_some();
    let blue_chest = project.chest_teams[1].is_some();
    if red_chest != blue_chest {
        issues.push(SkinAuditItem {
            severity: SkinAuditSeverity::Warning,
            title: "Asymmetric Team Textures",
            detail: "Red and Blue team skins should both be provided for CTF/DOM balance.".to_string(),
        });
    }

    // Check TalkTexture resolutions
    for face in &project.faces {
        if face.talk_texture.width != 64 || face.talk_texture.height != 64 {
            issues.push(SkinAuditItem {
                severity: SkinAuditSeverity::Warning,
                title: "Non-Standard TalkTexture Resolution",
                detail: format!(
                    "Face '{}' TalkTexture has resolution {}x{}, recommended 64x64 for HUD.",
                    face.face_name, face.talk_texture.width, face.talk_texture.height
                ),
            });
        }
    }

    if issues.is_empty() {
        issues.push(SkinAuditItem {
            severity: SkinAuditSeverity::Info,
            title: "Skin Ready for UT99 Export",
            detail: format!(
                "All slots valid. Package '{}.utx' and manifest '{}.int' are fully conforming.",
                project.package_name, project.package_name
            ),
        });
    }

    issues
}

// -------------------------------------------------------------------------------------------------
// Procedural Skin Studio Engine, Archetypes, Camo Generators & Insignia Stencils
// -------------------------------------------------------------------------------------------------

/// 12 High-Impact Tactical & Fantasy Character Archetypes for Unreal Tournament 99.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default)]
pub enum SkinArchetype {
    #[default]
    TacticalSpecOps,   // Military woodland/urban digital camo, ballistic vest, NVG goggles
    LiandriCyberMech,  // Titanium plating, carbon weave, glowing neon circuits, hazard chevrons
    NecrisShadow,      // Pale deathly skin, obsidian leather, crimson occult runes, dark armor
    GladiatorArena,    // Studded leather harness, heavy bronze/steel plates, warpaint, cross-belts
    HazardShockTrooper,// Hazmat rubber canvas, biohazard/radiation emblems, gasmask, caution trim
    SkaarjWarHybrid,   // Reptilian chitin scales, bone armor plates, predatory eye slits, tribal marks
    ArcticJaeger,      // White/slate snow splinter camo, thermal insulated armor, polar goggles
    DesertNomad,       // Sandstorm khaki camo, dust goggles, brass fittings, desert straps
    IronGuardVeteran,  // Scuffed industrial plate, welding blast marks, skull & bones emblem
    NeonSynthwave,     // High-contrast cyber armor, fluorescent magenta/cyan piping, mirrored visor
    ToxicMutant,       // Toxic sludge green flesh, leaking chemical conduits, glowing bio-core
    ClassicArena90s,   // Retro tournament styling, bold saturated armor panels, tournament chevron
}

impl SkinArchetype {
    pub fn all() -> &'static [SkinArchetype] {
        &[
            Self::TacticalSpecOps,
            Self::LiandriCyberMech,
            Self::NecrisShadow,
            Self::GladiatorArena,
            Self::HazardShockTrooper,
            Self::SkaarjWarHybrid,
            Self::ArcticJaeger,
            Self::DesertNomad,
            Self::IronGuardVeteran,
            Self::NeonSynthwave,
            Self::ToxicMutant,
            Self::ClassicArena90s,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::TacticalSpecOps => "🎖 Tactical SpecOps",
            Self::LiandriCyberMech => "🤖 Liandri Cyber-Mech",
            Self::NecrisShadow => "🧛 Necris Shadow Assassin",
            Self::GladiatorArena => "⚔ Gladiator Champion",
            Self::HazardShockTrooper => "☣ Hazard Shock Trooper",
            Self::SkaarjWarHybrid => "🦎 Skaarj War Hybrid",
            Self::ArcticJaeger => "❄ Arctic Jaeger",
            Self::DesertNomad => "🏜 Desert Nomad",
            Self::IronGuardVeteran => "🛡 Iron Guard Veteran",
            Self::NeonSynthwave => "⚡ Neon Synthwave",
            Self::ToxicMutant => "☢ Toxic Sludge Mutant",
            Self::ClassicArena90s => "🏆 Classic '99 Arena",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::TacticalSpecOps => "Ballistic nylon, tactical webbing, digital/flecktarn camo, night-vision optics",
            Self::LiandriCyberMech => "Titanium plating, carbon-fiber weave, glowing neon circuitry, hazard chevrons",
            Self::NecrisShadow => "Pale deathly skin, dark obsidian leather, crimson occult glyphs, vampiric ocular veins",
            Self::GladiatorArena => "Heavy studded leather harness, polished brass/bronze armor, bold facial warpaint",
            Self::HazardShockTrooper => "Heavy rubberized hazmat suit, biohazard sludge insignias, dual gasmask canisters",
            Self::SkaarjWarHybrid => "Reptilian chitin scales, bone armor plating, tribal clan etchings, predatory pupil slits",
            Self::ArcticJaeger => "White/slate snow camouflage, thermal insulated fur collars, polar tinted goggles",
            Self::DesertNomad => "Sandstorm-weathered khaki canvas, dust goggles, brass compass badge",
            Self::IronGuardVeteran => "Scuffed industrial steel plate, welding blast marks, skull-and-crossbones insignia",
            Self::NeonSynthwave => "Vibrant synth-grid armor, fluorescent magenta/cyan neon piping, mirrored visor",
            Self::ToxicMutant => "Mottled green necrotic flesh, leaking chemical conduits, glowing bio-core",
            Self::ClassicArena90s => "Nostalgic 1999 tournament colors, high-contrast armor segments, Liandri badge",
        }
    }
}

/// Procedural Camouflage & Surface Texture Patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default)]
pub enum SkinCamoPattern {
    #[default]
    Solid,
    Woodland,
    DesertDigital,
    Flecktarn,
    Tigerstripe,
    HexCamo,
    UrbanSplinter,
    CarbonWeave,
    HazardStripes,
    BrushedMetal,
    ChitinScales,
}

impl SkinCamoPattern {
    pub fn all() -> &'static [SkinCamoPattern] {
        &[
            Self::Solid,
            Self::Woodland,
            Self::DesertDigital,
            Self::Flecktarn,
            Self::Tigerstripe,
            Self::HexCamo,
            Self::UrbanSplinter,
            Self::CarbonWeave,
            Self::HazardStripes,
            Self::BrushedMetal,
            Self::ChitinScales,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Solid => "Solid Fabric",
            Self::Woodland => "Woodland Camo",
            Self::DesertDigital => "Desert Digital",
            Self::Flecktarn => "Flecktarn Dots",
            Self::Tigerstripe => "Tigerstripe",
            Self::HexCamo => "Hexagonal Honeycomb",
            Self::UrbanSplinter => "Urban Splinter",
            Self::CarbonWeave => "Carbon Fiber Weave",
            Self::HazardStripes => "Hazard Caution Stripes",
            Self::BrushedMetal => "Brushed Industrial Steel",
            Self::ChitinScales => "Chitin Reptile Scales",
        }
    }
}

/// Chest Armor Insignias & Decal Stencils.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default)]
pub enum SkinInsignia {
    #[default]
    None,
    LiandriLogo,
    IronGuardShield,
    SkullAndBones,
    Biohazard,
    Radiation,
    Lightning,
    Crosshair,
    VLogo,
}

impl SkinInsignia {
    pub fn all() -> &'static [SkinInsignia] {
        &[
            Self::None,
            Self::LiandriLogo,
            Self::IronGuardShield,
            Self::SkullAndBones,
            Self::Biohazard,
            Self::Radiation,
            Self::Lightning,
            Self::Crosshair,
            Self::VLogo,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::LiandriLogo => "Liandri Mining Corp",
            Self::IronGuardShield => "Iron Guard Shield",
            Self::SkullAndBones => "Skull & Crossbones",
            Self::Biohazard => "Biohazard Trefoil",
            Self::Radiation => "Nuclear Radiation",
            Self::Lightning => "Lightning Strike",
            Self::Crosshair => "Tactical Crosshair",
            Self::VLogo => "Tournament V-Victory",
        }
    }
}

/// Facial Styling & Headwear Accessories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default)]
pub enum SkinFaceStyle {
    #[default]
    Clean,
    Stubble,
    CombatBeard,
    WarPaint,
    EyeScar,
    CyberneticEye,
    TacticalVisor,
    Gasmask,
    PilotGoggles,
    NecrisVeins,
}

impl SkinFaceStyle {
    pub fn all() -> &'static [SkinFaceStyle] {
        &[
            Self::Clean,
            Self::Stubble,
            Self::CombatBeard,
            Self::WarPaint,
            Self::EyeScar,
            Self::CyberneticEye,
            Self::TacticalVisor,
            Self::Gasmask,
            Self::PilotGoggles,
            Self::NecrisVeins,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Clean => "Clean Shaven",
            Self::Stubble => "5 O'Clock Stubble",
            Self::CombatBeard => "Tactical Beard",
            Self::WarPaint => "Combat Warpaint",
            Self::EyeScar => "Battle Scar",
            Self::CyberneticEye => "Cybernetic Eye Ocular",
            Self::TacticalVisor => "Tactical LED Visor",
            Self::Gasmask => "Toxic Gasmask Filter",
            Self::PilotGoggles => "Aviator Combat Goggles",
            Self::NecrisVeins => "Necris Death Veins",
        }
    }
}

/// Configuration settings for the Procedural Skin Studio Generator Wizard.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProceduralSkinConfig {
    pub seed: u64,
    pub archetype: SkinArchetype,
    pub camo_pattern: SkinCamoPattern,
    pub insignia: SkinInsignia,
    pub face_style: SkinFaceStyle,
    pub weathering: f32,          // 0.0 (pristine) .. 1.0 (gritty/scratched)
    pub armor_coverage: f32,      // 0.0 (light fabric) .. 1.0 (heavy armor plates)
    pub team_trim_intensity: f32, // 0.0 (subtle) .. 1.0 (high saturation team glow)
    pub skin_name: String,
    pub skin_description: String,
    pub author: String,
}

impl Default for ProceduralSkinConfig {
    fn default() -> Self {
        Self {
            seed: 1337,
            archetype: SkinArchetype::TacticalSpecOps,
            camo_pattern: SkinCamoPattern::Woodland,
            insignia: SkinInsignia::LiandriLogo,
            face_style: SkinFaceStyle::Stubble,
            weathering: 0.25,
            armor_coverage: 0.65,
            team_trim_intensity: 0.85,
            skin_name: "spec".to_string(),
            skin_description: "SpecOps Commando".to_string(),
            author: "LevelDesigner".to_string(),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Deterministic SplitMix64 Pseudo-Random Number Generator
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SplitMix64Rng {
    state: u64,
}

impl SplitMix64Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    pub fn next_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) / ((1 << 24) as f32)
    }

    pub fn next_range(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        let span = (max - min + 1) as f32;
        min + ((self.next_f32() * span) as i32).clamp(0, max - min)
    }

    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            None
        } else {
            let idx = (self.next_u64() as usize) % slice.len();
            Some(&slice[idx])
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Procedural Texture Synthesizers
// -------------------------------------------------------------------------------------------------

/// Generates a randomized `ProceduralSkinConfig` seeded with a 64-bit integer.
pub fn randomize_skin_config(seed: u64, forced_archetype: Option<SkinArchetype>) -> ProceduralSkinConfig {
    let mut rng = SplitMix64Rng::new(seed);
    let arch = forced_archetype.unwrap_or_else(|| *rng.choose(SkinArchetype::all()).unwrap());

    let (camo, ins, face, name_stem, desc) = match arch {
        SkinArchetype::TacticalSpecOps => {
            let camo = *rng.choose(&[SkinCamoPattern::Woodland, SkinCamoPattern::UrbanSplinter, SkinCamoPattern::Flecktarn]).unwrap();
            let ins = *rng.choose(&[SkinInsignia::Crosshair, SkinInsignia::IronGuardShield]).unwrap();
            let face = *rng.choose(&[SkinFaceStyle::Stubble, SkinFaceStyle::CombatBeard, SkinFaceStyle::TacticalVisor]).unwrap();
            (camo, ins, face, "spec", "Tactical SpecOps Operative")
        }
        SkinArchetype::LiandriCyberMech => {
            let camo = *rng.choose(&[SkinCamoPattern::CarbonWeave, SkinCamoPattern::BrushedMetal, SkinCamoPattern::HexCamo]).unwrap();
            let ins = SkinInsignia::LiandriLogo;
            let face = *rng.choose(&[SkinFaceStyle::CyberneticEye, SkinFaceStyle::TacticalVisor]).unwrap();
            (camo, ins, face, "mech", "Liandri Cyber-Combatant")
        }
        SkinArchetype::NecrisShadow => {
            let camo = *rng.choose(&[SkinCamoPattern::Solid, SkinCamoPattern::CarbonWeave]).unwrap();
            let ins = SkinInsignia::SkullAndBones;
            let face = *rng.choose(&[SkinFaceStyle::NecrisVeins, SkinFaceStyle::EyeScar]).unwrap();
            (camo, ins, face, "necr", "Necris Shadow Assassin")
        }
        SkinArchetype::GladiatorArena => {
            let camo = *rng.choose(&[SkinCamoPattern::Solid, SkinCamoPattern::ChitinScales]).unwrap();
            let ins = SkinInsignia::VLogo;
            let face = *rng.choose(&[SkinFaceStyle::WarPaint, SkinFaceStyle::EyeScar, SkinFaceStyle::CombatBeard]).unwrap();
            (camo, ins, face, "glad", "Gladiator Arena Champion")
        }
        SkinArchetype::HazardShockTrooper => {
            let camo = *rng.choose(&[SkinCamoPattern::HazardStripes, SkinCamoPattern::HexCamo, SkinCamoPattern::Solid]).unwrap();
            let ins = *rng.choose(&[SkinInsignia::Biohazard, SkinInsignia::Radiation]).unwrap();
            let face = SkinFaceStyle::Gasmask;
            (camo, ins, face, "hazd", "Hazard Shock Trooper")
        }
        SkinArchetype::SkaarjWarHybrid => {
            let camo = SkinCamoPattern::ChitinScales;
            let ins = SkinInsignia::SkullAndBones;
            let face = SkinFaceStyle::WarPaint;
            (camo, ins, face, "skrj", "Skaarj War Hybrid")
        }
        SkinArchetype::ArcticJaeger => {
            let camo = SkinCamoPattern::UrbanSplinter;
            let ins = SkinInsignia::IronGuardShield;
            let face = SkinFaceStyle::PilotGoggles;
            (camo, ins, face, "arct", "Arctic Jaeger Infiltrator")
        }
        SkinArchetype::DesertNomad => {
            let camo = SkinCamoPattern::DesertDigital;
            let ins = SkinInsignia::Crosshair;
            let face = SkinFaceStyle::CombatBeard;
            (camo, ins, face, "dsrt", "Desert Nomad Raider")
        }
        SkinArchetype::IronGuardVeteran => {
            let camo = SkinCamoPattern::BrushedMetal;
            let ins = SkinInsignia::IronGuardShield;
            let face = *rng.choose(&[SkinFaceStyle::EyeScar, SkinFaceStyle::Stubble]).unwrap();
            (camo, ins, face, "irong", "Iron Guard Veteran")
        }
        SkinArchetype::NeonSynthwave => {
            let camo = SkinCamoPattern::HexCamo;
            let ins = SkinInsignia::Lightning;
            let face = SkinFaceStyle::TacticalVisor;
            (camo, ins, face, "neon", "Neon Synthwave Cyber")
        }
        SkinArchetype::ToxicMutant => {
            let camo = SkinCamoPattern::HexCamo;
            let ins = SkinInsignia::Biohazard;
            let face = SkinFaceStyle::NecrisVeins;
            (camo, ins, face, "toxc", "Toxic Sludge Mutant")
        }
        SkinArchetype::ClassicArena90s => {
            let camo = SkinCamoPattern::Solid;
            let ins = SkinInsignia::VLogo;
            let face = SkinFaceStyle::Clean;
            (camo, ins, face, "tour", "Tournament '99 Arena Master")
        }
    };

    let weathering = (rng.next_f32() * 0.5 + 0.1).clamp(0.05, 0.95);
    let armor_coverage = (rng.next_f32() * 0.4 + 0.45).clamp(0.3, 0.95);
    let team_trim_intensity = (rng.next_f32() * 0.3 + 0.7).clamp(0.5, 1.0);

    let rnd_id = rng.next_range(10, 99);
    let skin_name = format!("{name_stem}{rnd_id}");

    ProceduralSkinConfig {
        seed,
        archetype: arch,
        camo_pattern: camo,
        insignia: ins,
        face_style: face,
        weathering,
        armor_coverage,
        team_trim_intensity,
        skin_name,
        skin_description: desc.to_string(),
        author: "SkinWizard".to_string(),
    }
}

/// Procedurally synthesizes a 256x256 Torso/Chest texture conforming to UT99.
pub fn generate_procedural_chest(config: &ProceduralSkinConfig, team: SkinTeamColor) -> SkinTexture {
    let w = 256u32;
    let h = 256u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];

    let palette = get_archetype_color_palette(config.archetype);
    synthesize_camo_base(&mut rgba, w, h, config.camo_pattern, palette, config.seed);
    apply_armor_plating(&mut rgba, w, h, config.armor_coverage, palette[1], palette[2]);
    apply_insignia_stencil(&mut rgba, w, h, config.insignia, 128, 110, 48, [230, 230, 240, 255]);

    if team != SkinTeamColor::None {
        apply_team_accent_trim(&mut rgba, w, h, team, config.team_trim_intensity);
    }
    apply_weathering_scratches(&mut rgba, w, h, config.weathering, config.seed ^ 0xFEEDFACE);

    let name = format!("Chest_{}{}", config.skin_name, team.suffix());
    SkinTexture::from_rgba(name, w, h, rgba)
}

/// Procedurally synthesizes a 256x256 Legs/Lower body texture conforming to UT99.
pub fn generate_procedural_legs(config: &ProceduralSkinConfig, team: SkinTeamColor) -> SkinTexture {
    let w = 256u32;
    let h = 256u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];

    let palette = get_archetype_color_palette(config.archetype);
    synthesize_camo_base(&mut rgba, w, h, config.camo_pattern, palette, config.seed.wrapping_add(101));
    apply_leg_armor_and_boots(&mut rgba, w, h, config.armor_coverage, palette[1], palette[3]);

    if team != SkinTeamColor::None {
        apply_leg_team_stripes(&mut rgba, w, h, team, config.team_trim_intensity);
    }
    apply_weathering_scratches(&mut rgba, w, h, config.weathering, config.seed ^ 0xCAFEBABE);

    let name = format!("Legs_{}{}", config.skin_name, team.suffix());
    SkinTexture::from_rgba(name, w, h, rgba)
}

/// Procedurally synthesizes a 256x256 Arms/Shoulders texture conforming to UT99.
pub fn generate_procedural_arms(config: &ProceduralSkinConfig) -> SkinTexture {
    let w = 256u32;
    let h = 256u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];

    let palette = get_archetype_color_palette(config.archetype);
    synthesize_camo_base(&mut rgba, w, h, config.camo_pattern, palette, config.seed.wrapping_add(202));
    apply_shoulder_guards(&mut rgba, w, h, palette[1], palette[2]);
    apply_weathering_scratches(&mut rgba, w, h, config.weathering, config.seed ^ 0xDEADBEEF);

    let name = format!("Arms_{}", config.skin_name);
    SkinTexture::from_rgba(name, w, h, rgba)
}

/// Procedurally synthesizes a 256x256 Head & Face texture conforming to UT99.
pub fn generate_procedural_face(config: &ProceduralSkinConfig) -> SkinTexture {
    let w = 256u32;
    let h = 256u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];

    let skin_base = get_archetype_skin_tone(config.archetype);
    synthesize_face_mesh_base(&mut rgba, w, h, skin_base, config.seed.wrapping_add(303));
    apply_face_styling(&mut rgba, w, h, config.face_style, config.archetype);

    let name = format!("Face_{}", config.skin_name);
    SkinTexture::from_rgba(name, w, h, rgba)
}

/// Generates a styled 64x64 HUD TalkTexture with retro CRT scanlines and tactical frame.
pub fn generate_styled_talk_texture(face_tex: &SkinTexture, config: &ProceduralSkinConfig, team: SkinTeamColor) -> SkinTexture {
    let base_talk = generate_talk_texture_from_face(face_tex);
    let mut rgba = base_talk.rgba;

    // Apply CRT Scanlines (15% darkening on odd rows)
    for y in 0..64 {
        if y % 2 == 1 {
            for x in 0..64 {
                let idx = (y * 64 + x) * 4;
                rgba[idx] = (rgba[idx] as f32 * 0.85) as u8;
                rgba[idx + 1] = (rgba[idx + 1] as f32 * 0.85) as u8;
                rgba[idx + 2] = (rgba[idx + 2] as f32 * 0.85) as u8;
            }
        }
    }

    // Apply Tactical Corner Brackets (HUD Reticle)
    let bracket_color = if team == SkinTeamColor::None {
        [80, 220, 255, 255]
    } else {
        let [r, g, b] = team.badge_rgb();
        [r, g, b, 255]
    };

    let corners = [(2, 2, 1, 1), (61, 2, -1, 1), (2, 61, 1, -1), (61, 61, -1, -1)];
    for (cx, cy, dx, dy) in corners {
        for step in 0..6 {
            let px = (cx as i32 + dx * step) as usize;
            let py = cy as usize;
            let idx1 = (py * 64 + px) * 4;
            rgba[idx1..idx1 + 4].copy_from_slice(&bracket_color);

            let qx = cx as usize;
            let qy = (cy as i32 + dy * step) as usize;
            let idx2 = (qy * 64 + qx) * 4;
            rgba[idx2..idx2 + 4].copy_from_slice(&bracket_color);
        }
    }

    let name = format!("Talk_{}{}", config.skin_name, team.suffix());
    SkinTexture::from_rgba(name, 64, 64, rgba)
}

/// 1-Click Master Generator: Produces a fully conforming, ready-to-export `SkinStudioProject`.
pub fn generate_complete_skin_project(config: &ProceduralSkinConfig, model_idx: usize) -> SkinStudioProject {
    let models = get_builtin_skin_models();
    let m = &models[model_idx.min(models.len().saturating_sub(1))];

    let package_name = format!("{}Skins_{}", m.model_prefix, config.skin_name);
    let mut proj = SkinStudioProject {
        package_name,
        skin_id: config.skin_name.clone(),
        skin_description: config.skin_description.clone(),
        author: config.author.clone(),
        selected_model_index: model_idx,
        active_team: SkinTeamColor::None,
        active_face_index: 0,
        active_camera_preset_index: 0,
        active_animation: "Breath".to_string(),
        chest_base: None,
        chest_teams: [None, None, None, None],
        legs_base: None,
        legs_teams: [None, None, None, None],
        arms_base: None,
        faces: Vec::new(),
        use_shared_palette: true,
        single_mipmap_only: false,
    };

    // Synthesize Slot 1: Chest (Base & 4 Teams)
    proj.chest_base = Some(generate_procedural_chest(config, SkinTeamColor::None));
    proj.chest_teams[0] = Some(generate_procedural_chest(config, SkinTeamColor::Red));
    proj.chest_teams[1] = Some(generate_procedural_chest(config, SkinTeamColor::Blue));
    proj.chest_teams[2] = Some(generate_procedural_chest(config, SkinTeamColor::Green));
    proj.chest_teams[3] = Some(generate_procedural_chest(config, SkinTeamColor::Gold));

    // Synthesize Slot 2: Legs (Base & 4 Teams)
    proj.legs_base = Some(generate_procedural_legs(config, SkinTeamColor::None));
    proj.legs_teams[0] = Some(generate_procedural_legs(config, SkinTeamColor::Red));
    proj.legs_teams[1] = Some(generate_procedural_legs(config, SkinTeamColor::Blue));
    proj.legs_teams[2] = Some(generate_procedural_legs(config, SkinTeamColor::Green));
    proj.legs_teams[3] = Some(generate_procedural_legs(config, SkinTeamColor::Gold));

    // Synthesize Slot 3: Arms (Base)
    proj.arms_base = Some(generate_procedural_arms(config));

    // Synthesize Slot 4 & 5: Face & TalkTexture
    let face_tex = generate_procedural_face(config);
    let talk_tex = generate_styled_talk_texture(&face_tex, config, SkinTeamColor::None);

    let mut face_entry = SkinFaceEntry {
        face_name: config.skin_name.clone(),
        display_name: config.skin_description.clone(),
        face_texture: face_tex.clone(),
        talk_texture: talk_tex,
        team_talk_textures: [None, None, None, None],
    };

    for (t_idx, team) in [SkinTeamColor::Red, SkinTeamColor::Blue, SkinTeamColor::Green, SkinTeamColor::Gold].iter().enumerate() {
        face_entry.team_talk_textures[t_idx] = Some(generate_styled_talk_texture(&face_tex, config, *team));
    }

    proj.faces.push(face_entry);
    proj
}

// -------------------------------------------------------------------------------------------------
// Low-Level Graphic Primitives for Texture Compositing
// -------------------------------------------------------------------------------------------------

fn get_archetype_color_palette(arch: SkinArchetype) -> [[u8; 3]; 4] {
    match arch {
        SkinArchetype::TacticalSpecOps => [
            [45, 55, 38],   // Olive green base
            [30, 36, 25],   // Dark foliage
            [75, 85, 60],   // Khaki highlight
            [22, 25, 20],   // Deep shadow
        ],
        SkinArchetype::LiandriCyberMech => [
            [32, 35, 42],   // Titanium graphite
            [55, 60, 70],   // Chrome plate
            [20, 22, 26],   // Carbon weave
            [220, 130, 20], // Liandri orange accent
        ],
        SkinArchetype::NecrisShadow => [
            [18, 16, 20],   // Obsidian black
            [30, 25, 35],   // Dark leather
            [50, 42, 55],   // Deep violet sheen
            [160, 25, 35],  // Blood crimson rune
        ],
        SkinArchetype::GladiatorArena => [
            [70, 48, 32],   // Saddle leather
            [130, 95, 50],  // Polished bronze
            [45, 30, 20],   // Heavy stud shadow
            [180, 140, 60], // Brass trim
        ],
        SkinArchetype::HazardShockTrooper => [
            [40, 42, 45],   // Rubberized hazmat grey
            [60, 62, 66],   // Reinforced knee canvas
            [25, 26, 28],   // Gas canister black
            [210, 180, 25], // Radiation hazard yellow
        ],
        SkinArchetype::SkaarjWarHybrid => [
            [50, 68, 42],   // Reptilian scale green
            [75, 95, 55],   // Acid dorsal ridge
            [32, 45, 28],   // Chitin shadow
            [160, 145, 110],// Bone plate armor
        ],
        SkinArchetype::ArcticJaeger => [
            [215, 222, 230],// Snow white
            [165, 175, 190],// Slate shadow
            [120, 135, 155],// Polar navy
            [70, 140, 210], // Frost cyan trim
        ],
        SkinArchetype::DesertNomad => [
            [165, 140, 105],// Desert khaki sand
            [125, 100, 70], // Weathered canvas
            [200, 175, 135],// Bright dune dust
            [140, 75, 35],  // Leather strap
        ],
        SkinArchetype::IronGuardVeteran => [
            [75, 80, 85],   // Scuffed iron plate
            [105, 112, 120],// Highlight edge
            [45, 48, 52],   // Dark cast metal
            [180, 50, 45],  // Iron Guard red stripe
        ],
        SkinArchetype::NeonSynthwave => [
            [15, 18, 28],   // Void navy
            [35, 40, 60],   // Cyber mesh
            [235, 40, 130], // Fluorescent neon magenta
            [40, 220, 240], // Electric cyan
        ],
        SkinArchetype::ToxicMutant => [
            [45, 85, 35],   // Toxic slime green
            [75, 135, 45],  // Glowing bile highlight
            [28, 52, 22],   // Necrotic shadow
            [210, 235, 30], // Acid bio-core
        ],
        SkinArchetype::ClassicArena90s => [
            [35, 55, 95],   // Classic UT blue-slate
            [65, 85, 135],  // Saturated panel highlight
            [22, 32, 55],   // Deep blue shadow
            [215, 175, 40], // Tournament gold crest
        ],
    }
}

fn get_archetype_skin_tone(arch: SkinArchetype) -> [u8; 3] {
    match arch {
        SkinArchetype::NecrisShadow => [195, 200, 210],    // Pale deathly grey
        SkinArchetype::SkaarjWarHybrid => [105, 145, 95], // Reptilian scale green
        SkinArchetype::LiandriCyberMech => [165, 170, 178],// Chrome metallic flesh
        SkinArchetype::ToxicMutant => [95, 160, 80],      // Sickly necrotic green
        SkinArchetype::DesertNomad => [180, 135, 100],     // Deep desert tan
        _ => [215, 165, 135],                              // Classic heroic flesh tone
    }
}

fn synthesize_camo_base(
    buffer: &mut [u8],
    w: u32,
    h: u32,
    pattern: SkinCamoPattern,
    palette: [[u8; 3]; 4],
    seed: u64,
) {
    let mut rng = SplitMix64Rng::new(seed);

    for y in 0..h {
        for x in 0..w {
            let pal_idx = match pattern {
                SkinCamoPattern::Solid => {
                    let noise = (rng.next_u64() % 100) as i32 - 50;
                    if noise > 35 { 2 } else if noise < -35 { 3 } else { 0 }
                }
                SkinCamoPattern::Woodland => {
                    let fx = (x as f32 * 0.04).sin();
                    let fy = (y as f32 * 0.04).cos();
                    let fxy = ((x as f32 + y as f32) * 0.02).sin();
                    let val = fx + fy + fxy;
                    if val > 0.8 { 1 } else if val > 0.0 { 0 } else if val > -0.7 { 2 } else { 3 }
                }
                SkinCamoPattern::DesertDigital | SkinCamoPattern::UrbanSplinter => {
                    let cell_x = x / 16;
                    let cell_y = y / 16;
                    let hash = (cell_x.wrapping_mul(73856093) ^ cell_y.wrapping_mul(19349663) ^ (seed as u32)) % 4;
                    hash as usize
                }
                SkinCamoPattern::Flecktarn => {
                    let d = ((x % 24) as i32 - 12).pow(2) + ((y % 24) as i32 - 12).pow(2);
                    if d < 18 { 1 } else if d < 45 { 2 } else { 0 }
                }
                SkinCamoPattern::Tigerstripe => {
                    let wave = (x as f32 * 0.02 + (y as f32 * 0.1).sin() * 2.0) as i32;
                    let stripe = (y as i32 + wave * 4) % 36;
                    if stripe < 10 { 3 } else if stripe < 18 { 1 } else if stripe < 26 { 2 } else { 0 }
                }
                SkinCamoPattern::HexCamo => {
                    let q = (x as i32) / 20;
                    let r = (y as i32) / 18;
                    let is_line = (x % 20 < 2) || (y % 18 < 2);
                    if is_line { 2 } else if (q + r) % 2 == 0 { 0 } else { 1 }
                }
                SkinCamoPattern::CarbonWeave => {
                    let diag = (x / 4 + y / 4) % 2;
                    if diag == 0 { 0 } else { 2 }
                }
                SkinCamoPattern::HazardStripes => {
                    let diag = (x + y) / 18;
                    if diag % 2 == 0 { 0 } else { 3 }
                }
                SkinCamoPattern::BrushedMetal => {
                    let streak = (rng.next_u64() % 4) as usize;
                    streak
                }
                SkinCamoPattern::ChitinScales => {
                    let sc_x = (x % 32) as i32 - 16;
                    let sc_y = (y % 32) as i32 - 16;
                    let dist = (sc_x * sc_x + sc_y * sc_y) as f32;
                    if dist < 80.0 { 1 } else if dist < 180.0 { 0 } else { 2 }
                }
            };

            let col = palette[pal_idx % 4];
            let idx = ((y * w + x) * 4) as usize;
            buffer[idx] = col[0];
            buffer[idx + 1] = col[1];
            buffer[idx + 2] = col[2];
            buffer[idx + 3] = 255;
        }
    }
}

fn apply_armor_plating(
    buffer: &mut [u8],
    w: u32,
    h: u32,
    coverage: f32,
    plate_color: [u8; 3],
    highlight_color: [u8; 3],
) {
    let plate_half_w = (60.0 * coverage) as i32;
    let chest_y0 = 60i32;
    let chest_y1 = (60.0 + 110.0 * coverage) as i32;

    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let cx = x - 128;
            if cx.abs() < plate_half_w && y >= chest_y0 && y <= chest_y1 {
                let idx = ((y as u32 * w + x as u32) * 4) as usize;

                // Bevel highlight top-left, shadow bottom-right
                let is_top = (y - chest_y0).abs() < 3;
                let is_left = (cx + plate_half_w).abs() < 3;
                let is_bottom = (chest_y1 - y).abs() < 3;
                let is_right = (plate_half_w - cx).abs() < 3;

                if is_top || is_left {
                    buffer[idx] = highlight_color[0];
                    buffer[idx + 1] = highlight_color[1];
                    buffer[idx + 2] = highlight_color[2];
                } else if is_bottom || is_right {
                    buffer[idx] = (plate_color[0] as f32 * 0.6) as u8;
                    buffer[idx + 1] = (plate_color[1] as f32 * 0.6) as u8;
                    buffer[idx + 2] = (plate_color[2] as f32 * 0.6) as u8;
                } else {
                    buffer[idx] = plate_color[0];
                    buffer[idx + 1] = plate_color[1];
                    buffer[idx + 2] = plate_color[2];
                }

                // Rivets on plate corners
                let near_corner = (cx.abs() - plate_half_w + 8).abs() < 3 && ((y - chest_y0 - 8).abs() < 3 || (chest_y1 - y - 8).abs() < 3);
                if near_corner {
                    buffer[idx] = 230;
                    buffer[idx + 1] = 230;
                    buffer[idx + 2] = 240;
                }
            }
        }
    }
}

fn apply_insignia_stencil(
    buffer: &mut [u8],
    w: u32,
    h: u32,
    insignia: SkinInsignia,
    cx: u32,
    cy: u32,
    size: u32,
    tint: [u8; 4],
) {
    if insignia == SkinInsignia::None {
        return;
    }
    let half = (size / 2) as i32;

    for dy in -half..=half {
        for dx in -half..=half {
            let px = cx as i32 + dx;
            let py = cy as i32 + dy;
            if px < 0 || px >= w as i32 || py < 0 || py >= h as i32 {
                continue;
            }

            let r_sq = dx * dx + dy * dy;
            let draw = match insignia {
                SkinInsignia::None => false,
                SkinInsignia::LiandriLogo => {
                    let in_outer_ring = (r_sq > (half - 4).pow(2)) && (r_sq <= half.pow(2));
                    let in_l_vert = dx >= -half / 3 && dx <= -half / 3 + 4 && dy.abs() < half * 2 / 3;
                    let in_l_horiz = dy >= half / 3 - 4 && dy <= half / 3 && dx >= -half / 3 && dx <= half / 3;
                    in_outer_ring || in_l_vert || in_l_horiz
                }
                SkinInsignia::IronGuardShield => {
                    let in_shield = dx.abs() <= (half - dy.max(0) / 2) && dy >= -half && dy <= half;
                    let cross = dx.abs() < 3 || dy.abs() < 3;
                    in_shield && cross
                }
                SkinInsignia::SkullAndBones => {
                    let in_cranium = dx * dx + (dy + 4) * (dy + 4) <= (half / 2).pow(2);
                    let in_jaw = dx.abs() < 6 && dy >= 4 && dy <= 12;
                    let eye_sockets = (dx.abs() == 4 || dx.abs() == 5) && dy == 0;
                    (in_cranium || in_jaw) && !eye_sockets
                }
                SkinInsignia::Biohazard => {
                    let c1 = dx * dx + (dy + half / 2).pow(2) < (half / 3).pow(2);
                    let c2 = (dx - half / 2).pow(2) + (dy - half / 3).pow(2) < (half / 3).pow(2);
                    let c3 = (dx + half / 2).pow(2) + (dy - half / 3).pow(2) < (half / 3).pow(2);
                    let center_hole = r_sq < (half / 6).pow(2);
                    (c1 || c2 || c3) && !center_hole
                }
                SkinInsignia::Radiation => {
                    let angle = (dy as f32).atan2(dx as f32).to_degrees() + 180.0;
                    let in_fan = (angle >= 0.0 && angle < 60.0) || (angle >= 120.0 && angle < 180.0) || (angle >= 240.0 && angle < 300.0);
                    let center_dot = r_sq < (half / 4).pow(2);
                    let in_range = r_sq <= half.pow(2) && r_sq >= (half / 3).pow(2);
                    (in_range && in_fan) || center_dot
                }
                SkinInsignia::Lightning => {
                    let on_zig = (dx + dy / 2).abs() < 4 && dy < 0;
                    let on_zag = (dx - 4 + dy / 2).abs() < 4 && dy >= 0;
                    on_zig || on_zag
                }
                SkinInsignia::Crosshair => {
                    let ring = (r_sq > (half - 3).pow(2)) && (r_sq <= half.pow(2));
                    let cross = (dx.abs() < 2 && dy.abs() <= half) || (dy.abs() < 2 && dx.abs() <= half);
                    ring || cross
                }
                SkinInsignia::VLogo => {
                    let on_v = (dx.abs() - dy).abs() < 4 && dy >= -half && dy <= half;
                    on_v
                }
            };

            if draw {
                let idx = ((py as u32 * w + px as u32) * 4) as usize;
                buffer[idx] = tint[0];
                buffer[idx + 1] = tint[1];
                buffer[idx + 2] = tint[2];
            }
        }
    }
}

fn apply_team_accent_trim(
    buffer: &mut [u8],
    w: u32,
    h: u32,
    team: SkinTeamColor,
    intensity: f32,
) {
    let [tr, tg, tb] = team.badge_rgb();

    for y in 0..h {
        for x in 0..w {
            let is_shoulder_trim = (y < 45) && (x < 65 || x > 190);
            let is_chest_chevron = (y >= 80 && y <= 95) && ((x as i32 - 128).abs() < 35);
            let is_collar = (y >= 45 && y <= 55) && ((x as i32 - 128).abs() < 25);

            if is_shoulder_trim || is_chest_chevron || is_collar {
                let idx = ((y * w + x) * 4) as usize;
                let cur_r = buffer[idx] as f32;
                let cur_g = buffer[idx + 1] as f32;
                let cur_b = buffer[idx + 2] as f32;

                buffer[idx] = (cur_r * (1.0 - intensity) + tr as f32 * intensity).clamp(0.0, 255.0) as u8;
                buffer[idx + 1] = (cur_g * (1.0 - intensity) + tg as f32 * intensity).clamp(0.0, 255.0) as u8;
                buffer[idx + 2] = (cur_b * (1.0 - intensity) + tb as f32 * intensity).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

fn apply_leg_armor_and_boots(
    buffer: &mut [u8],
    w: u32,
    h: u32,
    coverage: f32,
    plate_color: [u8; 3],
    boot_color: [u8; 3],
) {
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            // Boots at bottom 30%
            if y > 180 {
                buffer[idx] = boot_color[0];
                buffer[idx + 1] = boot_color[1];
                buffer[idx + 2] = boot_color[2];
            }
            // Kneepads in middle
            else if y >= 110 && y <= 145 && coverage > 0.3 {
                let is_left_pad = (x as i32 - 64).abs() < 24;
                let is_right_pad = (x as i32 - 192).abs() < 24;
                if is_left_pad || is_right_pad {
                    buffer[idx] = plate_color[0];
                    buffer[idx + 1] = plate_color[1];
                    buffer[idx + 2] = plate_color[2];
                }
            }
        }
    }
}

fn apply_leg_team_stripes(
    buffer: &mut [u8],
    w: u32,
    _h: u32,
    team: SkinTeamColor,
    intensity: f32,
) {
    let [tr, tg, tb] = team.badge_rgb();
    for y in 30..170 {
        for x in [35..45, 145..155] {
            for px in x {
                let idx = ((y * w + px) * 4) as usize;
                let r = buffer[idx] as f32;
                let g = buffer[idx + 1] as f32;
                let b = buffer[idx + 2] as f32;
                buffer[idx] = (r * (1.0 - intensity) + tr as f32 * intensity) as u8;
                buffer[idx + 1] = (g * (1.0 - intensity) + tg as f32 * intensity) as u8;
                buffer[idx + 2] = (b * (1.0 - intensity) + tb as f32 * intensity) as u8;
            }
        }
    }
}

fn apply_shoulder_guards(
    buffer: &mut [u8],
    w: u32,
    _h: u32,
    plate_color: [u8; 3],
    highlight_color: [u8; 3],
) {
    for y in 20..90 {
        for x in 0..w {
            let is_epaulet = (x < 80) || (x > 175);
            if is_epaulet {
                let idx = ((y * w + x) * 4) as usize;
                let is_edge = y == 20 || y == 89;
                let col = if is_edge { highlight_color } else { plate_color };
                buffer[idx] = col[0];
                buffer[idx + 1] = col[1];
                buffer[idx + 2] = col[2];
            }
        }
    }
}

fn synthesize_face_mesh_base(
    buffer: &mut [u8],
    w: u32,
    h: u32,
    skin_base: [u8; 3],
    seed: u64,
) {
    let mut rng = SplitMix64Rng::new(seed);
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let variation = (rng.next_u64() % 16) as i32 - 8;
            let r = (skin_base[0] as i32 + variation).clamp(0, 255) as u8;
            let g = (skin_base[1] as i32 + variation).clamp(0, 255) as u8;
            let b = (skin_base[2] as i32 + variation).clamp(0, 255) as u8;
            buffer[idx] = r;
            buffer[idx + 1] = g;
            buffer[idx + 2] = b;
            buffer[idx + 3] = 255;
        }
    }

    // Draw Eyes (Central face region: x around 100 & 156, y around 100)
    for (eye_x, eye_y) in [(102i32, 98i32), (154i32, 98i32)] {
        for dy in -5..=5 {
            for dx in -8..=8 {
                let px = eye_x + dx;
                let py = eye_y + dy;
                if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                    let idx = ((py as u32 * w + px as u32) * 4) as usize;
                    if idx + 2 < buffer.len() {
                        if dx.abs() <= 3 && dy.abs() <= 3 {
                            // Pupil
                            buffer[idx] = 20;
                            buffer[idx + 1] = 25;
                            buffer[idx + 2] = 30;
                        } else {
                            // Sclera
                            buffer[idx] = 240;
                            buffer[idx + 1] = 242;
                            buffer[idx + 2] = 245;
                        }
                    }
                }
            }
        }
    }

    // Draw Mouth / Lips (x: 128, y: 155)
    for dy in -3..=3 {
        for dx in -18..=18 {
            let px = 128i32 + dx;
            let py = 155i32 + dy;
            if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                let idx = ((py as u32 * w + px as u32) * 4) as usize;
                if idx + 2 < buffer.len() {
                    buffer[idx] = (skin_base[0] as f32 * 0.75) as u8;
                    buffer[idx + 1] = (skin_base[1] as f32 * 0.65) as u8;
                    buffer[idx + 2] = (skin_base[2] as f32 * 0.65) as u8;
                }
            }
        }
    }
}

fn apply_face_styling(
    buffer: &mut [u8],
    w: u32,
    h: u32,
    style: SkinFaceStyle,
    _arch: SkinArchetype,
) {
    let mut put_pixel = |x: i32, y: i32, rgb: [u8; 3]| {
        if x >= 0 && x < w as i32 && y >= 0 && y < h as i32 {
            let idx = ((y as u32 * w + x as u32) * 4) as usize;
            if idx + 2 < buffer.len() {
                buffer[idx] = rgb[0];
                buffer[idx + 1] = rgb[1];
                buffer[idx + 2] = rgb[2];
            }
        }
    };

    match style {
        SkinFaceStyle::Clean => {}
        SkinFaceStyle::Stubble | SkinFaceStyle::CombatBeard => {
            let beard_y0 = if style == SkinFaceStyle::CombatBeard { 135 } else { 145 };
            for y in beard_y0..210 {
                for x in 85..170 {
                    if (x + y * 3) % 2 == 0 {
                        put_pixel(x, y, [40, 35, 30]);
                    }
                }
            }
        }
        SkinFaceStyle::WarPaint => {
            for y in 70..160 {
                for offset in [-35, 35] {
                    for w_off in -4..=4 {
                        put_pixel(128 + offset + w_off, y, [190, 25, 25]);
                    }
                }
            }
        }
        SkinFaceStyle::EyeScar => {
            for step in -30..30 {
                put_pixel(102 + step / 2, 98 + step, [180, 70, 70]);
            }
        }
        SkinFaceStyle::CyberneticEye => {
            for dy in -8..=8 {
                for dx in -8..=8 {
                    let col = if dx * dx + dy * dy < 36 {
                        // Glowing Red Ocular
                        [255, 30, 30]
                    } else {
                        // Metal Housing
                        [80, 85, 95]
                    };
                    put_pixel(154 + dx, 98 + dy, col);
                }
            }
        }
        SkinFaceStyle::TacticalVisor => {
            for y in 88..108 {
                for x in 85..172 {
                    let is_frame = y == 88 || y == 107 || x == 85 || x == 171;
                    let col = if is_frame {
                        [40, 45, 50]
                    } else {
                        // Glowing Cyan Visor
                        [30, 220, 245]
                    };
                    put_pixel(x, y, col);
                }
            }
        }
        SkinFaceStyle::Gasmask => {
            for y in 135i32..195 {
                for x in 95i32..162 {
                    let dx = x - 128;
                    let dy = y - 165;
                    let dist_c = (dx * dx + dy * dy) as f32;
                    if dist_c < 750.0 {
                        put_pixel(x, y, [35, 38, 42]);
                    }
                }
            }
        }
        SkinFaceStyle::PilotGoggles => {
            for center_x in [102i32, 154i32] {
                for dy in -12i32..=12 {
                    for dx in -14i32..=14 {
                        let col = if dx.abs() > 10 || dy.abs() > 9 {
                            [160, 135, 60] // Brass rim
                        } else {
                            [40, 120, 180] // Tinted glass
                        };
                        put_pixel(center_x + dx, 98 + dy, col);
                    }
                }
            }
        }
        SkinFaceStyle::NecrisVeins => {
            for y in 70..170 {
                let px = 128 + ((y as f32 * 0.15).sin() * 22.0) as i32;
                put_pixel(px, y, [75, 25, 65]);
            }
        }
    }
}

fn apply_weathering_scratches(
    buffer: &mut [u8],
    w: u32,
    h: u32,
    weathering: f32,
    seed: u64,
) {
    if weathering <= 0.05 {
        return;
    }
    let mut rng = SplitMix64Rng::new(seed);
    let scratch_count = (120.0 * weathering) as usize;

    for _ in 0..scratch_count {
        let sx = rng.next_range(10, w as i32 - 20) as u32;
        let sy = rng.next_range(10, h as i32 - 20) as u32;
        let len = rng.next_range(4, 18);
        let dx = rng.next_range(-1, 1);
        let dy = rng.next_range(0, 2);

        for step in 0..len {
            let px = (sx as i32 + dx * step).clamp(0, w as i32 - 1) as u32;
            let py = (sy as i32 + dy * step).clamp(0, h as i32 - 1) as u32;
            let idx = ((py * w + px) * 4) as usize;
            buffer[idx] = (buffer[idx] as f32 * 0.4 + 140.0).min(255.0) as u8;
            buffer[idx + 1] = (buffer[idx + 1] as f32 * 0.4 + 140.0).min(255.0) as u8;
            buffer[idx + 2] = (buffer[idx + 2] as f32 * 0.4 + 145.0).min(255.0) as u8;
        }
    }
}


// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_models_catalog() {
        let models = get_builtin_skin_models();
        assert_eq!(models.len(), 10);
        let soldier = &models[0];
        assert_eq!(soldier.display_name, "Male Soldier");
        assert_eq!(soldier.model_prefix, "Soldier");
        assert_eq!(soldier.mesh_package, "Botpack.Soldier");
        assert_eq!(soldier.sections.len(), 5);
    }

    #[test]
    fn test_talk_texture_generation() {
        let face = SkinTexture::create_solid("TestFace".to_string(), 256, 256, [200, 150, 120, 255]);
        let talk = generate_talk_texture_from_face(&face);
        assert_eq!(talk.width, 64);
        assert_eq!(talk.height, 64);
        assert_eq!(talk.rgba.len(), 64 * 64 * 4);
    }

    #[test]
    fn test_quantization_and_mipmaps() {
        let tex = SkinTexture::create_solid("SolidTest".to_string(), 64, 64, [100, 150, 200, 255]);
        assert!(tex.palette.is_some());
        assert!(tex.indices.is_some());
        assert_eq!(tex.palette.unwrap().len(), 256);
        assert_eq!(tex.indices.unwrap().len(), 64 * 64);
        // 64 -> 32 -> 16 -> 8 -> 4 -> 2 -> 1 = 6 mipmap levels
        assert_eq!(tex.mipmaps.len(), 6);
        assert_eq!(tex.mipmaps.last().unwrap().width, 1);
        assert_eq!(tex.mipmaps.last().unwrap().height, 1);
    }

    #[test]
    fn test_int_manifest_generation() {
        let proj = SkinStudioProject::default();
        let int_txt = generate_int_registration_manifest(&proj);
        assert!(int_txt.contains("[public]"));
        assert!(int_txt.contains("Object=(Name=SoldierSkins_Custom.cust1,Class=Texture,Description=\"Custom Soldier\")"));
        assert!(int_txt.contains("Object=(Name=SoldierSkins_Custom.cust2,Class=Texture)"));
        assert!(int_txt.contains("Object=(Name=SoldierSkins_Custom.cust3,Class=Texture)"));
        assert!(int_txt.contains("Object=(Name=SoldierSkins_Custom.cust4,Class=Texture)"));
        assert!(int_txt.contains("Object=(Name=SoldierSkins_Custom.cust4Marcus,Class=Texture,Description=\"Marcus\")"));
        assert!(int_txt.contains("Object=(Name=SoldierSkins_Custom.cust5Marcus,Class=Texture)"));
        assert!(int_txt.contains("Object=(Name=SoldierSkins_Custom.cust1T_0,Class=Texture)"));
        assert!(int_txt.contains("Object=(Name=SoldierSkins_Custom.cust2T_3,Class=Texture)"));
    }

    #[test]
    fn test_audit_skin_project_validation() {
        let mut proj = SkinStudioProject::default();
        let report = audit_skin_project(&proj);
        assert_eq!(report[0].severity, SkinAuditSeverity::Info);

        // Blank out chest to trigger error
        proj.chest_base = None;
        let err_report = audit_skin_project(&proj);
        assert!(err_report.iter().any(|r| r.severity == SkinAuditSeverity::Error));
    }

    #[test]
    fn test_randomize_skin_config_all_archetypes() {
        for (i, arch) in SkinArchetype::all().iter().enumerate() {
            let seed = 42 + i as u64 * 100;
            let cfg = randomize_skin_config(seed, Some(*arch));
            assert_eq!(cfg.archetype, *arch);
            assert!(!cfg.skin_name.is_empty());
            assert!(!cfg.skin_description.is_empty());
            assert!(cfg.weathering >= 0.0 && cfg.weathering <= 1.0);
            assert!(cfg.armor_coverage >= 0.0 && cfg.armor_coverage <= 1.0);
        }
    }

    #[test]
    fn test_procedural_chest_and_team_synthesis() {
        let cfg = ProceduralSkinConfig::default();
        let chest_dm = generate_procedural_chest(&cfg, SkinTeamColor::None);
        assert_eq!(chest_dm.width, 256);
        assert_eq!(chest_dm.height, 256);
        assert!(chest_dm.palette.is_some());
        assert_eq!(chest_dm.palette.unwrap().len(), 256);

        let chest_red = generate_procedural_chest(&cfg, SkinTeamColor::Red);
        assert_eq!(chest_red.width, 256);
        assert_eq!(chest_red.height, 256);
        // Team color should affect pixels
        assert_ne!(chest_dm.rgba, chest_red.rgba);
    }

    #[test]
    fn test_procedural_face_and_styled_talk_texture() {
        let cfg = ProceduralSkinConfig::default();
        let face_tex = generate_procedural_face(&cfg);
        assert_eq!(face_tex.width, 256);
        assert_eq!(face_tex.height, 256);

        let talk_tex = generate_styled_talk_texture(&face_tex, &cfg, SkinTeamColor::None);
        assert_eq!(talk_tex.width, 64);
        assert_eq!(talk_tex.height, 64);
        assert_eq!(talk_tex.rgba.len(), 64 * 64 * 4);

        let talk_blue = generate_styled_talk_texture(&face_tex, &cfg, SkinTeamColor::Blue);
        assert_eq!(talk_blue.width, 64);
        assert_eq!(talk_blue.height, 64);
    }

    #[test]
    fn test_1click_complete_skin_project_generation() {
        let cfg = ProceduralSkinConfig {
            seed: 9999,
            archetype: SkinArchetype::LiandriCyberMech,
            camo_pattern: SkinCamoPattern::CarbonWeave,
            insignia: SkinInsignia::LiandriLogo,
            face_style: SkinFaceStyle::TacticalVisor,
            weathering: 0.3,
            armor_coverage: 0.7,
            team_trim_intensity: 0.9,
            skin_name: "cyber".to_string(),
            skin_description: "Liandri Heavy Cyber-Mech".to_string(),
            author: "ReaAuthor".to_string(),
        };

        let proj = generate_complete_skin_project(&cfg, 0); // Soldier model
        assert!(proj.chest_base.is_some());
        assert!(proj.legs_base.is_some());
        assert!(proj.arms_base.is_some());
        assert_eq!(proj.chest_teams.len(), 4);
        for t in &proj.chest_teams {
            assert!(t.is_some());
        }
        for t in &proj.legs_teams {
            assert!(t.is_some());
        }
        assert_eq!(proj.faces.len(), 1);
        let f0 = &proj.faces[0];
        assert_eq!(f0.face_name, "cyber");
        assert_eq!(f0.display_name, "Liandri Heavy Cyber-Mech");
        assert_eq!(f0.talk_texture.width, 64);

        // Verify package audit passes
        let audit = audit_skin_project(&proj);
        let errors = audit.iter().filter(|r| r.severity == SkinAuditSeverity::Error).count();
        assert_eq!(errors, 0);

        // Verify INT manifest is fully populated
        let manifest = generate_int_registration_manifest(&proj);
        assert!(manifest.contains("Object=(Name=SoldierSkins_cyber.cyber1,Class=Texture,Description=\"Liandri Heavy Cyber-Mech\")"));
        assert!(manifest.contains("Object=(Name=SoldierSkins_cyber.cyber1T_0,Class=Texture)"));
        assert!(manifest.contains("Object=(Name=SoldierSkins_cyber.cyber2T_3,Class=Texture)"));
        assert!(manifest.contains("Object=(Name=SoldierSkins_cyber.cyber4cyber,Class=Texture,Description=\"Liandri Heavy Cyber-Mech\")"));
        assert!(manifest.contains("Object=(Name=SoldierSkins_cyber.cyber5cyber,Class=Texture)"));
    }

    #[test]
    fn test_export_skin_project_to_utx_and_int() {
        let cfg = ProceduralSkinConfig {
            seed: 42,
            archetype: SkinArchetype::TacticalSpecOps,
            camo_pattern: SkinCamoPattern::DesertDigital,
            insignia: SkinInsignia::Crosshair,
            face_style: SkinFaceStyle::TacticalVisor,
            weathering: 0.2,
            armor_coverage: 0.8,
            team_trim_intensity: 0.8,
            skin_name: "spec".to_string(),
            skin_description: "Tactical SpecOps".to_string(),
            author: "Tester".to_string(),
        };

        let proj = generate_complete_skin_project(&cfg, 0);
        let temp_dir = std::env::temp_dir().join(format!("forge99_test_export_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let res = export_skin_project(&proj, &temp_dir, true).expect("Export should succeed");
        assert!(res.package_path.exists());
        assert!(res.manifest_path.exists());
        assert!(res.package_bytes > 0);
        assert!(res.manifest_bytes > 0);
        assert_eq!(res.textures_compiled, 14); // 1 chest base + 4 teams + 1 leg base + 4 teams + 1 arm + 1 def face + 1 face + 1 talk

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_multiple_consecutive_random_rolls() {
        for style in SkinFaceStyle::all() {
            let mut cfg = ProceduralSkinConfig::default();
            cfg.face_style = *style;
            let face_tex = generate_procedural_face(&cfg);
            assert_eq!(face_tex.width, 256);
            assert_eq!(face_tex.height, 256);
            assert_eq!(face_tex.rgba.len(), 256 * 256 * 4);
        }

        let mut seed = 12345u64;
        for i in 0..200 {
            let rand_seed = (seed.wrapping_mul(6364136223846793005)).wrapping_add(1442695040888963407);
            let cfg = randomize_skin_config(rand_seed, None);
            seed = cfg.seed;
            let proj = generate_complete_skin_project(&cfg, i % 10);
            let _audit = audit_skin_project(&proj);
            assert!(!proj.faces.is_empty(), "Iteration {i} produced empty faces");
        }
    }
}

