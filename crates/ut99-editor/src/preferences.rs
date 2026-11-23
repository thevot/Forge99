fn default_drag_threshold() -> f32 { 6.0 }
fn default_ut99_executable_path() -> String { "UnrealTournament/System/UnrealTournament.exe".to_string() }
fn default_ut99_launch_args() -> String { "?game=Botpack.DeathMatchPlus -log".to_string() }
fn default_author() -> String { "Unknown Mapper".to_string() }
fn default_auto_save_interval() -> f32 { 600.0 }
fn default_true() -> bool { true }
fn default_one() -> f32 { 1.0 }
fn default_frame_latency() -> u32 { 2 }

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::editor_state::EditorState;

/// Persistent user preferences for the UT99 map editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorPreferences {
    /// UI scale factor (e.g. 1.0, 1.25)
    pub ui_scale: f32,
    /// Spatial grid snapping step (uu)
    pub grid_step: f32,
    /// Whether spatial grid snapping is active
    pub snap_to_grid: bool,
    /// Rotational snapping step in degrees
    pub rotation_snap_deg: f32,
    /// Whether rotation snapping is active
    pub snap_to_rotation: bool,
    /// Camera flight speed preset index (1..=8)
    pub camera_speed_preset: u8,
    /// Perspective camera Field of View in degrees
    pub camera_fov_deg: f32,
    /// Active viewport shading mode ("Wireframe", "Zone/Portal", "Unlit", "Lit", "Depth Complexity")
    pub viewport_shading: String,
    /// Whether the minimap / radar overlay is visible
    pub show_minimap: bool,
    /// Whether actor collision & lighting radii are rendered
    pub show_actor_radii: bool,
    /// Dimension of texture thumbnails in pixels
    pub texture_thumbnail_size: f32,
    /// Recent maps history
    pub recent_maps: Vec<PathBuf>,
    /// UI experience mode ("Standard" vs "Architect")
    #[serde(default)]
    pub ui_experience_mode: crate::editor_state::UiExperienceMode,
    /// Target engine compatibility profile
    #[serde(default)]
    pub target_engine: crate::editor_state::TargetEngine,
    /// Keyboard shortcut profile ("Modern" vs "ClassicUnrealEd2")
    #[serde(default)]
    pub keymap_profile: crate::editor_state::KeymapProfile,
    /// Active gizmo coordinate space ("World" vs "Local")
    #[serde(default)]
    pub gizmo_space: crate::editor_state::GizmoCoordinateSpace,
    /// Minimum mouse travel in pixels to begin a direct viewport drag
    #[serde(default = "default_drag_threshold")]
    pub drag_threshold_px: f32,

    // Centralized New User Settings
    /// Default custom export directory for maps and assets
    #[serde(default)]
    pub default_export_dir: Option<PathBuf>,
    /// Destination for wizard-generated maps (default: Documents/Forge99/GeneratedMaps)
    #[serde(default)]
    pub wizard_output_dir: Option<PathBuf>,
    /// Path to UnrealTournament.exe binary
    #[serde(default = "default_ut99_executable_path")]
    pub ut99_executable_path: String,
    /// Default launch arguments for playtesting
    #[serde(default = "default_ut99_launch_args")]
    pub ut99_launch_args: String,
    /// Interface localization language
    #[serde(default)]
    pub language: crate::i18n::AppLanguage,
    /// Visual UI theme
    #[serde(default)]
    pub theme: crate::gui::theme::AppTheme,
    /// Default author name for new maps and generator templates
    #[serde(default = "default_author")]
    pub default_author: String,
    /// Whether periodic auto-save is enabled
    #[serde(default = "default_true")]
    pub auto_save_enabled: bool,
    /// Interval in seconds between automatic saves
    #[serde(default = "default_auto_save_interval")]
    pub auto_save_interval_secs: f32,
    /// Dynamic view frustum culling engine active
    #[serde(default = "default_true")]
    pub frustum_culling_enabled: bool,
    /// Hardware scaling profile (`Auto`, `High`, `Balanced`, `Constrained`)
    #[serde(default)]
    pub hardware_profile: crate::graphics::HardwareProfile,
    /// Preferred GPU graphics backend (`Auto`, `Vulkan`, `Dx12`, `Gl`, `Metal`)
    #[serde(default)]
    pub graphics_backend: crate::graphics::GraphicsBackendPreference,
    /// Preferred GPU adapter name (`None` = automatic ranked selection)
    #[serde(default)]
    pub preferred_adapter_name: Option<String>,
    /// 3D viewport render resolution scale (`0.5..=1.0`; UI stays native 100%)
    #[serde(default = "default_one")]
    pub viewport_render_scale: f32,
    /// Swapchain presentation mode (`AutoVsync`, `Fifo`, `Mailbox`, `Immediate`)
    #[serde(default)]
    pub present_mode: crate::graphics::PresentModePreference,
    /// Maximum frames queued ahead on the swapchain (`1` or `2`)
    #[serde(default = "default_frame_latency")]
    pub max_frame_latency: u32,
    /// Real-time distance fog in 3D viewport
    #[serde(default)]
    pub distance_fog_enabled: bool,
    /// Whether the first-run onboarding welcome wizard has been completed
    #[serde(default)]
    pub first_run_completed: bool,
    /// Whether the editor window should start maximized to fill the screen
    #[serde(default = "default_true")]
    pub start_maximized: bool,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            grid_step: 16.0,
            snap_to_grid: true,
            rotation_snap_deg: 15.0,
            snap_to_rotation: true,
            camera_speed_preset: 4,
            camera_fov_deg: 75.0,
            viewport_shading: "Lit".to_string(),
            show_minimap: false,
            show_actor_radii: false,
            texture_thumbnail_size: 110.0,
            recent_maps: Vec::new(),
            ui_experience_mode: crate::editor_state::UiExperienceMode::Standard,
            target_engine: crate::editor_state::TargetEngine::Ut99Standard,
            keymap_profile: crate::editor_state::KeymapProfile::Modern,
            gizmo_space: crate::editor_state::GizmoCoordinateSpace::World,
            drag_threshold_px: 6.0,
            default_export_dir: None,
            wizard_output_dir: None,
            ut99_executable_path: default_ut99_executable_path(),
            ut99_launch_args: default_ut99_launch_args(),
            language: crate::i18n::AppLanguage::English,
            theme: crate::gui::theme::AppTheme::UnrealSlateDark,
            default_author: default_author(),
            auto_save_enabled: true,
            auto_save_interval_secs: 600.0,
            frustum_culling_enabled: true,
            hardware_profile: crate::graphics::HardwareProfile::Auto,
            graphics_backend: crate::graphics::GraphicsBackendPreference::Auto,
            preferred_adapter_name: None,
            viewport_render_scale: 1.0,
            present_mode: crate::graphics::PresentModePreference::AutoVsync,
            max_frame_latency: 2,
            distance_fog_enabled: false,
            first_run_completed: false,
            start_maximized: true,
        }
    }
}

