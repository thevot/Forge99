mod audio_catalog;
mod playtest;
mod frame_schedule;
mod surface_recovery;
mod gui_texture_backup;
pub mod app;
pub mod graphics;
pub mod camera;
pub mod compiler;
pub mod document;
pub mod editor_state;
pub mod grid;
pub mod gui;
pub mod history;
pub mod prefab;
pub mod prefab_templates;
pub mod raycast;
pub mod transform_session;
pub mod renderer;
pub mod mesh_manager;
pub mod texture_manager;
pub mod unrealscript;
pub mod preferences;
pub mod culling;
pub mod i18n;

pub use app::EditorApp;
pub use ut99_format::TrackedVec;
pub use i18n::{register_runtime_translations, tr, AppLanguage};
pub use camera::{Camera, CameraBookmark, ViewportLayout, ViewportType};
pub use compiler::CompilerBridge;
pub use culling::{Frustum, FrustumPlane};
pub use document::{DocumentSummary, LevelDocument};
pub use editor_state::{
    normalize_path_for_windows_explorer, reveal_in_file_explorer,
    BackgroundDisplayMode, EditorMode, EditorState, HelpTab, InspectorPanelMode, MapBudgetStats,
    MasterBrowserTab, OutlinerCategory, WizardMinimapSelection, WizardMinimapTool, WorkspacePreset,
};
pub use history::{Action, History};
pub use mesh_manager::{EditorMeshResource, MeshLibrary};
pub use preferences::EditorPreferences;
pub use prefab::Prefab;
pub use prefab_templates::{
    SmartPrefabCategory, SmartPrefabLibrary, SmartPrefabOutput, SmartPrefabParams,
    SmartPrefabTemplate,
};
pub use raycast::{GizmoAxis, GizmoMode, Ray};
pub use texture_manager::TextureManager;
pub use unrealscript::{get_class_metadata, UcClassMetadata, UcFunction, UcVariable, UcViewerTab};
pub use gui::icons::{self, Icon};
pub use gui::theme::AppTheme;



