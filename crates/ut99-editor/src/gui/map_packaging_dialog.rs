use crate::editor_state::EditorState;
use egui::{Color32, RichText};

pub fn render_map_packaging_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_map_packaging_dialog {
        return;
    }

    let mut open = state.show_map_packaging_dialog;

    egui::Window::new("Map Packaging & Release Manifest")
        .open(&mut open)
        .resizable(true)
        .default_width(580.0)
        .default_height(540.0)
        .show(ctx, |ui| {
            ui.heading("Map Packaging & Dependency Audit");
            ui.label(
                RichText::new("Automated asset audit, stock package verification, and community release packaging.")
                    .size(11.0)
                    .color(Color32::from_rgb(180, 180, 180)),
            );
            ui.separator();

            if state.packaging_map_name_edit.is_empty() {
                state.packaging_map_name_edit = state.map_path.as_ref()
                    .and_then(|p| p.file_stem())
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "UntitledMap".to_string());
            }

            let mut manifest = state.generate_map_manifest();
            if !state.packaging_map_name_edit.trim().is_empty() {
                manifest.map_name = state.packaging_map_name_edit.trim().trim_end_matches(".unr").to_string();
            }

            // Compatibility Status Banner
            ui.horizontal(|ui| {
                if manifest.is_vanilla_compatible() {
                    ui.label(
                        RichText::new("✅ 100% VANILLA UT99 COMPATIBLE")
                            .color(Color32::from_rgb(80, 230, 120))
                            .strong(),
                    );
                    ui.label("(All textures, sounds, and classes use stock packages)");
                } else {
                    ui.label(
                        RichText::new(format!("⚠ {} CUSTOM PACKAGES REQUIRED", manifest.custom_dependencies.len()))
                            .color(Color32::from_rgb(255, 180, 50))
                            .strong(),
                    );
                    ui.label("(Must bundle external asset files with release)");
                }
            });

            ui.add_space(4.0);

            // Level Info Header & Interactive Metadata Editor
            egui::CollapsingHeader::new(RichText::new("◈ Map Metadata & Release Specifications").strong())
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("manifest_meta_grid").num_columns(2).spacing([12.0, 6.0]).striped(true).show(ui, |ui| {
                        ui.label(RichText::new("Map Filename:").strong());
                        ui.horizontal(|ui| {
                            let resp = ui.text_edit_singleline(&mut state.packaging_map_name_edit);
                            if resp.changed() {
                                let clean = state.packaging_map_name_edit.trim();
                                if !clean.is_empty() {
                                    let stem = clean.trim_end_matches(".unr").trim_end_matches(".UNR");
                                    if let Some(ref mut p) = state.map_path {
                                        p.set_file_name(format!("{stem}.unr"));
                                    } else {
                                        state.map_path = Some(std::path::PathBuf::from(format!("UnrealTournament/Maps/{stem}.unr")));
                                    }
                                }
                            }
                            ui.label(".unr");
                        });
                        ui.end_row();

                        ui.label(RichText::new("Title:").strong());
                        ui.text_edit_singleline(&mut state.level_title);
                        ui.end_row();

                        ui.label(RichText::new("Author / Mapper:").strong());
                        ui.text_edit_singleline(&mut state.level_author);
                        ui.end_row();

                        ui.label(RichText::new("Ideal Players:").strong());
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut state.level_ideal_player_count);
                            for preset in &["1v1", "2-4", "4-8", "8-16", "16-32"] {
                                if ui.small_button(*preset).on_hover_text(format!("Set to {preset} Players")).clicked() {
                                    state.level_ideal_player_count = preset.to_string();
                                }
                            }
                        });
                        ui.end_row();

                        ui.label(RichText::new("Music Track:").strong());
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut state.level_song);
                            if ui.button("♪ Browse...").on_hover_text("Open UT99 Music Browser").clicked() {
                                state.show_music_browser = true;
                            }
                            egui::ComboBox::from_id_salt("quick_music_preset_combo")
                                .selected_text("Presets...")
                                .show_ui(ui, |ui| {
                                    let tracks = [
                                        "Cannon", "Colossus", "Course", "Enigma", "Foregone", "GoDown",
                                        "Lock", "Mechanism", "Mission", "Nether", "Organic", "Phantom",
                                        "Razor-ub", "Run", "Saveme", "Seeker", "Skyward", "Strider",
                                        "Subrosa", "Superfist", "Theme", "Underworld", "Wheels",
                                    ];
                                    for track in &tracks {
                                        if ui.selectable_label(state.level_song.eq_ignore_ascii_case(track), *track).clicked() {
                                            state.level_song = format!("{track}.umx");
                                        }
                                    }
                                });
                        });
                        ui.end_row();

                        ui.label(RichText::new("GameType:").strong());
                        ui.horizontal(|ui| {
                            let presets = [
                                "Botpack.DeathMatchPlus",
                                "Botpack.CTFGame",
                                "Botpack.Domination",
                                "Botpack.Assault",
                                "Botpack.TeamGamePlus",
                                "UnrealShare.SinglePlayer",
                            ];
                            egui::ComboBox::from_id_salt("packaging_gametype_combo")
                                .selected_text(if state.level_game_type.is_empty() { "Select Preset..." } else { &state.level_game_type })
                                .show_ui(ui, |ui| {
                                    for preset in &presets {
                                        ui.selectable_value(&mut state.level_game_type, (*preset).to_string(), *preset);
                                    }
                                });
                            ui.text_edit_singleline(&mut state.level_game_type);
                        });
                        ui.end_row();

                        ui.label(RichText::new("Level Enter Text:").strong());
                        ui.text_edit_singleline(&mut state.level_enter_text);
                        ui.end_row();

                        ui.label(RichText::new("Total Actors:").strong());
                        ui.label(format!("{}", manifest.total_actors));
                        ui.end_row();
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("💾 Apply Changes to LevelInfo & Map").strong().color(Color32::from_rgb(100, 240, 130)))
                            .on_hover_text("Synchronizes Title, Author, Players, Music, and GameType to LevelInfo0 actor and marks map dirty")
                            .clicked()
                        {
                            let map_name = state.packaging_map_name_edit.clone();
                            let title = state.level_title.clone();
                            let author = state.level_author.clone();
                            let players = state.level_ideal_player_count.clone();
                            let song = state.level_song.clone();
                            let game_type = state.level_game_type.clone();
                            let enter_text = state.level_enter_text.clone();
                            state.apply_manifest_metadata(&map_name, &title, &author, &players, &song, Some(&game_type), Some(&enter_text));
                            state.toast_success("Applied manifest metadata to LevelInfo & Map");
                        }

                        if ui.button("↺ Reset from Level").on_hover_text("Restore fields from active map path and LevelInfo").clicked() {
                            if let Some(ref path) = state.map_path {
                                state.packaging_map_name_edit = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "UntitledMap".to_string());
                            }
                            state.toast_info("Reset manifest input fields");
                        }
                    });
                });

            ui.add_space(8.0);
            ui.separator();

            // Tabs for Stock vs Custom vs Readme
            ui.collapsing(format!("Stock Dependencies ({} packages)", manifest.stock_dependencies.len()), |ui| {
                egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                    for dep in &manifest.stock_dependencies {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&dep.name).strong());
                            ui.label(format!("[{}]", dep.package_type));
                            ui.colored_label(Color32::from_rgb(120, 200, 120), "Stock v436");
                        });
                    }
                });
            });

            if !manifest.custom_dependencies.is_empty() {
                ui.collapsing(format!("⚠ Custom External Dependencies ({} packages)", manifest.custom_dependencies.len()), |ui| {
                    egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                        for dep in &manifest.custom_dependencies {
                            let folder = match dep.package_type.as_str() {
                                "Texture" => "Textures/",
                                "Sound" => "Sounds/",
                                "Music" => "Music/",
                                _ => "System/",
                            };
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&dep.name).strong().color(Color32::from_rgb(255, 200, 80)));
                                ui.label(format!("Type: {}", dep.package_type));
                                ui.colored_label(Color32::from_rgb(180, 180, 255), format!("Deploy to: UnrealTournament/{}", folder));
                            });
                        }
                    });
                });
            }

            ui.add_space(8.0);
            ui.separator();

            // Release Readme Generator
            ui.label(RichText::new("📄 Release Readme.txt Preview").strong());
            let readme = manifest.generate_readme();
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    let mut text_copy = readme.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut text_copy)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                    );
                });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("📋 Copy Readme to Clipboard").clicked() {
                    ctx.copy_text(readme.clone());
                    state.log("Release Readme copied to clipboard.");
                }

                if ui.button("💾 Export Readme.txt").clicked() {
                    let export_path = if let Some(ref p) = state.map_path {
                        p.with_file_name(format!("{}_Readme.txt", manifest.map_name))
                    } else {
                        std::path::PathBuf::from(format!("{}_Readme.txt", manifest.map_name))
                    };

                    match std::fs::write(&export_path, &readme) {
                        Ok(_) => state.log(format!("Successfully exported release readme to '{}'", export_path.display())),
                        Err(e) => state.log(format!("Failed to write readme: {}", e)),
                    }
                }
            });
        });

    state.show_map_packaging_dialog = open;
}
