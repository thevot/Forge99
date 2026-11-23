use crate::editor_state::EditorState;

pub fn render_autosave_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_autosave_dialog {
        return;
    }

    let mut open = state.show_autosave_dialog;
    let mut close_dialog = false;

    crate::gui::modal::standard_dialog_window(
        ctx,
        "Auto-Save & Session Recovery",
        &mut open,
        [500.0, 460.0],
        [380.0, 320.0],
    )
    .show(ctx, |ui| {
            ui.label(egui::RichText::new("Automatic rolling snapshot system to protect your map progress from accidental loss.").italics());
            ui.separator();

            ui.group(|ui| {
                ui.label(egui::RichText::new("Configuration").strong());
                ui.checkbox(&mut state.auto_save_enabled, "Enable Periodic Auto-Save");

                ui.horizontal(|ui| {
                    ui.label("Interval (minutes):");
                    let mut mins = (state.auto_save_interval_secs / 60.0).round() as i32;
                    if ui.add(egui::Slider::new(&mut mins, 1..=60).suffix(" min")).changed() {
                        state.auto_save_interval_secs = (mins as f32) * 60.0;
                    }
                });

                let elapsed_mins = state.auto_save_timer / 60.0;
                let next_save_mins = (state.auto_save_interval_secs - state.auto_save_timer).max(0.0) / 60.0;
                ui.label(format!("Time elapsed since last check: {:.1} min (Next auto-save in {:.1} min)", elapsed_mins, next_save_mins));

                let backup_dir = state.auto_save_directory().to_path_buf();
                ui.label(format!("Session backups (T3D / UNR): {}", backup_dir.display()));
                if ui.button("Open Backup Folder").clicked() {
                    match std::fs::create_dir_all(&backup_dir) {
                        Ok(()) => state.reveal_in_file_explorer(&backup_dir),
                        Err(e) => {
                            let msg = format!("Could not open backup folder '{}': {e}", backup_dir.display());
                            state.log(&msg);
                            state.toast_error(msg);
                        }
                    }
                }

                ui.horizontal(|ui| {
                    if ui.button("💾 Trigger Auto-Save Snapshot Now").clicked() {
                        state.trigger_auto_save();
                    }
                    if ui
                        .button("🔄 Save Rotating Auto0..Auto9.unr")
                        .on_hover_text("Write rotating UnrealEd 2.0 compatible Auto0.unr..Auto9.unr binary backup in the session folder shown above")
                        .clicked()
                    {
                        match state.save_rotating_autosave_unr(&backup_dir) {
                            Ok(path) => state.toast_success(format!("UNR backup saved to '{}'", path.display())),
                            Err(e) => {
                                let msg = format!("UNR backup failed at '{}': {e}", backup_dir.display());
                                state.log(&msg);
                                state.toast_error(msg);
                            }
                        }
                    }
                    if ui
                        .button("🛡 Open .unr in Safe Recovery Mode...")
                        .on_hover_text("Recover brushes, actors, and embedded MyLevel assets from a map with corrupted compiled BSP nodes")
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Unreal Map (*.unr)", &["unr"])
                            .pick_file()
                        {
                            if let Err(e) = state.load_unr_safe_recovery(&path) {
                                let msg = format!("Safe recovery failed for '{}': {e}", path.display());
                                state.log(&msg);
                                state.toast_error(msg);
                            }
                        }
                    }
                });
            });

            ui.add_space(8.0);
            ui.label(egui::RichText::new("Available Rolling Snapshots (Most Recent First)").strong());
            ui.label("Restore replaces the current map with an unsaved copy. Choose a destination when saving.");
            ui.separator();

            if state.auto_save_snapshots.is_empty() {
                ui.label(egui::RichText::new("No auto-save snapshots captured yet in this session.").italics().color(egui::Color32::GRAY));
            } else {
                let mut restore_idx = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, (timestamp, t3d_data)) in state.auto_save_snapshots.iter().enumerate().rev() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("Snapshot #{}", i + 1)).strong());
                                ui.label(format!("Time: {}", timestamp));
                                ui.label(egui::RichText::new(format!("{:.1} KB", t3d_data.len() as f32 / 1024.0)).monospace().size(11.0));

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("↺ Restore Copy").on_hover_text("Replace the current map with this snapshot as an unsaved copy; saving requires a destination").clicked() {
                                        restore_idx = Some(i);
                                    }
                                });
                            });
                        });
                        ui.add_space(2.0);
                    }
                });

                if let Some(idx) = restore_idx {
                    if let Err(e) = state.restore_auto_save_snapshot(idx) {
                        let msg = format!("Error restoring snapshot: {e}");
                        state.log(&msg);
                        state.toast_error(msg);
                    }
                }
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    close_dialog = true;
                }
            });
        });

    if close_dialog {
        open = false;
    }
    state.show_autosave_dialog = open;
}
