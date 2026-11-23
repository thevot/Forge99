use egui::{Color32, Frame, Margin, RichText, Stroke, Ui};
use crate::compiler::CompilerBridge;
use crate::editor_state::{EditorState, KeymapProfile};
use crate::gui::theme::ThemeColors;
use crate::i18n::tr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterExperience {
    StarterArena,
    MysteryMap,
    BlankRoom,
}

pub fn render_welcome_wizard(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_welcome_wizard {
        return;
    }

    let screen_rect = ctx.input(|i| i.screen_rect());
    if screen_rect.width() <= 10.0 || screen_rect.height() <= 10.0 {
        return;
    }

    let max_w = (screen_rect.width() - 40.0).max(360.0);
    let max_h = (screen_rect.height() - 40.0).max(300.0);
    let window_width = 860.0_f32.min(max_w).max(560.0_f32.min(max_w));
    let window_height = 680.0_f32.min(max_h).max(480.0_f32.min(max_h));

    let window_id = egui::Id::new("forge99_welcome_wizard_window");

    let override_id = egui::Id::new("welcome_wizard_dont_show_override");
    let dont_show_pref = ctx
        .data_mut(|d| d.get_temp::<bool>(override_id))
        .unwrap_or(true);

    let mut open = true;
    let lang = state.language;
    egui::Window::new(format!("🌟 {}", tr(lang, "welcome.window_title")))
        .id(window_id)
        .order(egui::Order::Foreground)
        .fade_in(false)
        .open(&mut open)
        .default_size([window_width, window_height])
        .fixed_size([window_width, window_height])
        .max_size([max_w, max_h])
        .min_width(500.0_f32.min(max_w))
        .min_height(380.0_f32.min(max_h))
        .collapsible(false)
        .resizable(false)
        .constrain(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if ui.is_sizing_pass() {
                ui.ctx().request_repaint();
            } else {
                // Semi-transparent backdrop overlay in Order::Middle so it dims the background workspace
                // while the Welcome Wizard window itself renders cleanly on top in Order::Foreground.
                crate::gui::modal::render_modal_backdrop(
                    ui.ctx(),
                    "welcome_wizard_backdrop",
                    Color32::from_black_alpha(160),
                );
            }
            ui.set_min_size(egui::vec2(
                (window_width - 28.0).max(320.0),
                (window_height - 64.0).max(240.0),
            ));
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    render_welcome_content(ui, state);
                });
        });

    if !open {
        state.show_welcome_wizard = false;
        state.first_run_completed = dont_show_pref;
        state.save_preferences();
    }
}

fn render_welcome_content(ui: &mut Ui, state: &mut EditorState) {
    // 1. TOP HEADER & PROGRESS STEPS
    ui.vertical_centered(|ui| {
        ui.add_space(6.0);
        ui.label(
            RichText::new("FORGE 99")
                .size(24.0)
                .strong()
                .color(ThemeColors::ACCENT_GOLD),
        );
        ui.label(
            RichText::new(tr(state.language, "welcome.tagline"))
                .size(13.0)
                .color(ThemeColors::TEXT_MUTED),
        );
        ui.label(
            RichText::new("Created by Nicola \"thevot\" Selenu")
                .size(11.0)
                .color(ThemeColors::TEXT_SECONDARY),
        );
        ui.add_space(10.0);

        let lang = state.language;
        // Step Indicator Pills
        ui.horizontal(|ui| {
            let steps = [
                tr(lang, "welcome.step1"),
                tr(lang, "welcome.step2"),
                tr(lang, "welcome.step3"),
            ];
            for (idx, label) in steps.iter().enumerate() {
                let is_current = state.welcome_wizard_step == idx;
                let is_past = state.welcome_wizard_step > idx;

                let (bg_color, text_color) = if is_current {
                    (ThemeColors::ACCENT_AZURE, Color32::WHITE)
                } else if is_past {
                    (Color32::from_rgb(35, 75, 45), Color32::from_rgb(140, 230, 160))
                } else {
                    (ThemeColors::BG_WIDGET, ThemeColors::TEXT_MUTED)
                };

                let text = if is_past {
                    format!("✔ {label}")
                } else {
                    label.to_string()
                };

                Frame::new()
                    .fill(bg_color)
                    .corner_radius(4.0)
                    .inner_margin(Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        if ui.selectable_label(is_current, RichText::new(text).size(11.5).color(text_color).strong()).clicked() {
                            state.welcome_wizard_step = idx;
                        }
                    });

                if idx < steps.len() - 1 {
                    ui.label(RichText::new("➔").color(ThemeColors::TEXT_MUTED));
                }
            }
        });
    });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(10.0);

    // 2. ACTIVE STEP CONTENT
    match state.welcome_wizard_step {
        0 => render_step_game_detection(ui, state),
        1 => render_step_workflow_profile(ui, state),
        _ => render_step_starter_launch(ui, state),
    }
}

