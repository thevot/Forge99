use super::*;
use glam::Vec3;

/// Settings and real-time parallax projection configuration for 3D SkyBoxes
#[derive(Debug, Clone, PartialEq)]
pub struct SkyZoneSettings {
    pub enabled: bool,
    pub parallax_scale: f32, // 0.0 = fixed infinite horizon, >0.0 = parallax relative to anchor
    pub show_frustum: bool,
    pub highlight_sky_surfaces: bool,
    pub custom_anchor: Option<glam::Vec3>,
}

impl Default for SkyZoneSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            parallax_scale: 0.0,
            show_frustum: true,
            highlight_sky_surfaces: true,
            custom_anchor: None,
        }
    }
}

/// Canonical environmental presets for Unreal Tournament zones
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZonePhysicsPreset {
    StandardEarth,
    WaterSwimming,
    MuddySwamp,
    ToxicSlime,
    VolcanicLava,
    LowGravity,
    SpaceVacuum,
}

impl ZonePhysicsPreset {
    pub fn label(&self) -> &'static str {
        match self {
            ZonePhysicsPreset::StandardEarth => "Standard Earth (1.0G, Air)",
            ZonePhysicsPreset::WaterSwimming => "Water Fluid (0.4G, High Friction)",
            ZonePhysicsPreset::MuddySwamp => "Muddy Swamp (0.5G, Extreme Friction)",
            ZonePhysicsPreset::ToxicSlime => "Toxic Acid / Slime (Acid Damage)",
            ZonePhysicsPreset::VolcanicLava => "Volcanic Lava (Severe Burn Damage)",
            ZonePhysicsPreset::LowGravity => "Low Gravity Arena (0.2G)",
            ZonePhysicsPreset::SpaceVacuum => "Deep Space Vacuum (Zero-G, Suffocation)",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ZonePhysicsPreset::StandardEarth => "◎",
            ZonePhysicsPreset::WaterSwimming => "≋",
            ZonePhysicsPreset::MuddySwamp => "≈",
            ZonePhysicsPreset::ToxicSlime => "☣",
            ZonePhysicsPreset::VolcanicLava => "▲",
            ZonePhysicsPreset::LowGravity => "◇",
            ZonePhysicsPreset::SpaceVacuum => "◌",
        }
    }
}

/// Deconstructed environmental and physics profile for a map zone
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneProfile {
    pub actor_tag: String,
    pub class_name: String,
    pub location: glam::Vec3,
    pub zone_name: String,
    pub is_sky_zone: bool,
    pub is_water_zone: bool,
    pub gravity: glam::Vec3,
    pub velocity: glam::Vec3,
    pub fluid_friction: f32,
    pub damage_per_sec: i32,
    pub damage_type: String,
    pub ambient_brightness: u8,
    pub ambient_hue: u8,
    pub ambient_saturation: u8,
}

/// Positional and ambient sound emitter profile in the level
#[derive(Debug, Clone, PartialEq)]
pub struct SoundEmitterProfile {
    pub actor_tag: String,
    pub actor_name: String,
    pub class_name: String,
    pub location: glam::Vec3,
    pub sound_name: String,
    pub sound_radius_units: f32, // SoundRadius * 40.0 uu
    pub sound_radius_byte: u8,
    pub sound_volume_byte: u8,
    pub sound_pitch_byte: u8,
    pub sound_volume_pct: f32,   // SoundVolume / 255.0
    pub sound_pitch_ratio: f32,  // SoundPitch / 64.0
    pub is_ambient: bool,
}

/// Dynamic acoustic raycasting and occlusion evaluation result
#[derive(Debug, Clone, PartialEq)]
pub struct AcousticOcclusionResult {
    pub emitter_tag: String,
    pub emitter_location: glam::Vec3,
    pub distance: f32,
    pub is_in_range: bool,
    pub direct_volume_db: f32,
    pub walls_occluding_count: usize,
    pub occlusion_attenuation_db: f32,
    pub final_effective_volume_db: f32,
    pub low_pass_cutoff_hz: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AcousticListenerSource {
    #[default]
    CameraPosition,
    SelectedActor,
}

/// Settings and real-time visualization toggles for 3D acoustics
#[derive(Debug, Clone, PartialEq)]
pub struct AcousticStudioSettings {
    pub enabled: bool,
    pub show_radii_spheres: bool,
    pub show_listener_rays: bool,
    pub show_occlusion_cutoffs: bool,
    pub listener_source: AcousticListenerSource,
}

impl Default for AcousticStudioSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_radii_spheres: true,
            show_listener_rays: true,
            show_occlusion_cutoffs: true,
            listener_source: AcousticListenerSource::CameraPosition,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct ToastNotification {
    pub id: u64,
    pub message: String,
    pub kind: ToastKind,
    pub created_at: Instant,
    pub duration: Duration,
}



/// Destructive document action awaiting user confirmation when `map_is_dirty` is true (`UX-08` & `UX-12`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingDirtyAction {
    ExitApp,
    NewMap,
    CloseDocumentTab(usize),
    OpenUnrDialog,
    OpenT3dDialog,
    OpenMapPath(std::path::PathBuf),
}

/// Target engine compatibility profile (`Deliverable D` & `Deliverable E.1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TargetEngine {
    #[default]
    Ut99Standard,
    Unreal227,
}

impl TargetEngine {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ut99Standard => "UT99 Standard (v436/v469)",
            Self::Unreal227 => "Unreal 227 / UnrealEd 2.1",
        }
    }
}

/// Keyboard shortcut profile (`Deliverable E.2` & `Deliverable G`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum KeymapProfile {
    #[default]
    Modern,
    ClassicUnrealEd2,
}

impl KeymapProfile {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Modern => "Modern (Ctrl+S Save, Ctrl+A Select All)",
            Self::ClassicUnrealEd2 => "Classic UnrealEd 2.0 (Ctrl+A Add, Ctrl+S Subtract, Ctrl+L Save)",
        }
    }
}

/// Unified tabbed browser mode replicating and extending UnrealEd 2.0's docked `Master Browser` (`Deliverable B Categoria A.1` & `H.1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MasterBrowserTab {
    #[default]
    Textures,
    ActorClasses,
    Groups,
    Sounds,
    Music,
    Meshes,
    UnrealScript,
    SmartPrefabs,
}

impl MasterBrowserTab {
    pub fn all() -> &'static [MasterBrowserTab] {
        &[
            Self::Textures,
            Self::ActorClasses,
            Self::Groups,
            Self::Sounds,
            Self::Music,
            Self::Meshes,
            Self::UnrealScript,
            Self::SmartPrefabs,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Textures => "🖼 Textures",
            Self::ActorClasses => "🎭 Classes",
            Self::Groups => "🗂 Groups",
            Self::Sounds => "🔊 Sounds",
            Self::Music => "🎵 Music",
            Self::Meshes => "🗿 Meshes",
            Self::UnrealScript => "📜 Scripts",
            Self::SmartPrefabs => "🧩 Smart Prefabs",
        }
    }
}

/// 2D Viewport reference background display mode (`View -> Background Image -> Center / Tile / Stretch`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackgroundDisplayMode {
    #[default]
    Center,
    Tile,
    Stretch,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WarpZonePair {
    pub source_name: String,
    pub source_tag: String,
    pub source_location: Vec3,
    pub target_name: String,
    pub target_tag: String,
    pub target_location: Vec3,
    pub distance: f32,
    pub is_paired: bool,
}

/// Connection status for WarpZone portals in the map topology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarpConnectionStatus {
    MatchedBiDirectional,
    LoopChain,
    Unidirectional,
    OrphanBroken,
}

impl WarpConnectionStatus {
    pub fn label(&self) -> &'static str {
        match self {
            WarpConnectionStatus::MatchedBiDirectional => "Matched Bi-directional (A <-> B)",
            WarpConnectionStatus::LoopChain => "Loop / Ring Chain (A -> B -> ... -> A)",
            WarpConnectionStatus::Unidirectional => "Unidirectional One-Way (A -> B)",
            WarpConnectionStatus::OrphanBroken => "Orphan / Broken Destination",
        }
    }

    pub fn badge_color(&self) -> [f32; 4] {
        match self {
            WarpConnectionStatus::MatchedBiDirectional => [0.15, 0.85, 0.45, 1.0], // Emerald Green
            WarpConnectionStatus::LoopChain => [0.75, 0.35, 1.00, 1.0],           // Cosmic Purple
            WarpConnectionStatus::Unidirectional => [1.00, 0.75, 0.20, 1.0],       // Amber
            WarpConnectionStatus::OrphanBroken => [1.00, 0.25, 0.25, 1.0],         // Scarlet Alert
        }
    }
}

/// Comprehensive topological and metric entry for WarpZone portals
#[derive(Debug, Clone, PartialEq)]
pub struct WarpZoneTopologyEntry {
    pub actor_index: usize,
    pub name: String,
    pub tag: String,
    pub this_tag: String,
    pub other_side_url: String,
    pub location: Vec3,
    pub rotation: (i32, i32, i32),
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub roll_deg: f32,
    pub forward_vector: Vec3,

    // Resolved connection
    pub status: WarpConnectionStatus,
    pub target_index: Option<usize>,
    pub target_name: Option<String>,
    pub target_tag: Option<String>,
    pub target_location: Option<Vec3>,
    pub target_rotation: Option<(i32, i32, i32)>,
    pub target_yaw_deg: Option<f32>,
    pub target_forward_vector: Option<Vec3>,

    // Spatial & Non-Euclidean Metrics
    pub delta_translation: Option<Vec3>,
    pub distance_uu: Option<f32>,
    pub delta_yaw_deg: Option<f32>,
    pub delta_pitch_deg: Option<f32>,
    pub delta_roll_deg: Option<f32>,
    pub is_seamless_mirror: bool,

    // Geometry alignment
    pub has_adjacent_portal_surface: bool,
    pub nearest_portal_surface_distance: f32,
}

/// Studio settings and visualizer parameters for WarpZone non-euclidean portals
#[derive(Debug, Clone, PartialEq)]
pub struct WarpZoneStudioSettings {
    pub show_portal_camera_frustum: bool,
    pub show_portal_normal_vectors: bool,
    pub show_connecting_helices: bool,
    pub selected_warp_index: Option<usize>,
    pub pair_generation_distance: f32,
    pub pair_generation_axis: usize, // 0 = X, 1 = Y, 2 = Z
}

impl Default for WarpZoneStudioSettings {
    fn default() -> Self {
        Self {
            show_portal_camera_frustum: true,
            show_portal_normal_vectors: true,
            show_connecting_helices: true,
            selected_warp_index: None,
            pair_generation_distance: 1024.0,
            pair_generation_axis: 0,
        }
    }
}

/// Canonical tournament lighting presets with calibrated hue, saturation, brightness and radius
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TournamentLightPreset {
    FacingWorldsMidnight, // Deep space azure
    Deck16ToxicSlime,     // Acid green
    PhobosCyberRed,       // High-contrast crimson
    LavaCoreAmber,        // Warm volcanic amber
    MorpheusHighNoon,     // Solar golden yellow
    LiandriPureWhite,     // Industrial crisp neutral white
}

impl TournamentLightPreset {
    pub fn label(&self) -> &'static str {
        match self {
            TournamentLightPreset::FacingWorldsMidnight => "Facing Worlds Midnight (Deep Azure)",
            TournamentLightPreset::Deck16ToxicSlime => "Deck 16 Slime (Toxic Emerald)",
            TournamentLightPreset::PhobosCyberRed => "Phobos Moon (Cyberpunk Crimson)",
            TournamentLightPreset::LavaCoreAmber => "Lava Core (Volcanic Amber)",
            TournamentLightPreset::MorpheusHighNoon => "Morpheus Skyscraper (Warm Solar)",
            TournamentLightPreset::LiandriPureWhite => "Liandri Industrial (Pure Neutral White)",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TournamentLightPreset::FacingWorldsMidnight => "◈",
            TournamentLightPreset::Deck16ToxicSlime => "◆",
            TournamentLightPreset::PhobosCyberRed => "▲",
            TournamentLightPreset::LavaCoreAmber => "▼",
            TournamentLightPreset::MorpheusHighNoon => "☀",
            TournamentLightPreset::LiandriPureWhite => "☼",
        }
    }

    pub fn values(&self) -> (u8, u8, u8, u8) {
        // (hue, saturation, brightness, radius)
        match self {
            TournamentLightPreset::FacingWorldsMidnight => (160, 35, 190, 48),
            TournamentLightPreset::Deck16ToxicSlime => (75, 25, 180, 40),
            TournamentLightPreset::PhobosCyberRed => (0, 15, 210, 54),
            TournamentLightPreset::LavaCoreAmber => (25, 20, 240, 60),
            TournamentLightPreset::MorpheusHighNoon => (40, 210, 220, 64),
            TournamentLightPreset::LiandriPureWhite => (0, 255, 170, 48),
        }
    }

    pub fn preview_rgb(&self) -> [f32; 3] {
        let (h, s, v, _) = self.values();
        let col = ut99_bsp::ut99_light_to_rgb(h, s, v);
        [col.x, col.y, col.z]
    }
}

/// Photometric audit and coverage statistics evaluated across all level lightmaps
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LuxmeterCoverageStats {
    pub total_surfaces: usize,
    pub lit_surfaces: usize,
    pub total_luxels: usize,
    pub under_lit_count: usize, // < 15 Lux (black/shadow hazard)
    pub shadow_count: usize,    // 15 - 50 Lux (tactical darkness)
    pub tournament_ideal_count: usize, // 50 - 100 Lux (competitive sweet spot)
    pub bright_count: usize,    // 100 - 180 Lux (well-lit)
    pub overbright_count: usize,// > 180 Lux (glare risk)
    pub average_lux: f32,
}

