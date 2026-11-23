use crate::editor_state::EditorState;

pub fn render_play_settings_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_play_settings_dialog {
        return;
    }

    let mut open = state.show_play_settings_dialog;
    let mut close_dialog = false;

    crate::gui::modal::compact_dialog_window(ctx, "UT99 Playtest & Launch Settings", &mut open)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("Configure Unreal Tournament 99 binary path and command-line execution flags for one-click playtesting (F8).").italics());
            ui.separator();

            ui.group(|ui| {
                ui.label(egui::RichText::new("Game Executable or UT Folder").strong());
                ui.horizontal(|ui| {
                    if ui.text_edit_singleline(&mut state.ut99_executable_path).lost_focus() {
                        state.sync_audio_installation();
                    }
                    if ui.button("📁 Browse...").clicked() {
                        let mut dialog = rfd::FileDialog::new();
                        #[cfg(windows)]
                        { dialog = dialog.add_filter("Executable (*.exe)", &["exe"]); }
                        let current_path = std::path::Path::new(&state.ut99_executable_path);
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
                });

                match state.resolved_playtest_executable() {
                    Ok(path) => { ui.colored_label(egui::Color32::from_rgb(100, 220, 120), format!("✔ Executable: {}", path.display())); }
                    Err(message) => { ui.colored_label(egui::Color32::from_rgb(255, 140, 80), message); }
                }
            });

            ui.add_space(6.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Launch Arguments & Game Options").strong());
                ui.text_edit_singleline(&mut state.ut99_launch_args);
                ui.label("Use double quotes around paths containing spaces. ?game and ?mutator options are added to the map URL.");

                ui.add_space(4.0);
                ui.label("Quick Presets:");
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

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("▶ Save & Launch Playtest (F8)").clicked() {
                    if state.play_map_in_ut99().is_ok() { close_dialog = true; }
                }
                if ui.button("Close").clicked() {
                    close_dialog = true;
                }
            });
        });

    if close_dialog {
        open = false;
    }
    state.show_play_settings_dialog = open;
}
