use std::path::{Path, PathBuf};
use egui::{self, Color32, Margin, RichText, Stroke};
use crate::editor_state::{EditorState, SettingsTab, UiExperienceMode, KeymapProfile, TargetEngine, GizmoCoordinateSpace};
use crate::gui::theme::{apply_theme_variant, AppTheme, SemanticPalette, ThemeColors};
use crate::i18n::{tr, AppLanguage};
use crate::camera::ViewportShading;

/// Renders the centralized modern Settings & Preferences dialog.
pub fn render_settings_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_settings_dialog {
        return;
    }

    let mut open = state.show_settings_dialog;
    let mut close_requested = false;
    let lang = state.language;

    let screen = ctx.screen_rect();
    let max_w = (screen.width() - 32.0).clamp(640.0, 1150.0);
    let max_h = (screen.height() - 48.0).clamp(480.0, 880.0);
    let default_w = (screen.width() * 0.85).clamp(840.0, 1020.0).min(max_w);
    let default_h = (screen.height() * 0.82).clamp(580.0, 760.0).min(max_h);
    let min_w = 600.0_f32.min(max_w);
    let min_h = 440.0_f32.min(max_h);

    egui::Window::new(format!("⚙ {}", tr(lang, "settings.title")))
        .open(&mut open)
        .default_size([default_w, default_h])
        .max_size([max_w, max_h])
        .min_width(min_w)
        .min_height(min_h)
        .default_pos(screen.center())
        .pivot(egui::Align2::CENTER_CENTER)
        .collapsible(false)
        .resizable(true)
        .constrain(true)
        .show(ctx, |ui| {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close_requested = true;
            }

            // Header Banner
            ui.horizontal(|ui| {
                ui.label(RichText::new("⚙").size(20.0).color(ThemeColors::ACCENT_GOLD));
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr(lang, "settings.title")).strong().size(15.0).color(ThemeColors::TEXT_PRIMARY));
                    ui.label(RichText::new(tr(lang, "settings.subtitle")).size(11.0).color(ThemeColors::TEXT_MUTED));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(format!("✕ {}", tr(lang, "action.close"))).clicked() {
                        close_requested = true;
                    }
                });
            });
            ui.separator();

            // Main Body: Split into Left Navigation Sidebar and Right Content Panel
            let footer_reserve = 52.0_f32;
            let body_h = (ui.available_height() - footer_reserve).max(240.0);

            ui.horizontal(|ui| {
                // Sidebar (Left)
                ui.allocate_ui_with_layout(
                    egui::vec2(185.0, body_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::Frame::NONE
                            .fill(ThemeColors::BG_CANVAS)
                            .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
                            .corner_radius(4.0)
                            .inner_margin(Margin::same(4))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("settings_sidebar_scroll")
                                    .max_height(body_h)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.set_width(176.0);
                                        let tabs = [
                                            (SettingsTab::General, "🌐", "settings.tab.general"),
                                            (SettingsTab::Appearance, "🎨", "settings.tab.appearance"),
                                            (SettingsTab::GameIntegration, "🎮", "settings.tab.game"),
                                            (SettingsTab::ExportPaths, "📁", "settings.tab.paths"),
                                            (SettingsTab::GridSnapping, "📐", "settings.tab.grid"),
                                            (SettingsTab::Viewport3D, "🎥", "settings.tab.viewport"),
                                            (SettingsTab::AutoSave, "🛡", "settings.tab.autosave"),
                                            (SettingsTab::KeymapInput, "⌨", "settings.tab.keymap"),
                                        ];

                                        for (tab, icon, key) in tabs {
                                            let is_active = state.active_settings_tab == tab;
                                            let label = format!("{icon} {}", tr(lang, key));
                                            let response = if is_active {
                                                ui.add(egui::Button::new(
                                                    RichText::new(label).strong().color(ThemeColors::ACCENT_AZURE),
                                                ).fill(ThemeColors::BG_WIDGET_ACTIVE).min_size(egui::vec2(170.0, 26.0)))
                                            } else {
                                                ui.add(egui::Button::new(
                                                    RichText::new(label).color(ThemeColors::TEXT_SECONDARY),
                                                ).fill(Color32::TRANSPARENT).min_size(egui::vec2(170.0, 26.0)))
                                            };

                                            if response.clicked() {
                                                state.active_settings_tab = tab;
                                            }
                                            ui.add_space(2.0);
                                        }
                                    });
                            });
                    },
                );

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                // Content Panel (Right)
                let content_w = ui.available_width().max(300.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(content_w, body_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("settings_content_scroll")
                            .max_height(body_h)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                match state.active_settings_tab {
                                    SettingsTab::General => render_tab_general(ui, state),
                                    SettingsTab::Appearance => render_tab_appearance(ui, state, ctx),
                                    SettingsTab::GameIntegration => render_tab_game(ui, state),
                                    SettingsTab::ExportPaths => render_tab_paths(ui, state),
                                    SettingsTab::GridSnapping => render_tab_grid(ui, state),
                                    SettingsTab::Viewport3D => render_tab_viewport(ui, state),
                                    SettingsTab::AutoSave => render_tab_autosave(ui, state),
                                    SettingsTab::KeymapInput => render_tab_keymap(ui, state),
                                }
                            });
                    },
                );
            });

            ui.add_space(6.0);
            ui.separator();

            // Footer Actions (Sticky at bottom)
            ui.horizontal(|ui| {
                if ui.button(format!("↺ {}", tr(lang, "action.reset_defaults")))
                    .on_hover_text(tr(lang, "action.reset_confirm"))
                    .clicked()
                {
                    state.default_author = "Unknown Mapper".to_string();
                    state.default_export_dir = None;
                    state.language = AppLanguage::English;
                    state.theme = AppTheme::UnrealSlateDark;
                    apply_theme_variant(ctx, state.theme);
                    state.ui_scale = 1.0;
                    state.grid_step = 16.0;
                    state.grid.step = 16.0;
                    state.snap_to_grid = true;
                    state.rotation_snap_deg = 15.0;
                    state.snap_to_rotation = true;
                    state.drag_threshold_px = 6.0;
                    state.camera_speed_preset = 4;
                    state.camera.fov_y = 75.0_f32.to_radians();
                    state.viewport_shading = ViewportShading::Lit;
                    state.auto_save_enabled = true;
                    state.auto_save_interval_secs = 600.0;
                    state.frustum_culling_enabled = true;
                    state.hardware_profile = crate::graphics::HardwareProfile::Auto;
                    state.graphics_backend = crate::graphics::GraphicsBackendPreference::Auto;
                    state.preferred_adapter_name = None;
                    state.viewport_render_scale = 1.0;
                    state.present_mode = crate::graphics::PresentModePreference::AutoVsync;
                    state.max_frame_latency = 2;
                    state.pending_graphics_reinit = true;
                    state.distance_fog_enabled = false;
                    state.keymap_profile = KeymapProfile::Modern;
                    state.gizmo_space = GizmoCoordinateSpace::World;
                    state.ui_experience_mode = UiExperienceMode::Standard;
                    state.toast_info("Settings reset to defaults");
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new(format!("✔ {}", tr(lang, "action.close"))).strong()).clicked() {
                        state.save_preferences();
                        close_requested = true;
                    }

                    if ui.button(RichText::new(format!("💾 {}", tr(lang, "action.save_preferences"))).color(ThemeColors::ACCENT_AZURE)).clicked() {
                        if state.save_preferences() {
                            state.toast_success(tr(lang, "action.preferences_saved"));
                        }
                    }
                });
            });
        });

    if close_requested {
        open = false;
    }
    state.show_settings_dialog = open;
}