// -----------------------------------------------------------------------------
// STEP 1: GAME & ENGINE DETECTION
// -----------------------------------------------------------------------------
fn render_step_game_detection(ui: &mut Ui, state: &mut EditorState) {
    ui.label(
        RichText::new("Step 1: Locate Unreal Tournament 99 Installation")
            .size(16.0)
            .strong()
            .color(Color32::WHITE),
    );
    ui.label(
        RichText::new(
            "Forge99 needs access to your game packages (.utx textures, .unr maps, .u classes) \
             and executable to enable instant 1-click playtesting (F8) and asset browsing.",
        )
        .size(12.5)
        .color(ThemeColors::TEXT_MUTED),
    );
    ui.add_space(12.0);

    let detected_opt = CompilerBridge::find_ut_dir();

    Frame::group(ui.style())
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            if let Some(ref ut_dir) = detected_opt {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("✔").size(22.0).color(Color32::from_rgb(100, 220, 120)));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Unreal Tournament 99 Installation Detected!")
                                .strong()
                                .size(14.0)
                                .color(Color32::from_rgb(120, 230, 140)),
                        );
                        ui.label(RichText::new(ut_dir.to_string_lossy().to_string()).size(12.0).monospace().color(Color32::WHITE));
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                let exe_candidate = ut_dir.join("System").join("UnrealTournament.exe");
                let maps_dir = ut_dir.join("Maps");
                let textures_dir = ut_dir.join("Textures");

                let map_count = if maps_dir.exists() {
                    std::fs::read_dir(&maps_dir).map(|rd| rd.filter_map(Result::ok).count()).unwrap_or(0)
                } else { 0 };

                let tex_count = if textures_dir.exists() {
                    std::fs::read_dir(&textures_dir).map(|rd| rd.filter_map(Result::ok).count()).unwrap_or(0)
                } else { 0 };

                ui.horizontal(|ui| {
                    ui.label(RichText::new("• Executable:").size(12.0).color(ThemeColors::TEXT_MUTED));
                    if exe_candidate.exists() {
                        ui.label(RichText::new("Found (System/UnrealTournament.exe)").size(12.0).color(Color32::from_rgb(100, 220, 120)));
                    } else {
                        ui.label(RichText::new("Missing executable in System/").size(12.0).color(ThemeColors::WARNING));
                    }

                    ui.add_space(16.0);
                    ui.label(RichText::new(format!("• Official Maps: {map_count}")).size(12.0).color(ThemeColors::TEXT_MUTED));
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("• Textures: {tex_count}")).size(12.0).color(ThemeColors::TEXT_MUTED));
                });

                // Auto-apply if not set
                if state.ut99_executable_path.is_empty() || state.ut99_executable_path == "UnrealTournament/System/UnrealTournament.exe" {
                    if exe_candidate.exists() {
                        state.ut99_executable_path = exe_candidate.to_string_lossy().to_string();
                    }
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠").size(22.0).color(ThemeColors::WARNING));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Automatic game detection found no standard UT99 folder")
                                .strong()
                                .size(14.0)
                                .color(ThemeColors::WARNING),
                        );
                        ui.label(
                            RichText::new("Checked Windows Registry, Steam libraries, GOG Galaxy and root directories.")
                                .size(12.0)
                                .color(ThemeColors::TEXT_MUTED),
                        );
                    });
                });
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Path:").strong().size(12.0));
                if ui.add(
                    egui::TextEdit::singleline(&mut state.ut99_executable_path)
                        .desired_width(400.0)
                        .hint_text("Path to UnrealTournament folder or executable"),
                ).lost_focus() {
                    state.sync_audio_installation();
                }

                if ui.button("📁 Select Folder...").on_hover_text("Select your UnrealTournament root folder (Windows, Linux, macOS)").clicked() {
                    if let Some(folder) = rfd::FileDialog::new()
                        .set_title("Select UnrealTournament Root Folder")
                        .pick_folder()
                    {
                        let exe = folder.join("System").join("UnrealTournament.exe");
                        if exe.exists() {
                            state.ut99_executable_path = exe.to_string_lossy().to_string();
                        } else {
                            state.ut99_executable_path = folder.to_string_lossy().to_string();
                        }
                        state.sync_audio_installation();
                    }
                }

                if ui.button("📄 Browse Exe...").on_hover_text("Select UnrealTournament executable in System/").clicked() {
                    if let Some(file) = rfd::FileDialog::new()
                        .set_title("Select UnrealTournament Executable")
                        .pick_file()
                    {
                        state.ut99_executable_path = file.to_string_lossy().to_string();
                        state.sync_audio_installation();
                    }
                }

                if ui.button("🔍 Re-scan").clicked() {
                    CompilerBridge::set_custom_ut_dir(None);
                    if let Some(ut) = CompilerBridge::find_ut_dir() {
                        let exe = ut.join("System").join("UnrealTournament.exe");
                        if exe.exists() {
                            state.ut99_executable_path = exe.to_string_lossy().to_string();
                        } else {
                            state.ut99_executable_path = ut.to_string_lossy().to_string();
                        }
                        state.sync_audio_installation();
                        state.toast_success("Detected UT99 installation successfully!");
                    }
                }
            });
        });

    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new("Next: Workflow & Controls ➔").strong().size(13.0)).clicked() {
                state.welcome_wizard_step = 1;
            }
        });
    });
}

