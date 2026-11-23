use crate::camera::{Camera, ViewportLayout, ViewportType};
use crate::grid::Grid;
use crate::history::{Action, History};
use crate::raycast::{GizmoAxis, GizmoMode, Ray};
use glam::Vec3;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use ut99_bsp::{
    analyze_combat_sightlines, analyze_item_balance, build_arch,
    build_cone, build_cube, build_curved_staircase, build_cylinder, build_dispatcher_sequence,
    build_sheet, build_sheet_advanced, build_sphere, build_spiral_staircase, build_staircase,
    build_terrain, build_tetrahedron, build_torus, build_volumetric, build_zone_pvs_graph,
    clip_brush_by_plane, evaluate_brush_shadow_gradient, export_brushes_to_obj,
    export_map_to_gltf_json, generate_symmetrical_ctf_base, get_eax_preset, parse_eax_preset,
    split_brush_by_plane, test_zone_watertightness_with_bounds, ArchConfig, AutoPathReport, BotSimulator,
    BspNode, Brush, CombatCoverageReport, CsgOperation, CtfSymmetryConfig, CtfSymmetryResult,
    DispatcherSequence, EaxEnvironment, ItemBalanceReport, LevelDoctor, LightSourceShape,
    detect_cascading_dispatcher_chains, DispatcherChainLink, DispatcherPresetTemplate,
    MapIssue, MoverGlideType, MoverKeyframe as BspMoverKeyframe, MoverTrajectory, NavGraph,
    ParticleEmitter, ParticleEmitterType, ParticlePresetTemplate, PenumbraGradientProfile, Plane,
    ProjectedDecalPoly, Projector, ProjectorBlendMode, Shape2D, SheetAxis, SheetPreset,
    SkeletalMesh,
    JumpPadTrajectoryArc, LiftSyncReport, TacticalCoverEntry, TacticalCoverRating,
    simulate_jumppad_trajectory,
    ZoneLeakConfig, ZonePortalEdge, ZonePvsMatrix, ZoneSealReport, RoundRobinSequence,
};
use ut99_format::{
    parse_t3d, scan_map_dependencies, write_t3d, MapManifest, PackageDependency,
    TaggedProperty, TaggedValue, T3dMap, TrackedVec, STOCK_PACKAGES,
};

static NEXT_MAP_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