// ----------------------------------------------------------------------------
// 1. GENERAL TAB
// ----------------------------------------------------------------------------
fn render_tab_general(ui: &mut egui::Ui, state: &mut EditorState) {
    let lang = state.language;
    ui.label(RichText::new(format!("🌐 {}", tr(lang, "settings.tab.general"))).heading().color(ThemeColors::TEXT_PRIMARY));
    ui.add_space(4.0);

    // Language Selector
    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(tr(lang, "general.language")).strong().size(13.0));
            ui.label(RichText::new(tr(lang, "general.language_desc")).size(11.0).color(ThemeColors::TEXT_MUTED));
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                for l in AppLanguage::all() {
                    let is_active = state.language == *l;
                    if ui.selectable_label(is_active, l.display_name()).clicked() {
                        state.language = *l;
                    }
                }
            });
        });

    ui.add_space(8.0);

    // Default Author
    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(tr(lang, "general.default_author")).strong().size(13.0));
            ui.label(RichText::new(tr(lang, "general.default_author_desc")).size(11.0).color(ThemeColors::TEXT_MUTED));
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label("Name / Handle:");
                let text_w = (ui.available_width() - 80.0).max(100.0);
                ui.add(egui::TextEdit::singleline(&mut state.default_author).desired_width(text_w));
                if ui.button("Default").clicked() {
                    state.default_author = "Unknown Mapper".to_string();
                }
            });
        });

    ui.add_space(8.0);

    // UI Scale & Magnification
    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(tr(lang, "general.ui_scale")).strong().size(13.0));
            ui.label(RichText::new(tr(lang, "general.ui_scale_desc")).size(11.0).color(ThemeColors::TEXT_MUTED));
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                let mut pct = (state.ui_scale * 100.0).round() as i32;
                if ui.add(egui::Slider::new(&mut pct, 75..=250).suffix("%")).changed() {
                    state.ui_scale = (pct as f32) / 100.0;
                }
                if ui.button("100%").clicked() {
                    state.ui_scale = 1.0;
                }
                if ui.button("125%").clicked() {
                    state.ui_scale = 1.25;
                }
                if ui.button("150%").clicked() {
                    state.ui_scale = 1.5;
                }
            });
        });

    ui.add_space(8.0);

    // UI Experience Level
    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(tr(lang, "general.experience_mode")).strong().size(13.0));
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                let is_standard = state.ui_experience_mode == UiExperienceMode::Standard;
                if ui.selectable_label(is_standard, tr(lang, "general.experience_standard")).clicked() {
                    state.ui_experience_mode = UiExperienceMode::Standard;
                }
                let is_architect = state.ui_experience_mode == UiExperienceMode::Architect;
                if ui.selectable_label(is_architect, tr(lang, "general.experience_architect")).clicked() {
                    state.ui_experience_mode = UiExperienceMode::Architect;
                }
            });
        });

    ui.add_space(8.0);

    // Window Startup & Fullscreen
    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new("📺 Window & Display Mode").strong().size(13.0));
            ui.add_space(6.0);
            if ui.checkbox(&mut state.start_maximized, "Start Maximized (A tutto schermo all'avvio)").changed() {
                state.save_preferences();
            }
            ui.label(RichText::new("When enabled, Forge99 launches maximized to fill your desktop. Press F11 or Alt+Enter at any time to toggle borderless fullscreen.").size(11.0).color(ThemeColors::TEXT_MUTED));

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            let mut show_onboarding_on_startup = !state.first_run_completed;
            if ui
                .checkbox(
                    &mut show_onboarding_on_startup,
                    "Show Welcome & Onboarding Wizard on Startup (Mostra Onboarding all'avvio)",
                )
                .changed()
            {
                state.first_run_completed = !show_onboarding_on_startup;
                let override_id = egui::Id::new("welcome_wizard_dont_show_override");
                ui.ctx()
                    .data_mut(|d| d.insert_temp(override_id, state.first_run_completed));
                state.save_preferences();
            }
            ui.add_space(4.0);
            if ui.button("🌟 Open Welcome & Onboarding Wizard...").clicked() {
                state.welcome_wizard_step = 0;
                state.show_welcome_wizard = true;
            }
        });
}

