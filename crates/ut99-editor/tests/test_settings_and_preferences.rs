use std::path::PathBuf;
use ut99_editor::editor_state::{EditorState, UiExperienceMode, TargetEngine, KeymapProfile, GizmoCoordinateSpace, SettingsTab};
use ut99_editor::preferences::EditorPreferences;
use ut99_editor::i18n::{register_runtime_translations, tr, AppLanguage};
use ut99_editor::gui::theme::{apply_theme_variant, check_wcag_aa, AppTheme, ThemeColors};
use ut99_editor::camera::ViewportShading;

#[test]
fn test_preferences_roundtrip_all_settings() {
    let mut state = EditorState::default();

    // Configure custom values for all settings
    state.default_author = "NeoMapper99".to_string();
    state.default_export_dir = Some(PathBuf::from("D:/UT99_Custom_Export/Maps"));
    state.ut99_executable_path = "D:/Games/UnrealTournament/System/UnrealTournament.exe".to_string();
    state.ut99_launch_args = "?game=Botpack.CTFGame -log -windowed".to_string();
    state.language = AppLanguage::Italian;
    state.theme = AppTheme::ClassicUnrealEd;
    state.auto_save_enabled = false;
    state.auto_save_interval_secs = 1200.0;
    state.frustum_culling_enabled = false;
    state.hardware_profile = ut99_editor::graphics::HardwareProfile::Constrained;
    state.graphics_backend = ut99_editor::graphics::GraphicsBackendPreference::Gl;
    state.preferred_adapter_name = Some("Intel(R) Graphics".to_string());
    state.viewport_render_scale = 0.75;
    state.present_mode = ut99_editor::graphics::PresentModePreference::Mailbox;
    state.max_frame_latency = 1;
    state.distance_fog_enabled = true;
    state.ui_scale = 1.35;
    state.grid_step = 64.0;
    state.snap_to_grid = false;
    state.rotation_snap_deg = 45.0;
    state.snap_to_rotation = false;
    state.camera_speed_preset = 7;
    state.camera.fov_y = 90.0_f32.to_radians();
    state.viewport_shading = ViewportShading::Wireframe;
    state.show_minimap = true;
    state.show_actor_radii = true;
    state.texture_thumbnail_size = 160.0;
    state.ui_experience_mode = UiExperienceMode::Architect;
    state.target_engine = TargetEngine::Unreal227;
    state.keymap_profile = KeymapProfile::ClassicUnrealEd2;
    state.gizmo_space = GizmoCoordinateSpace::Local;
    state.drag_threshold_px = 12.0;

    // Serialize to temporary file
    let temp_file = std::env::temp_dir().join("test_settings_roundtrip_prefs.json");
    let saved = state.save_preferences_to_path(&temp_file);
    assert!(saved, "Preferences must save successfully");
    assert!(temp_file.exists());

    // Restore into a pristine EditorState
    let mut restored = EditorState::default();
    let loaded = restored.load_preferences_from_path(&temp_file);
    assert!(loaded, "Preferences must load successfully");

    // Verify all fields are accurately restored
    assert_eq!(restored.default_author, "NeoMapper99");
    assert_eq!(restored.default_export_dir, Some(PathBuf::from("D:/UT99_Custom_Export/Maps")));
    assert_eq!(restored.ut99_executable_path, "D:/Games/UnrealTournament/System/UnrealTournament.exe");
    assert_eq!(restored.ut99_launch_args, "?game=Botpack.CTFGame -log -windowed");
    assert_eq!(restored.language, AppLanguage::Italian);
    assert_eq!(restored.theme, AppTheme::ClassicUnrealEd);
    assert_eq!(restored.auto_save_enabled, false);
    assert_eq!(restored.auto_save_interval_secs, 1200.0);
    assert_eq!(restored.frustum_culling_enabled, false);
    assert_eq!(restored.hardware_profile, ut99_editor::graphics::HardwareProfile::Constrained);
    assert_eq!(restored.graphics_backend, ut99_editor::graphics::GraphicsBackendPreference::Gl);
    assert_eq!(restored.preferred_adapter_name.as_deref(), Some("Intel(R) Graphics"));
    assert!((restored.viewport_render_scale - 0.75).abs() < 1e-6);
    assert_eq!(restored.present_mode, ut99_editor::graphics::PresentModePreference::Mailbox);
    assert_eq!(restored.max_frame_latency, 1);
    assert_eq!(restored.distance_fog_enabled, true);
    assert_eq!(restored.ui_scale, 1.35);
    assert_eq!(restored.grid_step, 64.0);
    assert_eq!(restored.grid.step, 64.0);
    assert_eq!(restored.snap_to_grid, false);
    assert_eq!(restored.rotation_snap_deg, 45.0);
    assert_eq!(restored.snap_to_rotation, false);
    assert_eq!(restored.camera_speed_preset, 7);
    assert!((restored.camera.fov_y.to_degrees() - 90.0).abs() < 1e-3);
    assert_eq!(restored.viewport_shading, ViewportShading::Wireframe);
    assert_eq!(restored.show_minimap, true);
    assert_eq!(restored.show_actor_radii, true);
    assert_eq!(restored.texture_thumbnail_size, 160.0);
    assert_eq!(restored.ui_experience_mode, UiExperienceMode::Architect);
    assert_eq!(restored.target_engine, TargetEngine::Unreal227);
    assert_eq!(restored.keymap_profile, KeymapProfile::ClassicUnrealEd2);
    assert_eq!(restored.gizmo_space, GizmoCoordinateSpace::Local);
    assert_eq!(restored.drag_threshold_px, 12.0);

    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_preferences_backward_compatibility_missing_fields() {
    // Legacy JSON without the new fields
    let legacy_json = r#"{
        "ui_scale": 1.1,
        "grid_step": 32.0,
        "snap_to_grid": true,
        "rotation_snap_deg": 15.0,
        "snap_to_rotation": true,
        "camera_speed_preset": 5,
        "camera_fov_deg": 80.0,
        "viewport_shading": "Unlit",
        "show_minimap": false,
        "show_actor_radii": false,
        "texture_thumbnail_size": 120.0,
        "recent_maps": []
    }"#;

    let prefs: EditorPreferences = serde_json::from_str(legacy_json)
        .expect("Legacy preferences JSON must parse cleanly with default values");

    let mut state = EditorState::default();
    prefs.apply_to_editor_state(&mut state);

    assert_eq!(state.ui_scale, 1.1);
    assert_eq!(state.grid_step, 32.0);
    assert_eq!(state.camera_speed_preset, 5);
    // New fields should have received their default values
    assert_eq!(state.default_author, "Unknown Mapper");
    assert_eq!(state.default_export_dir, None);
    assert_eq!(state.language, AppLanguage::English);
    assert_eq!(state.theme, AppTheme::UnrealSlateDark);
    assert_eq!(state.auto_save_enabled, true);
    assert_eq!(state.auto_save_interval_secs, 600.0);
    assert_eq!(state.frustum_culling_enabled, true);
    assert_eq!(state.hardware_profile, ut99_editor::graphics::HardwareProfile::Auto);
    assert_eq!(state.graphics_backend, ut99_editor::graphics::GraphicsBackendPreference::Auto);
    assert_eq!(state.preferred_adapter_name, None);
    assert!((state.viewport_render_scale - 1.0).abs() < 1e-6);
    assert_eq!(state.present_mode, ut99_editor::graphics::PresentModePreference::AutoVsync);
    assert_eq!(state.max_frame_latency, 2);
}