// -----------------------------------------------------------------------------
// STEP 2: WORKFLOW & CONTROLS PROFILE
// -----------------------------------------------------------------------------
fn render_step_workflow_profile(ui: &mut Ui, state: &mut EditorState) {
    ui.label(
        RichText::new("Step 2: Choose Your Interaction & Controls Profile")
            .size(16.0)
            .strong()
            .color(Color32::WHITE),
    );
    ui.label(
        RichText::new("Select how you want the 3D viewport, camera flight, and keyboard shortcuts to behave. You can switch this at any time in Preferences.")
            .size(12.5)
            .color(ThemeColors::TEXT_MUTED),
    );
    ui.add_space(14.0);

    let is_modern = state.keymap_profile == KeymapProfile::Modern;

    ui.columns(2, |cols| {
        // Option 1: Modern Level Designer
        cols[0].group(|ui| {
            let is_sel = is_modern;
            let border_color = if is_sel { ThemeColors::ACCENT_AZURE } else { ThemeColors::BORDER_SUBTLE };
            Frame::new()
                .fill(if is_sel { Color32::from_rgb(20, 32, 48) } else { ThemeColors::BG_PANEL })
                .stroke(Stroke::new(if is_sel { 2.0_f32 } else { 1.0_f32 }, border_color))
                .corner_radius(6.0)
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🌟").size(20.0));
                        ui.label(RichText::new("Modern Level Designer").strong().size(15.0).color(ThemeColors::ACCENT_AZURE));
                    });
                    ui.label(RichText::new("Recommended for newcomers, Blender, and Unreal 5 designers.").size(11.5).color(ThemeColors::TEXT_MUTED));
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.label(RichText::new("• Flight: WASD + RMB free camera look").size(12.0));
                    ui.label(RichText::new("• Gizmos: W=Move, E=Rotate, R=Scale").size(12.0));
                    ui.label(RichText::new("• Spacebar: Cycle transform tool").size(12.0));
                    ui.label(RichText::new("• Esc: Instant transaction rollback").size(12.0));
                    ui.label(RichText::new("• Alt+1..7: Shading diagnostics mode").size(12.0));
                    ui.label(RichText::new("• Ctrl+P: Fuzzy Command Palette").size(12.0));

                    ui.add_space(12.0);
                    if ui.selectable_label(is_sel, RichText::new(if is_sel { "✔ Active Profile" } else { "Select Modern" }).strong()).clicked() {
                        state.keymap_profile = KeymapProfile::Modern;
                    }
                });
        });

        // Option 2: Classic UnrealEd 2.0 Veteran
        cols[1].group(|ui| {
            let is_sel = !is_modern;
            let border_color = if is_sel { ThemeColors::ACCENT_GOLD } else { ThemeColors::BORDER_SUBTLE };
            Frame::new()
                .fill(if is_sel { Color32::from_rgb(45, 38, 20) } else { ThemeColors::BG_PANEL })
                .stroke(Stroke::new(if is_sel { 2.0_f32 } else { 1.0_f32 }, border_color))
                .corner_radius(6.0)
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🕹").size(20.0));
                        ui.label(RichText::new("Classic UnrealEd 2.0 Veteran").strong().size(15.0).color(ThemeColors::ACCENT_GOLD));
                    });
                    ui.label(RichText::new("For veterans with 20+ years of UnrealEd muscle memory.").size(11.5).color(ThemeColors::TEXT_MUTED));
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.label(RichText::new("• Flight: LMB+RMB classic pan & dolly").size(12.0));
                    ui.label(RichText::new("• Browsers: F4=Textures, Shift+F4=Classes").size(12.0));
                    ui.label(RichText::new("• Surfaces: F5=Surface Properties dialog").size(12.0));
                    ui.label(RichText::new("• Selection: Shift+B=Brush, Shift+S=All").size(12.0));
                    ui.label(RichText::new("• Texture Eyedropper: Alt+RMB Pick, Alt+LMB Apply").size(12.0));
                    ui.label(RichText::new("• Rebuild: Ctrl+F8 complete engine build").size(12.0));

                    ui.add_space(12.0);
                    if ui.selectable_label(is_sel, RichText::new(if is_sel { "✔ Active Profile" } else { "Select Classic" }).strong()).clicked() {
                        state.keymap_profile = KeymapProfile::ClassicUnrealEd2;
                    }
                });
        });
    });

    ui.add_space(16.0);
    ui.horizontal(|ui| {
        if ui.button("⬅ Back: Game Path").clicked() {
            state.welcome_wizard_step = 0;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new("Next: Start Mapping ➔").strong().size(13.0)).clicked() {
                state.welcome_wizard_step = 2;
            }
        });
    });
}