// ----------------------------------------------------------------------------
// 2. APPEARANCE & THEMES TAB
// ----------------------------------------------------------------------------
fn render_tab_appearance(ui: &mut egui::Ui, state: &mut EditorState, ctx: &egui::Context) {
    let lang = state.language;
    let palette = SemanticPalette::current(ctx);
    ui.label(RichText::new(tr(lang, "settings.tab.appearance")).heading().color(palette.text_primary));
    ui.add_space(4.0);

    egui::Frame::group(ui.style())
        .fill(palette.bg_panel)
        .stroke(Stroke::new(1.0_f32, palette.border_subtle))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(tr(lang, "appearance.theme")).strong().size(13.0).color(palette.text_primary));
            ui.label(RichText::new(tr(lang, "appearance.theme_desc")).size(11.0).color(palette.text_muted));
            ui.add_space(8.0);

            for theme in AppTheme::all() {
                let is_selected = state.theme == *theme;
                let bg_color = if is_selected { palette.bg_widget_active } else { palette.bg_widget };

                egui::Frame::NONE
                    .fill(bg_color)
                    .stroke(Stroke::new(
                        if is_selected { 1.5_f32 } else { 1.0_f32 },
                        if is_selected { palette.accent_azure } else { palette.border_subtle },
                    ))
                    .corner_radius(4.0)
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let radio_resp = ui.radio(
                                is_selected,
                                RichText::new(theme.display_name()).strong().color(palette.text_primary),
                            );
                            if radio_resp.clicked()
                                || ui
                                    .interact(ui.min_rect(), ui.id().with(theme.display_name()), egui::Sense::click())
                                    .clicked()
                            {
                                state.theme = *theme;
                                apply_theme_variant(ctx, state.theme);
                            }
                        });
                        ui.add_space(2.0);
                        ui.label(RichText::new(theme.description()).size(11.0).color(palette.text_secondary));
                    });
                ui.add_space(6.0);
            }
        });

    ui.add_space(8.0);

    egui::Frame::group(ui.style())
        .fill(palette.bg_panel)
        .stroke(Stroke::new(1.0_f32, palette.border_subtle))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new("Texture Browser & Display Options").strong().size(13.0).color(palette.text_primary));
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(tr(lang, "appearance.thumbnail_size"));
                ui.add(egui::Slider::new(&mut state.texture_thumbnail_size, 48.0..=256.0).suffix(" px"));
            });

            ui.add_space(4.0);
            ui.checkbox(&mut state.show_minimap, tr(lang, "appearance.minimap_default"));
            ui.checkbox(&mut state.show_actor_radii, tr(lang, "appearance.radii_default"));
        });
}