impl EditorPreferences {
    /// Captures the current configuration from an `EditorState`.
    pub fn from_editor_state(state: &EditorState) -> Self {
        Self {
            ui_scale: state.ui_scale,
            grid_step: state.grid_step,
            snap_to_grid: state.snap_to_grid,
            rotation_snap_deg: state.rotation_snap_deg,
            snap_to_rotation: state.snap_to_rotation,
            camera_speed_preset: state.camera_speed_preset,
            camera_fov_deg: state.camera.fov_y.to_degrees(),
            viewport_shading: state.viewport_shading.label().to_string(),
            show_minimap: state.show_minimap,
            show_actor_radii: state.show_actor_radii,
            texture_thumbnail_size: state.texture_thumbnail_size,
            recent_maps: state.recent_maps.clone(),
            ui_experience_mode: state.ui_experience_mode,
            target_engine: state.target_engine,
            keymap_profile: state.keymap_profile,
            gizmo_space: state.gizmo_space,
            drag_threshold_px: state.drag_threshold_px,
            default_export_dir: state.default_export_dir.clone(),
            wizard_output_dir: state.wizard_output_dir.clone(),
            ut99_executable_path: state.ut99_executable_path.clone(),
            ut99_launch_args: state.ut99_launch_args.clone(),
            language: state.language,
            theme: state.theme,
            default_author: state.default_author.clone(),
            auto_save_enabled: state.auto_save_enabled,
            auto_save_interval_secs: state.auto_save_interval_secs,
            frustum_culling_enabled: state.frustum_culling_enabled,
            hardware_profile: state.hardware_profile,
            graphics_backend: state.graphics_backend,
            preferred_adapter_name: state.preferred_adapter_name.clone(),
            viewport_render_scale: state.viewport_render_scale,
            present_mode: state.present_mode,
            max_frame_latency: state.max_frame_latency,
            distance_fog_enabled: state.distance_fog_enabled,
            first_run_completed: state.first_run_completed,
            start_maximized: state.start_maximized,
        }
    }