#[test]
fn test_default_author_propagation_to_new_maps() {
    let mut state = EditorState::default();
    assert_eq!(state.level_author, "Unknown Mapper");

    // Setting default_author to custom mapper name
    state.default_author = "Epic Architect Dave".to_string();

    // Calling new_map() should populate level_author with default_author
    state.new_map();
    assert_eq!(state.level_author, "Epic Architect Dave");

    // If default_author is empty string, fallback to "Unknown Mapper"
    state.default_author = "   ".to_string();
    state.new_map();
    assert_eq!(state.level_author, "Unknown Mapper");
}

#[test]
fn test_default_export_directory_resolution() {
    let mut state = EditorState::default();
    assert!(state.default_export_dir.is_none());

    // By default, default_maps_directory resolves to UnrealTournament/Maps or Maps or current dir
    let fallback = state.default_maps_directory();
    assert!(!fallback.as_os_str().is_empty());

    // With a custom default_export_dir pointing to an existing directory
    let temp_export_dir = std::env::temp_dir().join("test_ut99_export_dir");
    std::fs::create_dir_all(&temp_export_dir).unwrap();
    state.default_export_dir = Some(temp_export_dir.clone());

    let resolved = state.default_maps_directory();
    assert_eq!(resolved, temp_export_dir);

    // Clean up
    let _ = std::fs::remove_dir(&temp_export_dir);
}

