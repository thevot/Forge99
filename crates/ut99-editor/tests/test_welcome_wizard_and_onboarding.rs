use ut99_editor::compiler::CompilerBridge;
use ut99_editor::editor_state::{EditorState, KeymapProfile};
use ut99_editor::preferences::EditorPreferences;

#[test]
fn test_auto_detect_ut99_installation() {
    CompilerBridge::set_custom_ut_dir(None);
    let found = CompilerBridge::find_ut_dir();
    // On systems where UT99 is installed or present (e.g. in registry or workspace),
    // find_ut_dir must return Some(path) that is validated by is_valid_ut_dir.
    if let Some(ref path) = found {
        assert!(
            CompilerBridge::is_valid_ut_dir(path),
            "Detected UT99 path {:?} failed validation",
            path
        );
        let sys = path.join("System");
        let valid_marker = sys.join("UnrealTournament.exe").exists()
            || sys.join("UCC.exe").exists()
            || sys.join("Engine.u").exists()
            || sys.join("Core.u").exists()
            || sys.join("ut").exists()
            || sys.join("ut-bin").exists();
        assert!(valid_marker, "Detected UT99 path {:?} must contain valid System files", path);
    }
}

#[test]
fn test_custom_ut99_path_sync_and_cross_platform_engine_u_validation() {
    let temp_ut = std::env::temp_dir().join(format!("forge99_custom_ut_{}", std::process::id()));
    let sys_dir = temp_ut.join("System");
    std::fs::create_dir_all(&sys_dir).unwrap();
    // Simulate a Linux/macOS or extracted UT99 folder that has System/Engine.u without a Windows .exe
    std::fs::write(sys_dir.join("Engine.u"), b"dummy_engine_package").unwrap();

    assert!(
        CompilerBridge::is_valid_ut_dir(&temp_ut),
        "Directory with System/Engine.u must be recognized as valid UT99 installation across OSes"
    );

    let synced = CompilerBridge::sync_from_configured_path(&temp_ut.to_string_lossy());
    assert_eq!(synced.as_deref(), Some(temp_ut.as_path()));
    assert_eq!(
        CompilerBridge::find_ut_dir().as_deref(),
        Some(temp_ut.as_path()),
        "find_ut_dir() must prioritize user-configured path from Welcome Wizard / Settings"
    );

    // Clean up and reset override
    CompilerBridge::set_custom_ut_dir(None);
    let _ = std::fs::remove_dir_all(&temp_ut);
}

#[test]
fn test_welcome_wizard_initial_state_and_first_run() {
    let state = EditorState::default();
    assert!(!state.first_run_completed, "New EditorState must start with first_run_completed = false");
    assert!(!state.show_welcome_wizard, "Default state before preference check should have wizard closed");
    assert_eq!(state.welcome_wizard_step, 0, "Welcome wizard should start at step 0");
}

#[test]
fn test_welcome_wizard_completion_and_persistence() {
    let temp_dir = std::env::temp_dir().join(format!("forge99_test_wizard_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let test_prefs_file = temp_dir.join("forge99_prefs.json");

    let mut state = EditorState::default();
    state.first_run_completed = true;
    state.keymap_profile = KeymapProfile::Modern;
    state.save_preferences_to_path(&test_prefs_file);

    let mut new_state = EditorState::default();
    let loaded = new_state.load_preferences_from_path(&test_prefs_file);
    assert!(loaded, "Preferences should load successfully from test path");
    assert!(new_state.first_run_completed, "first_run_completed must persist across save/load");
    assert_eq!(new_state.keymap_profile, KeymapProfile::Modern);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_preferences_forge99_default_path() {
    let def_path = EditorPreferences::default_path();
    let fname = def_path.file_name().unwrap().to_string_lossy();
    assert!(
        fname == "forge99_prefs.json" || fname == "forge99_prefs.json",
        "Expected forge99_prefs.json or fallback forge99_prefs.json, got {}",
        fname
    );
}

#[test]
fn test_welcome_wizard_startup_with_maximized_window() {
    // 1. Verify state and preferences when launching with start_maximized = true
    let state = EditorState::default();
    assert!(state.start_maximized, "start_maximized must default to true");
    assert!(!state.first_run_completed, "first_run_completed must default to false");

    // Simulating startup load_preferences: wizard must open on first run
    let mut temp_state = EditorState::default();
    temp_state.show_welcome_wizard = !temp_state.first_run_completed;
    assert!(temp_state.show_welcome_wizard, "Welcome wizard must be scheduled to display on first run");

    // 2. Headless egui rendering across standard maximized desktop resolutions
    let maximized_resolutions = [
        (1920.0, 1080.0), // Standard 1080p Desktop Maximized
        (2560.0, 1440.0), // 1440p QHD Desktop Maximized
        (3840.0, 2160.0), // 4K UHD Desktop Maximized
        (1366.0, 768.0),  // 768p Laptop Maximized
        (3440.0, 1440.0), // 21:9 Ultra-Wide Maximized
    ];

    for (width, height) in maximized_resolutions {
        let ctx = egui::Context::default();
        let mut sim_state = EditorState::default();
        sim_state.show_welcome_wizard = true;

        for step in 0..=2 {
            sim_state.welcome_wizard_step = step;
            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(width, height),
                )),
                ..Default::default()
            };

            let full_output = ctx.run(raw_input, |ctx| {
                ut99_editor::gui::welcome_wizard::render_welcome_wizard(ctx, &mut sim_state);
            });

            // Ensure shapes/paint primitives were produced and no panic occurred
            assert!(
                !full_output.shapes.is_empty(),
                "Welcome wizard must render UI primitives on {}x{} screen (step {})",
                width,
                height,
                step
            );
        }
    }
}