    /// Applies saved preferences into an active `EditorState`.
    pub fn apply_to_editor_state(&self, state: &mut EditorState) {
        state.ui_scale = self.ui_scale.clamp(0.5, 3.0);
        state.grid_step = self.grid_step.clamp(1.0, 4096.0);
        state.grid.step = state.grid_step;
        state.snap_to_grid = self.snap_to_grid;
        state.rotation_snap_deg = self.rotation_snap_deg.clamp(1.0, 90.0);
        state.snap_to_rotation = self.snap_to_rotation;
        state.set_camera_speed_preset(self.camera_speed_preset.clamp(1, 8));
        state.camera.fov_y = self.camera_fov_deg.clamp(30.0, 140.0).to_radians();
        state.show_minimap = self.show_minimap;
        state.show_actor_radii = self.show_actor_radii;
        state.texture_thumbnail_size = self.texture_thumbnail_size.clamp(48.0, 256.0);
        state.recent_maps = self.recent_maps.clone();
        state.ui_experience_mode = self.ui_experience_mode;
        state.target_engine = self.target_engine;
        state.keymap_profile = self.keymap_profile;
        state.gizmo_space = self.gizmo_space;
        state.drag_threshold_px = self.drag_threshold_px.clamp(2.0, 32.0);
        state.default_export_dir = self.default_export_dir.clone();
        state.wizard_output_dir = self.wizard_output_dir.clone();
        state.ut99_executable_path = self.ut99_executable_path.clone();
        state.sync_audio_installation();
        state.ut99_launch_args = self.ut99_launch_args.clone();
        state.language = self.language;
        state.theme = self.theme;
        state.default_author = self.default_author.clone();
        state.auto_save_enabled = self.auto_save_enabled;
        state.auto_save_interval_secs = self.auto_save_interval_secs.max(60.0);
        state.frustum_culling_enabled = self.frustum_culling_enabled;
        state.hardware_profile = self.hardware_profile;
        state.graphics_backend = self.graphics_backend;
        state.preferred_adapter_name = self.preferred_adapter_name.clone();
        state.viewport_render_scale = self.viewport_render_scale.clamp(0.5, 1.0);
        state.present_mode = self.present_mode;
        state.max_frame_latency = self.max_frame_latency.clamp(1, 2);
        state.distance_fog_enabled = self.distance_fog_enabled;
        state.first_run_completed = self.first_run_completed;
        state.start_maximized = self.start_maximized;

        match self.viewport_shading.as_str() {
            "Wireframe" => state.viewport_shading = crate::camera::ViewportShading::Wireframe,
            "Zone/Portal" | "ZonePortal" => state.viewport_shading = crate::camera::ViewportShading::ZonePortal,
            "Unlit" => state.viewport_shading = crate::camera::ViewportShading::Unlit,
            "Lit" => state.viewport_shading = crate::camera::ViewportShading::Lit,
            "Depth Complexity" | "DepthComplexity" => state.viewport_shading = crate::camera::ViewportShading::DepthComplexity,
            "Texture Usage" | "TextureUsage" => state.viewport_shading = crate::camera::ViewportShading::TextureUsage,
            "BSP Cuts" | "BspCuts" => state.viewport_shading = crate::camera::ViewportShading::BspCuts,
            _ => {},
        }
    }