#[test]
fn test_i18n_localization_and_fallbacks() {
    // English (default)
    assert_eq!(tr(AppLanguage::English, "settings.title"), "Settings & Preferences");
    assert_eq!(tr(AppLanguage::English, "settings.tab.general"), "General");
    assert_eq!(tr(AppLanguage::English, "settings.tab.appearance"), "Appearance & Themes");
    assert_eq!(tr(AppLanguage::English, "settings.tab.game"), "UT99 Integration");

    // Italian
    assert_eq!(tr(AppLanguage::Italian, "settings.title"), "Impostazioni e Preferenze");
    assert_eq!(tr(AppLanguage::Italian, "settings.tab.general"), "Generale");
    assert_eq!(tr(AppLanguage::Italian, "settings.tab.appearance"), "Aspetto & Temi");
    assert_eq!(tr(AppLanguage::Italian, "settings.tab.game"), "Integrazione UT99");

    // Unknown key fallback to key itself
    assert_eq!(tr(AppLanguage::English, "custom.nonexistent.key"), "custom.nonexistent.key");
    assert_eq!(tr(AppLanguage::Italian, "custom.nonexistent.key"), "custom.nonexistent.key");

    // Language metadata
    for lang in AppLanguage::all() {
        assert!(!lang.code().is_empty());
        assert!(!lang.display_name().is_empty());
        assert!(!lang.native_name().is_empty());
    }
}