// ----------------------------------------------------------------------------
// 3. UT99 GAME INTEGRATION TAB
// ----------------------------------------------------------------------------
fn render_tab_game(ui: &mut egui::Ui, state: &mut EditorState) {
    let lang = state.language;
    ui.label(RichText::new(format!("🎮 {}", tr(lang, "settings.tab.game"))).heading().color(ThemeColors::TEXT_PRIMARY));
    ui.add_space(4.0);

    // Executable Path
    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(tr(lang, "game.exe_path")).strong().size(13.0));
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                let text_w = (ui.available_width() - 170.0).max(100.0);
                if ui.add(egui::TextEdit::singleline(&mut state.ut99_executable_path).desired_width(text_w)).lost_focus() {
                    state.sync_audio_installation();
                }

                if ui.button(format!("📁 {}", tr(lang, "action.browse"))).clicked() {
                    let mut dialog = rfd::FileDialog::new()
                        .add_filter("Unreal Tournament Executable (*.exe)", &["exe"]);
                    let current_path = Path::new(&state.ut99_executable_path);
                    if let Some(parent) = current_path.parent() {
                        if parent.exists() {
                            dialog = dialog.set_directory(parent);
                        }
                    }
                    if let Some(path) = dialog.pick_file() {
                        if let Some(path_str) = path.to_str() {
                            state.ut99_executable_path = path_str.to_string();
                            state.sync_audio_installation();
                        }
                    }
                }

                if ui.button(format!("🔍 {}", tr(lang, "action.auto_detect"))).clicked() {
                    crate::compiler::CompilerBridge::set_custom_ut_dir(None);
                    if let Some(ut_dir) = crate::compiler::CompilerBridge::find_ut_dir() {
                        let cand = ut_dir.join("System").join("UnrealTournament.exe");
                        state.ut99_executable_path = if cand.exists() { cand } else { ut_dir }.to_string_lossy().to_string();
                        state.sync_audio_installation();
                        state.toast_success("Auto-detected UT99 installation successfully");
                    } else {
                        state.toast_warning("Could not auto-detect UT99 path in standard locations");
                    }
                }
            });

            ui.add_space(4.0);
            let exe_exists = Path::new(&state.ut99_executable_path).exists();
            if exe_exists {
                ui.colored_label(Color32::from_rgb(100, 220, 120), tr(lang, "game.exe_found"));
            } else {
                ui.colored_label(Color32::from_rgb(255, 140, 80), tr(lang, "game.exe_missing"));
            }
        });

    ui.add_space(8.0);

    // Launch Flags & Presets
    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(tr(lang, "game.launch_args")).strong().size(13.0));
            ui.add_space(4.0);
            let args_w = (ui.available_width() - 8.0).max(100.0);
            ui.add(egui::TextEdit::singleline(&mut state.ut99_launch_args).desired_width(args_w));

            ui.add_space(6.0);
            ui.label(RichText::new(tr(lang, "game.presets")).color(ThemeColors::TEXT_MUTED));
            ui.horizontal_wrapped(|ui| {
                if ui.button("DeathMatch").clicked() {
                    state.ut99_launch_args = "?game=Botpack.DeathMatchPlus -log".to_string();
                }
                if ui.button("Capture The Flag").clicked() {
                    state.ut99_launch_args = "?game=Botpack.CTFGame -log".to_string();
                }
                if ui.button("Domination").clicked() {
                    state.ut99_launch_args = "?game=Botpack.Domination -log".to_string();
                }
                if ui.button("InstaGib DM").clicked() {
                    state.ut99_launch_args = "?game=Botpack.DeathMatchPlus?mutator=Botpack.InstaGibDM -log".to_string();
                }
                if ui.button("Windowed Mode").clicked() {
                    if !state.ut99_launch_args.contains("-windowed") {
                        state.ut99_launch_args.push_str(" -windowed");
                    }
                }
            });
        });

    ui.add_space(8.0);

    // Compatibility Engine Profile & Test Launch
    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(tr(lang, "game.target_engine")).strong().size(13.0));
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let is_ut99 = state.target_engine == TargetEngine::Ut99Standard;
                if ui.selectable_label(is_ut99, "UT99 Standard (v436 / v451 / v469)").clicked() {
                    state.target_engine = TargetEngine::Ut99Standard;
                }
                let is_u227 = state.target_engine == TargetEngine::Unreal227;
                if ui.selectable_label(is_u227, "Unreal 227 Extended (Weather / StaticMesh)").clicked() {
                    state.target_engine = TargetEngine::Unreal227;
                }
            });

            ui.add_space(8.0);
            if ui.button(format!("▶ {}", tr(lang, "action.test_launch"))).clicked() {
                let _ = state.play_map_in_ut99();
            }
        });
}