// -----------------------------------------------------------------------------
// STEP 3: STARTER LAUNCH & READY TO MAP
// -----------------------------------------------------------------------------
fn render_step_starter_launch(ui: &mut Ui, state: &mut EditorState) {
    ui.label(
        RichText::new("Step 3: Ready to Create!")
            .size(16.0)
            .strong()
            .color(Color32::WHITE),
    );
    ui.label(
        RichText::new("How would you like to begin your session? Choose a starting point:")
            .size(12.5)
            .color(ThemeColors::TEXT_MUTED),
    );
    ui.add_space(12.0);

    let mut start_choice = None;

    ui.vertical(|ui| {
        // Choice 1: Starter Arena
        Frame::group(ui.style())
            .fill(ThemeColors::BG_PANEL)
            .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🎮").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Open Interactive Starter Arena (Instant Playtest)").strong().size(14.0).color(ThemeColors::ACCENT_AZURE));
                        ui.label(
                            RichText::new("Loads a battle-ready deathmatch arena with lighting, weapon pickups, and bot pathing. Press F8 to play immediately!")
                                .size(11.5)
                                .color(ThemeColors::TEXT_MUTED),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("▶ Load Starter Arena").strong()).clicked() {
                            start_choice = Some(StarterExperience::StarterArena);
                        }
                    });
                });
            });

        ui.add_space(8.0);

        // Choice 2: 1-Click Mystery Map
        Frame::group(ui.style())
            .fill(ThemeColors::BG_PANEL)
            .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚡").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("1-Click Mystery Map (Procedural Synthesis)").strong().size(14.0).color(ThemeColors::ACCENT_GOLD));
                        ui.label(
                            RichText::new("Rolls a completely unique, tournament-legal 3D level with symmetrical bases, weapon circuits, and architectural rafters.")
                                .size(11.5)
                                .color(ThemeColors::TEXT_MUTED),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("🎲 Roll & Build 3D").strong()).clicked() {
                            start_choice = Some(StarterExperience::MysteryMap);
                        }
                    });
                });
            });

        ui.add_space(8.0);

        // Choice 3: Blank Subtractive Room
        Frame::group(ui.style())
            .fill(ThemeColors::BG_PANEL)
            .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⬜").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Blank Subtractive Canvas").strong().size(14.0).color(Color32::WHITE));
                        ui.label(
                            RichText::new("Starts with a clean carved room (1024x1024x512) and Red Builder Brush for classic hands-on level design.")
                                .size(11.5)
                                .color(ThemeColors::TEXT_MUTED),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Enter Blank Room").clicked() {
                            start_choice = Some(StarterExperience::BlankRoom);
                        }
                    });
                });
            });
    });

    ui.add_space(14.0);
    ui.separator();
    ui.add_space(10.0);

    let override_id = egui::Id::new("welcome_wizard_dont_show_override");
    let mut dont_show = ui
        .ctx()
        .data_mut(|d| d.get_temp::<bool>(override_id))
        .unwrap_or(true);

    ui.horizontal(|ui| {
        if ui
            .checkbox(
                &mut dont_show,
                "Don't show this Welcome Wizard on startup again",
            )
            .changed()
        {
            ui.ctx().data_mut(|d| d.insert_temp(override_id, dont_show));
            state.first_run_completed = dont_show;
            state.save_preferences();
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new("🚀 Launch Forge99").strong().size(14.0)).clicked() {
                start_choice = Some(StarterExperience::BlankRoom);
            }
            if ui.button("⬅ Back: Controls").clicked() {
                state.welcome_wizard_step = 1;
            }
        });
    });

    if let Some(choice) = start_choice {
        finish_welcome_wizard(state, choice, dont_show);
    }
}