#[test]
fn test_welcome_wizard_backdrop_z_order_below_window() {
    let ctx = egui::Context::default();
    let mut state = EditorState::default();
    state.show_welcome_wizard = true;

    let screen_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1920.0, 1080.0));
    let raw_input = egui::RawInput {
        screen_rect: Some(screen_rect),
        ..Default::default()
    };

    // Frame 0: egui sizing pass
    let _ = ctx.run(raw_input.clone(), |ctx| {
        ut99_editor::gui::render_ui(ctx, &mut state);
    });

    // Frame 1: visible pass
    let full_output = ctx.run(raw_input, |ctx| {
        ut99_editor::gui::render_ui(ctx, &mut state);
    });

    let backdrop_color = egui::Color32::from_black_alpha(160);
    let mut backdrop_index = None;
    let mut wizard_window_shape_indices = Vec::new();

    let wizard_rect = ctx
        .memory(|m| m.area_rect(egui::Id::new("forge99_welcome_wizard_window")))
        .expect("Welcome wizard window area_rect must exist on first frame");

    for (idx, clipped) in full_output.shapes.iter().enumerate() {
        if let egui::epaint::Shape::Rect(rect_shape) = &clipped.shape {
            if rect_shape.fill == backdrop_color
                && (rect_shape.rect.width() - 1920.0).abs() < 1.0
                && (rect_shape.rect.height() - 1080.0).abs() < 1.0
            {
                backdrop_index = Some(idx);
                continue;
            }
        }
        let bounds = clipped.shape.visual_bounding_rect();
        if bounds.is_finite()
            && bounds.width() > 50.0
            && bounds.width() <= wizard_rect.width() + 80.0
            && bounds.height() > 20.0
            && bounds.height() <= wizard_rect.height() + 80.0
            && wizard_rect.expand(40.0).contains_rect(bounds)
        {
            wizard_window_shape_indices.push(idx);
        }
    }

    let backdrop_idx = backdrop_index.expect("Welcome wizard backdrop shape must be present");
    assert!(
        !wizard_window_shape_indices.is_empty(),
        "Welcome wizard window shapes must be present inside wizard_rect {:?}",
        wizard_rect
    );

    let last_wizard_shape_idx = *wizard_window_shape_indices.last().unwrap();
    assert!(
        backdrop_idx < last_wizard_shape_idx,
        "Backdrop (idx {}) must be painted BEFORE the Welcome Wizard window shapes (last idx {}), never on top of the modal!",
        backdrop_idx,
        last_wizard_shape_idx
    );

    // Every single shape belonging to the Foreground Welcome Wizard window must appear AFTER the Middle backdrop,
    // and the window background rect must have full opacity (alpha == 255) without fade-in dimming.
    fn contains_opaque_window_rect(shape: &egui::epaint::Shape) -> bool {
        match shape {
            egui::epaint::Shape::Rect(rect_shape) => {
                rect_shape.rect.width() >= 800.0
                    && rect_shape.rect.height() >= 600.0
                    && rect_shape.fill.a() == 255
            }
            egui::epaint::Shape::Vec(shapes) => shapes.iter().any(contains_opaque_window_rect),
            _ => false,
        }
    }

    let found_opaque_window_frame_after_backdrop = full_output
        .shapes
        .iter()
        .enumerate()
        .any(|(i, clipped)| i > backdrop_idx && contains_opaque_window_rect(&clipped.shape));

    assert!(
        found_opaque_window_frame_after_backdrop,
        "Foreground Welcome Wizard window frame must be emitted after the backdrop with 100% opacity (alpha=255)"
    );
}

#[test]
fn test_welcome_wizard_first_frame_deterministic_size_and_visibility() {
    let ctx = egui::Context::default();
    let mut state = EditorState::default();
    state.show_welcome_wizard = true;

    let screen_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1920.0, 1080.0));
    let raw_input = egui::RawInput {
        screen_rect: Some(screen_rect),
        ..Default::default()
    };

    let _ = ctx.run(raw_input, |ctx| {
        ut99_editor::gui::render_ui(ctx, &mut state);
    });

    let wizard_rect = ctx
        .memory(|m| m.area_rect(egui::Id::new("forge99_welcome_wizard_window")))
        .expect("Welcome wizard window area_rect must be registered on frame 0");

    assert!(
        wizard_rect.width() >= 800.0 && wizard_rect.height() >= 600.0,
        "Welcome wizard must have full deterministic size on frame 0, got {:?}",
        wizard_rect
    );
    let center_diff = (wizard_rect.center() - screen_rect.center()).length();
    assert!(
        center_diff < 50.0,
        "Welcome wizard must be centered on screen on frame 0 (wizard center {:?}, screen center {:?})",
        wizard_rect.center(),
        screen_rect.center()
    );
}