#[test]
fn test_all_10_languages_localization_and_overrides() {
    // 1. Exactly 10 supported fundamental languages
    let all_languages = AppLanguage::all();
    assert_eq!(all_languages.len(), 10);

    let expected_codes = ["en", "it", "de", "fr", "pt", "es", "ru", "uk", "pl", "nl"];
    for (lang, &expected_code) in all_languages.iter().zip(expected_codes.iter()) {
        assert_eq!(lang.code(), expected_code);
        assert_eq!(AppLanguage::from_code(expected_code), Some(*lang));
        assert_eq!(AppLanguage::from_code(&expected_code.to_uppercase()), Some(*lang));
    }
    assert_eq!(AppLanguage::from_code("unknown_code"), None);

    // 2. Translations exist and resolve for every language
    let test_keys = [
        "menu.file",
        "settings.title",
        "save.success_title",
        "help.title",
        "welcome.window_title",
        "sidebar.editor_mode",
        "sidebar.builder_brush",
        "sidebar.csg_actions",
        "sidebar.grid_snap",
        "sidebar.active_material",
        "inspector.selection_properties",
        "inspector.split",
        "inspector.outliner",
        "inspector.inspector",
        "outliner.title",
        "builder.cube",
    ];

    for lang in all_languages {
        for key in &test_keys {
            let translated = tr(*lang, key);
            assert!(!translated.is_empty(), "Translation for {} in {:?} is empty", key, lang);
            assert_ne!(translated, *key, "Translation for {} in {:?} was not found", key, lang);
        }
    }

    // Specific verification for Italian UI and sidebars
    assert_eq!(tr(AppLanguage::Italian, "sidebar.editor_mode"), "Modalità Editor");
    assert_eq!(tr(AppLanguage::Italian, "sidebar.builder_brush"), "Brush del Builder");
    assert_eq!(tr(AppLanguage::Italian, "sidebar.csg_actions"), "Azioni CSG");
    assert_eq!(tr(AppLanguage::Italian, "builder.cube"), "Cubo");
    assert_eq!(tr(AppLanguage::Italian, "outliner.title"), "Livelli ed Elementi di Scena");
    assert_eq!(tr(AppLanguage::Italian, "inspector.selection_properties"), "Proprietà Selezione");

    // Specific verification for each newly added language
    assert_eq!(tr(AppLanguage::German, "menu.file"), "Datei");
    assert_eq!(tr(AppLanguage::French, "menu.file"), "Fichier");
    assert_eq!(tr(AppLanguage::Portuguese, "menu.file"), "Arquivo");
    assert_eq!(tr(AppLanguage::Spanish, "menu.file"), "Archivo");
    assert_eq!(tr(AppLanguage::Russian, "menu.file"), "Файл");
    assert_eq!(tr(AppLanguage::Ukrainian, "menu.file"), "Файл");
    assert_eq!(tr(AppLanguage::Polish, "menu.file"), "Plik");
    assert_eq!(tr(AppLanguage::Dutch, "menu.file"), "Bestand");

    // 3. Fallback mechanism: untranslated key falls back to English
    assert_eq!(
        tr(AppLanguage::Dutch, "keymap.modern"),
        "Modern Profile (UE5 / Blender / Maya standard)"
    );
    assert_eq!(
        tr(AppLanguage::Polish, "keymap.modern"),
        "Modern Profile (UE5 / Blender / Maya standard)"
    );

    // 4. Unknown key fallback to key itself
    assert_eq!(tr(AppLanguage::Russian, "some.unknown.test.key"), "some.unknown.test.key");

    // 5. Runtime override registration via JSON
    let custom_json = r#"{
        "menu.custom_item": "Mój własny element menu"
    }"#;
    let count = register_runtime_translations(AppLanguage::Polish, custom_json).expect("valid json");
    assert_eq!(count, 1);
    assert_eq!(tr(AppLanguage::Polish, "menu.custom_item"), "Mój własny element menu");
    // English should fallback to key because not registered for English
    assert_eq!(tr(AppLanguage::English, "menu.custom_item"), "menu.custom_item");

    // 6. Serde serialization & deserialization roundtrip for all 10 languages
    for lang in all_languages {
        let serialized = serde_json::to_string(lang).expect("serialize");
        let deserialized: AppLanguage = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(*lang, deserialized);
    }
}

#[test]
fn test_theme_variants_and_wcag_contrast() {
    let ctx = egui::Context::default();

    for theme in AppTheme::all() {
        // Apply theme to egui context
        apply_theme_variant(&ctx, *theme);
        assert!(!theme.display_name().is_empty());
        assert!(!theme.description().is_empty());
    }

    // Verify WCAG AA contrast for Unreal Precision Slate Dark
    assert!(check_wcag_aa(ThemeColors::TEXT_PRIMARY, ThemeColors::BG_CANVAS, false));
    assert!(check_wcag_aa(ThemeColors::TEXT_PRIMARY, ThemeColors::BG_PANEL, false));
    assert!(check_wcag_aa(ThemeColors::ACCENT_AZURE, ThemeColors::BG_PANEL, true));
    assert!(check_wcag_aa(ThemeColors::BORDER_FOCUS, ThemeColors::BG_PANEL, true));
}

#[test]
fn test_settings_dialog_toggle_and_close_all() {
    let mut state = EditorState::default();
    assert!(!state.show_settings_dialog);

    // Open settings dialog
    state.show_settings_dialog = true;
    state.active_settings_tab = SettingsTab::GameIntegration;
    assert_eq!(state.active_settings_tab, SettingsTab::GameIntegration);

    // Close all floating dialogs must close settings dialog as well
    let closed = state.close_all_floating_dialogs();
    assert!(closed >= 1);
    assert!(!state.show_settings_dialog);
}

#[test]
fn test_settings_dialog_rendering_and_screen_clamping() {
    use ut99_editor::gui::settings_dialog::render_settings_dialog;

    let ctx = egui::Context::default();
    let mut state = EditorState::default();
    state.show_settings_dialog = true;

    // Test across various screen resolutions: small netbook 1024x600, laptop 1366x768, 1080p, 1440p, 4K
    for (sw, sh) in [(1024.0, 600.0), (1366.0, 768.0), (1920.0, 1080.0), (2560.0, 1440.0)] {
        let _ = ctx.run(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(sw, sh))),
            ..Default::default()
        }, |ctx| {
            render_settings_dialog(ctx, &mut state);
        });
        assert!(state.show_settings_dialog);
    }
}