/// Canonical volumetric fog and atmospheric depth presets for UE1 zones
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FogPresetId {
    MistyMorningSwamp,
    DeepOceanicWater,
    ToxicAcidHaze,
    VolcanicSulfurAsh,
    CyberVoidFog,
    DenseCryptCold,
    ClearTournamentAtmosphere,
}

impl FogPresetId {
    pub fn label(&self) -> &'static str {
        match self {
            FogPresetId::MistyMorningSwamp => "Misty Morning Swamp (Humid Bog)",
            FogPresetId::DeepOceanicWater => "Deep Oceanic Water (WaterZone)",
            FogPresetId::ToxicAcidHaze => "Toxic Acid Haze (SlimeZone)",
            FogPresetId::VolcanicSulfurAsh => "Volcanic Sulfur Ash (LavaZone)",
            FogPresetId::CyberVoidFog => "Cyber Void Fog (Cosmic Space)",
            FogPresetId::DenseCryptCold => "Dense Crypt Cold (Subterranean)",
            FogPresetId::ClearTournamentAtmosphere => "Clear Tournament (Competitive)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            FogPresetId::MistyMorningSwamp => "Dense humid bog atmosphere with restricted sightlines and olive hue.",
            FogPresetId::DeepOceanicWater => "Submerged murky water zone with rapid distance attenuation and deep azure tint.",
            FogPresetId::ToxicAcidHaze => "Corrosive chemical vapor with pungent emerald glow and high obscurity.",
            FogPresetId::VolcanicSulfurAsh => "Smoldering magma fumes and glowing orange embers with heavy particle density.",
            FogPresetId::CyberVoidFog => "Expansive cosmic nebula with subtle purple horizon fade for orbital arenas.",
            FogPresetId::DenseCryptCold => "Chilly medieval subterranean dungeon atmosphere with cold slate grey mist.",
            FogPresetId::ClearTournamentAtmosphere => "Maximum competitive visibility with subtle depth cueing and crisp outlines.",
        }
    }

    pub fn color(&self) -> [f32; 4] {
        match self {
            FogPresetId::MistyMorningSwamp => [0.35, 0.42, 0.28, 1.0],
            FogPresetId::DeepOceanicWater => [0.05, 0.25, 0.55, 1.0],
            FogPresetId::ToxicAcidHaze => [0.20, 0.60, 0.10, 1.0],
            FogPresetId::VolcanicSulfurAsh => [0.65, 0.25, 0.08, 1.0],
            FogPresetId::CyberVoidFog => [0.18, 0.08, 0.35, 1.0],
            FogPresetId::DenseCryptCold => [0.15, 0.17, 0.22, 1.0],
            FogPresetId::ClearTournamentAtmosphere => [0.08, 0.10, 0.14, 1.0],
        }
    }

    pub fn distance(&self) -> f32 {
        match self {
            FogPresetId::MistyMorningSwamp => 2500.0,
            FogPresetId::DeepOceanicWater => 900.0,
            FogPresetId::ToxicAcidHaze => 1200.0,
            FogPresetId::VolcanicSulfurAsh => 1600.0,
            FogPresetId::CyberVoidFog => 5500.0,
            FogPresetId::DenseCryptCold => 1800.0,
            FogPresetId::ClearTournamentAtmosphere => 12000.0,
        }
    }

    pub fn start(&self) -> f32 {
        match self {
            FogPresetId::MistyMorningSwamp => 350.0,
            FogPresetId::DeepOceanicWater => 50.0,
            FogPresetId::ToxicAcidHaze => 80.0,
            FogPresetId::VolcanicSulfurAsh => 120.0,
            FogPresetId::CyberVoidFog => 800.0,
            FogPresetId::DenseCryptCold => 150.0,
            FogPresetId::ClearTournamentAtmosphere => 3000.0,
        }
    }

    pub fn density(&self) -> f32 {
        match self {
            FogPresetId::MistyMorningSwamp => 1.0,
            FogPresetId::DeepOceanicWater => 1.5,
            FogPresetId::ToxicAcidHaze => 1.2,
            FogPresetId::VolcanicSulfurAsh => 1.1,
            FogPresetId::CyberVoidFog => 0.9,
            FogPresetId::DenseCryptCold => 1.0,
            FogPresetId::ClearTournamentAtmosphere => 0.8,
        }
    }

    pub fn all() -> &'static [FogPresetId] {
        &[
            FogPresetId::MistyMorningSwamp,
            FogPresetId::DeepOceanicWater,
            FogPresetId::ToxicAcidHaze,
            FogPresetId::VolcanicSulfurAsh,
            FogPresetId::CyberVoidFog,
            FogPresetId::DenseCryptCold,
            FogPresetId::ClearTournamentAtmosphere,
        ]
    }
}

/// Extracted fog and attenuation properties for a specific zone in the level
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneFogProfile {
    pub actor_index: usize,
    pub actor_name: String,
    pub zone_name: String,
    pub zone_tag: String,
    pub class_name: String,
    pub location: Vec3,
    pub b_fog_zone: bool,
    pub fog_color: [f32; 4],
    pub fog_distance: f32,
    pub fog_start: f32,
    pub fog_density: f32,
}

impl ZoneFogProfile {
    pub fn visibility_at(&self, distance: f32) -> f32 {
        if !self.b_fog_zone {
            return 1.0;
        }
        let span = (self.fog_distance - self.fog_start).max(1.0);
        let factor = ((distance - self.fog_start) / span).clamp(0.0, 1.0);
        (1.0 - factor * self.fog_density.clamp(0.1, 2.0)).clamp(0.0, 1.0)
    }
}

/// Target center for rendering concentric 3D fog distance shells
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FogShellTargetMode {
    ActiveCamera,
    SelectedZone,
}

/// Interactive studio settings for real-time zone fog profiling and viewport rendering
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneFogStudioSettings {
    pub show_distance_shells: bool,
    pub shell_target_mode: FogShellTargetMode,
    pub preview_in_viewport: bool,
    pub selected_zone_index: Option<usize>,
    pub active_preset: Option<FogPresetId>,
    pub custom_color: [f32; 3],
    pub custom_distance: f32,
    pub custom_start: f32,
    pub custom_density: f32,
}

impl Default for ZoneFogStudioSettings {
    fn default() -> Self {
        Self {
            show_distance_shells: true,
            shell_target_mode: FogShellTargetMode::ActiveCamera,
            preview_in_viewport: true,
            selected_zone_index: None,
            active_preset: Some(FogPresetId::MistyMorningSwamp),
            custom_color: [0.35, 0.42, 0.28],
            custom_distance: 2500.0,
            custom_start: 350.0,
            custom_density: 1.0,
        }
    }
}

/// Interactive studio and scrubbable timeline settings for Dispatcher sequences
#[derive(Debug, Clone, PartialEq)]
pub struct DispatcherStudioSettings {
    pub selected_dispatcher_name: Option<String>,
    pub timeline_time: f32,
    pub is_playing: bool,
    pub play_speed: f32,
    pub show_3d_event_splines: bool,
    pub loop_preview: bool,
}

impl Default for DispatcherStudioSettings {
    fn default() -> Self {
        Self {
            selected_dispatcher_name: None,
            timeline_time: 0.0,
            is_playing: false,
            play_speed: 1.0,
            show_3d_event_splines: true,
            loop_preview: false,
        }
    }
}

/// Interactive 3D studio and character animation scrubber settings
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletalStudioSettings {
    pub selected_mesh_index: usize,
    pub active_animation: Option<String>,
    pub timeline_time: f32,
    pub is_playing: bool,
    pub play_speed: f32,
    pub loop_preview: bool,
    pub show_bones_wireframe: bool,
    pub show_socket_helpers: bool,
    pub selected_socket_name: Option<String>,
}

impl Default for SkeletalStudioSettings {
    fn default() -> Self {
        Self {
            selected_mesh_index: 0,
            active_animation: Some("Run".to_string()),
            timeline_time: 0.0,
            is_playing: false,
            play_speed: 1.0,
            loop_preview: true,
            show_bones_wireframe: true,
            show_socket_helpers: true,
            selected_socket_name: Some("WeaponPoint_R".to_string()),
        }
    }
}

/// Interactive 3D particle emitter simulator settings
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleStudioSettings {
    pub selected_emitter_index: usize,
    pub is_simulating: bool,
    pub sim_speed: f32,
    pub show_emitter_bounds: bool,
    pub show_collision_plane: bool,
    pub auto_detect_floor: bool,
}

impl Default for ParticleStudioSettings {
    fn default() -> Self {
        Self {
            selected_emitter_index: 0,
            is_simulating: true,
            sim_speed: 1.0,
            show_emitter_bounds: true,
            show_collision_plane: true,
            auto_detect_floor: true,
        }
    }
}

/// Interactive Bot AI Navigation Graph, Reachability Network & JumpPad Studio settings
#[derive(Debug, Clone, PartialEq)]
pub struct BotNavStudioSettings {
    pub selected_node_id: Option<usize>,
    pub show_jumppad_arcs: bool,
    pub show_lift_linkages: bool,
    pub show_tactical_cover: bool,
    pub filter_node_type: Option<String>,
}

impl Default for BotNavStudioSettings {
    fn default() -> Self {
        Self {
            selected_node_id: None,
            show_jumppad_arcs: true,
            show_lift_linkages: true,
            show_tactical_cover: true,
            filter_node_type: None,
        }
    }
}

/// Settings and parametric state for Procedural Architectural Prefab & Brush Macro Studio
#[derive(Debug, Clone, PartialEq)]
pub struct ArchitecturalStudioSettings {
    pub config: ut99_bsp::ArchitecturalPrefabConfig,
    pub show_preview: bool,
    pub wireframe_mode: bool,
}

impl Default for ArchitecturalStudioSettings {
    fn default() -> Self {
        Self {
            config: ut99_bsp::ArchitecturalPrefabConfig::default(),
            show_preview: true,
            wireframe_mode: true,
        }
    }
}

/// Settings and sculpting state for Terrain & Heightmap Mesh Sculptor Studio
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainStudioSettings {
    pub grid: ut99_bsp::TerrainGrid,
    pub sculpt_mode: ut99_bsp::TerrainSculptMode,
    pub brush_radius: f32,
    pub brush_strength: f32,
    pub brush_center: glam::Vec2,
    pub target_plateau_height: f32,
    pub show_preview: bool,
    pub wireframe_mode: bool,
}

impl Default for TerrainStudioSettings {
    fn default() -> Self {
        Self {
            grid: ut99_bsp::TerrainGrid::default(),
            sculpt_mode: ut99_bsp::TerrainSculptMode::Raise,
            brush_radius: 256.0,
            brush_strength: 32.0,
            brush_center: glam::Vec2::ZERO,
            target_plateau_height: 128.0,
            show_preview: true,
            wireframe_mode: true,
        }
    }
}

/// Settings for Static Mesh LOD & Convex Collision Hull Studio
#[derive(Debug, Clone, PartialEq)]
pub struct LodCollisionStudioSettings {
    pub target_lod_level: usize, // 0..3
    pub hull_type: ut99_bsp::CollisionHullType,
    pub hull_margin: f32,
    pub show_preview_hull: bool,
    pub show_preview_lod: bool,
    pub active_lod_chain: Option<ut99_bsp::MeshLodChain>,
}

impl Default for LodCollisionStudioSettings {
    fn default() -> Self {
        Self {
            target_lod_level: 0,
            hull_type: ut99_bsp::CollisionHullType::Kdop10,
            hull_margin: 4.0,
            show_preview_hull: true,
            show_preview_lod: true,
            active_lod_chain: None,
        }
    }
}

/// Settings and state for Cinematic Matinee Camera Path & Cutscene Sequencer Studio
#[derive(Debug, Clone, PartialEq)]
pub struct CinematicStudioSettings {
    pub active_track: ut99_bsp::CinematicCameraTrack,
    pub playhead_t: f32, // 0.0 .. 1.0
    pub is_playing: bool,
    pub playback_speed: f32, // 0.25 .. 2.0
    pub pilot_camera: bool,
    pub show_spline_path: bool,
    pub show_camera_frustum: bool,
    pub selected_keyframe_idx: Option<usize>,
}

impl Default for CinematicStudioSettings {
    fn default() -> Self {
        Self {
            active_track: ut99_bsp::CinematicCameraTrack::default(),
            playhead_t: 0.0,
            is_playing: false,
            playback_speed: 1.0,
            pilot_camera: false,
            show_spline_path: true,
            show_camera_frustum: true,
            selected_keyframe_idx: None,
        }
    }
}

/// Settings and state for Dynamic Light Effects & Animating Strobe/Spotlight Studio
#[derive(Debug, Clone, PartialEq)]
pub struct LightEffectsStudioSettings {
    pub selected_preset: ut99_bsp::DynamicLightPreset,
    pub preview_light: ut99_bsp::SceneLight,
    pub current_time_sec: f32,
    pub is_animating: bool,
    pub anim_speed: f32,
    pub show_animated_beams_viewport: bool,
    pub show_spotlight_cones_viewport: bool,
    pub waveform_samples: Vec<(f32, f32)>,
}

impl Default for LightEffectsStudioSettings {
    fn default() -> Self {
        let mut light = ut99_bsp::SceneLight::default();
        ut99_bsp::DynamicLightPreset::EerieFlickeringTorch.apply_to_light(&mut light);
        let samples = ut99_bsp::sample_light_waveform(&light, 2.0, 100);
        Self {
            selected_preset: ut99_bsp::DynamicLightPreset::EerieFlickeringTorch,
            preview_light: light,
            current_time_sec: 0.0,
            is_animating: true,
            anim_speed: 1.0,
            show_animated_beams_viewport: true,
            show_spotlight_cones_viewport: true,
            waveform_samples: samples,
        }
    }
}

