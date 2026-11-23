use crate::editor_state::EditorState;

pub fn render_prefab_browser(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_prefab_browser {
        return;
    }

    let mut open = state.show_prefab_browser;
    let mut instantiate_at_builder = None;
    let mut instantiate_at_origin = None;
    let mut save_error = None;

    crate::gui::modal::standard_dialog_window(ctx, "Prefab & Template Library", &mut open, [460.0, 520.0], [360.0, 320.0])
        .show(ctx, |ui| {
            ui.heading("Architectural & Gameplay Templates");
            ui.label(
                egui::RichText::new("Quickly stamp modular architectural components and gameplay assemblies into your map.")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(180, 190, 200)),
            );
            ui.separator();

            // Prefab List
            egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                for (idx, prefab) in state.prefabs.iter().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let icon = match prefab.category.as_str() {
                                "Architecture" => "🏛",
                                "Gameplay" => "⚡",
                                "Lighting" => "💡",
                                "Pickups" => "💊",
                                "Doors" => "🚪",
                                _ => "📦",
                            };
                            ui.label(egui::RichText::new(format!("{} {}", icon, prefab.name)).strong().size(14.0));
                            ui.label(
                                egui::RichText::new(format!("[{}]", prefab.category))
                                    .color(egui::Color32::from_rgb(120, 200, 255))
                                    .size(11.0),
                            );
                        });

                        ui.label(egui::RichText::new(&prefab.description).italics().size(12.0));

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Contents: {} brush(es), {} mover(s), {} actor(s)",
                                    prefab.brushes.len(),
                                    prefab.movers.len(),
                                    prefab.actors.len()
                                ))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(160, 160, 160)),
                            );
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.button("📍 Place at Builder").on_hover_text("Stamp prefab at current Active Builder Brush location").clicked() {
                                instantiate_at_builder = Some(idx);
                            }
                            if ui.button("Place at (0,0,0)").clicked() {
                                instantiate_at_origin = Some(idx);
                            }
                        });
                    });
                    ui.add_space(6.0);
                }
            });

            ui.separator();

            // Section: Save Selected as Custom Prefab
            ui.collapsing("💾 Save Selection as Custom Prefab", |ui| {
                ui.label(egui::RichText::new("Save the currently selected brush, mover, or mesh into your prefab library:").size(11.0));
                
                // Form fields stored in static/simple local storage using egui memory
                let id_name = ui.make_persistent_id("prefab_save_name");
                let id_cat = ui.make_persistent_id("prefab_save_cat");
                let id_desc = ui.make_persistent_id("prefab_save_desc");

                let mut name: String = ui.data_mut(|d| d.get_temp(id_name)).unwrap_or_else(|| "Custom_Prefab".to_string());
                let mut cat: String = ui.data_mut(|d| d.get_temp(id_cat)).unwrap_or_else(|| "Custom".to_string());
                let mut desc: String = ui.data_mut(|d| d.get_temp(id_desc)).unwrap_or_else(|| "User-defined custom template".to_string());

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut name);
                });
                ui.horizontal(|ui| {
                    ui.label("Category:");
                    ui.text_edit_singleline(&mut cat);
                });
                ui.horizontal(|ui| {
                    ui.label("Description:");
                    ui.text_edit_singleline(&mut desc);
                });

                if ui.button("➕ Save to Library").clicked() {
                    if let Err(e) = state.save_selection_as_prefab(&name, &cat, &desc) {
                        save_error = Some(e);
                    }
                }

                ui.data_mut(|d| {
                    d.insert_temp(id_name, name);
                    d.insert_temp(id_cat, cat);
                    d.insert_temp(id_desc, desc);
                });
            });

            if let Some(err) = save_error {
                ui.label(egui::RichText::new(format!("⚠ {err}")).color(egui::Color32::from_rgb(255, 120, 120)));
            }
        });

    if let Some(idx) = instantiate_at_builder {
        let loc = state.builder_brush.location;
        state.instantiate_prefab(idx, loc);
    } else if let Some(idx) = instantiate_at_origin {
        state.instantiate_prefab(idx, glam::Vec3::ZERO);
    }

    state.show_prefab_browser = open;
}