// ----------------------------------------------------------------------------
// 4. EXPORT & PATHS TAB
// ----------------------------------------------------------------------------
fn render_tab_paths(ui: &mut egui::Ui, state: &mut EditorState) {
    let lang = state.language;
    ui.label(RichText::new(format!("📁 {}", tr(lang, "settings.tab.paths"))).heading().color(ThemeColors::TEXT_PRIMARY));
    ui.add_space(4.0);

    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(tr(lang, "paths.export_dir")).strong().size(13.0));
            ui.label(RichText::new(tr(lang, "paths.export_dir_desc")).size(11.0).color(ThemeColors::TEXT_MUTED));
            ui.add_space(8.0);

            let current_display = state.default_export_dir
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| state.default_maps_directory().to_string_lossy().to_string());

            let mut path_edit = current_display.clone();
            ui.horizontal(|ui| {
                let text_w = (ui.available_width() - 190.0).max(100.0);
                if ui.add(egui::TextEdit::singleline(&mut path_edit).desired_width(text_w)).changed() {
                    if path_edit.trim().is_empty() {
                        state.default_export_dir = None;
                    } else {
                        state.default_export_dir = Some(PathBuf::from(path_edit.trim()));
                    }
                }

                if ui.button(format!("📁 {}", tr(lang, "action.browse"))).clicked() {
                    let mut dialog = rfd::FileDialog::new();
                    if let Some(ref dir) = state.default_export_dir {
                        if dir.exists() {
                            dialog = dialog.set_directory(dir);
                        }
                    }
                    if let Some(folder) = dialog.pick_folder() {
                        state.default_export_dir = Some(folder);
                    }
                }

                if ui.button(tr(lang, "paths.use_default_maps")).clicked() {
                    state.default_export_dir = None;
                }
            });

            ui.add_space(4.0);
            let effective_path = state.default_maps_directory();
            let dir_exists = effective_path.exists();
            ui.horizontal_wrapped(|ui| {
                if dir_exists {
                    ui.colored_label(Color32::from_rgb(100, 220, 120), format!("✔ Active: {}", effective_path.display()));
                } else {
                    ui.colored_label(Color32::from_rgb(255, 140, 80), format!("ℹ Path will be created on save: {}", effective_path.display()));
                }

                if ui.button(format!("📂 {}", tr(lang, "action.reveal_explorer"))).clicked() {
                    state.reveal_in_file_explorer(&effective_path);
                }
            });
        });

    ui.add_space(8.0);

    // Current Map File Info
    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new("Active Map Session Info").strong().size(13.0));
            ui.add_space(4.0);

            if let Some(ref p) = state.map_path {
                ui.label(format!("File: {}", p.display()));
                ui.label(format!("Dirty unsaved changes: {}", if state.map_is_dirty { "YES (Unsaved)" } else { "NO (Saved)" }));
            } else {
                ui.label(RichText::new("No map file saved to disk yet (Untitled Arena).").italics().color(ThemeColors::TEXT_MUTED));
            }
        });
}