    /// Returns the standard path for editor preferences JSON file.
    /// Checks portable local directory first, then falls back to the OS user configuration directory
    /// (`%APPDATA%/Forge99/forge99_prefs.json` on Windows, `~/.config/forge99/forge99_prefs.json` on Linux,
    /// `~/Library/Application Support/Forge99/forge99_prefs.json` on macOS).
    pub fn default_path() -> PathBuf {
        let forge_path = PathBuf::from("forge99_prefs.json");
        if forge_path.exists() {
            return forge_path;
        }
        if let Some(os_dir) = Self::os_config_dir() {
            return os_dir.join("forge99_prefs.json");
        }
        forge_path
    }

    /// Resolves the OS-native per-user configuration directory for Forge99.
    pub fn os_config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                if !appdata.trim().is_empty() {
                    return Some(PathBuf::from(appdata).join("Forge99"));
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            if let Ok(home) = std::env::var("HOME") {
                if !home.trim().is_empty() {
                    return Some(
                        PathBuf::from(home)
                            .join("Library")
                            .join("Application Support")
                            .join("Forge99"),
                    );
                }
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
                if !xdg.trim().is_empty() {
                    return Some(PathBuf::from(xdg).join("forge99"));
                }
            }
            if let Ok(home) = std::env::var("HOME") {
                if !home.trim().is_empty() {
                    return Some(PathBuf::from(home).join(".config").join("forge99"));
                }
            }
        }
        None
    }

    /// Serializes and writes preferences to a JSON file, creating parent directories if needed.
    pub fn save_to_path(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        std::fs::write(path, json)
    }

    /// Reads and deserializes preferences from a JSON file.
    pub fn load_from_path(path: &Path) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        let prefs: Self = serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(prefs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_maximized_defaults_to_true() {
        let prefs = EditorPreferences::default();
        assert!(prefs.start_maximized, "start_maximized must default to true");

        let mut state = EditorState::default();
        assert!(state.start_maximized, "EditorState start_maximized must default to true");
        assert!(!state.is_fullscreen, "is_fullscreen must default to false");
        assert!(!state.pending_toggle_fullscreen, "pending_toggle_fullscreen must default to false");

        // Verify preferences apply and roundtrip with state
        state.start_maximized = false;
        let prefs2 = EditorPreferences::from_editor_state(&state);
        assert!(!prefs2.start_maximized);

        prefs.apply_to_editor_state(&mut state);
        assert!(state.start_maximized);
    }

    #[test]
    fn test_start_maximized_serde_backwards_compatibility() {
        // Old JSON without `start_maximized` field should deserialize with `start_maximized == true`
        let json_old = r#"{
            "ui_scale": 1.0,
            "grid_step": 16.0,
            "snap_to_grid": true,
            "rotation_snap_deg": 15.0,
            "snap_to_rotation": true,
            "camera_speed_preset": 4,
            "camera_fov_deg": 75.0,
            "viewport_shading": "Lit",
            "show_minimap": false,
            "show_actor_radii": false,
            "texture_thumbnail_size": 110.0,
            "recent_maps": []
        }"#;

        let parsed: EditorPreferences = serde_json::from_str(json_old).expect("Failed to deserialize old JSON");
        assert!(parsed.start_maximized, "Missing start_maximized must default to true");
    }
}