/// Settings and state for Zone Portal Leak, Antiportal & PVS Occlusion Diagnostic Studio
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneOcclusionStudioSettings {
    pub selected_preset: ut99_bsp::ZoneOcclusionPreset,
    pub portal_sheet_width: f32,
    pub portal_sheet_height: f32,
    pub portal_sheet_normal: Vec3,
    pub antiportal_size: Vec3,
    pub show_pvs_matrix_grid: bool,
    pub show_leak_escape_beams: bool,
    pub show_antiportal_shadows: bool,
    pub selected_zone_idx: Option<usize>,
    pub last_audit_report: Option<ut99_bsp::ZoneOcclusionAudit>,
}

impl Default for ZoneOcclusionStudioSettings {
    fn default() -> Self {
        Self {
            selected_preset: ut99_bsp::ZoneOcclusionPreset::StandardDoorwayPortal,
            portal_sheet_width: 128.0,
            portal_sheet_height: 256.0,
            portal_sheet_normal: Vec3::X,
            antiportal_size: Vec3::new(256.0, 256.0, 256.0),
            show_pvs_matrix_grid: true,
            show_leak_escape_beams: true,
            show_antiportal_shadows: true,
            selected_zone_idx: None,
            last_audit_report: None,
        }
    }
}

/// Settings and state for Interactive Music Event & UMX Sequencing Studio
#[derive(Debug, Clone, PartialEq)]
pub struct MusicStudioSettings {
    pub selected_preset: ut99_bsp::MusicTrackPreset,
    pub current_track_info: ut99_bsp::TrackerModuleInfo,
    pub selected_section: u8,
    pub is_playing: bool,
    pub playback_time_sec: f32,
    pub volume: f32,
    pub crossfade_transition: ut99_bsp::MusicTransition,
    pub last_audit_report: Option<ut99_bsp::MusicAuditReport>,
    pub show_music_event_spheres_viewport: bool,
}

impl Default for MusicStudioSettings {
    fn default() -> Self {
        let preset = ut99_bsp::MusicTrackPreset::ForegoneDestruction;
        let track_info = preset.to_tracker_info();
        Self {
            selected_preset: preset,
            current_track_info: track_info,
            selected_section: 0,
            is_playing: false,
            playback_time_sec: 0.0,
            volume: 0.85,
            crossfade_transition: ut99_bsp::MusicTransition::Fade,
            last_audit_report: None,
            show_music_event_spheres_viewport: true,
        }
    }
}

/// Settings and state for Interactive Sun, Corona & Lens Flare Studio
#[derive(Debug, Clone, PartialEq)]
pub struct CoronaStudioSettings {
    pub selected_preset: ut99_bsp::CoronaPreset,
    pub preview_config: ut99_bsp::CoronaConfig,
    pub show_flares_viewport: bool,
    pub show_occlusion_rays_viewport: bool,
    pub simulate_camera_orbit: bool,
    pub orbit_angle_deg: f32,
}

impl Default for CoronaStudioSettings {
    fn default() -> Self {
        let preset = ut99_bsp::CoronaPreset::BlazingDesertSun;
        Self {
            selected_preset: preset,
            preview_config: preset.to_config(),
            show_flares_viewport: true,
            show_occlusion_rays_viewport: true,
            simulate_camera_orbit: false,
            orbit_angle_deg: 0.0,
        }
    }
}

/// Settings and state for Fluid Physics, Viscosity & Hazard Zone Studio
#[derive(Debug, Clone, PartialEq)]
pub struct FluidStudioSettings {
    pub selected_preset: ut99_bsp::FluidZonePreset,
    pub preview_physics: ut99_bsp::ZoneFluidPhysics,
    pub simulation_time: f32,
    pub is_simulating: bool,
    pub test_player_health: i32,
    pub show_current_streamers_viewport: bool,
    pub show_hazard_boxes_viewport: bool,
}

impl Default for FluidStudioSettings {
    fn default() -> Self {
        let preset = ut99_bsp::FluidZonePreset::CrystalLakeWater;
        Self {
            selected_preset: preset,
            preview_physics: preset.to_physics(),
            simulation_time: 0.0,
            is_simulating: true,
            test_player_health: 100,
            show_current_streamers_viewport: true,
            show_hazard_boxes_viewport: true,
        }
    }
}

/// Ballistic test probe marker positioned in level
#[derive(Debug, Clone, PartialEq)]
pub struct BallisticProbeData {
    pub id: usize,
    pub name: String,
    pub location: glam::Vec3,
    pub pitch_deg: f32,
    pub yaw_deg: f32,
    pub preset: ut99_bsp::WeaponCombatPreset,
}

/// Settings and state for Interactive Bot Combat Trajectory & Weapon Sightline Studio
#[derive(Debug, Clone, PartialEq)]
pub struct BallisticsStudioSettings {
    pub selected_preset: ut99_bsp::WeaponCombatPreset,
    pub preview_projectile: ut99_bsp::BallisticProjectile,
    pub pitch_deg: f32,
    pub yaw_deg: f32,
    pub speed_override: Option<f32>,
    pub max_simulation_time: f32,
    pub show_trajectories_viewport: bool,
    pub show_blast_spheres_viewport: bool,
    pub show_bounce_normals_viewport: bool,
    pub active_trajectory: Vec<ut99_bsp::TrajectoryPoint>,
    pub probes: Vec<BallisticProbeData>,
}

impl Default for BallisticsStudioSettings {
    fn default() -> Self {
        let preset = ut99_bsp::WeaponCombatPreset::EightballGrenadeArc;
        Self {
            selected_preset: preset,
            preview_projectile: preset.to_projectile(),
            pitch_deg: 25.0,
            yaw_deg: 0.0,
            speed_override: None,
            max_simulation_time: 3.0,
            show_trajectories_viewport: true,
            show_blast_spheres_viewport: true,
            show_bounce_normals_viewport: true,
            active_trajectory: Vec::new(),
            probes: Vec::new(),
        }
    }
}

/// Settings and state for Interactive Teleporter Destination & Network Link Studio
#[derive(Debug, Clone, PartialEq)]
pub struct TeleporterStudioSettings {
    pub selected_node_id: Option<usize>,
    pub auto_pair_prefix: String,
    pub auto_pair_offset: f32, // default 1000.0 uu
    pub show_portal_discs_viewport: bool,
    pub show_network_lines_viewport: bool,
    pub show_exit_vectors_viewport: bool,
    pub test_incoming_speed: f32,
    pub filter_query: String,
}

impl Default for TeleporterStudioSettings {
    fn default() -> Self {
        Self {
            selected_node_id: None,
            auto_pair_prefix: "BasePortal".to_string(),
            auto_pair_offset: 1200.0,
            show_portal_discs_viewport: true,
            show_network_lines_viewport: true,
            show_exit_vectors_viewport: true,
            test_incoming_speed: 400.0,
            filter_query: String::new(),
        }
    }
}

/// Translocator test probe marker positioned in level
#[derive(Debug, Clone, PartialEq)]
pub struct TranslocatorProbeData {
    pub id: usize,
    pub name: String,
    pub location: glam::Vec3,
    pub pitch_deg: f32,
    pub yaw_deg: f32,
    pub preset: ut99_bsp::TranslocatorPreset,
}

/// Settings and state for Interactive Translocator Trajectory & Teleport Beacon Studio
#[derive(Debug, Clone, PartialEq)]
pub struct TranslocatorStudioSettings {
    pub selected_preset: ut99_bsp::TranslocatorPreset,
    pub active_config: ut99_bsp::TranslocatorTossConfig,
    pub pitch_deg: f32,
    pub yaw_deg: f32,
    pub toss_speed: f32,
    pub elasticity: f32,
    pub friction: f32,
    pub show_arcs_viewport: bool,
    pub show_beacons_viewport: bool,
    pub show_bounce_rings_viewport: bool,
    pub active_simulation: Option<ut99_bsp::TranslocatorFlightSimulation>,
    pub probes: Vec<TranslocatorProbeData>,
}

impl Default for TranslocatorStudioSettings {
    fn default() -> Self {
        let preset = ut99_bsp::TranslocatorPreset::MaxDistanceArc45;
        let cfg = preset.to_config();
        Self {
            selected_preset: preset,
            active_config: cfg.clone(),
            pitch_deg: cfg.pitch_deg,
            yaw_deg: cfg.yaw_deg,
            toss_speed: cfg.toss_speed,
            elasticity: cfg.elasticity,
            friction: cfg.friction,
            show_arcs_viewport: true,
            show_beacons_viewport: true,
            show_bounce_rings_viewport: true,
            active_simulation: None,
            probes: Vec::new(),
        }
    }
}

/// Settings and state for Interactive LiftCenter, LiftExit & Elevator Pathing Studio
#[derive(Debug, Clone, PartialEq)]
pub struct LiftPathingStudioSettings {
    pub selected_elevator_idx: usize,
    pub playback_time: f32,
    pub is_playing: bool,
    pub playback_speed: f32,
    pub auto_loop: bool,
    pub show_visualizer_in_viewport: bool,
    pub show_bot_avatar_viewport: bool,
    pub show_shaft_lines_viewport: bool,
    pub show_keyframe_floors_viewport: bool,
    pub wizard_platform_radius: f32,
    pub wizard_exit_yaw_deg: f32,
    pub cached_audit: Option<ut99_bsp::ElevatorAuditSummary>,
    pub cached_simulation: Vec<ut99_bsp::LiftSimulationStep>,
}

impl Default for LiftPathingStudioSettings {
    fn default() -> Self {
        Self {
            selected_elevator_idx: 0,
            playback_time: 0.0,
            is_playing: true,
            playback_speed: 1.0,
            auto_loop: true,
            show_visualizer_in_viewport: true,
            show_bot_avatar_viewport: true,
            show_shaft_lines_viewport: true,
            show_keyframe_floors_viewport: true,
            wizard_platform_radius: 128.0,
            wizard_exit_yaw_deg: 0.0,
            cached_audit: None,
            cached_simulation: Vec::new(),
        }
    }
}

/// Settings and state for Interactive Dynamic Zone Water Surface & Wave Reflection Studio
#[derive(Debug, Clone, PartialEq)]
pub struct WaterSurfaceStudioSettings {
    pub selected_preset: ut99_bsp::WaterPreset,
    pub active_config: ut99_bsp::WaterSurfaceConfig,
    pub simulation: ut99_bsp::WaterHeightfieldSimulation,
    pub is_playing: bool,
    pub simulation_speed: f32,
    pub ripple_brush_radius: usize,
    pub ripple_brush_amplitude: f32,
    pub sheet_center: glam::Vec3,
    pub sheet_size_x: f32,
    pub sheet_size_y: f32,
    pub subdivisions_x: usize,
    pub subdivisions_y: usize,
    pub sheet_texture_name: String,
    pub show_wave_wireframe_viewport: bool,
    pub show_caustics_viewport: bool,
    pub show_flow_vectors_viewport: bool,
    pub show_boundary_box_viewport: bool,
    pub wave_displacement_scale: f32,
}

impl Default for WaterSurfaceStudioSettings {
    fn default() -> Self {
        let preset = ut99_bsp::WaterPreset::CrystalClearPool;
        let config = preset.to_config();
        Self {
            selected_preset: preset,
            active_config: config,
            simulation: ut99_bsp::WaterHeightfieldSimulation::new(32, 32, 16.0),
            is_playing: true,
            simulation_speed: 1.0,
            ripple_brush_radius: 2,
            ripple_brush_amplitude: 15.0,
            sheet_center: glam::Vec3::ZERO,
            sheet_size_x: 512.0,
            sheet_size_y: 512.0,
            subdivisions_x: 8,
            subdivisions_y: 8,
            sheet_texture_name: "GenFluid.Water.WetWater1".to_string(),
            show_wave_wireframe_viewport: true,
            show_caustics_viewport: true,
            show_flow_vectors_viewport: true,
            show_boundary_box_viewport: true,
            wave_displacement_scale: 1.0,
        }
    }
}

/// Settings and state for Interactive Mover Encroachment, Door Sequencing & Crushing Trap Studio
#[derive(Debug, Clone, PartialEq)]
pub struct MoverStudioSettings {
    pub selected_mover_idx: usize,
    pub selected_template: ut99_bsp::MoverTemplateArchetype,
    pub active_config: ut99_bsp::MoverSequenceConfig,
    pub playback_time: f32,
    pub is_playing: bool,
    pub playback_speed: f32,
    pub auto_loop: bool,
    pub show_swept_bounds_viewport: bool,
    pub show_keyframe_ghosts_viewport: bool,
    pub show_animated_brush_viewport: bool,
    pub show_pinch_hazards_viewport: bool,
    pub show_trigger_radius_viewport: bool,
    pub bot_test_position: glam::Vec3,
    pub trigger_approach_distance: f32,
    pub trigger_radius: f32,
    pub cached_hazards: Vec<ut99_bsp::MoverHazardDiagnostic>,
    pub cached_encroach_sim: Option<ut99_bsp::MoverEncroachSimResult>,
}

impl Default for MoverStudioSettings {
    fn default() -> Self {
        let template = ut99_bsp::MoverTemplateArchetype::AutomaticSlidingDoor;
        let config = template.to_config(glam::Vec3::ZERO);
        Self {
            selected_mover_idx: 0,
            selected_template: template,
            active_config: config,
            playback_time: 0.0,
            is_playing: true,
            playback_speed: 1.0,
            auto_loop: true,
            show_swept_bounds_viewport: true,
            show_keyframe_ghosts_viewport: true,
            show_animated_brush_viewport: true,
            show_pinch_hazards_viewport: true,
            show_trigger_radius_viewport: true,
            bot_test_position: glam::Vec3::new(64.0, 0.0, 0.0),
            trigger_approach_distance: 128.0,
            trigger_radius: 96.0,
            cached_hazards: Vec::new(),
            cached_encroach_sim: None,
        }
    }
}