// ----------------------------------------------------------------------------
// 5. GRID & SNAPPING TAB
// ----------------------------------------------------------------------------
fn render_tab_grid(ui: &mut egui::Ui, state: &mut EditorState) {
    let lang = state.language;
    ui.label(RichText::new(format!("📐 {}", tr(lang, "settings.tab.grid"))).heading().color(ThemeColors::TEXT_PRIMARY));
    ui.add_space(4.0);

    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.checkbox(&mut state.snap_to_grid, tr(lang, "grid.snap_spatial"));
            ui.add_space(4.0);

            ui.label(tr(lang, "grid.step"));
            ui.horizontal_wrapped(|ui| {
                for step in [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0, 1024.0] {
                    let is_active = (state.grid_step - step).abs() < 1e-3;
                    if ui.selectable_label(is_active, format!("{:.0}", step)).clicked() {
                        state.set_grid_step(step);
                    }
                }
            });
        });

    ui.add_space(8.0);

    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.checkbox(&mut state.snap_to_rotation, tr(lang, "grid.snap_rotation"));
            ui.add_space(4.0);

            ui.label(tr(lang, "grid.rot_step"));
            ui.horizontal_wrapped(|ui| {
                for deg in [1.0, 5.0, 11.25, 15.0, 22.5, 45.0, 90.0] {
                    let is_active = (state.rotation_snap_deg - deg).abs() < 1e-2;
                    if ui.selectable_label(is_active, format!("{}°", deg)).clicked() {
                        state.rotation_snap_deg = deg;
                    }
                }
            });
        });

    ui.add_space(8.0);

    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(tr(lang, "grid.drag_threshold"));
            ui.add(egui::Slider::new(&mut state.drag_threshold_px, 2.0..=32.0).suffix(" px"));

            ui.add_space(6.0);
            ui.label(tr(lang, "grid.gizmo_space"));
            ui.horizontal(|ui| {
                if ui.selectable_label(state.gizmo_space == GizmoCoordinateSpace::World, "World Space").clicked() {
                    state.gizmo_space = GizmoCoordinateSpace::World;
                }
                if ui.selectable_label(state.gizmo_space == GizmoCoordinateSpace::Local, "Local Space").clicked() {
                    state.gizmo_space = GizmoCoordinateSpace::Local;
                }
            });
        });
}

