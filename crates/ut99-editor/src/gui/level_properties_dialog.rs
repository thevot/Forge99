use egui::{self, Color32, RichText};
use crate::editor_state::EditorState;
use crate::gui::ut99_light_to_color;

pub fn render_level_properties_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_level_properties_dialog {
        return;
    }

    let mut open = state.show_level_properties_dialog;
    let mut close_requested = false;

    crate::gui::modal::standard_dialog_window(ctx, RichText::new("Level Properties (F6)").strong(), &mut open, [460.0, 500.0], [360.0, 320.0])
        .show(ctx, |ui| {
            ui.add_space(4.0);

            // Header Banner
            ui.horizontal(|ui| {
                ui.label(RichText::new("◫ Unreal Tournament 99 Map Meta & Rules").strong().color(Color32::from_rgb(100, 180, 255)));
            });
            ui.separator();

            // Section 1: Level Identification
            egui::CollapsingHeader::new(RichText::new("◈ Level Identification").strong())
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("level_id_grid")
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Map Title:");
                            ui.text_edit_singleline(&mut state.level_title);
                            ui.end_row();

                            ui.label("Author / Mapper:");
                            ui.text_edit_singleline(&mut state.level_author);
                            ui.end_row();

                            ui.label("Ideal Player Count:");
                            ui.text_edit_singleline(&mut state.level_ideal_player_count);
                            ui.end_row();

                            ui.label("Level Enter Text:");
                            ui.text_edit_singleline(&mut state.level_enter_text);
                            ui.end_row();
                        });
                });

            ui.add_space(6.0);

            // Section 2: Audio & Game Rules
            egui::CollapsingHeader::new(RichText::new("♪ Audio & Game Rules").strong())
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("level_rules_grid")
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Background Music (Song):");
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut state.level_song);
                                if ui.button("♪ Browse...").on_hover_text("Open UT99 Music Browser").clicked() {
                                    state.show_music_browser = true;
                                }
                            });
                            ui.end_row();

                            ui.label("Default GameType:");
                            ui.vertical(|ui| {
                                let presets = [
                                    "Botpack.DeathMatchPlus",
                                    "Botpack.CTFGame",
                                    "Botpack.Domination",
                                    "Botpack.Assault",
                                    "Botpack.TeamGamePlus",
                                    "UnrealShare.SinglePlayer",
                                ];
                                egui::ComboBox::from_id_salt("gametype_preset_combo")
                                    .selected_text(if state.level_game_type.is_empty() { "Select Preset..." } else { &state.level_game_type })
                                    .show_ui(ui, |ui| {
                                        for preset in &presets {
                                            ui.selectable_value(&mut state.level_game_type, (*preset).to_string(), *preset);
                                        }
                                    });
                                ui.text_edit_singleline(&mut state.level_game_type);
                            });
                            ui.end_row();
                        });
                });

            ui.add_space(6.0);

            // Section 3: Default Zone Lighting (Ambient)
            egui::CollapsingHeader::new(RichText::new("☼ Default Zone Ambient Light").strong())
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("level_ambient_grid")
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Ambient Brightness:");
                            ui.add(egui::Slider::new(&mut state.level_ambient_brightness, 0..=255));
                            ui.end_row();

                            ui.label("Ambient Hue:");
                            ui.add(egui::Slider::new(&mut state.level_ambient_hue, 0..=255));
                            ui.end_row();

                            ui.label("Ambient Saturation:");
                            ui.add(egui::Slider::new(&mut state.level_ambient_saturation, 0..=255));
                            ui.end_row();

                            ui.label("Color Swatch:");
                            let swatch = ut99_light_to_color(
                                state.level_ambient_hue,
                                state.level_ambient_saturation,
                                state.level_ambient_brightness,
                            );
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(48.0, 20.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 3.0, swatch);
                                ui.label(format!("#{:02X}{:02X}{:02X}", swatch.r(), swatch.g(), swatch.b()));
                            });
                            ui.end_row();
                        });
                });

            ui.add_space(6.0);

            // Section 4: Map Selection Preview (MyLevel.Screenshot)
            egui::CollapsingHeader::new(RichText::new("🖼 Map Preview Thumbnail (MyLevel.Screenshot)").strong())
                .default_open(true)
                .show(ui, |ui| {
                    let has_screenshot = state
                        .texture_manager
                        .get_package("MyLevel")
                        .map_or(false, |p| p.textures.iter().any(|t| t.name.eq_ignore_ascii_case("Screenshot")));
                    ui.horizontal(|ui| {
                        if has_screenshot {
                            ui.colored_label(Color32::from_rgb(80, 240, 120), "✔ Embedded in MyLevel.Screenshot");
                        } else {
                            ui.colored_label(Color32::from_rgb(200, 180, 100), "○ No MyLevel.Screenshot embedded");
                        }
                        if ui.button("🖼 Use Active Texture as Screenshot").clicked() {
                            if let Some(ref active) = state.active_texture.clone() {
                                let mut parts = active.splitn(2, '.');
                                let pkg = parts.next().unwrap_or("");
                                let name = parts.next().unwrap_or(active);
                                if let Some(tex) = state.texture_manager.get_texture(pkg, name).cloned() {
                                    state.set_mylevel_screenshot_rgba8(tex.width, tex.height, &tex.rgba_pixels);
                                }
                            }
                        }
                    });
                });

            ui.add_space(10.0);
            ui.separator();

            // Footer Buttons
            ui.horizontal(|ui| {
                if ui.button(RichText::new("✓ Apply & Close").strong()).clicked() {
                    state.toast_info("Level Properties updated");
                    close_requested = true;
                }
                if ui.button("↺ Reset Defaults").on_hover_text("Reset to standard DeathMatch defaults").clicked() {
                    state.level_title = "Untitled Arena".to_string();
                    state.level_author = "Unknown Mapper".to_string();
                    state.level_ideal_player_count = "4-8".to_string();
                    state.level_song.clear();
                    state.level_game_type = "Botpack.DeathMatchPlus".to_string();
                    state.level_enter_text.clear();
                    state.level_ambient_brightness = 0;
                    state.level_ambient_hue = 0;
                    state.level_ambient_saturation = 255;
                    state.log("Reset Level Properties to default values.");
                    state.toast_info("Reset Level Properties to defaults");
                }
            });
        });

    if close_requested {
        open = false;
    }

    state.show_level_properties_dialog = open;
}