/// Settings and state for Interactive Team Base Objective, CTF Flag Run Route & Defense AI Studio
#[derive(Debug, Clone, PartialEq)]
pub struct TeamObjectiveStudioSettings {
    pub active_game_mode: ut99_bsp::GameObjectiveType,
    pub selected_preset: Option<ut99_bsp::TeamObjectivePreset>,
    pub selected_objective_idx: usize,
    pub show_route_ribbons_viewport: bool,
    pub show_defense_cones_viewport: bool,
    pub show_chokepoints_viewport: bool,
    pub show_flag_holograms_viewport: bool,
    pub detected_objectives: Vec<ut99_bsp::ObjectiveNode>,
    pub detected_defense_points: Vec<ut99_bsp::DefensePointData>,
    pub routes: Vec<ut99_bsp::FlagRoute>,
    pub asymmetry: Option<ut99_bsp::RouteAsymmetryAnalysis>,
    pub dom_triangle: Option<ut99_bsp::DominationTriangleAnalysis>,
    pub audit_issues: Vec<ut99_bsp::DefenseAuditIssue>,
}

impl Default for TeamObjectiveStudioSettings {
    fn default() -> Self {
        Self {
            active_game_mode: ut99_bsp::GameObjectiveType::CaptureTheFlag,
            selected_preset: Some(ut99_bsp::TeamObjectivePreset::FaceTowerSniperAlley),
            selected_objective_idx: 0,
            show_route_ribbons_viewport: true,
            show_defense_cones_viewport: true,
            show_chokepoints_viewport: true,
            show_flag_holograms_viewport: true,
            detected_objectives: Vec::new(),
            detected_defense_points: Vec::new(),
            routes: Vec::new(),
            asymmetry: None,
            dom_triangle: None,
            audit_issues: Vec::new(),
        }
    }
}

/// Settings and state for Interactive Destructible Architecture, Breakable Glass & Fracture Studio
#[derive(Debug, Clone, PartialEq)]
pub struct DestructibleStudioSettings {
    pub selected_archetype: ut99_bsp::DestructibleArchetype,
    pub selected_pattern: ut99_bsp::FracturePatternType,
    pub active_config: ut99_bsp::DestructibleConfig,
    pub simulation: ut99_bsp::DestructibleSimulation,
    pub show_fracture_pattern_viewport: bool,
    pub show_debris_cone_viewport: bool,
    pub show_bot_reachability_viewport: bool,
    pub cached_shards: Vec<ut99_bsp::ShardPolygon>,
    pub cached_hazards: Vec<ut99_bsp::DestructibleHazard>,
    pub is_animating: bool,
}