// ----------------------------------------------------------------------------
// 6. VIEWPORT & 3D TAB
// ----------------------------------------------------------------------------
fn render_tab_viewport(ui: &mut egui::Ui, state: &mut EditorState) {
    let lang = state.language;
    ui.label(RichText::new(format!("🎥 {}", tr(lang, "settings.tab.viewport"))).heading().color(ThemeColors::TEXT_PRIMARY));
    ui.add_space(4.0);

    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(tr(lang, "viewport.speed"));
            ui.horizontal(|ui| {
                for spd in 1..=8 {
                    let is_active = state.camera_speed_preset == spd;
                    if ui.selectable_label(is_active, format!("{spd}")).clicked() {
                        state.set_camera_speed_preset(spd);
                    }
                }
            });

            ui.add_space(8.0);
            ui.label(tr(lang, "viewport.fov"));
            let mut fov = state.get_camera_fov_degrees();
            if ui.add(egui::Slider::new(&mut fov, 60.0..=120.0).suffix("°")).changed() {
                state.set_camera_fov_degrees(fov);
            }
        });

    ui.add_space(8.0);

    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(tr(lang, "viewport.shading"));
            ui.horizontal_wrapped(|ui| {
                let modes = [
                    (ViewportShading::Lit, "Lit"),
                    (ViewportShading::Unlit, "Unlit"),
                    (ViewportShading::Wireframe, "Wireframe"),
                    (ViewportShading::ZonePortal, "Zone/Portal"),
                    (ViewportShading::DepthComplexity, "Depth Complexity"),
                    (ViewportShading::TextureUsage, "Texture Usage"),
                    (ViewportShading::BspCuts, "BSP Cuts"),
                ];
                for (mode, label) in modes {
                    if ui.selectable_label(state.viewport_shading == mode, label).clicked() {
                        state.viewport_shading = mode;
                    }
                }
            });

            ui.add_space(8.0);
            ui.checkbox(&mut state.frustum_culling_enabled, tr(lang, "viewport.culling"));
            ui.checkbox(&mut state.distance_fog_enabled, tr(lang, "viewport.distance_fog"));
        });

    ui.add_space(8.0);

    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            use crate::graphics::{
                GraphicsBackendPreference, HardwareProfile, PresentModePreference,
            };
            ui.label(RichText::new("Hardware Performance & Display").strong().size(13.0));
            ui.add_space(6.0);

            ui.label("Hardware Profile");
            ui.horizontal_wrapped(|ui| {
                for &profile in &HardwareProfile::ALL {
                    let label = if profile == HardwareProfile::Auto {
                        format!("Auto ({})", state.graphics_diagnostics.resolved_tier.label())
                    } else {
                        profile.label().to_string()
                    };
                    if ui.selectable_label(state.hardware_profile == profile, label).clicked() {
                        state.hardware_profile = profile;
                    }
                }
            });
            ui.small(format!(
                "Active budgets: tier={} | diffuse_layers={} | lightmap={}x{} | retained={} MiB | tex_lru={} MiB",
                state.graphics_diagnostics.resolved_tier.label(),
                state.graphics_diagnostics.diffuse_texture_layer_cap,
                state.graphics_diagnostics.lightmap_atlas_dim,
                state.graphics_diagnostics.lightmap_atlas_dim,
                state.graphics_diagnostics.retained_vertex_budget_bytes / (1024 * 1024),
                state.graphics_diagnostics.texture_manager_budget_bytes / (1024 * 1024),
            ));

            ui.add_space(8.0);
            ui.label("3D Viewport Render Scale (UI panels & text stay 100% native)");
            ui.horizontal(|ui| {
                let mut scale_pct = (state.viewport_render_scale * 100.0).round() as i32;
                if ui.add(egui::Slider::new(&mut scale_pct, 50..=100).suffix("%")).changed() {
                    state.viewport_render_scale = (scale_pct as f32 / 100.0).clamp(0.5, 1.0);
                }
                for (pct, val) in [("50%", 0.50f32), ("75%", 0.75), ("85%", 0.85), ("100% Native", 1.0)] {
                    let selected = (state.viewport_render_scale - val).abs() < 0.015;
                    if ui.selectable_label(selected, pct).clicked() {
                        state.viewport_render_scale = val;
                    }
                }
            });

            ui.add_space(8.0);
            ui.label("Graphics Backend Preference");
            ui.horizontal_wrapped(|ui| {
                for &backend in &GraphicsBackendPreference::ALL {
                    if ui.selectable_label(state.graphics_backend == backend, backend.label()).clicked() {
                        state.graphics_backend = backend;
                    }
                }
            });

            ui.add_space(6.0);
            ui.label("Graphics Adapter");
            let selected_adapter_text = state
                .preferred_adapter_name
                .as_deref()
                .unwrap_or("Auto (Best Hardware)");
            egui::ComboBox::from_id_salt("settings_graphics_adapter")
                .selected_text(selected_adapter_text)
                .width(340.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(state.preferred_adapter_name.is_none(), "Auto (Best Hardware)")
                        .clicked()
                    {
                        state.preferred_adapter_name = None;
                    }
                    for adapter in &state.graphics_diagnostics.available_adapters {
                        let is_selected = state
                            .preferred_adapter_name
                            .as_deref()
                            .is_some_and(|n| n.eq_ignore_ascii_case(&adapter.name));
                        let entry = format!(
                            "{} ({:?}, {:?})",
                            adapter.name, adapter.backend, adapter.device_type
                        );
                        if ui.selectable_label(is_selected, entry).clicked() {
                            state.preferred_adapter_name = Some(adapter.name.clone());
                        }
                    }
                });

            ui.add_space(8.0);
            ui.label("Swapchain Presentation Mode");
            ui.horizontal_wrapped(|ui| {
                for &pm in &PresentModePreference::ALL {
                    if ui.selectable_label(state.present_mode == pm, pm.label()).clicked() {
                        state.present_mode = pm;
                    }
                }
            });
            ui.small(format!(
                "Active: {:?} | Monitor refresh: {:.1} Hz",
                state.graphics_diagnostics.active_present_mode,
                if state.graphics_diagnostics.monitor_refresh_hz > 0.0 {
                    state.graphics_diagnostics.monitor_refresh_hz
                } else {
                    60.0
                },
            ));

            ui.add_space(6.0);
            ui.label("Frame Queue Latency");
            ui.horizontal(|ui| {
                if ui.selectable_label(state.max_frame_latency == 1, "1 — Low Input Latency").clicked() {
                    state.max_frame_latency = 1;
                }
                if ui.selectable_label(state.max_frame_latency >= 2, "2 — Smooth Default").clicked() {
                    state.max_frame_latency = 2;
                }
            });

            ui.add_space(8.0);
            if ui.button("Apply Adapter / Backend / Profile Now").clicked() {
                state.pending_graphics_reinit = true;
                state.save_preferences();
                state.toast_info("Reinitializing graphics adapter and profile...");
            }
        });
}