fn finish_welcome_wizard(
    state: &mut EditorState,
    choice: StarterExperience,
    dont_show_on_startup: bool,
) {
    state.first_run_completed = dont_show_on_startup;
    state.show_welcome_wizard = false;
    state.save_preferences();

    match choice {
        StarterExperience::StarterArena => {
            // Attempt loading DM-Tutorial.unr from discovered UT path if present
            let ut_maps = CompilerBridge::find_ut_dir().map(|d| d.join("Maps"));
            let tutorial_map = ut_maps.as_ref().map(|m| m.join("DM-Tutorial.unr"));
            if let Some(ref map_path) = tutorial_map {
                if map_path.exists() {
                    let _ = state.load_unr(map_path);
                    state.toast_success("Loaded Starter Arena: DM-Tutorial.unr (Press F8 to play!)");
                    return;
                }
            }
            // Fallback: Generate a high-speed starter arena with wizard
            state.randomize_all_wizard_settings();
            state.generate_map_from_wizard();
            state.toast_success("Generated Starter Arena! Press F8 to play.");
        }
        StarterExperience::MysteryMap => {
            state.randomize_all_wizard_settings();
            state.generate_map_from_wizard();
            state.toast_success("Generated 1-Click Mystery Map! Press F8 to play.");
        }
        StarterExperience::BlankRoom => {
            state.toast_info("Welcome to Forge99. Ready for level authoring.");
        }
    }
}
