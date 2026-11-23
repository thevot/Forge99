use crate::editor_state::EditorState;
use egui::{Color32, RichText, Stroke};

pub fn render_mesh_browser(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_mesh_browser {
        return;
    }

    let mut is_open = state.show_mesh_browser;
    let mut close_requested = false;

    crate::gui::modal::studio_dialog_window(ctx, "Mesh & Static Mesh Browser", &mut is_open)
        .show(ctx, |ui| {
            // 1. Top Controls Bar: Category filter, Search query & Import OBJ
            ui.horizontal(|ui| {
                ui.label(RichText::new("Category:").strong());

                let categories = state.mesh_library.categories();
                let mut current_cat = state.mesh_browser_category.clone();
                egui::ComboBox::from_id_salt("mesh_browser_cat_combo")
                    .selected_text(&current_cat)
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for cat in &categories {
                            if ui.selectable_value(&mut current_cat, cat.clone(), cat).clicked() {
                                state.mesh_browser_category = current_cat.clone();
                            }
                        }
                    });
                if state.mesh_browser_category != current_cat {
                    state.mesh_browser_category = current_cat;
                }

                ui.separator();
                ui.label(RichText::new("Filter:").strong());
                ui.add(
                    egui::TextEdit::singleline(&mut state.mesh_browser_search)
                        .hint_text("Search mesh names...")
                        .desired_width(140.0),
                );
                if !state.mesh_browser_search.is_empty() {
                    if ui.button("✖").clicked() {
                        state.mesh_browser_search.clear();
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📥 Import Wavefront OBJ...")
                        .on_hover_text("Import a custom 3D .obj model file into the mesh library")
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Wavefront OBJ (*.obj)", &["obj"])
                            .pick_file()
                        {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("CustomMesh");
                                match state.import_obj_to_mesh_library(stem, "Custom", "Imported from external OBJ file", &content) {
                                    Ok(()) => state.log(format!("Imported mesh '{}' from '{}'", stem, path.display())),
                                    Err(e) => state.log(format!("Failed to import OBJ: {e}")),
                                }
                            }
                        }
                    }
                });
            });

            ui.separator();

            // Filter matching meshes
            let query = state.mesh_browser_search.trim().to_lowercase();
            let cur_cat = state.mesh_browser_category.clone();

            let filtered_meshes: Vec<crate::mesh_manager::EditorMeshResource> = state
                .mesh_library
                .meshes
                .iter()
                .filter(|m| {
                    let matches_cat = cur_cat == "All" || m.category.eq_ignore_ascii_case(&cur_cat);
                    let matches_query = query.is_empty() || m.name.to_lowercase().contains(&query) || m.description.to_lowercase().contains(&query);
                    matches_cat && matches_query
                })
                .cloned()
                .collect();

            // 2. Main Content Split: Left List / Right Inspection View
            ui.columns(2, |columns| {
                // Left Column: Mesh List
                columns[0].vertical(|ui| {
                    ui.label(
                        RichText::new(format!("Available Models ({}):", filtered_meshes.len()))
                            .strong()
                            .color(Color32::from_rgb(170, 200, 240)),
                    );
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .id_salt("mesh_browser_list_scroll")
                        .max_height(380.0)
                        .show(ui, |ui| {
                            for m in &filtered_meshes {
                                let is_selected = state.selected_mesh_name.as_deref() == Some(&m.name);
                                let (cat_color, cat_badge) = match m.category.as_str() {
                                    "Architecture" => (Color32::from_rgb(100, 200, 255), "🏛"),
                                    "Decorations" => (Color32::from_rgb(255, 215, 100), "🏺"),
                                    "Props" => (Color32::from_rgb(255, 160, 90), "📦"),
                                    _ => (Color32::from_rgb(140, 240, 140), "✨"),
                                };

                                let item_label = format!("{} {} ({} polys)", cat_badge, m.name, m.poly_count());
                                let resp = ui.selectable_label(is_selected, RichText::new(item_label).color(if is_selected { Color32::from_rgb(255, 230, 110) } else { Color32::WHITE }));
                                if resp.double_clicked() {
                                    state.selected_mesh_name = Some(m.name.clone());
                                    let loc = state.builder_brush.location;
                                    if let Err(e) = state.place_mesh_from_library(&m.name, loc) {
                                        state.log(format!("Failed to place mesh: {e}"));
                                    } else {
                                        state.toast_success(format!("Placed mesh '{}' at Builder Pivot", m.name));
                                    }
                                } else if resp.clicked() {
                                    state.selected_mesh_name = Some(m.name.clone());
                                }
                                resp.on_hover_ui(|ui| {
                                    ui.label(RichText::new(&m.name).strong());
                                    ui.label(RichText::new(format!("Category: {}", m.category)).color(cat_color));
                                    ui.label(&m.description);
                                    let dim = m.dimensions();
                                    ui.label(format!("Size: {:.0} × {:.0} × {:.0} uu", dim.x, dim.y, dim.z));
                                    ui.separator();
                                    ui.label(RichText::new("• Double-click to place in level at Builder Pivot").color(Color32::LIGHT_GRAY).size(10.0));
                                });
                            }
                        });
                });

                // Right Column: Mesh Details & Inspector
                columns[1].vertical(|ui| {
                    if let Some(sel_name) = state.selected_mesh_name.clone() {
                        if let Some(mesh) = state.mesh_library.get_mesh(&sel_name) {
                            let dim = mesh.dimensions();
                            let poly_count = mesh.poly_count();
                            let vert_count = mesh.vertex_count();

                            ui.label(
                                RichText::new(format!("🗿 {}", mesh.name))
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::from_rgb(255, 220, 100)),
                            );
                            ui.label(
                                RichText::new(format!("Category: {} | CSG Model", mesh.category))
                                    .size(11.0)
                                    .color(Color32::from_rgb(160, 190, 220)),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(&mesh.description)
                                    .italics()
                                    .color(Color32::from_rgb(200, 205, 215)),
                            );
                            ui.add_space(8.0);

                            // Metrics Frame
                            egui::Frame::group(ui.style())
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 75, 95)))
                                .fill(Color32::from_rgba_premultiplied(22, 28, 38, 220))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("📐 Geometry & Dimensions").strong().size(12.0));
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Bounding Box:").strong());
                                        ui.label(format!("{:.0} × {:.0} × {:.0} uu", dim.x, dim.y, dim.z));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Polygons:").strong());
                                        ui.label(format!("{poly_count} faces"));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Total Vertices:").strong());
                                        ui.label(format!("{vert_count} indices"));
                                    });

                                    // Material textures list
                                    let mut mats = std::collections::BTreeSet::new();
                                    for p in &mesh.brush.polys {
                                        if !p.texture.is_empty() {
                                            mats.insert(p.texture.clone());
                                        }
                                    }
                                    if !mats.is_empty() {
                                        ui.separator();
                                        ui.label(RichText::new("Materials:").strong());
                                        for mat in mats.iter().take(4) {
                                            ui.label(format!(" • {mat}"));
                                        }
                                    }
                                });

                            ui.add_space(10.0);

                            // Action Buttons
                            ui.vertical(|ui| {
                                if ui.button(RichText::new("🗿 Place Mesh in Level (at Builder Pivot)").strong().color(Color32::from_rgb(100, 230, 150)))
                                    .on_hover_text("Places a static instance of this mesh in the level at the current builder pivot location")
                                    .clicked()
                                {
                                    let loc = state.builder_brush.location;
                                    if let Err(e) = state.place_mesh_from_library(&sel_name, loc) {
                                        state.log(format!("Failed to place mesh: {e}"));
                                    }
                                }

                                ui.add_space(4.0);

                                if ui.button(RichText::new("✂ Copy to Red Builder Brush").color(Color32::from_rgb(255, 160, 100)))
                                    .on_hover_text("Loads this mesh into the active Red Builder Brush for subtractive/additive CSG carving")
                                    .clicked()
                                {
                                    if let Err(e) = state.extract_mesh_to_builder(&sel_name) {
                                        state.log(format!("Failed to extract mesh: {e}"));
                                    }
                                }

                                ui.add_space(4.0);

                                if ui.button("💾 Export Mesh to OBJ...")
                                    .on_hover_text("Export this 3D model to a Wavefront .obj file on disk")
                                    .clicked()
                                {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Wavefront OBJ (*.obj)", &["obj"])
                                        .set_file_name(&format!("{}.obj", sel_name))
                                        .save_file()
                                    {
                                        if let Some(obj_str) = state.mesh_library.export_obj(&sel_name) {
                                            if let Err(e) = std::fs::write(&path, obj_str) {
                                                state.log(format!("Failed to save OBJ: {e}"));
                                            } else {
                                                state.log(format!("Exported mesh '{sel_name}' to '{}'", path.display()));
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.label(RichText::new("👈 Select a 3D model from the left list").color(Color32::LIGHT_GRAY));
                            ui.add_space(8.0);
                            ui.label("You can inspect dimensions, place instances directly into the level, or copy the geometry into the Red Builder Brush.");
                        });
                    }
                });
            });

            ui.add_space(8.0);
            ui.separator();

            // 3. Bottom Action Bar
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Mesh Library: {} models available", state.mesh_library.meshes.len()))
                        .size(11.0)
                        .color(Color32::from_rgb(140, 160, 180)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });
            });
        });

    if close_requested {
        state.show_mesh_browser = false;
    } else {
        state.show_mesh_browser = is_open;
    }
}