#[test]
fn test_windows_explorer_path_normalization() {
    use std::path::Path;
    use ut99_editor::normalize_path_for_windows_explorer;

    // 1. Existing file
    let (buf, p_str) = normalize_path_for_windows_explorer(Path::new("Cargo.toml"));
    #[cfg(windows)]
    {
        assert!(buf.exists(), "Cargo.toml must exist");
        assert!(buf.is_file(), "Cargo.toml must be detected as a file");
    }
    let _ = &buf;
    assert!(!p_str.starts_with(r"\\?\"), "Must strip \\?\\ Win32 prefix for explorer.exe: {p_str}");
    assert!(!p_str.contains('/'), "Must normalize all slashes to Windows backslashes: {p_str}");

    // 2. Existing directory
    let (dir_buf, dir_str) = normalize_path_for_windows_explorer(Path::new("src"));
    #[cfg(windows)]
    {
        assert!(dir_buf.exists(), "src must exist");
        assert!(dir_buf.is_dir(), "src must be detected as a directory");
    }
    let _ = &dir_buf;
    assert!(!dir_str.starts_with(r"\\?\"), "Must strip \\?\\ Win32 prefix for explorer.exe: {dir_str}");
    assert!(!dir_str.contains('/'), "Must normalize all slashes to Windows backslashes: {dir_str}");

    // 3. Simulated Win32 device prefix \\?\
    let simulated_unc = Path::new(r"\\?\D:\UnrealTournament\Maps\DM-Deck16][.unr");
    let (unc_buf, unc_str) = normalize_path_for_windows_explorer(simulated_unc);
    assert!(!unc_str.starts_with(r"\\?\"), "Simulated prefix must be stripped: {unc_str}");
    assert!(unc_str.starts_with(r"D:\UnrealTournament\Maps\DM-Deck16][.unr"));
    assert_eq!(unc_buf, Path::new(r"D:\UnrealTournament\Maps\DM-Deck16][.unr"));

    // 4. Simulated network share \\?\UNC\
    let simulated_net = Path::new(r"\\?\UNC\Server\Share\Maps\CTF-Face.unr");
    let (net_buf, net_str) = normalize_path_for_windows_explorer(simulated_net);
    assert!(!net_str.starts_with(r"\\?\UNC\"), "Simulated UNC prefix must be stripped: {net_str}");
    assert!(net_str.starts_with(r"\\Server\Share\Maps\CTF-Face.unr"));
    assert_eq!(net_buf, Path::new(r"\\Server\Share\Maps\CTF-Face.unr"));
}

#[test]
fn test_wizard_map_path_preserved_after_generation() {
    let mut state = EditorState::default();
    state.map_wizard_config.map_name = "DM-TestExportFolderPreserve".to_string();

    let (_b, _m, _a) = state.generate_map_from_wizard();

    assert!(state.map_path.is_some(), "Wizard must set and retain map_path in state");
    let path = state.map_path.unwrap();
    let path_str = path.to_string_lossy();
    assert!(path_str.ends_with("DM-TestExportFolderPreserve.unr"), "map_path must point to generated map: {path_str}");
    assert_eq!(path.file_name().unwrap(), "DM-TestExportFolderPreserve.unr");

    let (norm_buf, norm_str) = ut99_editor::normalize_path_for_windows_explorer(&path);
    assert!(!norm_str.starts_with(r"\\?\"), "Normalized path must not contain \\?\\: {norm_str}");
    assert!(!norm_str.contains('/'), "Normalized path must not contain forward slashes: {norm_str}");
    assert!(norm_str.ends_with(r"\DM-TestExportFolderPreserve.unr"), "Normalized path must preserve map filename: {norm_str}");
    #[cfg(windows)]
    assert_eq!(norm_buf.file_name().unwrap(), "DM-TestExportFolderPreserve.unr");
    let _ = &norm_buf;
}

