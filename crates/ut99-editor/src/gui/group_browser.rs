use crate::editor_state::EditorState;
use egui::{Color32, RichText, Stroke};

pub fn render_group_browser(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_group_browser {
        return;
    }

    let mut open = state.show_group_browser;
    let mut close_requested = false;

    crate::gui::modal::standard_dialog_window(ctx, "Groups Browser", &mut open, [420.0, 520.0], [340.0, 320.0])
        .show(ctx, |ui| {
            // Header Info & Status
            ui.horizontal(|ui| {
                ui.label(RichText::new("UnrealEd 2.0 Group Organization").strong().color(Color32::from_rgb(100, 200, 255)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖ Close").clicked() {
                        close_requested = true;
                    }
                });
            });
            ui.separator();

            // 1. Search Bar & Global Visibility
            ui.horizontal(|ui| {
                ui.label("🔍 Search:");
                ui.text_edit_singleline(&mut state.group_browser_search);
                if ui.button("✖").clicked() {
                    state.group_browser_search.clear();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("👁‍🗨 Hide All").on_hover_text("Hide all groups from viewports").clicked() {
                        state.set_all_groups_visibility(false);
                    }
                    if ui.button("👁 Show All").on_hover_text("Make all groups visible in viewports").clicked() {
                        state.set_all_groups_visibility(true);
                    }
                });
            });

            ui.add_space(6.0);

            // 2. Groups List
            let groups = state.get_map_groups();
            let search_lower = state.group_browser_search.trim().to_lowercase();
            let filtered_groups: Vec<(String, usize, usize)> = groups
                .into_iter()
                .filter(|(name, _, _)| search_lower.is_empty() || name.to_lowercase().contains(&search_lower))
                .collect();

            let mut toggle_vis = None;
            let mut select_grp = None;
            let mut deselect_grp = None;
            let mut rename_req: Option<(String, String)> = None;
            let mut delete_req = None;

            egui::Frame::group(ui.style())
                .stroke(Stroke::new(1.0f32, Color32::from_rgb(50, 60, 75)))
                .fill(Color32::from_rgba_premultiplied(18, 22, 28, 220))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Active Groups in Map:").strong().size(12.0));
                        ui.label(RichText::new(format!("({} total)", filtered_groups.len())).size(11.0).color(Color32::GRAY));
                    });
                    ui.separator();

                    if filtered_groups.is_empty() {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("No groups defined in level yet.").italics().color(Color32::from_rgb(160, 170, 180)));
                            ui.label(RichText::new("Select objects and enter a group name below to create one.").size(11.0).color(Color32::DARK_GRAY));
                        });
                        ui.add_space(20.0);
                    } else {
                        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            egui::Grid::new("group_browser_table")
                                .num_columns(5)
                                .spacing([10.0, 6.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    // Table Headers
                                    ui.label(RichText::new("Vis").strong().size(11.0));
                                    ui.label(RichText::new("Group Name").strong().size(11.0));
                                    ui.label(RichText::new("Members").strong().size(11.0));
                                    ui.label(RichText::new("Selection").strong().size(11.0));
                                    ui.label(RichText::new("Actions").strong().size(11.0));
                                    ui.end_row();

                                    for (name, actor_count, brush_count) in &filtered_groups {
                                        let is_visible = state.is_actor_group_visible(name);

                                        // Eye toggle button
                                        let eye_text = if is_visible { "👁" } else { "👁‍🗨" };
                                        let eye_color = if is_visible {
                                            Color32::from_rgb(100, 230, 140)
                                        } else {
                                            Color32::from_rgb(130, 130, 130)
                                        };
                                        if ui.button(RichText::new(eye_text).color(eye_color).strong()).on_hover_text("Toggle viewport visibility").clicked() {
                                            toggle_vis = Some(name.clone());
                                        }

                                        // Group Name
                                        let name_color = if is_visible {
                                            Color32::from_rgb(220, 230, 240)
                                        } else {
                                            Color32::from_rgb(120, 125, 135)
                                        };
                                        ui.label(RichText::new(name).strong().color(name_color));

                                        // Badges for actors and brushes
                                        ui.horizontal(|ui| {
                                            if *actor_count > 0 {
                                                ui.label(RichText::new(format!("🎭 {}", actor_count)).size(10.5).color(Color32::from_rgb(140, 200, 255)));
                                            }
                                            if *brush_count > 0 {
                                                ui.label(RichText::new(format!("📦 {}", brush_count)).size(10.5).color(Color32::from_rgb(255, 210, 120)));
                                            }
                                        });

                                        // Select / Deselect
                                        ui.horizontal(|ui| {
                                            if ui.small_button("Select").on_hover_text("Select all objects in this group").clicked() {
                                                select_grp = Some(name.clone());
                                            }
                                            if ui.small_button("Deselect").on_hover_text("Deselect all objects in this group").clicked() {
                                                deselect_grp = Some(name.clone());
                                            }
                                        });

                                        // Context Actions
                                        ui.horizontal(|ui| {
                                            let rename_target = state.group_browser_new_name.trim();
                                            if !rename_target.is_empty() && rename_target != name {
                                                if ui.small_button("✏").on_hover_text(format!("Rename group to '{}'", rename_target)).clicked() {
                                                    rename_req = Some((name.clone(), rename_target.to_string()));
                                                }
                                            }
                                            if ui.small_button("🗑").on_hover_text("Delete group assignment").clicked() {
                                                delete_req = Some(name.clone());
                                            }
                                        });

                                        ui.end_row();
                                    }
                                });
                        });
                    }
                });

            // Apply queued group actions
            if let Some(grp) = toggle_vis {
                state.toggle_group_visibility(&grp);
            }
            if let Some(grp) = select_grp {
                state.select_objects_by_group(&grp);
            }
            if let Some(grp) = deselect_grp {
                state.deselect_objects_by_group(&grp);
            }
            if let Some((old_n, new_n)) = rename_req {
                state.rename_group(&old_n, &new_n);
            }
            if let Some(grp) = delete_req {
                state.delete_group(&grp);
            }

            ui.add_space(8.0);
            ui.separator();

            // 3. Group Assignment & Management Panel
            egui::Frame::group(ui.style())
                .stroke(Stroke::new(1.0f32, Color32::from_rgb(60, 75, 95)))
                .fill(Color32::from_rgba_premultiplied(22, 28, 36, 230))
                .show(ui, |ui| {
                    ui.label(RichText::new("⚙ Group Assignment & Management").strong().size(12.0));
                    ui.add_space(4.0);

                    // Selected count badge
                    let (sel_actors, sel_brushes) = {
                        let a_cnt = if state.multi_selected_actors.is_empty() {
                            if state.selected_actor_index.is_some() { 1 } else { 0 }
                        } else {
                            state.multi_selected_actors.len()
                        };
                        let b_cnt = if state.multi_selected_brushes.is_empty() {
                            if state.selected_brush_index.is_some() { 1 } else { 0 }
                        } else {
                            state.multi_selected_brushes.len()
                        };
                        (a_cnt, b_cnt)
                    };

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Currently Selected:").size(11.0));
                        ui.label(RichText::new(format!("{} actor(s), {} brush(es)", sel_actors, sel_brushes))
                            .strong()
                            .color(if sel_actors + sel_brushes > 0 { Color32::from_rgb(120, 240, 160) } else { Color32::GRAY }));
                    });

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Target Group:");
                        let edit = ui.text_edit_singleline(&mut state.group_browser_new_name);
                        if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let name = state.group_browser_new_name.trim().to_string();
                            if !name.is_empty() {
                                state.set_selected_objects_group(&name);
                            }
                        }
                    });

                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        let assign_enabled = sel_actors + sel_brushes > 0 && !state.group_browser_new_name.trim().is_empty();
                        if ui.add_enabled(
                            assign_enabled,
                            egui::Button::new(RichText::new("➕ Assign Selection").color(Color32::from_rgb(120, 240, 160)).strong())
                        ).on_hover_text("Assign currently selected actors and brushes to Target Group").clicked() {
                            let name = state.group_browser_new_name.trim().to_string();
                            state.set_selected_objects_group(&name);
                        }

                        let remove_enabled = sel_actors + sel_brushes > 0;
                        if ui.add_enabled(
                            remove_enabled,
                            egui::Button::new("➖ Remove from Group")
                        ).on_hover_text("Clear group attribute for selected objects").clicked() {
                            state.remove_selected_from_group();
                        }
                    });
                });
        });

    if close_requested {
        open = false;
    }
    state.show_group_browser = open;
}