impl Default for DestructibleStudioSettings {
    fn default() -> Self {
        let arch = ut99_bsp::DestructibleArchetype::BreakableWindowGlass;
        let config = arch.to_config(glam::Vec3::new(0.0, 0.0, 64.0), glam::Vec2::new(128.0, 192.0));
        let shards = ut99_bsp::generate_fracture_shards(config.size, config.fracture_pattern, config.shard_count);
        let hazards = ut99_bsp::audit_destructible_hazards(&config);
        Self {
            selected_archetype: arch,
            selected_pattern: config.fracture_pattern,
            active_config: config,
            simulation: ut99_bsp::DestructibleSimulation::new(0.0),
            show_fracture_pattern_viewport: true,
            show_debris_cone_viewport: true,
            show_bot_reachability_viewport: true,
            cached_shards: shards,
            cached_hazards: hazards,
            is_animating: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KickerStudioSettings {
    pub selected_archetype: ut99_bsp::KickerArchetype,
    pub active_config: ut99_bsp::KickerConfig,
    pub trajectory_analysis: ut99_bsp::TrajectoryAnalysis,
    pub cached_hazards: Vec<ut99_bsp::KickerHazard>,
    pub show_trajectory_ribbon_viewport: bool,
    pub show_landing_target_viewport: bool,
    pub show_ceiling_clearance_viewport: bool,
    pub show_bot_reachability_viewport: bool,
    pub flight_scrubber_secs: f32,
    pub is_animating: bool,
    pub target_apex_clearance_z: f32,
}

impl Default for KickerStudioSettings {
    fn default() -> Self {
        let arch = ut99_bsp::KickerArchetype::FaceTowerPerchLob;
        let config = arch.to_config(glam::Vec3::new(0.0, 0.0, 64.0));
        let analysis = ut99_bsp::calculate_kicker_trajectory(&config);
        let hazards = ut99_bsp::audit_kicker_hazards(&config, &analysis);
        Self {
            selected_archetype: arch,
            active_config: config,
            trajectory_analysis: analysis,
            cached_hazards: hazards,
            show_trajectory_ribbon_viewport: true,
            show_landing_target_viewport: true,
            show_ceiling_clearance_viewport: true,
            show_bot_reachability_viewport: true,
            flight_scrubber_secs: 0.0,
            is_animating: true,
            target_apex_clearance_z: 250.0,
        }
    }
}

/// Settings and state for Interactive BSP Tree Quality, Node Cut Minimizer & Sliver Polygon Purging Studio
#[derive(Debug, Clone, PartialEq)]
pub struct BspQualityStudioSettings {
    pub quality_metrics: ut99_bsp::BspQualityMetrics,
    pub detected_slivers: Vec<ut99_bsp::SliverPolygon>,
    pub detected_coplanar: Vec<ut99_bsp::CoplanarOverlap>,
    pub cut_offenders: Vec<ut99_bsp::BrushCutOffender>,
    pub min_sliver_area: f32, // default: 2.0 uu²
    pub max_aspect_ratio: f32, // default: 40.0
    pub show_cut_heatmap_viewport: bool,
    pub show_sliver_markers_viewport: bool,
    pub show_coplanar_markers_viewport: bool,
    pub selected_sliver_index: Option<usize>,
    pub selected_coplanar_index: Option<usize>,
    pub selected_offender_index: Option<usize>,
}

impl Default for BspQualityStudioSettings {
    fn default() -> Self {
        Self {
            quality_metrics: ut99_bsp::BspQualityMetrics::default(),
            detected_slivers: Vec::new(),
            detected_coplanar: Vec::new(),
            cut_offenders: Vec::new(),
            min_sliver_area: 2.0,
            max_aspect_ratio: 40.0,
            show_cut_heatmap_viewport: true,
            show_sliver_markers_viewport: true,
            show_coplanar_markers_viewport: true,
            selected_sliver_index: None,
            selected_coplanar_index: None,
            selected_offender_index: None,
        }
    }
}

/// Settings and state for Interactive 2D Shape Loft, Curved Pipe & Architectural Molding Studio
#[derive(Debug, Clone, PartialEq)]
pub struct LoftStudioSettings {
    pub active_config: ut99_bsp::LoftConfig,
    pub custom_profile: ut99_bsp::Shape2D,
    pub path_keyframes: Vec<ut99_bsp::LoftPathKeyframe>,
    pub cached_frames: Vec<ut99_bsp::LoftFrame>,
    pub cached_hazards: Vec<ut99_bsp::LoftHazard>,
    pub show_tangent_frames_viewport: bool,
    pub show_profile_cross_sections_viewport: bool,
    pub show_wireframe_ghost_viewport: bool,
    pub selected_keyframe_idx: Option<usize>,
    pub cached_preview_brush: Option<ut99_bsp::Brush>,
}

impl Default for LoftStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::LoftConfig::default();
        let profile = config.archetype.to_profile(config.profile_radius);
        let keyframes = vec![
            ut99_bsp::LoftPathKeyframe::new(glam::Vec3::new(0.0, 0.0, 0.0)),
            ut99_bsp::LoftPathKeyframe::new(glam::Vec3::new(0.0, 0.0, 256.0)),
            ut99_bsp::LoftPathKeyframe::new(glam::Vec3::new(256.0, 0.0, 512.0)),
        ];
        let frames = ut99_bsp::generate_rmf_frames(&keyframes, config.tessellation_steps);
        let hazards = ut99_bsp::audit_loft_hazards(&keyframes, config.profile_radius);
        let preview = ut99_bsp::generate_loft_brush(&profile, &keyframes, &config);

        Self {
            active_config: config,
            custom_profile: profile,
            path_keyframes: keyframes,
            cached_frames: frames,
            cached_hazards: hazards,
            show_tangent_frames_viewport: true,
            show_profile_cross_sections_viewport: true,
            show_wireframe_ghost_viewport: true,
            selected_keyframe_idx: Some(0),
            cached_preview_brush: Some(preview),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StairwayStudioSettings {
    pub active_config: ut99_bsp::StairConfig,
    pub cached_stair_brush: Option<ut99_bsp::Brush>,
    pub cached_ramp_brush: Option<ut99_bsp::Brush>,
    pub cached_handrail_brush: Option<ut99_bsp::Brush>,
    pub cached_waypoints: Vec<ut99_bsp::StairBotWaypoint>,
    pub cached_metrics: ut99_bsp::StairMetrics,
    pub cached_hazards: Vec<ut99_bsp::StairHazard>,
    pub show_stair_ghost_viewport: bool,
    pub show_collision_ramp_ghost_viewport: bool,
    pub show_handrail_ghost_viewport: bool,
    pub show_bot_waypoints_viewport: bool,
}

impl Default for StairwayStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::StairConfig::default();
        let stair = ut99_bsp::generate_stair_brush(&config);
        let ramp = ut99_bsp::generate_smooth_collision_ramp_brush(&config);
        let handrail = if config.add_handrail_banister {
            Some(ut99_bsp::generate_handrail_brush(&config))
        } else {
            None
        };
        let waypoints = ut99_bsp::generate_bot_path_nodes(&config, glam::Vec3::ZERO);
        let metrics = ut99_bsp::calculate_stair_metrics(&config);
        let hazards = ut99_bsp::audit_stair_safety(&config);

        Self {
            active_config: config,
            cached_stair_brush: Some(stair),
            cached_ramp_brush: Some(ramp),
            cached_handrail_brush: handrail,
            cached_waypoints: waypoints,
            cached_metrics: metrics,
            cached_hazards: hazards,
            show_stair_ghost_viewport: true,
            show_collision_ramp_ghost_viewport: true,
            show_handrail_ghost_viewport: true,
            show_bot_waypoints_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScatterStudioSettings {
    pub active_config: ut99_bsp::ScatterConfig,
    pub preview_instances: Vec<ut99_bsp::ScatterInstance>,
    pub cached_metrics: ut99_bsp::ScatterMetrics,
    pub cached_hazards: Vec<ut99_bsp::ScatterHazard>,
    pub show_instances_viewport: bool,
    pub show_exclusion_discs_viewport: bool,
    pub show_surface_normals_viewport: bool,
    pub show_bot_exclusion_zones_viewport: bool,
}

impl Default for ScatterStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::ScatterConfig::default();
        let metrics = ut99_bsp::ScatterMetrics {
            total_instances: 0,
            valid_clearance_count: 0,
            blocked_clearance_count: 0,
            average_spacing: 0.0,
            sampled_surface_area: 0.0,
        };

        Self {
            active_config: config,
            preview_instances: Vec::new(),
            cached_metrics: metrics,
            cached_hazards: Vec::new(),
            show_instances_viewport: true,
            show_exclusion_discs_viewport: true,
            show_surface_normals_viewport: false,
            show_bot_exclusion_zones_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LightShaftStudioSettings {
    pub active_config: ut99_bsp::LightShaftConfig,
    pub cached_preview_brush: Option<ut99_bsp::Brush>,
    pub cached_dust_motes: Vec<ut99_bsp::DustMote>,
    pub cached_hazards: Vec<ut99_bsp::LightShaftHazard>,
    pub show_beam_ghost_viewport: bool,
    pub show_core_hotspot_viewport: bool,
    pub show_dust_motes_viewport: bool,
    pub show_aperture_frame_viewport: bool,
    pub show_floor_disc_viewport: bool,
}

impl Default for LightShaftStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::LightShaftConfig::default();
        let brush = ut99_bsp::generate_light_shaft_brush(&config);
        let motes = ut99_bsp::generate_dust_motes(&config, 35);
        let hazards = ut99_bsp::audit_light_shaft_safety(&config);

        Self {
            active_config: config,
            cached_preview_brush: Some(brush),
            cached_dust_motes: motes,
            cached_hazards: hazards,
            show_beam_ghost_viewport: true,
            show_core_hotspot_viewport: true,
            show_dust_motes_viewport: true,
            show_aperture_frame_viewport: true,
            show_floor_disc_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackStudioSettings {
    pub active_config: ut99_bsp::TrackConfig,
    pub cached_rails_brush: Option<ut99_bsp::Brush>,
    pub cached_ties_brush: Option<ut99_bsp::Brush>,
    pub cached_keyframes: Vec<ut99_bsp::MoverKeyframeRecord>,
    pub cached_waypoints: Vec<ut99_bsp::StationWaypoint>,
    pub cached_hazards: Vec<ut99_bsp::TrackHazard>,
    pub timeline_t: f32, // 0.0 to 1.0 animation playhead
    pub is_animating: bool,
    pub show_rails_viewport: bool,
    pub show_ties_viewport: bool,
    pub show_keyframes_viewport: bool,
    pub show_animated_cart_viewport: bool,
    pub show_banking_frames_viewport: bool,
    pub show_station_waypoints_viewport: bool,
}

impl Default for TrackStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::TrackConfig::default();
        let (rails, ties) = ut99_bsp::generate_track_brushes(&config);
        let keyframes = ut99_bsp::bake_mover_keyframes(&config);
        let waypoints = ut99_bsp::generate_station_bot_waypoints(&config);
        let hazards = ut99_bsp::audit_track_safety(&config);

        Self {
            active_config: config,
            cached_rails_brush: Some(rails),
            cached_ties_brush: Some(ties),
            cached_keyframes: keyframes,
            cached_waypoints: waypoints,
            cached_hazards: hazards,
            timeline_t: 0.0,
            is_animating: false,
            show_rails_viewport: true,
            show_ties_viewport: true,
            show_keyframes_viewport: true,
            show_animated_cart_viewport: true,
            show_banking_frames_viewport: true,
            show_station_waypoints_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurveillanceStudioSettings {
    pub active_config: ut99_bsp::SurveillanceConfig,
    pub cached_monitor_brush: Option<ut99_bsp::Brush>,
    pub cached_frustum: Option<ut99_bsp::CameraFrustum>,
    pub cached_hazards: Vec<ut99_bsp::SurveillanceHazard>,
    pub simulated_time_seconds: f32,
    pub is_patrolling: bool,
    pub show_camera_frustum_viewport: bool,
    pub show_patrol_sweep_viewport: bool,
    pub show_sightline_target_viewport: bool,
    pub show_monitor_box_viewport: bool,
}

impl Default for SurveillanceStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::SurveillanceConfig::default();
        let brush = ut99_bsp::generate_monitor_brush(&config);
        let frustum = ut99_bsp::calculate_camera_frustum(
            &config.camera_mount,
            config.camera_mount.base_yaw_deg,
            config.camera_mount.base_pitch_deg,
            config.monitor.aspect_ratio_w as f32 / config.monitor.aspect_ratio_h.max(1) as f32,
        );
        let hazards = ut99_bsp::audit_surveillance_safety(&config, &[]);

        Self {
            active_config: config,
            cached_monitor_brush: Some(brush),
            cached_frustum: Some(frustum),
            cached_hazards: hazards,
            simulated_time_seconds: 0.0,
            is_patrolling: true,
            show_camera_frustum_viewport: true,
            show_patrol_sweep_viewport: true,
            show_sightline_target_viewport: true,
            show_monitor_box_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaserGridStudioSettings {
    pub active_config: ut99_bsp::LaserGridConfig,
    pub cached_beam_brush: Option<ut99_bsp::Brush>,
    pub cached_diode_brush: Option<ut99_bsp::Brush>,
    pub cached_beams: Vec<ut99_bsp::LaserBeamData>,
    pub cached_trigger_specs: Vec<ut99_bsp::BeamTriggerSpec>,
    pub cached_hazards: Vec<ut99_bsp::LaserHazard>,
    pub simulated_time_seconds: f32,
    pub is_pulsing: bool,
    pub show_beams_viewport: bool,
    pub show_diodes_viewport: bool,
    pub show_triggers_viewport: bool,
    pub show_sweep_arcs_viewport: bool,
}

impl Default for LaserGridStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::LaserGridConfig::default();
        let beams = ut99_bsp::generate_laser_beams(&config, 0.0);
        let (beam_b, diode_b) = ut99_bsp::generate_laser_brush_geometry(&config, &beams);
        let triggers = ut99_bsp::generate_beam_trigger_specs(&config, &beams);
        let hazards = ut99_bsp::audit_laser_grid_safety(&config, &beams, &[]);

        Self {
            active_config: config,
            cached_beam_brush: Some(beam_b),
            cached_diode_brush: Some(diode_b),
            cached_beams: beams,
            cached_trigger_specs: triggers,
            cached_hazards: hazards,
            simulated_time_seconds: 0.0,
            is_pulsing: true,
            show_beams_viewport: true,
            show_diodes_viewport: true,
            show_triggers_viewport: true,
            show_sweep_arcs_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VentilatorStudioSettings {
    pub active_config: ut99_bsp::VentilatorConfig,
    pub cached_fan_brush: Option<ut99_bsp::Brush>,
    pub cached_housing_brush: Option<ut99_bsp::Brush>,
    pub cached_lift: ut99_bsp::AerodynamicLiftProfile,
    pub cached_streamlines: Vec<ut99_bsp::WindStreamline>,
    pub cached_hazards: Vec<ut99_bsp::VentilatorHazard>,
    pub simulated_time_seconds: f32,
    pub is_spinning: bool,
    pub show_fan_viewport: bool,
    pub show_housing_viewport: bool,
    pub show_streamlines_viewport: bool,
    pub show_hazard_zone_viewport: bool,
    pub show_hover_height_viewport: bool,
}

impl Default for VentilatorStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::VentilatorConfig::default();
        let (fan_b, house_b) = ut99_bsp::generate_ventilator_brush(&config, 0.0);
        let lift = ut99_bsp::calculate_aerodynamic_lift(&config);
        let streams = ut99_bsp::generate_wind_streamlines(&config, 0.0);
        let hazards = ut99_bsp::audit_ventilator_safety(&config, &[]);

        Self {
            active_config: config,
            cached_fan_brush: Some(fan_b),
            cached_housing_brush: Some(house_b),
            cached_lift: lift,
            cached_streamlines: streams,
            cached_hazards: hazards,
            simulated_time_seconds: 0.0,
            is_spinning: true,
            show_fan_viewport: true,
            show_housing_viewport: true,
            show_streamlines_viewport: true,
            show_hazard_zone_viewport: true,
            show_hover_height_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConveyorStudioSettings {
    pub active_config: ut99_bsp::ConveyorConfig,
    pub cached_deck_brush: Option<ut99_bsp::Brush>,
    pub cached_rollers_brush: Option<ut99_bsp::Brush>,
    pub cached_frame_brush: Option<ut99_bsp::Brush>,
    pub cached_metrics: ut99_bsp::ConveyorSyncMetrics,
    pub cached_waypoints: (ut99_bsp::ConveyorBotWaypointSpec, ut99_bsp::ConveyorBotWaypointSpec),
    pub cached_hazards: Vec<ut99_bsp::ConveyorHazard>,
    pub simulated_time_seconds: f32,
    pub is_running: bool,
    pub show_deck_viewport: bool,
    pub show_rollers_viewport: bool,
    pub show_guardrails_viewport: bool,
    pub show_velocity_vectors_viewport: bool,
    pub show_waypoints_viewport: bool,
}

impl Default for ConveyorStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::ConveyorConfig::default();
        let (deck_b, rollers_b, frame_b) = ut99_bsp::generate_conveyor_brushes(&config);
        let metrics = ut99_bsp::calculate_conveyor_sync_metrics(&config);
        let waypoints = ut99_bsp::generate_conveyor_bot_waypoints(&config);
        let hazards = ut99_bsp::audit_conveyor_safety(&config, &[]);

        Self {
            active_config: config,
            cached_deck_brush: Some(deck_b),
            cached_rollers_brush: Some(rollers_b),
            cached_frame_brush: Some(frame_b),
            cached_metrics: metrics,
            cached_waypoints: waypoints,
            cached_hazards: hazards,
            simulated_time_seconds: 0.0,
            is_running: true,
            show_deck_viewport: true,
            show_rollers_viewport: true,
            show_guardrails_viewport: true,
            show_velocity_vectors_viewport: true,
            show_waypoints_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TurretStudioSettings {
    pub active_config: ut99_bsp::TurretConfig,
    pub cached_mount_brush: Option<ut99_bsp::Brush>,
    pub cached_shield_brush: Option<ut99_bsp::Brush>,
    pub cached_metrics: ut99_bsp::TurretFrustumMetrics,
    pub cached_waypoints: Vec<ut99_bsp::TurretEvasionWaypoint>,
    pub cached_hazards: Vec<ut99_bsp::TurretHazard>,
    pub simulated_time_seconds: f32,
    pub is_tracking: bool,
    pub show_mount_viewport: bool,
    pub show_shield_viewport: bool,
    pub show_tracking_cone_viewport: bool,
    pub show_line_of_fire_viewport: bool,
    pub show_waypoints_viewport: bool,
}

impl Default for TurretStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::TurretConfig::default();
        let (mount_b, shield_b) = ut99_bsp::generate_turret_mount_brushes(&config);
        let metrics = ut99_bsp::calculate_turret_tracking_metrics(&config, 0.0);
        let waypoints = ut99_bsp::generate_turret_bot_waypoints(&config, &[]);
        let hazards = ut99_bsp::audit_turret_safety(&config, &[]);

        Self {
            active_config: config,
            cached_mount_brush: Some(mount_b),
            cached_shield_brush: Some(shield_b),
            cached_metrics: metrics,
            cached_waypoints: waypoints,
            cached_hazards: hazards,
            simulated_time_seconds: 0.0,
            is_tracking: true,
            show_mount_viewport: true,
            show_shield_viewport: true,
            show_tracking_cone_viewport: true,
            show_line_of_fire_viewport: true,
            show_waypoints_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoundscapeStudioSettings {
    pub active_config: ut99_bsp::SoundscapeConfig,
    pub cached_zone_brush: Option<ut99_bsp::Brush>,
    pub cached_metrics: ut99_bsp::SoundscapeMetrics,
    pub cached_hazards: Vec<ut99_bsp::SoundscapeHazard>,
    pub probe_listener_distance_uu: f32,
    pub cached_probe_sample: ut99_bsp::AcousticProbeSample,
    pub show_emitter_viewport: bool,
    pub show_inner_radius_viewport: bool,
    pub show_outer_radius_viewport: bool,
    pub show_soundwaves_viewport: bool,
    pub show_zone_bounds_viewport: bool,
}

impl Default for SoundscapeStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::SoundscapeConfig::default();
        let zone_b = ut99_bsp::generate_acoustic_volume_brush(&config, 384.0);
        let metrics = ut99_bsp::calculate_soundscape_metrics(&config);
        let hazards = ut99_bsp::audit_soundscape_safety(&config, &[]);
        let probe = ut99_bsp::sample_sound_propagation(&config, config.position + Vec3::new(256.0, 0.0, 0.0), &[]);

        Self {
            active_config: config,
            cached_zone_brush: Some(zone_b),
            cached_metrics: metrics,
            cached_hazards: hazards,
            probe_listener_distance_uu: 256.0,
            cached_probe_sample: probe,
            show_emitter_viewport: true,
            show_inner_radius_viewport: true,
            show_outer_radius_viewport: true,
            show_soundwaves_viewport: true,
            show_zone_bounds_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HazardChamberStudioSettings {
    pub active_config: ut99_bsp::HazardChamberConfig,
    pub cached_brushes: Option<ut99_bsp::HazardChamberBrushes>,
    pub cached_metrics: ut99_bsp::HazardChamberMetrics,
    pub cached_waypoints: ut99_bsp::HazardChamberBotWaypoints,
    pub cached_hazards: Vec<ut99_bsp::HazardChamberHazard>,
    pub simulated_time_seconds: f32,
    pub is_animating: bool,
    pub show_chamber_viewport: bool,
    pub show_doors_viewport: bool,
    pub show_window_viewport: bool,
    pub show_console_viewport: bool,
    pub show_gas_volume_viewport: bool,
    pub show_waypoints_viewport: bool,
}

impl Default for HazardChamberStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::HazardChamberConfig::default();
        let brushes = ut99_bsp::generate_hazard_chamber_brushes(&config);
        let metrics = ut99_bsp::calculate_chamber_metrics(&config);
        let waypoints = ut99_bsp::generate_chamber_bot_waypoints(&config);
        let hazards = ut99_bsp::audit_hazard_chamber_safety(&config, &[]);

        Self {
            active_config: config,
            cached_brushes: Some(brushes),
            cached_metrics: metrics,
            cached_waypoints: waypoints,
            cached_hazards: hazards,
            simulated_time_seconds: 0.0,
            is_animating: true,
            show_chamber_viewport: true,
            show_doors_viewport: true,
            show_window_viewport: true,
            show_console_viewport: true,
            show_gas_volume_viewport: true,
            show_waypoints_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipeConduitStudioSettings {
    pub active_config: ut99_bsp::PipeConduitConfig,
    pub cached_brushes: Option<ut99_bsp::PipeConduitBrushes>,
    pub cached_metrics: ut99_bsp::PipeConduitMetrics,
    pub cached_hazards: Vec<ut99_bsp::PipeConduitHazard>,
    pub simulated_flow_time_secs: f32,
    pub is_animating: bool,
    pub show_pipe_viewport: bool,
    pub show_elbow_viewport: bool,
    pub show_flanges_viewport: bool,
    pub show_valve_viewport: bool,
    pub show_steam_hazard_viewport: bool,
    pub show_flow_centerline_viewport: bool,
}

impl Default for PipeConduitStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::PipeConduitConfig::default();
        let brushes = ut99_bsp::generate_pipe_conduit_brushes(&config);
        let metrics = ut99_bsp::calculate_pipe_metrics(&config);
        let hazards = ut99_bsp::audit_pipe_conduit_safety(&config, &[]);

        Self {
            active_config: config,
            cached_brushes: Some(brushes),
            cached_metrics: metrics,
            cached_hazards: hazards,
            simulated_flow_time_secs: 0.0,
            is_animating: true,
            show_pipe_viewport: true,
            show_elbow_viewport: true,
            show_flanges_viewport: true,
            show_valve_viewport: true,
            show_steam_hazard_viewport: true,
            show_flow_centerline_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HologramStudioSettings {
    pub active_config: ut99_bsp::HologramConfig,
    pub cached_brushes: Option<ut99_bsp::HologramBrushes>,
    pub cached_metrics: ut99_bsp::HologramMetrics,
    pub cached_hazards: Vec<ut99_bsp::HologramHazard>,
    pub simulated_time_seconds: f32,
    pub is_animating: bool,
    pub show_display_viewport: bool,
    pub show_pedestal_viewport: bool,
    pub show_projection_beam_viewport: bool,
    pub show_scanlines_viewport: bool,
    pub show_rotation_axis_viewport: bool,
}

impl Default for HologramStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::HologramConfig::default();
        let brushes = ut99_bsp::generate_hologram_brushes(&config);
        let metrics = ut99_bsp::evaluate_hologram_kinematics(&config, 0.0);
        let hazards = ut99_bsp::audit_hologram_safety(&config, &[]);

        Self {
            active_config: config,
            cached_brushes: Some(brushes),
            cached_metrics: metrics,
            cached_hazards: hazards,
            simulated_time_seconds: 0.0,
            is_animating: true,
            show_display_viewport: true,
            show_pedestal_viewport: true,
            show_projection_beam_viewport: true,
            show_scanlines_viewport: true,
            show_rotation_axis_viewport: true,
        }
    }
}

/// Settings and state for Interactive Dynamic Weather, Atmospheric Precipitation & Lightning Storm Studio
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherStudioSettings {
    pub active_config: ut99_bsp::WeatherConfig,
    pub cached_volume_brush: Option<ut99_bsp::Brush>,
    pub cached_lightning_branch: Vec<glam::Vec3>,
    pub cached_metrics: ut99_bsp::WeatherMetrics,
    pub cached_hazards: Vec<ut99_bsp::WeatherHazard>,
    pub simulated_time_seconds: f32,
    pub is_animating: bool,
    pub listener_probe_pos: glam::Vec3,
    pub show_weather_volume_viewport: bool,
    pub show_lightning_viewport: bool,
    pub show_rain_streaks_viewport: bool,
}

impl Default for WeatherStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::WeatherConfig::default();
        let volume_b = ut99_bsp::generate_weather_volume_brush(&config);
        let listener = glam::Vec3::ZERO;
        let metrics = ut99_bsp::calculate_weather_metrics(&config, 0.0, listener);
        let hazards = ut99_bsp::audit_weather_safety(&config);
        let cloud_top = config.position + glam::Vec3::new(0.0, 0.0, config.volume_extents.z * 0.5);
        let ground = config.position - glam::Vec3::new(0.0, 0.0, config.volume_extents.z * 0.5);
        let lightning = ut99_bsp::generate_lightning_bolt(cloud_top, ground, 4, 120.0, 42);

        Self {
            active_config: config,
            cached_volume_brush: Some(volume_b),
            cached_lightning_branch: lightning,
            cached_metrics: metrics,
            cached_hazards: hazards,
            simulated_time_seconds: 0.0,
            is_animating: true,
            listener_probe_pos: listener,
            show_weather_volume_viewport: true,
            show_lightning_viewport: true,
            show_rain_streaks_viewport: true,
        }
    }
}

/// Settings and state for Interactive Arena Match Rules, Tournament Mutator & Weapon Arena Studio
#[derive(Debug, Clone, PartialEq)]
pub struct ArenaRulesStudioSettings {
    pub active_config: ut99_bsp::ArenaRulesConfig,
    pub cached_metrics: ut99_bsp::ArenaRulesMetrics,
    pub cached_hazards: Vec<ut99_bsp::ArenaHazard>,
    pub world_weapon_count: usize,
    pub world_ammo_count: usize,
    pub filter_mutator_query: String,
    pub show_pickup_telemetry_viewport: bool,
    pub show_spawn_protection_viewport: bool,
}

impl Default for ArenaRulesStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::ArenaRulesConfig::default();
        let metrics = ut99_bsp::evaluate_arena_rules_metrics(&config, 0, 0);
        let hazards = ut99_bsp::audit_arena_rules_safety(&config, 0, 0);
        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            world_weapon_count: 0,
            world_ammo_count: 0,
            filter_mutator_query: String::new(),
            show_pickup_telemetry_viewport: true,
            show_spawn_protection_viewport: true,
        }
    }
}

/// Settings and state for Interactive Armor, Health & ShieldBelt Stacking & Regeneration Studio
#[derive(Debug, Clone, PartialEq)]
pub struct ArmorStackingStudioSettings {
    pub active_config: ut99_bsp::ArmorStackingConfig,
    pub cached_pedestal_brush: Option<ut99_bsp::Brush>,
    pub cached_metrics: ut99_bsp::ArmorStackingMetrics,
    pub cached_hazards: Vec<ut99_bsp::ArmorHazard>,
    pub simulated_damage_probe_hp: f32,
    pub simulated_damage_probe_shield: f32,
    pub simulated_damage_probe_body: f32,
    pub simulated_damage_probe_thigh: f32,
    pub test_incoming_damage: f32,
    pub cached_damage_result: ut99_bsp::DamageAbsorptionResult,
    pub show_pedestals_viewport: bool,
    pub show_absorption_bubbles_viewport: bool,
    pub show_cluster_hazards_viewport: bool,
}

impl Default for ArmorStackingStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::ArmorStackingConfig::default();
        let brush = ut99_bsp::generate_armor_pedestal_brush(&config);
        let metrics = ut99_bsp::evaluate_armor_stacking_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_armor_stacking_safety(&config, &[]);
        let test_dmg = 100.0_f32; // Shock Combo / Sniper Headshot
        let dmg_res = ut99_bsp::simulate_damage_absorption(test_dmg, 100.0, 100.0, 100.0, 50.0);

        Self {
            active_config: config,
            cached_pedestal_brush: Some(brush),
            cached_metrics: metrics,
            cached_hazards: hazards,
            simulated_damage_probe_hp: 100.0,
            simulated_damage_probe_shield: 100.0,
            simulated_damage_probe_body: 100.0,
            simulated_damage_probe_thigh: 50.0,
            test_incoming_damage: test_dmg,
            cached_damage_result: dmg_res,
            show_pedestals_viewport: true,
            show_absorption_bubbles_viewport: true,
            show_cluster_hazards_viewport: true,
        }
    }
}

/// Settings and state for Interactive Bot AI Tactical JumpSpot & Translocator Route Studio
#[derive(Debug, Clone, PartialEq)]
pub struct BotJumpRoutesStudioSettings {
    pub active_config: ut99_bsp::BotJumpRoutesConfig,
    pub cached_solution: ut99_bsp::JumpTrajectorySolution,
    pub cached_metrics: ut99_bsp::BotJumpMetrics,
    pub cached_hazards: Vec<ut99_bsp::BotJumpHazard>,
    pub cached_plate_brush: Option<ut99_bsp::Brush>,
    pub show_trajectory_viewport: bool,
    pub show_landing_zone_viewport: bool,
    pub show_jump_plate_viewport: bool,
}

impl Default for BotJumpRoutesStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::BotJumpRoutesConfig::default();
        let solution = ut99_bsp::solve_jump_trajectory(&config);
        let metrics = ut99_bsp::evaluate_jump_metrics(&config, &solution);
        let hazards = ut99_bsp::audit_jump_route_safety(&config, &solution);
        let brush = ut99_bsp::generate_jump_plate_brush(&config);

        Self {
            active_config: config,
            cached_solution: solution,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_plate_brush: Some(brush),
            show_trajectory_viewport: true,
            show_landing_zone_viewport: true,
            show_jump_plate_viewport: true,
        }
    }
}

/// Settings and state for Interactive Tournament Weapon Placement & Ammo Circuit Studio
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponCircuitStudioSettings {
    pub active_config: ut99_bsp::WeaponCircuitConfig,
    pub cached_metrics: ut99_bsp::WeaponCircuitMetrics,
    pub cached_hazards: Vec<ut99_bsp::WeaponCircuitHazard>,
    pub cached_altar_brush: Option<ut99_bsp::Brush>,
    pub show_circuit_links_viewport: bool,
    pub show_altar_viewport: bool,
    pub show_ammo_tethers_viewport: bool,
}

impl Default for WeaponCircuitStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::WeaponCircuitConfig::default();
        let metrics = ut99_bsp::evaluate_weapon_circuit_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_weapon_circuit_safety(&config, &[]);
        let brush = ut99_bsp::generate_weapon_altar_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_altar_brush: Some(brush),
            show_circuit_links_viewport: true,
            show_altar_viewport: true,
            show_ammo_tethers_viewport: true,
        }
    }
}

/// Settings and state for Interactive Tournament Powerup Control & Relic Altar Studio
#[derive(Debug, Clone, PartialEq)]
pub struct PowerupControlStudioSettings {
    pub active_config: ut99_bsp::PowerupControlConfig,
    pub cached_metrics: ut99_bsp::PowerupControlMetrics,
    pub cached_hazards: Vec<ut99_bsp::PowerupHazard>,
    pub cached_altar_brush: Option<ut99_bsp::Brush>,
    pub show_altar_viewport: bool,
    pub show_containment_cylinder_viewport: bool,
    pub show_contention_rays_viewport: bool,
}

impl Default for PowerupControlStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::PowerupControlConfig::default();
        let metrics = ut99_bsp::evaluate_powerup_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_powerup_safety(&config, &[]);
        let brush = ut99_bsp::generate_powerup_altar_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_altar_brush: Some(brush),
            show_altar_viewport: true,
            show_containment_cylinder_viewport: true,
            show_contention_rays_viewport: true,
        }
    }
}