/// Returns a globally unique monotonic identifier for each distinct map document instance.
#[inline]
pub fn next_map_instance_id() -> u64 {
    NEXT_MAP_INSTANCE_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    CameraSelection,
    VertexEdit,
    BrushClip,
    FreehandPolygon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuilderShape {
    Cube,
    Cylinder,
    Cone,
    Sheet,
    Stairs,
    CurvedStairs,
    SpiralStairs,
    Sphere,
    Tetrahedron,
    Torus,
    Volumetric,
    Terrain,
    Arch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpTab {
    #[default]
    QuickStart,
    Shortcuts,
    CsgGeometry,
    Lighting,
    MoversAndTriggers,
    ZonesAndAtmosphere,
    SurfacesAndMaterials,
    EventSystem,
    BotNavigation,
    GameModes,
    StudiosAndTools,
    UnrealEdGotchas,
    About,
}

impl HelpTab {
    pub fn title(&self) -> &'static str {
        match self {
            Self::QuickStart => "🚀 Quick Start",
            Self::Shortcuts => "⌨ Shortcuts",
            Self::CsgGeometry => "🧱 CSG & Geometry",
            Self::Lighting => "💡 Lighting & Radiosity",
            Self::MoversAndTriggers => "🚪 Movers & Lifts",
            Self::ZonesAndAtmosphere => "🌊 Zones & Atmosphere",
            Self::SurfacesAndMaterials => "🎨 Surfaces & Materials",
            Self::EventSystem => "⚡ Event & Tag Logic",
            Self::BotNavigation => "🤖 Bot AI & ReachSpecs",
            Self::GameModes => "🎮 Game Modes",
            Self::StudiosAndTools => "✨ Studios & Tools",
            Self::UnrealEdGotchas => "⚠️ UnrealEd 2.0 Gotchas",
            Self::About => "ℹ System & About",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspacePreset {
    Default,
    LevelDesign,
    LightingAndAtmosphere,
    BotAndGameplay,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarqueeBox {
    pub start_screen: (f32, f32),
    pub current_screen: (f32, f32),
    pub viewport_type: ViewportType,
}


#[derive(Debug, Clone, Default)]
pub struct EditorClipboard {
    pub brushes: Vec<Brush>,
    pub actors: Vec<ut99_format::unr::actor::ActorRecord>,
    pub t3d_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorAxis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrushSolidity {
    #[default]
    Solid,
    SemiSolid,
    NonSolid,
}

impl BrushSolidity {
    pub fn label(&self) -> &'static str {
        match self {
            BrushSolidity::Solid => "Solid",
            BrushSolidity::SemiSolid => "Semi-Solid",
            BrushSolidity::NonSolid => "Non-Solid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrushScaleMode {
    #[default]
    GeometryVertices,
    ActorTransform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FindActorCategory {
    #[default]
    All,
    Lights,
    Navigation,
    Triggers,
    Inventory,
    Movers,
    Sounds,
    Other,
}

impl FindActorCategory {
    pub fn label(&self) -> &'static str {
        match self {
            FindActorCategory::All => "All",
            FindActorCategory::Lights => "💡 Lights",
            FindActorCategory::Navigation => "🔷 Navigation",
            FindActorCategory::Triggers => "⚡ Triggers",
            FindActorCategory::Inventory => "🔫 Inventory",
            FindActorCategory::Movers => "🚪 Movers",
            FindActorCategory::Sounds => "🔊 Sounds",
            FindActorCategory::Other => "📦 Other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActorSearchResult {
    pub actor_index: usize,
    pub name: String,
    pub class_name: String,
    pub tag: String,
    pub event: String,
    pub group: String,
    pub location: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShearAxis {
    #[default]
    XZ,
    YZ,
    ZX,
    ZY,
    XY,
    YX,
}

impl ShearAxis {
    pub fn label(&self) -> &'static str {
        match self {
            ShearAxis::XZ => "X by Z (Ramp along X)",
            ShearAxis::YZ => "Y by Z (Ramp along Y)",
            ShearAxis::ZX => "Z by X (Incline Z on X)",
            ShearAxis::ZY => "Z by Y (Incline Z on Y)",
            ShearAxis::XY => "X by Y (Skew X on Y)",
            ShearAxis::YX => "Y by X (Skew Y on X)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignAxis {
    #[default]
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignMode {
    #[default]
    Center,
    Min,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextureReplaceScope {
    #[default]
    EntireLevel,
    SelectedBrushes,
    SelectedSurfaces,
}

impl TextureReplaceScope {
    pub fn label(&self) -> &'static str {
        match self {
            TextureReplaceScope::EntireLevel => "Entire Level",
            TextureReplaceScope::SelectedBrushes => "Selected Brushes",
            TextureReplaceScope::SelectedSurfaces => "Selected Surfaces",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextureMatchMode {
    #[default]
    ExactName,
    PackageOnly,
    ContainsSubstring,
}

impl TextureMatchMode {
    pub fn label(&self) -> &'static str {
        match self {
            TextureMatchMode::ExactName => "Exact Name",
            TextureMatchMode::PackageOnly => "Package Only",
            TextureMatchMode::ContainsSubstring => "Substring Match",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum UiExperienceMode {
    #[default]
    Standard,
    Architect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum GizmoCoordinateSpace {
    #[default]
    World,
    Local,
}

/// Asset or class payload carried during interactive drag & drop into the 3D viewport
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DragDropPayload {
    ActorClass(String),
    Texture(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SettingsTab {
    #[default]
    General,
    Appearance,
    GameIntegration,
    ExportPaths,
    GridSnapping,
    Viewport3D,
    AutoSave,
    KeymapInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardTab {
    #[default]
    MoverLift,
    TeleporterPair,
    WaterBasin,
    JumpPad,
    RoomCorridor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceFlagsPreset {
    Water,
    LavaSlime,
    GlassWindow,
    MaskedGrate,
    SkyBackdrop,
    InvisibleBarrier,
    FullbrightUnlit,
    ClearAll,
}

impl SurfaceFlagsPreset {
    pub fn label(&self) -> &'static str {
        match self {
            SurfaceFlagsPreset::Water => "Water Surface",
            SurfaceFlagsPreset::LavaSlime => "Lava / Slime",
            SurfaceFlagsPreset::GlassWindow => "Clear Glass Window",
            SurfaceFlagsPreset::MaskedGrate => "Masked Grate / Fence",
            SurfaceFlagsPreset::SkyBackdrop => "Sky Backdrop",
            SurfaceFlagsPreset::InvisibleBarrier => "Invisible Collision",
            SurfaceFlagsPreset::FullbrightUnlit => "Fullbright Unlit",
            SurfaceFlagsPreset::ClearAll => "Clear All Flags",
        }
    }

    pub fn flags(&self) -> u32 {
        match self {
            SurfaceFlagsPreset::Water => {
                ut99_bsp::PF_TRANSLUCENT | ut99_bsp::PF_SMALL_WAVY | ut99_bsp::PF_TWO_SIDED | ut99_bsp::PF_PORTAL
            }
            SurfaceFlagsPreset::LavaSlime => {
                ut99_bsp::PF_UNLIT | ut99_bsp::PF_BIG_WAVY | ut99_bsp::PF_TWO_SIDED
            }
            SurfaceFlagsPreset::GlassWindow => {
                ut99_bsp::PF_TRANSLUCENT | ut99_bsp::PF_TWO_SIDED
            }
            SurfaceFlagsPreset::MaskedGrate => {
                ut99_bsp::PF_MASKED | ut99_bsp::PF_TWO_SIDED
            }
            SurfaceFlagsPreset::SkyBackdrop => {
                ut99_bsp::PF_FAKE_BACKDROP | ut99_bsp::PF_UNLIT
            }
            SurfaceFlagsPreset::InvisibleBarrier => {
                ut99_bsp::PF_INVISIBLE
            }
            SurfaceFlagsPreset::FullbrightUnlit => {
                ut99_bsp::PF_UNLIT
            }
            SurfaceFlagsPreset::ClearAll => 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MapStatistics {
    pub total_brushes: usize,
    pub solid_brushes: usize,
    pub subtract_brushes: usize,
    pub semisolid_brushes: usize,
    pub nonsolid_brushes: usize,
    pub total_actors: usize,
    pub lights_count: usize,
    pub nav_points_count: usize,
    pub weapons_pickups_count: usize,
    pub triggers_count: usize,
    pub movers_count: usize,
    pub static_meshes_count: usize,
    pub total_polys: usize,
    pub total_vertices: usize,
    pub unique_textures_count: usize,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOperationKind {
    SaveUnr(PathBuf),
    SaveUnrNative(PathBuf),
    SaveT3d(PathBuf),
    ExportT3d(PathBuf),
    ExportGltf(PathBuf),
    ExportObj(PathBuf),
}

impl SaveOperationKind {
    /// Only document saves can update identity or satisfy a deferred save request.
    pub fn saves_document(&self) -> bool {
        matches!(self, Self::SaveUnr(_) | Self::SaveUnrNative(_) | Self::SaveT3d(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavePhase {
    /// Frame 0: display overlay and request immediate repaint
    PendingPreparation,
    /// Frame 1: executing the actual compile/serialize/IO task
    Executing,
    /// Successfully finished; show success summary card
    Completed,
    /// Failed with an error message
    Failed,
}

#[derive(Debug, Clone)]
pub enum SaveWorkerMessage {
    Stage {
        current_stage: usize,
        total_stages: usize,
        stage_message: String,
    },
    Success {
        saved_path: PathBuf,
        file_size_bytes: u64,
        duration: std::time::Duration,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct SaveFeedbackState {
    pub kind: SaveOperationKind,
    pub filename: String,
    pub target_path: PathBuf,
    pub current_stage: usize,
    pub total_stages: usize,
    pub stage_message: String,
    pub phase: SavePhase,
    pub started_at: std::time::Instant,
    pub completed_at: Option<std::time::Instant>,
    pub file_size_bytes: Option<u64>,
    pub error_message: Option<String>,
}

pub mod settings;
pub use settings::*;

/// Freshness of the last static Level Doctor report, not a runtime certification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelAuditFreshness {
    NotRun,
    Current,
    Stale,
}

/// Exact inputs consulted by Level Doctor. Snapshotting rather than relying on
/// the dirty bit catches direct edits, undo/redo, and changes after a save.
struct LevelAuditSnapshot {
    world_brushes: Vec<Brush>,
    builder_brush: Brush,
    movers: Vec<EditorMover>,
    nav_graph: NavGraph,
    unr_actors: Option<Vec<ut99_format::ActorRecord>>,
    t3d_actors: Vec<ut99_format::T3dActor>,
    map_path: Option<PathBuf>,
    level_title: String,
    level_game_type: String,
}

pub(crate) struct SceneBoundsCache {
    world_brushes_key: (u64, u64),
    pub(crate) world_brushes: Vec<(Vec3, Vec3)>,
    pub(crate) world_brushes_total_xy: (f32, f32, f32, f32),
    movers_key: (u64, u64),
    pub(crate) movers: Vec<(Vec3, Vec3)>,
    static_meshes_key: (u64, u64),
    pub(crate) static_meshes: Vec<(Vec3, Vec3)>,
}

impl Default for SceneBoundsCache {
    fn default() -> Self {
        Self {
            world_brushes_key: (0, 0),
            world_brushes: Vec::new(),
            world_brushes_total_xy: (-1024.0, 1024.0, -1024.0, 1024.0),
            movers_key: (0, 0),
            movers: Vec::new(),
            static_meshes_key: (0, 0),
            static_meshes: Vec::new(),
        }
    }
}

impl SceneBoundsCache {
    pub(crate) fn reset(&mut self) {
        self.world_brushes_key = (0, 0);
        self.world_brushes = Vec::new();
        self.world_brushes_total_xy = (-1024.0, 1024.0, -1024.0, 1024.0);
        self.movers_key = (0, 0);
        self.movers = Vec::new();
        self.static_meshes_key = (0, 0);
        self.static_meshes = Vec::new();
    }
}

#[derive(Default)]
pub(crate) struct OutlinerCache {
    pub(crate) key: Option<(u64, u64, u64, u64, u64, OutlinerCategory)>,
    pub(crate) filter: String,
    pub(crate) items: Vec<crate::gui::layers_outliner::LayerItemRef>,
}

impl OutlinerCache {
    pub(crate) fn reset(&mut self) {
        self.key = None;
        self.filter = String::new();
        self.items = Vec::new();
    }
}

#[allow(dead_code)]
pub struct EditorState {
    pub map_instance_id: u64,
    pub(crate) scene_bounds_cache: std::cell::RefCell<SceneBoundsCache>,
    pub(crate) outliner_cache: OutlinerCache,
    pub clipboard: EditorClipboard,
    pub camera: Camera,
    pub grid: Grid,
    pub builder_shape: BuilderShape,
    pub builder_cube_size: Vec3,
    pub builder_cylinder_params: (f32, f32, usize), // height, radius, sides
    pub builder_cone_params: (f32, f32, usize),     // height, radius, sides
    pub builder_sheet_params: (f32, f32),           // width, height
    pub builder_stairs_params: (usize, f32, f32, f32), // steps, height, width, length
    pub builder_curved_stairs_params: (usize, f32, f32, f32, f32, bool), // steps, height, inner_rad, width, angle, ccw
    pub builder_spiral_stairs_params: (usize, f32, f32, f32, f32, bool), // steps, height, inner_rad, width, angle, ccw
    pub builder_sphere_params: (f32, usize, usize), // radius, rings, segments
    pub builder_tetrahedron_params: f32,            // radius
    pub builder_torus_params: (f32, f32, usize, usize), // ring_radius, tube_radius, ring_sides, tube_sides
    pub builder_volumetric_params: (f32, f32, usize), // width, height, sheets
    pub builder_terrain_params: (usize, usize, f32, f32, u32), // width_steps, length_steps, step_size, height_ampl, seed
    pub builder_brush: Brush,
    pub world_brushes: TrackedVec<Brush>,
    pub map: T3dMap,
    pub selected_brush_index: Option<usize>,
    pub selected_actor_index: Option<usize>,
    pub is_builder_selected: bool,

    // Photoshop-Style Layers & Elements Visibility & Lock (01)
    pub hidden_brushes: HashSet<usize>,
    pub hidden_actors: HashSet<usize>,
    pub hidden_movers: HashSet<usize>,
    pub hidden_static_meshes: HashSet<usize>,
    pub show_builder_brush: bool,
    pub locked_brushes: HashSet<usize>,
    pub locked_actors: HashSet<usize>,
    pub locked_movers: HashSet<usize>,
    pub locked_static_meshes: HashSet<usize>,
    pub is_builder_locked: bool,
    pub outliner_last_clicked_row: Option<usize>,
    pub snap_to_grid: bool,
    pub grid_step: f32,
    pub console_messages: Vec<String>,
    pub console_command_input: String,
    pub show_bottom_console: bool,
    pub toasts: Vec<ToastNotification>,
    pub next_toast_id: u64,
    pub map_path: Option<PathBuf>,
    pub loaded_unr: Option<ut99_format::UnrLevel>,
    pub history: History,
    pub active_gizmo_axis: GizmoAxis,
    pub hovered_gizmo_axis: GizmoAxis,
    pub gizmo_space: GizmoCoordinateSpace,
    pub gizmo_mode: GizmoMode,
    pub active_drag_drop: Option<DragDropPayload>,
    pub drag_drop_preview_pos: Option<Vec3>,
    pub drag_drop_preview_normal: Option<Vec3>,
    pub surface_magnet_mode: bool,
    pub vertex_magnet_mode: bool,
    pub cursor_delta_text: Option<String>,
    pub laser_ruler_start: Option<Vec3>,
    pub laser_ruler_end: Option<Vec3>,
    pub selection_locked: bool,
    pub drag_threshold_px: f32,
    pub frustum_culling_enabled: bool,
    pub hardware_profile: crate::graphics::HardwareProfile,
    pub graphics_backend: crate::graphics::GraphicsBackendPreference,
    pub preferred_adapter_name: Option<String>,
    pub viewport_render_scale: f32,
    pub present_mode: crate::graphics::PresentModePreference,
    pub max_frame_latency: u32,
    pub pending_graphics_reinit: bool,
    pub graphics_diagnostics: crate::graphics::GraphicsDiagnostics,
    pub last_cull_stats: (usize, usize, usize, usize),
    pub rotation_snap_deg: f32,
    pub snap_to_rotation: bool,
    pub viewport_layout: ViewportLayout,
    pub active_viewport_type: ViewportType,
    pub viewport_rect: [f32; 4],
    pub ui_scale: f32,
    pub texture_manager: crate::texture_manager::TextureManager,
    pub active_texture: Option<String>,
    pub show_texture_browser: bool,
    pub show_surface_properties: bool,
    pub show_actor_properties: bool,
    pub actor_properties_search: String,
    pub selected_texture_package: String,
    pub selected_texture_group: String,
    pub texture_search_query: String,
    pub show_ai_paths: bool,
    pub show_actor_radii: bool,
    pub ai_path_links: TrackedVec<(Vec3, Vec3, [f32; 4])>,
    pub editor_mode: EditorMode,
    pub selected_vertex_indices: Vec<usize>,
    pub clip_points: Vec<Vec3>,
    pub clip_normal_flipped: bool,
    pub shape2d: Shape2D,
    pub show_shape_editor: bool,
    pub shape_editor_depth: f32,
    pub shape_editor_revolve_angle: f32,
    pub shape_editor_revolve_segs: usize,
    pub shape_editor_bevel_depth: f32,
    pub shape_editor_bevel_width: f32,
    pub movers: TrackedVec<EditorMover>,
    pub selected_mover_index: Option<usize>,
    pub show_add_special_dialog: bool,
    pub special_brush_flags: u32,
    pub special_is_subtractive: bool,
    pub lighting_config: ut99_bsp::LightingConfig,
    pub last_lighting_stats: Option<ut99_bsp::LightingStats>,
    pub show_lighting_dialog: bool,
    pub show_lighting_studio_dialog: bool,
    pub bsp_config: ut99_bsp::BspBuilderConfig,
    pub last_bsp_report: Option<ut99_bsp::BspBuildReport>,
    pub show_build_options_dialog: bool,
    pub show_scale_brush_dialog: bool,
    pub scale_brush_factors: Vec3,
    pub scale_brush_uniform: bool,
    pub scale_brush_mode: BrushScaleMode,
    pub scale_brush_adjust_textures: bool,
    pub show_find_actors_dialog: bool,
    pub find_actors_query: String,
    pub find_actors_category: FindActorCategory,
    pub show_shear_brush_dialog: bool,
    pub shear_brush_axis: ShearAxis,
    pub shear_brush_factor: f32,
    pub shear_brush_anchor_base: bool,
    pub show_actor_align_dialog: bool,
    pub show_map_statistics_dialog: bool,
    pub show_actor_browser: bool,
    pub show_group_browser: bool,
    pub hidden_actor_groups: std::collections::HashSet<String>,
    pub brush_groups: std::collections::HashMap<usize, String>,
    pub geometry_revision: u64,
    pub sampled_surface_uv: Option<(Vec3, Vec3, Vec3, i16, i16)>,
    pub shape2d_undo_stack: Vec<Shape2D>,
    pub shape2d_redo_stack: Vec<Shape2D>,
    pub group_browser_new_name: String,
    pub group_browser_search: String,
    pub show_sound_browser: bool,
    pub show_music_browser: bool,
    pub class_tree: Option<ut99_format::package::ClassTreeNode>,
    pub all_classes: Vec<ut99_format::package::ClassResource>,
    pub selected_class_name: Option<String>,
    pub selected_class_package: Option<String>,
    pub actor_browser_search: String,
    pub(crate) audio_catalog: crate::audio_catalog::AudioCatalog,
    pub sound_packages: Vec<String>,
    pub selected_sound_package: String,
    pub loaded_sounds: Vec<ut99_format::package::SoundResource>,
    pub mylevel_sounds: Vec<ut99_format::package::SoundResource>,
    pub selected_sound_name: Option<String>,
    pub sound_browser_search: String,
    pub music_packages: Vec<String>,
    pub selected_music_package: String,
    pub loaded_music: Option<ut99_format::package::MusicResource>,
    pub static_meshes: TrackedVec<Brush>,
    pub selected_static_mesh_index: Option<usize>,
    pub mesh_library: crate::mesh_manager::MeshLibrary,
    pub selected_mesh_name: Option<String>,
    pub mesh_browser_search: String,
    pub mesh_browser_category: String,
    pub show_mesh_browser: bool,
    pub procedural_simulator: ut99_bsp::ProceduralTextureSimulator,
    pub procedural_anim_playing: bool,
    pub procedural_texture_name: String,
    pub show_procedural_texture_dialog: bool,
    pub projectors: TrackedVec<Projector>,
    pub selected_projector_index: Option<usize>,
    pub active_projector_decals: TrackedVec<ProjectedDecalPoly>,
    pub show_decal_studio_dialog: bool,
    pub decal_width: f32,
    pub decal_height: f32,
    pub decal_blend_mode: ProjectorBlendMode,
    pub decal_max_distance: f32,
    pub decal_paint_mode: bool,
    pub nav_graph: NavGraph,
    pub bot_simulator: Option<BotSimulator>,
    pub show_bot_simulator_window: bool,
    pub sim_start_node: usize,
    pub sim_target_node: usize,
    pub map_audit_issues: Vec<MapIssue>,
    level_audit_snapshot: Option<LevelAuditSnapshot>,
    pub selected_issue_index: Option<usize>,
    pub show_diagnostic_markers: bool,
    pub show_level_doctor_window: bool,
    pub build_progress: Option<(f32, String)>,
    pub outliner_filter: String,
    pub outliner_category: OutlinerCategory,
    pub inspector_panel_mode: InspectorPanelMode,
    pub prefabs: Vec<crate::prefab::Prefab>,
    pub show_prefab_browser: bool,
    pub show_event_links: bool,

    // Bookmarks (0..9)
    pub camera_bookmarks: [Option<crate::camera::CameraBookmark>; 10],
    pub show_camera_bookmarks_dialog: bool,

    // Budget & Performance Profiler
    pub show_map_budget_dialog: bool,

    // Auto-Save & Recovery
    pub auto_save_enabled: bool,
    pub auto_save_interval_secs: f32,
    pub auto_save_timer: f32,
    pub auto_save_snapshots: Vec<(String, String)>,
    pub(super) auto_save_t3d_slot: usize,
    pub(super) auto_save_unr_slot: usize,
    pub(super) auto_save_session_dir: PathBuf,
    auto_save_worker: Option<autosave::AsyncAutoSaveCoordinator>,
    auto_save_latest_seq: u64,
    pub map_is_dirty: bool,
    pub show_autosave_dialog: bool,
    pub show_unsaved_exit_modal: bool,
    pub confirm_exit_without_saving: bool,

    // Playtest & Launch Settings
    pub ut99_executable_path: String,
    pub ut99_launch_args: String,
    pub show_play_settings_dialog: bool,

    // Centralized Settings & User Preferences
    pub show_settings_dialog: bool,
    pub active_settings_tab: SettingsTab,
    pub default_export_dir: Option<std::path::PathBuf>,
    /// Dedicated destination for wizard-generated maps (kept apart from the stock `Maps` folder).
    pub wizard_output_dir: Option<std::path::PathBuf>,
    pub default_author: String,
    pub language: crate::i18n::AppLanguage,
    pub theme: crate::gui::theme::AppTheme,

    // Flight Trajectories & Teleport Arcs
    pub show_trajectories: bool,

    // Real-time Luxmeter Lighting Heatmap
    pub render_lux_heatmap: bool,

    // Multi-Object Selection & Batching
    pub multi_selected_brushes: HashSet<usize>,
    pub multi_selected_actors: HashSet<usize>,

    // Marquee / Rubber-Band Box Selection (02)
    pub active_marquee: Option<MarqueeBox>,

    // Recent Commands MRU for Command Palette (02)
    pub recent_palette_commands: Vec<String>,


    // Surface Selection & Filtering (UnrealEd 2.0 Parity)
    pub selected_surfaces: HashSet<(usize, usize)>, // (brush_index, poly_index)
    pub memorized_surfaces: HashSet<(usize, usize)>, // UnrealEd 2.0 Shift+M/R/O/U/X memory

    // Texture Find & Replace (UnrealEd 2.0 Parity & Supremacy)
    pub show_replace_textures_dialog: bool,
    pub replace_texture_find: String,
    pub replace_texture_with: String,
    pub replace_texture_scope: TextureReplaceScope,
    pub replace_texture_match_mode: TextureMatchMode,

    // Progressive Disclosure & Action Hub (01)
    pub ui_experience_mode: UiExperienceMode,
    pub show_action_wizards_dialog: bool,
    pub show_visual_presets_dialog: bool,
    pub active_wizard_tab: WizardTab,
    pub wizard_mover_height: f32,
    pub wizard_mover_horizontal: f32,
    pub wizard_mover_delay: f32,
    pub wizard_mover_tag: String,
    pub wizard_teleport_enter_tag: String,
    pub wizard_teleport_exit_tag: String,
    pub wizard_teleport_offset_x: f32,
    pub wizard_teleport_offset_y: f32,
    pub wizard_teleport_offset_z: f32,
    pub wizard_basin_type: u8,
    pub wizard_basin_width: f32,
    pub wizard_basin_length: f32,
    pub wizard_basin_depth: f32,
    pub wizard_jump_height: f32,
    pub wizard_jump_dist: f32,
    pub wizard_room_width: f32,
    pub wizard_room_length: f32,
    pub wizard_room_height: f32,
    pub wizard_room_lights: u8,

    // SkyZone & Fake Backdrop Skybox Visualizer
    pub show_skybox_links: bool,
    pub sky_zone_settings: SkyZoneSettings,
    pub show_skybox_studio_dialog: bool,
    pub selected_zone_tag: Option<String>,

    // Bot Navigation Coverage Heatmap
    pub show_nav_coverage: bool,

    // Zone Audio & Reverb Acoustics
    pub show_audio_zones: bool,
    pub show_acoustics_dialog: bool,
    pub acoustic_settings: AcousticStudioSettings,
    pub selected_sound_emitter_tag: Option<String>,

    // Distance Fog & Volumetric Zone Fog Simulation
    pub distance_fog_enabled: bool,
    pub fog_start: f32,
    pub fog_end: f32,
    pub fog_density: f32,
    pub fog_color: [f32; 4],
    pub show_zone_fog_studio_dialog: bool,
    pub zone_fog_settings: ZoneFogStudioSettings,

    // Sheet & Curtain Builder
    pub builder_sheet_axis: ut99_bsp::SheetAxis,
    pub builder_sheet_preset: ut99_bsp::SheetPreset,

    // Map Packaging & Distribution Manifest
    pub show_map_packaging_dialog: bool,

    // WarpZone Portal Visualizer & Non-Euclidean Studio
    pub show_warpzones: bool,
    pub show_warp_zone_studio_dialog: bool,
    pub warp_zone_settings: WarpZoneStudioSettings,

    // Anti-Portal Occluder Analyzer
    pub show_anti_portals: bool,

    // Arch & Vault Builder
    pub builder_arch_config: ut99_bsp::ArchConfig,

    // Tournament Gameplay Balance Simulator
    pub show_gameplay_balance_dialog: bool,

    // Zone Boundary Leak Detector
    pub show_zone_seal_dialog: bool,
    pub show_zone_leaks_visual: bool,

    // Mover 8-Keyframe Trajectory & Scrubbing
    pub mover_preview_time: f32,
    pub mover_preview_playing: bool,
    pub show_mover_trajectories: bool,

    // Auto-Pathing Builder & ReachSpecs
    pub auto_path_max_distance: f32,
    pub auto_path_max_step: f32,
    pub auto_path_max_jump: f32,

    // Dispatcher Timeline & Trigger Sequence
    pub show_dispatcher_dialog: bool,
    pub selected_dispatcher_name: Option<String>,
    pub dispatcher_settings: DispatcherStudioSettings,

    // Skeletal Mesh, Rigging & Character Animator
    pub skeletal_meshes: Vec<ut99_bsp::SkeletalMesh>,
    pub show_skeletal_bones: bool,
    pub show_skeletal_studio_dialog: bool,
    pub skeletal_settings: SkeletalStudioSettings,

    // UT99 Skin & Character Authoring Studio
    pub show_skin_studio_dialog: bool,
    pub skin_studio_project: ut99_bsp::SkinStudioProject,
    pub skin_studio_settings: SkinStudioSettings,

    // Package Asset Extractor & .uz Decompressor Studio
    pub show_package_extractor_dialog: bool,
    pub package_extractor: PackageExtractorSettings,

    // UnrealScript Class Wizard & #exec Studio
    pub show_uc_class_wizard_dialog: bool,
    pub uc_scaffold_config: ut99_bsp::unrealscript_exec::UcScaffoldConfig,
    pub uc_scaffold_status: String,

    // Dynamic Particle FX & Volumetric Emitter Simulator
    pub particle_emitters: Vec<ut99_bsp::ParticleEmitter>,
    pub show_particles: bool,
    pub show_particle_studio_dialog: bool,
    pub particle_settings: ParticleStudioSettings,

    // Bot AI Navigation Graph, Reachability Network & JumpPad Studio
    pub show_bot_paths: bool,
    pub show_bot_nav_studio_dialog: bool,
    pub bot_nav_settings: BotNavStudioSettings,
    pub last_autopath_report: Option<ut99_bsp::AutoPathReport>,

    // Procedural Architectural Prefab & Brush Macro Studio
    pub show_architectural_studio_dialog: bool,
    pub architectural_settings: ArchitecturalStudioSettings,

    // Terrain & Heightmap Mesh Sculptor Studio
    pub show_terrain_studio_dialog: bool,
    pub terrain_settings: TerrainStudioSettings,

    // Static Mesh LOD & Convex Collision Hull Studio
    pub show_lod_collision_dialog: bool,
    pub lod_collision_settings: LodCollisionStudioSettings,

    // Cinematic Matinee Camera Path & Cutscene Sequencer Studio
    pub show_cinematic_studio_dialog: bool,
    pub cinematic_settings: CinematicStudioSettings,

    // Dynamic Lighting Effects & Animating Strobe/Spotlight Studio
    pub show_light_effects_studio_dialog: bool,
    pub light_effects_settings: LightEffectsStudioSettings,

    // Zone Portal Leak, Antiportal & PVS Occlusion Diagnostic Studio
    pub show_zone_occlusion_studio_dialog: bool,
    pub zone_occlusion_settings: ZoneOcclusionStudioSettings,
    pub antiportals: Vec<ut99_bsp::AntiPortalVolume>,

    // Interactive Music Event & UMX Track Sequencing Studio
    pub show_music_studio_dialog: bool,
    pub music_studio_settings: MusicStudioSettings,

    // Interactive Sun, Corona & Lens Flare Studio
    pub show_corona_studio_dialog: bool,
    pub corona_studio_settings: CoronaStudioSettings,

    // Fluid Physics, Viscosity & Hazard Zone Studio
    pub show_fluid_studio_dialog: bool,
    pub fluid_studio_settings: FluidStudioSettings,

    // Bot Combat Trajectory & Weapon Sightline Studio
    pub show_ballistics_studio_dialog: bool,
    pub ballistics_studio_settings: BallisticsStudioSettings,

    // Teleporter Destination & Network Link Studio
    pub show_teleporter_studio_dialog: bool,
    pub teleporter_studio_settings: TeleporterStudioSettings,

    // Translocator Trajectory & Teleport Beacon Studio
    pub show_translocator_studio_dialog: bool,
    pub translocator_studio_settings: TranslocatorStudioSettings,

    // LiftCenter, LiftExit & Elevator Pathing Studio
    pub show_lift_pathing_studio_dialog: bool,
    pub lift_pathing_studio_settings: LiftPathingStudioSettings,

    // Dynamic Zone Water Surface & Wave Reflection Studio
    pub show_water_surface_studio_dialog: bool,
    pub water_surface_studio_settings: WaterSurfaceStudioSettings,

    // Mover Encroachment, Door Sequencing & Crushing Trap Studio
    pub show_mover_studio_dialog: bool,
    pub mover_studio_settings: MoverStudioSettings,

    // Team Base Objective, CTF Flag Run Route & Defense AI Studio
    pub show_team_objective_dialog: bool,
    pub team_objective_settings: TeamObjectiveStudioSettings,

    // Destructible Architecture, Breakable Glass & Fracture Studio
    pub show_destructible_studio_dialog: bool,
    pub destructible_studio_settings: DestructibleStudioSettings,

    // Jump Pad, Aerial Kicker Trajectory & Low-Gravity Studio
    pub show_kicker_studio_dialog: bool,
    pub kicker_studio_settings: KickerStudioSettings,

    // BSP Tree Quality, Node Cut Minimizer & Sliver Polygon Purging Studio
    pub show_bsp_quality_studio_dialog: bool,
    pub bsp_quality_studio_settings: BspQualityStudioSettings,

    // 2D Shape Loft, Curved Pipe & Architectural Molding Studio
    pub show_loft_studio_dialog: bool,
    pub loft_studio_settings: LoftStudioSettings,

    // Curved Architectural Stairway, Spiral Ramp & Smooth Collision Hull Studio
    pub show_stairway_studio_dialog: bool,
    pub stairway_studio_settings: StairwayStudioSettings,

    // Surface Clutter, Procedural Foliage & Decoration Scatter Studio
    pub show_scatter_studio_dialog: bool,
    pub scatter_studio_settings: ScatterStudioSettings,

    // Volumetric Light Shaft, Window God Ray & Atmospheric Beam Studio
    pub show_light_shaft_dialog: bool,
    pub light_shaft_settings: LightShaftStudioSettings,

    // Spline Rail, Kinetic Track Mover & Rollercoaster Studio
    pub show_track_studio_dialog: bool,
    pub track_studio_settings: TrackStudioSettings,

    // Security CCTV Camera, Live Video Monitor & Surveillance Studio
    pub show_surveillance_dialog: bool,
    pub surveillance_settings: SurveillanceStudioSettings,

    // Security Laser Grid, Tripwire Alarm & Energy Barrier Studio
    pub show_laser_grid_dialog: bool,
    pub laser_grid_settings: LaserGridStudioSettings,

    // Environmental Ventilation Fan, Air Current & Wind Tunnel Studio
    pub show_ventilator_dialog: bool,
    pub ventilator_settings: VentilatorStudioSettings,

    // Industrial Conveyor Belt, Treadmill & Surface Velocity Studio
    pub show_conveyor_dialog: bool,
    pub conveyor_settings: ConveyorStudioSettings,

    // Automated Defense Turret, Sentry Cannon & Emplacement Studio
    pub show_turret_dialog: bool,
    pub turret_settings: TurretStudioSettings,

    // Ambient Soundscape, Spatial Audio Attenuation & Acoustic Reverb Studio
    pub show_soundscape_dialog: bool,
    pub soundscape_settings: SoundscapeStudioSettings,

    // Environmental Hazard Chamber, Pressure Airlock & Gas Decompression Studio
    pub show_hazard_chamber_dialog: bool,
    pub hazard_chamber_settings: HazardChamberStudioSettings,

    // Industrial Pipe Network, Steam Valve & Conduit Junction Studio
    pub show_pipe_conduit_dialog: bool,
    pub pipe_conduit_settings: PipeConduitStudioSettings,

    // Holographic Signage, Sci-Fi Billboard & Volumetric Display Studio
    pub show_hologram_dialog: bool,
    pub hologram_settings: HologramStudioSettings,

    // Dynamic Weather, Atmospheric Precipitation & Lightning Storm Studio
    pub show_weather_dialog: bool,
    pub weather_settings: WeatherStudioSettings,

    // Interactive Arena Match Rules, Tournament Mutator & Weapon Arena Studio
    pub show_arena_rules_dialog: bool,
    pub arena_rules_settings: ArenaRulesStudioSettings,

    // Interactive Armor, Health & ShieldBelt Stacking & Regeneration Studio
    pub show_armor_stacking_dialog: bool,
    pub armor_stacking_settings: ArmorStackingStudioSettings,

    // Interactive Bot AI Tactical JumpSpot & Translocator Route Studio
    pub show_bot_jump_routes_dialog: bool,
    pub bot_jump_routes_settings: BotJumpRoutesStudioSettings,

    // Interactive Tournament Weapon Placement & Ammo Circuit Studio
    pub show_weapon_circuit_dialog: bool,
    pub weapon_circuit_settings: WeaponCircuitStudioSettings,

    // Interactive Tournament Powerup Control & Relic Altar Studio
    pub show_powerup_control_dialog: bool,
    pub powerup_control_settings: PowerupControlStudioSettings,

    // Interactive Tournament Spawn Point Balancing & Telefrag Prevention Studio
    pub show_spawn_distribution_dialog: bool,
    pub spawn_distribution_settings: SpawnDistributionStudioSettings,

    // Interactive Domination Control Point & Point-Weight Triangle Studio
    pub show_domination_dialog: bool,
    pub domination_settings: DominationStudioSettings,

    // Interactive Assault Objective & Objective Chain Studio
    pub show_assault_dialog: bool,
    pub assault_settings: AssaultStudioSettings,

    // Interactive Team Base Radar & Jammer Studio
    pub show_radar_dialog: bool,
    pub radar_settings: RadarStudioSettings,

    // Interactive Capture The Flag Base & Transit Routes Studio
    pub show_ctf_base_dialog: bool,
    pub ctf_base_settings: CtfBaseStudioSettings,

    // Interactive Team Cannon & Stationary Turret Defense Studio
    pub show_team_cannon_dialog: bool,
    pub team_cannon_settings: TeamCannonStudioSettings,

    // Interactive Bio-Rifle Slime Trap & Volumetric Hazard Field Studio
    pub show_bio_trap_dialog: bool,
    pub bio_trap_settings: BioTrapStudioSettings,

    // Interactive Pressure Plate & Proximity Trigger Network Studio
    pub show_trigger_network_dialog: bool,
    pub trigger_network_settings: TriggerNetworkStudioSettings,

    // Interactive Elevator, LiftCenter/LiftExit & Vertical Transit Studio
    pub show_elevator_dialog: bool,
    pub elevator_settings: ElevatorStudioSettings,

    // Interactive Dynamic Zone Velocity, Atmospheric Convection & Low-G Space Vacuum Studio
    pub show_zone_velocity_dialog: bool,
    pub zone_velocity_settings: ZoneVelocityStudioSettings,

    // Interactive Explosive Barrel, Toxic Drum & Hazard Pyrotechnics Studio
    pub show_hazard_barrel_dialog: bool,
    pub hazard_barrel_settings: HazardBarrelStudioSettings,

    // Interactive Special Lit, Zone Lighting Mask & Light Filter Studio
    pub show_special_lit_dialog: bool,
    pub special_lit_settings: SpecialLitStudioSettings,

    // Interactive Sniper Vantage Point, Headshot Angle & Occlusion Nest Studio
    pub show_sniper_vantage_dialog: bool,
    pub sniper_vantage_settings: SniperVantageStudioSettings,

    // Interactive Secret Door, Concealed Passage & Keyhole Mover Studio
    pub show_secret_door_dialog: bool,
    pub secret_door_settings: SecretDoorStudioSettings,

    // Interactive Teleporter Portal Arch, Dimensional Vortex & Gate Studio
    pub show_teleporter_arch_dialog: bool,
    pub teleporter_arch_settings: TeleporterArchStudioSettings,

    // Interactive Toxic Slime Acid Basin, Lava Flow & Caustic Pool Studio
    pub show_fluid_basin_dialog: bool,
    pub fluid_basin_settings: FluidBasinStudioSettings,

    // Interactive Alarm Siren Beacon, Emergency Strobe & Warning Klaxon Studio
    pub show_alarm_beacon_dialog: bool,
    pub alarm_beacon_settings: AlarmBeaconStudioSettings,

    // Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio
    pub show_supply_locker_dialog: bool,
    pub supply_locker_settings: SupplyLockerStudioSettings,

    // Interactive Glass Skylight, Observation Dome & Structural Truss Studio
    pub show_skylight_dialog: bool,
    pub skylight_settings: SkylightStudioSettings,

    // Interactive Ventilation Duct, Crawlspace Shaft & Infiltration Grate Studio
    pub show_vent_duct_dialog: bool,
    pub vent_duct_settings: VentDuctStudioSettings,

    // Interactive Industrial Steam Piston, Hydraulic Compactor & Trap Hazard Studio
    pub show_piston_dialog: bool,
    pub piston_settings: PistonStudioSettings,

    // Interactive Jump Boots Cache, Anti-Gravity Booster & High-Vault Studio
    pub show_jump_boots_dialog: bool,
    pub jump_boots_settings: JumpBootsStudioSettings,

    // Interactive Force Field, Energy Barrier & Power Core Overload Studio
    pub show_force_field_dialog: bool,
    pub force_field_settings: ForceFieldStudioSettings,

    // Interactive Rotating Gyroscope, Reactor Core & Kinetic Ring Studio
    pub show_reactor_core_dialog: bool,
    pub reactor_core_settings: ReactorCoreStudioSettings,

    // Bot Combat & Vantage Simulator
    pub show_combat_sightlines: bool,
    pub combat_coverage_report: Option<CombatCoverageReport>,

    // Zone PVS Graph & Cell Occlusion
    pub show_zone_pvs: bool,
    pub zone_pvs_matrix: Option<ZonePvsMatrix>,

    // Soft Penumbra Shadow Gradient Profiler
    pub show_shadow_penumbra: bool,
    pub shadow_penumbra_profile: Option<PenumbraGradientProfile>,

    // Symmetrical CTF Base Generator
    pub show_ctf_symmetry_dialog: bool,
    pub ctf_symmetry_config: CtfSymmetryConfig,

    // Command Palette (Ctrl+P / F1)
    pub show_command_palette: bool,
    pub command_palette_search: String,
    pub command_palette_selected_index: usize,

    // Help & Documentation Center (? / Shift+F1 / F1)
    pub show_help_dialog: bool,
    pub help_active_tab: HelpTab,
    pub help_search_query: String,

    // Keyboard Shortcuts Cheatsheet (Shift+?)
    pub show_shortcut_help_dialog: bool,
    pub shortcut_help_search: String,

    // 3D Viewport Context Menu (MOU-01)
    pub show_viewport_context_menu: bool,
    pub context_menu_screen_pos: egui::Pos2,
    pub context_menu_world_pos: Option<Vec3>,

    // Viewport Shading & Camera Speed (FASE 2 & 3)
    pub viewport_shading: crate::camera::ViewportShading,
    pub viewport_shadings: [crate::camera::ViewportShading; 4],
    pub camera_speed_preset: u8,

    // Recent Files History
    pub recent_maps: Vec<std::path::PathBuf>,

    // Mini-Map Navigation Radar
    pub show_minimap: bool,

    // Level Properties (UnrealEd 2.0 F6 Parity)
    pub show_level_properties_dialog: bool,
    pub level_title: String,
    pub level_author: String,
    pub level_ideal_player_count: String,
    pub level_song: String,
    pub level_song_section: u8,
    pub level_game_type: String,
    pub level_enter_text: String,
    pub level_ambient_brightness: u8,
    pub level_ambient_hue: u8,
    pub level_ambient_saturation: u8,

    // Scale Entire Map Suite (UnrealEd 2.0 Tools Parity)
    pub show_scale_map_dialog: bool,
    pub scale_map_factor: glam::Vec3,
    pub scale_map_actors: bool,
    pub scale_map_lights: bool,

    // Package Dependency Inspector & Distribution Manifest
    pub show_package_manifest_dialog: bool,
    pub cached_map_manifest: Option<ut99_format::MapManifest>,
    pub packaging_map_name_edit: String,

    // Interactive Visual Save & Export Feedback System
    pub save_feedback: Option<SaveFeedbackState>,
    pub pending_save_op: Option<SaveOperationKind>,
    pub save_worker_rx: Option<std::sync::mpsc::Receiver<SaveWorkerMessage>>,

    // UnrealScript Class Source Viewer & DefaultProperties Live Inspector
    pub show_uc_viewer_dialog: bool,
    pub uc_viewer_class: String,
    pub uc_viewer_search: String,
    pub uc_viewer_tab: crate::unrealscript::UcViewerTab,
    pub uc_editable_defaults: Vec<(String, String)>,
    pub uc_source_cache: String,
    pub uc_class_metadata: Option<crate::unrealscript::UcClassMetadata>,

    // UX Phase D: UI Polish, Inspector & UV Precision
    pub texture_thumbnail_size: f32,
    pub numeric_edit_brush_loc_initial: Option<(usize, Vec3)>,
    pub numeric_edit_brush_rot_initial: Option<(usize, Vec3)>,
    pub numeric_edit_brush_scale_initial: Option<(usize, Vec3)>,
    pub numeric_edit_actor_loc_initial: Option<(usize, Vec3)>,
    pub uv_edit_initial_polys: Option<Vec<(usize, Vec<ut99_bsp::Polygon>)>>,
    pub uv_drag_pan_u: f32,
    pub uv_drag_pan_v: f32,
    pub uv_drag_scale_u: f32,
    pub uv_drag_scale_v: f32,
    pub uv_drag_rot: f32,

    // Cycle 6 (BG-16): Master Browser, Data-Driven Smart Prefabs, Freehand Polygon & Background Image
    pub show_master_browser: bool,
    pub master_browser_tab: MasterBrowserTab,
    pub smart_prefab_category: crate::prefab_templates::SmartPrefabCategory,
    pub smart_prefab_params: crate::prefab_templates::SmartPrefabParams,
    pub selected_smart_prefab_id: String,
    pub freehand_points: Vec<Vec3>,
    pub freehand_extrude_depth: f32,
    pub background_image_path: Option<String>,
    pub background_display_mode: BackgroundDisplayMode,

    // Fase 1-4 Modernization: Target Engine Compatibility, Keymap Profile, Mover Show Polys & 2D Shape Segment Selection
    pub target_engine: TargetEngine,
    pub keymap_profile: KeymapProfile,
    pub show_mover_polys: bool,
    pub shape_editor_selected_segment: usize,

    // Fase S & T: Real-time Gizmo HUD Telemetry & Multi-Selection Drag Undo Snapshot
    pub active_gizmo_delta: Vec3,
    pub active_gizmo_angle_deg: f32,
    pub gizmo_drag_initial_brushes: Vec<(usize, Vec3)>,
    pub gizmo_drag_initial_actors: Vec<(usize, Vec3)>,

    // UX-01 & UX-08: Persistent Math Evaluator State & Unsaved Changes Confirmation Target
    pub math_expression_input: String,
    pub last_math_result: Option<f32>,
    pub pending_dirty_action: Option<PendingDirtyAction>,
    pub confirmed_close_tab_index: Option<usize>,

    // Window & Display Settings
    pub start_maximized: bool,
    pub is_fullscreen: bool,
    pub pending_toggle_fullscreen: bool,

    // First-Run Onboarding Welcome Wizard
    pub first_run_completed: bool,
    pub show_welcome_wizard: bool,
    pub welcome_wizard_step: usize,

    // Map Generator Wizard & DNA Cloner
    pub show_map_wizard_dialog: bool,
    pub map_wizard_config: ut99_bsp::MapWizardConfig,
    pub map_wizard_step: usize,
    pub map_wizard_lock_game_mode: bool,
    pub map_wizard_archetype_filter: Option<ut99_bsp::WizardArchetypeCategory>,
    pub official_dna_mode_filter: String,
    pub selected_official_dna_map: String,
    pub available_official_maps: Vec<String>,
    pub last_cloned_dna: Option<ut99_format::OfficialMapDna>,
    pub official_corpus_stats: Vec<ut99_format::GameModeCorpusStats>,
    pub official_corpus_profiles: Vec<ut99_format::OfficialMapDna>,

    // Map Wizard Minimap Interactivity & Custom Overrides
    pub wizard_minimap_selected: Option<WizardMinimapSelection>,
    pub wizard_minimap_tool: WizardMinimapTool,
    pub wizard_minimap_connect_source: Option<String>,
    pub wizard_minimap_drag_state: WizardMinimapDragState,
    pub wizard_minimap_add_object_kind: String,
    pub wizard_minimap_pan: egui::Vec2,
    pub wizard_minimap_zoom: f32,
    pub wizard_minimap_grid_snap: f32,
    pub wizard_minimap_show_bot_paths: bool,
    pub wizard_preset_json_modal: Option<String>,
}

/// Element selection target on the 2D wizard schematic minimap.
#[derive(Debug, Clone, PartialEq)]
pub enum WizardMinimapSelection {
    Room(String),
    Marker(String),
    CustomConnection(String),
}

/// 8-direction transformation resize handles for rooms on the 2D minimap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardResizeHandle {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

/// Drag interaction state for moving or resizing rooms and markers on the 2D minimap.
#[derive(Debug, Clone, PartialEq)]
pub enum WizardMinimapDragState {
    None,
    MovingRoom {
        room_name: String,
        initial_click_world: glam::Vec2,
        initial_center: glam::Vec3,
    },
    ResizingRoom {
        room_name: String,
        handle: WizardResizeHandle,
        initial_click_world: glam::Vec2,
        initial_center: glam::Vec3,
        initial_size: glam::Vec3,
    },
    MovingMarker {
        marker_label: String,
        initial_click_world: glam::Vec2,
        initial_loc: glam::Vec3,
    },
}

/// Editing tool mode on the 2D wizard schematic minimap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardMinimapTool {
    #[default]
    Inspect,
    AddRoom,
    AddObject,
    ConnectRooms,
    Delete,
}


impl Default for EditorState {
    fn default() -> Self {
        let builder_cube_size = Vec3::new(256.0, 256.0, 256.0);
        let builder_cylinder_params = (256.0, 128.0, 16);
        let builder_cone_params = (256.0, 128.0, 16);
        let builder_sheet_params = (256.0, 256.0);
        let builder_stairs_params = (8, 16.0, 256.0, 32.0);
        let builder_curved_stairs_params = (8, 16.0, 128.0, 128.0, 90.0, true);
        let builder_spiral_stairs_params = (16, 16.0, 64.0, 128.0, 360.0, true);
        let builder_sphere_params = (128.0, 8, 12);
        let builder_tetrahedron_params = 128.0f32;
        let builder_torus_params = (256.0, 64.0, 16, 8);
        let builder_volumetric_params = (256.0, 512.0, 2);
        let builder_terrain_params = (8, 8, 128.0, 64.0, 42u32);
        let builder_brush = build_cube("ActiveBuilder", CsgOperation::Active, builder_cube_size);

        // Pre-create an initial demo subtractive room (so the world isn't totally empty)
        let mut initial_room = build_cube("Room0", CsgOperation::Subtract, Vec3::new(1024.0, 1024.0, 512.0));
        initial_room.location = Vec3::new(0.0, 0.0, 256.0);

        let texture_manager = crate::texture_manager::TextureManager::default();
        let default_pkg = texture_manager
            .available_packages()
            .iter()
            .find(|p| p.eq_ignore_ascii_case("Indus1"))
            .or_else(|| texture_manager.available_packages().first())
            .cloned()
            .unwrap_or_else(|| "Indus1".to_string());

        let mut state = Self {
            map_instance_id: next_map_instance_id(),
            scene_bounds_cache: Default::default(),
            outliner_cache: Default::default(),
            camera: Camera::default(),
            grid: Grid::default(),
            builder_shape: BuilderShape::Cube,
            builder_cube_size,
            builder_cylinder_params,
            builder_cone_params,
            builder_sheet_params,
            builder_stairs_params,
            builder_curved_stairs_params,
            builder_spiral_stairs_params,
            builder_sphere_params,
            builder_tetrahedron_params,
            builder_torus_params,
            builder_volumetric_params,
            builder_terrain_params,
            builder_brush,
            world_brushes: vec![initial_room].into(),
            map: T3dMap::new(),
            clipboard: EditorClipboard::default(),
            selected_brush_index: None,
            selected_actor_index: None,
            is_builder_selected: false,
            hidden_brushes: HashSet::new(),
            hidden_actors: HashSet::new(),
            hidden_movers: HashSet::new(),
            hidden_static_meshes: HashSet::new(),
            show_builder_brush: true,
            locked_brushes: HashSet::new(),
            locked_actors: HashSet::new(),
            locked_movers: HashSet::new(),
            locked_static_meshes: HashSet::new(),
            is_builder_locked: false,
            outliner_last_clicked_row: None,
            snap_to_grid: true,
            grid_step: 64.0,
            console_messages: Vec::new(),
            console_command_input: String::new(),
            show_bottom_console: false,
            toasts: Vec::new(),
            next_toast_id: 1,
            map_path: None,
            loaded_unr: Some(ut99_format::UnrLevel::default()),
            history: History::default(),
            active_gizmo_axis: GizmoAxis::None,
            hovered_gizmo_axis: GizmoAxis::None,
            gizmo_space: GizmoCoordinateSpace::World,
            gizmo_mode: GizmoMode::Translate,
            active_drag_drop: None,
            drag_drop_preview_pos: None,
            drag_drop_preview_normal: None,
            surface_magnet_mode: false,
            vertex_magnet_mode: false,
            cursor_delta_text: None,
            laser_ruler_start: None,
            laser_ruler_end: None,
            selection_locked: false,
            drag_threshold_px: 6.0,
            frustum_culling_enabled: true,
            hardware_profile: crate::graphics::HardwareProfile::Auto,
            graphics_backend: crate::graphics::GraphicsBackendPreference::Auto,
            preferred_adapter_name: None,
            viewport_render_scale: 1.0,
            present_mode: crate::graphics::PresentModePreference::AutoVsync,
            max_frame_latency: 2,
            pending_graphics_reinit: false,
            graphics_diagnostics: Default::default(),
            last_cull_stats: (0, 0, 0, 0),
            rotation_snap_deg: 15.0,
            snap_to_rotation: true,
            viewport_layout: ViewportLayout::Single,
            active_viewport_type: ViewportType::Perspective,
            viewport_rect: [0.0, 0.0, 1440.0, 900.0],
            ui_scale: 1.0,
            texture_manager,
            active_texture: Some("Indus1.Indus_B01".to_string()),
            show_texture_browser: false,
            show_surface_properties: false,
            show_actor_properties: false,
            actor_properties_search: String::new(),
            selected_texture_package: default_pkg,
            selected_texture_group: "All".to_string(),
            texture_search_query: String::new(),
            show_ai_paths: true,
            show_actor_radii: false,
            ai_path_links: TrackedVec::new(),
            editor_mode: EditorMode::CameraSelection,
            selected_vertex_indices: Vec::new(),
            clip_points: Vec::new(),
            clip_normal_flipped: false,
            shape2d: Shape2D::new_box(256.0, 256.0),
            show_shape_editor: false,
            shape_editor_depth: 128.0,
            shape_editor_revolve_angle: 360.0,
            shape_editor_revolve_segs: 16,
            shape_editor_bevel_depth: 32.0,
            shape_editor_bevel_width: 32.0,
            movers: TrackedVec::new(),
            selected_mover_index: None,
            show_add_special_dialog: false,
            special_brush_flags: ut99_bsp::PF_SEMISOLID,
            special_is_subtractive: false,
            lighting_config: ut99_bsp::LightingConfig::default(),
            last_lighting_stats: None,
            show_lighting_dialog: false,
            show_lighting_studio_dialog: false,
            bsp_config: ut99_bsp::BspBuilderConfig::default(),
            last_bsp_report: None,
            show_build_options_dialog: false,
            show_scale_brush_dialog: false,
            scale_brush_factors: Vec3::ONE,
            scale_brush_uniform: true,
            scale_brush_mode: BrushScaleMode::GeometryVertices,
            scale_brush_adjust_textures: true,
            show_find_actors_dialog: false,
            find_actors_query: String::new(),
            find_actors_category: FindActorCategory::All,
            show_shear_brush_dialog: false,
            shear_brush_axis: ShearAxis::XZ,
            shear_brush_factor: 0.5,
            shear_brush_anchor_base: true,
            show_actor_align_dialog: false,
            show_map_statistics_dialog: false,
            show_actor_browser: false,
            show_group_browser: false,
            hidden_actor_groups: std::collections::HashSet::new(),
            brush_groups: std::collections::HashMap::new(),
            geometry_revision: 0,
            sampled_surface_uv: None,
            shape2d_undo_stack: Vec::new(),
            shape2d_redo_stack: Vec::new(),
            group_browser_new_name: String::new(),
            group_browser_search: String::new(),
            show_sound_browser: false,
            show_music_browser: false,
            class_tree: None,
            all_classes: Vec::new(),
            selected_class_name: None,
            selected_class_package: None,
            actor_browser_search: String::new(),
            audio_catalog: crate::audio_catalog::AudioCatalog::default(),
            sound_packages: Vec::new(),
            selected_sound_package: String::new(),
            loaded_sounds: Vec::new(),
            mylevel_sounds: Vec::new(),
            selected_sound_name: None,
            sound_browser_search: String::new(),
            music_packages: Vec::new(),
            selected_music_package: String::new(),
            loaded_music: None,
            static_meshes: TrackedVec::new(),
            selected_static_mesh_index: None,
            mesh_library: crate::mesh_manager::MeshLibrary::default(),
            selected_mesh_name: Some("Pillar_Cylinder_256".to_string()),
            mesh_browser_search: String::new(),
            mesh_browser_category: "All".to_string(),
            show_mesh_browser: false,
            procedural_simulator: ut99_bsp::ProceduralTextureSimulator::new_fire(64, 64),
            procedural_anim_playing: true,
            procedural_texture_name: "Fire_TorchFlame_01".to_string(),
            show_procedural_texture_dialog: false,
            projectors: TrackedVec::new(),
            selected_projector_index: None,
            active_projector_decals: TrackedVec::new(),
            show_decal_studio_dialog: false,
            decal_width: 128.0,
            decal_height: 128.0,
            decal_blend_mode: ProjectorBlendMode::Modulated,
            decal_max_distance: 512.0,
            decal_paint_mode: false,
            nav_graph: NavGraph::new(),
            bot_simulator: None,
            show_bot_simulator_window: false,
            sim_start_node: 0,
            sim_target_node: 1,
            map_audit_issues: Vec::new(),
            level_audit_snapshot: None,
            selected_issue_index: None,
            show_diagnostic_markers: true,
            show_level_doctor_window: false,
            build_progress: None,
            outliner_filter: String::new(),
            outliner_category: OutlinerCategory::All,
            inspector_panel_mode: InspectorPanelMode::Split,
            prefabs: crate::prefab::get_default_prefabs(),
            show_prefab_browser: false,
            show_event_links: true,
            camera_bookmarks: [None, None, None, None, None, None, None, None, None, None],
            show_camera_bookmarks_dialog: false,
            show_map_budget_dialog: false,
            auto_save_enabled: true,
            auto_save_interval_secs: 300.0,
            auto_save_timer: 0.0,
            auto_save_snapshots: Vec::new(),
            auto_save_t3d_slot: 0,
            auto_save_unr_slot: 0,
            auto_save_session_dir: autosave::new_session_dir(),
            auto_save_worker: None,
            auto_save_latest_seq: 0,
            map_is_dirty: false,
            show_autosave_dialog: false,
            show_unsaved_exit_modal: false,
            confirm_exit_without_saving: false,
            ut99_executable_path: "UnrealTournament/System/UnrealTournament.exe".to_string(),
            ut99_launch_args: "?game=Botpack.DeathMatchPlus -log".to_string(),
            show_play_settings_dialog: false,
            show_settings_dialog: false,
            active_settings_tab: SettingsTab::General,
            default_export_dir: None,
            wizard_output_dir: None,
            default_author: "Unknown Mapper".to_string(),
            language: crate::i18n::AppLanguage::English,
            theme: crate::gui::theme::AppTheme::UnrealSlateDark,
            show_trajectories: true,
            render_lux_heatmap: false,
            multi_selected_brushes: HashSet::new(),
            multi_selected_actors: HashSet::new(),
            active_marquee: None,
            recent_palette_commands: Vec::new(),
            selected_surfaces: HashSet::new(),
            memorized_surfaces: HashSet::new(),
            show_replace_textures_dialog: false,
            replace_texture_find: String::new(),
            replace_texture_with: String::new(),
            replace_texture_scope: TextureReplaceScope::EntireLevel,
            replace_texture_match_mode: TextureMatchMode::ExactName,
            ui_experience_mode: UiExperienceMode::Standard,
            show_action_wizards_dialog: false,
            show_visual_presets_dialog: false,
            active_wizard_tab: WizardTab::MoverLift,
            wizard_mover_height: 256.0,
            wizard_mover_horizontal: 0.0,
            wizard_mover_delay: 4.0,
            wizard_mover_tag: "Elevator_01".into(),
            wizard_teleport_enter_tag: "Teleport_A".into(),
            wizard_teleport_exit_tag: "Teleport_B".into(),
            wizard_teleport_offset_x: 512.0,
            wizard_teleport_offset_y: 0.0,
            wizard_teleport_offset_z: 0.0,
            wizard_basin_type: 0,
            wizard_basin_width: 512.0,
            wizard_basin_length: 512.0,
            wizard_basin_depth: 256.0,
            wizard_jump_height: 256.0,
            wizard_jump_dist: 512.0,
            wizard_room_width: 1024.0,
            wizard_room_length: 1024.0,
            wizard_room_height: 384.0,
            wizard_room_lights: 2,
            show_skybox_links: true,
            sky_zone_settings: SkyZoneSettings::default(),
            show_skybox_studio_dialog: false,
            selected_zone_tag: None,
            show_nav_coverage: false,
            show_audio_zones: true,
            show_acoustics_dialog: false,
            acoustic_settings: AcousticStudioSettings::default(),
            selected_sound_emitter_tag: None,
            distance_fog_enabled: false,
            fog_start: 200.0,
            fog_end: 2500.0,
            fog_density: 1.0,
            fog_color: [0.12, 0.14, 0.18, 1.0],
            show_zone_fog_studio_dialog: false,
            zone_fog_settings: ZoneFogStudioSettings::default(),
            builder_sheet_axis: ut99_bsp::SheetAxis::XY,
            builder_sheet_preset: ut99_bsp::SheetPreset::Normal,
            show_map_packaging_dialog: false,
            builder_arch_config: ArchConfig::default(),
            show_warpzones: true,
            show_warp_zone_studio_dialog: false,
            warp_zone_settings: WarpZoneStudioSettings::default(),
            show_anti_portals: true,
            show_gameplay_balance_dialog: false,
            show_zone_seal_dialog: false,
            show_zone_leaks_visual: true,
            mover_preview_time: 0.0,
            mover_preview_playing: false,
            show_mover_trajectories: true,
            auto_path_max_distance: 1200.0,
            auto_path_max_step: 54.0,
            auto_path_max_jump: 96.0,
            show_dispatcher_dialog: false,
            selected_dispatcher_name: None,
            dispatcher_settings: DispatcherStudioSettings::default(),
            skeletal_meshes: vec![ut99_bsp::SkeletalMesh::create_canonical_ut99_humanoid()],
            show_skeletal_bones: true,
            show_skeletal_studio_dialog: false,
            skeletal_settings: SkeletalStudioSettings::default(),
            show_skin_studio_dialog: false,
            skin_studio_project: ut99_bsp::SkinStudioProject::default(),
            skin_studio_settings: SkinStudioSettings::default(),
            show_package_extractor_dialog: false,
            package_extractor: PackageExtractorSettings::default(),
            show_uc_class_wizard_dialog: false,
            uc_scaffold_config: ut99_bsp::unrealscript_exec::UcScaffoldConfig::default(),
            uc_scaffold_status: String::new(),
            particle_emitters: Vec::new(),
            show_particles: true,
            show_particle_studio_dialog: false,
            particle_settings: ParticleStudioSettings::default(),
            show_bot_paths: true,
            show_bot_nav_studio_dialog: false,
            bot_nav_settings: BotNavStudioSettings::default(),
            last_autopath_report: None,
            show_architectural_studio_dialog: false,
            architectural_settings: ArchitecturalStudioSettings::default(),
            show_terrain_studio_dialog: false,
            terrain_settings: TerrainStudioSettings::default(),
            show_lod_collision_dialog: false,
            lod_collision_settings: LodCollisionStudioSettings::default(),
            show_cinematic_studio_dialog: false,
            cinematic_settings: CinematicStudioSettings::default(),
            show_light_effects_studio_dialog: false,
            light_effects_settings: LightEffectsStudioSettings::default(),
            show_zone_occlusion_studio_dialog: false,
            zone_occlusion_settings: ZoneOcclusionStudioSettings::default(),
            antiportals: Vec::new(),
            show_music_studio_dialog: false,
            music_studio_settings: MusicStudioSettings::default(),
            show_corona_studio_dialog: false,
            corona_studio_settings: CoronaStudioSettings::default(),
            show_fluid_studio_dialog: false,
            fluid_studio_settings: FluidStudioSettings::default(),
            show_ballistics_studio_dialog: false,
            ballistics_studio_settings: BallisticsStudioSettings::default(),
            show_teleporter_studio_dialog: false,
            teleporter_studio_settings: TeleporterStudioSettings::default(),
            show_translocator_studio_dialog: false,
            translocator_studio_settings: TranslocatorStudioSettings::default(),
            show_lift_pathing_studio_dialog: false,
            lift_pathing_studio_settings: LiftPathingStudioSettings::default(),
            show_water_surface_studio_dialog: false,
            water_surface_studio_settings: WaterSurfaceStudioSettings::default(),
            show_mover_studio_dialog: false,
            mover_studio_settings: MoverStudioSettings::default(),
            show_team_objective_dialog: false,
            team_objective_settings: TeamObjectiveStudioSettings::default(),
            show_destructible_studio_dialog: false,
            destructible_studio_settings: DestructibleStudioSettings::default(),
            show_kicker_studio_dialog: false,
            kicker_studio_settings: KickerStudioSettings::default(),
            show_bsp_quality_studio_dialog: false,
            bsp_quality_studio_settings: BspQualityStudioSettings::default(),
            show_loft_studio_dialog: false,
            loft_studio_settings: LoftStudioSettings::default(),
            show_stairway_studio_dialog: false,
            stairway_studio_settings: StairwayStudioSettings::default(),
            show_scatter_studio_dialog: false,
            scatter_studio_settings: ScatterStudioSettings::default(),
            show_light_shaft_dialog: false,
            light_shaft_settings: LightShaftStudioSettings::default(),
            show_track_studio_dialog: false,
            track_studio_settings: TrackStudioSettings::default(),
            show_surveillance_dialog: false,
            surveillance_settings: SurveillanceStudioSettings::default(),
            show_laser_grid_dialog: false,
            laser_grid_settings: LaserGridStudioSettings::default(),
            show_ventilator_dialog: false,
            ventilator_settings: VentilatorStudioSettings::default(),
            show_conveyor_dialog: false,
            conveyor_settings: ConveyorStudioSettings::default(),
            show_turret_dialog: false,
            turret_settings: TurretStudioSettings::default(),
            show_soundscape_dialog: false,
            soundscape_settings: SoundscapeStudioSettings::default(),
            show_hazard_chamber_dialog: false,
            hazard_chamber_settings: HazardChamberStudioSettings::default(),
            show_pipe_conduit_dialog: false,
            pipe_conduit_settings: PipeConduitStudioSettings::default(),
            show_hologram_dialog: false,
            hologram_settings: HologramStudioSettings::default(),
            show_weather_dialog: false,
            weather_settings: WeatherStudioSettings::default(),
            show_arena_rules_dialog: false,
            arena_rules_settings: ArenaRulesStudioSettings::default(),
            show_armor_stacking_dialog: false,
            armor_stacking_settings: ArmorStackingStudioSettings::default(),
            show_bot_jump_routes_dialog: false,
            bot_jump_routes_settings: BotJumpRoutesStudioSettings::default(),
            show_weapon_circuit_dialog: false,
            weapon_circuit_settings: WeaponCircuitStudioSettings::default(),
            show_powerup_control_dialog: false,
            powerup_control_settings: PowerupControlStudioSettings::default(),
            show_spawn_distribution_dialog: false,
            spawn_distribution_settings: SpawnDistributionStudioSettings::default(),
            show_domination_dialog: false,
            domination_settings: DominationStudioSettings::default(),
            show_assault_dialog: false,
            assault_settings: AssaultStudioSettings::default(),
            show_radar_dialog: false,
            radar_settings: RadarStudioSettings::default(),
            show_ctf_base_dialog: false,
            ctf_base_settings: CtfBaseStudioSettings::default(),
            show_team_cannon_dialog: false,
            team_cannon_settings: TeamCannonStudioSettings::default(),
            show_bio_trap_dialog: false,
            bio_trap_settings: BioTrapStudioSettings::default(),
            show_trigger_network_dialog: false,
            trigger_network_settings: TriggerNetworkStudioSettings::default(),
            show_elevator_dialog: false,
            elevator_settings: ElevatorStudioSettings::default(),
            show_zone_velocity_dialog: false,
            zone_velocity_settings: ZoneVelocityStudioSettings::default(),
            show_hazard_barrel_dialog: false,
            hazard_barrel_settings: HazardBarrelStudioSettings::default(),
            show_special_lit_dialog: false,
            special_lit_settings: SpecialLitStudioSettings::default(),
            show_sniper_vantage_dialog: false,
            sniper_vantage_settings: SniperVantageStudioSettings::default(),
            show_secret_door_dialog: false,
            secret_door_settings: SecretDoorStudioSettings::default(),
            show_teleporter_arch_dialog: false,
            teleporter_arch_settings: TeleporterArchStudioSettings::default(),
            show_fluid_basin_dialog: false,
            fluid_basin_settings: FluidBasinStudioSettings::default(),
            show_alarm_beacon_dialog: false,
            alarm_beacon_settings: AlarmBeaconStudioSettings::default(),
            show_supply_locker_dialog: false,
            supply_locker_settings: SupplyLockerStudioSettings::default(),
            show_skylight_dialog: false,
            skylight_settings: SkylightStudioSettings::default(),
            show_vent_duct_dialog: false,
            vent_duct_settings: VentDuctStudioSettings::default(),
            show_piston_dialog: false,
            piston_settings: PistonStudioSettings::default(),
            show_jump_boots_dialog: false,
            jump_boots_settings: JumpBootsStudioSettings::default(),
            show_force_field_dialog: false,
            force_field_settings: ForceFieldStudioSettings::default(),
            show_reactor_core_dialog: false,
            reactor_core_settings: ReactorCoreStudioSettings::default(),
            show_combat_sightlines: false,
            combat_coverage_report: None,
            show_zone_pvs: false,
            zone_pvs_matrix: None,
            show_shadow_penumbra: false,
            shadow_penumbra_profile: None,
            show_ctf_symmetry_dialog: false,
            ctf_symmetry_config: CtfSymmetryConfig::default(),
            show_command_palette: false,
            command_palette_search: String::new(),
            command_palette_selected_index: 0,
            show_help_dialog: false,
            help_active_tab: HelpTab::QuickStart,
            help_search_query: String::new(),
            show_shortcut_help_dialog: false,
            shortcut_help_search: String::new(),
            show_viewport_context_menu: false,
            context_menu_screen_pos: egui::pos2(0.0, 0.0),
            context_menu_world_pos: None,
            viewport_shading: crate::camera::ViewportShading::default(),
            viewport_shadings: [
                crate::camera::ViewportShading::Wireframe,
                crate::camera::ViewportShading::Wireframe,
                crate::camera::ViewportShading::Wireframe,
                crate::camera::ViewportShading::Lit,
            ],
            camera_speed_preset: 4,
            recent_maps: Vec::new(),
            show_minimap: false,

            // Level Properties
            show_level_properties_dialog: false,
            level_title: "Untitled Arena".to_string(),
            level_author: "Unknown Mapper".to_string(),
            level_ideal_player_count: "4-8".to_string(),
            level_song: String::new(),
            level_song_section: 0,
            level_game_type: "Botpack.DeathMatchPlus".to_string(),
            level_enter_text: String::new(),
            level_ambient_brightness: 0,
            level_ambient_hue: 0,
            level_ambient_saturation: 255,

            // Scale Entire Map
            show_scale_map_dialog: false,
            scale_map_factor: glam::Vec3::ONE,
            scale_map_actors: true,
            scale_map_lights: true,

            // Package Dependency Inspector & Distribution Manifest
            show_package_manifest_dialog: false,
            cached_map_manifest: None,
            packaging_map_name_edit: String::new(),

            // Interactive Visual Save & Export Feedback System
            save_feedback: None,
            pending_save_op: None,
            save_worker_rx: None,

            // UnrealScript Class Source Viewer & DefaultProperties Live Inspector
            show_uc_viewer_dialog: false,
            uc_viewer_class: "Actor".to_string(),
            uc_viewer_search: String::new(),
            uc_viewer_tab: crate::unrealscript::UcViewerTab::Source,
            uc_editable_defaults: Vec::new(),
            uc_source_cache: String::new(),
            uc_class_metadata: None,

            // UX Phase D: UI Polish, Inspector & UV Precision
            texture_thumbnail_size: 110.0,
            numeric_edit_brush_loc_initial: None,
            numeric_edit_brush_rot_initial: None,
            numeric_edit_brush_scale_initial: None,
            numeric_edit_actor_loc_initial: None,
            uv_edit_initial_polys: None,
            uv_drag_pan_u: 0.0,
            uv_drag_pan_v: 0.0,
            uv_drag_scale_u: 1.0,
            uv_drag_scale_v: 1.0,
            uv_drag_rot: 0.0,

            // Cycle 6 (BG-16): Master Browser, Data-Driven Smart Prefabs, Freehand Polygon & Background Image
            show_master_browser: false,
            master_browser_tab: MasterBrowserTab::Textures,
            smart_prefab_category: crate::prefab_templates::SmartPrefabCategory::Architecture,
            smart_prefab_params: crate::prefab_templates::SmartPrefabParams::default(),
            selected_smart_prefab_id: "arch_hollow_room".to_string(),
            freehand_points: Vec::new(),
            freehand_extrude_depth: 128.0,
            background_image_path: None,
            background_display_mode: BackgroundDisplayMode::Center,
            target_engine: TargetEngine::Ut99Standard,
            keymap_profile: KeymapProfile::Modern,
            show_mover_polys: true,
            shape_editor_selected_segment: 0,
            active_gizmo_delta: Vec3::ZERO,
            active_gizmo_angle_deg: 0.0,
            gizmo_drag_initial_brushes: Vec::new(),
            gizmo_drag_initial_actors: Vec::new(),
            math_expression_input: String::new(),
            last_math_result: None,
            pending_dirty_action: None,
            confirmed_close_tab_index: None,
            start_maximized: true,
            is_fullscreen: false,
            pending_toggle_fullscreen: false,
            first_run_completed: false,
            show_welcome_wizard: false,
            welcome_wizard_step: 0,
            show_map_wizard_dialog: false,
            map_wizard_config: ut99_bsp::MapWizardConfig::default(),
            map_wizard_step: 0,
            map_wizard_lock_game_mode: true,
            map_wizard_archetype_filter: None,
            official_dna_mode_filter: "ALL".to_string(),
            selected_official_dna_map: String::new(),
            available_official_maps: Vec::new(),
            last_cloned_dna: None,
            official_corpus_stats: Vec::new(),
            official_corpus_profiles: Vec::new(),
            wizard_minimap_selected: None,
            wizard_minimap_tool: WizardMinimapTool::Inspect,
            wizard_minimap_connect_source: None,
            wizard_minimap_drag_state: WizardMinimapDragState::None,
            wizard_minimap_add_object_kind: "Weapon".to_string(),
            wizard_minimap_pan: egui::Vec2::ZERO,
            wizard_minimap_zoom: 1.0,
            wizard_minimap_grid_snap: 64.0,
            wizard_minimap_show_bot_paths: true,
            wizard_preset_json_modal: None,
        };

        state.load_actor_classes();
        state.refresh_audio_resources();

        state.log("Forge99 — 64-bit Level Editor initialized.");
        state.log("Command subsystem online. Ready for level editing.");
        state.log("Active Red Builder Brush ready at (0, 0, 0).");
        state
    }
}


impl EditorState {
    /// Ensures cached world brush AABBs and total XY extents match `(map_instance_id, world_brushes.rev())`.
    pub(crate) fn ensure_world_brush_bounds(&self) {
        let key = (self.map_instance_id, self.world_brushes.rev());
        let mut cache = self.scene_bounds_cache.borrow_mut();
        if cache.world_brushes_key == key && cache.world_brushes.len() == self.world_brushes.len() {
            return;
        }
        cache.world_brushes.clear();
        cache.world_brushes.reserve(self.world_brushes.len());
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (-1024.0_f32, 1024.0_f32, -1024.0_f32, 1024.0_f32);
        for brush in &self.world_brushes {
            let (b_min, b_max) = brush.world_bounds();
            min_x = min_x.min(b_min.x);
            max_x = max_x.max(b_max.x);
            min_y = min_y.min(b_min.y);
            max_y = max_y.max(b_max.y);
            cache.world_brushes.push((b_min, b_max));
        }
        cache.world_brushes_total_xy = (min_x, max_x, min_y, max_y);
        cache.world_brushes_key = key;
    }

    /// Ensures cached mover brush AABBs match `(map_instance_id, movers.rev())`.
    pub(crate) fn ensure_mover_bounds(&self) {
        let key = (self.map_instance_id, self.movers.rev());
        let mut cache = self.scene_bounds_cache.borrow_mut();
        if cache.movers_key == key && cache.movers.len() == self.movers.len() {
            return;
        }
        cache.movers.clear();
        cache.movers.reserve(self.movers.len());
        for mover in &self.movers {
            cache.movers.push(mover.brush.world_bounds());
        }
        cache.movers_key = key;
    }

    /// Ensures cached static mesh AABBs match `(map_instance_id, static_meshes.rev())`.
    pub(crate) fn ensure_static_mesh_bounds(&self) {
        let key = (self.map_instance_id, self.static_meshes.rev());
        let mut cache = self.scene_bounds_cache.borrow_mut();
        if cache.static_meshes_key == key && cache.static_meshes.len() == self.static_meshes.len() {
            return;
        }
        cache.static_meshes.clear();
        cache.static_meshes.reserve(self.static_meshes.len());
        for mesh in &self.static_meshes {
            cache.static_meshes.push(mesh.world_bounds());
        }
        cache.static_meshes_key = key;
    }

    /// Returns cached world-space AABB `(min, max)` for the brush at `index`.
    pub(crate) fn world_brush_bound(&self, index: usize) -> Option<(Vec3, Vec3)> {
        self.ensure_world_brush_bounds();
        self.scene_bounds_cache.borrow().world_brushes.get(index).copied()
    }

    pub fn open_help_tab(&mut self, tab: HelpTab) {
        self.help_active_tab = tab;
        self.show_help_dialog = true;
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        let s = msg.into();
        tracing::info!("{s}");
        self.console_messages.push(s);
    }


    /// Evaluates `self.math_expression_input` via [`ut99_bsp::evaluate_expression`], stores the result in
    /// `self.last_math_result`, logs it to the console, and shows a toast (`UX-01`).
    pub fn evaluate_math_expression_input(&mut self) -> Result<f32, String> {
        let expr = self.math_expression_input.trim().to_string();
        if expr.is_empty() {
            return Err("Empty math expression".to_string());
        }
        match ut99_bsp::evaluate_expression(&expr) {
            Ok(res) => {
                self.last_math_result = Some(res);
                self.log(format!("Formula: {} = {}", expr, res));
                self.toast_info(format!("{} = {}", expr, res));
                Ok(res)
            }
            Err(e) => {
                self.log(format!("Formula error in '{}': {}", expr, e));
                self.toast_warning(format!("Formula error: {}", e));
                Err(e)
            }
        }
    }

    /// Applies resolved adaptive hardware budgets to subsystems and diagnostics.
    pub fn apply_hardware_budgets(&mut self, budgets: crate::graphics::HardwareBudgets) {
        let _ = self
            .texture_manager
            .set_memory_budget_bytes(budgets.texture_manager_budget_bytes);
        let d = &mut self.graphics_diagnostics;
        d.resolved_tier = budgets.tier;
        d.diffuse_texture_layer_cap = budgets.diffuse_texture_layers;
        d.lightmap_atlas_dim = budgets.lightmap_atlas_dim;
        d.retained_vertex_budget_bytes = budgets.max_retained_vertex_bytes;
        d.texture_manager_budget_bytes = budgets.texture_manager_budget_bytes;
        d.gui_texture_backup_soft_cap_bytes = budgets.gui_texture_backup_soft_cap_bytes;
        d.texture_manager_used_bytes = self.texture_manager.total_memory_usage_bytes();
    }

}

/// Converts Euler angles in degrees (pitch, yaw, roll) into a standard 3x3 orthonormal rotation matrix.
pub fn euler_degrees_to_mat3(pitch: f32, yaw: f32, roll: f32) -> glam::Mat3 {
    let p = pitch.to_radians();
    let y = yaw.to_radians();
    let r = roll.to_radians();
    glam::Mat3::from_rotation_z(y) * glam::Mat3::from_rotation_y(p) * glam::Mat3::from_rotation_x(r)
}


/// Converts canonical Unreal Engine 1 integer Rotator angles (pitch, yaw, roll where 65536 = 360°)
/// into a standard 3x3 orthonormal rotation matrix.
pub fn unreal_rotator_to_mat3(pitch: i32, yaw: i32, roll: i32) -> glam::Mat3 {
    let to_rad = std::f32::consts::PI * 2.0 / 65536.0;
    let p = pitch as f32 * to_rad;
    let y = yaw as f32 * to_rad;
    let r = roll as f32 * to_rad;
    glam::Mat3::from_rotation_z(y) * glam::Mat3::from_rotation_y(p) * glam::Mat3::from_rotation_x(r)
}

mod actors;
mod audio;
mod autosave;
mod brush_io;
mod builder;
mod cinematics;
mod console;
pub(crate) mod csg;
pub use csg::SpecialBrushPreset;
mod dispatchers;
mod gameplay_analysis;
mod gameplay_mechanics;
mod geometry;
mod history_dispatch;
pub mod io;
pub use io::{normalize_path_for_windows_explorer, reveal_in_file_explorer};
mod level_doctor;
mod mesh_lod;
mod mesh_obj;
mod movers;
mod navigation;
mod particles;
mod picking;
mod preferences;
mod prefabs_terrain;
mod projectors;
mod selection;
mod skeletal;
mod studios;
pub(crate) mod surfaces;
pub use surfaces::{
    compute_replaced_texture, invert_brush_geometry, lux_to_heatmap_color, matches_texture,
    mirror_brush_geometry, scale_brush_geometry, shear_brush_geometry,
};
mod toasts;
mod unrealscript;
mod viewport;
mod wizard;
mod zones;

#[cfg(test)]
mod audit_regression_tests;
