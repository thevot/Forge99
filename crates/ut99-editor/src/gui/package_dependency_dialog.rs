use crate::editor_state::EditorState;
use egui::{Color32, RichText, Stroke};

pub fn render_package_dependency_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_package_manifest_dialog {
        return;
    }

    if state.cached_map_manifest.is_none() {
        state.refresh_map_manifest();
    }

    let mut open = state.show_package_manifest_dialog;
    let mut close_requested = false;
    let mut refresh_requested = false;
    let mut copy_readme_requested = false;
    let mut export_readme_requested = false;

    egui::Window::new("Package Dependencies & Distribution Manifest")
        .open(&mut open)
        .default_size([620.0, 680.0])
        .resizable(true)
        .show(ctx, |ui| {
            let manifest = state.cached_map_manifest.clone().unwrap_or_else(|| state.generate_map_manifest());

            // Header Banner
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Map: {}.unr", manifest.map_name)).strong().size(15.0).color(Color32::from_rgb(100, 200, 255)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖ Close").clicked() {
                        close_requested = true;
                    }
                    if ui.button("🔄 Refresh").on_hover_text("Re-scan level geometry, actors and audio for dependencies").clicked() {
                        refresh_requested = true;
                    }
                });
            });

            ui.add_space(4.0);

            // Compatibility Status Badge
            egui::Frame::group(ui.style())
                .stroke(Stroke::new(1.0f32, if manifest.is_vanilla_compatible() { Color32::from_rgb(40, 160, 60) } else { Color32::from_rgb(200, 140, 30) }))
                .fill(if manifest.is_vanilla_compatible() { Color32::from_rgba_premultiplied(15, 45, 20, 200) } else { Color32::from_rgba_premultiplied(50, 35, 10, 200) })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if manifest.is_vanilla_compatible() {
                            ui.label(RichText::new("✅ 100% VANILLA COMPATIBLE").strong().size(13.0).color(Color32::from_rgb(100, 240, 130)));
                            ui.label(RichText::new("— All required assets ship with original UT99 v436. Safe for all servers.").size(11.0));
                        } else {
                            ui.label(RichText::new(format!("⚠ REQUIRES {} CUSTOM PACKAGES", manifest.custom_dependencies.len())).strong().size(13.0).color(Color32::from_rgb(255, 195, 65)));
                            ui.label(RichText::new("— Non-stock assets must be distributed in archive or server redirect.").size(11.0));
                        }
                    });
                });

            ui.add_space(8.0);

            // Level Identification Metadata (Editable)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Title:").strong());
                ui.text_edit_singleline(&mut state.level_title);
                ui.separator();
                ui.label(RichText::new("Author:").strong());
                ui.text_edit_singleline(&mut state.level_author);
                ui.separator();
                ui.label(RichText::new("Players:").strong());
                ui.text_edit_singleline(&mut state.level_ideal_player_count);
                if ui.button(RichText::new("💾 Apply").strong().color(Color32::from_rgb(100, 240, 130))).on_hover_text("Save metadata to LevelInfo").clicked() {
                    let map_name = state.map_path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "UntitledMap".to_string());
                    let title = state.level_title.clone();
                    let author = state.level_author.clone();
                    let players = state.level_ideal_player_count.clone();
                    let song = state.level_song.clone();
                    state.apply_manifest_metadata(&map_name, &title, &author, &players, &song, None, None);
                    state.toast_success("Level metadata updated");
                }
                if ui.button("📦 Packaging Dialog...").on_hover_text("Open full Map Packaging & Release Manifest dialog").clicked() {
                    state.show_map_packaging_dialog = true;
                }
            });

            ui.separator();

            // Tabs / Section
            ui.collapsing(RichText::new(format!("Custom Dependencies ({})", manifest.custom_dependencies.len())).strong().color(if manifest.custom_dependencies.is_empty() { Color32::GRAY } else { Color32::from_rgb(255, 180, 50) }), |ui| {
                if manifest.custom_dependencies.is_empty() {
                    ui.label("No third-party packages detected. No extra files required.");
                } else {
                    egui::Grid::new("custom_deps_grid").striped(true).min_col_width(120.0).show(ui, |ui| {
                        ui.label(RichText::new("Package Name").strong());
                        ui.label(RichText::new("Type").strong());
                        ui.label(RichText::new("Target Subfolder").strong());
                        ui.end_row();

                        for dep in &manifest.custom_dependencies {
                            ui.label(RichText::new(&dep.name).strong());
                            ui.label(&dep.package_type);
                            let subfolder = match dep.package_type.as_str() {
                                "Texture" => "Textures/*.utx",
                                "Sound" => "Sounds/*.uax",
                                "Music" => "Music/*.umx",
                                _ => "System/*.u",
                            };
                            ui.label(RichText::new(subfolder).color(Color32::from_rgb(180, 210, 240)));
                            ui.end_row();
                        }
                    });
                }
            });

            ui.collapsing(RichText::new(format!("Stock UT99 Dependencies ({})", manifest.stock_dependencies.len())).strong(), |ui| {
                egui::Grid::new("stock_deps_grid").striped(true).min_col_width(120.0).show(ui, |ui| {
                    ui.label(RichText::new("Package Name").strong());
                    ui.label(RichText::new("Type").strong());
                    ui.label(RichText::new("Status").strong());
                    ui.end_row();

                    for dep in &manifest.stock_dependencies {
                        ui.label(&dep.name);
                        ui.label(&dep.package_type);
                        ui.label(RichText::new("Stock (Pre-installed)").color(Color32::from_rgb(120, 200, 140)));
                        ui.end_row();
                    }
                });
            });

            ui.separator();

            // Readme Generator Section
            ui.horizontal(|ui| {
                ui.label(RichText::new("📝 Community Release Readme Preview").strong().size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("💾 Export Readme.txt...").clicked() {
                        export_readme_requested = true;
                    }
                    if ui.button("📋 Copy to Clipboard").clicked() {
                        copy_readme_requested = true;
                    }
                });
            });

            ui.add_space(4.0);

            let readme_content = manifest.generate_readme();
            egui::ScrollArea::vertical()
                .max_height(240.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut readme_content.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .interactive(false)
                    );
                });
        });

    if refresh_requested {
        state.refresh_map_manifest();
        state.log("Package dependencies re-scanned successfully.");
    }

    if copy_readme_requested {
        let text = state.get_manifest_readme_text();
        ctx.copy_text(text);
        state.log("Copied community Readme.txt to system clipboard.");
    }

    if export_readme_requested {
        let def_name = format!("{}_Readme.txt", state.map_path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Map".to_string()));
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(&def_name)
            .add_filter("Text Document (*.txt)", &["txt"])
            .save_file()
        {
            if let Err(e) = state.export_manifest_readme(&path) {
                state.log(format!("Failed to export Readme.txt: {e}"));
            } else {
                state.log(format!("Exported community distribution Readme to {:?}", path));
            }
        }
    }

    if close_requested {
        state.show_package_manifest_dialog = false;
    } else {
        state.show_package_manifest_dialog = open;
    }
}