/// Settings and state for Interactive Tournament Spawn Point Balancing & Telefrag Prevention Studio
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnDistributionStudioSettings {
    pub active_config: ut99_bsp::SpawnDistributionConfig,
    pub cached_metrics: ut99_bsp::SpawnDistributionMetrics,
    pub cached_hazards: Vec<ut99_bsp::SpawnHazard>,
    pub cached_pad_brush: Option<ut99_bsp::Brush>,
    pub show_pad_viewport: bool,
    pub show_player_hull_viewport: bool,
    pub show_direction_vector_viewport: bool,
}

impl Default for SpawnDistributionStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::SpawnDistributionConfig::default();
        let metrics = ut99_bsp::evaluate_spawn_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_spawn_safety(&config, &[]);
        let brush = ut99_bsp::generate_spawn_pad_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_pad_brush: Some(brush),
            show_pad_viewport: true,
            show_player_hull_viewport: true,
            show_direction_vector_viewport: true,
        }
    }
}

/// Settings and state for Interactive Domination Control Point & Point-Weight Triangle Studio
#[derive(Debug, Clone, PartialEq)]
pub struct DominationStudioSettings {
    pub active_config: ut99_bsp::DominationConfig,
    pub cached_metrics: ut99_bsp::DominationMetrics,
    pub cached_hazards: Vec<ut99_bsp::DominationHazard>,
    pub cached_pedestal_brush: Option<ut99_bsp::Brush>,
    pub show_pedestal_viewport: bool,
    pub show_capture_radius_viewport: bool,
    pub show_triangle_sightlines_viewport: bool,
}

impl Default for DominationStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::DominationConfig::default();
        let metrics = ut99_bsp::evaluate_domination_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_domination_safety(&config, &[]);
        let brush = ut99_bsp::generate_domination_pedestal_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_pedestal_brush: Some(brush),
            show_pedestal_viewport: true,
            show_capture_radius_viewport: true,
            show_triangle_sightlines_viewport: true,
        }
    }
}

/// Settings and state for Interactive Assault Objective & Objective Chain Studio
#[derive(Debug, Clone, PartialEq)]
pub struct AssaultStudioSettings {
    pub active_config: ut99_bsp::AssaultChainConfig,
    pub cached_metrics: ut99_bsp::AssaultMetrics,
    pub cached_hazards: Vec<ut99_bsp::AssaultHazard>,
    pub cached_door_brush: Option<ut99_bsp::Brush>,
    pub show_door_viewport: bool,
    pub show_objective_beacon_viewport: bool,
    pub show_chain_links_viewport: bool,
}

impl Default for AssaultStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::AssaultChainConfig::default();
        let metrics = ut99_bsp::evaluate_assault_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_assault_safety(&config, &[]);
        let brush = ut99_bsp::generate_blast_door_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_door_brush: Some(brush),
            show_door_viewport: true,
            show_objective_beacon_viewport: true,
            show_chain_links_viewport: true,
        }
    }
}

/// Settings and state for Interactive Team Base Radar, Jammer & Threat Map Studio
#[derive(Debug, Clone, PartialEq)]
pub struct RadarStudioSettings {
    pub active_config: ut99_bsp::RadarStationConfig,
    pub cached_metrics: ut99_bsp::RadarMetrics,
    pub cached_hazards: Vec<ut99_bsp::RadarHazard>,
    pub cached_antenna_brush: Option<ut99_bsp::Brush>,
    pub show_antenna_viewport: bool,
    pub show_detection_radius_viewport: bool,
    pub show_sweep_cone_viewport: bool,
}

impl Default for RadarStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::RadarStationConfig::default();
        let metrics = ut99_bsp::evaluate_radar_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_radar_safety(&config, &[]);
        let brush = ut99_bsp::generate_radar_mast_and_dish_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_antenna_brush: Some(brush),
            show_antenna_viewport: true,
            show_detection_radius_viewport: true,
            show_sweep_cone_viewport: true,
        }
    }
}

/// Settings and state for Interactive Capture The Flag Base & Transit Routes Studio
#[derive(Debug, Clone, PartialEq)]
pub struct CtfBaseStudioSettings {
    pub active_config: ut99_bsp::CtfBaseConfig,
    pub cached_metrics: ut99_bsp::CtfBaseMetrics,
    pub cached_hazards: Vec<ut99_bsp::CtfBaseHazard>,
    pub cached_pedestal_brush: Option<ut99_bsp::Brush>,
    pub show_pedestal_viewport: bool,
    pub show_defense_radius_viewport: bool,
    pub show_transit_route_viewport: bool,
}

impl Default for CtfBaseStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::CtfBaseConfig::default();
        let metrics = ut99_bsp::evaluate_ctf_base_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_ctf_base_safety(&config, &[]);
        let brush = ut99_bsp::generate_ctf_flag_base_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_pedestal_brush: Some(brush),
            show_pedestal_viewport: true,
            show_defense_radius_viewport: true,
            show_transit_route_viewport: true,
        }
    }
}

/// Settings and state for Interactive Team Cannon & Automated Turret Defense Studio
#[derive(Debug, Clone, PartialEq)]
pub struct TeamCannonStudioSettings {
    pub active_config: ut99_bsp::TeamCannonConfig,
    pub cached_metrics: ut99_bsp::TeamCannonMetrics,
    pub cached_hazards: Vec<ut99_bsp::TeamCannonHazard>,
    pub cached_bunker_brush: Option<ut99_bsp::Brush>,
    pub show_bunker_viewport: bool,
    pub show_range_sphere_viewport: bool,
    pub show_targeting_line_viewport: bool,
}