// ----------------------------------------------------------------------------
// 7. AUTO-SAVE TAB
// ----------------------------------------------------------------------------
fn render_tab_autosave(ui: &mut egui::Ui, state: &mut EditorState) {
    let lang = state.language;
    ui.label(RichText::new(format!("🛡 {}", tr(lang, "settings.tab.autosave"))).heading().color(ThemeColors::TEXT_PRIMARY));
    ui.add_space(4.0);

    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.checkbox(&mut state.auto_save_enabled, tr(lang, "autosave.enable"));
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(tr(lang, "autosave.interval"));
                let mut mins = (state.auto_save_interval_secs / 60.0).round() as i32;
                if ui.add(egui::Slider::new(&mut mins, 1..=60).suffix(" min")).changed() {
                    state.auto_save_interval_secs = (mins as f32) * 60.0;
                }
            });

            ui.add_space(6.0);
            let elapsed_mins = state.auto_save_timer / 60.0;
            let next_mins = (state.auto_save_interval_secs - state.auto_save_timer).max(0.0) / 60.0;
            ui.label(RichText::new(format!("Time elapsed: {:.1} min | Next scheduled: in {:.1} min", elapsed_mins, next_mins)).size(11.0).color(ThemeColors::TEXT_MUTED));

            ui.add_space(8.0);
            if ui.button("💾 Trigger Auto-Save Snapshot Now").clicked() {
                state.trigger_auto_save();
            }
        });
}

// ----------------------------------------------------------------------------
// 8. KEYMAP & INPUT TAB
// ----------------------------------------------------------------------------
fn render_tab_keymap(ui: &mut egui::Ui, state: &mut EditorState) {
    let lang = state.language;
    ui.label(RichText::new(format!("⌨ {}", tr(lang, "settings.tab.keymap"))).heading().color(ThemeColors::TEXT_PRIMARY));
    ui.add_space(4.0);

    egui::Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(tr(lang, "keymap.profile")).strong().size(13.0));
            ui.add_space(6.0);

            ui.vertical(|ui| {
                let is_modern = state.keymap_profile == KeymapProfile::Modern;
                if ui.selectable_label(is_modern, tr(lang, "keymap.modern")).clicked() {
                    state.keymap_profile = KeymapProfile::Modern;
                }
                let is_classic = state.keymap_profile == KeymapProfile::ClassicUnrealEd2;
                if ui.selectable_label(is_classic, tr(lang, "keymap.classic")).clicked() {
                    state.keymap_profile = KeymapProfile::ClassicUnrealEd2;
                }
            });

            ui.add_space(10.0);
            if ui.button(format!("📖 {}", tr(lang, "keymap.help"))).clicked() {
                state.show_shortcut_help_dialog = true;
            }
        });
}