impl Default for TeamCannonStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::TeamCannonConfig::default();
        let metrics = ut99_bsp::evaluate_team_cannon_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_team_cannon_safety(&config, &[]);
        let brush = ut99_bsp::generate_team_cannon_bunker_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_bunker_brush: Some(brush),
            show_bunker_viewport: true,
            show_range_sphere_viewport: true,
            show_targeting_line_viewport: true,
        }
    }
}

/// Settings and state for Interactive Bio-Rifle Slime Trap & Volumetric Hazard Field Studio
#[derive(Debug, Clone, PartialEq)]
pub struct BioTrapStudioSettings {
    pub active_config: ut99_bsp::BioTrapConfig,
    pub cached_metrics: ut99_bsp::BioTrapMetrics,
    pub cached_hazards: Vec<ut99_bsp::BioTrapHazard>,
    pub cached_grate_brush: Option<ut99_bsp::Brush>,
    pub show_grate_viewport: bool,
    pub show_splash_radius_viewport: bool,
    pub show_adhesion_normal_viewport: bool,
}

impl Default for BioTrapStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::BioTrapConfig::default();
        let metrics = ut99_bsp::evaluate_bio_trap_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_bio_trap_safety(&config, &[]);
        let brush = ut99_bsp::generate_bio_drain_grate_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_grate_brush: Some(brush),
            show_grate_viewport: true,
            show_splash_radius_viewport: true,
            show_adhesion_normal_viewport: true,
        }
    }
}

/// Settings and state for Interactive Pressure Plate & Proximity Trigger Network Studio
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerNetworkStudioSettings {
    pub active_config: ut99_bsp::TriggerNetworkConfig,
    pub cached_metrics: ut99_bsp::TriggerNetworkMetrics,
    pub cached_hazards: Vec<ut99_bsp::TriggerNetworkHazard>,
    pub cached_plate_brush: Option<ut99_bsp::Brush>,
    pub show_plate_viewport: bool,
    pub show_activation_volume_viewport: bool,
    pub show_event_wiring_lines_viewport: bool,
}

impl Default for TriggerNetworkStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::TriggerNetworkConfig::default();
        let metrics = ut99_bsp::evaluate_trigger_network_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_trigger_network_safety(&config, &[]);
        let brush = ut99_bsp::generate_pressure_plate_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_plate_brush: Some(brush),
            show_plate_viewport: true,
            show_activation_volume_viewport: true,
            show_event_wiring_lines_viewport: true,
        }
    }
}

/// Settings and state for Interactive Elevator, LiftCenter/LiftExit & Vertical Transit Studio
#[derive(Debug, Clone, PartialEq)]
pub struct ElevatorStudioSettings {
    pub active_config: ut99_bsp::ElevatorConfig,
    pub cached_metrics: ut99_bsp::ElevatorMetrics,
    pub cached_hazards: Vec<ut99_bsp::ElevatorSafetyHazard>,
    pub cached_platform_brush: Option<ut99_bsp::Brush>,
    pub show_platform_viewport: bool,
    pub show_travel_axis_viewport: bool,
    pub show_bot_waypoints_viewport: bool,
}

impl Default for ElevatorStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::ElevatorConfig::default();
        let metrics = ut99_bsp::evaluate_elevator_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_elevator_safety(&config, &[]);
        let brush = ut99_bsp::generate_elevator_platform_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_platform_brush: Some(brush),
            show_platform_viewport: true,
            show_travel_axis_viewport: true,
            show_bot_waypoints_viewport: true,
        }
    }
}

/// Settings and state for Interactive Dynamic Zone Velocity, Atmospheric Convection & Low-G Space Vacuum Studio
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneVelocityStudioSettings {
    pub active_config: ut99_bsp::ZoneVelocityConfig,
    pub cached_metrics: ut99_bsp::ZoneVelocityMetrics,
    pub cached_hazards: Vec<ut99_bsp::ZoneVelocityHazard>,
    pub cached_portal_brush: Option<ut99_bsp::Brush>,
    pub show_portal_sheet_viewport: bool,
    pub show_velocity_vectors_viewport: bool,
    pub show_drift_trajectory_viewport: bool,
}

impl Default for ZoneVelocityStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::ZoneVelocityConfig::default();
        let metrics = ut99_bsp::evaluate_zone_velocity_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_zone_velocity_safety(&config, &[]);
        let brush = ut99_bsp::generate_zone_portal_sheet_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_portal_brush: Some(brush),
            show_portal_sheet_viewport: true,
            show_velocity_vectors_viewport: true,
            show_drift_trajectory_viewport: true,
        }
    }
}

/// Settings and state for Interactive Explosive Barrel, Toxic Drum & Hazard Pyrotechnics Studio
#[derive(Debug, Clone, PartialEq)]
pub struct HazardBarrelStudioSettings {
    pub active_config: ut99_bsp::HazardBarrelConfig,
    pub cached_metrics: ut99_bsp::HazardBarrelMetrics,
    pub cached_hazards: Vec<ut99_bsp::HazardBarrelSafetyAlert>,
    pub cached_barrel_brush: Option<ut99_bsp::Brush>,
    pub show_barrel_mesh_viewport: bool,
    pub show_blast_radius_viewport: bool,
    pub show_chain_links_viewport: bool,
    pub show_toxic_spill_viewport: bool,
}

impl Default for HazardBarrelStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::HazardBarrelConfig::default();
        let metrics = ut99_bsp::evaluate_hazard_barrel_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_hazard_barrel_safety(&config, &[]);
        let brush = ut99_bsp::generate_hazard_barrel_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_barrel_brush: Some(brush),
            show_barrel_mesh_viewport: true,
            show_blast_radius_viewport: true,
            show_chain_links_viewport: true,
            show_toxic_spill_viewport: true,
        }
    }
}

/// Settings and state for Interactive Special Lit, Zone Lighting Mask & Light Filter Studio
#[derive(Debug, Clone, PartialEq)]
pub struct SpecialLitStudioSettings {
    pub active_config: ut99_bsp::SpecialLitConfig,
    pub cached_metrics: ut99_bsp::SpecialLitMetrics,
    pub cached_hazards: Vec<ut99_bsp::SpecialLitHazard>,
    pub cached_fixture_brush: Option<ut99_bsp::Brush>,
    pub show_fixture_brush_viewport: bool,
    pub show_special_radius_viewport: bool,
    pub show_surface_rays_viewport: bool,
}

impl Default for SpecialLitStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::SpecialLitConfig::default();
        let metrics = ut99_bsp::evaluate_special_lit_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_special_lit_safety(&config, &[]);
        let brush = ut99_bsp::generate_special_lit_fixture_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_fixture_brush: Some(brush),
            show_fixture_brush_viewport: true,
            show_special_radius_viewport: true,
            show_surface_rays_viewport: true,
        }
    }
}

/// Settings and state for Interactive Sniper Vantage Point, Headshot Angle & Occlusion Nest Studio
#[derive(Debug, Clone, PartialEq)]
pub struct SniperVantageStudioSettings {
    pub active_config: ut99_bsp::SniperVantageConfig,
    pub cached_metrics: ut99_bsp::SniperVantageMetrics,
    pub cached_hazards: Vec<ut99_bsp::SniperVantageSafetyAlert>,
    pub cached_parapet_brush: Option<ut99_bsp::Brush>,
    pub show_parapet_mesh_viewport: bool,
    pub show_sightline_cone_viewport: bool,
    pub show_headshot_plane_viewport: bool,
}

impl Default for SniperVantageStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::SniperVantageConfig::default();
        let metrics = ut99_bsp::evaluate_sniper_vantage_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_sniper_vantage_safety(&config, &[]);
        let brush = ut99_bsp::generate_sniper_parapet_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_parapet_brush: Some(brush),
            show_parapet_mesh_viewport: true,
            show_sightline_cone_viewport: true,
            show_headshot_plane_viewport: true,
        }
    }
}

/// Settings and state for Interactive Secret Door, Concealed Passage & Keyhole Mover Studio
#[derive(Debug, Clone, PartialEq)]
pub struct SecretDoorStudioSettings {
    pub active_config: ut99_bsp::SecretDoorConfig,
    pub cached_metrics: ut99_bsp::SecretDoorMetrics,
    pub cached_hazards: Vec<ut99_bsp::SecretDoorSafetyHazard>,
    pub cached_door_brush: Option<ut99_bsp::Brush>,
    pub show_door_mesh_viewport: bool,
    pub show_displacement_path_viewport: bool,
    pub show_trigger_radius_viewport: bool,
}

impl Default for SecretDoorStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::SecretDoorConfig::default();
        let metrics = ut99_bsp::evaluate_secret_door_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_secret_door_safety(&config, &[]);
        let brush = ut99_bsp::generate_secret_door_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_door_brush: Some(brush),
            show_door_mesh_viewport: true,
            show_displacement_path_viewport: true,
            show_trigger_radius_viewport: true,
        }
    }
}

/// Settings and state for Interactive Teleporter Portal Arch, Dimensional Vortex & Gate Studio
#[derive(Debug, Clone, PartialEq)]
pub struct TeleporterArchStudioSettings {
    pub active_config: ut99_bsp::TeleporterArchConfig,
    pub cached_metrics: ut99_bsp::TeleporterArchMetrics,
    pub cached_hazards: Vec<ut99_bsp::TeleporterArchHazard>,
    pub cached_arch_brush: Option<ut99_bsp::Brush>,
    pub show_arch_mesh_viewport: bool,
    pub show_vortex_sheet_viewport: bool,
    pub show_exit_trajectory_viewport: bool,
}

impl Default for TeleporterArchStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::TeleporterArchConfig::default();
        let metrics = ut99_bsp::evaluate_teleporter_arch_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_teleporter_arch_safety(&config, &[]);
        let brush = ut99_bsp::generate_teleporter_arch_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_arch_brush: Some(brush),
            show_arch_mesh_viewport: true,
            show_vortex_sheet_viewport: true,
            show_exit_trajectory_viewport: true,
        }
    }
}

/// Settings and state for Interactive Toxic Slime Acid Basin, Lava Flow & Caustic Pool Studio
#[derive(Debug, Clone, PartialEq)]
pub struct FluidBasinStudioSettings {
    pub active_config: ut99_bsp::FluidBasinConfig,
    pub cached_metrics: ut99_bsp::FluidBasinMetrics,
    pub cached_hazards: Vec<ut99_bsp::FluidBasinHazard>,
    pub cached_basin_brush: Option<ut99_bsp::Brush>,
    pub show_basin_mesh_viewport: bool,
    pub show_fluid_surface_viewport: bool,
    pub show_escape_ramp_viewport: bool,
}

impl Default for FluidBasinStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::FluidBasinConfig::default();
        let metrics = ut99_bsp::evaluate_fluid_basin_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_fluid_basin_safety(&config, &[]);
        let brush = ut99_bsp::generate_fluid_basin_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_basin_brush: Some(brush),
            show_basin_mesh_viewport: true,
            show_fluid_surface_viewport: true,
            show_escape_ramp_viewport: true,
        }
    }
}

/// Settings and state for Interactive Alarm Siren Beacon, Emergency Strobe & Warning Klaxon Studio
#[derive(Debug, Clone, PartialEq)]
pub struct AlarmBeaconStudioSettings {
    pub active_config: ut99_bsp::AlarmBeaconConfig,
    pub cached_metrics: ut99_bsp::AlarmBeaconMetrics,
    pub cached_hazards: Vec<ut99_bsp::AlarmBeaconHazard>,
    pub cached_beacon_brush: Option<ut99_bsp::Brush>,
    pub show_beacon_mesh_viewport: bool,
    pub show_strobe_pulse_viewport: bool,
    pub show_audible_radius_viewport: bool,
}

impl Default for AlarmBeaconStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::AlarmBeaconConfig::default();
        let metrics = ut99_bsp::evaluate_alarm_beacon_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_alarm_beacon_safety(&config, &[]);
        let brush = ut99_bsp::generate_alarm_beacon_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_beacon_brush: Some(brush),
            show_beacon_mesh_viewport: true,
            show_strobe_pulse_viewport: true,
            show_audible_radius_viewport: true,
        }
    }
}

/// Settings and state for Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio
#[derive(Debug, Clone, PartialEq)]
pub struct SupplyLockerStudioSettings {
    pub active_config: ut99_bsp::SupplyLockerConfig,
    pub cached_metrics: ut99_bsp::SupplyLockerMetrics,
    pub cached_hazards: Vec<ut99_bsp::SupplyLockerHazard>,
    pub cached_locker_brush: Option<ut99_bsp::Brush>,
    pub show_locker_mesh_viewport: bool,
    pub show_contained_items_viewport: bool,
    pub show_shelf_clearance_viewport: bool,
}

impl Default for SupplyLockerStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::SupplyLockerConfig::default();
        let metrics = ut99_bsp::evaluate_supply_locker_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_supply_locker_safety(&config, &[]);
        let brush = ut99_bsp::generate_supply_locker_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_locker_brush: Some(brush),
            show_locker_mesh_viewport: true,
            show_contained_items_viewport: true,
            show_shelf_clearance_viewport: true,
        }
    }
}

/// Settings and state for Interactive Glass Skylight, Observation Dome & Structural Truss Studio
#[derive(Debug, Clone, PartialEq)]
pub struct SkylightStudioSettings {
    pub active_config: ut99_bsp::SkylightConfig,
    pub cached_metrics: ut99_bsp::SkylightMetrics,
    pub cached_hazards: Vec<ut99_bsp::SkylightHazard>,
    pub cached_skylight_brush: Option<ut99_bsp::Brush>,
    pub show_skylight_mesh_viewport: bool,
    pub show_sunlight_shaft_viewport: bool,
    pub show_mullion_truss_viewport: bool,
}

impl Default for SkylightStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::SkylightConfig::default();
        let metrics = ut99_bsp::evaluate_skylight_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_skylight_safety(&config, &[]);
        let brush = ut99_bsp::generate_skylight_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_skylight_brush: Some(brush),
            show_skylight_mesh_viewport: true,
            show_sunlight_shaft_viewport: true,
            show_mullion_truss_viewport: true,
        }
    }
}

/// Settings and state for Interactive Ventilation Duct, Crawlspace Shaft & Infiltration Grate Studio
#[derive(Debug, Clone, PartialEq)]
pub struct VentDuctStudioSettings {
    pub active_config: ut99_bsp::VentDuctConfig,
    pub cached_metrics: ut99_bsp::VentDuctMetrics,
    pub cached_hazards: Vec<ut99_bsp::VentDuctHazard>,
    pub cached_duct_brush: Option<ut99_bsp::Brush>,
    pub show_duct_mesh_viewport: bool,
    pub show_air_current_viewport: bool,
    pub show_crouch_clearance_viewport: bool,
}

impl Default for VentDuctStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::VentDuctConfig::default();
        let metrics = ut99_bsp::evaluate_vent_duct_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_vent_duct_safety(&config, &[]);
        let brush = ut99_bsp::generate_vent_duct_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_duct_brush: Some(brush),
            show_duct_mesh_viewport: true,
            show_air_current_viewport: true,
            show_crouch_clearance_viewport: true,
        }
    }
}

/// Settings and state for Interactive Industrial Steam Piston, Hydraulic Compactor & Trap Hazard Studio
#[derive(Debug, Clone, PartialEq)]
pub struct PistonStudioSettings {
    pub active_config: ut99_bsp::PistonConfig,
    pub cached_metrics: ut99_bsp::PistonMetrics,
    pub cached_hazards: Vec<ut99_bsp::PistonHazard>,
    pub cached_piston_brush: Option<ut99_bsp::Brush>,
    pub show_piston_mesh_viewport: bool,
    pub show_stroke_trajectory_viewport: bool,
    pub show_squish_hazard_viewport: bool,
}

impl Default for PistonStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::PistonConfig::default();
        let metrics = ut99_bsp::evaluate_piston_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_piston_safety(&config, &[]);
        let brush = ut99_bsp::generate_piston_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_piston_brush: Some(brush),
            show_piston_mesh_viewport: true,
            show_stroke_trajectory_viewport: true,
            show_squish_hazard_viewport: true,
        }
    }
}

/// Settings and state for Interactive Jump Boots Cache, Anti-Gravity Booster & High-Vault Studio
#[derive(Debug, Clone, PartialEq)]
pub struct JumpBootsStudioSettings {
    pub active_config: ut99_bsp::JumpBootsConfig,
    pub cached_metrics: ut99_bsp::JumpBootsMetrics,
    pub cached_hazards: Vec<ut99_bsp::JumpBootsHazard>,
    pub cached_cradle_brush: Option<ut99_bsp::Brush>,
    pub show_cradle_mesh_viewport: bool,
    pub show_ballistic_arc_viewport: bool,
    pub show_target_platform_viewport: bool,
}

impl Default for JumpBootsStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::JumpBootsConfig::default();
        let metrics = ut99_bsp::evaluate_jump_boots_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_jump_boots_safety(&config, &[]);
        let brush = ut99_bsp::generate_jump_boots_brush(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_cradle_brush: Some(brush),
            show_cradle_mesh_viewport: true,
            show_ballistic_arc_viewport: true,
            show_target_platform_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForceFieldStudioSettings {
    pub active_config: ut99_bsp::ForceFieldConfig,
    pub cached_metrics: ut99_bsp::ForceFieldMetrics,
    pub cached_hazards: Vec<ut99_bsp::ForceFieldHazard>,
    pub cached_field_brush: Option<ut99_bsp::Brush>,
    pub cached_pylon_brushes: Vec<ut99_bsp::Brush>,
    pub show_field_slab_viewport: bool,
    pub show_pylons_viewport: bool,
    pub show_generator_link_viewport: bool,
}

impl Default for ForceFieldStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::ForceFieldConfig::default();
        let metrics = ut99_bsp::evaluate_force_field_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_force_field_safety(&config, &[]);
        let (field_brush, pylon_brushes) = ut99_bsp::generate_force_field_brushes(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_field_brush: Some(field_brush),
            cached_pylon_brushes: pylon_brushes,
            show_field_slab_viewport: true,
            show_pylons_viewport: true,
            show_generator_link_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReactorCoreStudioSettings {
    pub active_config: ut99_bsp::ReactorCoreConfig,
    pub cached_metrics: ut99_bsp::ReactorCoreMetrics,
    pub cached_hazards: Vec<ut99_bsp::ReactorCoreHazard>,
    pub cached_core_brush: Option<ut99_bsp::Brush>,
    pub cached_ring_brushes: Vec<ut99_bsp::Brush>,
    pub show_core_brush_viewport: bool,
    pub show_rings_viewport: bool,
    pub show_hazard_radius_viewport: bool,
}

impl Default for ReactorCoreStudioSettings {
    fn default() -> Self {
        let config = ut99_bsp::ReactorCoreConfig::default();
        let metrics = ut99_bsp::evaluate_reactor_core_metrics(&config, &[]);
        let hazards = ut99_bsp::audit_reactor_core_safety(&config, &[]);
        let (core_brush, ring_brushes) = ut99_bsp::generate_reactor_core_brushes(&config);

        Self {
            active_config: config,
            cached_metrics: metrics,
            cached_hazards: hazards,
            cached_core_brush: Some(core_brush),
            cached_ring_brushes: ring_brushes,
            show_core_brush_viewport: true,
            show_rings_viewport: true,
            show_hazard_radius_viewport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AntiPortalReport {
    pub total_anti_portals: usize,
    pub anti_portal_brushes: Vec<usize>,
    pub total_anti_portal_polys: usize,
    pub total_world_polys: usize,
    pub estimated_culling_efficiency: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneAudioReport {
    pub zone_name: String,
    pub actor_index: usize,
    pub location: Vec3,
    pub is_reverb: bool,
    pub preset: ut99_bsp::EaxEnvironment,
    pub reverb_time: f32,
    pub sound_radius: f32,
    pub sound_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkyboxLink {
    pub skyzone_name: String,
    pub skyzone_location: Vec3,
    pub backdrop_centroid: Vec3,
    pub backdrop_vertices: Vec<Vec3>,
    pub brush_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrajectoryArc {
    pub source_name: String,
    pub source_class: String,
    pub start: Vec3,
    pub points: Vec<Vec3>,
    pub landing_point: Option<Vec3>,
    pub flight_time: f32,
    pub is_teleport: bool,
}

/// UT99 `TournamentPlayer` ballistic jump/launch modes (`BL-P3-04`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ut99JumpMoveType {
    /// Standard running jump (`GroundSpeed = 400`, `JumpZ = 325`)
    StandingJump,
    /// UT99 Dodge-Jump (`DodgeSpeed = 600`, `DodgeJumpZ = 210`)
    DodgeJump,
    /// Impact Hammer Charged Jump (`GroundSpeed = 400`, `ImpactJumpZ = 680`)
    ImpactHammerJump,
    /// Anti-Gravity JumpBoots (`GroundSpeed = 400`, `JumpZ = 325 * 3 = 975`)
    JumpBootsJump,
    /// Translocator Disc Throw (`ThrowSpeed = 1150`, upward pitch boost `+200`)
    TranslocatorArc,
    /// Explicit Kicker velocity vector (`KickVelocity`)
    CustomKicker(Vec3),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapBudgetStats {
    pub total_brushes: usize,
    pub additive_brushes: usize,
    pub subtractive_brushes: usize,
    pub semisolid_brushes: usize,
    pub nonsolid_brushes: usize,
    pub total_polygons: usize,
    pub estimated_bsp_nodes: usize,
    pub total_actors: usize,
    pub light_actors: usize,
    pub nav_actors: usize,
    pub pickup_actors: usize,
    pub trigger_actors: usize,
    pub other_actors: usize,
    pub total_movers: usize,
    pub total_static_meshes: usize,
    pub total_projectors: usize,
    pub total_reachspecs: usize,
    pub total_zones: usize,

    // Status ratios compared to UE1 Engine Maximums:
    pub node_budget_ratio: f32,
    pub brush_budget_ratio: f32,
    pub actor_budget_ratio: f32,
    pub reachspec_budget_ratio: f32,
    pub zone_budget_ratio: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlinerCategory {
    All,
    Brushes,
    Movers,
    Lights,
    Navigation,
    Pickups,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InspectorPanelMode {
    #[default]
    Split,
    OutlinerOnly,
    InspectorOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoverKeyframe {
    pub location: Vec3,
    pub rotation: Vec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditorMover {
    pub name: String,
    /// Canonical UT99 Mover class (`Mover`, `AssertMover`, `AttachMover`, `ElevatorMover`,
    /// `GradualMover`, `LoopMover`, `MixMover`, `RotatingMover`).
    pub mover_class: String,
    pub brush: Brush,
    pub current_key: usize,
    pub keyframes: [Option<MoverKeyframe>; 8],
    /// Canonical UT99 `Object.InitialState` (`StandOpenTimed`, `BumpOpenTimed`, `BumpButton`,
    /// `TriggerOpenTimed`, `TriggerToggle`, `TriggerControl`, `TriggerPound`, `ConstantLoop`).
    pub initial_state: String,
    pub stay_open_time: f32,
    pub move_time: f32,
    pub delay_time: f32,
    pub glide_type: String,
    pub encroach_type: String,
    pub tag: String,
    pub event: String,
    pub opening_sound: String,
    pub opened_sound: String,
    pub closing_sound: String,
    pub closed_sound: String,
    pub move_ambient_sound: String,
}

impl EditorMover {
    pub fn new(brush: Brush) -> Self {
        let loc = brush.location;
        let rot = brush.rotation;
        let mut keyframes: [Option<MoverKeyframe>; 8] = [None, None, None, None, None, None, None, None];
        keyframes[0] = Some(MoverKeyframe { location: loc, rotation: rot });
        keyframes[1] = Some(MoverKeyframe { location: loc + Vec3::new(0.0, 0.0, 128.0), rotation: rot });
        Self {
            name: "Mover".to_string(),
            mover_class: "Mover".to_string(),
            brush,
            current_key: 0,
            keyframes,
            initial_state: "BumpOpenTimed".to_string(),
            stay_open_time: 4.0,
            move_time: 1.0,
            delay_time: 0.0,
            glide_type: "MV_Glide".to_string(),
            encroach_type: "ME_ReturnWhenEncroach".to_string(),
            tag: "MoverTag".to_string(),
            event: String::new(),
            opening_sound: String::new(),
            opened_sound: String::new(),
            closing_sound: String::new(),
            closed_sound: String::new(),
            move_ambient_sound: String::new(),
        }
    }
}

/// Cache of uploaded egui GPU textures for the skin studio to prevent per-frame GPU reallocations
#[derive(Clone, Default, PartialEq)]
pub struct SkinTextureCache(pub std::collections::HashMap<String, egui::TextureHandle>);

impl std::fmt::Debug for SkinTextureCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SkinTextureCache").field(&self.0.len()).finish()
    }
}

/// Settings and interactive state for UT99 Skin & Character Studio
#[derive(Debug, Clone, PartialEq)]
pub struct SkinStudioSettings {
    pub preview_3d_zoom: f32,
    pub preview_3d_yaw: f32,
    pub preview_3d_pitch: f32,
    pub auto_rotate: bool,
    pub show_wireframe_guide: bool,
    pub active_tab: usize, // 0: Skin Wizard, 1: Model & Identity, 2: Textures & Teams, 3: Faces & TalkTextures, 4: 3D Viewport, 5: Export & Manifest
    pub export_status_message: Option<String>,
    pub wizard_config: ut99_bsp::ProceduralSkinConfig,
    pub show_uv_wireframe_2d: bool,
    pub export_output_dir: String,
    pub export_subfolders: bool,
    pub skin_texture_cache: SkinTextureCache,
}

impl Default for SkinStudioSettings {
    fn default() -> Self {
        Self {
            preview_3d_zoom: 1.0,
            preview_3d_yaw: 0.0,
            preview_3d_pitch: 5.0,
            auto_rotate: false,
            show_wireframe_guide: false,
            active_tab: 0,
            export_status_message: None,
            wizard_config: ut99_bsp::ProceduralSkinConfig::default(),
            show_uv_wireframe_2d: true,
            export_output_dir: "ExportOutput".to_string(),
            export_subfolders: true,
            skin_texture_cache: SkinTextureCache::default(),
        }
    }
}

/// Settings and state for Native Package Asset Extractor & .uz Decompressor
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PackageExtractorSettings {
    pub source_path: String,
    pub output_dir: String,
    pub cat_textures: bool,
    pub cat_sounds: bool,
    pub cat_music: bool,
    pub cat_meshes: bool,
    pub cat_scripts: bool,
    pub subfolder_by_package: bool,
    pub subfolder_by_category: bool,
    pub export_psk_for_meshes: bool,
    pub overwrite: bool,
    pub status_message: String,
    pub last_report: Option<String>,
    pub scan_summaries: Vec<ut99_bsp::PackageContentSummary>,
}

impl Default for PackageExtractorSettings {
    fn default() -> Self {
        Self {
            source_path: String::new(),
            output_dir: "ExportOutput".to_string(),
            cat_textures: true,
            cat_sounds: true,
            cat_music: true,
            cat_meshes: true,
            cat_scripts: true,
            subfolder_by_package: true,
            subfolder_by_category: true,
            export_psk_for_meshes: true,
            overwrite: true,
            status_message: "Ready. Select a package (.u, .utx, .uax, .umx, .unr, .uz) or directory to extract.".to_string(),
            last_report: None,
            scan_summaries: Vec::new(),
        }
    }
}


